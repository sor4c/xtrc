//! Tier 4 Real-World Application Workloads: Multi-layer lifecycle verification, monorepos, and CI pipelines.
//!
//! // @trace:test SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-CLI-VERIFY,
//! // @trace:test SPEC-CLI-MATRIX, SPEC-CLI-TREE, SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT,
//! // @trace:test SPEC-DAG-VALIDATE, SPEC-DAG-CYCLE, SPEC-DAG-STORAGE, SPEC-FMT-FRONTMATTER

use crate::common::{run_cli, TestWorkspace};

// ==============================================================================
// Real-World Workload 1: Complete Scaffold-to-Strict-Audit Lifecycle
// ==============================================================================

// @trace:test SPEC-DISC-INIT, SPEC-CLI-VERIFY, SPEC-CLI-MATRIX, SPEC-CLI-EXPORT
#[test]
fn test_t4_rw01_full_lifecycle_scaffold_to_strict_audit() {
    let ws = TestWorkspace::new();
    ws.init_git();

    // 1. Initialize workspace via CLI
    let mut init_cmd = ws.cmd();
    init_cmd.arg("init").assert().success();

    // 2. Author complete 6-layer specifications
    let spec = r#"---
document_type: requirements_specification
id: SPEC-LIFECYCLE-001
title: Product Lifecycle Specification
version: 1.0.0
status: approved
---

### VIS-PRODUCT: Cloud Traceability
- **ID**: VIS-PRODUCT
- **Parent**: None
- **Type**: Vision

### STRAT-STORAGE: Git-Native Storage
- **ID**: STRAT-STORAGE
- **Parent**: VIS-PRODUCT
- **Type**: Strategy

### EPIC-CORE-ENGINE: Core Graph Engine
- **ID**: EPIC-CORE-ENGINE
- **Parent**: STRAT-STORAGE
- **Type**: Epic

### US-GRAPH-QUERY: Querying the Graph
- **ID**: US-GRAPH-QUERY
- **Parent**: EPIC-CORE-ENGINE
- **Type**: User Story

### REQ-QUERY-TREE: Tree Visualizer
- **ID**: REQ-QUERY-TREE
- **Parent**: US-GRAPH-QUERY
- **Type**: Requirement

### SPEC-TREE-RENDER: Unicode Tree Renderer
- **ID**: SPEC-TREE-RENDER
- **Parent**: REQ-QUERY-TREE
- **Type**: TechnicalSpec
"#;
    ws.write_file("docs/product.md", spec);

    // 3. Implement production code with trace markers
    ws.write_file(
        "src/tree_renderer.rs",
        "// @trace SPEC-TREE-RENDER, REQ-QUERY-TREE\npub fn render_tree() {}\n",
    );

    // 4. Implement automated verification test with trace:test marker
    ws.write_file(
        "tests/test_tree_render.rs",
        "// @trace:test SPEC-TREE-RENDER, REQ-QUERY-TREE\n#[test] fn test_render() {}\n",
    );

    // 5. Run xtrc check --strict
    let mut check_cmd = ws.cmd();
    check_cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut check_cmd);
        assert!(output.status.success(), "Strict check must pass 100%");
    }

    // 6. Generate matrix report
    let mut matrix_cmd = ws.cmd();
    matrix_cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut matrix_cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-QUERY-TREE"));
    }

    // 7. Generate compliance export document
    let export_path = ws.path().join("audit/compliance.md");
    let export_str = export_path.to_str().expect("valid str");
    let mut export_cmd = ws.cmd();
    export_cmd.args(["export", "--output", export_str]);
    {
        let output = run_cli(&mut export_cmd);
        assert_eq!(output.status.code(), Some(0));
        assert!(export_path.is_file());
    }
}

// ==============================================================================
// Real-World Workload 2: Polyglot Microservices Monorepo
// ==============================================================================

