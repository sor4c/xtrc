//! Ignore-aware filesystem traversal and streaming binary asset detection.
//!
//! // @trace SPEC-DISC-TRAVERSAL, SPEC-PERF-BOUNDS, REQ-IGNORE-RULES, REQ-NFR-PERF

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use ignore::overrides::OverrideBuilder;
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::config::WorkspaceConfig;
use crate::error::ConfigError;

/// Number of initial bytes to inspect for null bytes to detect binary files.
pub const BINARY_SNIFF_BUFFER_SIZE: usize = 512;

/// Built-in directory names that must be pruned immediately without descent.
const BUILTIN_SKIP_DIRS: &[&str] = &[".git", "target", "node_modules"];

/// Substring matching agent collaboration metadata directories that must never be scanned.
const BUILTIN_SKIP_PATH_SUBSTRING: &str = ".agents/teamwork";

/// Result of classifying workspace text files by their role.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CategorizedFiles {
    /// Specification files matching `specs.include` and not `specs.exclude`.
    pub specs: Vec<PathBuf>,
    /// Source code files matching `code.include` with configured extensions.
    pub code: Vec<PathBuf>,
    /// Test files matching `tests.include`.
    pub tests: Vec<PathBuf>,
    /// Remaining discovered text files not matched by `specs`, `code`, or `tests`.
    pub other: Vec<PathBuf>,
    /// All discovered text files in the workspace.
    pub all_text: Vec<PathBuf>,
}

/// Checks whether a byte slice contains any null bytes (`0x00`).
///
/// Under POSIX and standard VCS conventions, text files (UTF-8, ASCII) do not contain null bytes.
/// Any null byte indicates a binary asset (compiled binary, object, image, compressed archive, etc.).
#[inline]
pub fn is_binary_buffer(buf: &[u8]) -> bool {
    buf.contains(&0x00)
}

/// Inspects the first [`BINARY_SNIFF_BUFFER_SIZE`] (512) bytes of a file to check for binary content.
///
/// Returns `Ok(true)` if a null byte is found, `Ok(false)` if no null byte is found,
/// or `Err(std::io::Error)` if the file cannot be opened or read.
pub fn sniff_binary(path: &Path) -> Result<bool, std::io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; BINARY_SNIFF_BUFFER_SIZE];
    let bytes_read = file.read(&mut buffer)?;
    Ok(is_binary_buffer(&buffer[..bytes_read]))
}

/// Resiliently checks if a candidate file is binary.
///
/// If an I/O error occurs (e.g., permission denied, broken symlink, or unreadable file),
/// logs a non-fatal warning to `stderr` and returns `true` (skipping the candidate file
/// so traversal continues uninterrupted without aborting).
pub fn is_binary_file(path: &Path) -> bool {
    match sniff_binary(path) {
        Ok(is_bin) => is_bin,
        Err(err) => {
            eprintln!("warning: unable to read '{}': {}", path.display(), err);
            true
        }
    }
}

/// Evaluates whether a directory or file entry should be pruned during traversal.
///
/// Checks directory names against built-in skip list (`.git`, `target`, `node_modules`)
/// and path substrings (`.agents/teamwork`).
pub fn should_skip_entry(entry: &DirEntry) -> bool {
    let file_name = entry.file_name();
    for &skip_dir in BUILTIN_SKIP_DIRS {
        if file_name == skip_dir {
            return true;
        }
    }

    let path_str = entry.path().to_string_lossy();
    if path_str.contains(BUILTIN_SKIP_PATH_SUBSTRING) || path_str.contains(".agents\\teamwork") {
        return true;
    }

    // Skip nested git repositories / submodules (depth > 0 containing a .git directory or file)
    if entry.depth() > 0 && entry.file_type().is_some_and(|ft| ft.is_dir()) {
        let git_marker = entry.path().join(".git");
        if git_marker.exists() {
            return true;
        }
    }

    false
}

