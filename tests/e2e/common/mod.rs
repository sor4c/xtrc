//! Shared test helpers and fixture builders for xtrc E2E integration tests.
//!
//! // @trace:test SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-CLI-OUTPUT

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use tempfile::{tempdir, TempDir};

/// An isolated, temporary workspace for E2E testing.
pub struct TestWorkspace {
    dir: TempDir,
}

impl TestWorkspace {
    /// Creates a new, pristine temporary workspace.
    pub fn new() -> Self {
        let dir = tempdir().expect("Failed to create temporary directory for test");
        Self { dir }
    }

    /// Returns the root path of the temporary workspace.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Initializes a Git repository inside the workspace with standard user credentials.
    pub fn init_git(&self) {
        let status = std::process::Command::new("git")
            .arg("init")
            .current_dir(self.path())
            .status()
            .expect("Failed to run git init");
        assert!(status.success(), "git init failed");

        let _ = std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(self.path())
            .status();

        let _ = std::process::Command::new("git")
            .args(["config", "user.name", "Test Runner"])
            .current_dir(self.path())
            .status();
    }

    /// Writes a text file relative to the workspace root, creating parent directories if needed.
    pub fn write_file(&self, rel_path: impl AsRef<Path>, content: &str) -> PathBuf {
        let full_path = self.path().join(rel_path.as_ref());
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        fs::write(&full_path, content).expect("Failed to write file");
        full_path
    }

    /// Writes a binary file relative to the workspace root.
    pub fn write_binary_file(&self, rel_path: impl AsRef<Path>, bytes: &[u8]) -> PathBuf {
        let full_path = self.path().join(rel_path.as_ref());
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        fs::write(&full_path, bytes).expect("Failed to write binary file");
        full_path
    }

    /// Writes `.xtrc.toml` configuration at the workspace root.
    pub fn write_config(&self, content: &str) -> PathBuf {
        self.write_file(".xtrc.toml", content)
    }

    /// Writes default `.xtrc.toml` configuration at the workspace root.
    pub fn write_default_config(&self) -> PathBuf {
        let default_config = r#"version = "1"

[project]
name = "xtrc-test"
root = "."

[specs]
include = ["docs/**/*.md"]
exclude = [".agents/**", "target/**"]

[code]
include = ["src/**/*.rs", "lib/**/*.*"]
extensions = [".rs", ".py", ".go", ".ts", ".js", ".c", ".cpp"]

[tests]
include = ["tests/**/*.*"]

[validation]
strict = false
require_test_coverage = true
"#;
        self.write_config(default_config)
    }

