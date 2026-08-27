use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast::{Ast, Boolean, Call, Concat, Expression, Lambda};
use crate::lexer::Token;
use crate::parser::Extra;
use crate::parser::map::{expression_map, expression_product, expression_sum};
use crate::parser::math::expression_math;
use crate::parser::property::{property, type_name};

pub fn expression_string<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! { Token::StringLiteral(s) => Expression::String(s) }
}

pub fn boolean<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, Boolean, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::True => Boolean(true),
        Token::False => Boolean(false),
    }
}

pub fn expression_boolean<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    boolean().map(Expression::Boolean)
}

pub fn expression_lambda<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    let typed_param = property()
        .then_ignore(just(Token::Colon))
        .then(type_name())
        .map(|(name, _ty)| name);

    let paren = just(Token::RoundBracketLeft)
        .ignore_then(typed_param.repeated().collect())
        .then_ignore(just(Token::RoundBracketRight))
        .then_ignore(just(Token::Arrow))
        .then(ast.clone())
        .map(|(params, body)| {
            Expression::Lambda(Lambda {
                params,
                body: Box::new(body),
            })
        });

    let backslash = just(Token::Backslash)
        .ignore_then(property().repeated().collect())
        .then_ignore(just(Token::Arrow))
        .then(ast.clone())
        .map(|(params, body)| {
            Expression::Lambda(Lambda {
                params,
                body: Box::new(body),
            })
        });

    paren.or(backslash)
}

pub fn expression_not<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    just(Token::Bang)
        .ignore_then(boolean())
        .map(Expression::Not)
}

pub fn expression_list<'tok, 'src: 'tok, I, E>(
    expr: E,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    E: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::SquareBracketLeft)
        .ignore_then(expr.repeated().collect())
        .then_ignore(just(Token::SquareBracketRight))
        .map(Expression::List)
}

pub fn atom<'tok, 'src: 'tok, I, M>(
    map: M,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    M: Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    choice((expression_string(), expression_boolean(), map))
}

pub fn call<'tok, 'src: 'tok, I, E, M>(
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

    let arg = choice((paren_args, map.clone()));

    property()
        .then(arg.repeated().at_least(1).collect::<Vec<_>>())
        .map(|(name, args)| {
            let function = Expression::Reference(name);
            args.into_iter().fold(function, |function, argument| {
                Expression::Call(Call {
                    function: Box::new(function),
                    argument: Box::new(argument),
                })
            })
        })
}

pub fn expression<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    recursive(|expr| {
        let map = expression_map(ast.clone());
        let list = expression_list(expr.clone());
        let product = expression_product(ast.clone());
        let sum = expression_sum(ast.clone());
        let call = call(expr.clone(), map.clone());
        let lambda = expression_lambda(ast.clone());
        let not = expression_not();

        let term = choice((
            call,
            sum,
            product,
            lambda,
            list,
            expression_math(),
            not,
            atom(map),
        ));

        term.clone().foldl(
            just(Token::Concat)
                .ignore_then(term.clone())
                .repeated(),
            |left, right| Expression::Concat(Box::new(Concat { left, right })),
        )
    })
}
