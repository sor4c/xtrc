//! Polyglot comment regex scanner extracting `@trace` annotations across programming languages.
//!
//! // @trace SPEC-SCAN-COMMENTS, REQ-CODE-SCAN, REQ-TEST-SCAN, US-CODE-01, US-TEST-01

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::error::ParseError;
use crate::model::{LinkKind, NodeId, TraceLink};

/// Compiled regular expression matching polyglot `@trace` comment annotations.
///
/// Matches:
/// - `@trace <ID>`
/// - `@trace:code <ID>`
/// - `@trace:test <ID>`
/// - Comma-separated or semicolon-separated IDs: `@trace <ID>, <ID>; <ID>`
/// - Case-insensitive: `@trace`, `@TRACE`, `@Trace:Test`
/// - Preceded by start-of-line or non-word/non-@ boundary character
static TRACE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    match regex::Regex::new(
        r"(?i)(?:^|[^\w@])@trace(?::(?P<tag>[a-zA-Z_-]+))?\s+(?P<ids>[A-Z][A-Z0-9]*(-[A-Z0-9]+)+(?:\s*[,;]\s*[A-Z][A-Z0-9]*(-[A-Z0-9]+)+)*)",
    ) {
        Ok(re) => re,
        Err(_) => unreachable!(),
    }
});

/// Known single-line comment delimiters that extend to the end of the line.
const SINGLE_LINE_COMMENT_DELIMITERS: &[&str] = &["//", "#", "--", ";", "%", "rem ", "REM "];

/// Known multi-line block comment openers and their respective closing tags.
const MULTI_LINE_COMMENT_DELIMITERS: &[(&str, &str)] = &[
    ("/*", "*/"),
    ("<!--", "-->"),
    ("(*", "*)"),
    ("{-", "-}"),
    ("\"\"\"", "\"\"\""),
    ("'''", "'''"),
];

/// Multi-character closing comment delimiters stripped from identifier tokens.
const CLOSING_DELIMITER_SUFFIXES: &[&str] = &["*/", "-->", "*)", "-}", "\"\"\"", "'''"];

/// Cleans an extracted identifier token by stripping trailing delimiters, punctuation, and whitespace.
///
/// Strips trailing semicolons, commas, dots, colons, quotes, brackets, and closing comment tags
/// (such as `*/`, `-->`, `*)`, `-}`) while preserving internal hyphens in identifiers like `REQ-01`.
#[must_use]
pub fn clean_id_token(mut token: &str) -> &str {
    token = token.trim();
    loop {
        let prev = token;
        token = token.trim_end();

        for suffix in CLOSING_DELIMITER_SUFFIXES {
            if let Some(stripped) = token.strip_suffix(suffix) {
                token = stripped.trim_end();
            }
        }

        token = token.trim_end_matches(|c: char| {
            matches!(
                c,
                ';' | ','
                    | '.'
                    | ':'
                    | '"'
                    | '\''
                    | ')'
                    | ']'
                    | '}'
                    | '*'
                    | '/'
                    | '>'
                    | '-'
                    | ' '
                    | '\t'
            )
        });

        if token == prev {
            break;
        }
    }
    token.trim()
}

/// Evaluates whether a candidate file path represents an automated test file based on conventions.
///
/// Matches:
/// - Directory components: `tests/**`, `test/**`, `spec/**`, `specs/**`
/// - Filename prefixes: `test_*.*`, `test-*.*`
/// - Filename suffixes / stems: `*_test.*`, `*-test.*`, `*.spec.*`, `*.test.*`, `*Test.*`, `*Tests.*`
#[must_use]
pub fn is_test_file_path(path: &Path) -> bool {
    // 1. Check directory components
    let components = path.components();
    for comp in components {
        let comp_str = comp.as_os_str().to_string_lossy().to_ascii_lowercase();
        if matches!(comp_str.as_str(), "test" | "tests" | "spec" | "specs") {
            return true;
        }
    }

    // 2. Check filename and stem patterns
    let file_name = match path.file_name() {
        Some(name) => name.to_string_lossy(),
        None => return false,
    };

    if file_name.starts_with("test_") || file_name.starts_with("test-") {
        return true;
    }

    if file_name.contains(".test.") || file_name.contains(".spec.") {
        return true;
    }

    let stem = match path.file_stem() {
        Some(s) => s.to_string_lossy(),
        None => return false,
    };

    if stem.ends_with("_test")
        || stem.ends_with("-test")
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
        || stem.ends_with("Test")
        || stem.ends_with("Tests")
    {
        return true;
    }

    false
}

