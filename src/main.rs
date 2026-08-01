use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\r\f]+")]
#[logos(skip(r"#[^\n]*", allow_greedy = true))]
#[allow(dead_code)]
enum Token {
    #[token("let")]
    Let,

    #[token("in")]
    In,

    #[token("expr")]
    Expr,

    #[token("where")]
    Where,

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

    #[token("::")]
    DoubleColon,

    #[token(":")]
    Colon,

    #[token("=>")]
    Arrow,

    #[token("=")]
    Equal,

    #[token("-")]
    Minus,

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

    #[regex("[0-9]+")]
    Number,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
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

    #[test]
    fn annotation() {
        let tokens = compute("Thing = ~ String");
        assert_eq!(
            tokens,
            vec![Token::Unknown, Token::Equal, Token::Tilde, Token::Unknown]
        );
    }

    #[test]
    fn sum_type() {
        let tokens = compute("Name = + { First = String Second = String }");
        assert_eq!(
            tokens,
            vec![
                Token::Unknown,
                Token::Equal,
                Token::Plus,
                Token::CurlyLeft,
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::CurlyRight,
            ]
        );
    }

    #[test]
    fn product_type() {
        let tokens = compute("Name = * { First = String Second = String }");
        assert_eq!(
            tokens,
            vec![
                Token::Unknown,
                Token::Equal,
                Token::Star,
                Token::CurlyLeft,
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::CurlyRight,
            ]
        );
    }

    #[test]
    fn type_access() {
        let tokens = compute("f = Name::First");
        assert_eq!(
            tokens,
            vec![
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::DoubleColon,
                Token::Unknown,
            ]
        );
    }

    #[test]
    fn value_access() {
        let tokens = compute("f = Name.First");
        assert_eq!(
            tokens,
            vec![
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::DotSingle,
                Token::Unknown,
            ]
        );
    }

    #[test]
    fn function() {
        let tokens = compute("f = x y => x * y");
        assert_eq!(
            tokens,
            vec![
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::Unknown,
                Token::Arrow,
                Token::Unknown,
                Token::Star,
                Token::Unknown,
            ]
        );
    }

    #[test]
    fn minus_type() {
        let tokens = compute("Name = - { First = {} Second = {} }");
        assert_eq!(
            tokens,
            vec![
                Token::Unknown,
                Token::Equal,
                Token::Minus,
                Token::CurlyLeft,
                Token::Unknown,
                Token::Equal,
                Token::CurlyLeft,
                Token::CurlyRight,
                Token::Unknown,
                Token::Equal,
                Token::CurlyLeft,
                Token::CurlyRight,
                Token::CurlyRight,
            ]
        );
    }

    #[test]
    fn comma() {
        let tokens = compute("Try { Name = String, Age = U32 } = { Ok = {} }");
        assert!(tokens.contains(&Token::Comma));
    }

    #[test]
    fn number() {
        let tokens = compute("age = 32");
        assert_eq!(
            tokens,
            vec![Token::Unknown, Token::Equal, Token::Number]
        );
    }

    #[test]
    fn comment_is_skipped() {
        let tokens = compute("# this is a comment\nName = String");
        assert_eq!(
            tokens,
            vec![Token::Newline, Token::Unknown, Token::Equal, Token::Unknown]
        );
    }

    #[test]
    fn let_in_block() {
        let input = r#"
            let
                Name = String
            in
                Name
        "#;

        let tokens = compute(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Newline,
                Token::Unknown,
                Token::Equal,
                Token::Unknown,
                Token::Newline,
                Token::In,
                Token::Newline,
                Token::Unknown,
            ]
        );
    }

    #[test]
    fn expr_where_block() {
        let input = r#"
            expr
                name
            where
                name = 2
        "#;

        let tokens = compute(input);
        assert_eq!(
            tokens,
            vec![
                Token::Expr,
                Token::Newline,
                Token::Unknown,
                Token::Newline,
                Token::Where,
                Token::Newline,
                Token::Unknown,
                Token::Equal,
                Token::Number,
            ]
        );
    }

    #[test]
    fn readme_example() {
        let input = r#"
            let
              Thing = ~ String
              Thang = String

              Name1 = + {
                First = Thing
                Second = Thang
              }

              Name2 = * {
                First = Thing
                Second = Thang
              }

              # type access
              f_oath = Name1::First

              # value access
              f_vark = Name2.First

              closure = x y => x * y
              clos = x => closure 3
              ure = clos 5
            in
              Name1

            expr
              thins
            where
              thins = 2
        "#;

        let tokens = compute(input);
        assert!(tokens.contains(&Token::Let));
        assert!(tokens.contains(&Token::In));
        assert!(tokens.contains(&Token::Expr));
        assert!(tokens.contains(&Token::Where));
        assert!(tokens.contains(&Token::Tilde));
        assert!(tokens.contains(&Token::Plus));
        assert!(tokens.contains(&Token::Star));
        assert!(tokens.contains(&Token::DoubleColon));
        assert!(tokens.contains(&Token::DotSingle));
        assert!(tokens.contains(&Token::Arrow));
        assert!(tokens.contains(&Token::Number));
    }
}
