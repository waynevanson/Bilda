use std::fs;
use std::process;

use bilda::lexer::Token;
use bilda::parser;
use bilda::runtime;
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
    Jit { file: String },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Check { file } => check(&file),
        Command::Run { file } => run(&file),
        Command::Jit { file } => jit(&file),
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

    match runtime::run(&ast) {
        Ok(runtime::Value::Unit) => {}
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("runtime error: {error}");
            process::exit(1);
        }
    }
}

fn jit(path: &str) {
    let ast = parse(path);

    let func = bilda::compiler::Compiler::new()
        .and_then(|mut compiler| compiler.compile(&ast))
        .unwrap_or_else(|error| {
            eprintln!("jit error: {error}");
            process::exit(1);
        });

    let (value, tag) = func();
    if tag != 0 {
        println!("{value}");
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