/// Classifies a traceability link kind based on explicit marker tag, caller override, and path heuristics.
///
/// Precedence:
/// 1. Tag override: `@trace:test` -> `LinkKind::AutomatedTest`; `@trace:code` -> `LinkKind::SourceCode`
/// 2. Caller override: `default_kind: Some(kind)`
/// 3. Path heuristic: `is_test_file_path(path)` -> `LinkKind::AutomatedTest`, else `LinkKind::SourceCode`
#[must_use]
pub fn classify_link_kind(
    tag: Option<&str>,
    path: &Path,
    default_kind: Option<LinkKind>,
) -> LinkKind {
    if let Some(t) = tag {
        let lower = t.to_ascii_lowercase();
        if lower == "test" || lower == "tests" {
            return LinkKind::AutomatedTest;
        }
        if lower == "code" || lower == "src" {
            return LinkKind::SourceCode;
        }
    }

    if let Some(kind) = default_kind {
        return kind;
    }

    if is_test_file_path(path) {
        LinkKind::AutomatedTest
    } else {
        LinkKind::SourceCode
    }
}

/// Evaluates whether a `@trace` match position on a line should be excluded.
///
/// Returns `true` if the `@trace` token resides inside an unescaped string literal
/// or on an un-commented plain text line (e.g. a markdown table row or bare code).
/// Returns `false` if it is confirmed to reside inside a genuine comment.
///
/// Operates with zero heap allocations, zero naked unwraps, and O(prefix length) time.
fn is_string_literal(line: &str, at_trace_pos: usize) -> bool {
    let prefix = match line.get(..at_trace_pos) {
        Some(p) => p,
        None => return true,
    };

    // Check if line begins with a block comment continuation '*' after whitespace
    let trimmed = line.trim_start();
    if trimmed.starts_with('*') && !trimmed.starts_with("*/") && !prefix.contains("*/") {
        return false;
    }

    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut in_backtick = false;
    let mut escaped = false;

    let mut i = 0;
    while i < prefix.len() {
        let rest = match prefix.get(i..) {
            Some(r) => r,
            None => break,
        };

        let ch = match rest.chars().next() {
            Some(c) => c,
            None => break,
        };
        let ch_len = ch.len_utf8();

        if escaped {
            escaped = false;
            i += ch_len;
            continue;
        }

        if (in_double_quote || in_single_quote || in_backtick) && ch == '\\' {
            escaped = true;
            i += ch_len;
            continue;
        }

        // When outside all string literals, check for comment delimiters
        if !in_double_quote && !in_single_quote && !in_backtick {
            let mut matched_multiline = false;
            for &(opener, closer) in MULTI_LINE_COMMENT_DELIMITERS {
                if let Some(after_opener) = rest.strip_prefix(opener) {
                    matched_multiline = true;
                    if let Some(close_pos) = after_opener.find(closer) {
                        i += opener.len() + close_pos + closer.len();
                    } else {
                        return false;
                    }
                    break;
                }
            }
            if matched_multiline {
                continue;
            }

            for &delim in SINGLE_LINE_COMMENT_DELIMITERS {
                if rest.starts_with(delim) {
                    // For single-char delimiters that double as code operators (; in C/Rust, % in C/Rust modulo),
                    // verify they are preceded by whitespace or occur at the start of the line.
                    if delim == ";" || delim == "%" {
                        let is_at_line_start_or_after_ws = i == 0
                            || match prefix.get(..i) {
                                Some(before) => before.ends_with(char::is_whitespace),
                                None => false,
                            };
                        if !is_at_line_start_or_after_ws {
                            continue;
                        }
                    }
                    return false;
                }
            }
        }

        // String literal toggles
        if !in_single_quote && !in_backtick && ch == '"' {
            in_double_quote = !in_double_quote;
            i += ch_len;
            continue;
        }

        if !in_double_quote && !in_backtick && ch == '\'' {
            in_single_quote = !in_single_quote;
            i += ch_len;
            continue;
        }

        if !in_double_quote && !in_single_quote && ch == '`' {
            in_backtick = !in_backtick;
            i += ch_len;
            continue;
        }

        i += ch_len;
    }

    if in_double_quote || in_single_quote || in_backtick {
        return true;
    }

    // If not in quotes and no comment delimiter was matched, it's un-commented plain text
    true
}

