use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn write_temp_python_binary_search(test_name: &str) -> PathBuf {
    let source = r#"
def binary_search(nums, target):
    left, right = 0, len(nums) - 1
    while left <= right:
        mid = (left + right) // 2
        if nums[mid] == target:
            return mid
        elif nums[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
"#;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "algosketch-{test_name}-{}-{unique}.py",
        std::process::id()
    ));
    fs::write(&path, source).unwrap();
    path
}

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
fn default_output_includes_pseudocode_and_explanation_en() {
    let fixture = write_temp_python_binary_search("default-output-en");
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(&fixture)
        .arg("--lang")
        .arg("en")
        .assert()
        .success()
        .stdout(contains("## binary_search"))
        .stdout(contains("### Pseudocode"))
        .stdout(contains("FUNCTION binary_search"))
        .stdout(contains("### Explanation"))
        .stdout(contains("Purpose:"))
        .stdout(contains("Steps:"));

    fs::remove_file(fixture).unwrap();
}

#[test]
fn no_pseudo_outputs_explanation_only_zh() {
    let fixture = write_temp_python_binary_search("no-pseudo-zh");
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(&fixture)
        .arg("--no-pseudo")
        .arg("--lang")
        .arg("zh")
        .assert()
        .success()
        .stdout(contains("## binary_search"))
        .stdout(contains("### 解释"))
        .stdout(contains("函数 binary_search"))
        .stdout(contains("目的："))
        .stdout(contains("步骤："))
        .stdout(contains("FUNCTION binary_search").not());

    fs::remove_file(fixture).unwrap();
}

#[test]
fn no_explain_outputs_pseudocode_only() {
    let fixture = write_temp_python_binary_search("no-explain");
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(&fixture)
        .arg("--no-explain")
        .assert()
        .success()
        .stdout(contains("## binary_search"))
        .stdout(contains("FUNCTION binary_search"))
        .stdout(contains("Purpose:").not())
        .stdout(contains("目的：").not())
        .stdout(contains("### Explanation").not())
        .stdout(contains("### 解释").not());

    fs::remove_file(fixture).unwrap();
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

#[test]
fn help_shows_explanation_flags() {
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("--lang"))
        .stdout(contains("--no-pseudo"))
        .stdout(contains("--no-explain"));
}