    /// Spawns a CLI command initialized with `current_dir` set to the workspace root.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("xtrc").expect("Failed to locate xtrc binary");
        cmd.current_dir(self.path());
        cmd
    }

    /// Spawns a CLI command initialized with `current_dir` set to a nested subdirectory.
    pub fn cmd_in(&self, rel_subdir: impl AsRef<Path>) -> Command {
        let mut cmd = Command::cargo_bin("xtrc").expect("Failed to locate xtrc binary");
        cmd.current_dir(self.path().join(rel_subdir.as_ref()));
        cmd
    }

    /// Sets up a minimal 100% complete, fully valid 7-layer project.
    pub fn setup_valid_full_project(&self) {
        self.write_default_config();

        let spec_content = r#"---
document_type: requirements_specification
id: SPEC-DOC-001
title: Core Requirements
version: 1.0.0
status: approved
---

### VIS-VCS-NATIVE: Version Control Traceability
- **ID**: VIS-VCS-NATIVE
- **Parent**: None
- **Type**: Vision

### STRAT-PLAIN-TEXT: Plain Text Storage
- **ID**: STRAT-PLAIN-TEXT
- **Parent**: VIS-VCS-NATIVE
- **Type**: Strategy

### EPIC-CORE: Core Engine
- **ID**: EPIC-CORE
- **Parent**: STRAT-PLAIN-TEXT
- **Type**: Epic

### US-VERIFY: Verification Flow
- **ID**: US-VERIFY
- **Parent**: EPIC-CORE
- **Type**: User Story

### REQ-VERIFY-01: Bidirectional Verification
- **ID**: REQ-VERIFY-01
- **Parent**: US-VERIFY
- **Type**: Requirement

### SPEC-VERIFY-ENGINE: Validator Specification
- **ID**: SPEC-VERIFY-ENGINE
- **Parent**: REQ-VERIFY-01
- **Type**: TechnicalSpec
"#;
        self.write_file("docs/requirements.md", spec_content);

        let code_content = r#"//! Production source file.
// @@trace SPEC-VERIFY-ENGINE, REQ-VERIFY-01
pub fn execute_verification() -> bool {
    true
}
"#;
        self.write_file(
            "src/validator.rs",
            &code_content.replace("@@trace", "@trace"),
        );

        let test_content = r#"//! Automated test file.
// @@trace:test SPEC-VERIFY-ENGINE, REQ-VERIFY-01
#[test]
fn test_verification() {
    assert!(true);
}
"#;
        self.write_file(
            "tests/test_validator.rs",
            &test_content.replace("@@trace", "@trace"),
        );
    }

    /// Sets up a project containing a broken link (referencing a non-existent requirement ID).
    pub fn setup_project_with_broken_link(&self) {
        self.write_default_config();

        let spec_content = r#"---
document_type: requirements_specification
id: SPEC-DOC-001
title: Broken Link Spec
version: 1.0.0
status: approved
---

### VIS-VCS-NATIVE: Root Vision
- **ID**: VIS-VCS-NATIVE
- **Parent**: None
- **Type**: Vision

### REQ-BROKEN-01: Requirement with Missing Parent
- **ID**: REQ-BROKEN-01
- **Parent**: US-NON-EXISTENT
- **Type**: Requirement
"#;
        self.write_file("docs/requirements.md", spec_content);
    }

    /// Sets up a project containing a cyclic dependency.
    pub fn setup_project_with_cycle(&self) {
        self.write_default_config();

        let spec_content = r#"---
document_type: requirements_specification
id: SPEC-DOC-001
title: Cyclic Spec
version: 1.0.0
status: approved
---

### VIS-CYCLE-A: Node A
- **ID**: VIS-CYCLE-A
- **Parent**: VIS-CYCLE-B
- **Type**: Vision

### VIS-CYCLE-B: Node B
- **ID**: VIS-CYCLE-B
- **Parent**: VIS-CYCLE-A
- **Type**: Vision
"#;
        self.write_file("docs/cycle.md", spec_content);
    }

    /// Sets up a project containing an orphan node (non-root item without a parent link).
    pub fn setup_project_with_orphan(&self) {
        self.write_default_config();

        let spec_content = r#"---
document_type: requirements_specification
id: SPEC-DOC-001
title: Orphan Spec
version: 1.0.0
status: approved
---

### VIS-ROOT: Root Vision
- **ID**: VIS-ROOT
- **Parent**: None
- **Type**: Vision

### REQ-ORPHAN: Unlinked Requirement
- **ID**: REQ-ORPHAN
- **Parent**: None
- **Type**: Requirement
"#;
        self.write_file("docs/orphan.md", spec_content);
    }

    /// Sets up a project with incomplete coverage (requirement missing code and test fulfillment).
    pub fn setup_project_with_uncovered_req(&self) {
        self.write_default_config();

        let spec_content = r#"---
document_type: requirements_specification
id: SPEC-DOC-001
title: Uncovered Spec
version: 1.0.0
status: approved
---

### VIS-VCS-NATIVE: Root Vision
- **ID**: VIS-VCS-NATIVE
- **Parent**: None
- **Type**: Vision

### STRAT-TEST: Strategy
- **ID**: STRAT-TEST
- **Parent**: VIS-VCS-NATIVE
- **Type**: Strategy

### EPIC-TEST: Epic
- **ID**: EPIC-TEST
- **Parent**: STRAT-TEST
- **Type**: Epic

### US-TEST: User Story
- **ID**: US-TEST
- **Parent**: EPIC-TEST
- **Type**: User Story

### REQ-UNCOVERED-01: Requirement Lacking Code and Tests
- **ID**: REQ-UNCOVERED-01
- **Parent**: US-TEST
- **Type**: Requirement
"#;
        self.write_file("docs/requirements.md", spec_content);
    }
}

/// Executes a CLI command and returns its captured `Output`.
///
/// Every E2E assertion runs unconditionally; there is no skip path for
/// unimplemented subcommands, so a missing or renamed CLI surface fails loudly.
pub fn run_cli(cmd: &mut Command) -> Output {
    cmd.output().expect("Failed to execute command")
}

/// Validates that an output's stdout parses as valid JSON conforming to the universal JSON schema.
pub fn parse_stdout_json(output: &Output) -> serde_json::Value {
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout_str).unwrap_or_else(|e| {
        panic!(
            "Failed to parse stdout as valid JSON: {}\nStdout was:\n{}",
            e, stdout_str
        )
    })
}
