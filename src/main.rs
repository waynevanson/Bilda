use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\r\f]+")]
enum Token {
    #[token("<")]
    AngleLeft,

    #[token(">")]
    AngleRight,

    #[token("(")]
    BracketLeft,

    #[token(")")]
    BracketRight,

    #[token(",")]
    Comma,

    #[token("{")]
    CurlyLeft,

    #[token("}")]
    CurlyRight,

    #[token(".")]
    DotSingle,

    #[token(":")]
    Colon,

    #[token("=")]
    Equal,

    #[token("+")]
    Plus,

    #[token("[")]
    SquareLeft,

    #[token("]")]
    SquareRight,

    #[token("*")]
    Star,

    #[token("~")]
    Tilde,

    #[token("\n")]
    Newline,

    #[regex("[a-zA-Z0-9]+")]
    Unknown,
}

fn main() {}

#[cfg(test)]
mod test {

    use super::*;

    fn compute<'a>(input: &'a str) -> Vec<Token> {
        let input = input.trim();
        let lex = Token::lexer(input).into_iter();
        let tokens = Result::<Vec<Token>, ()>::from_iter(lex).unwrap();
        tokens
    }

    #[test]
    fn simple() {
        let input = r#"
            Name = String
        "#;

        let tokens = compute(input);
        let expected = vec![Token::Unknown, Token::Equal, Token::Unknown];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn multiline_assign() {
        let input = r#"
            Name = String
            User = Agent
        "#;

        let tokens = compute(input);
        let expected = vec![
            Token::Unknown,
            Token::Equal,
            Token::Unknown,
            Token::Newline,
            Token::Unknown,
            Token::Equal,
            Token::Unknown,
        ];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn types() {
        let input = r#"
            Name = * {
                Fire = String
            }
        "#;

        let tokens = compute(input);
        let expected = vec![
            Token::Unknown,
            Token::Equal,
            Token::Star,
            Token::CurlyLeft,
            Token::Newline,
            Token::Unknown,
            Token::Equal,
            Token::Unknown,
            Token::Newline,
            Token::CurlyRight,
        ];

        assert_eq!(tokens, expected)
    }
}
