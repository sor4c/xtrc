---
document_type: requirements_specification
id: REQ-SPEC-001
title: xtrc Foundational Requirements Specification
version: 1.0.0
status: approved
governance_tracking:
  rule_1_issue: "#3"
  rule_2: "Requirements Before Design"
  rule_3: "Intent Over Implementation"
traceability_layers:
  - Vision
  - Strategy
  - Epics
  - User Stories
  - Requirements
schema_version: "1.0.0"
---

# xtrc Foundational Requirements Specification

## 1. Traceability Architecture & Governance
This document defines the foundational requirements specification for `xtrc`, establishing Layers 1 through 5 of the 7-layer Directed Acyclic Graph (DAG) traceability hierarchy. Every item is grounded in GitHub Issue #3 (Rule 1: Issue-Driven Development), precedes technical design (Rule 2: Requirements Before Design), and specifies observable behavioral contracts without prescribing internal implementation mechanics (Rule 3: Intent Over Implementation).

## 2. Layer 1: Vision (VIS-*)

### VIS-VCS-NATIVE: Fast, Plain-Text Traceability Native to Version Control
- **ID**: VIS-VCS-NATIVE
- **Parent**: None
- **Type**: Vision
- **Intent & Value**: Eliminate heavy, proprietary external database silos (e.g., Jira, DOORS, Jama) by embedding complete software lifecycle traceability directly into version-controlled plain text so teams can verify alignment instantly within developer workflows.
- **Observable Contract & Behavior**: All traceability artifacts reside within repository version control as human-readable, plain-text files and source code comments, parseable and verifiable via a local command-line interface without network dependencies.
- **Acceptance Criteria**: Traceability data branches, merges, and diffs alongside code using standard Git operations; terminal verification executes in sub-second time.

## 3. Layer 2: Strategy (STRAT-*)

### STRAT-PLAIN-TEXT: Plain-Text & Repository-Resident Artifacts
- **ID**: STRAT-PLAIN-TEXT
- **Parent**: VIS-VCS-NATIVE
- **Type**: Strategy
- **Intent & Value**: Provide human-readable, diff-friendly specification formats that prevent vendor lock-in and enable seamless code review of requirement changes.
- **Observable Contract & Behavior**: Stores specification nodes in Markdown documents with structured YAML frontmatter, queryable with standard Unix text tools and versioned in Git.
- **Acceptance Criteria**: Specifications are parseable from repository markdown files; zero external databases or binary storage formats are required.

### STRAT-POLYGLOT-COMMENT: Language-Agnostic Source & Automated Test Traceability
- **ID**: STRAT-POLYGLOT-COMMENT
- **Parent**: VIS-VCS-NATIVE
- **Type**: Strategy
- **Intent & Value**: Allow developers to link source code and automated tests to specifications in any programming language without specialized compiler plugins or heavy IDE extensions.
- **Observable Contract & Behavior**: Scans polyglot source and test files using line-oriented comment annotations (`@trace <ID>`) across standard single-line, multi-line, and doc-comment syntax.
- **Acceptance Criteria**: Comment annotations in major languages (e.g., Rust, Python, Go, TypeScript, C, Shell) are discovered, parsed, and mapped into the traceability graph with file and line provenance.

### STRAT-STRICT-INTEGRITY: Deterministic Graph Verification & Bidirectional Auditing
- **ID**: STRAT-STRICT-INTEGRITY
- **Parent**: VIS-VCS-NATIVE
- **Type**: Strategy
- **Intent & Value**: Ensure strict mathematical rigor across the development lifecycle, preventing broken references, circular logic, and ungrounded gold-plating.
- **Observable Contract & Behavior**: Validates bidirectional relationships across all layers using Directed Acyclic Graph (DAG) integrity checks: forward completeness, backward justification, cycle detection, and orphan detection.
- **Acceptance Criteria**: Verification deterministically identifies missing references, loops, and orphan nodes; exits with reproducible non-zero status codes upon violation.

