use std::fs;
use std::process;

use bilda::jit::compile;
use bilda::lexer::Token;
use bilda::parser::ast;
use bilda::runtime::bilda_print;
use chumsky::Parser;
use chumsky::input::Stream;
use clap::{Parser as ClapParser, Subcommand};
use logos::Logos;

#[derive(ClapParser)]
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

fn read_source(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", path);
        process::exit(1);
    })
}

fn lex(source: &str) -> Vec<Token<'_>> {
    Token::lexer(source)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| {
            eprintln!("lex error");
            process::exit(1);
        })
}

fn check(path: &str) {
    let _source = read_source(path);
    let _tokens = lex(&_source);
}

fn run(path: &str) {
    let source = read_source(path);
    let tokens = lex(&source);
    let ast = ast()
        .parse(Stream::from_iter(tokens))
        .into_result()
        .unwrap_or_else(|_| {
            eprintln!("parse error");
            process::exit(1);
        });

    let compiled = compile(&ast).unwrap_or_else(|e| {
        eprintln!("compile error: {e:?}");
        process::exit(1);
    });

    let result = compiled.run();
    unsafe {
        bilda_print(result);
    }
    println!();
}