/// Normalizes a file path relative to a base workspace root if specified.
fn normalize_source_path(path: &Path, base_root: Option<&Path>) -> PathBuf {
    match base_root {
        Some(root) => match path.strip_prefix(root) {
            Ok(rel) => rel.to_path_buf(),
            Err(_) => path.to_path_buf(),
        },
        None => path.to_path_buf(),
    }
}

/// Scans a single line of text for `@trace` annotations and extracts all declared `TraceLink`s.
///
/// Line number is 1-indexed. Multiple comma- or semicolon-separated IDs on the same line
/// yield distinct `TraceLink` instances sharing the same provenance and classification.
pub fn scan_comment_line(
    line: &str,
    line_number: usize,
    file_path: &Path,
    default_kind: Option<LinkKind>,
) -> Vec<TraceLink> {
    let mut links = Vec::new();

    for cap in TRACE_REGEX.captures_iter(line) {
        // Find exact start position of '@' in the match to verify string literal exclusion
        let full_match = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        let match_str = full_match.as_str();
        let at_offset = match_str.to_ascii_lowercase().find("@trace").unwrap_or(0);
        let at_trace_pos = full_match.start() + at_offset;

        if is_string_literal(line, at_trace_pos) {
            continue;
        }

        let tag = cap.name("tag").map(|m| m.as_str());
        let kind = classify_link_kind(tag, file_path, default_kind);

        let ids_str = match cap.name("ids") {
            Some(m) => m.as_str(),
            None => continue,
        };

        // Split on comma or semicolon
        for piece in ids_str.split([',', ';']) {
            let cleaned = clean_id_token(piece);
            if cleaned.is_empty() {
                continue;
            }

            match NodeId::new(cleaned) {
                Ok(node_id) => {
                    links.push(TraceLink::new(
                        node_id,
                        file_path.to_path_buf(),
                        line_number,
                        kind,
                    ));
                }
                Err(err) => {
                    eprintln!(
                        "warning: invalid identifier '{}' in comment at {}:{}: {}",
                        cleaned,
                        file_path.display(),
                        line_number,
                        err
                    );
                }
            }
        }
    }

    links
}

/// Scans an in-memory UTF-8 text string for `@trace` comment annotations.
///
/// Records 1-indexed line numbers and normalizes the stored file path relative to `base_root`.
pub fn scan_comments_text(
    text: &str,
    file_path: &Path,
    base_root: Option<&Path>,
    default_kind: Option<LinkKind>,
) -> Vec<TraceLink> {
    let normalized_path = normalize_source_path(file_path, base_root);
    let mut links = Vec::new();

    for (idx, line) in text.lines().enumerate() {
        let line_number = idx + 1;
        let mut line_links = scan_comment_line(line, line_number, &normalized_path, default_kind);
        links.append(&mut line_links);
    }

    links
}

/// Reads a source code or test file from disk and scans it for `@trace` comment annotations.
///
/// Returns `Err(ParseError::Io)` if the file cannot be opened or read.
pub fn scan_comments_file(
    file_path: &Path,
    base_root: Option<&Path>,
    default_kind: Option<LinkKind>,
) -> Result<Vec<TraceLink>, ParseError> {
    let content = fs::read_to_string(file_path).map_err(|err| ParseError::Io {
        path: file_path.to_path_buf(),
        source: err,
    })?;

    Ok(scan_comments_text(
        &content,
        file_path,
        base_root,
        default_kind,
    ))
}

