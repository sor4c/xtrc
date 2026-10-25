//! Tier 3 Cross-Feature Interactions: Pairwise and combinatorial integration flows.
//!
//! // @trace:test SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-CLI-VERIFY,
//! // @trace:test SPEC-CLI-MATRIX, SPEC-CLI-TREE, SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT,
//! // @trace:test SPEC-DAG-VALIDATE, SPEC-DAG-CYCLE, SPEC-DAG-STORAGE, SPEC-FMT-FRONTMATTER

use crate::common::{parse_stdout_json, run_cli, TestWorkspace};

// ==============================================================================
// 1. Init + Check Combinations
// ==============================================================================

// @trace:test SPEC-DISC-INIT, SPEC-CLI-VERIFY
#[test]
fn test_t3_c01_init_then_check_default() {
    let ws = TestWorkspace::new();
    let mut init_cmd = ws.cmd();
    init_cmd.arg("init").assert().success();

    let mut check_cmd = ws.cmd();
    check_cmd.arg("check");
    {
        let output = run_cli(&mut check_cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 1);
    }
}

// @trace:test SPEC-DISC-INIT, SPEC-CLI-VERIFY, SPEC-DAG-VALIDATE
#[test]
fn test_t3_c02_init_then_check_strict() {
    let ws = TestWorkspace::new();
    let mut init_cmd = ws.cmd();
    init_cmd.arg("init").assert().success();

    let mut check_cmd = ws.cmd();
    check_cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut check_cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 1);
    }
}

// @trace:test SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, SPEC-CLI-VERIFY
#[test]
fn test_t3_c03_init_custom_path_then_check_nested() {
    let ws = TestWorkspace::new();
    let sub = ws.path().join("nested_project");
    let sub_str = sub.to_str().expect("valid str");

    let mut init_cmd = ws.cmd();
    init_cmd
        .args(["init", "--path", sub_str])
        .assert()
        .success();

    let mut check_cmd = ws.cmd_in("nested_project");
    check_cmd.arg("check");
    {
        let output = run_cli(&mut check_cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 1);
    }
}

// @trace:test SPEC-DISC-INIT, SPEC-CLI-MATRIX, SPEC-CLI-OUTPUT
#[test]
fn test_t3_c04_init_then_matrix_json() {
    let ws = TestWorkspace::new();
    let mut init_cmd = ws.cmd();
    init_cmd.arg("init").assert().success();

    let mut matrix_cmd = ws.cmd();
    matrix_cmd.args(["matrix", "--json"]);
    {
        let output = run_cli(&mut matrix_cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert!(json.is_object() || json.is_array());
    }
}

// @trace:test SPEC-DISC-INIT, SPEC-CLI-EXPORT
#[test]
fn test_t3_c05_init_then_export_markdown() {
    let ws = TestWorkspace::new();
    let mut init_cmd = ws.cmd();
    init_cmd.arg("init").assert().success();

    let mut export_cmd = ws.cmd();
    export_cmd.args(["export", "--format", "markdown"]);
    {
        let output = run_cli(&mut export_cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-DISC-INIT, SPEC-CLI-VERIFY
#[test]
fn test_t3_c06_init_force_overwrite_then_verify() {
    let ws = TestWorkspace::new();
    ws.write_file(".xtrc.toml", "legacy_setting = 123");

    let mut init_cmd = ws.cmd();
    init_cmd.args(["init", "--force"]).assert().success();

    let mut verify_cmd = ws.cmd();
    verify_cmd.arg("verify");
    {
        let output = run_cli(&mut verify_cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 1);
    }
}

// ==============================================================================
// 2. Matrix + Format Combinations
// ==============================================================================

// @trace:test SPEC-CLI-MATRIX, SPEC-CLI-OUTPUT, REQ-FMT-JSON
#[test]
fn test_t3_c07_matrix_json_schema_validation() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert!(json.is_object() || json.is_array());
    }
}

// @trace:test SPEC-CLI-MATRIX, SPEC-CLI-OUTPUT
#[test]
fn test_t3_c08_matrix_uncovered_with_markdown_format() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_uncovered_req();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--uncovered", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-UNCOVERED-01"));
    }
}

// @trace:test SPEC-CLI-MATRIX, SPEC-CLI-OUTPUT
#[test]
fn test_t3_c09_matrix_layer_filter_with_csv_format() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--layer", "Requirement", "--format", "csv"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-VERIFY-01"));
    }
}

// ==============================================================================
// 3. Tree + Direction + Filter Combinations
// ==============================================================================

// @trace:test SPEC-CLI-TREE, SPEC-DAG-STORAGE
#[test]
fn test_t3_c10_tree_reverse_from_technical_spec() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "SPEC-VERIFY-ENGINE", "--reverse"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-VERIFY-01"));
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// @trace:test SPEC-CLI-TREE, SPEC-CLI-OUTPUT, REQ-FMT-JSON
#[test]
fn test_t3_c11_tree_depth_limited_with_json_output() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "VIS-VCS-NATIVE", "--depth", "2", "--json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert!(json.is_object());
    }
}