// @trace:test SPEC-SCAN-COMMENTS, SPEC-DISC-TRAVERSAL, SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t4_rw02_polyglot_microservices_monorepo() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Create polyglot services and build outputs to ignore
    ws.write_file(".gitignore", "node_modules/\ndist/\n__pycache__/\n*.pyc\n");

    // Go microservice
    ws.write_file(
        "services/auth/main.go",
        "// @trace SPEC-VERIFY-ENGINE\npackage main\nfunc main() {}\n",
    );

    // TypeScript frontend
    ws.write_file(
        "services/web/src/index.ts",
        "// @trace SPEC-VERIFY-ENGINE\nexport const app = {};\n",
    );

    // Python data pipeline
    ws.write_file(
        "services/data/pipeline.py",
        "# @trace SPEC-VERIFY-ENGINE\ndef process(): pass\n",
    );

    // SQL migration
    ws.write_file(
        "migrations/001_init.sql",
        "-- @trace SPEC-VERIFY-ENGINE\nCREATE TABLE logs (id INT);\n",
    );

    // Build artifact that should be ignored
    ws.write_file(
        "services/web/dist/bundle.js",
        "// @trace REQ-SHOULD-BE-IGNORED-IN-BUILD-OUTPUT\n",
    );

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 3: CI Pull Request Broken Link Detection & Fix Flow
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, SPEC-DAG-VALIDATE, REQ-BROKEN-LINK, REQ-STRICT-GATE
#[test]
fn test_t4_rw03_ci_pull_request_broken_link_rejection_and_fix() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Step 1: CI baseline passes
    let mut cmd_pass = ws.cmd();
    cmd_pass.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd_pass);
        assert!(output.status.success());
    }

    // Step 2: PR introduces broken link in code
    let pr_file = ws.write_file(
        "src/feature_pr.rs",
        "// @trace REQ-UNAPPROVED-OR-TYPO\npub fn rogue() {}\n",
    );

    let mut cmd_fail = ws.cmd();
    cmd_fail.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd_fail);
        // CI gate rejects with code 1
        assert_eq!(output.status.code(), Some(1));
    }

    // Step 3: Developer fixes typo in code link to match real REQ
    ws.write_file(
        &pr_file,
        "// @trace REQ-VERIFY-01\npub fn approved_feature() {}\n",
    );

    let mut cmd_fixed = ws.cmd();
    cmd_fixed.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd_fixed);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 4: CI Pull Request Cycle Detection & Remediation
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t4_rw04_ci_pull_request_cycle_rejection() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // PR introduces circular dependency in requirements document
    let cycle_spec = r#"---
document_type: specification
id: SPEC-CYCLE-PR
title: Cycle PR Spec
version: 1.0.0
status: approved
---

### VIS-A: Node A
- **ID**: VIS-A
- **Parent**: VIS-B
- **Type**: Vision

### VIS-B: Node B
- **ID**: VIS-B
- **Parent**: VIS-A
- **Type**: Vision
"#;
    let cycle_file = ws.write_file("docs/cycle_pr.md", cycle_spec);

    let mut cmd_fail = ws.cmd();
    cmd_fail.arg("check");
    {
        let output = run_cli(&mut cmd_fail);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("VIS-A") || stderr.contains("VIS-B") || stderr.contains("->"));
    }

    // Remove cycle PR file
    std::fs::remove_file(cycle_file).expect("remove cycle file");

    let mut cmd_clean = ws.cmd();
    cmd_clean.arg("check");
    {
        let output = run_cli(&mut cmd_clean);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 5: CI Strict Gate Blocks Uncovered Requirement
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, SPEC-DAG-VALIDATE, REQ-FORWARD-COV, REQ-STRICT-GATE
#[test]
fn test_t4_rw05_ci_strict_gate_uncovered_requirement() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // New requirement added without implementation or tests
    let new_req_spec = r#"---
document_type: specification
id: SPEC-NEW-FEATURE
title: New Feature
version: 1.0.0
status: approved
---

### REQ-NEW-FEATURE: Unimplemented Feature
- **ID**: REQ-NEW-FEATURE
- **Parent**: US-VERIFY
- **Type**: Requirement
"#;
    ws.write_file("docs/new_req.md", new_req_spec);

    // Standard check passes with exit code 0 (summary mode)
    let mut cmd_default = ws.cmd();
    cmd_default.arg("check");
    {
        let output = run_cli(&mut cmd_default);
        assert_eq!(output.status.code(), Some(0));
    }

    // Strict CI gate fails with exit code 1
    let mut cmd_strict = ws.cmd();
    cmd_strict.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd_strict);
        assert_eq!(output.status.code(), Some(1));
    }

    // Developer fulfills requirement with code and test
    ws.write_file(
        "src/new_feature.rs",
        "// @trace REQ-NEW-FEATURE\npub fn feat() {}\n",
    );
    ws.write_file(
        "tests/test_new_feature.rs",
        "// @trace:test REQ-NEW-FEATURE\n#[test] fn test_feat() {}\n",
    );

    // Strict CI gate now passes
    let mut cmd_strict_pass = ws.cmd();
    cmd_strict_pass.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd_strict_pass);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 6: Scope Creep Orphan Code Detection
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, SPEC-DAG-VALIDATE, REQ-ORPHAN-DETECT, REQ-BACKWARD-JUST
#[test]
fn test_t4_rw06_scope_creep_orphan_code_detection() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Developer adds code referencing an ungrounded specification node
    let rogue_spec = r#"---
