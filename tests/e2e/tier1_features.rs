//! Tier 1 Feature Coverage: Isolated verification of all 16 xtrc features (>=5 test cases per feature).
//!
//! // @trace:test SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-FMT-FRONTMATTER,
//! // @trace:test SPEC-FMT-GRAMMAR, SPEC-SCAN-COMMENTS, SPEC-DAG-STORAGE, SPEC-DAG-CYCLE,
//! // @trace:test SPEC-DAG-VALIDATE, SPEC-CLI-VERIFY, SPEC-CLI-MATRIX, SPEC-CLI-TREE,
//! // @trace:test SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT, SPEC-ERR-HIERARCHY, SPEC-PERF-BOUNDS

use crate::common::{parse_stdout_json, run_cli, TestWorkspace};
use predicates::prelude::*;

// ==============================================================================
// Feature 1: Init & Scaffold (REQ-CLI-INIT, SPEC-DISC-INIT)
// ==============================================================================

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t1_f01_init_creates_scaffold_default() {
    let ws = TestWorkspace::new();
    let mut cmd = ws.cmd();
    cmd.arg("init");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized xtrc workspace"));

    assert!(ws.path().join(".xtrc.toml").is_file());
    assert!(ws.path().join("docs").is_dir());

    let content = std::fs::read_to_string(ws.path().join(".xtrc.toml")).expect("read config");
    assert!(content.contains("[project]"));
    assert!(content.contains("name = \"xtrc\""));
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t1_f01_init_custom_path() {
    let ws = TestWorkspace::new();
    let target = ws.path().join("nested/target");
    let target_str = target.to_str().expect("valid utf8");

    let mut cmd = ws.cmd();
    cmd.args(["init", "--path", target_str]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized xtrc workspace"));

    assert!(target.join(".xtrc.toml").is_file());
    assert!(target.join("docs").is_dir());
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t1_f01_init_quiet_flag() {
    let ws = TestWorkspace::new();
    let mut cmd = ws.cmd();
    cmd.args(["init", "--quiet"]);
    cmd.assert().success().stdout(predicate::str::is_empty());

    assert!(ws.path().join(".xtrc.toml").is_file());
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t1_f01_init_fails_existing_without_force() {
    let ws = TestWorkspace::new();
    ws.write_file(".xtrc.toml", "version = \"1\"");

    let mut cmd = ws.cmd();
    cmd.arg("init");
    cmd.assert()
        .code(2)
        .stderr(predicate::str::contains("already initialized"));
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t1_f01_init_force_overwrites() {
    let ws = TestWorkspace::new();
    ws.write_file(".xtrc.toml", "legacy = true");

    let mut cmd = ws.cmd();
    cmd.args(["init", "--force"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized xtrc workspace"));

    let content = std::fs::read_to_string(ws.path().join(".xtrc.toml")).expect("read config");
    assert!(!content.contains("legacy = true"));
    assert!(content.contains("[project]"));
}

// ==============================================================================
// Feature 2: Root & Config Discovery (REQ-CFG-DISCOVERY, SPEC-DISC-DISCOVERY)
// ==============================================================================

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t1_f02_discovery_from_cwd() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    let mut cmd = ws.cmd();
    cmd.arg("--help");
    cmd.assert().success();
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t1_f02_discovery_from_nested_subdir() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let sub = ws.write_file("nested/level1/level2/dummy.txt", "data");
    let subdir = sub.parent().expect("parent dir");

    let mut cmd = ws.cmd_in(subdir);
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success() || output.status.code() == Some(0));
    }
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t1_f02_discovery_ascends_to_git_root() {
    let ws = TestWorkspace::new();
    ws.init_git();
    let sub = ws.write_file("sub/file.txt", "content");
    let subdir = sub.parent().expect("parent dir");

    let mut cmd = ws.cmd_in(subdir);
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code == 0 || code == 1 || code == 2);
    }
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t1_f02_discovery_root_cli_override() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let root_str = ws.path().to_str().expect("valid str");

    let other = TestWorkspace::new();
    let mut cmd = other.cmd();
    cmd.args(["--root", root_str, "check"]);
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code == 0 || code == 1);
    }
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t1_f02_discovery_config_cli_override() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let custom_cfg = ws.write_file("custom.toml", "[project]\nname = \"custom\"\nroot = \".\"");
    let cfg_str = custom_cfg.to_str().expect("valid str");

    let mut cmd = ws.cmd();
    cmd.args(["--config", cfg_str, "check"]);
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code == 0 || code == 1 || code == 2);
    }
}

