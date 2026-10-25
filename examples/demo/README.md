# xtrc End-to-End Tracing Demo & Execution Environment

This directory provides a realistic, turnkey demonstration of **plain-text, version-control-native traceability** using `xtrc`.

It models an e-commerce **Order & Payment Checkout Gateway** spanning the complete 7-layer lifecycle DAG:
**Vision $\to$ Strategy $\to$ Epics $\to$ User Stories $\to$ Requirements $\to$ Technical Specs $\to$ Polyglot Code & Tests**.

---

## 1. Traceability Architecture & Example Setup

The example project structure demonstrates how specifications, polyglot source code, and automated tests are organized:

```
examples/demo/
├── .xtrc.toml                  # Workspace scanning & validation configuration
├── specs/
│   └── checkout.md             # Markdown specifications across Layers 1–6
├── src/
│   ├── checkout.py             # Python token authentication with @trace annotations
│   └── ledger.rs               # Rust atomic double-entry ledger with @trace annotations
├── tests/
│   ├── test_checkout.py        # Python unit & security tests with @trace annotations
│   └── test_ledger.rs          # Rust ledger unit tests with @trace annotations
├── reports/                    # Pre-generated compliance & audit reports
│   ├── traceability-report.html# Interactive standalone HTML5 compliance dashboard
│   ├── traceability-report.json# Machine-readable JSON compliance telemetry
│   ├── traceability-report.md  # GitHub-flavored Markdown audit report
│   ├── traceability-report.csv # RFC 4180 CSV export for spreadsheets/BI
│   ├── matrix.html             # HTML traceability matrix table
│   └── matrix.md               # Markdown traceability matrix table
└── run-demo.sh                 # Turnkey demo runner script
```

### The 7-Layer Dependency Graph

| Layer | Node ID | Title / Role | Links & Parents |
|---|---|---|---|
| **Layer 1: Vision** | `VIS-DEMO-PAY` | Instant, Frictionless, and Compliant Cloud Payments | Root |
| **Layer 2: Strategy** | `STRAT-DEMO-RELIABILITY` | Sub-100ms P99 Latency and PCI Zero-Trust Security | Parent: `VIS-DEMO-PAY` |
| **Layer 3: Epic** | `EPIC-DEMO-PAY-ENGINE` | Real-Time Payment Processing Engine | Parent: `STRAT-DEMO-RELIABILITY` |
| **Layer 4: User Story** | `US-DEMO-CHECKOUT` | Customer One-Click Checkout | Parent: `EPIC-DEMO-PAY-ENGINE` |
| **Layer 5: Requirement** | `REQ-DEMO-AUTH` | Tokenized Card Authentication | Parent: `US-DEMO-CHECKOUT` |
| **Layer 5: Requirement** | `REQ-DEMO-CAPTURE` | Atomic Transaction Capture and Settlement | Parent: `US-DEMO-CHECKOUT` |
| **Layer 6: Technical Spec** | `SPEC-DEMO-AUTH-SVC` | Cryptographic Token Validation Service | Parent: `REQ-DEMO-AUTH` |
| **Layer 6: Technical Spec** | `SPEC-DEMO-CAPTURE-SVC` | Atomic Ledger Settlement Engine | Parent: `REQ-DEMO-CAPTURE` |
| **Layer 7: Source Code** | `src/checkout.py`, `src/ledger.rs` | Implementation logic | `@trace SPEC-*, REQ-*` |
| **Layer 7: Tests** | `tests/test_checkout.py`, `tests/test_ledger.rs` | Automated verification | `@trace SPEC-*, REQ-*` |

---

## 2. Execution Environment

### Option A: Run the Turnkey Automated Demo Script

From anywhere in the repository:

```bash
# Execute the complete automated demo runner
./examples/demo/run-demo.sh
```

To specify an explicit `xtrc` binary:
```bash
./examples/demo/run-demo.sh --xtrc-bin /path/to/xtrc
```

The script automatically:
1. Runs strict bidirectional verification (`xtrc check --strict`).
2. Visualizes the hierarchical relationship tree (`xtrc tree VIS-DEMO-PAY`).
3. Generates the multidimensional matrix in Table, Markdown, and HTML formats.
4. Exports comprehensive compliance reports in HTML, JSON, Markdown, and CSV formats.
5. Verifies the existence, non-emptiness, and integrity of all report artifacts.

### Option B: Run Individual CLI Commands

You can execute `xtrc` commands directly against this workspace:

```bash
# 1. Verify strict bidirectional traceability (100% test coverage gate)
xtrc --root examples/demo check --strict

# 2. Visualize full tree starting from the Vision root
xtrc --root examples/demo tree VIS-DEMO-PAY

# 3. Generate terminal table matrix
xtrc --root examples/demo matrix

# 4. Generate standalone HTML5 compliance audit report
xtrc --root examples/demo export --format html -o examples/demo/reports/traceability-report.html

# 5. Generate machine-readable JSON compliance report
xtrc --root examples/demo export --format json -o examples/demo/reports/traceability-report.json
```

### Option C: Continuous Integration Gate (GitHub Actions)

Add this step to `.github/workflows/ci.yml` to block pull requests on broken links or uncovered requirements:

```yaml
- name: Verify Demo Traceability Gate
  run: xtrc --root examples/demo check --strict
```

---

## 3. Resulting Reports

All resulting reports are pre-rendered and saved in the [`reports/`](reports/) directory:

1. [**`traceability-report.html`**](reports/traceability-report.html):
   - Standalone, zero-dependency HTML5 document with dark theme styling.
   - Executive metrics grid (Total Nodes: 8, Reqs: 2, Covered Reqs: 2, Code Links: 8, Test Links: 12, Coverage: 100.0%).
   - Prominent green `PASS` compliance badge.
   - Detailed breakdown tables across all 7 lifecycle layers.
2. [**`traceability-report.json`**](reports/traceability-report.json):
   - Conforms to the universal schema (`schema_version: "1.0.0"`).
   - Structured JSON payload suitable for programmatic ingestion by SIEMs, dashboards, and compliance databases.
3. [**`traceability-report.md`**](reports/traceability-report.md):
   - GitHub-flavored Markdown tables.
   - Easily embedded into GitHub Pull Request summaries or Release Notes.
4. [**`traceability-report.csv`**](reports/traceability-report.csv):
   - RFC 4180 CSV export mapping requirement IDs, titles, parent stories, child specs, code counts, test counts, and coverage status for spreadsheet review.
5. [**`matrix.html`**](reports/matrix.html) & [**`matrix.md`**](reports/matrix.md):
   - Clean tabular representation of requirement-to-code and requirement-to-test verification status.