/// Batch-scans a collection of workspace source and test files, returning all extracted `TraceLink`s.
pub fn scan_workspace_comments(
    files: &[PathBuf],
    base_root: Option<&Path>,
    default_kind: Option<LinkKind>,
) -> Result<Vec<TraceLink>, ParseError> {
    let mut all_links = Vec::new();

    for file in files {
        let mut file_links = scan_comments_file(file, base_root, default_kind)?;
        all_links.append(&mut file_links);
    }

    Ok(all_links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_id_token_strips_punctuation_and_delimiters() {
        assert_eq!(clean_id_token("REQ-01"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01;"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01,"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01*/"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01-->"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01; */"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01, */"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01)"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01\"]"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01\"\"\""), "REQ-01");
        assert_eq!(clean_id_token("REQ-01*)"), "REQ-01");
        assert_eq!(clean_id_token("REQ-01-}"), "REQ-01");
        assert_eq!(clean_id_token("SPEC-AUTH-LOGIN"), "SPEC-AUTH-LOGIN");
    }

    #[test]
    fn test_is_test_file_path_heuristics() {
        assert!(is_test_file_path(Path::new("tests/cli/test_init.rs")));
        assert!(is_test_file_path(Path::new(
            "tests/scanner/test_comments.rs"
        )));
        assert!(is_test_file_path(Path::new("test/unit/foo.py")));
        assert!(is_test_file_path(Path::new("specs/model_spec.rb")));
        assert!(is_test_file_path(Path::new("src/foo_test.go")));
        assert!(is_test_file_path(Path::new("src/test_bar.py")));
        assert!(is_test_file_path(Path::new("src/user.spec.ts")));
        assert!(is_test_file_path(Path::new("src/user.test.jsx")));
        assert!(is_test_file_path(Path::new("src/UserTest.java")));
        assert!(is_test_file_path(Path::new("src/UserTests.cs")));

        assert!(!is_test_file_path(Path::new("src/scanner/comments.rs")));
        assert!(!is_test_file_path(Path::new("src/lib.rs")));
        assert!(!is_test_file_path(Path::new("src/main.rs")));
        assert!(!is_test_file_path(Path::new("lib/engine.c")));
    }

    #[test]
    fn test_classify_link_kind_tag_overrides() {
        let src_path = Path::new("src/foo.rs");
        let test_path = Path::new("tests/foo.rs");

        // Untagged follows path
        assert_eq!(
            classify_link_kind(None, src_path, None),
            LinkKind::SourceCode
        );
        assert_eq!(
            classify_link_kind(None, test_path, None),
            LinkKind::AutomatedTest
        );

        // Tag override in src
        assert_eq!(
            classify_link_kind(Some("test"), src_path, None),
            LinkKind::AutomatedTest
        );
        assert_eq!(
            classify_link_kind(Some("TEST"), src_path, None),
            LinkKind::AutomatedTest
        );

        // Tag override in tests
        assert_eq!(
            classify_link_kind(Some("code"), test_path, None),
            LinkKind::SourceCode
        );
        assert_eq!(
            classify_link_kind(Some("CODE"), test_path, None),
            LinkKind::SourceCode
        );
    }

    #[test]
    fn test_polyglot_comment_styles() {
        let path = Path::new("src/sample.rs");

        // Rust line comment
        let links = scan_comment_line("// @trace REQ-01", 10, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-01");
        assert_eq!(links[0].line, 10);
        assert_eq!(links[0].kind, LinkKind::SourceCode);

        // Rust inner doc comment
        let links = scan_comment_line("//! // @trace REQ-DOC-PARSE", 3, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-DOC-PARSE");

        // C-style block comment
        let links = scan_comment_line("/* @trace REQ-01 */", 15, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-01");

        // Multi-line C-style continuation
        let links = scan_comment_line(" * @trace REQ-02", 20, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-02");

        // Python / Shell hash comment
        let links = scan_comment_line("# @trace REQ-03", 5, Path::new("script.py"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-03");

        // SQL dash comment
        let links = scan_comment_line("-- @trace REQ-04;", 8, Path::new("query.sql"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-04");

        // Assembly semicolon comment
        let links = scan_comment_line("; @trace REQ-05", 12, Path::new("boot.asm"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-05");

        // Erlang percent comment
        let links = scan_comment_line("% @trace REQ-06", 18, Path::new("server.erl"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-06");

        // HTML comment
        let links = scan_comment_line("<!-- @trace SPEC-01 -->", 25, Path::new("index.html"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "SPEC-01");

        // Windows Batch REM
        let links = scan_comment_line("rem @trace REQ-07", 30, Path::new("build.bat"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-07");

        // Python docstring
        let links = scan_comment_line("\"\"\" @trace REQ-08 \"\"\"", 35, Path::new("app.py"), None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-08");
    }

    #[test]
    fn test_comma_and_semicolon_separated_ids() {
        let path = Path::new("src/engine.rs");
        let links = scan_comment_line("// @trace REQ-01, REQ-02, SPEC-03", 42, path, None);
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target_id().as_str(), "REQ-01");
        assert_eq!(links[1].target_id().as_str(), "REQ-02");
        assert_eq!(links[2].target_id().as_str(), "SPEC-03");
        for link in &links {
            assert_eq!(link.line, 42);
            assert_eq!(link.kind, LinkKind::SourceCode);
        }

        // Semicolon separation
        let links = scan_comment_line("// @trace REQ-A; REQ-B;", 50, path, None);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target_id().as_str(), "REQ-A");
        assert_eq!(links[1].target_id().as_str(), "REQ-B");
    }

    #[test]
    fn test_skips_string_literals() {
        let path = Path::new("src/main.rs");

        // Code assigning @trace as string literal must be skipped
        let links = scan_comment_line("let marker = \"@trace REQ-01\";", 10, path, None);
        assert!(links.is_empty(), "String literal must be skipped");

        let links = scan_comment_line("printf('@trace REQ-02')", 11, path, None);
        assert!(
            links.is_empty(),
            "Single-quoted string literal must be skipped"
        );

        // Comment containing quotes must NOT be skipped
        let links = scan_comment_line("// \"important\" @trace REQ-03", 12, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-03");
    }

    #[test]
    fn test_case_insensitivity() {
        let path = Path::new("tests/sample.rs");

        let links = scan_comment_line("// @TRACE REQ-01", 1, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-01");

        let links = scan_comment_line("// @Trace:Code SPEC-02", 2, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "SPEC-02");
        assert_eq!(links[0].kind, LinkKind::SourceCode);

        let links = scan_comment_line("// @TRACE:TEST REQ-03", 3, path, None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id().as_str(), "REQ-03");
        assert_eq!(links[0].kind, LinkKind::AutomatedTest);
    }

    #[test]
    fn test_string_literals_with_comment_markers_and_markdown_tables_are_excluded() {
        let path = Path::new("src/main.rs");

        // 1. String literals containing comment markers must be excluded
        let fp_cases = [
            "let s = \"// @trace REQ-FP-01\";",
            "println!(\"// @trace REQ-FP-02\");",
            "query = \"SELECT * -- @trace REQ-FP-03\"",
            "note = '# @trace REQ-FP-04'",
            "let s = \"@trace REQ-FP-05\";",
            "let s = '@trace REQ-FP-06';",
            "let s = `@trace REQ-FP-07`;",
            "| Feature | @trace REQ-FP-08 |",
            "let x = 10; @trace REQ-FP-09;",
            "/* block */ let s = \"@trace REQ-FP-10\";",
            "/* block */ let s = \"// @trace REQ-FP-11\";",
            "let email = \"contact@trace.org\";",
        ];

        for line in fp_cases {
            let links = scan_comment_line(line, 1, path, None);
            assert!(
                links.is_empty(),
                "Expected no links extracted for false-positive case '{}', got {:?}",
                line,
                links
            );
        }

        // 2. Genuine comments must be preserved and extracted
        let valid_cases = [
            ("// @trace REQ-VALID-01", "REQ-VALID-01"),
            ("/* @trace REQ-VALID-02 */", "REQ-VALID-02"),
            (" * @trace REQ-VALID-03", "REQ-VALID-03"),
            ("/// @trace REQ-VALID-04", "REQ-VALID-04"),
            ("//! @trace REQ-VALID-05", "REQ-VALID-05"),
            ("# @trace REQ-VALID-06", "REQ-VALID-06"),
            ("-- @trace REQ-VALID-07", "REQ-VALID-07"),
            ("; @trace REQ-VALID-08", "REQ-VALID-08"),
            ("% @trace REQ-VALID-09", "REQ-VALID-09"),
            ("rem @trace REQ-VALID-10", "REQ-VALID-10"),
            ("REM @trace REQ-VALID-11", "REQ-VALID-11"),
            ("<!-- @trace REQ-VALID-12 -->", "REQ-VALID-12"),
            ("\"\"\" @trace REQ-VALID-13 \"\"\"", "REQ-VALID-13"),
            ("''' @trace REQ-VALID-14 '''", "REQ-VALID-14"),
            ("(* @trace REQ-VALID-15 *)", "REQ-VALID-15"),
            ("{- @trace REQ-VALID-16 -}", "REQ-VALID-16"),
            ("let x = 10; // @trace REQ-VALID-17", "REQ-VALID-17"),
            ("/* c */ // @trace REQ-VALID-18", "REQ-VALID-18"),
            ("// \"important\" @trace REQ-VALID-19", "REQ-VALID-19"),
            ("// Don't skip @trace REQ-VALID-20", "REQ-VALID-20"),
        ];

        for (line, expected_id) in valid_cases {
            let links = scan_comment_line(line, 1, path, None);
            assert_eq!(
                links.len(),
                1,
                "Expected 1 link for valid comment '{}', got {:?}",
                line,
                links
            );
            assert_eq!(links[0].target_id().as_str(), expected_id);
        }
    }
}
