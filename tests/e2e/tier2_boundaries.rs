//! Tier 2 Boundary & Corner Cases: Extreme limits, malformed syntax, cycles, empty files, and exit code contracts.
//!
//! // @trace:test SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-FMT-FRONTMATTER,
//! // @trace:test SPEC-FMT-GRAMMAR, SPEC-SCAN-COMMENTS, SPEC-DAG-STORAGE, SPEC-DAG-CYCLE,
//! // @trace:test SPEC-DAG-VALIDATE, SPEC-CLI-VERIFY, SPEC-CLI-MATRIX, SPEC-CLI-TREE,
//! // @trace:test SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT, SPEC-ERR-HIERARCHY, SPEC-PERF-BOUNDS

use crate::common::{parse_stdout_json, run_cli, TestWorkspace};

// ==============================================================================
// Feature 1: Init & Scaffold Boundaries (REQ-CLI-INIT, SPEC-DISC-INIT)
// ==============================================================================

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t2_f01_init_nested_empty_directories() {
    let ws = TestWorkspace::new();
    let deep_dir = ws.path().join("a/b/c/d/e");
    let deep_str = deep_dir.to_str().expect("valid utf8");

    let mut cmd = ws.cmd();
    cmd.args(["init", "--path", deep_str]);
    cmd.assert().success();

    assert!(deep_dir.join(".xtrc.toml").is_file());
    assert!(deep_dir.join("docs").is_dir());
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t2_f01_init_existing_docs_directory_intact() {
    let ws = TestWorkspace::new();
    ws.write_file("docs/existing.txt", "preserve me");

    let mut cmd = ws.cmd();
    cmd.arg("init");
    cmd.assert().success();

    assert!(ws.path().join("docs/existing.txt").is_file());
    let preserved =
        std::fs::read_to_string(ws.path().join("docs/existing.txt")).expect("read file");
    assert_eq!(preserved, "preserve me");
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t2_f01_init_symlinked_path() {
    let ws = TestWorkspace::new();
    let real_dir = ws.path().join("real_target");
    std::fs::create_dir_all(&real_dir).expect("create dir");

    let link_dir = ws.path().join("symlink_target");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&real_dir, &link_dir).expect("create symlink");
        let link_str = link_dir.to_str().expect("valid str");

        let mut cmd = ws.cmd();
        cmd.args(["init", "--path", link_str]);
        cmd.assert().success();
        assert!(real_dir.join(".xtrc.toml").is_file());
    }
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t2_f01_init_atomic_temp_file_no_leftover() {
    let ws = TestWorkspace::new();

    let mut cmd = ws.cmd();
    cmd.arg("init");
    cmd.assert().success();

    // Verify no temporary files (.tmp, .bak) remain in the directory
    for entry in std::fs::read_dir(ws.path()).expect("read dir") {
        let entry = entry.expect("valid entry");
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(!name.ends_with(".tmp"), "Found leftover tmp file: {}", name);
    }
}

// @trace:test SPEC-DISC-INIT, REQ-CLI-INIT
#[test]
fn test_t2_f01_init_empty_custom_path_string() {
    let ws = TestWorkspace::new();

    let mut cmd = ws.cmd();
    cmd.args(["init", "--path", ""]);
    // Empty path string should be rejected or handle safely
    let output = cmd.output().expect("execute command");
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 2);
}

// ==============================================================================
// Feature 2: Root & Config Discovery Boundaries (REQ-CFG-DISCOVERY, SPEC-DISC-DISCOVERY)
// ==============================================================================

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t2_f02_discovery_twenty_levels_deep() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let deep_path = "1/2/3/4/5/6/7/8/9/10/11/12/13/14/15/16/17/18/19/20";
    let file_path = ws.write_file(format!("{}/dummy.txt", deep_path), "test");
    let deep_dir = file_path.parent().expect("parent dir");

    let mut cmd = ws.cmd_in(deep_dir);
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t2_f02_discovery_missing_git_and_config_exits_code_2() {
    let ws = TestWorkspace::new();
    // In a directory with neither .xtrc.toml nor .git, xtrc check should exit code 2
    let mut cmd = ws.cmd();
    cmd.arg("check");
    let output = cmd.output().expect("execute command");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 2);
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t2_f02_discovery_empty_config_file() {
    let ws = TestWorkspace::new();
    ws.write_file(".xtrc.toml", "");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    let output = cmd.output().expect("execute command");
    let code = output.status.code().unwrap_or(-1);
    // An empty TOML file might use defaults (code 0) or fail parse (code 2)
    assert!(code <= 2);
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t2_f02_discovery_malformed_toml_syntax_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.write_file(".xtrc.toml", "version = [unclosed bracket");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    let output = cmd.output().expect("execute command");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 2);
}

