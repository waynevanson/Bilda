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

fn run(path: &str) {
    let ast = parse(path);

    let func = bilda::compiler::Compiler::new()
        .and_then(|mut compiler| compiler.compile(&ast))
        .unwrap_or_else(|error| {
            eprintln!("run error: {error}");
            process::exit(1);
        });

    let (value, tag) = func();
    match tag {
        0 => {}
        1 => println!("{value}"),
        2 => println!("<closure>"),
        _ => println!("<unknown>"),
    }
}

fn parse(path: &str) -> bilda::ast::Ast {
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

    parser::parse(&tokens).unwrap_or_else(|errors| {
        for error in errors {
            eprintln!("{error:?}");
        }
        process::exit(1);
    })
}
