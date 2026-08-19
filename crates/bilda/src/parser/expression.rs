use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast::{Ast, Boolean, Call, Expression, Lambda};
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

pub fn expression_boolean<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::True => Expression::Boolean(Boolean(true)),
        Token::False => Expression::Boolean(Boolean(false)),
    }
}

pub fn expression_lambda<'tok, 'src: 'tok, I, E>(
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

pub fn expression_not<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    just(Token::Bang)
        .ignore_then(expression_boolean())
        .map(|e| match e {
            Expression::Boolean(b) => Expression::Not(b),
            _ => unreachable!(),
        })
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

    property()
        .then(choice((paren_args, map.clone())))
        .map(|(name, arg)| {
            Expression::Call(Call {
                function: Box::new(Expression::Reference(name)),
                argument: Box::new(arg),
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
        let product = expression_product(ast.clone());
        let sum = expression_sum(ast.clone());
        let call = call(expr.clone(), map.clone());
        let lambda = expression_lambda(expr.clone());
        let not = expression_not();
        let choices = (call, sum, product, lambda, expression_math(), not, atom(map));

        choice(choices)
    })
}
