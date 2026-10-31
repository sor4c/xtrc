//! Public library API contract tests.
//!
//! // @trace:test SPEC-ERR-HIERARCHY

use std::error::Error;
use std::fs;

use tempfile::tempdir;
use xtrc::config::WorkspaceConfig;
use xtrc::{check, load_workspace, CliError};

#[test]
fn workspace_load_and_check_are_available_through_the_public_api() -> Result<(), Box<dyn Error>> {
    let root = tempdir()?;
    let workspace = load_workspace(Some(root.path()), None)?;
    let report = check(&workspace)?;

    assert_eq!(workspace.root(), root.path());
    assert_eq!(workspace.config(), &WorkspaceConfig::default());
    assert_eq!(report.command, "check");
    assert_eq!(report.status, "pass");
    Ok(())
}

#[test]
fn strict_workspace_configuration_rejects_an_empty_graph() -> Result<(), Box<dyn Error>> {
    let root = tempdir()?;
    fs::write(
        root.path().join(".xtrc.toml"),
        "[validation]\nstrict = true\n",
    )?;
    let workspace = load_workspace(Some(root.path()), None)?;

    assert!(matches!(
        check(&workspace),
        Err(CliError::VerificationFailure { violations: 1 })
    ));
    Ok(())
}

#[test]
fn check_honors_strictness_for_validation_warnings() -> Result<(), Box<dyn Error>> {
    let root = tempdir()?;
    let docs = root.path().join("docs");
    fs::create_dir_all(&docs)?;
    fs::write(
        docs.join("requirement.md"),
        "---\ndocument_type: specification\nid: REQ-PUBLIC-API\ntitle: Public API check\nversion: 1.0.0\nstatus: approved\n---\n\n### REQ-PUBLIC-API: Public API check\n- **ID**: REQ-PUBLIC-API\n- **Type**: Requirement\n",
    )?;
    fs::write(
        root.path().join(".xtrc.toml"),
        "[validation]\nstrict = false\n",
    )?;

    let workspace = load_workspace(Some(root.path()), None)?;
    let report = check(&workspace)?;
    assert_eq!(report.status, "pass");
    assert!(report.violations.iter().any(|violation| matches!(
        violation,
        xtrc::report::ValidationViolation::OrphanNode { .. }
    )));

    fs::write(
        root.path().join(".xtrc.toml"),
        "[validation]\nstrict = true\n",
    )?;
    let workspace = load_workspace(Some(root.path()), None)?;
    assert!(matches!(
        check(&workspace),
        Err(CliError::VerificationFailure { .. })
    ));
    Ok(())
}
