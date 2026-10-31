// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY

use std::fs;
use tempfile::tempdir;
use xtrc::config::discovery::discover_workspace;
use xtrc::config::WorkspaceConfig;
use xtrc::error::ConfigError;

#[test]
fn test_find_project_root_from_cwd() {
    let temp = tempdir().expect("tempdir failed");
    let cfg_path = temp.path().join(".xtrc.toml");
    fs::write(&cfg_path, WorkspaceConfig::default_starter_toml()).expect("write config");

    let (root, config) =
        discover_workspace(temp.path(), None, None).expect("discover_workspace failed");
    assert_eq!(root, temp.path());
    assert_eq!(config.project.name, "xtrc");
    assert_eq!(config.version, "1");
}

#[test]
fn test_find_project_root_from_nested() {
    let temp = tempdir().expect("tempdir failed");
    let cfg_path = temp.path().join(".xtrc.toml");
    fs::write(&cfg_path, WorkspaceConfig::default_starter_toml()).expect("write config");

    let nested = temp.path().join("deep").join("nested").join("subfolder");
    fs::create_dir_all(&nested).expect("create_dir_all failed");

    let (root, config) =
        discover_workspace(&nested, None, None).expect("discover_workspace failed");
    assert_eq!(root, temp.path());
    assert_eq!(config.project.name, "xtrc");
}

#[test]
fn test_find_project_root_git_fallback() {
    let temp = tempdir().expect("tempdir failed");
    let git_dir = temp.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git failed");

    let (root, config) =
        discover_workspace(temp.path(), None, None).expect("discover_workspace failed");
    assert_eq!(root, temp.path());
    assert_eq!(config.project.name, "xtrc");
}

#[test]
fn test_config_override_precedence() {
    let temp = tempdir().expect("tempdir failed");
    let custom_cfg = temp.path().join("custom.toml");
    let custom_content = r#"
    version = "1"
    [project]
    name = "override-project"
    "#;
    fs::write(&custom_cfg, custom_content).expect("write custom config");

    let (root, config) =
        discover_workspace(temp.path(), None, Some(&custom_cfg)).expect("discover failed");
    assert_eq!(root, temp.path());
    assert_eq!(config.project.name, "override-project");
}

#[test]
fn test_root_override_precedence() {
    let temp = tempdir().expect("tempdir failed");
    let custom_root = temp.path().join("my_root");
    fs::create_dir_all(&custom_root).expect("create custom root");

    let cfg_path = custom_root.join(".xtrc.toml");
    let content = r#"
    version = "1"
    [project]
    name = "scoped-root-project"
    "#;
    fs::write(&cfg_path, content).expect("write config");

    let (root, config) =
        discover_workspace(temp.path(), Some(&custom_root), None).expect("discover failed");
    assert_eq!(root, custom_root);
    assert_eq!(config.project.name, "scoped-root-project");
}

#[test]
fn test_invalid_toml_error() {
    let temp = tempdir().expect("tempdir failed");
    let cfg_path = temp.path().join(".xtrc.toml");
    fs::write(&cfg_path, "not a valid [toml = syntax").expect("write bad toml");

    let err = discover_workspace(temp.path(), None, None).unwrap_err();
    match err {
        ConfigError::InvalidToml { path, .. } => {
            assert_eq!(path, cfg_path);
        }
        other => panic!("Expected ConfigError::InvalidToml, got {other:?}"),
    }
}
