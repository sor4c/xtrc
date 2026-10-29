---
document_type: design_specification
id: DES-SPEC-001
title: xtrc Architecture and Derived Technical Design Specification
version: 1.0.0
status: approved
governance_tracking:
  rule_1_issue: "#3"
  rule_2: "Requirements Drive Design"
  rule_3: "Intent Over Implementation"
traceability_layers:
  - Technical Specifications (SPEC-*)
  - Source Code Forward Declarations (src/)
  - Automated Tests Forward Declarations (tests/)
upstream_document: "docs/requirements.md"
schema_version: "1.0.0"
---

# xtrc Architecture and Derived Technical Design Specification

## 1. Architectural Blueprint & 7-Layer DAG Integration
This document establishes the technical architecture and derived design specifications for `xtrc`, a fast, plain-text, Git-native traceability CLI written in Rust. It fulfills Layer 6 (`SPEC-*`) and defines forward contracts for Layer 7 (`src/` Source Code and `tests/` Automated Tests), derived directly from `docs/requirements.md` in strict adherence to Rule 2 (Requirements Drive Design) and Rule 3 (Intent Over Implementation).

```
┌────────────────────────────────────────────────────────────────────────┐
│                        PROBLEM SPACE (Intent)                          │
│  Layer 1: Vision (VIS-*)                                               │
│  Layer 2: Strategy (STRAT-*)                                           │
│  Layer 3: Epics (EPIC-*)                                               │
│  Layer 4: User Stories (US-*)                                          │
│  Layer 5: Requirements (REQ-*) [Functional & Non-Functional]           │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Contract Boundary
┌───────────────────────────────────▼────────────────────────────────────┐
│                       SOLUTION SPACE (Realization)                     │
│  Layer 6: Technical Specifications (SPEC-*) [docs/design.md]           │
│    │                                                                   │
│    ├──► Forward Declaration: Source Code (src/**/*.rs // @trace <ID>)  │
│    └──► Forward Declaration: Automated Tests (tests/**/*.rs)           │
│  Layer 7: Implementation & Verification Leaves                         │
└────────────────────────────────────────────────────────────────────────┘
```

The system is organized into decoupled modules with strict boundaries:
- `cli`: Command-line interface definitions, subcommands, arguments, and terminal presentation.
- `config`: Workspace root discovery, configuration parsing, and default setting resolution.
- `scanner`: High-speed directory traversal, `.gitignore` filtering, and polyglot comment scanning.
- `parser`: Markdown YAML frontmatter extraction, inline header scanning, and ID grammar validation.
- `graph`: In-memory 7-layer directed graph representation, cycle detection, and bidirectional validation.
- `model`: Core domain entities, identifier types, link associations, and layer taxonomies.
- `error`: Strongly typed domain error enums (`thiserror`) and application-level context wrapping.

---

## 2. Workspace Discovery & Traversal Specifications

### SPEC-DISC-INIT: Workspace Initialization & Configuration Scaffolding
- **ID**: SPEC-DISC-INIT
- **Parent Requirements**: REQ-CLI-INIT, US-INIT-01
- **Component Target**: src/cli/init.rs
- **Verification Target**: tests/cli/test_init.rs
- **Technical Specification**:
  - The `init` command scaffolds `.xtrc.toml` at the project root and initializes starter specification directories (`docs/`).
  - Default `.xtrc.toml` schema:
    ```toml
    version = "1"
    [project]
    name = "xtrc"
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
    ```
  - The optional top-level `layers` array replaces the default hierarchy. Its entries specify
    `{ name, prefix }` in root-to-leaf order; if omitted, the built-in layers apply.
- **Validation & Behavioral Rules**:
  - If `.xtrc.toml` exists and `--force` is false, abort with exit code 2 and emit an error to `stderr`.
  - Atomic file generation: writes configuration via temporary file rename to prevent partial initialization on interruption.

### SPEC-DISC-DISCOVERY: Upward Root Discovery & Configuration Resolution
- **ID**: SPEC-DISC-DISCOVERY
- **Parent Requirements**: REQ-CFG-DISCOVERY, US-DISC-01
- **Component Target**: src/config/discovery.rs
- **Verification Target**: tests/config/test_discovery.rs
- **Technical Specification**:
  - Algorithm: Starting at `std::env::current_dir()`, inspect each directory for `.xtrc.toml`, then for `.git`. Ascend to parent until found or filesystem root reached.
  - CLI overrides `--root <PATH>` or `--config <PATH>` take absolute precedence over upward discovery.
  - Deserializes `.xtrc.toml` using `toml::from_str::<WorkspaceConfig>`.
