use chumsky::pratt::*;
use chumsky::prelude::*;

use crate::ast::*;
use crate::lexer::Token;

type Tok<'src> = &'src [Token];
type Extra<'src> = extra::Err<Simple<'src, Token>>;

fn ident<'src>() -> impl Parser<'src, Tok<'src>, String, Extra<'src>> + Clone {
    select! { Token::Ident(name) => name }
}

fn keyword<'src>(token: Token) -> impl Parser<'src, Tok<'src>, (), Extra<'src>> + Clone {
    just(token).ignored()
}

fn newlines<'src>() -> impl Parser<'src, Tok<'src>, (), Extra<'src>> + Clone {
    just(Token::Newline).repeated().ignored()
}

fn newlines_req<'src>() -> impl Parser<'src, Tok<'src>, (), Extra<'src>> + Clone {
    just(Token::Newline).repeated().at_least(1).ignored()
}

fn expr_parser<'src>() -> impl Parser<'src, Tok<'src>, Expr, Extra<'src>> + Clone {
    let pratt_expr = recursive(|pratt_expr| {
        let field = ident()
            .then(just(Token::Equal).ignore_then(pratt_expr.clone()).or_not())
            .map(|(name, value)| match value {
                Some(value) => Binding { name, value },
                None => Binding {
                    name: name.clone(),
                    value: Expr::Ident(name),
                },
            });

        let record = field
            .repeated()
            .collect::<Vec<_>>()
            .then_ignore(newlines())
            .delimited_by(
                just(Token::CurlyLeft).then_ignore(newlines()),
                just(Token::CurlyRight),
            )
            .map(Expr::Record);

        let base = choice((
            record,
            select! { Token::Number(n) => Expr::Number(n) },
            ident().map(Expr::Ident),
            pratt_expr
                .clone()
                .delimited_by(just(Token::BracketLeft), just(Token::BracketRight)),
        ));

        base.pratt((
            postfix(
                4,
                just(Token::DotSingle).ignore_then(ident()),
                |lhs, field, _| Expr::Access {
                    target: Box::new(lhs),
                    field,
                    kind: AccessKind::Value,
                },
            ),
            postfix(
                4,
                just(Token::DoubleColon).ignore_then(ident()),
                |lhs, field, _| Expr::Access {
                    target: Box::new(lhs),
                    field,
                    kind: AccessKind::Type,
                },
            ),
            prefix(3, just(Token::Tilde), |_, rhs, _| Expr::Prefix {
                op: PrefixOp::Annotation,
                expr: Box::new(rhs),
            }),
            prefix(3, just(Token::Plus), |_, rhs, _| Expr::Prefix {
                op: PrefixOp::Sum,
                expr: Box::new(rhs),
            }),
            prefix(3, just(Token::Star), |_, rhs, _| Expr::Prefix {
                op: PrefixOp::Product,
                expr: Box::new(rhs),
            }),
            prefix(3, just(Token::Minus), |_, rhs, _| Expr::Prefix {
                op: PrefixOp::Difference,
                expr: Box::new(rhs),
            }),
            infix(left(2), just(Token::Star), |l, _, r, _| Expr::Infix {
                op: InfixOp::Mul,
                left: Box::new(l),
                right: Box::new(r),
            }),
            infix(left(1), just(Token::Plus), |l, _, r, _| Expr::Infix {
                op: InfixOp::Add,
                left: Box::new(l),
                right: Box::new(r),
            }),
            infix(left(1), just(Token::Minus), |l, _, r, _| Expr::Infix {
                op: InfixOp::Sub,
                left: Box::new(l),
                right: Box::new(r),
            }),
        ))
    });

    let application = pratt_expr
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|atoms| {
            let mut iter = atoms.into_iter();
            let first = iter.next().unwrap();
            iter.fold(first, |acc, arg| Expr::Apply {
                func: Box::new(acc),
                arg: Box::new(arg),
            })
        });

    let lambda = ident()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then_ignore(just(Token::Arrow));

    recursive(|expr| {
        let lambda = lambda
            .then(expr.clone())
            .map(|(params, body)| Expr::Lambda {
                params,
                body: Box::new(body),
            });

        lambda.or(application)
    })
}

