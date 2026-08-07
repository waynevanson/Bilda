use std::process::Command;

fn bin() -> Command {
    let path = env!("CARGO_BIN_EXE_bilda");
    Command::new(path)
}

#[test]
fn jit_good_file_prints_value() {
    let output = bin()
        .arg("jit")
        .arg("tests/fixtures/good.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn jit_arithmetic_file_prints_value() {
    let output = bin()
        .arg("jit")
        .arg("tests/fixtures/arithmetic.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "14");
}

#[test]
fn jit_echoehco_file_prints_value() {
    let output = bin()
        .arg("jit")
        .arg("tests/fixtures/echoehco.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "32");
}