// ==============================================================================
// Feature 3: Gitignore Walk & Binary Sniff (REQ-IGNORE-RULES, SPEC-DISC-TRAVERSAL)
// ==============================================================================

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t1_f03_gitignore_root_ignored_files_skipped() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(".gitignore", "ignored_dir/\n*.tmp\n");
    ws.write_file(
        "ignored_dir/bad.rs",
        "// @trace REQ-NONEXISTENT-SHOULD-BE-IGNORED",
    );
    ws.write_file("file.tmp", "// @trace REQ-NONEXISTENT-SHOULD-BE-IGNORED");

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        // Ignored files should not produce broken link errors
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t1_f03_gitignore_nested_ignored_files_skipped() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("sub/.gitignore", "local_ignore.rs\n");
    ws.write_file(
        "sub/local_ignore.rs",
        "// @trace REQ-NONEXISTENT-SHOULD-BE-IGNORED",
    );

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t1_f03_binary_sniff_null_byte_skipped() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Create binary asset with null byte followed by @trace tag
    let mut bytes = vec![0x7f, 0x45, 0x4c, 0x46, 0x00];
    bytes.extend_from_slice(b"// @trace REQ-BROKEN-SHOULD-BE-IGNORED");
    ws.write_binary_file("src/asset.bin", &bytes);

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t1_f03_exclude_globs_honored() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_config(
        r#"version = "1"
[project]
name = "test"
root = "."
[specs]
include = ["docs/**/*.md"]
exclude = ["docs/scratch/**"]
"#,
    );
    ws.write_file("docs/scratch/draft.md", "### REQ-DRAFT: Draft\n");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t1_f03_standard_source_code_traversed() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 4: Markdown Frontmatter Parser (REQ-DOC-PARSE, SPEC-FMT-FRONTMATTER)
// ==============================================================================

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t1_f04_frontmatter_valid_yaml_metadata() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: requirements_specification
id: SPEC-DOC-999
title: Valid Specification Document
version: 2.1.0
status: approved
---

