use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast::{Assignment, Ast, Expression, LetIn};
use crate::lexer::Token;
use crate::parser::Extra;
use crate::parser::expression::expression;

pub fn ast<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    recursive(|ast| {
        let expr = expression(ast.clone());
        let_in(ast.clone(), expr.clone()).or(expr.map(Ast::Expression))
    })
}

pub fn assignment<'tok, 'src: 'tok, I, A>(
    ast: A,
) -> impl Parser<'tok, I, Assignment<'src>, Extra<'tok, 'src>> + Clone
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
    A: Parser<'tok, I, Ast<'src>, Extra<'tok, 'src>> + Clone + 'tok,
{
    crate::parser::property::property()
        .then_ignore(just(Token::Equal))
        .then(ast)
        .map(|(name, value)| Assignment {
            name,
            value: Box::new(value),
        })
}

pub fn let_in<'tok, 'src: 'tok, I, A, E>(
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
