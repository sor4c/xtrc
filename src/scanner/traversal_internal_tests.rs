// @trace:test SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES, REQ-NFR-PERF

use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;
use tempfile::tempdir;

use xtrc::config::WorkspaceConfig;
use xtrc::scanner::traversal::{
    build_glob_set, categorize_files, is_binary_file, sniff_binary, walk_workspace,
};

fn glob_match(pattern: &str, path: &str) -> bool {
    let patterns = [pattern.to_string()];
    build_glob_set(&patterns).is_match(path.replace('\\', "/"))
}

#[test]
fn test_binary_sniff_empty_file() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("empty.txt");
    File::create(&file_path).expect("create file failed");

    let is_bin = sniff_binary(&file_path).expect("sniff failed");
    assert!(!is_bin, "Empty file must be classified as text, not binary");
    assert!(!is_binary_file(&file_path));
}

#[test]
fn test_binary_sniff_ascii_text() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("sample.rs");
    let mut file = File::create(&file_path).expect("create file failed");
    writeln!(file, "pub fn add(a: i32, b: i32) -> i32 {{ a + b }}").expect("write failed");

    let is_bin = sniff_binary(&file_path).expect("sniff failed");
    assert!(
        !is_bin,
        "Standard Rust source code must be classified as text"
    );
}

#[test]
fn test_binary_sniff_utf8_multibyte() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("doc.md");
    let mut file = File::create(&file_path).expect("create file failed");
    writeln!(file, "# 需求规范: 敏捷追踪系统 🚀").expect("write failed");

    let is_bin = sniff_binary(&file_path).expect("sniff failed");
    assert!(
        !is_bin,
        "UTF-8 multi-byte content must be classified as text"
    );
}

#[test]
fn test_binary_sniff_null_byte_at_start() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("binary.bin");
    let mut file = File::create(&file_path).expect("create file failed");
    file.write_all(&[0x00, 0x01, 0x02, 0x03])
        .expect("write failed");

    let is_bin = sniff_binary(&file_path).expect("sniff failed");
    assert!(
        is_bin,
        "File starting with 0x00 must be classified as binary"
    );
}

#[test]
fn test_binary_sniff_null_byte_at_boundary_511() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("boundary.bin");
    let mut data = vec![b'A'; 512];
    data[511] = 0x00; // Null byte at exact boundary of 512 buffer
    fs::write(&file_path, &data).expect("write failed");

    let is_bin = sniff_binary(&file_path).expect("sniff failed");
    assert!(
        is_bin,
        "File with null byte at index 511 must be classified as binary"
    );
}

#[test]
fn test_binary_sniff_streaming_large_file() {
    let dir = tempdir().expect("tempdir failed");
    let file_path = dir.path().join("large.bin");
    let mut data = vec![b'x'; 1024 * 1024]; // 1MB file
    data[100] = 0x00; // Null byte within the first 512 bytes
    fs::write(&file_path, &data).expect("write failed");

    let is_bin = sniff_binary(&file_path).expect("sniff failed");
    assert!(
        is_bin,
        "Large file with null byte in first 512 bytes must be classified as binary"
    );
}

#[test]
fn test_traversal_respects_root_gitignore() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    // Create root .gitignore
    fs::write(root.join(".gitignore"), "ignored.txt\nbuild/\n").expect("write .gitignore failed");

    // Create tracked and ignored files
    fs::write(root.join("tracked.rs"), "fn main() {}").expect("write failed");
    fs::write(root.join("ignored.txt"), "secret data").expect("write failed");
    fs::create_dir_all(root.join("build")).expect("create_dir failed");
    fs::write(root.join("build/artifact.rs"), "fn artifact() {}").expect("write failed");

    let config = WorkspaceConfig::default();
    let paths = walk_workspace(root, &config).expect("walk failed");

    assert!(
        paths.iter().any(|p| p.ends_with("tracked.rs")),
        "tracked.rs must be included"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("ignored.txt")),
        "ignored.txt must be excluded by .gitignore"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("artifact.rs")),
        "build/ must be excluded by .gitignore"
    );
}

#[test]
fn test_traversal_respects_nested_gitignore() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    let sub = root.join("packages/sub");
    fs::create_dir_all(&sub).expect("create_dir failed");
    fs::write(sub.join(".gitignore"), "*.log\n").expect("write failed");
    fs::write(sub.join("valid.rs"), "fn sub() {}").expect("write failed");
    fs::write(sub.join("app.log"), "log line").expect("write failed");

    let config = WorkspaceConfig::default();
    let paths = walk_workspace(root, &config).expect("walk failed");

    assert!(
        paths.iter().any(|p| p.ends_with("valid.rs")),
        "valid.rs must be included"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("app.log")),
        "app.log must be excluded by nested .gitignore"
    );
}

