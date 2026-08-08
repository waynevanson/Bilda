use std::process::Command;

fn bin() -> Command {
    let path = env!("CARGO_BIN_EXE_bilda");
    Command::new(path)
}

#[test]
fn check_good_file_prints_ok() {
    let output = bin()
        .arg("check")
        .arg("tests/fixtures/good.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
}

#[test]
fn check_typed_file_prints_ok() {
    let output = bin()
        .arg("check")
        .arg("tests/fixtures/typed_simple.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
}

#[test]
fn check_echoehco_file_prints_ok() {
    let output = bin()
        .arg("check")
        .arg("tests/fixtures/echoehco.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
}

#[test]
fn check_bad_file_fails() {
    let output = bin()
        .arg("check")
        .arg("tests/fixtures/bad.bilda")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Newline"));
}

#[test]
fn check_missing_file_fails() {
    let output = bin()
        .arg("check")
        .arg("tests/fixtures/missing.bilda")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn run_good_file_prints_value() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/good.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn run_echoehco_file_prints_value() {
    let output = bin()
        .arg("run")
        .arg("tests/fixtures/echoehco.bilda")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "32");
}
