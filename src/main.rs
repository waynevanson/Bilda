use std::{io::Lines, ops::Index, range::Range, str::Split};

struct Tokenizer<'input> {
    input: &'input str,
    position: usize,
}

struct Rule {
    str: &'static str,
    mutiple: bool,
}

const RULES: [(&str, Token<'_>); 13] = [
    ("(", Token::BracketLeft),
    (")", Token::BracketRight),
    (",", Token::Comma),
    ("{", Token::CurlyLeft),
    ("}", Token::CurlyRight),
    (".", Token::DotSingle),
    (":", Token::DotDouble),
    ("=", Token::Equal),
    ("+", Token::Plus),
    ("[", Token::SquareLeft),
    ("]", Token::SquareRight),
    ("*", Token::Star),
    ("~", Token::Tilde),
];

impl<'input> Iterator for Tokenizer<'input> {
    type Item = Result<Token<'input>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        // single char matches
        for (chars, token) in RULES {
            let size = chars.len();

            if self.input.get(self.position..size)? == chars {
                self.position += size;
                return Some(Ok(token));
            }
        }

        // repeat matches for new lines

        None
    }
}

enum Token<'input> {
    ArrowLeft,
    ArrowRight,
    BracketLeft,
    BracketRight,
    Comma,
    CurlyLeft,
    CurlyRight,
    DotSingle,
    DotDouble,
    Equal,
    Plus,
    Newline,
    SquareLeft,
    SquareRight,
    Star,
    Tilde,
    Unknown(&'input str),
}

fn main() {
    println!("Hello, world!");
}
