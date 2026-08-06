use std::fs;
use std::process;

use bilda::lexer::Token;
use bilda::parser;
use clap::{Parser, Subcommand};
use logos::Logos;

#[derive(Parser)]
#[command(name = "bilda")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check { file: String },
    Run { file: String },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Check { file } => check(&file),
        Command::Run { file } => run(&file),
    }
}

fn check(path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", path);
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

fn run(_path: &str) {
    todo!();
}
