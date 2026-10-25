//! Integration tests for polyglot comment scanner and link classification.
//!
//! // @trace SPEC-SCAN-COMMENTS, REQ-CODE-SCAN, REQ-TEST-SCAN, US-CODE-01, US-TEST-01

use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

use xtrc::model::LinkKind;
use xtrc::scanner::comments::{scan_comments_file, scan_comments_text, scan_workspace_comments};

/// Restores literal trace markers in multi-line fixture text.
///
/// Fixtures spell the marker as `@@trace` so that the repository's own
/// `xtrc check` (which scans this file line by line and cannot see Rust
/// raw-string boundaries) does not report the fixture IDs as broken links.
fn fixture(text: &str) -> String {
    text.replace("@@trace", "@trace")
}

#[test]
fn test_integration_polyglot_source_files() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // 1. Rust source file
    let rs_path = root.join("src").join("auth.rs");
    std::fs::create_dir_all(rs_path.parent().expect("parent dir")).expect("create_dir");
    let mut f = File::create(&rs_path).expect("create file");
    writeln!(f, "//! // @trace SPEC-AUTH-01, REQ-AUTH-01").expect("write");
    writeln!(f, "pub fn login() {{").expect("write");
    writeln!(f, "    // @trace REQ-AUTH-02;").expect("write");
    writeln!(f, "    /* @trace:test REQ-AUTH-VERIFY */").expect("write");
    writeln!(f, "}}").expect("write");

    let links = scan_comments_file(&rs_path, Some(root), None).expect("scan");
    assert_eq!(links.len(), 4);
    assert_eq!(links[0].target_id().as_str(), "SPEC-AUTH-01");
    assert_eq!(links[0].kind, LinkKind::SourceCode);
    assert_eq!(links[0].line, 1);
    assert_eq!(links[0].source_file, Path::new("src/auth.rs"));

    assert_eq!(links[1].target_id().as_str(), "REQ-AUTH-01");
    assert_eq!(links[1].kind, LinkKind::SourceCode);
    assert_eq!(links[1].line, 1);

    assert_eq!(links[2].target_id().as_str(), "REQ-AUTH-02");
    assert_eq!(links[2].kind, LinkKind::SourceCode);
    assert_eq!(links[2].line, 3);

    // Explicit tag override forces AutomatedTest
    assert_eq!(links[3].target_id().as_str(), "REQ-AUTH-VERIFY");
    assert_eq!(links[3].kind, LinkKind::AutomatedTest);
    assert_eq!(links[3].line, 4);

    // 2. Python test file
    let py_path = root.join("tests").join("test_auth.py");
    std::fs::create_dir_all(py_path.parent().expect("parent dir")).expect("create_dir");
    let mut f = File::create(&py_path).expect("create file");
    writeln!(f, "# @trace REQ-AUTH-01").expect("write");
    writeln!(f, "def test_login():").expect("write");
    writeln!(f, "    \"\"\" @trace REQ-AUTH-02 \"\"\"").expect("write");
    writeln!(f, "    # @trace:code SPEC-AUTH-HELPER").expect("write");
    writeln!(f, "    pass").expect("write");

    let links = scan_comments_file(&py_path, Some(root), None).expect("scan");
    assert_eq!(links.len(), 3);
    assert_eq!(links[0].target_id().as_str(), "REQ-AUTH-01");
    assert_eq!(links[0].kind, LinkKind::AutomatedTest);
    assert_eq!(links[0].line, 1);

    assert_eq!(links[1].target_id().as_str(), "REQ-AUTH-02");
    assert_eq!(links[1].kind, LinkKind::AutomatedTest);
    assert_eq!(links[1].line, 3);

    // Explicit code override in test directory
    assert_eq!(links[2].target_id().as_str(), "SPEC-AUTH-HELPER");
    assert_eq!(links[2].kind, LinkKind::SourceCode);
    assert_eq!(links[2].line, 4);
}

#[test]
fn test_integration_delimiter_stripping_and_delimiters() {
    let text = r#"
    // @@trace REQ-01, REQ-02, SPEC-03;
    /* @@trace REQ-04*/
    <!-- @@trace SPEC-HTML-01 -->
    <!-- @@trace:code SPEC-HTML-02-->
    -- @@trace REQ-SQL-01;
    rem @@trace REQ-BAT-01
    * @@trace REQ-JAVADOC-01
    /* @@trace:test REQ-TEST-01, REQ-TEST-02; */
    "#;

    let links = scan_comments_text(&fixture(text), Path::new("src/sample.txt"), None, None);
    assert_eq!(links.len(), 11);
    assert_eq!(links[0].target_id().as_str(), "REQ-01");
    assert_eq!(links[1].target_id().as_str(), "REQ-02");
    assert_eq!(links[2].target_id().as_str(), "SPEC-03");
    assert_eq!(links[3].target_id().as_str(), "REQ-04");
    assert_eq!(links[4].target_id().as_str(), "SPEC-HTML-01");
    assert_eq!(links[5].target_id().as_str(), "SPEC-HTML-02");
    assert_eq!(links[6].target_id().as_str(), "REQ-SQL-01");
    assert_eq!(links[7].target_id().as_str(), "REQ-BAT-01");
    assert_eq!(links[8].target_id().as_str(), "REQ-JAVADOC-01");
    assert_eq!(links[9].target_id().as_str(), "REQ-TEST-01");
    assert_eq!(links[10].target_id().as_str(), "REQ-TEST-02");
}

