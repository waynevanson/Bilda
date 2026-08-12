use crate::{
    ast::{Assignment, Ast, Expression, Math, MathSign, MathTarget},
    lexer::Token,
};
use chumsky::{
    input::ValueInput,
    pratt::{infix, left},
    prelude::*,
    primitive::Select,
};

pub fn ast<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Ast<'tok>, extra::Err<Rich<'tok, Token<'src>>>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    let property = select! {
        Token::Identifier(name) => name,
    };

    let equal = just(Token::Equal);
    let r#in = just(Token::In);

    recursive(|ast| {
        let expr = recursive(|expr| {
            let map_assignment =
                {
                    let full = property.then_ignore(equal.clone()).then(ast.clone()).map(
                        |(name, value)| Assignment {
                            name,
                            r#type: None,
                            value: Box::new(value),
                        },
                    );

                    let shorthand = property.map(|name| Assignment {
                        name,
                        r#type: None,
                        value: Box::new(Ast::Expression(Expression::Reference(name))),
                    });

                    full.or(shorthand)
                };

            let map = just(Token::CurlyBracketLeft)
                .ignore_then(map_assignment.repeated().collect())
                .then_ignore(just(Token::CurlyBracketRight))
                .map(|assignments| Expression::Map { assignments });

            let atom = choice((
                select! { Token::StringLiteral(s) => Expression::String(s) },
                select! {
                    Token::True => Expression::Boolean(true),
                    Token::False => Expression::Boolean(false),
                },
                map.clone(),
            ));

            let call = {
                let name = select! { Token::Identifier(name) => name };
                let paren_args = expr.clone().delimited_by(
                    just(Token::RoundBracketLeft),
                    just(Token::RoundBracketRight),
                );
                name.then(choice((paren_args, map.clone())))
                    .map(|(name, arg)| Expression::Call {
                        function: name,
                        argument: Box::new(arg),
                    })
            };

            let math_target = select! {
                Token::Number(n) => MathTarget::Number(n),
                Token::Identifier(str) => MathTarget::Reference(str),
                Token::String => MathTarget::Reference("String"),
                Token::Int => MathTarget::Reference("Int"),
            };

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

            let math = recursive(|math| {
                math.delimited_by(
                    just(Token::RoundBracketLeft),
                    just(Token::RoundBracketRight),
                )
                .or(math_target)
                .pratt((
                    infix(left(2), sign_high, move |l, sign, r, _| combine(l, sign, r)),
                    infix(left(1), sign_low, move |l, sign, r, _| combine(l, sign, r)),
                ))
            });

            let math_expr = math.map(|target| match target {
                MathTarget::Number(n) => Expression::Int(n),
                MathTarget::Reference(name) => Expression::Reference(name),
                MathTarget::Math(math) => Expression::Math(*math),
            });

            choice((call, math_expr, atom))
                .separated_by(just(Token::Pipe))
                .at_least(1)
                .collect()
                .map(|parts: Vec<Expression>| {
                    if parts.len() == 1 {
                        parts.into_iter().next().unwrap()
                    } else {
                        Expression::Union(parts)
                    }
                })
        });

        let assignment = property
            .then_ignore(equal)
            .then(ast.clone())
            .map(|(name, value)| Assignment {
                name,
                r#type: None,
                value: Box::new(value),
            });

        let assignments = assignment.repeated().at_least(1).collect();

        let let_in = just(Token::Let)
            .ignore_then(assignments)
            .then_ignore(r#in)
            .then(expr.clone())
            .map(|(assignments, expression)| Ast::LetIn {
                assignments,
                expression: Box::new(expression),
            });

        let_in.or(expr.map(Ast::Expression))
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