document_type: specification
id: SPEC-ROGUE
title: Rogue Spec
version: 1.0.0
status: approved
---

### REQ-ROGUE: Ungrounded Requirement
- **ID**: REQ-ROGUE
- **Parent**: None
- **Type**: Requirement
"#;
    ws.write_file("docs/rogue.md", rogue_spec);
    ws.write_file("src/rogue.rs", "// @trace REQ-ROGUE\npub fn rogue() {}");

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        // Orphan requirement lacking path to Vision blocks under strict mode
        assert_eq!(output.status.code(), Some(1));
    }
}

// ==============================================================================
// Real-World Workload 7: Compliance Audit Export Pipeline
// ==============================================================================

// @trace:test SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT, REQ-CMD-EXPORT
#[test]
fn test_t4_rw07_compliance_audit_export_pipeline() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let audit_dir = ws.path().join("compliance_audit");
    let md_path = audit_dir.join("traceability_matrix.md");
    let json_path = audit_dir.join("traceability_matrix.json");

    let md_str = md_path.to_str().expect("valid str");
    let json_str = json_path.to_str().expect("valid str");

    let mut cmd_md = ws.cmd();
    cmd_md.args(["export", "--format", "markdown", "--output", md_str]);
    {
        let output = run_cli(&mut cmd_md);
        assert_eq!(output.status.code(), Some(0));
        assert!(md_path.is_file());
    }

    let mut cmd_json = ws.cmd();
    cmd_json.args(["export", "--format", "json", "--output", json_str]);
    {
        let output = run_cli(&mut cmd_json);
        assert_eq!(output.status.code(), Some(0));
        assert!(json_path.is_file());
        let json_text = std::fs::read_to_string(&json_path).expect("read file");
        let parsed: serde_json::Value = serde_json::from_str(&json_text).expect("valid json");
        assert!(parsed.is_object() || parsed.is_array());
    }
}

// ==============================================================================
// Real-World Workload 8: Nested Git Submodules and Vendor Trees
// ==============================================================================

// @trace:test SPEC-DISC-TRAVERSAL, SPEC-CLI-VERIFY, REQ-IGNORE-RULES
#[test]
fn test_t4_rw08_nested_git_submodules_and_vendor_trees() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Create simulated submodule with its own .git and bad trace tags
    let submodule_dir = ws.path().join("vendor/thirdparty");
    std::fs::create_dir_all(submodule_dir.join(".git")).expect("create submodule git");
    ws.write_file(
        "vendor/thirdparty/lib.rs",
        "// @trace REQ-EXTERNAL-VENDOR-TAG\n",
    );

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        // Vendor submodules should not cause broken link errors in the parent repo
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 9: Multi-File Specification Layout
// ==============================================================================

