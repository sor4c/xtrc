// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_init_creates_files_and_exits_zero() {
    let temp = tempdir().expect("tempdir failed");
    let target_path = temp.path().to_str().expect("path to str");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.args(["init", "--path", target_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized xtrc workspace"));

    assert!(temp.path().join(".xtrc.toml").is_file());
    assert!(temp.path().join("docs").is_dir());

    let content = std::fs::read_to_string(temp.path().join(".xtrc.toml")).expect("read config");
    assert!(content.contains("[project]"));
    assert!(content.contains("name = \"xtrc\""));
}

#[test]
fn test_init_fails_if_exists_without_force() {
    let temp = tempdir().expect("tempdir failed");
    let target_path = temp.path().to_str().expect("path to str");

    // Pre-create .xtrc.toml
    std::fs::write(temp.path().join(".xtrc.toml"), "version = \"1\"").expect("write file");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.args(["init", "--path", target_path])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already initialized"));
}

#[test]
fn test_init_overwrites_with_force() {
    let temp = tempdir().expect("tempdir failed");
    let target_path = temp.path().to_str().expect("path to str");

    // Pre-create .xtrc.toml
    std::fs::write(temp.path().join(".xtrc.toml"), "stale_content = true").expect("write file");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.args(["init", "--path", target_path, "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized xtrc workspace"));

    let content = std::fs::read_to_string(temp.path().join(".xtrc.toml")).expect("read config");
    assert!(content.contains("[project]"));
    assert!(!content.contains("stale_content"));
}

#[test]
fn test_init_quiet_suppresses_stdout() {
    let temp = tempdir().expect("tempdir failed");
    let target_path = temp.path().to_str().expect("path to str");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.args(["init", "--path", target_path, "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(temp.path().join(".xtrc.toml").is_file());
}

#[test]
fn test_init_in_cwd() {
    let temp = tempdir().expect("tempdir failed");

    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.current_dir(temp.path())
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized xtrc workspace"));

    assert!(temp.path().join(".xtrc.toml").is_file());
    assert!(temp.path().join("docs").is_dir());
}

#[test]
fn test_cli_no_args_prints_help_and_exits_zero() {
    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: xtrc [OPTIONS] [COMMAND]"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_cli_invalid_arg_prints_single_error_and_exits_two() {
    let mut cmd = Command::cargo_bin("xtrc").expect("find bin");
    let assert = cmd.arg("--invalid-flag-xyz").assert();
    let assert =
        assert
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains(
                "unexpected argument '--invalid-flag-xyz'",
            ));

    // Verify error diagnostic is not duplicated on stderr
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let count = stderr.matches("unexpected argument").count();
    assert_eq!(count, 1, "Error message must only appear once on stderr");
}
