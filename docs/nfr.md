---
document_type: specification
id: SPEC-NFR-001
title: Quantitative Non-Functional Requirements (NFR) and CI/CD Performance Gate Specification
version: 1.0.0
status: approved
governance_tracking:
  rule_1_issue: "#17"
  rule_2: "Requirements Before Design"
  rule_3: "Intent Over Implementation"
traceability_layers:
  - User Stories
  - Requirements
  - Technical Specs
schema_version: "1.0.0"
---

# Quantitative Non-Functional Requirements (NFR) and CI/CD Performance Gate Specification

## 1. Layer 4: User Stories (US-*)

### US-PERF-01: Quantitative Performance, Memory, Scalability, and Determinism Guarantees
- **ID**: US-PERF-01
- **Parent**: EPIC-VERIFY, EPIC-CLI-CONTRACT
- **Type**: User Story
- **Intent & Value**: As a software engineer and CI pipeline maintainer, I want `xtrc` to enforce quantitative Non-Functional Requirements (sub-second latency, bounded memory usage, linear $O(V + E)$ scalability, and deterministic parallel output) so that traceability verification never bottlenecks developer inner loops or CI/CD pipelines.
- **Observable Contract & Behavior**: Executes repository scanning, specification parsing, graph validation, and query commands within strict latency and memory budgets across single-core and multi-core environments with deterministic, reproducible output.
- **Acceptance Criteria**: All NFR budgets (`< 500ms` for 1,000-file workspace, `< 200ms` for self-repository CLI verification, `<= 512B` binary header probe, `< 50 MiB` memory footprint, $O(V + E)$ scaling, and byte-identical parallel output) are verified by automated tests and CI/CD performance gates.

## 2. Layer 5: Non-Functional Requirements (REQ-*)

### REQ-NFR-MEM: Bounded Memory Footprint and Streaming Binary Sniffing
- **ID**: REQ-NFR-MEM
- **Parent**: US-PERF-01, US-IGNORE-01
- **Type**: Non-Functional Requirement
- **Intent & Value**: Prevent excessive heap allocations or memory exhaustion when scanning repositories containing large binary assets or thousands of source files in constrained developer terminals and CI containers.
- **Observable Contract & Behavior**: Binary file detection inspects at most the first 512 bytes of any candidate file and immediately skips binary assets containing `0x00` null bytes without reading the remainder of the file into memory. Non-matching source files incur zero regex or line-table heap allocations, keeping workspace graph memory consumption bounded (`< 50 MiB` for 1,000-file repositories).
- **Acceptance Criteria**: Automated tests verify that multi-megabyte binary files are classified and skipped via 512-byte header probes in `< 25ms` and that 1,000-file workspace graph construction stays within the `< 50 MiB` memory budget.

### REQ-NFR-SCALE: Multi-Core Linear Scalability and O(V + E) Graph Complexity
- **ID**: REQ-NFR-SCALE
- **Parent**: US-PERF-01, US-GRAPH-01
- **Type**: Non-Functional Requirement
- **Intent & Value**: Guarantee that file categorization, comment scanning, cycle detection, and bidirectional graph validation scale linearly with repository file count and graph size without quadratic $O(N^2)$ or $O(V^2)$ degradation.
- **Observable Contract & Behavior**: Workspace file categorization partitions specification, source, test, and other text files in a single $O(N)$ pass; specification parsing and comment scanning scale across available CPU cores via work-stealing parallelism; and 3-color DFS cycle detection and Vision reachability validation execute in $O(V + E)$ linear time.
- **Acceptance Criteria**: Automated scalability benchmarks confirm linear $O(V + E)$ graph validation (`< 100ms` on multi-thousand-node DAGs) and fast literal pre-filtering (`< 50ms` on 1 MiB of non-matching text) without quadratic blowup.

### REQ-NFR-DETERM: Deterministic and Reproducible Multi-Core Output
- **ID**: REQ-NFR-DETERM
- **Parent**: US-PERF-01, US-UNIX-01
- **Type**: Non-Functional Requirement
- **Intent & Value**: Ensure parallel multi-core file traversal and comment scanning never introduce nondeterministic ordering in CLI tables, dependency trees, exports, JSON reports, or error diagnostics.
- **Observable Contract & Behavior**: Regardless of OS thread scheduling or filesystem walk order during parallel execution, all graph node indices, edge adjacency lists, validation violations, matrix rows, tree branches, and compliance exports are canonically ordered so that repeated runs emit byte-for-byte identical output.
- **Acceptance Criteria**: Automated determinism tests execute repeated parallel workspace scans and graph validations and verify byte-identical serialized outputs across runs.

