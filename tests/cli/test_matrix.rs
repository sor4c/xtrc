// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn setup_matrix_workspace(dir: &std::path::Path) {
    let config = r#"version = "1"
[project]
name = "matrix-test"
root = "."

[specs]
include = ["docs/**/*.md"]

[code]
include = ["src/**/*.rs"]
extensions = [".rs"]

[tests]
include = ["tests/**/*.rs"]
"#;
    std::fs::write(dir.join(".xtrc.toml"), config).expect("write config");
    std::fs::create_dir_all(dir.join("docs")).expect("create docs");
    std::fs::create_dir_all(dir.join("src")).expect("create src");
    std::fs::create_dir_all(dir.join("tests")).expect("create tests");

    let spec = r#"---
document_type: requirements_specification
id: SPEC-MATRIX-001
title: Matrix Spec
version: 1.0.0
status: approved
---

### VIS-MATRIX: Matrix Vision
- **ID**: VIS-MATRIX
- **Parent**: None
- **Type**: Vision

### US-MATRIX: User Story Matrix
- **ID**: US-MATRIX
- **Parent**: VIS-MATRIX
- **Type**: User Story

### REQ-MATRIX-01: Covered Requirement
- **ID**: REQ-MATRIX-01
- **Parent**: US-MATRIX
- **Type**: Requirement

### SPEC-MATRIX-01: Technical Spec
- **ID**: SPEC-MATRIX-01
- **Parent**: REQ-MATRIX-01
- **Type**: TechnicalSpec

### REQ-MATRIX-02: Uncovered Requirement
- **ID**: REQ-MATRIX-02
- **Parent**: US-MATRIX
- **Type**: Requirement
"#;
    std::fs::write(dir.join("docs/spec.md"), spec).expect("write spec");

    let code = r#"//! Source code
// @@trace SPEC-MATRIX-01, REQ-MATRIX-01
pub fn matrix_fn() {}
"#;
    std::fs::write(dir.join("src/lib.rs"), code.replace("@@trace", "@trace")).expect("write code");

    let test = r#"//! Test code
// @@trace:test SPEC-MATRIX-01, REQ-MATRIX-01
#[test]
fn test_matrix() {}
"#;
    std::fs::write(
        dir.join("tests/test_matrix.rs"),
        test.replace("@@trace", "@trace"),
    )
    .expect("write test");
}

#[test]
fn test_matrix_default_table() {
    let temp = tempdir().expect("tempdir");
    setup_matrix_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("matrix")
        .assert()
        .success()
        .stdout(predicate::str::contains("Requirement ID"))
        .stdout(predicate::str::contains("REQ-MATRIX-01"))
        .stdout(predicate::str::contains("REQ-MATRIX-02"));
}

#[test]
fn test_matrix_markdown_format() {
    let temp = tempdir().expect("tempdir");
    setup_matrix_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["matrix", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "| Requirement ID | Title | Parent User Story | Child Technical Specification | Code Links | Test Links | Status |",
        ))
        .stdout(predicate::str::contains("| REQ-MATRIX-01 |"));
}

#[test]
fn test_matrix_csv_format() {
    let temp = tempdir().expect("tempdir");
    setup_matrix_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["matrix", "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Requirement ID,Title,Parent User Story,Child Technical Specification,Code Links,Test Links,Status",
        ))
        .stdout(predicate::str::contains(
            "REQ-MATRIX-01,Covered Requirement",
        ));
}

#[test]
fn test_matrix_columns_follow_configured_layers() {
    let temp = tempdir().expect("tempdir");
    let config = r#"layers = [{ name = "Requirement", prefix = "REQ" }]
[specs]
include = ["docs/**/*.md"]
[code]
include = ["src/**/*.rs"]
extensions = [".rs"]
[tests]
include = ["tests/**/*.rs"]
"#;
    std::fs::write(temp.path().join(".xtrc.toml"), config).expect("write config");
    std::fs::create_dir_all(temp.path().join("docs")).expect("create docs");
    std::fs::create_dir_all(temp.path().join("src")).expect("create src");
    std::fs::create_dir_all(temp.path().join("tests")).expect("create tests");
    std::fs::write(
        temp.path().join("docs/req.md"),
        r#"---
document_type: requirements_specification
id: REQ-SPEC-001
title: Requirements
version: 1.0.0
status: approved
---

### REQ-01: One requirement
- **Type**: Requirement
- **Parent**: None
"#,
    )
    .expect("write requirement");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["matrix", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "| Requirement ID | Title | Parents | Children | Code Links | Test Links | Status |",
        ))
        .stdout(predicate::str::contains("Parent US").not());
}

#[test]
fn test_matrix_json_format() {
    let temp = tempdir().expect("tempdir");
    setup_matrix_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd
        .current_dir(temp.path())
        .args(["matrix", "--json"])
        .assert()
        .success();

    let output = assert.get_output();
    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("stdout must be valid JSON");
    assert_eq!(val["command"], "matrix");
    assert_eq!(val["schema_version"], "1.0.0");
    assert!(val.get("rows").is_some());
}

#[test]
fn test_matrix_uncovered_filtering() {
    let temp = tempdir().expect("tempdir");
    setup_matrix_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["matrix", "--uncovered", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REQ-MATRIX-02"))
        .stdout(predicate::str::contains("REQ-MATRIX-01").not());
}

#[test]
fn test_matrix_unsupported_format_exits_code_two() {
    let temp = tempdir().expect("tempdir");
    setup_matrix_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["matrix", "--format", "invalid_fmt"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Unsupported output format"));
}