#[test]
fn test_integration_string_literal_exclusion() {
    let text = r#"
    let marker = "@trace REQ-STRING-01";
    let single = '@trace REQ-STRING-02';
    // Genuine comment @@trace REQ-COMMENT-01
    let tmpl = `@trace REQ-STRING-03`;
    /* Another comment: @@trace REQ-COMMENT-02 */
    "#;

    let links = scan_comments_text(&fixture(text), Path::new("src/main.rs"), None, None);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target_id().as_str(), "REQ-COMMENT-01");
    assert_eq!(links[1].target_id().as_str(), "REQ-COMMENT-02");
}

#[test]
fn test_integration_batch_scan_workspace() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let file_a = root.join("src").join("a.rs");
    let file_b = root.join("tests").join("b_test.rs");
    std::fs::create_dir_all(file_a.parent().expect("parent dir")).expect("create_dir");
    std::fs::create_dir_all(file_b.parent().expect("parent dir")).expect("create_dir");

    std::fs::write(&file_a, "// @trace REQ-A-01\n").expect("write");
    std::fs::write(&file_b, "// @trace REQ-B-01\n").expect("write");

    let files = vec![file_a, file_b];
    let links = scan_workspace_comments(&files, Some(root), None).expect("scan");

    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target_id().as_str(), "REQ-A-01");
    assert_eq!(links[0].kind, LinkKind::SourceCode);
    assert_eq!(links[0].source_file, Path::new("src/a.rs"));

    assert_eq!(links[1].target_id().as_str(), "REQ-B-01");
    assert_eq!(links[1].kind, LinkKind::AutomatedTest);
    assert_eq!(links[1].source_file, Path::new("tests/b_test.rs"));
}

// ============================================================================
// Milestone 2 Iteration 2: Polyglot Comment Scanner Regression Suite
// ============================================================================

/// Verifies that string literals containing comment delimiters before `@trace`
/// (e.g. a quoted `//` marker, a `println!` format string, SQL queries with `--`,
/// and hash comments in single quotes) are NOT extracted as trace links.
///
/// // @trace SPEC-SCAN-COMMENTS, REQ-CODE-SCAN, REQ-TEST-SCAN
#[test]
fn test_regression_scanner_string_literals_with_comment_markers() {
    let text = r#"
    // Case 1: Double-quoted Rust string literal containing '//'
    let s1 = "// @trace REQ-STRING-COMMENT-01";

    // Case 2: Println macro with '//' inside format string
    println!("// @trace REQ-STRING-COMMENT-02");

    // Case 3: SQL query string literal containing '--' marker
    let sql = "SELECT * FROM specs WHERE tag = '-- @trace REQ-STRING-COMMENT-03'";

    // Case 4: Python/Ruby single-quoted string containing '#' marker
    let py = '# @trace REQ-STRING-COMMENT-04';

    // Case 5: Block comment markers inside double-quoted string
    let block = "/* @trace REQ-STRING-COMMENT-05 */";

    // Case 6: JavaScript/TypeScript backtick template literal containing '//'
    let tmpl = `// @trace REQ-STRING-COMMENT-06`;

    // Case 7: Escaped double quote preceding comment marker inside string
    let escaped = "Escaped quote: \" // @trace REQ-STRING-COMMENT-07";

    // Case 8: Format macro invocation with multiple string literals on single line
    let formatted = format!("// @trace REQ-STRING-COMMENT-08: {}", "-- @trace REQ-STRING-COMMENT-09");
    "#;

    let links = scan_comments_text(text, Path::new("src/main.rs"), None, None);
    assert_eq!(
        links.len(),
        0,
        "String literals containing comment delimiters must not yield trace links. Extracted: {:?}",
        links
    );
}

