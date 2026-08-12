use crate::{
    ast::{Assignment, Ast, Expression, Math, MathSign, MathTarget},
    lexer::Token,
};
use chumsky::{input::ValueInput, pratt::{infix, left}, prelude::*};

type Extra<'tok, 'src> = extra::Err<Rich<'tok, Token<'src>>>;

pub fn ast<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    recursive(|ast| {
        let expr = expression(ast.clone());
        let_in(ast.clone(), expr.clone()).or(expr.map(Ast::Expression))
    })
}

fn property<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, &'src str, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::Identifier(name) => name,
    }
}

fn atom<'tok, 'src: 'tok, I, M>(map: M) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    M: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    choice((
        select! { Token::StringLiteral(s) => Expression::String(s) },
        select! {
            Token::True => Expression::Boolean(true),
            Token::False => Expression::Boolean(false),
        },
        map,
    ))
}

fn map_assignment<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    let full = property()
        .then_ignore(just(Token::Equal))
        .then(ast.clone())
        .map(|(name, value)| Assignment {
            name,
            r#type: None,
            value: Box::new(value),
        });

    let shorthand = property().map(|name| Assignment {
        name,
        r#type: None,
        value: Box::new(Ast::Expression(Expression::Reference(name))),
    });

    full.or(shorthand)
}

fn map_expr<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::CurlyBracketLeft)
        .ignore_then(map_assignment(ast).repeated().collect())
        .then_ignore(just(Token::CurlyBracketRight))
        .map(|assignments| Expression::Map { assignments })
}

fn call<'tok, 'src: 'tok, I, E, M>(
    expr: E,
    map: M,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    E: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
    M: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    let paren_args = expr.clone().delimited_by(
        just(Token::RoundBracketLeft),
        just(Token::RoundBracketRight),
    );

    property()
        .then(choice((paren_args, map.clone())))
        .map(|(name, arg)| Expression::Call {
            function: name,
            argument: Box::new(arg),
        })
}

fn math_target<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, MathTarget<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::Number(n) => MathTarget::Number(n),
        Token::Identifier(str) => MathTarget::Reference(str),
        Token::String => MathTarget::Reference("String"),
        Token::Int => MathTarget::Reference("Int"),
    }
}

fn math<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, MathTarget<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    recursive(|math| {
        let sign_high = select! {
            Token::Asterisk => MathSign::Multiplication,
            Token::ForwardSlash => MathSign::Division,
        };

        let sign_low = select! {
            Token::Plus => MathSign::Addition,
            Token::Minus => MathSign::Subtraction,
        };

        let combine =
            |left, sign, right| MathTarget::Math(Box::new(Math { left, sign, right }));

        math.delimited_by(
            just(Token::RoundBracketLeft),
            just(Token::RoundBracketRight),
        )
        .or(math_target())
        .pratt((
            infix(left(2), sign_high, move |l, sign, r, _| combine(l, sign, r)),
            infix(left(1), sign_low, move |l, sign, r, _| combine(l, sign, r)),
        ))
    })
}

fn expression<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    recursive(|expr| {
        let map = map_expr(ast.clone());
        let call = call(expr.clone(), map.clone());
        let math_expr = math().map(|target| match target {
            MathTarget::Number(n) => Expression::Int(n),
            MathTarget::Reference(name) => Expression::Reference(name),
            MathTarget::Math(math) => Expression::Math(*math),
        });

        choice((call, math_expr, atom(map)))
            .separated_by(just(Token::Pipe))
            .at_least(1)
            .collect()
            .map(|parts: Vec<Expression<'src>>| {
                if parts.len() == 1 {
                    parts.into_iter().next().unwrap()
                } else {
                    Expression::Union(parts)
                }
            })
    })
}

fn assignment<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    property()
        .then_ignore(just(Token::Equal))
        .then(ast)
        .map(|(name, value)| Assignment {
            name,
            r#type: None,
            value: Box::new(value),
        })
}