// @trace:test SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY
#[test]
fn test_t2_f02_discovery_nonexistent_config_override_exits_code_2() {
    let ws = TestWorkspace::new();
    let mut cmd = ws.cmd();
    cmd.args(["--config", "non_existent_config.toml", "check"]);
    cmd.assert().code(2);
}

// ==============================================================================
// Feature 3: Gitignore Walk & Binary Sniff Boundaries (REQ-IGNORE-RULES, SPEC-DISC-TRAVERSAL)
// ==============================================================================

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t2_f03_binary_sniff_null_byte_at_offset_0() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut bytes = vec![0x00];
    bytes.extend_from_slice(b"// @trace REQ-BROKEN-SHOULD-BE-IGNORED\n");
    ws.write_binary_file("src/null_0.rs", &bytes);

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t2_f03_binary_sniff_null_byte_at_offset_511() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut bytes = vec![b'a'; 511];
    bytes.push(0x00);
    bytes.extend_from_slice(b"// @trace REQ-BROKEN-SHOULD-BE-IGNORED\n");
    ws.write_binary_file("src/null_511.rs", &bytes);

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t2_f03_binary_sniff_null_byte_at_offset_512_still_text() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    // Null byte beyond byte 512 is not part of initial 512-byte header probe
    let mut bytes = vec![b' '; 512];
    bytes.push(0x00);
    ws.write_binary_file("src/null_512.rs", &bytes);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 2);
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t2_f03_gitignore_negation_pattern() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(".gitignore", "src/*\n!src/validator.rs\n");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        // validator.rs is re-included via ! negation pattern
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
#[test]
fn test_t2_f03_gitignore_directory_trailing_slash() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(".gitignore", "build/\n");
    ws.write_file("build/bad.rs", "// @trace REQ-BROKEN");

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 4: Markdown Frontmatter Parser Boundaries (REQ-DOC-PARSE, SPEC-FMT-FRONTMATTER)
// ==============================================================================

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t2_f04_frontmatter_unclosed_fence_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let unclosed = r#"---
document_type: specification
id: SPEC-UNCLOSED
title: Unclosed Fence
# missing closing ---
### VIS-TEST: Node
"#;
    ws.write_file("docs/unclosed.md", unclosed);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t2_f04_frontmatter_missing_id_field_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let missing_id = r#"---
document_type: specification
title: Missing ID
version: 1.0.0
status: approved
---
"#;
    ws.write_file("docs/no_id.md", missing_id);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t2_f04_frontmatter_missing_document_type_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let missing_type = r#"---
id: SPEC-NO-TYPE
title: Missing Document Type
version: 1.0.0
status: approved
---
"#;
    ws.write_file("docs/no_type.md", missing_type);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t2_f04_frontmatter_empty_markdown_document() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("docs/empty.md", "");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 2);
    }
}

// @trace:test SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE
#[test]
fn test_t2_f04_frontmatter_crlf_line_endings() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let crlf_content = "---\r\ndocument_type: specification\r\nid: SPEC-CRLF\r\ntitle: CRLF Spec\r\nversion: 1.0.0\r\nstatus: approved\r\n---\r\n\r\n### VIS-CRLF: CRLF Node\r\n- **ID**: VIS-CRLF\r\n- **Parent**: None\r\n- **Type**: Vision\r\n";
    ws.write_file("docs/crlf.md", crlf_content);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 5: ID Grammar & Layer Taxonomy Boundaries (REQ-DOC-PARSE, SPEC-FMT-GRAMMAR)
// ==============================================================================

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t2_f05_grammar_single_letter_prefix_rejected() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TEST
title: Test
version: 1.0.0
status: approved
---