/// Configures and builds an [`ignore::WalkBuilder`] with all standard ignore rules,
/// built-in skip lists, and optional user overrides.
pub fn create_walk_builder(
    root: &Path,
    config: Option<&WorkspaceConfig>,
) -> Result<WalkBuilder, ConfigError> {
    let mut builder = WalkBuilder::new(root);

    builder
        .hidden(true) // Skip hidden files/directories (.git, .vscode, etc.)
        .git_ignore(true) // Respect root and nested .gitignore files
        .git_exclude(true) // Respect .git/info/exclude
        .git_global(false) // Deterministic scans across development machines and CI
        .parents(true) // Respect parent directory ignore rules
        .require_git(false) // Respect .gitignore even if directory lacks a .git folder
        .filter_entry(|entry| !should_skip_entry(entry));

    // Apply custom exclude rules from configuration if present
    if let Some(cfg) = config {
        if !cfg.specs.exclude.is_empty() {
            let mut override_builder = OverrideBuilder::new(root);
            for pattern in &cfg.specs.exclude {
                let rule = if pattern.starts_with('!') {
                    pattern.to_string()
                } else {
                    format!("!{pattern}")
                };
                if let Err(err) = override_builder.add(&rule) {
                    eprintln!("warning: invalid exclude pattern '{pattern}': {err}");
                }
            }
            if let Ok(overrides) = override_builder.build() {
                builder.overrides(overrides);
            }
        }
    }

    Ok(builder)
}

/// Performs a multi-core, ignore-aware filesystem traversal of the workspace directory.
///
/// Returns a sorted list of candidate text files, filtering out binary assets and ignored paths.
pub fn walk_workspace(root: &Path, config: &WorkspaceConfig) -> Result<Vec<PathBuf>, ConfigError> {
    let builder = create_walk_builder(root, Some(config))?;
    let walker = builder.build_parallel();
    let (tx, rx) = mpsc::channel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |entry_res| {
            match entry_res {
                Ok(entry) => {
                    if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        let path = entry.into_path();
                        if !is_binary_file(&path) {
                            let _ = tx.send(path);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("warning: traversal error: {err}");
                }
            }
            WalkState::Continue
        })
    });

    drop(tx);
    let mut paths: Vec<PathBuf> = rx.into_iter().collect();
    paths.sort();
    Ok(paths)
}

/// Performs a synchronous ignore-aware traversal of the workspace directory.
///
/// Useful in single-threaded contexts, unit tests, and deterministic debugging.
pub fn walk_workspace_sync(
    root: &Path,
    config: Option<&WorkspaceConfig>,
) -> Result<Vec<PathBuf>, ConfigError> {
    let builder = create_walk_builder(root, config)?;
    let mut paths = Vec::new();

    for entry_res in builder.build() {
        match entry_res {
            Ok(entry) => {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let path = entry.into_path();
                    if !is_binary_file(&path) {
                        paths.push(path);
                    }
                }
            }
            Err(err) => {
                eprintln!("warning: traversal error: {err}");
            }
        }
    }

    paths.sort();
    Ok(paths)
}

/// Compiles a slice of glob pattern strings into an optimized [`GlobSet`].
///
/// Normalizes path separators and enforces `literal_separator(true)` so that `*`
/// wildcards do not cross directory delimiters. Any syntactically invalid pattern
/// logs a warning to `stderr` and is omitted without panicking.
///
/// // @trace SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
pub fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let clean = pattern.replace('\\', "/");
        let clean = clean.strip_prefix("./").unwrap_or(&clean);
        match GlobBuilder::new(clean).literal_separator(true).build() {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(err) => {
                eprintln!("warning: invalid glob pattern '{pattern}': {err}");
            }
        }
    }
    match builder.build() {
        Ok(set) => set,
        Err(err) => {
            eprintln!("warning: failed to build glob set: {err}");
            GlobSet::empty()
        }
    }
}