#[test]
fn test_traversal_skips_builtin_target_and_node_modules() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    let target = root.join("target/debug");
    fs::create_dir_all(&target).expect("create_dir failed");
    fs::write(target.join("built.rs"), "fn built() {}").expect("write failed");

    let node_modules = root.join("node_modules/package");
    fs::create_dir_all(&node_modules).expect("create_dir failed");
    fs::write(node_modules.join("index.js"), "console.log('hi');").expect("write failed");

    let src = root.join("src");
    fs::create_dir_all(&src).expect("create_dir failed");
    fs::write(src.join("lib.rs"), "pub fn ok() {}").expect("write failed");

    let config = WorkspaceConfig::default();
    let paths = walk_workspace(root, &config).expect("walk failed");

    assert!(
        paths.iter().any(|p| p.ends_with("lib.rs")),
        "src/lib.rs must be included"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("built.rs")),
        "target/ must be skipped by built-in skip-list"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("index.js")),
        "node_modules/ must be skipped by built-in skip-list"
    );
}

#[test]
fn test_traversal_skips_builtin_agents_teamwork() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    let agent_dir = root.join(".agents/teamwork/worker_1");
    fs::create_dir_all(&agent_dir).expect("create_dir failed");
    fs::write(agent_dir.join("plan.md"), "# Agent Plan").expect("write failed");

    let docs = root.join("docs");
    fs::create_dir_all(&docs).expect("create_dir failed");
    fs::write(docs.join("specs.md"), "# Spec").expect("write failed");

    let config = WorkspaceConfig::default();
    let paths = walk_workspace(root, &config).expect("walk failed");

    assert!(
        paths.iter().any(|p| p.ends_with("specs.md")),
        "docs/specs.md must be included"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("plan.md")),
        ".agents/teamwork/ must be skipped by built-in skip-list"
    );
}

#[test]
fn test_traversal_skips_binary_files_automatically() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    fs::write(root.join("hello.rs"), "fn main() {}").expect("write failed");
    fs::write(
        root.join("image.png"),
        [0x89, 0x50, 0x4E, 0x47, 0x00, 0x0D, 0x0A],
    )
    .expect("write failed");

    let config = WorkspaceConfig::default();
    let paths = walk_workspace(root, &config).expect("walk failed");

    assert!(
        paths.iter().any(|p| p.ends_with("hello.rs")),
        "hello.rs text must be included"
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("image.png")),
        "image.png binary must be skipped"
    );
}

#[test]
fn test_traversal_deterministic_ordering() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    for i in 0..20 {
        fs::write(
            root.join(format!("file_{i:02}.rs")),
            format!("fn f{i}() {{}}"),
        )
        .expect("write failed");
    }

    let config = WorkspaceConfig::default();
    let paths1 = walk_workspace(root, &config).expect("walk 1 failed");
    let paths2 = walk_workspace(root, &config).expect("walk 2 failed");

    assert_eq!(
        paths1, paths2,
        "Parallel walk results must be sorted deterministically"
    );
}

#[test]
fn test_traversal_performance_benchmark_under_500ms() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    // Create 1,000 files spread across 10 subdirectories
    for d in 0..10 {
        let sub = root.join(format!("dir_{d}"));
        fs::create_dir_all(&sub).expect("create_dir failed");
        for f in 0..100 {
            fs::write(sub.join(format!("file_{f}.rs")), "fn test() {}").expect("write failed");
        }
    }

    let config = WorkspaceConfig::default();
    let start = Instant::now();
    let paths = walk_workspace(root, &config).expect("walk failed");
    let elapsed = start.elapsed();

    assert_eq!(paths.len(), 1000, "Must discover all 1,000 text files");
    assert!(
        elapsed.as_millis() < 500,
        "Traversal took {:?}, exceeding the 500ms requirement of REQ-NFR-PERF",
        elapsed
    );
}

#[test]
fn test_integration_glob_match_boundary_semantics() {
    assert!(glob_match("docs/**/*.md", "docs/spec.md"));
    assert!(!glob_match("docs/**/*.md", "docstrings/spec.md"));
    assert!(glob_match("src/**/*.rs", "src/scanner/traversal.rs"));
    assert!(!glob_match("target/**", "target_dir/foo.rs"));
}