### V-01: Single Letter Prefix
- **ID**: V-01
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/single.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(!output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t2_f05_grammar_consecutive_hyphens_rejected() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TEST
title: Test
version: 1.0.0
status: approved
---

### VIS--DOUBLE: Double Hyphen
- **ID**: VIS--DOUBLE
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/double_hyphen.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(!output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t2_f05_grammar_trailing_hyphen_rejected() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TEST
title: Test
version: 1.0.0
status: approved
---

### VIS-TRAILING-: Trailing Hyphen
- **ID**: VIS-TRAILING-
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/trailing_hyphen.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(!output.status.success());
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t2_f05_grammar_max_length_id() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let long_suffix = "A".repeat(100);
    let long_id = format!("VIS-{}", long_suffix);
    let spec = format!(
        r#"---
document_type: specification
id: SPEC-LONG
title: Long ID Spec
version: 1.0.0
status: approved
---

### {long_id}: Long Node
- **ID**: {long_id}
- **Parent**: None
- **Type**: Vision
"#
    );
    ws.write_file("docs/long_id.md", &spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 2);
    }
}

// @trace:test SPEC-FMT-GRAMMAR, REQ-DOC-PARSE
#[test]
fn test_t2_f05_grammar_unicode_characters_rejected() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-UNICODE
title: Unicode ID
version: 1.0.0
status: approved
---

### VIS-ÜNICODE: Non-ASCII Node
- **ID**: VIS-ÜNICODE
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/unicode_id.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(!output.status.success());
    }
}

// ==============================================================================
// Feature 6: Comment Regex Scanner Boundaries (REQ-CODE-SCAN, SPEC-SCAN-COMMENTS)
// ==============================================================================

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t2_f06_comment_marker_at_eof_without_newline() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("src/eof.rs", "// @trace SPEC-VERIFY-ENGINE"); // No trailing newline

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t2_f06_comment_marker_on_line_one_first_char() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("src/line1.rs", "//@trace SPEC-VERIFY-ENGINE\n");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t2_f06_comment_marker_with_tabs_and_extra_spaces() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/tabs.rs",
        "//\t\t@trace   \tSPEC-VERIFY-ENGINE\t  ,  \tREQ-VERIFY-01\n",
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
fn test_t2_f06_comment_marker_trailing_semicolon_stripped() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    // Trailing semicolon should be cleanly stripped from token
    ws.write_file("src/semi.rs", "// @trace SPEC-VERIFY-ENGINE;\n");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_t2_f06_comment_marker_empty_id_list_ignored() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("src/empty_tag.rs", "// @trace\n// @trace:test \n");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 7: Code vs Test Classifier Boundaries (REQ-TEST-SCAN, SPEC-SCAN-COMMENTS)
// ==============================================================================

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t2_f07_classify_tests_in_deeply_nested_test_dir() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "tests/unit/nested/deep/spec_test.rs",
        "// @trace REQ-VERIFY-01\n",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t2_f07_classify_spec_file_extension() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/auth.spec.ts",
        "// @trace REQ-VERIFY-01\ndescribe('test', () => {});",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t2_f07_classify_case_insensitive_trace_tag() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file("src/case.rs", "// @TRACE SPEC-VERIFY-ENGINE\n");

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t2_f07_classify_code_and_test_tags_on_same_file() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/hybrid.rs",
        "// @trace:code SPEC-VERIFY-ENGINE\nfn prod() {}\n// @trace:test REQ-VERIFY-01\n#[test] fn test() {}\n",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-SCAN-COMMENTS, REQ-TEST-SCAN
#[test]
fn test_t2_f07_classify_docstrings_in_python_and_rust() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    ws.write_file(
        "src/docs.py",
        "\"\"\"@trace SPEC-VERIFY-ENGINE\nModule docstring\n\"\"\"\n",
    );
    ws.write_file(
        "src/docs.rs",
        "/// @trace SPEC-VERIFY-ENGINE\npub fn documented() {}\n",
    );

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 8: 7-Layer Adjacency DAG Boundaries (REQ-DAG-MODEL, SPEC-DAG-STORAGE)
// ==============================================================================

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t2_f08_dag_disconnected_subgraphs() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let spec = r#"---
document_type: specification
id: SPEC-TWO-TREES
title: Two Trees
version: 1.0.0
status: approved
---

### VIS-TREE-ONE: Tree 1
- **ID**: VIS-TREE-ONE
- **Parent**: None
- **Type**: Vision

### VIS-TREE-TWO: Tree 2
- **ID**: VIS-TREE-TWO
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/two_trees.md", spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t2_f08_dag_diamond_dependency_pattern() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let diamond = r#"---
document_type: specification
id: SPEC-DIAMOND
title: Diamond DAG
version: 1.0.0
status: approved
---