/// Partitions discovered candidate files into specification, source code, test, and other categories
/// in a single $O(N)$ pass.
///
/// // @trace SPEC-DISC-TRAVERSAL, SPEC-PERF-BOUNDS, REQ-IGNORE-RULES, REQ-NFR-PERF
pub fn categorize_files(
    root: &Path,
    all_files: &[PathBuf],
    config: &WorkspaceConfig,
) -> CategorizedFiles {
    let mut categorized = CategorizedFiles {
        specs: Vec::new(),
        code: Vec::new(),
        tests: Vec::new(),
        other: Vec::new(),
        all_text: all_files.to_vec(),
    };

    let specs_include = build_glob_set(&config.specs.include);
    let specs_exclude = build_glob_set(&config.specs.exclude);
    let tests_include = build_glob_set(&config.tests.include);
    let code_include = build_glob_set(&config.code.include);

    for path in all_files {
        let rel_path = match path.strip_prefix(root) {
            Ok(p) => p,
            Err(_) => path,
        };
        let rel_str = rel_path.to_string_lossy();
        let normalized_cow = if rel_str.contains('\\') {
            std::borrow::Cow::Owned(rel_str.replace('\\', "/"))
        } else {
            rel_str
        };
        let clean_rel = normalized_cow.strip_prefix("./").unwrap_or(&normalized_cow);

        // 1. Check specs match
        if specs_include.is_match(clean_rel) && !specs_exclude.is_match(clean_rel) {
            categorized.specs.push(path.clone());
            continue;
        }

        // 2. Check tests match
        if tests_include.is_match(clean_rel) {
            categorized.tests.push(path.clone());
            continue;
        }

        // 3. Check code match & extension
        if code_include.is_match(clean_rel) && has_matching_extension(path, &config.code.extensions)
        {
            categorized.code.push(path.clone());
            continue;
        }

        // 4. Remaining uncategorized workspace text files
        categorized.other.push(path.clone());
    }

    categorized
}

/// Checks if a relative path matches any glob pattern in the provided slice.
///
/// // @trace SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
pub fn matches_any_glob(path_str: &str, patterns: &[String]) -> bool {
    let clean_path = path_str.replace('\\', "/");
    let clean_path = clean_path.strip_prefix("./").unwrap_or(&clean_path);
    for pattern in patterns {
        if glob_match(pattern, clean_path) {
            return true;
        }
    }
    false
}

/// Wildcard glob matcher supporting `*`, `**`, `?`, character classes, and cross-platform path separators.
///
/// Employs genuine glob parsing with `literal_separator(true)` so that `*` and `?`
/// are restricted to a single path component and only `**` traverses directory boundaries.
///
/// // @trace SPEC-DISC-TRAVERSAL, REQ-IGNORE-RULES
pub fn glob_match(pattern: &str, path_str: &str) -> bool {
    let clean_pattern = pattern.replace('\\', "/");
    let clean_pattern = clean_pattern.strip_prefix("./").unwrap_or(&clean_pattern);
    let clean_path = path_str.replace('\\', "/");
    let clean_path = clean_path.strip_prefix("./").unwrap_or(&clean_path);

    match GlobBuilder::new(clean_pattern)
        .literal_separator(true)
        .build()
    {
        Ok(glob) => glob.compile_matcher().is_match(clean_path),
        Err(_) => false,
    }
}

