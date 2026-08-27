use core::num::{ParseFloatError, ParseIntError};
use core::str::FromStr;
use logos::Logos;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LexerError {
    #[default]
    InvalidToken,
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
}

impl From<ParseIntError> for LexerError {
    fn from(value: ParseIntError) -> Self {
        LexerError::ParseInt(value)
    }
}

impl From<ParseFloatError> for LexerError {
    fn from(value: ParseFloatError) -> Self {
        LexerError::ParseFloat(value)
    }
}

#[derive(Clone, Debug, Logos, PartialEq)]
#[logos(
    error = LexerError,
    // skip spaces
    skip r"\s+",
    // skip comments
    skip r"#[^\f\n\r\v]*"
)]
pub enum Token<'input> {
    // Keywords
    #[token("let")]
    Let,
    #[token("in")]
    In,
    #[token("sum")]
    Sum,
    #[token("product")]
    Product,

    // Brackets
    #[token("(")]
    RoundBracketLeft,
    #[token(")")]
    RoundBracketRight,
    #[token("{")]
    CurlyBracketLeft,
    #[token("}")]
    CurlyBracketRight,
    #[token("[")]
    SquareBracketLeft,
    #[token("]")]
    SquareBracketRight,

    // Primitives
    #[token("True")]
    True,
    #[token("False")]
    False,
    #[token("Int")]
    Int,
    #[token("String")]
    String,

    // Symbols
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
    #[token("++")]
    Concat,
    #[token("/")]
    ForwardSlash,
    #[token("*")]
    Asterisk,
    #[token("!")]
    Bang,
    #[token("\\")]
    Backslash,
    #[regex(r#""[^"]*""#, |lexer| {
        let slice = lexer.slice();
        &slice[1..slice.len() - 1]
    })]
    StringLiteral(&'input str),

    // Identifiers
    #[regex("[a-zA-Z][a-zA-Z0-9_]*")]
    Identifier(&'input str),

    #[regex(r"[\+\-]?[0-9]+", |lexer| isize::from_str(lexer.slice()))]
    Number(isize),

    #[regex(r#"[\+\-]?[0-9]+(\.[0-9]+)"#, |lexer| f64::from_str(lexer.slice()))]
    Float(f64),
}