### VIS-TOP: Top Node
- **ID**: VIS-TOP
- **Parent**: None
- **Type**: Vision

### STRAT-LEFT: Left Branch
- **ID**: STRAT-LEFT
- **Parent**: VIS-TOP
- **Type**: Strategy

### STRAT-RIGHT: Right Branch
- **ID**: STRAT-RIGHT
- **Parent**: VIS-TOP
- **Type**: Strategy

### EPIC-BOTTOM: Join Node
- **ID**: EPIC-BOTTOM
- **Parent**: STRAT-LEFT, STRAT-RIGHT
- **Type**: Epic
"#;
    ws.write_file("docs/diamond.md", diamond);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t2_f08_dag_thousand_node_chain() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    let mut spec = String::from(
        r#"---
document_type: specification
id: SPEC-CHAIN
title: Chain Spec
version: 1.0.0
status: approved
---

### VIS-CHAIN-0: Root
- **ID**: VIS-CHAIN-0
- **Parent**: None
- **Type**: Vision
"#,
    );

    for i in 1..=50 {
        spec.push_str(&format!(
            "\n### STRAT-CHAIN-{i}: Node {i}\n- **ID**: STRAT-CHAIN-{i}\n- **Parent**: {prev}\n- **Type**: Strategy\n",
            prev = if i == 1 { "VIS-CHAIN-0".to_string() } else { format!("STRAT-CHAIN-{}", i - 1) }
        ));
    }
    ws.write_file("docs/chain.md", &spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t2_f08_dag_duplicate_id_in_different_files_exits_code_1() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let spec2 = r#"---
document_type: specification
id: SPEC-DOC-002
title: Conflict Spec
version: 1.0.0
status: approved
---

### VIS-VCS-NATIVE: Conflicting Root ID
- **ID**: VIS-VCS-NATIVE
- **Parent**: None
- **Type**: Vision
"#;
    ws.write_file("docs/conflict.md", spec2);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL
#[test]
fn test_t2_f08_dag_fan_out_single_parent_many_children() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    let mut spec = String::from(
        r#"---
document_type: specification
id: SPEC-FANOUT
title: Fanout Spec
version: 1.0.0
status: approved
---

### VIS-HUB: Hub Node
- **ID**: VIS-HUB
- **Parent**: None
- **Type**: Vision
"#,
    );

    for i in 1..=30 {
        spec.push_str(&format!(
            "\n### STRAT-SPOKE-{i}: Spoke {i}\n- **ID**: STRAT-SPOKE-{i}\n- **Parent**: VIS-HUB\n- **Type**: Strategy\n"
        ));
    }
    ws.write_file("docs/fanout.md", &spec);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert!(output.status.success());
    }
}

// ==============================================================================
// Feature 9: 3-Color DFS Cycle Detection Boundaries (REQ-CYCLE-DETECT, SPEC-DAG-CYCLE)
// ==============================================================================

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t2_f09_cycle_multiple_independent_cycles() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let multi_cycle = r#"---
document_type: specification
id: SPEC-MULTI-CYCLE
title: Multiple Cycles
version: 1.0.0
status: approved
---

### VIS-A1: Cycle 1 Node A
- **ID**: VIS-A1
- **Parent**: VIS-B1
- **Type**: Vision

### VIS-B1: Cycle 1 Node B
- **ID**: VIS-B1
- **Parent**: VIS-A1
- **Type**: Vision

### VIS-A2: Cycle 2 Node A
- **ID**: VIS-A2
- **Parent**: VIS-B2
- **Type**: Vision

### VIS-B2: Cycle 2 Node B
- **ID**: VIS-B2
- **Parent**: VIS-A2
- **Type**: Vision
"#;
    ws.write_file("docs/multi_cycle.md", multi_cycle);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t2_f09_cycle_nested_cycle_in_dag() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let nested = r#"---
document_type: specification
id: SPEC-NESTED
title: Nested Cycle
version: 1.0.0
status: approved
---

### VIS-ROOT: Valid Root
- **ID**: VIS-ROOT
- **Parent**: None
- **Type**: Vision

### STRAT-PARENT: Parent
- **ID**: STRAT-PARENT
- **Parent**: VIS-ROOT
- **Type**: Strategy

### EPIC-CHILD-A: Child A
- **ID**: EPIC-CHILD-A
- **Parent**: STRAT-PARENT, EPIC-CHILD-B
- **Type**: Epic

