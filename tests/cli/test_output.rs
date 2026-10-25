// @trace:test SPEC-CLI-OUTPUT, REQ-FMT-JSON, REQ-UNIX-STREAMS, REQ-EXIT-CODES

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn setup_output_workspace(dir: &std::path::Path) {
    let config = r#"version = "1"
[project]
name = "output-test"
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
id: SPEC-OUT-001
title: Output Spec
version: 1.0.0
status: approved
---

### VIS-OUT: Vision Root
- **ID**: VIS-OUT
- **Parent**: None
- **Type**: Vision

### REQ-OUT-01: Requirement Out
- **ID**: REQ-OUT-01
- **Parent**: VIS-OUT
- **Type**: Requirement

### SPEC-OUT-01: Technical Spec Out
- **ID**: SPEC-OUT-01
- **Parent**: REQ-OUT-01
- **Type**: TechnicalSpec
"#;
    std::fs::write(dir.join("docs/spec.md"), spec).expect("write spec");

    let code = r#"//! Code
// @@trace SPEC-OUT-01, REQ-OUT-01
pub fn f() {}
"#;
    std::fs::write(dir.join("src/lib.rs"), code.replace("@@trace", "@trace")).expect("write code");

    let test = r#"//! Test
// @@trace:test SPEC-OUT-01, REQ-OUT-01
#[test]
fn t() {}
"#;
    std::fs::write(dir.join("tests/t.rs"), test.replace("@@trace", "@trace")).expect("write test");
}

#[test]
fn test_stream_separation_stdout_pure_data() {
    let temp = tempdir().expect("tempdir");
    setup_output_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd
        .current_dir(temp.path())
        .args(["check", "--json"])
        .assert()
        .success();

    let output = assert.get_output();
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    // Must be valid JSON with zero extraneous logging
    let _: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("stdout must be strict pure JSON");
}

#[test]
fn test_stream_separation_stderr_diagnostics() {
    let temp = tempdir().expect("tempdir");
    setup_output_workspace(temp.path());

    // Add a cycle to generate stderr error diagnostic
    let cycle_spec = r#"
### REQ-C1: Cycle 1
- **ID**: REQ-C1
- **Parent**: REQ-C2
- **Type**: Requirement

### REQ-C2: Cycle 2
- **ID**: REQ-C2
- **Parent**: REQ-C1
- **Type**: Requirement
"#;
    let mut spec = std::fs::read_to_string(temp.path().join("docs/spec.md")).expect("read spec");
    spec.push_str(cycle_spec);
    std::fs::write(temp.path().join("docs/spec.md"), spec).expect("write spec");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "[ERROR] Cyclic dependency detected:",
        ));
}

#[test]
fn test_non_tty_pipe_no_ansi_colors() {
    let temp = tempdir().expect("tempdir");
    setup_output_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd.current_dir(temp.path()).arg("check").assert().success();

    let stdout_str = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout_str.contains("\x1b["));
}

#[test]
fn test_universal_json_schema_across_commands() {
    let temp = tempdir().expect("tempdir");
    setup_output_workspace(temp.path());

    // 1. Check
    let mut cmd1 = Command::cargo_bin("xtrc").expect("find bin");
    let a1 = cmd1
        .current_dir(temp.path())
        .args(["check", "--json"])
        .assert()
        .success();
    let j1: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&a1.get_output().stdout)).expect("json");
    assert_eq!(j1["schema_version"], "1.0.0");
    assert_eq!(j1["command"], "check");
    assert!(j1.get("statistics").is_some());

    // 2. Matrix
    let mut cmd2 = Command::cargo_bin("xtrc").expect("find bin");
    let a2 = cmd2
        .current_dir(temp.path())
        .args(["matrix", "--json"])
        .assert()
        .success();
    let j2: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&a2.get_output().stdout)).expect("json");
    assert_eq!(j2["schema_version"], "1.0.0");
    assert_eq!(j2["command"], "matrix");
    assert!(j2.get("statistics").is_some());

    // 3. Tree
    let mut cmd3 = Command::cargo_bin("xtrc").expect("find bin");
    let a3 = cmd3
        .current_dir(temp.path())
        .args(["tree", "--json"])
        .assert()
        .success();
    let j3: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&a3.get_output().stdout)).expect("json");
    assert_eq!(j3["schema_version"], "1.0.0");
    assert_eq!(j3["command"], "tree");
    assert!(j3.get("statistics").is_some());

    // 4. Export
    let mut cmd4 = Command::cargo_bin("xtrc").expect("find bin");
    let a4 = cmd4
        .current_dir(temp.path())
        .args(["export", "--format", "json"])
        .assert()
        .success();
    let j4: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&a4.get_output().stdout)).expect("json");
    assert_eq!(j4["schema_version"], "1.0.0");
    assert_eq!(j4["command"], "export");
    assert!(j4.get("statistics").is_some());
}
