// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn setup_export_workspace(dir: &std::path::Path) {
    let config = r#"version = "1"
[project]
name = "export-test"
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
id: SPEC-EXPORT-001
title: Export Spec
version: 1.0.0
status: approved
---

### VIS-EXPORT: Export Vision
- **ID**: VIS-EXPORT
- **Parent**: None
- **Type**: Vision

### REQ-EXPORT-01: Export Requirement
- **ID**: REQ-EXPORT-01
- **Parent**: VIS-EXPORT
- **Type**: Requirement

### SPEC-EXPORT-01: Technical Spec
- **ID**: SPEC-EXPORT-01
- **Parent**: REQ-EXPORT-01
- **Type**: TechnicalSpec
"#;
    std::fs::write(dir.join("docs/spec.md"), spec).expect("write spec");

    let code = r#"//! Source code
// @@trace SPEC-EXPORT-01, REQ-EXPORT-01
pub fn export_fn() {}
"#;
    std::fs::write(dir.join("src/lib.rs"), code.replace("@@trace", "@trace")).expect("write code");

    let test = r#"//! Test code
// @@trace:test SPEC-EXPORT-01, REQ-EXPORT-01
#[test]
fn test_export() {}
"#;
    std::fs::write(
        dir.join("tests/test_export.rs"),
        test.replace("@@trace", "@trace"),
    )
    .expect("write test");
}

#[test]
fn test_export_default_markdown_stdout() {
    let temp = tempdir().expect("tempdir");
    setup_export_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("export")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "# Traceability Compliance Audit Report",
        ))
        .stdout(predicate::str::contains("VIS-EXPORT"))
        .stdout(predicate::str::contains("REQ-EXPORT-01"));
}

#[test]
fn test_export_format_json() {
    let temp = tempdir().expect("tempdir");
    setup_export_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd
        .current_dir(temp.path())
        .args(["export", "--format", "json"])
        .assert()
        .success();

    let output = assert.get_output();
    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("stdout must be valid JSON");
    assert_eq!(val["command"], "export");
    assert_eq!(val["schema_version"], "1.0.0");
    assert!(val.get("nodes").is_some());
}

#[test]
fn test_export_format_csv() {
    let temp = tempdir().expect("tempdir");
    setup_export_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["export", "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "ID,Title,Layer,Parents,Children,Code Links,Test Links,Coverage Status",
        ))
        .stdout(predicate::str::contains("VIS-EXPORT"));
}

#[test]
fn test_export_output_file_atomic_write() {
    let temp = tempdir().expect("tempdir");
    setup_export_workspace(temp.path());
    let out_file = temp.path().join("audit/report.md");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["export", "--output", out_file.to_str().expect("str")])
        .assert()
        .success();

    assert!(out_file.is_file());
    let content = std::fs::read_to_string(&out_file).expect("read file");
    assert!(content.contains("# Traceability Compliance Audit Report"));
    assert!(content.contains("VIS-EXPORT"));
}

#[test]
fn test_export_file_content_matches_stdout() {
    let temp = tempdir().expect("tempdir");
    setup_export_workspace(temp.path());
    let out_file = temp.path().join("file.md");

    let mut cmd_out = Command::cargo_bin("xtrc").expect("find bin");
    cmd_out
        .current_dir(temp.path())
        .args(["export", "--output", out_file.to_str().expect("str")])
        .assert()
        .success();

    let file_content = std::fs::read_to_string(&out_file).expect("read file");

    let mut cmd_stdout = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd_stdout
        .current_dir(temp.path())
        .arg("export")
        .assert()
        .success();
    let stdout_content = String::from_utf8_lossy(&assert.get_output().stdout);

    assert_eq!(file_content.trim(), stdout_content.trim());
}

#[test]
fn test_export_unsupported_format_exits_code_two() {
    let temp = tempdir().expect("tempdir");
    setup_export_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["export", "--format", "pdf"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Unsupported output format"));
}

#[test]
fn test_export_empty_project() {
    let temp = tempdir().expect("tempdir");
    let config = r#"version = "1"
[project]
name = "empty-export"
root = "."
"#;
    std::fs::write(temp.path().join(".xtrc.toml"), config).expect("write config");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("export")
        .assert()
        .success();
}