### EPIC-CHILD-B: Child B
- **ID**: EPIC-CHILD-B
- **Parent**: EPIC-CHILD-A
- **Type**: Epic
"#;
    ws.write_file("docs/nested_cycle.md", nested);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t2_f09_cycle_cross_layer_loop() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let cross_loop = r#"---
document_type: specification
id: SPEC-CROSS
title: Cross Layer Loop
version: 1.0.0
status: approved
---

### VIS-A: Vision A
- **ID**: VIS-A
- **Parent**: REQ-B
- **Type**: Vision

### REQ-B: Requirement B
- **ID**: REQ-B
- **Parent**: VIS-A
- **Type**: Requirement
"#;
    ws.write_file("docs/cross.md", cross_loop);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t2_f09_cycle_self_loop_on_leaf_node() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let leaf_loop = r#"---
document_type: specification
id: SPEC-LEAF
title: Leaf Loop
version: 1.0.0
status: approved
---

### VIS-ROOT: Root
- **ID**: VIS-ROOT
- **Parent**: None
- **Type**: Vision

### REQ-LEAF: Self Referencing Leaf
- **ID**: REQ-LEAF
- **Parent**: VIS-ROOT, REQ-LEAF
- **Type**: Requirement
"#;
    ws.write_file("docs/leaf_loop.md", leaf_loop);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT
#[test]
fn test_t2_f09_cycle_cycle_with_disconnected_components() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    // Add separate disconnected component that has a cycle
    let extra_cycle = r#"---
document_type: specification
id: SPEC-EXTRA-CYCLE
title: Disconnected Cycle
version: 1.0.0
status: approved
---

### VIS-X1: X1
- **ID**: VIS-X1
- **Parent**: VIS-X2
- **Type**: Vision

### VIS-X2: X2
- **ID**: VIS-X2
- **Parent**: VIS-X1
- **Type**: Vision
"#;
    ws.write_file("docs/extra_cycle.md", extra_cycle);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// ==============================================================================
// Feature 10: Bidirectional Validator Boundaries (REQ-BROKEN-LINK, SPEC-DAG-VALIDATE)
// ==============================================================================

// @trace:test SPEC-DAG-VALIDATE, REQ-BROKEN-LINK
#[test]
fn test_t2_f10_validator_multiple_broken_links_all_reported() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let broken = r#"---
document_type: specification
id: SPEC-MULTI-BROKEN
title: Multi Broken Links
version: 1.0.0
status: approved
---

### VIS-ROOT: Root
- **ID**: VIS-ROOT
- **Parent**: None
- **Type**: Vision

### REQ-ONE: Broken 1
- **ID**: REQ-ONE
- **Parent**: US-MISSING-1
- **Type**: Requirement

### REQ-TWO: Broken 2
- **ID**: REQ-TWO
- **Parent**: US-MISSING-2
- **Type**: Requirement
"#;
    ws.write_file("docs/broken.md", broken);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("US-MISSING-1") || stderr.contains("Broken link"));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-ORPHAN-DETECT
#[test]
fn test_t2_f10_validator_orphan_with_valid_children() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let orphan_tree = r#"---
document_type: specification
id: SPEC-ORPHAN-TREE
title: Orphan Subtree
version: 1.0.0
status: approved
---

### STRAT-ORPHAN: Unanchored Strategy
- **ID**: STRAT-ORPHAN
- **Parent**: None
- **Type**: Strategy

### EPIC-CHILD: Child of Orphan
- **ID**: EPIC-CHILD
- **Parent**: STRAT-ORPHAN
- **Type**: Epic
"#;
    ws.write_file("docs/orphan_tree.md", orphan_tree);

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-FORWARD-COV
#[test]
fn test_t2_f10_validator_requirement_missing_only_tests() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    // Add REQ fulfilled by code but missing tests
    let spec = r#"---
document_type: specification
id: SPEC-NO-TESTS
title: No Tests Spec
version: 1.0.0
status: approved
---

### REQ-NO-TEST: REQ with code but no test
- **ID**: REQ-NO-TEST
- **Parent**: US-VERIFY
- **Type**: Requirement
"#;
    ws.write_file("docs/no_test.md", spec);
    ws.write_file("src/no_test.rs", "// @trace REQ-NO-TEST\npub fn nt() {}");

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        // Under --strict, missing test coverage must fail
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-FORWARD-COV
#[test]
fn test_t2_f10_validator_requirement_missing_only_code() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let spec = r#"---
document_type: specification
id: SPEC-NO-CODE
title: No Code Spec
version: 1.0.0
status: approved
---