### STRAT-CLI-FIRST: Sub-Second Terminal Verification & CI/CD Gates
- **ID**: STRAT-CLI-FIRST
- **Parent**: VIS-VCS-NATIVE
- **Type**: Strategy
- **Intent & Value**: Empower developers with instant local verification and provide automated CI/CD pipelines with deterministic quality gates.
- **Observable Contract & Behavior**: Provides command-line interface conforming strictly to Unix stream conventions, sub-second execution latencies, and machine-readable structured output formats.
- **Acceptance Criteria**: Commands execute locally in under 500ms on standard repositories; output cleanly separates data on stdout from diagnostics on stderr; automated gates block failing pull requests.


## 4. Layer 3: Epics (EPIC-*)

### EPIC-WORKSPACE: Workspace Initialization & Configuration Discovery
- **ID**: EPIC-WORKSPACE
- **Parent**: STRAT-PLAIN-TEXT
- **Type**: Epic
- **Intent & Value**: Provide seamless bootstrapping and automatic environment discovery so developers can operate xtrc effortlessly from any directory in a repository.
- **Observable Contract & Behavior**: Handles repository initialization (`xtrc init`), upward search for project root (`.xtrc.toml` or `.git`), and traversal filtering honoring `.gitignore`.
- **Acceptance Criteria**: Initializes new projects with valid default configuration; climbs directory hierarchy to find project root; excludes ignored and binary files from scanning.

### EPIC-PARSER: Plain-Text Document & Polyglot Comment Parsing
- **ID**: EPIC-PARSER
- **Parent**: STRAT-POLYGLOT-COMMENT
- **Type**: Epic
- **Intent & Value**: Ingest specifications, source code annotations, and test verifications into structured traceable nodes across multiple programming languages.
- **Observable Contract & Behavior**: Parses Markdown files with YAML frontmatter to extract upper-layer nodes, and scans source/test files for `@trace <ID>` comment markers.
- **Acceptance Criteria**: Extracts node metadata and parent links from Markdown; extracts line-level source and test references; maps language-agnostic comments to target nodes.

### EPIC-DAG: In-Memory 7-Layer Directed Acyclic Graph Engine
- **ID**: EPIC-DAG
- **Parent**: STRAT-STRICT-INTEGRITY
- **Type**: Epic
- **Intent & Value**: Construct and maintain a formal in-memory directed graph representing all lifecycle relationships with full validation capabilities.
- **Observable Contract & Behavior**: Builds bidirectional adjacency structures, enforces 7-layer hierarchy constraints, detects circular reference paths, and discovers ungrounded orphan nodes.
- **Acceptance Criteria**: Correctly indexes nodes and edges; detects cycles and reports full loops; detects orphans lacking upward links.

### EPIC-VERIFY: Bidirectional Traceability Verification & CI Gate
- **ID**: EPIC-VERIFY
- **Parent**: STRAT-CLI-FIRST
- **Type**: Epic
- **Intent & Value**: Provide comprehensive pre-commit verification and automated CI quality gates to guarantee project alignment and coverage.
- **Observable Contract & Behavior**: Evaluates forward completeness, backward justification, and referential integrity via `xtrc check` / `xtrc verify`, enforcing strict zero-tolerance gates when requested.
- **Acceptance Criteria**: Identifies broken links and unfulfilled requirements; exits 0 on valid graphs and 1 on integrity failures; strictly enforces 100% coverage with `--strict`.

### EPIC-QUERY: Traceability Matrix, Dependency Tree & Compliance Export
- **ID**: EPIC-QUERY
- **Parent**: STRAT-CLI-FIRST
- **Type**: Epic
- **Intent & Value**: Give engineers, managers, and external auditors immediate visibility into requirement status, dependency trees, and compliance records.
- **Observable Contract & Behavior**: Renders terminal matrices (`xtrc matrix`), hierarchical trees (`xtrc tree <ID>`), compliance exports (`xtrc export`), and machine-readable JSON across query commands.
- **Acceptance Criteria**: Renders formatted tables and trees; outputs valid JSON conforming to standard schemas; exports complete audit documents.