- **Validation & Behavioral Rules**:
  - If neither `.xtrc.toml` nor `.git` is found, return `ConfigError::RootNotFound` and terminate with exit code 2.
  - Validates that paths within `.xtrc.toml` are relative to the discovered repository root.

### SPEC-DISC-TRAVERSAL: Gitignore-Aware Fast Traversal & Asset Filtering
- **ID**: SPEC-DISC-TRAVERSAL
- **Parent Requirements**: REQ-IGNORE-RULES, REQ-NFR-PERF, US-IGNORE-01
- **Component Target**: src/scanner/traversal.rs
- **Verification Target**: tests/scanner/test_traversal.rs
- **Technical Specification**:
  - Uses `ignore::WalkBuilder` to traverse directory trees honoring root `.gitignore`, nested `.gitignore` files, and `.git/info/exclude`.
  - Automatic skip-list: `.git/`, `target/`, `node_modules/`, `.agents/teamwork/`, and hidden files/directories.
  - Binary file detection: Reads the first 512 bytes; if any `0x00` null byte is present, file is classified as binary and skipped immediately without full buffer read.
- **Validation & Behavioral Rules**:
  - Traversal must operate lock-free across rayon or thread pools for multi-core scalability.
  - Unreadable or permission-denied files log a warning to `stderr` and continue traversal without aborting.

---

## 3. Artifact Format & Grammar Specifications

### SPEC-FMT-FRONTMATTER: Plain-Text Document YAML Frontmatter Parsing
- **ID**: SPEC-FMT-FRONTMATTER
- **Parent Requirements**: REQ-DOC-PARSE, US-DOC-01
- **Component Target**: src/parser/document.rs
- **Verification Target**: tests/parser/test_document.rs
- **Technical Specification**:
  - Specification files must begin with a YAML frontmatter fence (`---`).
  - Required frontmatter fields:
    ```yaml
    document_type: string
    id: string
    title: string
    version: string
    status: "draft" | "approved" | "deprecated"
    ```
  - Document body contains Markdown headings defining child nodes:
    `### <ID>: <Title>`
    followed by key-value bullet points:
    `- **ID**: <ID>`
    `- **Parent**: <PARENT-ID>` (or `- **Parent Requirements**: ...`)
    `- **Type**: <LAYER-NAME>`
- **Validation & Behavioral Rules**:
  - Frontmatter must parse into a typed `SpecDocument` struct; unclosed fences or missing required fields emit `ParseError` with line numbers and exit with code 2.
  - Leading and trailing whitespace around keys and values is trimmed.

### SPEC-FMT-GRAMMAR: Identifier Grammar & Reference Link Syntax
- **ID**: SPEC-FMT-GRAMMAR
- **Parent Requirements**: REQ-DOC-PARSE, REQ-BROKEN-LINK, US-DOC-01
- **Component Target**: src/model/identifier.rs
- **Verification Target**: tests/model/test_identifier.rs
- **Technical Specification**:
  - ID Regex Grammar: `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`
  - Standard Layer Prefixes:
    - Layer 1: `VIS-*` (Vision)
    - Layer 2: `STRAT-*` (Strategy)
    - Layer 3: `EPIC-*` (Epic)
    - Layer 4: `US-*` (User Story)
    - Layer 5: `REQ-*` (Requirement)
    - Layer 6: `SPEC-*` (Technical Specification)
  - Parent Reference Syntax: Single ID (`Parent: REQ-01`) or comma-separated list (`Parents: [REQ-01, REQ-02]`).
- **Validation & Behavioral Rules**:
  - Identifiers containing lowercase letters, underscores, whitespace, or punctuation outside `-` fail validation with `GrammarError::InvalidIdentifier`.
  - Layer prefixes must adhere to defined hierarchy order; child nodes referencing lower-layer parents trigger a Layer Inversion error.


---

## 4. Polyglot Comment Scanner Specification