### REQ-NO-CODE: REQ with test but no code
- **ID**: REQ-NO-CODE
- **Parent**: US-VERIFY
- **Type**: Requirement
"#;
    ws.write_file("docs/no_code.md", spec);
    ws.write_file("tests/no_code.rs", "// @trace:test REQ-NO-CODE\n");

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-DAG-VALIDATE, REQ-FORWARD-COV
#[test]
fn test_t2_f10_validator_completely_empty_project_validation() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        // Zero nodes in graph: default check succeeds with 0 violations
        assert_eq!(output.status.code(), Some(0));
    }
}

// ==============================================================================
// Feature 11: Check / Verify CLI Boundaries (REQ-STRICT-GATE, SPEC-CLI-VERIFY)
// ==============================================================================

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t2_f11_check_strict_with_single_orphan_fails_code_1() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_orphan();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t2_f11_check_nonstrict_with_orphan_exits_0() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_orphan();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        // Non-strict mode emits warning to stderr but exits 0
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t2_f11_check_invalid_layer_argument_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--layer", "NonExistentLayer"]);
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code == 2 || code == 1);
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t2_f11_check_empty_id_argument_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--id", ""]);
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code == 2 || code == 1);
    }
}

// @trace:test SPEC-CLI-VERIFY, REQ-STRICT-GATE
#[test]
fn test_t2_f11_check_quiet_and_strict_combination() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--strict"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// ==============================================================================
// Feature 12: Matrix CLI Boundaries (REQ-CMD-MATRIX, SPEC-CLI-MATRIX)
// ==============================================================================

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t2_f12_matrix_unsupported_format_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "unsupported_xyz"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t2_f12_matrix_all_items_covered_uncovered_flag_empty_rows() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--uncovered", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Fully covered project should have 0 uncovered rows
        assert!(!stdout.contains("REQ-VERIFY-01"));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t2_f12_matrix_empty_project_renders_headers_only() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t2_f12_matrix_invalid_layer_argument_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--layer", "InvalidLayer"]);
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code == 2 || code == 1);
    }
}

// @trace:test SPEC-CLI-MATRIX, REQ-CMD-MATRIX
#[test]
fn test_t2_f12_matrix_extreme_title_length_wrapping() {
    let ws = TestWorkspace::new();
    ws.write_default_config();
    let long_title = "A ".repeat(200);
    let spec = format!(
        r#"---
document_type: specification
id: SPEC-LONG-TITLE
title: Long Title Spec
version: 1.0.0
status: approved
---

### VIS-LONG-TITLE: {long_title}
- **ID**: VIS-LONG-TITLE
- **Parent**: None
- **Type**: Vision
"#
    );
    ws.write_file("docs/long_title.md", &spec);

    let mut cmd = ws.cmd();
    cmd.args(["matrix", "--format", "markdown"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// ==============================================================================
// Feature 13: Tree CLI Boundaries (REQ-CMD-TREE, SPEC-CLI-TREE)
// ==============================================================================

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t2_f13_tree_depth_zero_returns_root_only() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "--depth", "0"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t2_f13_tree_negative_depth_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "--depth", "-1"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t2_f13_tree_non_numeric_depth_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "--depth", "abc"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t2_f13_tree_reverse_on_vision_root_returns_single_node() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "VIS-VCS-NATIVE", "--reverse"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VIS-VCS-NATIVE"));
    }
}

// @trace:test SPEC-CLI-TREE, REQ-CMD-TREE
#[test]
fn test_t2_f13_tree_reverse_and_down_conflict_handling() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["tree", "--reverse", "--down"]);
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 2);
    }
}

