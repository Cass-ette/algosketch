use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

struct TempPythonFile {
    path: PathBuf,
}

impl TempPythonFile {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TempPythonFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn write_temp_python_file(test_name: &str, source: &str) -> TempPythonFile {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "algosketch-{test_name}-{}-{unique}.py",
        std::process::id()
    ));
    fs::write(&path, source).unwrap();
    TempPythonFile { path }
}

fn write_temp_python_binary_search(test_name: &str) -> TempPythonFile {
    write_temp_python_file(
        test_name,
        r#"
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
"#,
    )
}

#[test]
fn explains_only_in_chinese() {
    let fixture = format!("{}/fixtures/binary_search.py", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture).arg("--no-pseudo").arg("--lang").arg("zh");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"))
        .stdout(predicate::str::contains("目的："))
        .stdout(predicate::str::contains("步骤："))
        .stdout(predicate::str::contains("FUNCTION binary_search").not());
}

#[test]
fn outputs_pseudocode_and_explanation_by_default() {
    let fixture = format!("{}/fixtures/binary_search.py", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture).arg("--lang").arg("en");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("## binary_search"))
        .stdout(predicate::str::contains("### Pseudocode"))
        .stdout(predicate::str::contains("FUNCTION binary_search"))
        .stdout(predicate::str::contains("### Explanation"))
        .stdout(predicate::str::contains("Purpose:"))
        .stdout(predicate::str::contains("Steps:"));
}

#[test]
fn outputs_pseudocode_only() {
    let fixture = format!("{}/fixtures/binary_search.py", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture).arg("--no-explain");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("FUNCTION binary_search"))
        .stdout(predicate::str::contains("Purpose:").not())
        .stdout(predicate::str::contains("目的：").not());
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

    cmd.arg(fixture.path())
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
}

#[test]
fn no_pseudo_outputs_explanation_only_zh() {
    let fixture = write_temp_python_binary_search("no-pseudo-zh");
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture.path())
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
}

#[test]
fn no_explain_outputs_pseudocode_only() {
    let fixture = write_temp_python_binary_search("no-explain");
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    cmd.arg(fixture.path())
        .arg("--no-explain")
        .assert()
        .success()
        .stdout(contains("## binary_search"))
        .stdout(contains("FUNCTION binary_search"))
        .stdout(contains("Purpose:").not())
        .stdout(contains("目的：").not())
        .stdout(contains("### Explanation").not())
        .stdout(contains("### 解释").not());
}

#[test]
fn markdown_multi_function_output_uses_single_blank_line_between_sections() {
    let fixture = write_temp_python_file(
        "multi-function-spacing",
        r#"
def first():
    return 1

def second():
    return 2
"#,
    );
    let mut cmd = Command::cargo_bin("algosketch").unwrap();

    let output = cmd
        .arg(fixture.path())
        .arg("--lang")
        .arg("en")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();

    assert!(
        !output.contains("\n\n\n"),
        "output has excessive blank lines:\n{output}"
    );
    assert!(
        output.contains("END FUNCTION\n```\n\n### Explanation"),
        "pseudocode and explanation should be separated by one blank line:\n{output}"
    );
    assert!(
        output.contains("Steps:\n  1. Return 1\n\n## second"),
        "functions should be separated by one blank line:\n{output}"
    );
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

#[test]
fn detects_chinese_locale_from_lang() {
    let fixture = format!("{}/fixtures/binary_search.py", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture)
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("PSEUDOCODE_LANG")
        .env("LANG", "zh_CN.UTF-8");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"));
}

#[test]
fn pseudocode_lang_overrides_lang() {
    let fixture = format!("{}/fixtures/binary_search.py", env!("CARGO_MANIFEST_DIR"));
    let mut cmd = Command::cargo_bin("algosketch").unwrap();
    cmd.arg(fixture)
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env("LANG", "en_US.UTF-8")
        .env("PSEUDOCODE_LANG", "zh");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("函数 binary_search"));
}