#[test]
fn test_categorize_files_false_positive_resistance() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    let valid_spec = root.join("docs/requirements.md");
    let false_spec = root.join("docstrings/foo.md");
    let valid_code = root.join("src/lib.rs");
    let false_code1 = root.join("src_gen/code.rs");
    let false_code2 = root.join("target_dir/foo.rs");
    let valid_test = root.join("tests/test_init.rs");
    let false_test = root.join("tests_perf/bench.rs");

    let all_files = vec![
        valid_spec.clone(),
        false_spec.clone(),
        valid_code.clone(),
        false_code1.clone(),
        false_code2.clone(),
        valid_test.clone(),
        false_test.clone(),
    ];

    let mut config = WorkspaceConfig::default();
    config.specs.include = vec!["docs/**/*.md".to_string()];
    config.specs.exclude = Vec::new();
    config.code.include = vec!["src/**/*.rs".to_string()];
    config.code.extensions = vec![".rs".to_string()];
    config.tests.include = vec!["tests/**".to_string()];

    let categorized = categorize_files(root, &all_files, &config);

    assert_eq!(
        categorized.specs,
        vec![valid_spec],
        "docstrings/foo.md must not be categorized into specs"
    );
    assert_eq!(
        categorized.code,
        vec![valid_code],
        "src_gen and target_dir must not be categorized into code"
    );
    assert_eq!(
        categorized.tests,
        vec![valid_test],
        "tests_perf must not be categorized into tests"
    );
}

#[test]
fn test_categorize_files_polyglot_language_support() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    let spec_md = root.join("docs/requirements.md");
    let spec_markdown = root.join("docs/sub/design.markdown");
    let code_rust = root.join("src/main.rs");
    let code_go = root.join("backend/server.go");
    let code_py = root.join("scripts/deploy.py");
    let code_ts = root.join("frontend/src/index.ts");
    let code_tsx = root.join("frontend/src/App.tsx");
    let test_rust = root.join("tests/cli/test_init.rs");
    let test_py = root.join("tests/e2e/test_runner.py");
    let auxiliary_txt = root.join("misc/notes.txt");
    let auxiliary_json = root.join("data/schema.json");

    let all_files = vec![
        spec_md.clone(),
        spec_markdown.clone(),
        code_rust.clone(),
        code_go.clone(),
        code_py.clone(),
        code_ts.clone(),
        code_tsx.clone(),
        test_rust.clone(),
        test_py.clone(),
        auxiliary_txt.clone(),
        auxiliary_json.clone(),
    ];

    let mut config = WorkspaceConfig::default();
    config.specs.include = vec!["docs/**/*.md".to_string(), "docs/**/*.markdown".to_string()];
    config.code.include = vec![
        "src/**/*.rs".to_string(),
        "backend/**/*.go".to_string(),
        "scripts/**/*.py".to_string(),
        "frontend/src/**/*.ts".to_string(),
        "frontend/src/**/*.tsx".to_string(),
    ];
    config.code.extensions = vec![
        ".rs".to_string(),
        ".go".to_string(),
        ".py".to_string(),
        ".ts".to_string(),
        ".tsx".to_string(),
    ];
    config.tests.include = vec!["tests/**/*.*".to_string()];

    let categorized = categorize_files(root, &all_files, &config);

    assert_eq!(
        categorized.specs.len(),
        2,
        "Expected 2 spec files (.md, .markdown)"
    );
    assert!(categorized.specs.contains(&spec_md));
    assert!(categorized.specs.contains(&spec_markdown));

    assert_eq!(categorized.code.len(), 5, "Expected 5 polyglot code files");
    assert!(categorized.code.contains(&code_rust));
    assert!(categorized.code.contains(&code_go));
    assert!(categorized.code.contains(&code_py));
    assert!(categorized.code.contains(&code_ts));
    assert!(categorized.code.contains(&code_tsx));

    assert_eq!(categorized.tests.len(), 2, "Expected 2 test files");
    assert!(categorized.tests.contains(&test_rust));
    assert!(categorized.tests.contains(&test_py));

    assert_eq!(
        categorized.specs.len()
            + categorized.code.len()
            + categorized.tests.len()
            + categorized.other.len(),
        11,
        "Each candidate text file must be assigned to exactly one category"
    );
}

#[test]
fn test_categorize_files_wildcard_depth_and_exclude_rules() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    let top_spec = root.join("docs/top_level.md");
    let deep_spec = root.join("docs/nested/deep.md");
    let excluded_spec = root.join("docs/scratch.md");
    let test_root = root.join("tests/test_root.rs");
    let test_deep = root.join("tests/unit/test_deep.rs");
    let test_helper = root.join("tests/unit/helper.rs");

    let all_files = vec![
        top_spec.clone(),
        deep_spec.clone(),
        excluded_spec.clone(),
        test_root.clone(),
        test_deep.clone(),
        test_helper.clone(),
    ];

    let mut config = WorkspaceConfig::default();
    config.specs.include = vec!["docs/*.md".to_string()]; // Single-level wildcard
    config.specs.exclude = vec!["docs/scratch.md".to_string()];
    config.tests.include = vec!["tests/**/test_*.rs".to_string()];

    let categorized = categorize_files(root, &all_files, &config);

    assert_eq!(
        categorized.specs,
        vec![top_spec],
        "Single-level glob must not include nested files, and exclude must prune scratch.md"
    );

    assert_eq!(
        categorized.tests.len(),
        2,
        "Pattern tests/**/test_*.rs must only match files starting with test_"
    );
    assert!(categorized.tests.contains(&test_root));
    assert!(categorized.tests.contains(&test_deep));
    assert!(!categorized.tests.contains(&test_helper));
}

