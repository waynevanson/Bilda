use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast::{Assignment, Ast, Expression, Map, Product, Sum};
use crate::lexer::Token;
use crate::parser::Extra;
use crate::parser::property::property;

fn unit_ast<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    empty().map(|_| Ast::Expression(Expression::Unit))
}

pub fn full_property<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    property()
        .then_ignore(just(Token::Equal))
        .then(ast.or(unit_ast()))
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

fn bare_type_field<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    property().map(|name| Assignment {
        name,
        value: Box::new(Ast::Expression(Expression::Unit)),
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

fn type_assignment<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    full_property(ast).or(bare_type_field())
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

fn expression_sum_fields<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::CurlyBracketLeft)
        .ignore_then(type_assignment(ast).repeated().collect())
        .then_ignore(just(Token::CurlyBracketRight))
        .map(|assignments| Expression::Sum(Sum { assignments }))
}

fn expression_product_fields<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::CurlyBracketLeft)
        .ignore_then(type_assignment(ast).repeated().collect())
        .then_ignore(just(Token::CurlyBracketRight))
        .map(|assignments| Expression::Product(Product { assignments }))
}

pub fn expression_sum<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::Sum)
        .or(just(Token::Asterisk))
        .ignore_then(expression_sum_fields(ast))
}

pub fn expression_product<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Expression<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    just(Token::Product)
        .or(just(Token::Plus))
        .ignore_then(expression_product_fields(ast))
}
