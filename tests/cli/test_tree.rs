// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn setup_tree_workspace(dir: &std::path::Path) {
    let config = r#"version = "1"
[project]
name = "tree-test"
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
id: SPEC-TREE-001
title: Tree Spec
version: 1.0.0
status: approved
---

### VIS-TREE: Tree Root Vision
- **ID**: VIS-TREE
- **Parent**: None
- **Type**: Vision

### STRAT-TREE: Tree Strategy
- **ID**: STRAT-TREE
- **Parent**: VIS-TREE
- **Type**: Strategy

### EPIC-TREE: Tree Epic
- **ID**: EPIC-TREE
- **Parent**: STRAT-TREE
- **Type**: Epic

### REQ-TREE-01: Tree Requirement
- **ID**: REQ-TREE-01
- **Parent**: EPIC-TREE
- **Type**: Requirement
"#;
    std::fs::write(dir.join("docs/spec.md"), spec).expect("write spec");

    let code = r#"//! Source code
// @@trace REQ-TREE-01
pub fn tree_code() {}
"#;
    std::fs::write(dir.join("src/lib.rs"), code.replace("@@trace", "@trace")).expect("write code");
}

#[test]
fn test_tree_visualizes_from_vision_root() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .arg("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("VIS-TREE: Tree Root Vision"))
        .stdout(predicate::str::contains("STRAT-TREE: Tree Strategy"))
        .stdout(predicate::str::contains("EPIC-TREE: Tree Epic"))
        .stdout(predicate::str::contains("REQ-TREE-01: Tree Requirement"));
}

#[test]
fn test_tree_anchored_at_specific_id() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["tree", "EPIC-TREE"])
        .assert()
        .success()
        .stdout(predicate::str::contains("EPIC-TREE: Tree Epic"))
        .stdout(predicate::str::contains("REQ-TREE-01: Tree Requirement"))
        .stdout(predicate::str::contains("VIS-TREE").not());
}

#[test]
fn test_tree_reverse_upward_traversal() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["tree", "REQ-TREE-01", "--reverse"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REQ-TREE-01: Tree Requirement"))
        .stdout(predicate::str::contains("EPIC-TREE: Tree Epic"))
        .stdout(predicate::str::contains("VIS-TREE: Tree Root Vision"));
}

#[test]
fn test_tree_depth_limit() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["tree", "--depth", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("VIS-TREE"))
        .stdout(predicate::str::contains("STRAT-TREE"))
        .stdout(predicate::str::contains("REQ-TREE-01").not());
}

#[test]
fn test_tree_invalid_id_fails_exit_code_one() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["tree", "NON-EXISTENT-ID"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found in traceability graph"));
}

#[test]
fn test_tree_reverse_and_down_conflict_exits_code_two() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["tree", "--reverse", "--down"])
        .assert()
        .code(2);
}

#[test]
fn test_tree_json_output() {
    let temp = tempdir().expect("tempdir");
    setup_tree_workspace(temp.path());

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd
        .current_dir(temp.path())
        .args(["tree", "--json"])
        .assert()
        .success();

    let output = assert.get_output();
    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("stdout must be valid JSON");
    assert_eq!(val["command"], "tree");
    assert_eq!(val["schema_version"], "1.0.0");
    assert!(val.get("tree").is_some());
}