// @trace:test SPEC-FMT-FRONTMATTER, SPEC-DAG-STORAGE, SPEC-CLI-VERIFY
#[test]
fn test_t4_rw09_large_scale_spec_hierarchy_verification() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    // Vision
    ws.write_file(
        "docs/vision.md",
        r#"---
document_type: specification
id: SPEC-V
title: Vision
version: 1.0.0
status: approved
---
### VIS-MAIN: Global Vision
- **ID**: VIS-MAIN
- **Parent**: None
- **Type**: Vision
"#,
    );

    // Strategy
    ws.write_file(
        "docs/strategy.md",
        r#"---
document_type: specification
id: SPEC-S
title: Strategy
version: 1.0.0
status: approved
---
### STRAT-SCALE: Scalability Strategy
- **ID**: STRAT-SCALE
- **Parent**: VIS-MAIN
- **Type**: Strategy
"#,
    );

    // Epic
    ws.write_file(
        "docs/epics/epic1.md",
        r#"---
document_type: specification
id: SPEC-E
title: Epic
version: 1.0.0
status: approved
---
### EPIC-PERF: Performance Epic
- **ID**: EPIC-PERF
- **Parent**: STRAT-SCALE
- **Type**: Epic
"#,
    );

    // Story
    ws.write_file(
        "docs/stories/us1.md",
        r#"---
document_type: specification
id: SPEC-U
title: User Story
version: 1.0.0
status: approved
---
### US-SUBSECOND: Subsecond Queries
- **ID**: US-SUBSECOND
- **Parent**: EPIC-PERF
- **Type**: User Story
"#,
    );

    // Requirement
    ws.write_file(
        "docs/requirements/req1.md",
        r#"---
document_type: specification
id: SPEC-R
title: Requirement
version: 1.0.0
status: approved
---
### REQ-SUBSECOND-EXEC: Subsecond Execution Budget
- **ID**: REQ-SUBSECOND-EXEC
- **Parent**: US-SUBSECOND
- **Type**: Requirement
"#,
    );

    // Implementation & Test
    ws.write_file(
        "src/perf.rs",
        "// @trace REQ-SUBSECOND-EXEC\npub fn fast() {}\n",
    );
    ws.write_file(
        "tests/test_perf.rs",
        "// @trace:test REQ-SUBSECOND-EXEC\n#[test] fn t_fast() {}\n",
    );

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 10: Refactoring Split Module Provenance Preservation
// ==============================================================================

// @trace:test SPEC-DAG-VALIDATE, SPEC-SCAN-COMMENTS, SPEC-CLI-VERIFY
#[test]
fn test_t4_rw10_refactoring_split_module_provenance_preservation() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Refactor single file validator.rs into submodules
    ws.write_file("src/validator/mod.rs", "pub mod broken;\npub mod cycle;\n");
    ws.write_file(
        "src/validator/broken.rs",
        "// @trace SPEC-VERIFY-ENGINE, REQ-VERIFY-01\npub fn check_broken() {}\n",
    );
    ws.write_file(
        "src/validator/cycle.rs",
        "// @trace SPEC-VERIFY-ENGINE, REQ-VERIFY-01\npub fn check_cycles() {}\n",
    );

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Real-World Workload 11: CI Verification, Packaging & SPDX SBOM Delivery Gates
// ==============================================================================