### EPIC-CLI-CONTRACT: Unix Terminal Philosophy, Exit Codes & Robust Error Handling
- **ID**: EPIC-CLI-CONTRACT
- **Parent**: STRAT-CLI-FIRST
- **Type**: Epic
- **Intent & Value**: Deliver a reliable, script-friendly Unix CLI tool that behaves predictably in pipes, CI runners, and interactive terminals without unexpected crashes.
- **Observable Contract & Behavior**: Strictly separates stdout and stderr streams, emits deterministic exit codes, suppresses formatting in non-TTY environments, and provides actionable diagnostics.
- **Acceptance Criteria**: Exits 0 on success, 1 on validation failure, 2 on usage/syntax/IO errors; zero unhandled panics or stack traces on invalid inputs.


## 5. Layer 4: User Stories (US-*)

### US-INIT-01: Workspace Bootstrap
- **ID**: US-INIT-01
- **Parent**: EPIC-WORKSPACE
- **Type**: User Story
- **Intent & Value**: As a developer, I want to initialize xtrc in my repository with a single command so that I have a working configuration and starter directory structure immediately.
- **Observable Contract & Behavior**: Running `xtrc init` creates `.xtrc.toml` with default configuration and establishes initial specification directories without overwriting existing files unless `--force` is provided.
- **Acceptance Criteria**: Command generates valid `.xtrc.toml`; rejects initialization if already present without `--force` (exit code 2); succeeds with exit code 0 when creating files.

### US-DISC-01: Upward Project Discovery
- **ID**: US-DISC-01
- **Parent**: EPIC-WORKSPACE
- **Type**: User Story
- **Intent & Value**: As a developer working in a deeply nested subdirectory, I want xtrc to automatically discover the repository root so that I do not need to pass root paths manually.
- **Observable Contract & Behavior**: When executed from any child directory, xtrc traverses parent directories upward until locating `.xtrc.toml` or `.git`, establishing that directory as the project root.
- **Acceptance Criteria**: Commands succeed identically whether invoked from repository root or deep subdirectories; emits descriptive error to stderr if root cannot be located.

### US-IGNORE-01: Gitignore-Aware Scanning
- **ID**: US-IGNORE-01
- **Parent**: EPIC-WORKSPACE
- **Type**: User Story
- **Intent & Value**: As a developer, I want xtrc to respect `.gitignore` rules and skip build artifacts and binary files so that scanning remains fast and free of false positives.
- **Observable Contract & Behavior**: Repository traversal respects `.gitignore`, `.git/`, and configured exclude globs, bypassing binary files and build directories.
- **Acceptance Criteria**: Ignored files and binary assets are never scanned for `@trace` markers; ignored files do not produce orphan or broken reference errors.

### US-DOC-01: Markdown Requirements Authoring
- **ID**: US-DOC-01
- **Parent**: EPIC-PARSER
- **Type**: User Story
- **Intent & Value**: As a systems engineer, I want to author requirements in standard Markdown with YAML frontmatter so that specifications are easily read, edited, and reviewed in Git.
- **Observable Contract & Behavior**: Ingests Markdown files matching configured specification paths, extracting frontmatter metadata, structured headings, and parent relationships.
- **Acceptance Criteria**: Parser correctly extracts node IDs, parents, titles, and body content; reports actionable errors with line numbers for malformed frontmatter.

### US-CODE-01: Source Code Annotation
- **ID**: US-CODE-01
- **Parent**: EPIC-PARSER
- **Type**: User Story
- **Intent & Value**: As a software engineer, I want to annotate source code implementations with `@trace <ID>` comments so that implementation artifacts link directly back to requirements.
- **Observable Contract & Behavior**: Scans source files across multiple programming languages for `@trace <ID>` markers, recording file paths and line numbers as implementation evidence.
- **Acceptance Criteria**: Comment markers in line, block, and doc comment formats are extracted and associated with the target specification nodes.

### US-TEST-01: Automated Test Verification Linking
- **ID**: US-TEST-01
- **Parent**: EPIC-PARSER
- **Type**: User Story
- **Intent & Value**: As a test engineer, I want to annotate automated tests with `@trace <ID>` comments so that test executions prove requirement verification.
- **Observable Contract & Behavior**: Scans test files (identified by path patterns or marker tags) for `@trace <ID>`, registering them as verification nodes that satisfy the test fulfillment requirement.
- **Acceptance Criteria**: Test references are cataloged separately from production code and count toward verification coverage of target requirements.

