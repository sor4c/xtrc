---
document_type: specification
id: SPEC-DEMO-001
title: End-to-End Tracing Demo, Execution Environment, and Reports Specification
version: 1.0.0
status: approved
governance_tracking:
  rule_1_issue: "#21"
  rule_2: "Requirements Before Design"
  rule_3: "Intent Over Implementation"
traceability_layers:
  - User Stories
  - Requirements
  - Technical Specs
schema_version: "1.0.0"
---

# End-to-End Tracing Demo, Execution Environment, and Reports Specification

## 1. Layer 4: User Stories (US-*)

### US-DEMO-01: End-to-End Traceability Demonstration and Inspection
- **ID**: US-DEMO-01
- **Parent**: EPIC-QUERY, EPIC-CLI-CONTRACT
- **Type**: User Story
- **Intent & Value**: As a software engineer, engineering lead, or compliance auditor evaluating or adopting `xtrc`, I want a complete, realistic, runnable end-to-end demo showing an example 7-layer tracing setup, an automated execution environment, and resulting reports (HTML, JSON, Markdown, CSV) so that I can immediately understand how to structure, annotate, and verify traceability in real projects.
- **Observable Contract & Behavior**: Provides a self-contained example workspace (`examples/demo/`), a turnkey execution runner script (`examples/demo/run-demo.sh`), and committed resulting reports (`examples/demo/reports/`), accompanied by clear documentation and automated integration test coverage.
- **Acceptance Criteria**: The example workspace passes strict verification (`xtrc check --strict`), the runner generates all 5 report formats without error, and all generated artifacts are valid.

## 2. Layer 5: Requirements (REQ-*)

### REQ-DEMO-SETUP: Self-Contained 7-Layer Tracing Example Setup
- **ID**: REQ-DEMO-SETUP
- **Parent**: US-DEMO-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide a realistic example repository demonstrating all 7 lifecycle layers: Vision, Strategy, Epic, User Story, Requirement, Technical Spec, Code implementation, and Automated Tests.
- **Observable Contract & Behavior**: Located in `examples/demo/`, containing `.xtrc.toml`, specifications in `specs/`, polyglot source code in `src/`, and test suites in `tests/`. Running `xtrc check --strict` on this workspace passes with 100% test coverage, 0 broken links, 0 cycles, and 0 orphan nodes.
- **Acceptance Criteria**: All 7 layers are fully linked; running `xtrc check --strict` in `examples/demo/` exits 0 with 100% coverage.

### REQ-DEMO-RUNNER: Turnkey Demo Execution Environment and Script
- **ID**: REQ-DEMO-RUNNER
- **Parent**: US-DEMO-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide a single-command automated execution environment that runs every core `xtrc` capability and populates the reports directory.
- **Observable Contract & Behavior**: Executable script `examples/demo/run-demo.sh` locates the `xtrc` binary, executes `check --strict`, `tree`, `matrix` (multiple formats), and `export` (HTML, JSON, Markdown, CSV), logging progress and writing outputs to `examples/demo/reports/`.
- **Acceptance Criteria**: Runs non-interactively with exit code 0; supports overriding the binary path via `--xtrc-bin <PATH>` or `$XTRC_BIN`; requires zero network calls or external dependencies.

### REQ-DEMO-REPORTS: Pre-Generated Resulting Audit and Compliance Reports
- **ID**: REQ-DEMO-REPORTS
- **Parent**: US-DEMO-01
- **Type**: Functional Requirement
- **Intent & Value**: Allow users, evaluators, and auditors to inspect the resulting reports directly from GitHub or local checkout without needing to build from source first.
- **Observable Contract & Behavior**: Stores valid, up-to-date reports in `examples/demo/reports/`: `traceability-report.html` (standalone HTML5 with styled metrics and badges), `traceability-report.json` (universal envelope), `traceability-report.md` (GitHub-flavored Markdown), `traceability-report.csv` (RFC 4180), and `matrix.html` (HTML table).
- **Acceptance Criteria**: All 5 report files exist, are valid according to their schemas, and are updated deterministically by `run-demo.sh`.

