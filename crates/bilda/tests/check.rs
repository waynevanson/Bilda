use std::process::Command;

fn bin() -> Command {
    let path = env!("CARGO_BIN_EXE_bilda");
    Command::new(path)
}