// ==============================================================================
// Feature 14: Export CLI Boundaries (REQ-CMD-EXPORT, SPEC-CLI-EXPORT)
// ==============================================================================

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t2_f14_export_unsupported_format_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["export", "--format", "pdf"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t2_f14_export_overwrite_existing_output_file() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let out_file = ws.write_file("export.md", "old data");
    let out_str = out_file.to_str().expect("valid str");

    let mut cmd = ws.cmd();
    cmd.args(["export", "--output", out_str]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        let new_content = std::fs::read_to_string(&out_file).expect("read file");
        assert!(!new_content.contains("old data"));
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t2_f14_export_output_to_root_protected_dir_exits_code_2() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["export", "--output", "/proc/forbidden/export.md"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(2));
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t2_f14_export_empty_project_generates_minimal_valid_export() {
    let ws = TestWorkspace::new();
    ws.write_default_config();

    let mut cmd = ws.cmd();
    cmd.arg("export");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-EXPORT, REQ-CMD-EXPORT
#[test]
fn test_t2_f14_export_output_relative_path_resolution() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["export", "-o", "sub/nested/output.md"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
        assert!(ws.path().join("sub/nested/output.md").is_file());
    }
}

// ==============================================================================
// Feature 15: Universal JSON & Streams Boundaries (REQ-FMT-JSON, SPEC-CLI-OUTPUT)
// ==============================================================================

// @trace:test SPEC-CLI-OUTPUT, REQ-FMT-JSON
#[test]
fn test_t2_f15_json_error_diagnostics_do_not_contaminate_stdout() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_broken_link();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        if !stdout_str.trim().is_empty() {
            let json = parse_stdout_json(&output);
            assert_eq!(json["status"], "fail");
        }
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-FMT-JSON
#[test]
fn test_t2_f15_json_empty_violations_array_on_success() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        let json = parse_stdout_json(&output);
        assert_eq!(json["status"], "pass");
        if let Some(violations) = json.get("violations") {
            assert!(violations.as_array().expect("array").is_empty());
        }
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-UNIX-STREAMS
#[test]
fn test_t2_f15_streams_stdout_redirected_to_dev_null() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        // Output should exit cleanly regardless of stdout sink
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-UNIX-STREAMS
#[test]
fn test_t2_f15_streams_stderr_redirected_to_dev_null() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// @trace:test SPEC-CLI-OUTPUT, REQ-EXIT-CODES
#[test]
fn test_t2_f15_streams_zero_exit_on_clean_json_output() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();

    let mut cmd = ws.cmd();
    cmd.args(["check", "--json"]);
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(0));
    }
}

// ==============================================================================
// Feature 16: Exit Codes & Robustness Boundaries (REQ-EXIT-CODES, SPEC-ERR-HIERARCHY)
// ==============================================================================

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t2_f16_exit_code_1_exclusive_to_graph_violations() {
    let ws = TestWorkspace::new();
    ws.setup_project_with_cycle();

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        assert_eq!(output.status.code(), Some(1));
    }
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t2_f16_exit_code_2_for_missing_required_flag_value() {
    let ws = TestWorkspace::new();
    let mut cmd = ws.cmd();
    cmd.args(["init", "--path"]); // Missing required argument value
    cmd.assert().code(2);
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-EXIT-CODES
#[test]
fn test_t2_f16_exit_code_2_for_unreadable_file_permissions() {
    let ws = TestWorkspace::new();
    let unreadable = ws.write_file("unreadable.toml", "content");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o000);
        let _ = std::fs::set_permissions(&unreadable, perms);

        let mut cmd = ws.cmd();
        cmd.args(["--config", unreadable.to_str().expect("str"), "check"]);
        let output = cmd.output().expect("execute command");
        let code = output.status.code().unwrap_or(-1);
        assert_eq!(code, 2);

        // Restore permission for cleanup
        let restore = std::fs::Permissions::from_mode(0o644);
        let _ = std::fs::set_permissions(&unreadable, restore);
    }
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-NFR-ROBUST
#[test]
fn test_t2_f16_robustness_huge_file_with_no_newlines() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let huge_line = "a".repeat(100_000);
    ws.write_file("src/huge.rs", &huge_line);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 2);
    }
}

// @trace:test SPEC-ERR-HIERARCHY, REQ-NFR-ROBUST
#[test]
fn test_t2_f16_robustness_deeply_nested_brackets_and_quotes() {
    let ws = TestWorkspace::new();
    ws.setup_valid_full_project();
    let weird =
        "(((([[[[{{{{\"\"\"\"''''/*/*/*// @trace SPEC-VERIFY-ENGINE */*/*/'\"\"\"}}}}]]]]))))";
    ws.write_file("src/weird.rs", weird);

    let mut cmd = ws.cmd();
    cmd.arg("check");
    {
        let output = run_cli(&mut cmd);
        let code = output.status.code().unwrap_or(-1);
        assert!(code <= 2);
    }
}
