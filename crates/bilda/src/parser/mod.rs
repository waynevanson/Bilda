use chumsky::prelude::*;

pub mod ast;
pub mod expression;
pub mod map;
pub mod math;
pub mod property;

type Extra<'tok, 'src> = extra::Err<Rich<'tok, crate::lexer::Token<'src>>>;

pub use crate::parser::ast::ast;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        Assignment, Ast, Boolean, Call, Concat, Expression, Lambda, LetIn, Map, Math, MathSign,
        MathTarget, Product, Sum,
    };
    use crate::lexer::Token;
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
        Ast::LetIn(LetIn {
            assignments: vec![
                Assignment {
                    name: "a",
                    value: Box::new(Ast::Expression(Expression::Int(2))),
                },
                Assignment {
                    name: "b",
                    value: Box::new(Ast::LetIn(LetIn {
                        assignments: vec![Assignment {
                            name: "c",
                            value: Box::new(Ast::Expression(Expression::Int(1))),
                        }],
                        expression: Box::new(Expression::Reference("c")),
                    })),
                },
            ],
            expression: Box::new(Expression::Math(Math {
                sign: MathSign::Addition,
                left: MathTarget::Reference("a"),
                right: MathTarget::Reference("b"),
            })),
        })
    )]
    #[case(
        "!True",
        Ast::Expression(Expression::Not(Boolean(true)))
    )]
    #[case(
        "!False",
        Ast::Expression(Expression::Not(Boolean(false)))
    )]
    #[case(
        r#"
            let
              Status = + {
                completed = True
                duration = u32
              }
              Chore = * {
                title = String
                description = String
                status = Status
              }
            in
              Chore {
                title = "Vacuum"
                description = "Get the machine do the sucky in every room"
                status = Status {
                  completed = True
                }
              }
        "#,
        Ast::LetIn(LetIn {
            assignments: vec![
                Assignment {
                    name: "Status",
                    value: Box::new(Ast::Expression(Expression::Product(Product {
                        assignments: vec![
                            Assignment {
                                name: "completed",
                                value: Box::new(Ast::Expression(Expression::Boolean(Boolean(true)))),
                            },
                            Assignment {
                                name: "duration",
                                value: Box::new(Ast::Expression(Expression::Reference("u32"))),
                            },
                        ],
                    }))),
                },
                Assignment {
                    name: "Chore",
                    value: Box::new(Ast::Expression(Expression::Sum(Sum {
                        assignments: vec![
                            Assignment {
                                name: "title",
                                value: Box::new(Ast::Expression(Expression::Reference("String"))),
                            },
                            Assignment {
                                name: "description",
                                value: Box::new(Ast::Expression(Expression::Reference("String"))),
                            },
                            Assignment {
                                name: "status",
                                value: Box::new(Ast::Expression(Expression::Reference("Status"))),
                            },
                        ],
                    }))),
                },
            ],
            expression: Box::new(Expression::Call(Call {
                function: Box::new(Expression::Reference("Chore")),
                argument: Box::new(Expression::Map(Map {
                    assignments: vec![
                        Assignment {
                            name: "title",
                            value: Box::new(Ast::Expression(Expression::String("Vacuum"))),
                        },
                        Assignment {
                            name: "description",
                            value: Box::new(Ast::Expression(Expression::String(
                                "Get the machine do the sucky in every room",
                            ))),
                        },
                        Assignment {
                            name: "status",
                            value: Box::new(Ast::Expression(Expression::Call(Call {
                                function: Box::new(Expression::Reference("Status")),
                                argument: Box::new(Expression::Map(Map {
                                    assignments: vec![Assignment {
                                        name: "completed",
                                        value: Box::new(Ast::Expression(Expression::Boolean(Boolean(true)))),
                                    }],
                                })),
                            }))),
                        },
                    ],
                })),
            })),
        })
    )]
    #[case(
        r#""a" ++ "b""#,
        Ast::Expression(Expression::Concat(Box::new(Concat {
            left: Expression::String("a"),
            right: Expression::String("b"),
        })))
    )]
    #[case(
        r#"["a" "b"]"#,
        Ast::Expression(Expression::List(vec![
            Expression::String("a"),
            Expression::String("b"),
        ]))
    )]
    #[case(
        "{ ok = }",
        Ast::Expression(Expression::Map(Map {
            assignments: vec![Assignment {
                name: "ok",
                value: Box::new(Ast::Expression(Expression::Unit)),
            }],
        }))
    )]
    #[case(
        "sum { ok error }",
        Ast::Expression(Expression::Sum(Sum {
            assignments: vec![
                Assignment {
                    name: "ok",
                    value: Box::new(Ast::Expression(Expression::Unit)),
                },
                Assignment {
                    name: "error",
                    value: Box::new(Ast::Expression(Expression::Unit)),
                },
            ],
        }))
    )]
    #[case(
        r"\x y => x",
        Ast::Expression(Expression::Lambda(Lambda {
            params: vec!["x", "y"],
            body: Box::new(Ast::Expression(Expression::Reference("x"))),
        }))
    )]
    #[case(
        "f 1 2 3",
        Ast::Expression(Expression::Call(Call {
            function: Box::new(Expression::Call(Call {
                function: Box::new(Expression::Call(Call {
                    function: Box::new(Expression::Reference("f")),
                    argument: Box::new(Expression::Int(1)),
                })),
                argument: Box::new(Expression::Int(2)),
            })),
            argument: Box::new(Expression::Int(3)),
        }))
    )]
    #[case(
        "add x y",
        Ast::Expression(Expression::Call(Call {
            function: Box::new(Expression::Call(Call {
                function: Box::new(Expression::Reference("add")),
                argument: Box::new(Expression::Reference("x")),
            })),
            argument: Box::new(Expression::Reference("y")),
        }))
    )]
    fn expression(#[case] source: &str, #[case] expected: Ast<'_>) {
        let tokens: Vec<Token<'_>> = Token::lexer(source)
            .collect::<Result<_, _>>()
            .expect("lex error");
        let actual = ast().parse(Stream::from_iter(tokens)).into_result();
        assert_eq!(actual, Ok(expected));
    }
}
