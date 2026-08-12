use crate::{
    ast::{Assignment, Ast, Expression, Math, MathSign, MathTarget},
    lexer::Token,
};
use chumsky::{
    input::{BorrowInput, ValueInput},
    prelude::*,
};

pub fn ast<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Ast<'tok>, extra::Err<Rich<'tok, Token<'src>>>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + BorrowInput<'tok> + Input<'tok>,
{
    let math_sign = select! {
        Token::Minus => MathSign::Subtraction,
        Token::Plus => MathSign::Addition,
        Token::Asterisk => MathSign::Multiplication,
        Token::ForwardSlash => MathSign::Subtraction
    };

    let math_target = select_ref! {
      Token::Number(n) => MathTarget::Number(n),
      Token::Identifier(str) => MathTarget::Reference(str)
    };

    let math_brackets_off = math_target
        .then(math_sign)
        .then(math_target)
        .map(|((left, sign), right)| Math { left, sign, right });

    // todo: test: brackets, bodmas
    let math = recursive(|math| {
        let math_brackets_on = math.delimited_by(
            just(Token::RoundBracketLeft),
            just(Token::RoundBracketRight),
        );

        math_brackets_on.or(math_brackets_off)
    });

    // `in <expression>`
    let expression = select_ref! {
        Token::Number(n) => Expression::Int(n),
        Token::Identifier(name) => Expression::Reference(name),
        Token::True =>Expression::Boolean(true),
        Token::False => Expression::Boolean(false),
    }
    .or(math.map(Expression::Math));

    let property = select! {
        Token::Identifier(name) => name,
    };

    let equal = just(Token::Equal);

    let r#in = just(Token::In);

    recursive(|ast| {
        let assignment = property
            .then_ignore(equal)
            .then(ast.clone())
            .map(|(name, ast)| Assignment {
                name: name,
                r#type: None,
                value: Box::new(ast),
            });

        let assignments = assignment.repeated().at_least(1).collect();

        let let_in = just(Token::Let)
            .ignore_then(assignments)
            .then_ignore(r#in)
            .then(expression)
            .map(|(assignments, expression)| Ast::LetIn {
                assignments,
                expression: Box::new(expression),
            });

        let_in
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;
    use logos::Logos;

    fn parse(source: &str) -> Result<(Vec<Token<'_>>, Ast<'_>), Vec<Rich<'_, Token<'_>>>> {
        let tokens: Vec<Token<'_>> = Token::lexer(source)
            .collect::<Result<_, _>>()
            .expect("lex error");
        let result = ast().parse(tokens.as_slice()).into_result();
        result.map(|ast| (tokens, ast))
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