### US-GRAPH-01: 7-Layer DAG Construction
- **ID**: US-GRAPH-01
- **Parent**: EPIC-DAG
- **Type**: User Story
- **Intent & Value**: As an engineer, I want xtrc to build an in-memory graph connecting Vision down to Code and Tests so that the full product hierarchy can be analyzed.
- **Observable Contract & Behavior**: Combines specification nodes and comment links into a unified directed graph supporting bidirectional traversal across all 7 layers.
- **Acceptance Criteria**: Graph indexes forward fulfillment and backward justification edges; ensures fast traversal across all nodes.

### US-CYCLE-01: Cycle Detection
- **ID**: US-CYCLE-01
- **Parent**: EPIC-DAG
- **Type**: User Story
- **Intent & Value**: As a tech lead, I want xtrc to detect circular dependencies between items so that specification hierarchies remain strictly acyclic.
- **Observable Contract & Behavior**: Evaluates graph edges for circular references; reports the exact loop path and file locations when a cycle is detected.
- **Acceptance Criteria**: Identifies direct and indirect cycles; emits human-readable cycle path to stderr; terminates execution with exit code 1.

### US-ORPHAN-01: Orphan Node Detection
- **ID**: US-ORPHAN-01
- **Parent**: EPIC-DAG
- **Type**: User Story
- **Intent & Value**: As a project manager, I want xtrc to identify specification nodes or code markers lacking upward justification so that ungrounded features are flagged.
- **Observable Contract & Behavior**: Inspects all non-root nodes for valid parent links; identifies orphan nodes lacking upward justification to Vision.
- **Acceptance Criteria**: Lists all orphan nodes with file locations; reports warning in default mode and triggers failure in strict mode.

### US-VERIFY-01: Local Pre-Commit Alignment Verification
- **ID**: US-VERIFY-01
- **Parent**: EPIC-VERIFY
- **Type**: User Story
- **Intent & Value**: As a developer, I want to run `xtrc check` locally to verify link validity and fulfillment coverage before pushing code.
- **Observable Contract & Behavior**: Executes verification checks covering broken links, orphan nodes, and coverage metrics; displays clear status summary on terminal.
- **Acceptance Criteria**: Flags broken links, reports total coverage percentage, and exits 0 on valid graphs or 1 on integrity errors.

### US-GATE-01: Strict CI Gate Blocking
- **ID**: US-GATE-01
- **Parent**: EPIC-VERIFY
- **Type**: User Story
- **Intent & Value**: As a DevOps engineer, I want to run `xtrc check --strict` in CI pipelines to reject pull requests that introduce gaps or warnings.
- **Observable Contract & Behavior**: In `--strict` mode, elevates all warnings, orphan nodes, and incomplete coverage to fatal failures, terminating with exit code 1.
- **Acceptance Criteria**: Exits 1 if coverage is less than 100% or if any orphan nodes or broken links exist; exits 0 only when graph is 100% complete and valid.

### US-MATRIX-01: Full Traceability Matrix
- **ID**: US-MATRIX-01
- **Parent**: EPIC-QUERY
- **Type**: User Story
- **Intent & Value**: As a compliance lead, I want to view a comprehensive traceability matrix showing requirements mapped to specs, code, and tests.
- **Observable Contract & Behavior**: `xtrc matrix` generates a tabular matrix of requirement rows against implementation and verification columns, with optional filtering for uncovered items.
- **Acceptance Criteria**: Displays clear terminal table; supports `--uncovered` to show only items missing implementation or tests; exits 0 on success.

### US-TREE-01: Dependency Tree Inspection
- **ID**: US-TREE-01
- **Parent**: EPIC-QUERY
- **Type**: User Story
- **Intent & Value**: As an architect, I want to visualize the hierarchical tree of a specific node so that I can inspect its decomposition or upward provenance.
- **Observable Contract & Behavior**: `xtrc tree <ID>` renders an ASCII/Unicode hierarchy tree; supports `--reverse` to inspect upward justification back to Vision.
- **Acceptance Criteria**: Renders formatted tree to stdout; returns error (exit code 1) if the specified ID does not exist in the graph.

