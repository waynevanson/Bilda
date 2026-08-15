use crate::{
    ast::{
        Assignment, Ast, Call, Expression, Lambda, LetIn, Map, Math, MathSign, MathTarget, Product,
        Sum,
    },
    lexer::Token,
};
use chumsky::{
    input::ValueInput,
    pratt::{infix, left},
    prelude::*,
};

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

fn expression_string<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! { Token::StringLiteral(s) => Expression::String(s) }
}

fn expression_boolean<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::True => Expression::Boolean(true),
        Token::False => Expression::Boolean(false),
    }
}

fn type_name<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, &'src str, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::Identifier(name) => name,
        Token::Int => "Int",
        Token::String => "String",
    }
}

fn expression_lambda<'tok, 'src: 'tok, I, E>(
    expr: E,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    E: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    let param = property()
        .then_ignore(just(Token::Colon))
        .then(type_name())
        .map(|(name, _ty)| name);

    just(Token::RoundBracketLeft)
        .ignore_then(param.repeated().collect())
        .then_ignore(just(Token::RoundBracketRight))
        .then_ignore(just(Token::Arrow))
        .then(expr)
        .map(|(params, body)| {
            Expression::Lambda(Lambda {
                params,
                body: Box::new(body),
            })
        })
}

fn atom<'tok, 'src: 'tok, I, M>(
    map: M,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    M: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    choice((expression_string(), expression_boolean(), map))
}

fn full_property<'tok, 'src: 'tok, I, A>(
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

fn shorthand_property<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    property().map(|name| Assignment {
        name,
        r#type: None,
        value: Box::new(Ast::Expression(Expression::Reference(name))),
    })
}

fn map_assignment<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    full_property(ast.clone()).or(shorthand_property())
}

fn expression_map<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::CurlyBracketLeft)
        .ignore_then(map_assignment(ast).repeated().collect())
        .then_ignore(just(Token::CurlyBracketRight))
        .map(|assignments| Expression::Map(Map { assignments }))
}

fn expression_product<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::Plus)
        .ignore_then(expression_map(ast))
        .map(|expression| match expression {
            Expression::Map(Map { assignments }) => Expression::Product(Product { assignments }),
            _ => unreachable!(),
        })
}

fn expression_sum<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::Asterisk)
        .ignore_then(expression_map(ast))
        .map(|expression| match expression {
            Expression::Map(Map { assignments }) => Expression::Sum(Sum { assignments }),
            _ => unreachable!(),
        })
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
        .map(|(name, arg)| {
            Expression::Call(Call {
                function: Box::new(Expression::Reference(name)),
                argument: Box::new(arg),
            })
        })
}

fn math_target<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, MathTarget<'src>, Extra<'tok, 'src>> + Clone
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

        let combine = |left, sign, right| MathTarget::Math(Box::new(Math { left, sign, right }));

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

fn expression_math<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    math().map(|target| match target {
        MathTarget::Number(n) => Expression::Int(n),
        MathTarget::Reference(name) => Expression::Reference(name),
        MathTarget::Math(math) => Expression::Math(*math),
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
        let map = expression_map(ast.clone());
        let product = expression_product(ast.clone());
        let sum = expression_sum(ast.clone());
        let call = call(expr.clone(), map.clone());
        let lambda = expression_lambda(expr.clone());
        let choices = (call, sum, product, lambda, expression_math(), atom(map));

        choice(choices)
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
        .map(|(assignments, expression)| {
            Ast::LetIn(LetIn {
                assignments,
                expression: Box::new(expression),
            })
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
        Ast::LetIn(LetIn {
            assignments: vec![
                Assignment {
                    name: "a",
                    r#type: None,
                    value: Box::new(Ast::Expression(Expression::Int(2))),
                },
                Assignment {
                    name: "b",
                    r#type: None,
                    value: Box::new(Ast::LetIn(LetIn {
                        assignments: vec![Assignment {
                            name: "c",
                            r#type: None,
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
                    r#type: None,
                    value: Box::new(Ast::Expression(Expression::Product(Product {
                        assignments: vec![
                            Assignment {
                                name: "completed",
                                r#type: None,
                                value: Box::new(Ast::Expression(Expression::Boolean(true))),
                            },
                            Assignment {
                                name: "duration",
                                r#type: None,
                                value: Box::new(Ast::Expression(Expression::Reference("u32"))),
                            },
                        ],
                    }))),
                },
                Assignment {
                    name: "Chore",
                    r#type: None,
                    value: Box::new(Ast::Expression(Expression::Sum(Sum {
                        assignments: vec![
                            Assignment {
                                name: "title",
                                r#type: None,
                                value: Box::new(Ast::Expression(Expression::Reference("String"))),
                            },
                            Assignment {
                                name: "description",
                                r#type: None,
                                value: Box::new(Ast::Expression(Expression::Reference("String"))),
                            },
                            Assignment {
                                name: "status",
                                r#type: None,
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
                            r#type: None,
                            value: Box::new(Ast::Expression(Expression::String("Vacuum"))),
                        },
                        Assignment {
                            name: "description",
                            r#type: None,
                            value: Box::new(Ast::Expression(Expression::String(
                                "Get the machine do the sucky in every room",
                            ))),
                        },
                        Assignment {
                            name: "status",
                            r#type: None,
                            value: Box::new(Ast::Expression(Expression::Call(Call {
                                function: Box::new(Expression::Reference("Status")),
                                argument: Box::new(Expression::Map(Map {
                                    assignments: vec![Assignment {
                                        name: "completed",
                                        r#type: None,
                                        value: Box::new(Ast::Expression(Expression::Boolean(true))),
                                    }],
                                })),
                            }))),
                        },
                    ],
                })),
            })),
        })
    )]
    fn expression(#[case] source: &str, #[case] expected: Ast<'_>) {
        let tokens: Vec<Token<'_>> = Token::lexer(source)
            .collect::<Result<_, _>>()
            .expect("lex error");
        let actual = ast().parse(Stream::from_iter(tokens)).into_result();
        assert_eq!(actual, Ok(expected));
    }
}
