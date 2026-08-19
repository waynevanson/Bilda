use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::lexer::Token;
use crate::parser::Extra;

pub fn property<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, &'src str, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::Identifier(name) => name,
    }
}

pub fn type_name<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, &'src str, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    select! {
        Token::Identifier(name) => name,
        Token::Int => "Int",
        Token::String => "String",
    }
}