### REQ-CI-PERF: Automated CI/CD Performance Measurement and NFR Gate Enforcement
- **ID**: REQ-CI-PERF
- **Parent**: US-CICD-01, US-PERF-01
- **Type**: Non-Functional Requirement
- **Intent & Value**: Continuously measure real CLI and engine execution latencies during CI/CD runs and automatically block pull requests or deliveries if any performance requirement (`REQ-NFR-PERF`, `REQ-NFR-MEM`, `REQ-NFR-SCALE`, `REQ-NFR-DETERM`) regresses or is unfulfilled.
- **Observable Contract & Behavior**: Both `.github/workflows/ci.yml` and `.github/workflows/delivery.yml` execute the automated NFR benchmark suite (`cargo test --test test_benchmarks`) and the release-binary performance measurement harness (`bash scripts/measure-perf.sh`), generating a structured JSON performance report (`dist/perf-report.json`) and Markdown latency summary while enforcing strict SLA thresholds (`<= 500ms` for 1,000-file workspace verification, `<= 200ms` for self-repository CLI operations).
- **Acceptance Criteria**: CI and CD workflows fail with a non-zero exit code if any NFR benchmark or SLA threshold fails; `perf-report.json` records measured latencies, SLA budgets, and pass/fail status across all benchmarked workloads.

## 3. Layer 6: Technical Specifications (SPEC-*)

- **ID**: SPEC-PERF-BOUNDS
- **Parent**: REQ-NFR-PERF, US-PERF-01
- **Type**: TechnicalSpec
- **Component Target**: src/scanner/comments.rs, src/scanner/traversal.rs, src/cli/output.rs, src/cli/tree.rs, src/cli/matrix.rs, src/cli/export.rs, src/cli/verify.rs
- **Verification Target**: tests/perf/test_benchmarks.rs, tests/scanner/test_comments.rs, tests/cli/test_output.rs
- **Technical Specification**:
  - Comment scanning literal `@trace` pre-filtering rejecting non-matching lines with zero regex invocations.
  - CLI output streams write through `write_buffered_stdout` (`BufWriter` over locked `stdout`).
  - Sub-second execution thresholds for 1,000-file repositories.

### SPEC-PERF-HARNESS: Automated NFR Benchmark & SLA Verification Suite
- **ID**: SPEC-PERF-HARNESS
- **Parent**: REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, US-PERF-01
- **Type**: TechnicalSpec
- **Component Target**: src/cli/pipeline.rs, src/scanner/comments.rs, src/scanner/traversal.rs, src/graph/validator.rs
- **Verification Target**: tests/perf/test_benchmarks.rs
- **Technical Specification**:
  - Dedicated integration benchmark test suite `tests/perf/test_benchmarks.rs` (`[[test]] name = "test_benchmarks"` in `Cargo.toml`) executed automatically during `cargo test --all-targets --all-features`.
  - Verifies four quantitative NFR dimensions:
    1. **Sub-Second 1,000-File Pipeline Latency (`REQ-NFR-PERF`)**: Generates a realistic 1,000-file polyglot workspace across all 7 traceability layers and verifies end-to-end `load_workspace_graph`, `detect_cycles`, and `validate_graph` execution completes in `< 500ms`.
    2. **Bounded Binary Sniffing & Memory Efficiency (`REQ-NFR-MEM`)**: Verifies multi-megabyte binary files containing `0x00` within the initial 512-byte header are skipped in `< 25ms` without reading the full file payload, and verifies 1,000-file graph memory footprint remains `< 50 MiB`.
    3. **Linear $O(V + E)$ Scalability & Pre-Filter Throughput (`REQ-NFR-SCALE`)**: Verifies 2,400-node DAG validation and cycle detection completes in `< 100ms` and literal `@trace` pre-filtering scans 1 MiB of non-matching source code in `< 50ms`.
    4. **Parallel Multi-Core Determinism (`REQ-NFR-DETERM`)**: Executes repeated parallel workspace loads and CLI query formatting (`check --json`, `matrix`, `tree`, `export`) and asserts byte-for-byte identical output.

### SPEC-CI-PERF: Automated CI/CD Performance Measurement Harness and SLA Gate Contract
- **ID**: SPEC-CI-PERF
- **Parent**: REQ-CI-PERF, REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, US-CICD-01, US-PERF-01
- **Type**: TechnicalSpec
- **Component Target**: scripts/measure-perf.sh, .github/workflows/ci.yml, .github/workflows/delivery.yml, src/cli/pipeline.rs
- **Verification Target**: tests/perf/test_benchmarks.rs, tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Script Path: `scripts/measure-perf.sh [OUTPUT_JSON] [BINARY_PATH]`.
  - Measures release-binary wall-clock latency (in milliseconds) and enforces hard SLA budgets across five benchmark workloads:
    1. `self_check_strict`: `xtrc check --strict --json` on the `xtrc` repository (budget: `<= 200ms`).
    2. `self_matrix_json`: `xtrc matrix --format json` on the `xtrc` repository (budget: `<= 200ms`).
    3. `self_tree_json`: `xtrc tree --json` on the `xtrc` repository (budget: `<= 200ms`).
    4. `self_export_markdown`: `xtrc export --format markdown` on the `xtrc` repository (budget: `<= 200ms`).
    5. `synthetic_1000_files_check_strict`: `xtrc check --strict --json` on a generated 1,000-file polyglot workspace across all 7 traceability layers plus binary assets (budget: `<= 500ms`).
  - Outputs deterministic JSON report (`dist/perf-report.json`) with `schema_version`, `status` (`"pass"` / `"fail"`), `benchmarks` array (`name`, `measured_ms`, `budget_ms`, `passed`), and appends a Markdown summary table to `$GITHUB_STEP_SUMMARY` when running in GitHub Actions.
  - Exits with code `0` only if all SLA budgets pass; exits with code `1` if any workload breaches its budget or fails verification.
