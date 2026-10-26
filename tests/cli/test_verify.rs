// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn setup_valid_workspace(dir: &std::path::Path) {
    let config = r#"version = "1"
[project]
name = "verify-test"
root = "."

[specs]
include = ["docs/**/*.md"]

[code]
include = ["src/**/*.rs"]
extensions = [".rs"]

[tests]
include = ["tests/**/*.rs"]

[validation]
strict = false
"#;
    std::fs::write(dir.join(".xtrc.toml"), config).expect("write config");
    std::fs::create_dir_all(dir.join("docs")).expect("create docs");
    std::fs::create_dir_all(dir.join("src")).expect("create src");
    std::fs::create_dir_all(dir.join("tests")).expect("create tests");

    let spec = r#"---
document_type: requirements_specification
id: SPEC-VERIFY-001
title: Verify Test Spec
version: 1.0.0
status: approved
---

### VIS-VERIFY: Verification Vision
- **ID**: VIS-VERIFY
- **Parent**: None
- **Type**: Vision

### REQ-VERIFY-01: Verification Requirement
- **ID**: REQ-VERIFY-01
- **Parent**: VIS-VERIFY
- **Type**: Requirement

### SPEC-VERIFY-01: Verification Spec
- **ID**: SPEC-VERIFY-01
- **Parent**: REQ-VERIFY-01
- **Type**: TechnicalSpec
"#;
    std::fs::write(dir.join("docs/spec.md"), spec).expect("write spec");

    let code = r#"//! Source code
// @@trace SPEC-VERIFY-01, REQ-VERIFY-01
pub fn verify_fn() {}
"#;
    std::fs::write(dir.join("src/lib.rs"), code.replace("@@trace", "@trace")).expect("write code");

    let test = r#"//! Test code
// @@trace:test SPEC-VERIFY-01, REQ-VERIFY-01
#[test]
fn test_verify() {}
"#;
    std::fs::write(
        dir.join("tests/test_verify.rs"),
        test.replace("@@trace", "@trace"),
    )
    .expect("write test");
}

#[test]
fn test_check_clean_project_exits_zero() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Coverage Rate:"))
        .stdout(predicate::str::contains("100.0%"));
}

#[test]
fn test_verify_alias_equivalence() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("verify")
        .assert()
        .success()
        .stdout(predicate::str::contains("Coverage Rate:"));
}

#[test]
fn test_check_strict_fails_on_uncovered() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    // Add uncovered requirement
    let extra_spec = r#"
### REQ-UNCOVERED: Uncovered Requirement
- **ID**: REQ-UNCOVERED
- **Parent**: VIS-VERIFY
- **Type**: Requirement
"#;
    let mut current_spec =
        std::fs::read_to_string(temp.path().join("docs/spec.md")).expect("read spec");
    current_spec.push_str(extra_spec);
    std::fs::write(temp.path().join("docs/spec.md"), current_spec).expect("write spec");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["check", "--strict"])
        .assert()
        .code(1);
}

#[test]
fn test_check_layer_filter() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["check", "--layer", "Requirement"])
        .assert()
        .success();
}

#[test]
fn test_check_id_filter() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["check", "--id", "REQ-VERIFY-01"])
        .assert()
        .success();
}

#[test]
fn test_check_invalid_id_fails_exit_code_one() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["check", "--id", "REQ-NONEXISTENT"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found in traceability graph"));
}

#[test]
fn test_check_quiet_flag_suppresses_summary() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["check", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_check_json_output() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd
        .current_dir(temp.path())
        .args(["check", "--json"])
        .assert()
        .success();

    let output = assert.get_output();
    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("stdout must be valid JSON");
    assert_eq!(val["status"], "pass");
    assert_eq!(val["command"], "check");
    assert_eq!(val["schema_version"], "1.0.0");
    assert!(val.get("statistics").is_some());
}

#[test]
fn test_unconfigured_files_like_readme_not_scanned_for_trace_links() {
    let temp = tempdir().expect("tempdir");
    setup_valid_workspace(temp.path());

    // Add a README.md and a non-included script outside code.include / tests.include
    std::fs::write(
        temp.path().join("README.md"),
        "# README\nExample snippet:\n```rust\n// @trace REQ-VERIFY-01\n```\n",
    )
    .expect("write readme");

    std::fs::write(
        temp.path().join("script.py"),
        "# @trace REQ-VERIFY-01\nprint('hello')\n",
    )
    .expect("write script");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd
        .current_dir(temp.path())
        .args(["check", "--json"])
        .assert()
        .success();

    let output = assert.get_output();
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    // src/lib.rs has 2 code links
    // tests/test_verify.rs has 2 test links
    // README.md and script.py must NOT add any links because they are outside configured globs
    assert_eq!(val["statistics"]["code_links"], 2);
    assert_eq!(val["statistics"]["test_links"], 2);
}

#[test]
fn test_cli_argument_error_formatting() {
    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.args(["--unknown-flag"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "error: unexpected argument '--unknown-flag'",
        ));
}

#[test]
fn test_check_empty_graph_warns_and_exits_zero() {
    let temp = tempdir().expect("tempdir");
    let config = r#"version = "1"
[project]
name = "empty-test"
root = "."

[specs]
include = ["docs/**/*.md"]
"#;
    std::fs::write(temp.path().join(".xtrc.toml"), config).expect("write config");
    std::fs::create_dir_all(temp.path().join("docs")).expect("create docs");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "[WARN] Empty traceability graph: no specification nodes found matching spec include patterns: [docs/**/*.md]",
        ));
}

#[test]
fn test_check_empty_graph_strict_fails_with_exit_code_one() {
    let temp = tempdir().expect("tempdir");
    let config = r#"version = "1"
[project]
name = "empty-test"
root = "."

[specs]
include = ["docs/**/*.md"]
"#;
    std::fs::write(temp.path().join(".xtrc.toml"), config).expect("write config");
    std::fs::create_dir_all(temp.path().join("docs")).expect("create docs");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["check", "--strict"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "[ERROR] Empty traceability graph: no specification nodes found matching spec include patterns: [docs/**/*.md]",
        ));
}
