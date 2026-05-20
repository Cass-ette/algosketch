use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn binary_search_file_outputs_pseudocode() {
    let fixture = format!(
        "{}/tests/fixtures/binary_search.py",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture)
        .assert()
        .success()
        .stdout(contains("FUNCTION binary_search(nums, target)"))
        .stdout(contains("WHILE left ≤ right"))
        .stdout(contains("mid ← (left + right) DIV 2"))
        .stdout(contains("RETURN -1"));
}

#[test]
fn invalid_python_returns_parse_error() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg("-")
        .arg("--source-lang")
        .arg("python")
        .write_stdin("def f(:\n    pass\n")
        .assert()
        .code(2)
        .stderr(contains("parse error"));
}