### US-EXPORT-01: Compliance Exporting
- **ID**: US-EXPORT-01
- **Parent**: EPIC-QUERY
- **Type**: User Story
- **Intent & Value**: As an auditor, I want to export complete traceability records in Markdown or JSON so that compliance reports can be submitted for certification.
- **Observable Contract & Behavior**: `xtrc export --format <markdown|json>` writes a complete traceability report to standard output or a specified `--output` file path.
- **Acceptance Criteria**: Generates self-contained Markdown or JSON documentation; fails with exit code 2 if target directory is unwritable.

### US-JSON-01: Machine-Readable Output
- **ID**: US-JSON-01
- **Parent**: EPIC-QUERY
- **Type**: User Story
- **Intent & Value**: As a developer building tooling, I want a universal `--json` flag across all commands so that xtrc can be integrated into automated scripts and dashboards.
- **Observable Contract & Behavior**: Passing `--json` emits machine-parseable JSON to stdout conforming to a versioned schema across all check and query commands.
- **Acceptance Criteria**: Emits valid, schema-compliant JSON on stdout; logs and diagnostics remain restricted to stderr.

### US-UNIX-01: Unix Stream Philosophy
- **ID**: US-UNIX-01
- **Parent**: EPIC-CLI-CONTRACT
- **Type**: User Story
- **Intent & Value**: As a developer using shell pipelines, I want pure data on stdout and logs on stderr so that I can pipe xtrc output cleanly to jq, grep, or files.
- **Observable Contract & Behavior**: Directs structured output (tables, trees, JSON, exports) exclusively to stdout; directs logs, progress indicators, warnings, and errors to stderr.
- **Acceptance Criteria**: Piping stdout to another command receives clean data with zero diagnostic text; terminal escape sequences are omitted when stdout is not a TTY.

### US-EXIT-01: Deterministic Exit Codes
- **ID**: US-EXIT-01
- **Parent**: EPIC-CLI-CONTRACT
- **Type**: User Story
- **Intent & Value**: As an automation author, I want reliable, standard exit codes so that scripts can distinguish between success, verification failure, and syntax errors.
- **Observable Contract & Behavior**: Terminates with standardized POSIX exit codes: 0 for success, 1 for traceability violations, 2 for CLI usage/syntax/configuration errors, and >2 for system errors.
- **Acceptance Criteria**: Verification failures reliably exit with 1; invalid arguments or missing files reliably exit with 2; successful checks exit with 0.


## 6. Layer 5: Requirements (REQ-*)

### REQ-CLI-INIT: Repository Initialization Command
- **ID**: REQ-CLI-INIT
- **Parent**: US-INIT-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide a deterministic command to bootstrap xtrc configuration and initial specification directory structure in any repository.
- **Observable Contract & Behavior**: Command `xtrc init` accepts `--path <PATH>`, `--force`, and `--quiet`. Creates `.xtrc.toml` with default include/exclude globs and starter directory hierarchy.
- **Acceptance Criteria**: Exits 0 and outputs confirmation to stdout upon creating `.xtrc.toml`; fails with exit code 2 and descriptive error to stderr if `.xtrc.toml` already exists without `--force`; overwrites atomically when `--force` is passed.

### REQ-CFG-DISCOVERY: Root and Configuration Discovery
- **ID**: REQ-CFG-DISCOVERY
- **Parent**: US-DISC-01
- **Type**: Functional Requirement
- **Intent & Value**: Enable transparent execution from nested subdirectories by discovering repository root and configuration automatically.
- **Observable Contract & Behavior**: Searches current working directory and successive parent directories until finding `.xtrc.toml` or `.git`. Accepts optional `--root <PATH>` or `--config <PATH>` overrides.
- **Acceptance Criteria**: Successfully identifies project root from nested directories; executes all commands relative to discovered root; exits code 2 with helpful diagnostic if no project root is discovered.

### REQ-IGNORE-RULES: Filesystem Traversal and Ignore Filtering
- **ID**: REQ-IGNORE-RULES
- **Parent**: US-IGNORE-01
- **Type**: Functional Requirement
- **Intent & Value**: Ensure fast, clean repository scanning by respecting Git ignore semantics and ignoring non-text binary assets.
- **Observable Contract & Behavior**: Traverses files while strictly honoring root and nested `.gitignore` files, `.git/` metadata, and user-configured exclude globs. Detects and skips binary files.
- **Acceptance Criteria**: Files matching `.gitignore` patterns are completely omitted from scan results; binary assets containing null bytes are ignored without reading entire files into memory.

