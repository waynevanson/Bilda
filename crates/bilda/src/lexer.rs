use logos::Logos;

#[derive(Clone, Debug, Logos, PartialEq)]
#[logos(
    // skip spaces
    skip r"\s+",
    // skip comments
    skip r"#[^\f\n\r\v]*"
)]
pub enum ExpressionContextToken {
    // Keywords
    #[token("let")]
    Let,
    #[token("do")]
    Do,
    #[token("on")]
    On,
    #[token("in")]
    In,

    // Brackets
    #[token("<")]
    AngleBracketLeft,
    #[token(">")]
    AngleBracketRight,
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
    BooleanTrue,
    #[token("False")]
    BooleanFalse,
    // #[token("String")]`
    // String,
    #[token("Int")]
    Int,
    #[token("String")]
    String,

    // Symbols
    #[token(".")]
    DotSingle,
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
    #[token(r#"\*"#)]
    Star,
    #[token(r#"""#)]
    Quotation,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,

    #[token("!")]
    Exclamation,

    #[token("_")]
    Underscore,

    // Identifiers
    #[regex("_?([A-Z][a-z]*)+_+")]
    CamelCase,

    #[regex("_?[a-z][a-z_?]*")]
    SnakeCase,

    #[regex("_?[a-z]([A-Z][a-z]*)?_+")]
    KebabCase,

    #[regex("[0-9]+")]
    Number,

    #[regex(r#"[0-9]+(\.[0-9]+)"#)]
    Float,

    #[regex(r#"(([\./])+([a-zA-Z0-9_.\(\)])+)+"#)]
    FilePath,

    // todo: assert valid glob expression
    #[regex(r#"'[^\f\n\r\t\v]+'"#)]
    GlobPath,
}
