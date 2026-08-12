use crate::{
    ast::{Assignment, Ast, Expression, Math, MathSign, MathTarget},
    lexer::Token,
};
use chumsky::{input::ValueInput, prelude::*};

pub fn ast<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Ast<'tok>, extra::Err<Rich<'tok, Token<'src>>>>
where
    I: ValueInput<'tok, Token = Token<'src>, Span = SimpleSpan> + Input<'tok>,
{
    let math_sign = select! {
        Token::Minus => MathSign::Subtraction,
        Token::Plus => MathSign::Addition,
        Token::Asterisk => MathSign::Multiplication,
        Token::ForwardSlash => MathSign::Subtraction
    };

    let math_target = select! {
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
    let expression = math.map(Expression::Math).or(select! {
        Token::Number(n) => Expression::Int(n),
        Token::Identifier(name) => Expression::Reference(name),
        Token::True => Expression::Boolean(true),
        Token::False => Expression::Boolean(false),
    });

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
                name,
                r#type: None,
                value: Box::new(ast),
            });

        let assignments = assignment.repeated().at_least(1).collect();

        let let_in = just(Token::Let)
            .ignore_then(assignments)
            .then_ignore(r#in)
            .then(expression.clone())
            .map(|(assignments, expression)| Ast::LetIn {
                assignments,
                expression: Box::new(expression),
            });

        let_in.or(expression.map(Ast::Expression))
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