#[test]
fn test_end_to_end_walk_and_categorize_polyglot() {
    let dir = tempdir().expect("tempdir failed");
    let root = dir.path();

    // 1. Setup gitignore
    fs::write(root.join(".gitignore"), "*.tmp\nbuild/\n").expect("write .gitignore failed");

    // 2. Setup ignored, binary, and pruned files
    let build_dir = root.join("build");
    fs::create_dir_all(&build_dir).expect("create_dir failed");
    fs::write(build_dir.join("artifact.rs"), "fn ignored() {}").expect("write failed");

    fs::write(root.join("scratch.tmp"), "temp data").expect("write failed");

    let git_dir = root.join(".git/objects");
    fs::create_dir_all(&git_dir).expect("create_dir failed");
    fs::write(git_dir.join("object.pack"), "pack data").expect("write failed");

    let teamwork_dir = root.join(".agents/teamwork/worker");
    fs::create_dir_all(&teamwork_dir).expect("create_dir failed");
    fs::write(teamwork_dir.join("plan.md"), "# Plan").expect("write failed");

    let assets_dir = root.join("assets");
    fs::create_dir_all(&assets_dir).expect("create_dir failed");
    let mut bin_data = vec![0x42u8; 100];
    bin_data[10] = 0x00; // null byte marks as binary
    fs::write(assets_dir.join("logo.png"), bin_data).expect("write failed");

    // 3. Setup valid polyglot files
    let docs_dir = root.join("docs/sub");
    fs::create_dir_all(&docs_dir).expect("create_dir failed");
    fs::write(root.join("docs/requirements.md"), "# Requirements").expect("write failed");
    fs::write(docs_dir.join("design.markdown"), "# Design").expect("write failed");

    let docstrings_dir = root.join("docstrings");
    fs::create_dir_all(&docstrings_dir).expect("create_dir failed");
    fs::write(docstrings_dir.join("foo.md"), "# Docstring").expect("write failed");

    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).expect("create_dir failed");
    fs::write(src_dir.join("lib.rs"), "pub fn ok() {}").expect("write failed");

    let backend_dir = root.join("backend");
    fs::create_dir_all(&backend_dir).expect("create_dir failed");
    fs::write(backend_dir.join("main.go"), "package main").expect("write failed");

    let tests_dir = root.join("tests");
    fs::create_dir_all(&tests_dir).expect("create_dir failed");
    fs::write(tests_dir.join("test_app.rs"), "#[test] fn t() {}").expect("write failed");

    // 4. Configure workspace
    let mut config = WorkspaceConfig::default();
    config.specs.include = vec!["docs/**/*.md".to_string(), "docs/**/*.markdown".to_string()];
    config.code.include = vec!["src/**/*.rs".to_string(), "backend/**/*.go".to_string()];
    config.code.extensions = vec![".rs".to_string(), ".go".to_string()];
    config.tests.include = vec!["tests/**".to_string()];

    // 5. Execute walk and categorize
    let discovered = walk_workspace(root, &config).expect("walk failed");
    let categorized = categorize_files(root, &discovered, &config);

    // 6. Verify pipeline outcome
    assert!(!discovered.iter().any(|p| p.ends_with("artifact.rs")));
    assert!(!discovered.iter().any(|p| p.ends_with("scratch.tmp")));
    assert!(!discovered.iter().any(|p| p.ends_with("object.pack")));
    assert!(!discovered.iter().any(|p| p.ends_with("plan.md")));
    assert!(!discovered.iter().any(|p| p.ends_with("logo.png")));

    // False positive resistance: docstrings/foo.md is text, but NOT a spec
    assert!(discovered.iter().any(|p| p.ends_with("foo.md")));
    assert!(
        !categorized.specs.iter().any(|p| p.ends_with("foo.md")),
        "docstrings/foo.md must not be categorized into specs"
    );

    // Correctly partitioned
    assert_eq!(categorized.specs.len(), 2);
    assert_eq!(categorized.code.len(), 2);
    assert_eq!(categorized.tests.len(), 1);
}