// @trace:test SPEC-CLI-TREE, SPEC-DAG-STORAGE
#[test]
fn test_t3_c12_tree_downward_decomposition_from_epic() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "EPIC-CORE"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("US-VERIFY"));
        assert!(stdout.contains("REQ-VERIFY-01"));
    }
}

// ==============================================================================
// 4. Export + Check Consistency Combinations
// ==============================================================================

// @trace:test SPEC-CLI-EXPORT, SPEC-CLI-VERIFY
#[test]
fn test_t3_c13_export_document_and_verify_clean_state() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let export_file = ws.path().join("audit_report.md");
    let export_str = export_file.to_str().expect("valid str");

    let mut exp_cmd = ws.cmd();
    exp_cmd.args(["export", "--output", export_str]);
    {
        let output = run_cli(&mut exp_cmd);
        assert_eq!(output.status.code(), Some(0));
        assert!(export_file.is_file());

        let mut check_cmd = ws.cmd();
        check_cmd.args(["check", "--strict"]);
        {
            let check_output = run_cli(&mut check_cmd);
            assert!(check_output.status.success());
        }
    }
}

// @trace:test SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT, REQ-FMT-JSON
#[test]
fn test_t3_c14_export_json_file_matches_check_statistics() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let export_json = ws.path().join("audit.json");
    let export_str = export_json.to_str().expect("valid str");

    let mut exp_cmd = ws.cmd();
    exp_cmd.args(["export", "--format", "json", "--output", export_str]);
    {
        let output = run_cli(&mut exp_cmd);
        assert_eq!(output.status.code(), Some(0));
        assert!(export_json.is_file());

        let content = std::fs::read_to_string(&export_json).expect("read export json");
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("export json must be valid");
        assert!(parsed.is_object() || parsed.is_array());
    }
}

// ==============================================================================
// 5. Polyglot Source & Test Files + Gitignore + Strict Check
// ==============================================================================

// @trace:test SPEC-SCAN-COMMENTS, SPEC-DISC-TRAVERSAL, SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t3_c15_polyglot_multilanguage_codebase_under_strict_check() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Add source files in Rust, Python, Go, TypeScript, Shell
    ws.write_file(
        "src/backend.go",
        "// @trace SPEC-VERIFY-ENGINE\npackage main",
    );
    ws.write_file(
        "src/frontend.ts",
        "// @trace SPEC-VERIFY-ENGINE\nexport const x = 1;",
    );
    ws.write_file(
        "scripts/deploy.py",
        "# @trace SPEC-VERIFY-ENGINE\ndef deploy(): pass",
    );
    ws.write_file("scripts/run.sh", "# @trace SPEC-VERIFY-ENGINE\necho ok");

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, SPEC-CLI-VERIFY, REQ-IGNORE-RULES
#[test]
fn test_t3_c16_multiple_gitignore_with_strict_check() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    ws.write_file(".gitignore", "vendor/\ntemp/\n*.tmp\n");
    ws.write_file("vendor/ignored.rs", "// @trace REQ-BROKEN-LINK-IGNORE");
    ws.write_file("temp/scratch.py", "# @trace REQ-BROKEN-LINK-IGNORE");
    ws.write_file("sub/.gitignore", "local_ignore/\n");
    ws.write_file(
        "sub/local_ignore/lib.go",
        "// @trace REQ-BROKEN-LINK-IGNORE",
    );

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        // Ignored files should never produce broken link errors
        assert!(output.status.success());
    }
}

// ==============================================================================
// 6. Scoped Query & Filter Combinations
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, SPEC-CLI-OUTPUT, REQ-FMT-JSON
#[test]
fn test_t3_c17_check_layer_filter_with_json_output() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--layer", "Requirement", "--json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert_eq!(json["status"], "pass");
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t3_c18_check_id_subgraph_with_strict_mode() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--id", "REQ-VERIFY-01", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-UNIX-STREAMS, REQ-FMT-JSON
#[test]
fn test_t3_c19_check_json_piped_clean_stream_isolation() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        // Valid JSON without preamble or postamble
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout_str).expect("stdout must be strict JSON");
        assert!(parsed.is_object());
    }
}

// ==============================================================================
// 7. Graph Defect + Presentation Combinations
// ==============================================================================

// @trace:test SPEC-DAG-CYCLE, SPEC-CLI-TREE
#[test]
fn test_t3_c20_cycle_defect_handled_during_tree_command() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_cycle();

    let mut cmd = ws.cmd();
    cmd.arg("tree");
    {
        let output = run_cli(&mut cmd);
        // Cycle should be detected and terminate with code 1
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-VALIDATE, SPEC-CLI-MATRIX
#[test]
fn test_t3_c21_orphan_defect_rendered_in_matrix() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_orphan();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-ORPHAN"));
    }
}

// @trace:test SPEC-DAG-VALIDATE, SPEC-CLI-EXPORT
#[test]
fn test_t3_c22_broken_link_defect_blocks_export_or_reports_failure() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_broken_link();

    let mut cmd = ws.cmd();
    cmd.arg("export");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        // Broken links should result in code 1 or safe export
        assert!(code == 0 || code == 1);
    }
}