/// Verifies that plain markdown tables and un-commented statements containing `@trace`
/// without comment delimiters are skipped, yielding 0 extracted links.
///
/// // @trace SPEC-SCAN-COMMENTS, REQ-CODE-SCAN
#[test]
fn test_regression_scanner_uncommented_markdown_tables() {
    let text = r#"
| Feature | Trace Target | Status |
|---|---|---|
| Configuration Discovery | @trace REQ-TABLE-01 | Implemented |
| Comment Scanner Engine   | @trace REQ-TABLE-02 | In Progress |
| Spec Document Parser     | @trace REQ-TABLE-03 | Verified |

let unquoted_code = @trace REQ-RAW-CODE-01;
- Bullet point referencing @trace REQ-RAW-BULLET-02 in plain text documentation
"#;

    let links = scan_comments_text(text, Path::new("docs/summary.md"), None, None);
    assert_eq!(
        links.len(),
        0,
        "Un-commented markdown tables and bare code lines without comment delimiters must yield 0 links. Extracted: {:?}",
        links
    );
}

/// Verifies that genuine comments containing quotes (double, single, or backticks)
/// inside the comment body (either before or after `@trace`) correctly extract the intended links.
///
/// // @trace SPEC-SCAN-COMMENTS, REQ-CODE-SCAN, REQ-TEST-SCAN
#[test]
fn test_regression_scanner_real_comments_with_embedded_quotes() {
    let text = r#"
    // @@trace REQ-QUOTE-01: author's note "important"
    // "Important note from lead": @@trace REQ-QUOTE-02
    /* @@trace REQ-QUOTE-03: edge case with 'single' quote */
    # @@trace REQ-QUOTE-04: header "Content-Type"
    -- @@trace REQ-QUOTE-05: filter on 'admin' role
    // @@trace REQ-QUOTE-06, REQ-QUOTE-07: verified against "RFC 2119" and 'ISO-9001'
    * @@trace REQ-QUOTE-08: javadoc comment for "public_api"
    // @@trace REQ-QUOTE-09: regex pattern "r\"^[a-z]+$\""
    "#;

    let links = scan_comments_text(&fixture(text), Path::new("src/service.rs"), None, None);
    assert_eq!(
        links.len(),
        9,
        "Real comments with embedded quotes must be extracted correctly. Extracted: {:?}",
        links
    );

    let extracted_ids: Vec<&str> = links.iter().map(|l| l.target_id().as_str()).collect();
    assert_eq!(
        extracted_ids,
        vec![
            "REQ-QUOTE-01",
            "REQ-QUOTE-02",
            "REQ-QUOTE-03",
            "REQ-QUOTE-04",
            "REQ-QUOTE-05",
            "REQ-QUOTE-06",
            "REQ-QUOTE-07",
            "REQ-QUOTE-08",
            "REQ-QUOTE-09"
        ]
    );

    for link in &links {
        assert_eq!(link.kind, LinkKind::SourceCode);
        assert_eq!(link.source_file, Path::new("src/service.rs"));
    }
}

/// Comprehensive end-to-end integration test creating a temporary polyglot file on disk
/// with a realistic mix of string literals containing comment delimiters, un-commented markdown
/// tables, and genuine comments with embedded quotes.
///
/// // @trace SPEC-SCAN-COMMENTS, REQ-CODE-SCAN, REQ-TEST-SCAN
#[test]
fn test_regression_scanner_mixed_polyglot_file_end_to_end() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let src_file = root.join("src").join("controller.rs");
    std::fs::create_dir_all(src_file.parent().expect("parent dir")).expect("create_dir");

    let mut f = File::create(&src_file).expect("create file");
    writeln!(f, "//! Controller module with polyglot regression tests").expect("write");
    writeln!(f, "// @trace REQ-REAL-01: genuine comment with \"quotes\"").expect("write");
    writeln!(f, "let s1 = \"// @trace REQ-FAKE-01\";").expect("write");
    writeln!(f, "println!(\"// @trace REQ-FAKE-02\");").expect("write");
    writeln!(f, "/* @trace REQ-REAL-02: another genuine comment */").expect("write");
    writeln!(f, "let sql = \"SELECT * -- @trace REQ-FAKE-03\";").expect("write");
    writeln!(f, "// @trace REQ-REAL-03").expect("write");

    let links = scan_comments_file(&src_file, Some(root), None).expect("scan");
    assert_eq!(
        links.len(),
        3,
        "Only genuine comments should produce links. Extracted: {:?}",
        links
    );
    assert_eq!(links[0].target_id().as_str(), "REQ-REAL-01");
    assert_eq!(links[0].line, 2);
    assert_eq!(links[1].target_id().as_str(), "REQ-REAL-02");
    assert_eq!(links[1].line, 5);
    assert_eq!(links[2].target_id().as_str(), "REQ-REAL-03");
    assert_eq!(links[2].line, 7);
}
