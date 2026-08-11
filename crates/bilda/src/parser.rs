use crate::{
    ast::{Assignment, AssignmentProperty, Ast, Expression, ReservedValue},
    lexer::Token,
};
use chumsky::{input::ValueInput, prelude::*};

pub fn ast<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Ast<'tok>, extra::Err<Rich<'tok, Token<'src>>>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan>,
{
    recursive(|ast| {
        let property = select! {
            Token::SnakeCase(name) => AssignmentProperty { name, r#type: None },
        };

        let assignment = property
            .then_ignore(just(Token::Equal))
            .then(ast.clone())
            .map(|(target, value)| Assignment {
                target,
                value: Box::new(value),
            });

        let let_in = just(Token::Let)
            .ignore_then(assignment.repeated().at_least(1).collect())
            .then_ignore(just(Token::In))
            .then(ast.clone())
            .map(|(assignments, expression)| Ast::LetIn {
                assignments,
                expression: Box::new(expression),
            });

        let reference = select! {
            Token::SnakeCase(name) => Ast::Expression(Expression::Reference(name)),
        };

        let number = select! {
            Token::Number(n) => Ast::Expression(Expression::ReservedValue(
                ReservedValue::Int(n.parse().unwrap()),
            )),
        };

        let atom = choice((number, reference));
        let sum = atom.clone().foldl(
            just(Token::Plus).ignore_then(atom.clone()).repeated(),
            |lhs, rhs| Ast::Expression(Expression::Add(Box::new(lhs), Box::new(rhs))),
        );

        choice((let_in, sum))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::{Parser, input::Stream};
    use logos::Logos;

    fn parse(source: &str) -> Result<Ast<'_>, Vec<Rich<'_, Token<'_>>>> {
        let tokens: Vec<Token<'_>> = Token::lexer(source)
            .collect::<Result<_, _>>()
            .expect("lex error");
        ast().parse(Stream::from_iter(tokens)).into_result()
    }

    #[test]
    fn nested_let() {
        let source = r#"
            let
              a = 2
              b = let
                c = 1
              in
                c
            in
              a + b
        "#;
        assert!(parse(source).is_ok());
    }
}