### REQ-VER-BUMP-PATCH: Semantic Version Patch Increment
- **ID**: REQ-VER-BUMP-PATCH
- **Parent**: REQ-VER-POLICY, US-PERF-01
- **Type**: Functional Requirement
- **Intent & Value**: Increment patch version in `Cargo.toml` from `0.1.1` to `0.1.2` to release bug fixes, performance enhancements, and the new demo/reports feature.
- **Observable Contract & Behavior**: Updates `Cargo.toml` and `Cargo.lock` to `0.1.2`, ensuring Head-of-Main delivery produces release `v0.1.2`.
- **Acceptance Criteria**: `cargo check` and `scripts/compute-version.sh` reflect `0.1.2`.

## 3. Layer 6: Technical Specifications (SPEC-*)

### SPEC-DEMO-WORKSPACE: Structure and Content of Example Tracing Workspace
- **ID**: SPEC-DEMO-WORKSPACE
- **Parent**: REQ-DEMO-SETUP, REQ-DEMO-RUNNER
- **Type**: TechnicalSpec
- **Component Target**: examples/demo/, examples/demo/.xtrc.toml, examples/demo/specs/, examples/demo/src/, examples/demo/tests/
- **Verification Target**: tests/e2e/test_demo.rs
- **Technical Specification**:
  - Example Domain: `Order & Payment Checkout Gateway`
  - Specification Document: `examples/demo/specs/checkout.md` containing:
    - Vision: `VIS-DEMO-PAY` (Instant, Frictionless, and Compliant Cloud Payments)
    - Strategy: `STRAT-DEMO-RELIABILITY` (Sub-100ms P99 Latency and PCI Zero-Trust Compliance)
    - Epic: `EPIC-DEMO-PAY-ENGINE` (Real-Time Payment Processing Engine)
    - User Story: `US-DEMO-CHECKOUT` (Customer One-Click Checkout)
    - Requirements: `REQ-DEMO-AUTH` (Tokenized Card Authentication) and `REQ-DEMO-CAPTURE` (Atomic Transaction Capture)
    - Technical Specs: `SPEC-DEMO-AUTH-SVC` (Cryptographic Token Validation Service) and `SPEC-DEMO-CAPTURE-SVC` (Atomic Ledger Settlement Engine)
  - Code Implementation: `examples/demo/src/checkout.py` and `examples/demo/src/ledger.rs` with `// @trace` annotations.
  - Automated Tests: `examples/demo/tests/test_checkout.py` and `examples/demo/tests/test_ledger.rs` with `// @trace` annotations.

### SPEC-DEMO-REPORTS: Resulting Reports Generation and Formats
- **ID**: SPEC-DEMO-REPORTS
- **Parent**: REQ-DEMO-REPORTS, SPEC-REPORT-HTML
- **Type**: TechnicalSpec
- **Component Target**: examples/demo/reports/, examples/demo/run-demo.sh
- **Verification Target**: tests/e2e/test_demo.rs
- **Technical Specification**:
  - Runner Script: `examples/demo/run-demo.sh`
    - Option `--xtrc-bin <PATH>` / env var `XTRC_BIN`.
    - Automated commands:
      - `xtrc check --strict`
      - `xtrc tree VIS-DEMO-PAY`
      - `xtrc matrix --format html -o reports/matrix.html`
      - `xtrc matrix --format markdown -o reports/matrix.md`
      - `xtrc export --format html -o reports/traceability-report.html`
      - `xtrc export --format json -o reports/traceability-report.json`
      - `xtrc export --format markdown -o reports/traceability-report.md`
      - `xtrc export --format csv -o reports/traceability-report.csv`
  - Reports directory committed with all generated outputs for immediate inspection.

### SPEC-VER-BUMP: Monotonic Semantic Version Increment
- **ID**: SPEC-VER-BUMP
- **Parent**: REQ-VER-BUMP-PATCH, SPEC-VER-POLICY
- **Type**: TechnicalSpec
- **Component Target**: Cargo.toml, Cargo.lock, build.rs
- **Verification Target**: tests/e2e/test_demo.rs, tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Increments package version in `Cargo.toml` from `0.1.1` to `0.1.2`.
  - Statically and dynamically guarantees unique delivery releases via `scripts/compute-version.sh`.