### REQ-DOC-PARSE: Plain-Text Specification Document Parsing
- **ID**: REQ-DOC-PARSE
- **Parent**: US-DOC-01
- **Type**: Functional Requirement
- **Intent & Value**: Ingest human-readable Markdown specifications with YAML frontmatter into structured traceability nodes.
- **Observable Contract & Behavior**: Parses Markdown documents matching specification globs. Extracts YAML frontmatter metadata (`document_type`, `id`, `title`, `version`, `status`) and heading-based nodes (`### <ID>: <Title>`) with key-value metadata.
- **Acceptance Criteria**: Accurately extracts node ID, parent links, layer, type, and text content; reports parse errors with exact file and line numbers to stderr; rejects invalid YAML or missing required fields with exit code 2.

### REQ-CODE-SCAN: Language-Agnostic Source Code Comment Scanning
- **ID**: REQ-CODE-SCAN
- **Parent**: US-CODE-01
- **Type**: Functional Requirement
- **Intent & Value**: Extract implementation links from source files across any programming language without external plugins.
- **Observable Contract & Behavior**: Scans files matching source code globs using a line-oriented regular expression matching `@trace <ID>` markers within single-line, block, or doc comments.
- **Acceptance Criteria**: Records file path, line number, and target ID for each valid marker; supports multi-target comma-separated comments; skips string literals and binary files.

### REQ-TEST-SCAN: Automated Test Annotation Scanning
- **ID**: REQ-TEST-SCAN
- **Parent**: US-TEST-01
- **Type**: Functional Requirement
- **Intent & Value**: Extract verification links from automated test files to prove requirement verification.
- **Observable Contract & Behavior**: Scans files matching test globs or directory patterns for `@trace <ID>` markers, categorizing extracted nodes as verification evidence.
- **Acceptance Criteria**: Accurately classifies matches in test directories/files as test verification nodes; links tests to target specification nodes in the graph.

### REQ-DAG-MODEL: In-Memory 7-Layer Directed Acyclic Graph
- **ID**: REQ-DAG-MODEL
- **Parent**: US-GRAPH-01
- **Type**: Functional Requirement
- **Intent & Value**: Maintain a consistent, queryable graph representation of all lifecycle items and their bidirectional relationships.
- **Observable Contract & Behavior**: Ingests all parsed specification nodes, source code links, and test links into an in-memory graph. Maintains forward decomposition and backward justification adjacency indices.
- **Acceptance Criteria**: Supports fast lookups of parents and children for any node ID; maintains node provenance (file path and line number); rejects duplicate node IDs with exit code 1.

### REQ-CYCLE-DETECT: Graph Cycle Detection and Diagnostics
- **ID**: REQ-CYCLE-DETECT
- **Parent**: US-CYCLE-01
- **Type**: Functional Requirement
- **Intent & Value**: Prevent circular logic and maintain a strict hierarchical Directed Acyclic Graph across all layers.
- **Observable Contract & Behavior**: Performs cycle detection on the graph. When a circular dependency is detected, extracts and formats the complete cycle loop path.
- **Acceptance Criteria**: Detects circular reference loops (e.g., A -> B -> C -> A); prints the full cycle chain and source file locations to stderr; exits with code 1.

### REQ-ORPHAN-DETECT: Orphan Node and Upward Justification Validation
- **ID**: REQ-ORPHAN-DETECT
- **Parent**: US-ORPHAN-01
- **Type**: Functional Requirement
- **Intent & Value**: Prevent ungrounded features and dead code by verifying that every non-root item links back to business intent.
- **Observable Contract & Behavior**: Identifies any node (excluding Layer 1 Vision roots) that lacks an upward parent justification edge or whose parent references cannot be resolved.
- **Acceptance Criteria**: Discovers and reports all orphan nodes with file locations; emits warnings to stderr in default mode; causes command failure (exit code 1) in `--strict` mode.