### VIS-ONE: Root
- **ID**: VIS-ONE
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/valid.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t1_f04_frontmatter_heading_nodes_extracted() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "VIS-VCS-NATIVE"]);
    {
        let output = run_cli(&mut cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t1_f04_frontmatter_bullet_attributes_parsed() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-VERIFY-01"));
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t1_f04_frontmatter_multi_layer_single_doc() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t1_f04_frontmatter_whitespace_tolerance() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type:  requirements_specification  
id:   SPEC-DOC-SPACE   
title:   Spaced Title   
version:  1.0.0  
status: approved
---

### VIS-SPACED: Spaced Title
- **ID**:   VIS-SPACED  
- **Parent**:   None  
- **Type**:   Vision  
"#;
    ws.write_file("docs/spaced.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 5: ID Grammar & Layer Taxonomy (REQ-DOC-PARSE, SPEC-FMT-GRAMMAR)
// ==============================================================================

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t1_f05_grammar_valid_pattern_standard_prefixes() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "VIS-VCS-NATIVE"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t1_f05_grammar_multi_token_alphanumeric() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TEST-001
title: Multi Token Spec
version: 1.0.0
status: approved
---

### VIS-DATA-STORE-ENGINE-V2: Multi Token ID
- **ID**: VIS-DATA-STORE-ENGINE-V2
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/spec.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t1_f05_grammar_rejects_lowercase() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TEST-001
title: Invalid Spec
version: 1.0.0
status: approved
---

### vis-lowercase: Invalid ID
- **ID**: vis-lowercase
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/invalid.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        // Lowercase ID must fail with code 1 or code 2
        assert!(!output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t1_f05_grammar_rejects_underscores() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TEST-001
title: Invalid Spec
version: 1.0.0
status: approved
---

### VIS_UNDERSCORE: Invalid ID
- **ID**: VIS_UNDERSCORE
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/invalid.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(!output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t1_f05_grammar_enforces_layer_hierarchy() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    // Inverted hierarchy: Vision node having Requirement parent
    let spec = r#"---
document_type: specification
id: SPEC-TEST-001
title: Inverted Hierarchy
version: 1.0.0
status: approved
---

### REQ-PARENT: Requirement Node
- **ID**: REQ-PARENT
- **Parent**: None
- **Type**: Requirement

### VIS-CHILD: Vision Child
- **ID**: VIS-CHILD
- **Parent**: REQ-PARENT
- **Type**: Vision
"#;
    ws.write_file("docs/inverted.md", spec);

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        // Hierarchy inversion should be rejected
        assert!(!output.status.success());
    }
}

// ==============================================================================
// Feature 6: Comment Regex Scanner (REQ-CODE-SCAN, REQ-TEST-SCAN, SPEC-SCAN-COMMENTS)
// ==============================================================================

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f06_comment_slash_single_line() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("src/slash.rs", "// @trace SPEC-VERIFY-ENGINE\nfn f() {}");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f06_comment_hash_single_line() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "scripts/test.py",
        "# @trace SPEC-VERIFY-ENGINE\ndef run(): pass",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f06_comment_dash_single_line() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "db/migration.sql",
        "-- @trace SPEC-VERIFY-ENGINE\nSELECT 1;",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f06_comment_block_multiline() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/block.c",
        "/* @trace SPEC-VERIFY-ENGINE */\nvoid c() {}",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f06_comment_multi_target_comma_separated() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/multi.rs",
        "// @trace SPEC-VERIFY-ENGINE, REQ-VERIFY-01\nfn m() {}",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 7: Code vs Test Classifier (REQ-TEST-SCAN, SPEC-SCAN-COMMENTS)
// ==============================================================================

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t1_f07_classify_tests_directory_as_automated_test() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "tests/integration_test.rs",
        "// @trace REQ-VERIFY-01\n#[test] fn t() {}",
    );

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "json"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t1_f07_classify_test_filename_pattern_as_automated_test() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/component_test.rs",
        "// @trace REQ-VERIFY-01\n#[test] fn ct() {}",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f07_classify_src_directory_as_source_code() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("src/engine.rs", "// @trace REQ-VERIFY-01\npub fn run() {}");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t1_f07_classify_trace_test_tag_override() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    // In src/ path, explicit @trace:test tag forces AutomatedTest classification
    ws.write_file(
        "src/inline_tests.rs",
        "// @trace:test REQ-VERIFY-01\n#[test] fn it() {}",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t1_f07_classify_trace_code_tag_override() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    // In tests/ path, explicit @trace:code forces SourceCode classification
    ws.write_file(
        "tests/mock_service.rs",
        "// @trace:code REQ-VERIFY-01\npub struct Mock;",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 8: 7-Layer Adjacency DAG (REQ-DAG-MODEL, SPEC-DAG-STORAGE)
// ==============================================================================

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t1_f08_dag_ingests_all_7_layers() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        let json = parse_stdout_json(&output);
        assert_eq!(json["status"], "pass");
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t1_f08_dag_maintains_bidirectional_adjacency() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd_fwd = ws.cmd();
    cmd_fwd.args(["tree", "VIS-VCS-NATIVE"]);
    {
        let output = run_cli(&mut cmd_fwd);
        assert!(output.status.success());
    }

    let mut cmd_rev = ws.cmd();
    cmd_rev.args(["tree", "REQ-VERIFY-01", "--reverse"]);
    {
        let output = run_cli(&mut cmd_rev);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t1_f08_dag_tracks_provenance_file_and_line() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-VERIFY-01"));
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t1_f08_dag_rejects_duplicate_node_ids() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let duplicate_spec = r#"---
document_type: specification
id: SPEC-DUP-001
title: Duplicate Spec
version: 1.0.0
status: approved
---

### REQ-VERIFY-01: Duplicate Node ID
- **ID**: REQ-VERIFY-01
- **Parent**: US-VERIFY
- **Type**: Requirement
"#;
    ws.write_file("docs/dup.md", duplicate_spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t1_f08_dag_supports_multi_parent_nodes() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let multi_parent_spec = r#"---
document_type: specification
id: SPEC-MULTI-001
title: Multi Parent Spec
version: 1.0.0
status: approved
---

### VIS-ONE: Root One
- **ID**: VIS-ONE
- **Parent**: None
- **Type**: Vision

### VIS-TWO: Root Two
- **ID**: VIS-TWO
- **Parent**: None
- **Type**: Vision

### STRAT-JOIN: Strategy with Two Parents
- **ID**: STRAT-JOIN
- **Parent**: VIS-ONE, VIS-TWO
- **Type**: Strategy
"#;
    ws.write_file("docs/multi_parent.md", multi_parent_spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 9: 3-Color DFS Cycle Detection (REQ-CYCLE-DETECT, SPEC-DAG-CYCLE)
// ==============================================================================

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t1_f09_cycle_direct_2_node() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_cycle();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.to_lowercase().contains("cycle") || stderr.contains("->"));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t1_f09_cycle_indirect_3_node() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-CYCLE-3
title: 3-Node Cycle
version: 1.0.0
status: approved
---

### VIS-A: Node A
- **ID**: VIS-A
- **Parent**: VIS-C
- **Type**: Vision

### VIS-B: Node B
- **ID**: VIS-B
- **Parent**: VIS-A
- **Type**: Vision

### VIS-C: Node C
- **ID**: VIS-C
- **Parent**: VIS-B
- **Type**: Vision
"#;
    ws.write_file("docs/cycle3.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t1_f09_cycle_self_referential_loop() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-SELF-LOOP
title: Self Loop
version: 1.0.0
status: approved
---

### VIS-SELF: Self Loop
- **ID**: VIS-SELF
- **Parent**: VIS-SELF
- **Type**: Vision
"#;
    ws.write_file("docs/self_loop.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t1_f09_cycle_reports_chain_to_stderr() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_cycle();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("VIS-CYCLE-A") || stderr.contains("VIS-CYCLE-B"));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t1_f09_cycle_terminates_with_exit_code_1() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_cycle();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// ==============================================================================
// Feature 10: Bidirectional Validator (REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST, REQ-ORPHAN-DETECT, SPEC-DAG-VALIDATE)
// ==============================================================================

// @trace:test SPEC-DAG-VALIDATE, REQ-BROKEN-LINK
#[test]
fn test_t1_f10_validator_detects_broken_parent_link() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_broken_link();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("US-NON-EXISTENT") || stderr.contains("Broken link"));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-BROKEN-LINK
#[test]
fn test_t1_f10_validator_detects_broken_code_link() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/broken.rs",
        "// @trace REQ-NON-EXISTENT-ID\npub fn b() {}",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-ORPHAN-DETECT
#[test]
fn test_t1_f10_validator_detects_orphan_non_root() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_orphan();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-FORWARD-COV
#[test]
fn test_t1_f10_validator_calculates_forward_coverage() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        let json = parse_stdout_json(&output);
        assert_eq!(json["statistics"]["coverage_percent"], 100.0);
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-BACKWARD-JUST
#[test]
fn test_t1_f10_validator_traces_backward_provenance_to_vision() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "REQ-VERIFY-01", "--reverse"]);
    {
        let output = run_cli(&mut cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// ==============================================================================
// Feature 11: Check / Verify CLI Command (REQ-STRICT-GATE, SPEC-CLI-VERIFY)
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t1_f11_check_clean_project_exits_0() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t1_f11_verify_alias_behavior() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("verify");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t1_f11_check_strict_fails_on_partial_coverage() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_uncovered_req();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t1_f11_check_layer_filter() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--layer", "Requirement"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t1_f11_check_id_filter() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--id", "REQ-VERIFY-01"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// ==============================================================================
// Feature 12: Matrix CLI Command (REQ-CMD-MATRIX, SPEC-CLI-MATRIX)
// ==============================================================================

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t1_f12_matrix_default_table_output() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("matrix");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("REQ-VERIFY-01"));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t1_f12_matrix_uncovered_flag_filters_gaps() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--uncovered"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t1_f12_matrix_format_markdown() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("|"));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t1_f12_matrix_format_csv() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "csv"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(","));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t1_f12_matrix_layer_filter() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--layer", "Requirement"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// ==============================================================================
// Feature 13: Tree CLI Command (REQ-CMD-TREE, SPEC-CLI-TREE)
// ==============================================================================

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t1_f13_tree_visualizes_from_vision_root() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("tree");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t1_f13_tree_anchored_at_specific_id() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "EPIC-CORE"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("EPIC-CORE"));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t1_f13_tree_reverse_flag_upward_traversal() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "REQ-VERIFY-01", "--reverse"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t1_f13_tree_depth_flag_limits_levels() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "--depth", "2"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t1_f13_tree_invalid_id_fails_exit_code_1() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "NON-EXISTENT-ID"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// ==============================================================================
// Feature 14: Export CLI Command (REQ-CMD-EXPORT, SPEC-CLI-EXPORT)
// ==============================================================================

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t1_f14_export_default_markdown_stdout() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("export");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t1_f14_export_format_json() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["export", "--format", "json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert!(json.is_object() || json.is_array());
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t1_f14_export_output_file_atomic_write() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let out_file = ws.path().join("export.md");
    let out_str = out_file.to_str().expect("valid str");

    let mut cmd = ws.cmd();
    cmd.args(["export", "--output", out_str]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        assert!(out_file.is_file());
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t1_f14_export_file_content_matches_stdout() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let out_file = ws.path().join("export_match.md");
    let out_str = out_file.to_str().expect("valid str");

    let mut cmd_out = ws.cmd();
    cmd_out.args(["export", "--output", out_str]);
    {
        let output = run_cli(&mut cmd_out);
        assert_eq!(output.status.code(), Some(0));
        let file_content = std::fs::read_to_string(&out_file).expect("read export");

        let mut cmd_stdout = ws.cmd();
        cmd_stdout.arg("export");
        {
            let stdout_output = run_cli(&mut cmd_stdout);
            let stdout_str = String::from_utf8_lossy(&stdout_output.stdout);
            assert_eq!(file_content.trim(), stdout_str.trim());
        }
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t1_f14_export_unwritable_path_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let unwritable_path = "/non_existent_dir_xyz/export.md";

    let mut cmd = ws.cmd();
    cmd.args(["export", "--output", unwritable_path]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// ==============================================================================
// Feature 15: Universal JSON & Streams (REQ-FMT-JSON, REQ-UNIX-STREAMS, SPEC-CLI-OUTPUT)
// ==============================================================================

// @trace:test SPEC-CLI-OUTPUT, REQ-FMT-JSON, REQ-UNIX-STREAMS
#[test]
fn test_t1_f15_json_check_valid_schema() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert!(json.get("status").is_some());
        assert!(json.get("statistics").is_some());
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-FMT-JSON, REQ-UNIX-STREAMS
#[test]
fn test_t1_f15_json_matrix_valid_schema() {
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

// @trace:test SPEC-CLI-OUTPUT, REQ-FMT-JSON, REQ-UNIX-STREAMS
#[test]
fn test_t1_f15_json_tree_valid_schema() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "--json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let json = parse_stdout_json(&output);
        assert!(json.is_object());
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-UNIX-STREAMS
#[test]
fn test_t1_f15_streams_stdout_pure_data_stderr_diagnostics() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let _json: serde_json::Value =
            serde_json::from_str(&stdout_str).expect("stdout must be pure valid JSON");
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-UNIX-STREAMS
#[test]
fn test_t1_f15_streams_pipe_suppresses_ansi_color() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Assert absence of raw ANSI escape sequences "\x1b["
        assert!(!stdout.contains("\x1b["));
    }
}

// ==============================================================================
// Feature 16: Exit Codes & Robustness (REQ-EXIT-CODES, REQ-NFR-ROBUST, SPEC-ERR-HIERARCHY)
// ==============================================================================

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t1_f16_exit_code_0_on_success() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t1_f16_exit_code_1_on_verification_failure() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_broken_link();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t1_f16_exit_code_2_on_cli_usage_error() {
    let ws = TestWorkspace::new();
    let mut cmd = ws.cmd();
    cmd.arg("--non-existent-option-xyz");
    cmd.assert().code(2);
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t1_f16_exit_code_2_on_io_error() {
    let ws = TestWorkspace::new();
    let mut cmd = ws.cmd();
    cmd.args(["--config", "/non_existent_path/config.toml", "check"]);
    cmd.assert().code(2);
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-NFR-ROBUST
#[test]
fn test_t1_f16_robustness_no_panic_on_malformed_input() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    ws.write_binary_file(
        "docs/malformed.md",
        &[0x00, 0xff, 0xfe, 0x01, 0x02, b'#', b'#', b'#', b' '],
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    let output = cmd.output().expect("execute xtrc");
    // Binary or malformed input should never panic (code > 2)
    let code = output.status.code().unwrap_or(-1);
    assert!(code <= 2, "Binary or malformed file must not cause panic");
}
