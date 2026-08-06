use std::fs;
use std::process;

use bilda::lexer::Token;
use bilda::parser;
use clap::Parser;
use logos::Logos;

#[derive(Parser)]
#[command(name = "bilda")]
struct Args {
    #[arg(long, help = "Check syntax and exit")]
    check: bool,

    file: String,
}

fn main() {
    let args = Args::parse();

    if !args.check {
        todo!();
    }

    let source = fs::read_to_string(&args.file).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", args.file);
        process::exit(1);
    });

    let tokens: Vec<Token> = Token::lexer(&source)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| {
            eprintln!("lex error");
            process::exit(1);
        });

    match parser::parse(&tokens) {
        Ok(_) => {
            println!("ok");
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{error:?}");
            }
            process::exit(1);
        }
    }
}