### REQ-BROKEN-LINK: Broken Reference and Dangling Link Detection
- **ID**: REQ-BROKEN-LINK
- **Parent**: US-VERIFY-01
- **Type**: Functional Requirement
- **Intent & Value**: Guarantee referential integrity across the entire repository by flagging links to non-existent items.
- **Observable Contract & Behavior**: Checks every declared parent reference in specifications and every `@trace <ID>` target in code/tests against the registered node symbol table.
- **Acceptance Criteria**: Identifies all dangling references to non-existent IDs; outputs origin file, line number, and missing target ID to stderr; exits with code 1.


### REQ-FORWARD-COV: Forward Completeness and Coverage Tracking
- **ID**: REQ-FORWARD-COV
- **Parent**: US-VERIFY-01
- **Type**: Functional Requirement
- **Intent & Value**: Verify that all functional and non-functional requirements are realized by technical specifications, source code, and automated tests.
- **Observable Contract & Behavior**: Audits each requirement for downward fulfillment paths reaching implementation and automated tests. Computes repository-wide and per-epic coverage percentages.
- **Acceptance Criteria**: Enumerates all uncovered and partially covered requirements; prints coverage statistics to stdout; exits 0 in default mode if graph is otherwise valid.

### REQ-BACKWARD-JUST: Backward Traceability and Provenance Verification
- **ID**: REQ-BACKWARD-JUST
- **Parent**: US-VERIFY-01
- **Type**: Functional Requirement
- **Intent & Value**: Ensure every line of annotated code and test directly serves approved requirements, eliminating dead code and rogue scope.
- **Observable Contract & Behavior**: Traces every code marker and test assertion upward through technical specifications and user stories to confirm a continuous path to Vision.
- **Acceptance Criteria**: Identifies items lacking a path to root Vision; outputs provenance trace paths; reports ungrounded items in validation summaries.

### REQ-STRICT-GATE: Strict CI Verification Gate
- **ID**: REQ-STRICT-GATE
- **Parent**: US-GATE-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide a zero-tolerance validation mode suitable for automated CI/CD gating and pre-release compliance.
- **Observable Contract & Behavior**: When `--strict` is passed to `xtrc check` or `xtrc verify`, treats any warning, orphan node, broken link, or requirement missing code/test fulfillment as a fatal error.
- **Acceptance Criteria**: Exits with status code 1 if coverage is under 100% or any orphan/broken reference exists; exits with status code 0 only when the graph is complete, fully justified, and defect-free.

### REQ-CMD-MATRIX: Traceability Matrix Reporting
- **ID**: REQ-CMD-MATRIX
- **Parent**: US-MATRIX-01
- **Type**: Functional Requirement
- **Intent & Value**: Give teams an instant tabular overview of all requirements mapped across specifications, implementation code, and test verification.
- **Observable Contract & Behavior**: Command `xtrc matrix` prints an aligned text table to stdout. Supports `--uncovered` to filter for gaps, and `--format <table|csv|json>`.
- **Acceptance Criteria**: Renders complete matrix with ID, title, spec link, code link, and test link; correctly filters rows with `--uncovered`; exits 0 on success and 2 on invalid format.

### REQ-CMD-TREE: Hierarchical Dependency Tree Visualization
- **ID**: REQ-CMD-TREE
- **Parent**: US-TREE-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide interactive terminal exploration of parent-child relationships for any node in the traceability hierarchy.
- **Observable Contract & Behavior**: Command `xtrc tree <ID>` renders an ASCII/Unicode hierarchy tree starting from `<ID>`. Supports `--reverse` to render upward ancestors to Vision, and `--depth <N>`.
- **Acceptance Criteria**: Correctly renders indentation and tree branches to stdout; exits 1 with diagnostic to stderr if `<ID>` is not found; exits 2 if depth is non-numeric.

### REQ-CMD-EXPORT: Compliance and Traceability Export
- **ID**: REQ-CMD-EXPORT
- **Parent**: US-EXPORT-01
- **Type**: Functional Requirement
- **Intent & Value**: Enable generation of static compliance documentation for formal audits, safety reviews, and external stakeholders.
- **Observable Contract & Behavior**: Command `xtrc export --format <markdown|json>` writes a consolidated, self-contained traceability document to stdout or to a file specified by `-o, --output <PATH>`.
- **Acceptance Criteria**: Emits complete specification and link records; creates target output file atomically; exits 0 on success; exits 2 if output directory does not exist or cannot be written.