fn let_in<'tok, 'src: 'tok, I, A, E>(
    ast: A,
    expr: E,
) -> impl Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
    E: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::Let)
        .ignore_then(assignment(ast).repeated().at_least(1).collect())
        .then_ignore(just(Token::In))
        .then(expr)
        .map(|(assignments, expression)| Ast::LetIn {
            assignments,
            expression: Box::new(expression),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::{Parser, input::Stream};
    use logos::Logos;
    use rstest::rstest;

    #[rstest]
    #[case(
        "1 + 2",
        Ast::Expression(Expression::Math(Math {
            sign: MathSign::Addition,
            left: MathTarget::Number(1),
            right: MathTarget::Number(2),
        }))
    )]
    #[case(
        "1 - 2",
        Ast::Expression(Expression::Math(Math {
            sign: MathSign::Subtraction,
            left: MathTarget::Number(1),
            right: MathTarget::Number(2),
        }))
    )]
    #[case(
        "2 * 3",
        Ast::Expression(Expression::Math(Math {
            sign: MathSign::Multiplication,
            left: MathTarget::Number(2),
            right: MathTarget::Number(3),
        }))
    )]
    #[case(
        "4 / 2",
        Ast::Expression(Expression::Math(Math {
            sign: MathSign::Division,
            left: MathTarget::Number(4),
            right: MathTarget::Number(2),
        }))
    )]
    #[case(
        "1 + 2 * 3",
        Ast::Expression(Expression::Math(Math {
            sign: MathSign::Addition,
            left: MathTarget::Number(1),
            right: MathTarget::Math(Box::new(Math {
                sign: MathSign::Multiplication,
                left: MathTarget::Number(2),
                right: MathTarget::Number(3),
            })),
        }))
    )]
    #[case(
        "(1 + 2) * 3",
        Ast::Expression(Expression::Math(Math {
            sign: MathSign::Multiplication,
            left: MathTarget::Math(Box::new(Math {
                sign: MathSign::Addition,
                left: MathTarget::Number(1),
                right: MathTarget::Number(2),
            })),
            right: MathTarget::Number(3),
        }))
    )]
    #[case(
        r#"
            let
              a = 2
              b = let
                c = 1
              in
                c
            in
              a + b
        "#,
        Ast::LetIn {
            assignments: vec![
                Assignment {
                    name: "a",
                    r#type: None,
                    value: Box::new(Ast::Expression(Expression::Int(2))),
                },
                Assignment {
                    name: "b",
                    r#type: None,
                    value: Box::new(Ast::LetIn {
                        assignments: vec![Assignment {
                            name: "c",
                            r#type: None,
                            value: Box::new(Ast::Expression(Expression::Int(1))),
                        }],
                        expression: Box::new(Expression::Reference("c")),
                    }),
                },
            ],
            expression: Box::new(Expression::Math(Math {
                sign: MathSign::Addition,
                left: MathTarget::Reference("a"),
                right: MathTarget::Reference("b"),
            })),
        }
    )]
    fn expression(#[case] source: &str, #[case] expected: Ast<'_>) {
        let tokens: Vec<Token<'_>> = Token::lexer(source)
            .collect::<Result<_, _>>()
            .expect("lex error");
        let actual = ast().parse(Stream::from_iter(tokens)).into_result();
        assert_eq!(actual, Ok(expected));
    }

    #[test]
    fn chore_snippet() {
        let source = r#"
            let
              Chore = {
                description = String
                owner = String
                completed = False | { TimeTaken }
              }
              TimeTaken = Int
              completed = TimeTaken(2)
              chore = Chore {
                description = "Vacuum"
                owner = "Wayne"
                completed
              }
            in
              chore
        "#;

        let tokens: Vec<Token<'_>> = Token::lexer(source)
            .collect::<Result<_, _>>()
            .expect("lex error");
        let actual = ast().parse(Stream::from_iter(tokens)).into_result();
        assert!(actual.is_ok(), "{actual:#?}");
    }
}
