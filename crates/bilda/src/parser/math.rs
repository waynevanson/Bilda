use chumsky::input::ValueInput;
use chumsky::pratt::{infix, left};
use chumsky::prelude::*;

use crate::ast::{Expression, Math, MathSign, MathTarget};
use crate::lexer::Token;
use crate::parser::Extra;

pub fn math_target<'tok, 'src: 'tok, I>()
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

pub fn math<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, MathTarget<'src>, Extra<'tok, 'src>> + Clone
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

pub fn expression_math<'tok, 'src: 'tok, I>()
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