### SPEC-SCAN-COMMENTS: Regex-Oriented Polyglot Source & Test Comment Scanner
- **ID**: SPEC-SCAN-COMMENTS
- **Parent Requirements**: REQ-CODE-SCAN, REQ-TEST-SCAN, REQ-NFR-PERF, US-CODE-01, US-TEST-01
- **Component Target**: src/scanner/comments.rs
- **Verification Target**: tests/scanner/test_comments.rs
- **Technical Specification**:
  - Scanning Engine: Fast literal pre-filtered, line-oriented regular expression parser scanning UTF-8 source files without requiring language-specific AST compilers.
  - Fast Literal Pre-Filter & Lazy Line Counting:
    - Scans raw byte/text buffers for the `@` byte followed by ASCII case-insensitive `trace` before invoking full regex capture matching or splitting the file into lines.
    - Files or buffers containing no `@trace` marker return immediately in a single byte scan with zero line splits and zero regex evaluations.
    - When a candidate `@trace` marker is found, isolates only the enclosing line (`\n` to `\n`) and lazily advances the 1-indexed line counter from the last matched offset (`nlscan` high-water mark pattern).
  - Extraction Regex Pattern:
    ```regex
    (?i:@trace(?::[a-zA-Z_-]+)?)\s+(?P<ids>[A-Z][A-Z0-9]*(-[A-Z0-9]+)+(?:\s*,\s*[A-Z][A-Z0-9]*(-[A-Z0-9]+)+)*)
    ```
  - Polyglot Comment Syntax Support:
    - Single-line slash comments: `// @trace REQ-01` (Rust, C/C++, Go, Java, TypeScript, JavaScript, Swift, C#)
    - Single-line hash comments: `# @trace REQ-01` (Python, Ruby, Shell, Perl, Elixir, YAML, Dockerfile)
    - Single-line dash comments: `-- @trace REQ-01` (SQL, Lua, Haskell, Ada)
    - Single-line semicolon/percent: `; @trace REQ-01` (Assembly, Clojure, Lisp), `% @trace REQ-01` (Erlang, LaTeX)
    - Multi-line block comments: `/* @trace REQ-01 */` (C-style), `<!-- @trace REQ-01 -->` (HTML/XML), `""" @trace REQ-01 """` (Python docstrings)
  - Link Classification (Source vs Test):
    - Path Heuristic: Files located in directories matching `tests/**`, `test/**`, `spec/**`, or filename patterns `*_test.*`, `test_*.*`, `*.spec.*` are marked as `LinkKind::AutomatedTest`.
    - Marker Tag Override: Explicit `@trace:test <ID>` forces classification as `LinkKind::AutomatedTest`; `@trace:code <ID>` or default `@trace <ID>` outside test paths defaults to `LinkKind::SourceCode`.
- **Validation & Behavioral Rules**:
  - Multiple comma-separated target IDs on a single line (e.g., `// @trace REQ-AUTH-01, REQ-AUTH-02`) are extracted into distinct, independent traceability links sharing the same source provenance.
  - Trailing punctuation (such as trailing semicolons, commas, or closing comment delimiters) is stripped clean from identifier tokens.
  - Line numbers are 1-indexed; file paths are normalized relative to the discovered repository root.


---

## 5. In-Memory DAG Engine & Graph Algorithms

### SPEC-DAG-STORAGE: In-Memory 7-Layer Directed Acyclic Graph Data Structures
- **ID**: SPEC-DAG-STORAGE
- **Parent Requirements**: REQ-DAG-MODEL, US-GRAPH-01
- **Component Target**: src/graph/model.rs
- **Verification Target**: tests/graph/test_model.rs
- **Technical Specification**:
  - Graph Representation: Bidirectional adjacency graph structure `TraceabilityGraph`.
    ```rust
    pub struct TraceabilityGraph {
        nodes: HashMap<NodeId, TraceNode>,
        forward_edges: HashMap<NodeId, Vec<NodeId>>,  // parent -> children (decomposition)
        backward_edges: HashMap<NodeId, Vec<NodeId>>, // child -> parents (justification)
    }

    pub struct TraceNode {
        id: NodeId,
        layer: Layer,
        title: String,
        origin: FileOrigin,
        attributes: HashMap<String, String>,
    }

    pub struct FileOrigin {
        path: PathBuf,
        line: usize,
    }
    ```
  - Layer Taxonomy: Strictly orders layers: `Vision` (1) -> `Strategy` (2) -> `Epic` (3) -> `UserStory` (4) -> `Requirement` (5) -> `TechnicalSpec` (6) -> `ImplementationOrTest` (7).
- **Validation & Behavioral Rules**:
  - Ingestion checks for identifier uniqueness; if an ID is registered more than once across specifications, graph construction aborts with `GraphError::DuplicateIdentifier` listing colliding files and line numbers (exit code 1).
  - Graph maintains bidirectional indexes enabling $O(1)$ lookups of upstream parents and downstream children.

### SPEC-DAG-CYCLE: 3-Color DFS Cycle Detection & Loop Diagnostics
- **ID**: SPEC-DAG-CYCLE
- **Parent Requirements**: REQ-CYCLE-DETECT, US-CYCLE-01
- **Component Target**: src/graph/cycle.rs
- **Verification Target**: tests/graph/test_cycle.rs
- **Technical Specification**:
  - Algorithm: Three-color Depth First Search (DFS) over forward decomposition edges.
    - `White`: Node unvisited.
    - `Gray`: Node actively on the current recursion call stack (currently visiting).
    - `Black`: Node and all descendants completely visited.
  - Cycle Trigger: Encountering an edge $(u, v)$ where color of $v$ is `Gray` indicates a directed cycle.
  - Diagnostic Loop Extraction: When a Gray node $v$ is encountered, the algorithm records the active recursion stack slice from $v$ to $u$, formatting the cycle path: `v -> ... -> u -> v`.
- **Validation & Behavioral Rules**:
  - Detects all independent cycles in $O(V + E)$ time without infinite recursion.
  - Prints the complete cycle loop chain with file origins to `stderr` and terminates execution with exit code 1.

### SPEC-DAG-VALIDATE: Bidirectional Integrity, Provenance & Completeness Validation
- **ID**: SPEC-DAG-VALIDATE
- **Parent Requirements**: REQ-ORPHAN-DETECT, REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST, REQ-STRICT-GATE, US-VERIFY-01, US-ORPHAN-01, US-GATE-01
- **Component Target**: src/graph/validator.rs
- **Verification Target**: tests/graph/test_validator.rs
- **Technical Specification**:
  - Four-part verification suite executed sequentially:
    1. **Broken Link Audit**: Verifies every target ID declared in parent lists or `@trace` comments exists in the node registry.
    2. **Orphan Node Audit**: Verifies every node except Layer 1 (`VIS-*`) has $\ge 1$ upward parent justification link.
    3. **Backward Provenance Audit**: Traverses backward edges from all Layer 7 code and test leaves to verify every leaf possesses a valid directed path terminating at a `VIS-*` node.
    4. **Forward Completeness Audit**: For every `REQ-*` node, validates downstream fulfillment:
       - Has $\ge 1$ child `SPEC-*` node.
       - Has $\ge 1$ associated `LinkKind::SourceCode` leaf.
       - Has $\ge 1$ associated `LinkKind::AutomatedTest` leaf.
  - Coverage Metrics:
    $$\text{Coverage Rate} = \frac{|REQ_{\text{fulfilled}}|}{|REQ_{\text{total}}|} \times 100\%$$
- **Validation & Behavioral Rules**:
  - Broken links always trigger fatal errors and terminate with exit code 1.
  - In default mode: orphans and partial coverage emit warnings to `stderr`, format summary to `stdout`, and exit code 0.
  - In `--strict` mode: any orphan node, broken reference, or requirement lacking code/test coverage triggers a fatal gate violation and terminates with exit code 1.


---

## 6. CLI Command Contracts & Output Format Specifications

### SPEC-CLI-VERIFY: xtrc check / verify Command Contract & Strict CI Mode
- **ID**: SPEC-CLI-VERIFY
- **Parent Requirements**: REQ-STRICT-GATE, REQ-UNIX-STREAMS, REQ-EXIT-CODES, REQ-NFR-PERF, US-VERIFY-01, US-GATE-01
- **Component Target**: src/cli/verify.rs
- **Verification Target**: tests/cli/test_verify.rs
- **Technical Specification**:
  - Command Signatures: `xtrc check [OPTIONS]` (alias `xtrc verify`).
  - Options:
    - `-s, --strict`: Elevate all warnings and incomplete coverage to fatal failures.
    - `--json`: Emit machine-parseable verification summary to `stdout`.
    - `--layer <LAYER>`: Scope verification to a specific layer boundary.
    - `--id <ID>`: Scope verification to the connected subgraph of `<ID>`.
  - Terminal Display:
    - Interactive TTY: Scan spinner and timing benchmarks on `stderr`; aligned summary table on `stdout`.
    - Non-interactive / Pipe: Disables ANSI colors; emits clean tabular data or JSON.
- **Validation & Behavioral Rules**:
  - Terminates with exit code 0 on complete, clean validation.
  - Terminates with exit code 1 on broken links, cycle loops, orphan nodes in strict mode, or requirements lacking code/test coverage in strict mode.
  - Terminates with exit code 2 on invalid CLI syntax or configuration errors.

### SPEC-CLI-MATRIX: Multidimensional Traceability Matrix Contract & Formats
- **ID**: SPEC-CLI-MATRIX
- **Parent Requirements**: REQ-CMD-MATRIX, REQ-UNIX-STREAMS, US-MATRIX-01
- **Component Target**: src/cli/matrix.rs
- **Verification Target**: tests/cli/test_matrix.rs
- **Technical Specification**:
  - Command Signature: `xtrc matrix [OPTIONS]`.
  - Options:
    - `-f, --format <table|markdown|csv|json>`: Output format (defaults to `table` on TTY, `markdown` when piped).
    - `-u, --uncovered`: Filter matrix rows to only display requirements missing code implementation or test verification.
    - `--layer <LAYER>`: Filter rows by layer.
  - Column Schema:
    `| Requirement ID | Title | Parent US | Specs | Code Links | Test Links | Status |`
- **Validation & Behavioral Rules**:
  - All rendered rows stream cleanly to `stdout`; diagnostic logs remain on `stderr`.
  - Exits 0 on success; exits 2 if an invalid format is requested.

### SPEC-CLI-TREE: Hierarchical Dependency Tree Visualizer Contract
- **ID**: SPEC-CLI-TREE
- **Parent Requirements**: REQ-CMD-TREE, REQ-UNIX-STREAMS, US-TREE-01
- **Component Target**: src/cli/tree.rs
- **Verification Target**: tests/cli/test_tree.rs
- **Technical Specification**:
  - Command Signature: `xtrc tree [ID] [OPTIONS]`.
  - Arguments: Optional `[ID]` root anchor. If omitted, visualizes tree from top-level `VIS-*` nodes.
  - Options:
    - `--reverse`: Upward traversal (justification path toward Vision).
    - `--down`: Downward traversal (decomposition path toward Code/Tests, default).
    - `-d, --depth <N>`: Maximum recursion depth.
    - `--json`: Output nested tree hierarchy as JSON.
  - Tree Rendering: Emits standard Unicode box-drawing characters (`├──`, `└──`, `│   `).
- **Validation & Behavioral Rules**:
  - If the requested `[ID]` does not exist in the graph, emits an error to `stderr` and terminates with exit code 1.
  - Exits 2 if `--depth` is non-numeric or negative.

### SPEC-CLI-EXPORT: Compliance & Audit Documentation Exporter Contract
- **ID**: SPEC-CLI-EXPORT
- **Parent Requirements**: REQ-CMD-EXPORT, REQ-UNIX-STREAMS, US-EXPORT-01
- **Component Target**: src/cli/export.rs
- **Verification Target**: tests/cli/test_export.rs
- **Technical Specification**:
  - Command Signature: `xtrc export [OPTIONS]`.
  - Options:
    - `-f, --format <markdown|json>`: Output document format (defaults to `markdown`).
    - `-o, --output <PATH>`: Target output file path (defaults to `stdout`).
  - Content Structure: Emits a comprehensive, self-contained audit trail detailing Vision -> Strategy -> Epics -> User Stories -> Requirements -> Technical Specs -> Source Code -> Automated Tests.
- **Validation & Behavioral Rules**:
  - When `-o` is provided, writes atomically using a temporary file rename.
  - Exits with code 2 if target directory does not exist or has unwritable permissions.

### SPEC-CLI-OUTPUT: Standard Unix Stream Separation & Universal JSON Schemas
- **ID**: SPEC-CLI-OUTPUT
- **Parent Requirements**: REQ-FMT-JSON, REQ-UNIX-STREAMS, REQ-EXIT-CODES, US-JSON-01, US-UNIX-01, US-EXIT-01
- **Component Target**: src/cli/output.rs
- **Verification Target**: tests/cli/test_output.rs
- **Technical Specification**:
  - Pure Stream Protocol:
    - `stdout`: Pure structured data only (tables, markdown, CSV, tree text, or JSON payload). Never contaminated by logs or debug statements.
    - `stderr`: Progress indicators, spinners, scan telemetry, warnings, and error diagnostics.
  - Universal JSON Schema (`--json`):
    ```json
    {
      "schema_version": "1.0.0",
      "command": "check",
      "timestamp": "2026-10-24T17:30:00Z",
      "status": "pass",
      "statistics": {
        "total_nodes": 49,
        "specs_count": 16,
        "code_links": 28,
        "test_links": 31,
        "broken_links": 0,
        "cycles_count": 0,
        "orphan_nodes": 0,
        "coverage_percent": 100.0
      },
      "violations": []
    }
    ```
  - Exit Code Mapping:
    - `0`: Success (validation passed clean, query completed).
    - `1`: Traceability verification failure (broken links, cycle loops, strict mode violations).
    - `2`: CLI usage error, invalid options, file I/O or syntax error.
    - `>2`: Internal unexpected error.
- **Validation & Behavioral Rules**:
  - Non-TTY pipes automatically suppress ANSI escape codes unless explicitly overridden by `--color=always`.
  - Piped commands (e.g., `xtrc check --json | jq .`) parse cleanly with zero trailing garbage.


---

## 7. Rust Error Handling & Performance Strategy

### SPEC-ERR-HIERARCHY: Structured thiserror Domain Enums & anyhow Context Boundary
- **ID**: SPEC-ERR-HIERARCHY
- **Parent Requirements**: REQ-NFR-ROBUST, REQ-EXIT-CODES, US-UNIX-01, US-EXIT-01
- **Component Target**: src/error.rs
- **Verification Target**: tests/error/test_error.rs
- **Technical Specification**:
  - Library Layer Domain Errors: Modeled with `thiserror` enums:
    ```rust
    #[derive(thiserror::Error, Debug)]
    pub enum ConfigError {
        #[error("Could not find repository root or .git directory from '{0}'")]
        RootNotFound(PathBuf),
        #[error("Invalid TOML syntax in '{path}': {source}")]
        InvalidToml { path: PathBuf, source: toml::de::Error },
        #[error("I/O error reading configuration '{path}': {source}")]
        Io { path: PathBuf, source: std::io::Error },
    }

    #[derive(thiserror::Error, Debug)]
    pub enum ParseError {
        #[error("Malformed YAML frontmatter in '{path}': {reason}")]
        InvalidFrontmatter { path: PathBuf, reason: String },
        #[error("Invalid identifier token '{token}' at {path}:{line}: {reason}")]
        GrammarViolation { token: String, path: PathBuf, line: usize, reason: String },
    }

    #[derive(thiserror::Error, Debug)]
    pub enum GraphError {
        #[error("Duplicate identifier '{id}' found in {path_a}:{line_a} and {path_b}:{line_b}")]
        DuplicateIdentifier { id: String, path_a: PathBuf, line_a: usize, path_b: PathBuf, line_b: usize },
        #[error("Broken link in {origin}: target '{target_id}' does not exist")]
        BrokenLink { origin: String, target_id: String },
        #[error("Cyclic dependency detected: {cycle_chain}")]
        CycleDetected { cycle_chain: String },
    }
    ```
  - CLI Layer Error Boundary: Main entry point uses `anyhow::Result<()>` with `.with_context(...)` to append user-actionable guidance.
  - Zero Naked Unwrap Mandate: Strictly zero `.unwrap()` or `.expect()` calls in production codepaths, enforced via Clippy `-D clippy::unwrap_used` and `-D clippy::expect_used`.
- **Validation & Behavioral Rules**:
  - Error messages printed to `stderr` must contain file path and line number context whenever available.
  - No unexpected panics or aborts under malformed or hostile input.

### SPEC-PERF-BOUNDS: High-Performance Multi-Core Execution & Memory Budget
- **ID**: SPEC-PERF-BOUNDS
- **Parent Requirements**: REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, US-PERF-01, US-UNIX-01
- **Component Target**: src/lib.rs, src/cli/pipeline.rs, src/scanner/comments.rs, src/scanner/traversal.rs, src/parser/document.rs, src/graph/validator.rs, src/graph/cycle.rs
- **Verification Target**: tests/perf/test_benchmarks.rs, tests/scanner/test_comments.rs
- **Technical Specification**:
  - Parallel Traversal & Single-Pass Partitioning: Leverages `ignore::WalkParallel` and `rayon` work-stealing thread pools to pipeline directory discovery, single-pass `O(N)` file categorization (including `other` text files without $O(N^2)$ linear slice searches), parallel specification parsing, and parallel comment scanning.
  - Literal Pre-Filtered Scanning & Single-Pass File Inspection: Performs binary NUL-byte sniffing on the initial read buffer and applies fast `@trace` literal pre-filtering with lazy line-number resolution (`nlscan`) so files without annotations incur zero regex engine or line-splitting overhead.
  - Zero-Copy Frontmatter Slicing & Linear-Time Graph Validation: Slices YAML frontmatter directly without whole-file `Vec<&str>` line tables, computes upward `Vision` reachability in a single $O(V + E)$ pass from root nodes, traverses pre-sorted cycle adjacency slices without per-visit cloning, and batches CLI output over locked `BufWriter` streams.
  - Memory Footprint Bounding: Uses bounded streaming buffers for binary file sniffing (<=512 bytes read per binary asset); overall memory usage is bounded by repository metadata size (<50 MiB for 1,000 files).
  - Canonical Deterministic Ordering: Maintains `BTreeMap` / `BTreeSet` and sorted adjacency vectors across parallel worker reductions so all CLI commands produce byte-for-byte identical outputs across runs.
  - Sub-Second Target: Full repository traversal, graph parsing, cycle detection, and verification completes in <500ms for repositories with 1,000 files and <200ms on the `xtrc` repository itself.
- **Validation & Behavioral Rules**:
  - Zero network round-trips or disk-bound temporary databases.
  - Benchmark assertions verify performance, memory, scalability, and determinism budgets during automated testing and CI/CD gates.

---

## 8. Traceability Matrix & Forward Declarations Summary

### 8.1 Forward Requirements Coverage Matrix (100% Coverage of Layer 5 REQ-*)

| Requirement ID | Requirement Title | Fulfilling Technical Spec(s) | Component Target (`src/`) | Verification Target (`tests/`) |
|---|---|---|---|---|
| `REQ-CLI-INIT` | Repository Initialization Command | `SPEC-DISC-INIT` | `src/cli/init.rs` | `tests/cli/test_init.rs` |
| `REQ-CFG-DISCOVERY` | Root and Configuration Discovery | `SPEC-DISC-DISCOVERY` | `src/config/discovery.rs` | `tests/config/test_discovery.rs` |
| `REQ-IGNORE-RULES` | Filesystem Traversal and Ignore Filtering | `SPEC-DISC-TRAVERSAL` | `src/scanner/traversal.rs` | `tests/scanner/test_traversal.rs` |
| `REQ-DOC-PARSE` | Plain-Text Specification Document Parsing | `SPEC-FMT-FRONTMATTER`, `SPEC-FMT-GRAMMAR` | `src/parser/document.rs`, `src/model/identifier.rs` | `tests/parser/test_document.rs`, `tests/model/test_identifier.rs` |
| `REQ-CODE-SCAN` | Language-Agnostic Source Code Comment Scanning | `SPEC-SCAN-COMMENTS` | `src/scanner/comments.rs` | `tests/scanner/test_comments.rs` |
| `REQ-TEST-SCAN` | Automated Test Annotation Scanning | `SPEC-SCAN-COMMENTS` | `src/scanner/comments.rs` | `tests/scanner/test_comments.rs` |
| `REQ-DAG-MODEL` | In-Memory 7-Layer Directed Acyclic Graph | `SPEC-DAG-STORAGE` | `src/graph/model.rs` | `tests/graph/test_model.rs` |
| `REQ-CYCLE-DETECT` | Graph Cycle Detection and Diagnostics | `SPEC-DAG-CYCLE` | `src/graph/cycle.rs` | `tests/graph/test_cycle.rs` |
| `REQ-ORPHAN-DETECT` | Orphan Node and Upward Justification Validation | `SPEC-DAG-VALIDATE` | `src/graph/validator.rs` | `tests/graph/test_validator.rs` |
| `REQ-BROKEN-LINK` | Broken Reference and Dangling Link Detection | `SPEC-FMT-GRAMMAR`, `SPEC-DAG-VALIDATE` | `src/model/identifier.rs`, `src/graph/validator.rs` | `tests/model/test_identifier.rs`, `tests/graph/test_validator.rs` |
| `REQ-FORWARD-COV` | Forward Completeness and Coverage Tracking | `SPEC-DAG-VALIDATE` | `src/graph/validator.rs` | `tests/graph/test_validator.rs` |
| `REQ-BACKWARD-JUST` | Backward Traceability and Provenance Verification | `SPEC-DAG-VALIDATE` | `src/graph/validator.rs` | `tests/graph/test_validator.rs` |
| `REQ-STRICT-GATE` | Strict CI Verification Gate | `SPEC-DAG-VALIDATE`, `SPEC-CLI-VERIFY` | `src/graph/validator.rs`, `src/cli/verify.rs` | `tests/graph/test_validator.rs`, `tests/cli/test_verify.rs` |
| `REQ-CMD-MATRIX` | Traceability Matrix Reporting | `SPEC-CLI-MATRIX` | `src/cli/matrix.rs` | `tests/cli/test_matrix.rs` |
| `REQ-CMD-TREE` | Hierarchical Dependency Tree Visualization | `SPEC-CLI-TREE` | `src/cli/tree.rs` | `tests/cli/test_tree.rs` |
| `REQ-CMD-EXPORT` | Compliance and Traceability Export | `SPEC-CLI-EXPORT` | `src/cli/export.rs` | `tests/cli/test_export.rs` |
| `REQ-FMT-JSON` | Universal Machine-Readable JSON Output | `SPEC-CLI-OUTPUT` | `src/cli/output.rs` | `tests/cli/test_output.rs` |
| `REQ-UNIX-STREAMS` | Standard Unix Streams Separation | `SPEC-CLI-VERIFY`, `SPEC-CLI-MATRIX`, `SPEC-CLI-TREE`, `SPEC-CLI-EXPORT`, `SPEC-CLI-OUTPUT` | `src/cli/*.rs` | `tests/cli/test_output.rs` |
| `REQ-EXIT-CODES` | Deterministic Process Exit Codes | `SPEC-CLI-VERIFY`, `SPEC-CLI-OUTPUT`, `SPEC-ERR-HIERARCHY` | `src/cli/output.rs`, `src/error.rs` | `tests/cli/test_output.rs`, `tests/error/test_error.rs` |
| `REQ-NFR-PERF` | Sub-Second Execution Performance | `SPEC-DISC-TRAVERSAL`, `SPEC-CLI-VERIFY`, `SPEC-PERF-BOUNDS`, `SPEC-PERF-HARNESS` | `src/scanner/traversal.rs`, `src/cli/pipeline.rs`, `src/lib.rs` | `tests/perf/test_benchmarks.rs` |
| `REQ-NFR-MEM` | Bounded Memory Footprint and Streaming Binary Sniffing | `SPEC-PERF-BOUNDS`, `SPEC-PERF-HARNESS` | `src/scanner/traversal.rs`, `src/scanner/comments.rs` | `tests/perf/test_benchmarks.rs` |
| `REQ-NFR-SCALE` | Multi-Core Linear Scalability and O(V + E) Graph Complexity | `SPEC-PERF-BOUNDS`, `SPEC-PERF-HARNESS` | `src/cli/pipeline.rs`, `src/graph/validator.rs`, `src/graph/cycle.rs` | `tests/perf/test_benchmarks.rs` |
| `REQ-NFR-DETERM` | Deterministic and Reproducible Multi-Core Output | `SPEC-PERF-BOUNDS`, `SPEC-PERF-HARNESS` | `src/cli/pipeline.rs`, `src/graph/model.rs`, `src/graph/validator.rs` | `tests/perf/test_benchmarks.rs` |
| `REQ-NFR-ROBUST` | Robust Error Handling and Crash-Free Operation | `SPEC-ERR-HIERARCHY` | `src/error.rs` | `tests/error/test_error.rs` |

### 8.2 Backward Justification & Upward Linkage Summary
- Total `SPEC-*` Nodes: 16 (plus derived NFR & CI/CD specifications in `docs/nfr.md` and `docs/delivery.md`).
- Broken Parent Links: 0 (All upstream `REQ-*` and `US-*` references resolve directly to `docs/requirements.md` and `docs/nfr.md`).
- Forward Target Declarations: 100% of `SPEC-*` nodes specify explicit `Component Target` (`src/`) and `Verification Target` (`tests/`), providing a complete design blueprint for subsequent implementation.