/// Checks if a file path matches any of the configured file extensions without heap allocation.
fn has_matching_extension(path: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        extensions
            .iter()
            .any(|configured| configured.strip_prefix('.').unwrap_or(configured) == ext)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary_buffer() {
        assert!(!is_binary_buffer(b"Hello, world!\n"));
        assert!(!is_binary_buffer(b"pub fn test() {}"));
        assert!(is_binary_buffer(&[0x00]));
        assert!(is_binary_buffer(&[0x48, 0x65, 0x00, 0x6C, 0x6F]));
    }

    #[test]
    fn test_glob_match_false_positive_resistance() {
        // Directory prefix boundary collisions (Reviewer 1 critical finding)
        assert!(!glob_match("docs/**/*.md", "docstrings/foo.md"));
        assert!(!glob_match("target/**", "target_dir/foo.rs"));
        assert!(!glob_match("src/**/*.rs", "src_generated/code.rs"));
        assert!(!glob_match("tests/**", "tests_integration/foo.rs"));
        assert!(!glob_match("specs/**/*.md", "my_specs/a.md"));
        assert!(!glob_match("docs/**", "docs.txt"));

        // Single wildcard '/' boundary enforcement
        assert!(!glob_match("docs/*.md", "docs/sub/nested.md"));
        assert!(!glob_match("docs/*.md", "docstrings/foo.md"));

        // Suffix and extension false positives
        assert!(!glob_match("docs/**/*.md", "docs/file.mdx"));
        assert!(!glob_match("docs/**/*.md", "docs/file.md.bak"));
        assert!(!glob_match("*.rs", "main.rs.old"));
        assert!(!glob_match("src/**/*.rs", "src/main.rs.bak"));
        assert!(!glob_match("sub/dir/**", "sub/directory/file.txt"));
    }

    #[test]
    fn test_glob_match_polyglot_extensions() {
        // Rust (.rs)
        assert!(glob_match("src/**/*.rs", "src/lib.rs"));
        assert!(glob_match("src/**/*.rs", "src/scanner/traversal.rs"));
        assert!(glob_match("src/**/*.rs", "src/sub/deep/module.rs"));
        assert!(!glob_match("src/**/*.rs", "src/main.py"));

        // Python (.py)
        assert!(glob_match("src/**/*.py", "src/main.py"));
        assert!(glob_match("src/**/*.py", "src/utils/math.py"));
        assert!(glob_match("scripts/**/*.py", "scripts/deploy/build.py"));
        assert!(!glob_match("src/**/*.py", "src/main.rs"));

        // TypeScript & TSX (.ts, .tsx)
        assert!(glob_match("frontend/**/*.ts", "frontend/src/index.ts"));
        assert!(glob_match(
            "frontend/**/*.ts",
            "frontend/src/components/Button.ts"
        ));
        assert!(glob_match(
            "frontend/**/*.tsx",
            "frontend/src/components/Button.tsx"
        ));
        assert!(!glob_match(
            "frontend/**/*.ts",
            "frontend/src/components/Button.tsx"
        ));

        // Go (.go)
        assert!(glob_match("backend/**/*.go", "backend/server.go"));
        assert!(glob_match("backend/**/*.go", "backend/pkg/auth/token.go"));
        assert!(!glob_match("backend/**/*.go", "backend/server.rs"));

        // Markdown variants (.md, .markdown)
        assert!(glob_match("docs/**/*.md", "docs/requirements.md"));
        assert!(glob_match("docs/**/*.md", "docs/sub/requirements.md"));
        assert!(glob_match("docs/**/*.markdown", "docs/design.markdown"));
        assert!(!glob_match("docs/**/*.markdown", "docs/design.md"));
        assert!(!glob_match("docs/**/*.md", "docs/design.markdown"));

        // Structured formats (.yaml, .yml, .toml, .json)
        assert!(glob_match("schemas/**/*.yaml", "schemas/v1/config.yaml"));
        assert!(glob_match("schemas/**/*.yml", "schemas/v1/config.yml"));
        assert!(glob_match("configs/**/*.toml", "configs/production.toml"));
        assert!(glob_match("data/**/*.json", "data/fixtures/test.json"));
        assert!(!glob_match("schemas/**/*.yaml", "schemas/v1/config.json"));
    }

    #[test]
    fn test_glob_match_wildcard_variations() {
        // Exact filenames
        assert!(glob_match("README.md", "README.md"));
        assert!(glob_match("docs/requirements.md", "docs/requirements.md"));
        assert!(!glob_match("docs/requirements.md", "docs/design.md"));
        assert!(!glob_match(
            "docs/requirements.md",
            "sub/docs/requirements.md"
        ));

        // Single-level wildcard (*)
        assert!(glob_match("docs/*.md", "docs/requirements.md"));
        assert!(!glob_match("docs/*.md", "docs/sub/requirements.md"));
        assert!(glob_match("*.md", "README.md"));
        assert!(!glob_match("*.md", "docs/README.md"));
        assert!(glob_match("src/*/*.rs", "src/scanner/traversal.rs"));
        assert!(!glob_match("src/*/*.rs", "src/lib.rs"));
        assert!(!glob_match("src/*/*.rs", "src/a/b/c.rs"));

        // Recursive wildcard (**)
        assert!(glob_match("**/*", "anything.txt"));
        assert!(glob_match("**/*", "a/b/c/anything.txt"));
        assert!(glob_match("**", "anything.txt"));
        assert!(glob_match("**", "a/b/c/anything.txt"));
        assert!(glob_match("docs/**", "docs/requirements.md"));
        assert!(glob_match("docs/**", "docs/sub/deep/spec.md"));
        assert!(!glob_match("docs/**", "docstrings/foo.md"));

        // Prefix and suffix wildcards
        assert!(glob_match("**/test_*.rs", "test_main.rs"));
        assert!(glob_match("**/test_*.rs", "tests/test_cli.rs"));
        assert!(glob_match("**/test_*.rs", "tests/unit/test_deep.rs"));
        assert!(!glob_match("**/test_*.rs", "tests/unit/deep_test.rs"));
        assert!(glob_match("tests/**/*.*", "tests/cli/test_init.rs"));
        assert!(glob_match("tests/**/*.*", "tests/data/config.json"));
        assert!(!glob_match("tests/**/*.*", "tests/no_extension_file"));

        // Single character (?) and character classes ([...])
        assert!(glob_match("docs/req-?.md", "docs/req-1.md"));
        assert!(glob_match("docs/req-?.md", "docs/req-a.md"));
        assert!(!glob_match("docs/req-?.md", "docs/req-10.md"));
        assert!(glob_match("docs/req-[0-9].md", "docs/req-5.md"));
        assert!(!glob_match("docs/req-[0-9].md", "docs/req-a.md"));
    }

    #[test]
    fn test_glob_match_path_normalization() {
        // Windows backslash normalization
        assert!(glob_match("docs/**/*.md", r"docs\requirements.md"));
        assert!(glob_match("docs/**/*.md", r"docs\sub\requirements.md"));
        assert!(glob_match(r"docs\**\*.md", "docs/sub/requirements.md"));
        assert!(glob_match(r"docs\*.md", r"docs\spec.md"));
        assert!(!glob_match("docs/**/*.md", r"docstrings\foo.md"));
    }

    #[test]
    fn test_categorize_files() {
        let root = Path::new("/workspace");
        let files = vec![
            PathBuf::from("/workspace/docs/requirements.md"),
            PathBuf::from("/workspace/docstrings/note.md"),
            PathBuf::from("/workspace/src/lib.rs"),
            PathBuf::from("/workspace/src/script.py"),
            PathBuf::from("/workspace/src/notes.txt"),
            PathBuf::from("/workspace/tests/cli/test_init.rs"),
            PathBuf::from("/workspace/.agents/teamwork/plan.md"),
        ];

        let mut config = WorkspaceConfig::default();
        config.specs.include = vec!["docs/**/*.md".to_string()];
        config.specs.exclude = vec![".agents/**".to_string()];
        config.code.include = vec!["src/**/*.*".to_string()];
        config.code.extensions = vec![".rs".to_string(), ".py".to_string()];
        config.tests.include = vec!["tests/**/*.*".to_string()];

        let categorized = categorize_files(root, &files, &config);

        assert_eq!(
            categorized.specs,
            vec![PathBuf::from("/workspace/docs/requirements.md")]
        );
        // docstrings/note.md must NOT be included in specs
        assert!(!categorized
            .specs
            .contains(&PathBuf::from("/workspace/docstrings/note.md")));

        // code must include .rs and .py, but exclude .txt
        assert!(categorized
            .code
            .contains(&PathBuf::from("/workspace/src/lib.rs")));
        assert!(categorized
            .code
            .contains(&PathBuf::from("/workspace/src/script.py")));
        assert!(!categorized
            .code
            .contains(&PathBuf::from("/workspace/src/notes.txt")));

        // tests must include test_init.rs
        assert_eq!(
            categorized.tests,
            vec![PathBuf::from("/workspace/tests/cli/test_init.rs")]
        );
    }

    #[test]
    fn test_has_matching_extension() {
        let extensions = vec![".rs".to_string(), ".py".to_string()];
        assert!(has_matching_extension(
            Path::new("src/main.rs"),
            &extensions
        ));
        assert!(has_matching_extension(Path::new("script.py"), &extensions));
        assert!(!has_matching_extension(Path::new("doc.md"), &extensions));
    }
}