// @trace:test SPEC-CI-VERIFY, SPEC-CD-DELIVERY, SPEC-VER-POLICY, REQ-PKG-DIST, REQ-PKG-SBOM, REQ-CI-VERIFY, REQ-CD-MAIN, REQ-VER-POLICY
#[test]
fn test_t4_rw11_ci_cd_packaging_and_spdx_sbom_delivery_contracts() {
    let cargo_toml_raw =
        std::fs::read_to_string("Cargo.toml").expect("Cargo.toml must be readable");
    let cargo_toml: toml::Value =
        toml::from_str(&cargo_toml_raw).expect("Cargo.toml must be valid TOML");

    // (1) Verify standard Rust package & Debian / cargo-binstall metadata
    let deb_meta = cargo_toml
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("deb"))
        .expect("[package.metadata.deb] must be configured in Cargo.toml");
    assert!(
        deb_meta.get("assets").is_some(),
        "Debian package metadata must declare binary and doc assets"
    );

    let binstall_meta = cargo_toml
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("binstall"))
        .expect("[package.metadata.binstall] must be configured in Cargo.toml");
    assert_eq!(
        binstall_meta.get("pkg-fmt").and_then(|v| v.as_str()),
        Some("tgz")
    );

    // (5) Verify Continuous Verification GitHub Action (.github/workflows/ci.yml)
    let ci_raw = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");
    let ci_yaml: serde_yaml::Value =
        serde_yaml::from_str(&ci_raw).expect("ci.yml must be valid YAML");
    assert!(ci_yaml.get("on").is_some(), "ci.yml must define triggers");
    assert!(ci_raw.contains("cargo fmt --check"));
    assert!(ci_raw.contains("cargo clippy --all-targets --all-features -- -D warnings"));
    assert!(ci_raw.contains("cargo test --all-targets --all-features"));
    assert!(ci_raw.contains("check --strict"));

    // (3) & (4) Verify Head-of-Main Delivery & SPDX JSON SBOM GitHub Action (.github/workflows/delivery.yml)
    let delivery_raw = std::fs::read_to_string(".github/workflows/delivery.yml")
        .expect(".github/workflows/delivery.yml must exist");
    let delivery_yaml: serde_yaml::Value =
        serde_yaml::from_str(&delivery_raw).expect("delivery.yml must be valid YAML");
    let deliver_job = delivery_yaml
        .get("jobs")
        .and_then(|j| j.get("deliver"))
        .expect("delivery.yml must contain 'deliver' job");
    assert_eq!(
        deliver_job.get("needs").and_then(|n| n.as_str()),
        Some("verify"),
        "deliver job must depend on verify job passing first"
    );
    assert!(
        delivery_raw.contains("scripts/generate-sbom.sh")
            && delivery_raw.contains("xtrc.spdx.json"),
        "delivery.yml must generate an SPDX JSON SBOM"
    );
    assert!(
        delivery_raw.contains("cargo deb --no-build --deb-version")
            && delivery_raw.contains(".tar.gz")
            && delivery_raw.contains("scripts/compute-version.sh")
            && delivery_raw.contains("scripts/generate-homebrew-formula.sh")
            && delivery_raw.contains("xtrc.rb"),
        "delivery.yml must compute unique version and build .deb, .tar.gz, and Homebrew formula (xtrc.rb)"
    );

    // Verify scripts/compute-version.sh produces distinct versions for distinct commits
    let tmp_dir = tempfile::tempdir().expect("create temp dir for sbom and version test");
    let repo_dir = tmp_dir.path().join("ver_repo");
    std::fs::create_dir_all(&repo_dir).expect("create ver_repo");
    std::fs::write(
        repo_dir.join("Cargo.toml"),
        "[package]\nname = \"xtrc\"\nversion = \"0.1.0\"\n",
    )
    .expect("write Cargo.toml");

    let run_git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(args)
            .status()
            .expect("git command");
        assert!(status.success());
    };
    run_git(&["init", "-q"]);
    run_git(&["config", "user.email", "test@xtrc.dev"]);
    run_git(&["config", "user.name", "xtrc test"]);
    run_git(&["add", "Cargo.toml"]);
    run_git(&["commit", "-q", "-m", "commit 1"]);

    let v1_out = std::process::Command::new("bash")
        .arg("scripts/compute-version.sh")
        .arg(&repo_dir)
        .output()
        .expect("compute-version commit 1");
    assert!(v1_out.status.success());
    let v1 = String::from_utf8_lossy(&v1_out.stdout).trim().to_string();
    assert!(
        v1.starts_with("0.1.0-r1.g"),
        "Expected 0.1.0-r1.g<sha>, got {v1}"
    );

    std::fs::write(repo_dir.join("README.md"), "# rev 2\n").expect("write README");
    run_git(&["add", "README.md"]);
    run_git(&["commit", "-q", "-m", "commit 2"]);

    let v2_out = std::process::Command::new("bash")
        .arg("scripts/compute-version.sh")
        .arg(&repo_dir)
        .output()
        .expect("compute-version commit 2");
    assert!(v2_out.status.success());
    let v2 = String::from_utf8_lossy(&v2_out.stdout).trim().to_string();
    assert!(
        v2.starts_with("0.1.0-r2.g"),
        "Expected 0.1.0-r2.g<sha>, got {v2}"
    );
    assert_ne!(
        v1, v2,
        "Different commits must never share the same delivery version"
    );

    // Exact release tag v0.1.0 must emit clean 0.1.0
    run_git(&["tag", "v0.1.0"]);
    let v_tag_out = std::process::Command::new("bash")
        .arg("scripts/compute-version.sh")
        .arg(&repo_dir)
        .output()
        .expect("compute-version tagged");
    assert!(v_tag_out.status.success());
    let v_tag = String::from_utf8_lossy(&v_tag_out.stdout)
        .trim()
        .to_string();
    assert_eq!(v_tag, "0.1.0");

    // (3) Execute scripts/generate-sbom.sh with explicit unique version and validate the generated SPDX 2.3 JSON SBOM
    let sbom_path = tmp_dir.path().join("xtrc.spdx.json");
    let status = std::process::Command::new("bash")
        .arg("scripts/generate-sbom.sh")
        .arg(&sbom_path)
        .arg(&v2)
        .status()
        .expect("execute scripts/generate-sbom.sh");
    assert!(status.success(), "SBOM generator script must succeed");

    let sbom_raw = std::fs::read_to_string(&sbom_path).expect("read generated SPDX JSON SBOM");
    let sbom_json: serde_json::Value =
        serde_json::from_str(&sbom_raw).expect("SBOM must be valid JSON");
    assert_eq!(
        sbom_json.get("spdxVersion").and_then(|v| v.as_str()),
        Some("SPDX-2.3")
    );
    assert_eq!(
        sbom_json.get("dataLicense").and_then(|v| v.as_str()),
        Some("CC0-1.0")
    );
    let packages = sbom_json
        .get("packages")
        .and_then(|p| p.as_array())
        .expect("SPDX JSON must contain packages array");
    assert!(
        packages.iter().any(|p| {
            p.get("name").and_then(|n| n.as_str()) == Some("xtrc")
                && p.get("versionInfo").and_then(|v| v.as_str()) == Some(v2.as_str())
        }),
        "SPDX packages must include root package xtrc stamped with unique delivery version {v2}"
    );
    let relationships = sbom_json
        .get("relationships")
        .and_then(|r| r.as_array())
        .expect("SPDX JSON must contain relationships array");
    assert!(
        !relationships.is_empty(),
        "SPDX relationships must not be empty"
    );

    // (1) Execute scripts/generate-homebrew-formula.sh and validate the generated Homebrew formula (xtrc.rb)
    let dummy_tarball = tmp_dir
        .path()
        .join(format!("xtrc-v{v2}-x86_64-unknown-linux-gnu.tar.gz"));
    std::fs::write(&dummy_tarball, b"dummy-tarball-bytes").expect("write dummy tarball");
    let formula_path = tmp_dir.path().join("xtrc.rb");
    let brew_status = std::process::Command::new("bash")
        .arg("scripts/generate-homebrew-formula.sh")
        .arg(&v2)
        .arg(&dummy_tarball)
        .arg(&formula_path)
        .status()
        .expect("execute scripts/generate-homebrew-formula.sh");
    assert!(
        brew_status.success(),
        "Homebrew formula generator script must succeed"
    );
    let formula_raw = std::fs::read_to_string(&formula_path).expect("read generated xtrc.rb");
    assert!(formula_raw.contains("class Xtrc < Formula"));
    assert!(formula_raw.contains("bin.install \"xtrc\""));
    assert!(formula_raw.contains("doc.install \"README.md\", \"xtrc.spdx.json\""));
    assert!(formula_raw.contains(&format!("version \"{v2}\"")));
}
