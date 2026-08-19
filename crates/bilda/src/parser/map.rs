use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast::{Assignment, Ast, Expression, Map, Product, Sum};
use crate::lexer::Token;
use crate::parser::Extra;
use crate::parser::property::property;

pub fn full_property<'tok, 'src: 'tok, I, A>(
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
            value: Box::new(value),
        })
}

pub fn shorthand_property<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    property().map(|name| Assignment {
        name,
        value: Box::new(Ast::Expression(Expression::Reference(name))),
    })
}

pub fn map_assignment<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    full_property(ast.clone()).or(shorthand_property())
}

pub fn expression_map<'tok, 'src: 'tok, I, A>(
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

pub fn expression_product<'tok, 'src: 'tok, I, A>(
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

pub fn expression_sum<'tok, 'src: 'tok, I, A>(
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
