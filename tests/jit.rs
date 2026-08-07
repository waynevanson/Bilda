use std::process::Command;

fn bin() -> Command {
    let path = env!("CARGO_BIN_EXE_bilda");
    Command::new(path)
}

#[test]
fn run_good_file_prints_value_via_jit() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/good.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn run_arithmetic_file_prints_value_via_jit() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/arithmetic.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "14");
}

#[test]
fn run_echoehco_file_prints_value_via_jit() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/echoehco.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "32");
}

#[test]
fn run_lambda_file_prints_value() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/lambda.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn run_lambda_partial_file_prints_value() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/lambda_partial.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "15");
}

#[test]
fn run_lambda_capture_file_prints_value() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/lambda_capture.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "15");
}
