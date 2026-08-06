use logos::Logos;

#[derive(Clone, Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\r\f]+")]
#[logos(skip(r"#[^\n]*", allow_greedy = true))]
#[allow(dead_code)]
pub enum Token {
    #[token("let")]
    Let,

    #[token("expr")]
    Expr,

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

    #[regex("[0-9]+", |lex| lex.slice().to_string())]
    Number(String),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}

#[cfg(test)]
mod test {

    use super::*;

    fn compute(input: &str) -> Vec<Token> {
        let input = input.trim();
        Result::<Vec<Token>, ()>::from_iter(Token::lexer(input)).unwrap()
    }

    fn id(s: &str) -> Token {
        Token::Ident(s.to_string())
    }

    fn num(s: &str) -> Token {
        Token::Number(s.to_string())
    }

    #[test]
    fn simple() {
        let input = r#"
            Name = String
        "#;

        let tokens = compute(input);
        let expected = vec![id("Name"), Token::Equal, id("String")];

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
            id("Name"),
            Token::Equal,
            id("String"),
            Token::Newline,
            id("User"),
            Token::Equal,
            id("Agent"),
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
            id("Name"),
            Token::Equal,
            Token::Star,
            Token::CurlyLeft,
            Token::Newline,
            id("Fire"),
            Token::Equal,
            id("String"),
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
            vec![id("Thing"), Token::Equal, Token::Tilde, id("String")]
        );
    }

    #[test]
    fn sum_type() {
        let tokens = compute("Name = + { First = String Second = String }");
        assert_eq!(
            tokens,
            vec![
                id("Name"),
                Token::Equal,
                Token::Plus,
                Token::CurlyLeft,
                id("First"),
                Token::Equal,
                id("String"),
                id("Second"),
                Token::Equal,
                id("String"),
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
                id("Name"),
                Token::Equal,
                Token::Star,
                Token::CurlyLeft,
                id("First"),
                Token::Equal,
                id("String"),
                id("Second"),
                Token::Equal,
                id("String"),
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
                id("f"),
                Token::Equal,
                id("Name"),
                Token::DoubleColon,
                id("First"),
            ]
        );
    }

    #[test]
    fn value_access() {
        let tokens = compute("f = Name.First");
        assert_eq!(
            tokens,
            vec![
                id("f"),
                Token::Equal,
                id("Name"),
                Token::DotSingle,
                id("First"),
            ]
        );
    }

    #[test]
    fn function() {
        let tokens = compute("f = x y => x * y");
        assert_eq!(
            tokens,
            vec![
                id("f"),
                Token::Equal,
                id("x"),
                id("y"),
                Token::Arrow,
                id("x"),
                Token::Star,
                id("y"),
            ]
        );
    }

    #[test]
    fn minus_type() {
        let tokens = compute("Name = - { First = {} Second = {} }");
        assert_eq!(
            tokens,
            vec![
                id("Name"),
                Token::Equal,
                Token::Minus,
                Token::CurlyLeft,
                id("First"),
                Token::Equal,
                Token::CurlyLeft,
                Token::CurlyRight,
                id("Second"),
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
        assert_eq!(tokens, vec![id("age"), Token::Equal, num("32")]);
    }

    #[test]
    fn comment_is_skipped() {
        let tokens = compute("# this is a comment\nName = String");
        assert_eq!(
            tokens,
            vec![Token::Newline, id("Name"), Token::Equal, id("String")]
        );
    }

    #[test]
    fn let_expr_block() {
        let input = r#"
            let
                Name = String
            expr
                Name
        "#;

        let tokens = compute(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Newline,
                id("Name"),
                Token::Equal,
                id("String"),
                Token::Newline,
                Token::Expr,
                Token::Newline,
                id("Name"),
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
            expr
              Name1
        "#;

        let tokens = compute(input);
        assert!(tokens.contains(&Token::Let));
        assert!(tokens.contains(&Token::Expr));
        assert!(tokens.contains(&Token::Tilde));
        assert!(tokens.contains(&Token::Plus));
        assert!(tokens.contains(&Token::Star));
        assert!(tokens.contains(&Token::DoubleColon));
        assert!(tokens.contains(&Token::DotSingle));
        assert!(tokens.contains(&Token::Arrow));
        assert!(tokens.iter().any(|t| matches!(t, Token::Number(_))));
    }
}