fn program_parser<'src>() -> impl Parser<'src, Tok<'src>, Ast, Extra<'src>> + Clone {
    let expr = expr_parser();

    let declaration = ident()
        .then_ignore(just(Token::Equal))
        .then(expr.clone())
        .map(|(name, value)| Binding { name, value });

    let declarations = declaration
        .clone()
        .separated_by(newlines_req())
        .allow_trailing()
        .collect::<Vec<_>>();

    let vars_block = keyword(Token::Vars)
        .ignore_then(newlines_req())
        .then(declarations.clone())
        .then_ignore(newlines())
        .then_ignore(keyword(Token::Expr))
        .then_ignore(newlines_req())
        .then(expr.clone())
        .map(|((_, bindings), body)| {
            Block::Let(LetBlock {
                bindings,
                body: Box::new(body),
            })
        });

    let block = vars_block;

    block
        .separated_by(newlines_req())
        .allow_trailing()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map(Ast)
}

pub fn parse(tokens: &[Token]) -> Result<Ast, Vec<Simple<'_, Token>>> {
    program_parser().parse(tokens).into_result()
}

#[cfg(test)]
mod tests {
    use logos::Logos;

    use super::*;
    use crate::lexer::Token;

    fn lex(input: &str) -> Vec<Token> {
        let input = input.trim();
        Result::<Vec<Token>, ()>::from_iter(Token::lexer(input)).unwrap()
    }

    #[test]
    fn vars_expr() {
        let tokens = lex(r#"
            vars
                Name = String
            expr
                Name
        "#);

        let Ast(blocks) = parse(&tokens).unwrap();
        assert_eq!(blocks.len(), 1);

        let Block::Let(block) = &blocks[0];
        assert_eq!(block.bindings.len(), 1);
        assert_eq!(block.bindings[0].name, "Name");
        assert_eq!(block.bindings[0].value, Expr::Ident("String".to_string()));
        assert_eq!(block.body.as_ref(), &Expr::Ident("Name".to_string()));
    }

    #[test]
    fn lambda_and_application() {
        let tokens = lex(r#"
            vars
                closure = x y => x * y
                answer = closure 6 7
            expr
                answer
        "#);

        let Ast(blocks) = parse(&tokens).unwrap();
        let Block::Let(block) = &blocks[0];

        assert_eq!(block.bindings[0].name, "closure");
        assert!(matches!(block.bindings[0].value, Expr::Lambda { .. }));
        assert_eq!(block.bindings[1].name, "answer");
        assert!(matches!(block.bindings[1].value, Expr::Apply { .. }));
    }

    #[test]
    fn record_fields() {
        let tokens = lex(r#"
            vars
                Name = + { First = String Second = String }
            expr
                Name
        "#);

        let Ast(blocks) = parse(&tokens).unwrap();
        let Block::Let(block) = &blocks[0];

        assert_eq!(block.bindings[0].name, "Name");
        let Expr::Prefix {
            op: PrefixOp::Sum,
            expr,
        } = &block.bindings[0].value
        else {
            panic!("expected sum prefix");
        };
        let Expr::Record(fields) = expr.as_ref() else {
            panic!("expected record");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "First");
        assert_eq!(fields[1].name, "Second");
    }

    #[test]
    fn access() {
        let tokens = lex(r#"
            vars
                type_access = Name::First
                value_access = Name.First
            expr
                value_access
        "#);

        let Ast(blocks) = parse(&tokens).unwrap();
        let Block::Let(block) = &blocks[0];

        assert!(matches!(
            block.bindings[0].value,
            Expr::Access {
                kind: AccessKind::Type,
                ..
            }
        ));
        assert!(matches!(
            block.bindings[1].value,
            Expr::Access {
                kind: AccessKind::Value,
                ..
            }
        ));
    }

}