### REQ-FMT-JSON: Universal Machine-Readable JSON Output
- **ID**: REQ-FMT-JSON
- **Parent**: US-JSON-01
- **Type**: Functional Requirement
- **Intent & Value**: Facilitate automated scripting, CI tooling, and dashboard integrations by providing structured, machine-parseable data.
- **Observable Contract & Behavior**: All verification and query commands (`check`, `matrix`, `tree`, `export`) accept `--json`. When active, command produces a single valid JSON document on stdout.
- **Acceptance Criteria**: Output parses with standard JSON parsers (`jq`); contains structured schema version, node arrays, link lists, and summary statistics; diagnostic logs remain on stderr.

### REQ-UNIX-STREAMS: Standard Unix Streams Separation
- **ID**: REQ-UNIX-STREAMS
- **Parent**: US-UNIX-01
- **Type**: Functional Requirement
- **Intent & Value**: Conform to the Unix philosophy to allow predictable composition with command-line pipes, redirections, and shell filters.
- **Observable Contract & Behavior**: All program data, reports, tables, trees, and JSON payloads are written strictly to standard output (`stdout`). All log messages, scan spinners, progress updates, warnings, and error diagnostics are written strictly to standard error (`stderr`).
- **Acceptance Criteria**: Commands piped to downstream utilities (e.g., `xtrc check --json | jq .`) receive pure data on stdout without log pollution; disables ANSI color codes when stdout is not a terminal.

### REQ-EXIT-CODES: Deterministic Process Exit Codes
- **ID**: REQ-EXIT-CODES
- **Parent**: US-EXIT-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide clear, deterministic process return codes so CI/CD pipelines and developer scripts can automate control flow reliably.
- **Observable Contract & Behavior**: Emits exit code 0 when command succeeds and verification passes; exit code 1 when traceability integrity fails (broken links, cycles, or strict-mode violations); exit code 2 when command-line syntax, argument parsing, or file I/O fails; exit code >2 for internal fatal errors.
- **Acceptance Criteria**: Verification failures reliably return exit code 1; invalid arguments or missing configurations return exit code 2; valid checks return exit code 0.

### REQ-NFR-PERF: Sub-Second Execution Performance
- **ID**: REQ-NFR-PERF
- **Parent**: US-PERF-01, US-UNIX-01
- **Type**: Non-Functional Requirement
- **Intent & Value**: Ensure developers run traceability verification frequently as part of their inner loop without noticeable workflow delays, matching the throughput and resource efficiency of standard Unix text utilities.
- **Observable Contract & Behavior**: Repository traversal, file parsing, graph construction, cycle detection, and full graph verification execute in under 500 milliseconds on repositories containing at least 1,000 files, and self-repository CLI operations (`check --strict`, `matrix`, `tree`, `export`) execute in under 200 milliseconds. Scanning and parsing employ fast literal pre-filtering before regex matching, lazy line boundary and line-number resolution around matches, single-pass binary sniffing and file ingestion, single-pass streaming document parsing, $O(V + E)$ linear-time graph reachability validation, and buffered output stream emission.
- **Acceptance Criteria**: Automated benchmarks and CI/CD performance measurements confirm 1,000-file repository verification completes within `< 500ms` and self-repository CLI commands complete within `< 200ms`; files lacking `@trace` markers bypass full regex evaluation and line splitting.

### REQ-NFR-ROBUST: Robust Error Handling and Crash-Free Operation
- **ID**: REQ-NFR-ROBUST
- **Parent**: US-PERF-01, US-UNIX-01
- **Type**: Non-Functional Requirement
- **Intent & Value**: Guarantee stability and actionable feedback under all unexpected inputs, preventing tool crashes, panics, or unhelpful stack traces.
- **Observable Contract & Behavior**: All error conditions (malformed files, unreadable paths, permission errors, syntax issues) are captured and reported as user-actionable messages on stderr with relevant file, line, and column context.
- **Acceptance Criteria**: CLI never terminates with an unhandled panic or naked assertion failure on malformed input; diagnostic messages guide the user on how to resolve the issue.
