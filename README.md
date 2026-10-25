# xtrc

> xtrc is a fast, plain-text traceability CLI written in Rust. It tracks relationships across your entire workflow, linking vision, strategy, epics, and stories directly to technical specs and code. It lives purely in version control, ditching heavy databases so you can verify project alignment instantly right from your terminal.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org)
[![Verify](https://github.com/sor4c/xtrc/actions/workflows/ci.yml/badge.svg)](https://github.com/sor4c/xtrc/actions/workflows/ci.yml)
[![Head-of-Main Delivery](https://github.com/sor4c/xtrc/actions/workflows/delivery.yml/badge.svg)](https://github.com/sor4c/xtrc/actions/workflows/delivery.yml)

---

## The Vision

In modern software engineering, understanding *why* a piece of code exists and *what* user value it delivers is surprisingly difficult. Traditional Application Lifecycle Management (ALM) and requirements tracking tools—such as Jira, IBM DOORS, Enterprise Architect, and Jama—isolate requirements within heavyweight, proprietary database silos detached from the source code.

This centralized database architecture causes three critical problems:
1. **Synchronization Drift**: As code rapidly evolves in Git branches, database tickets fall out of sync. Requirements become stale artifacts rather than living contracts.
2. **Audit Friction & Compliance Drag**: Regulated industries and security-conscious teams spend weeks manually cross-referencing Git commit hashes with web tickets and spreadsheet rows to prove compliance.
3. **Developer Context Switching**: Forcing engineers out of their terminal and code editor into slow web dashboards creates friction, leading to abandoned or superficial tracking.

**`xtrc` eliminates the database silo.** By moving traceability directly into version control as human-readable plain text, traceability becomes as native to software engineering as Git branching, code review, and automated testing.

| Traditional ALM (Jira / DOORS) | The `xtrc` Plain-Text / VCS-Native Approach |
|---|---|
| **Database Silos**: Stored on remote servers behind proprietary APIs. | **Repository-Resident**: Plain text files committed alongside code. |
| **Out-of-Sync Branches**: Tickets represent only one "current" state. | **Branch-Aware**: Requirements branch, merge, and rebase with code. |
| **Heavyweight UI**: Sluggish web portals, complex permissions. | **Sub-Second CLI**: Instant terminal feedback in milliseconds. |
| **Disjointed Audits**: Manual cross-referencing between Git and ALMs. | **Automated Audits**: Deterministic CI gates and verifiable graphs. |

---

## Core Concepts

### 1. Plain-Text Traceability
All requirements, stories, architectural specifications, and verification records are stored as clean, human-readable plain text (Markdown, YAML, or TOML) inside your repository. Because they are plain text, they can be searched with standard Unix utilities (`grep`, `rg`), edited in any editor, and reviewed via pull requests.

### 2. VCS-Native Architecture
`xtrc` requires no external databases, no hosted cloud services, and zero network calls. Everything is tracked within Git. When you branch a feature, you branch its requirements. When a pull request merges, its traceability links and code are updated atomically.

### 3. Multidimensional Relationship Graph
`xtrc` parses your repository artifacts to build a comprehensive, directed acyclic graph (DAG) spanning business intent down to the binary level:

```
[ Vision ]
    │
    ▼
[ Strategy ]
    │
    ▼
[ Epics ]
    │
    ▼
[ User Stories ]
    │
    ▼
[ Technical Specs ]
    │
    ▼
[ Source Code ]
    │
    ▼
[ Automated Tests ]
```

Every node in the graph represents a traceable artifact with an identifier, and every edge represents a documented dependency or fulfillment relationship.

### 4. Bidirectional Traceability
Traceability must work in both directions to provide true project integrity:
- **Forward Traceability (Completeness)**: Ensures every high-level business requirement, epic, and user story is fully decomposed into technical specifications, implemented in source code, and verified by passing automated tests. It prevents forgotten requirements and unfulfilled promises.
- **Backward Traceability (Justification)**: Ensures every line of source code, technical specification, and architectural decision can be traced back to a legitimate user story and strategic objective. It eliminates gold-plating, rogue features, and orphan dead code.

### 5. Instant Terminal Verification
`xtrc` delivers blazing-fast, sub-second audits directly in your terminal. You can immediately identify broken references, unlinked code, or missing test coverage before pushing commits.

---

## Planned Workflow

`xtrc` seamlessly embeds into the standard developer lifecycle:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ 1. Intent   │ ──> │  2. Specs   │ ──> │   3. Code   │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
┌─────────────┐     ┌─────────────┐            ▼
│  6. Query   │ <── │   5. CI/CD  │ <── ┌─────────────┐
│  & Report   │     │    Gate     │     │  4. Verify  │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Step 1: Capture Intent
Define high-level product intent—vision, strategic goals, epics, and user stories—in version-controlled Markdown files or structured issue templates (`.github/ISSUE_TEMPLATE/`). Each requirement receives a unique identifier (e.g., `REQ-AUTH-01`).

### Step 2: Derive Technical Specifications
Derive technical specifications, architecture decisions (ADRs), and interface contracts that address the high-level requirements. Technical specs explicitly reference their parent requirement IDs.

### Step 3: Implement & Link in Code
Implement features in idiomatic code, embedding lightweight traceability annotations directly in docstrings and comments:
```rust
// @trace REQ-AUTH-01
// Implements token validation logic according to spec.
pub fn validate_session_token(token: &str) -> Result<Session, AuthError> {
    // ...
}
```

### Step 4: Verify Alignment Locally
Before committing, run `xtrc check` or `xtrc verify` in your terminal. `xtrc` parses the repository, builds the relationship graph, and surfaces any orphaned code, unmapped requirements, or broken reference links in milliseconds.

```console
$ xtrc check
Scanning repository artifacts...
Loaded: 12 requirements, 8 specs, 42 code links, 18 test links.
[OK] Bidirectional traceability verified (100% coverage, 0 broken links).
```

### Step 5: Enforce in CI/CD Gates
Integrate `xtrc` into your continuous integration pipeline (such as GitHub Actions). Pull requests with broken traceability links, missing test assertions, or unfulfilled acceptance criteria are blocked automatically.

```yaml
# .github/workflows/traceability.yml
- name: Verify Traceability
  run: xtrc check --strict
```

### Step 6: Query, Visualize & Report
Inspect the traceability graph on demand:
- `xtrc matrix`: Generate a full traceability matrix (requirements vs code vs tests).
- `xtrc tree REQ-AUTH-01`: Print the dependency tree for a specific requirement.
- `xtrc export --format html`: Export self-contained, publication-ready HTML compliance audit reports.
- `xtrc export --format json`: Export machine-readable JSON compliance telemetry.
- `xtrc export --format markdown`: Export compliance documentation in Markdown.

---

## Live End-to-End Demo & Example Setup

A complete, self-contained demonstration workspace is available in [`examples/demo/`](examples/demo/), modeling an e-commerce **Order & Payment Checkout Gateway** with full 7-layer traceability:

- **Specification Setup**: [`examples/demo/specs/checkout.md`](examples/demo/specs/checkout.md) defines Vision (`VIS-DEMO-PAY`), Strategy, Epics, User Stories, Requirements (`REQ-DEMO-AUTH`, `REQ-DEMO-CAPTURE`), and Technical Specs.
- **Polyglot Code & Test Annotations**:
  - Python token authentication in [`examples/demo/src/checkout.py`](examples/demo/src/checkout.py) and [`examples/demo/tests/test_checkout.py`](examples/demo/tests/test_checkout.py).
  - Rust transaction ledger in [`examples/demo/src/ledger.rs`](examples/demo/src/ledger.rs) and [`examples/demo/tests/test_ledger.rs`](examples/demo/tests/test_ledger.rs).
- **Turnkey Demo Runner**: Execute `./examples/demo/run-demo.sh` to run verification, tree visualizer, matrix generator, and all export formats in one command.
- **Resulting Reports**:
  - [Interactive HTML5 Audit Report](examples/demo/reports/traceability-report.html)
  - [Machine-Readable JSON Telemetry](examples/demo/reports/traceability-report.json)
  - [Markdown PR & Release Documentation](examples/demo/reports/traceability-report.md)
  - [CSV Compliance Matrix](examples/demo/reports/traceability-report.csv)
  - [HTML Matrix Table](examples/demo/reports/matrix.html)

For complete details, see the [Demo Documentation](examples/demo/README.md).

---

## Development & Getting Started

### Turnkey Dev Container (Recommended)
This repository includes a complete [VS Code Remote - Containers / GitHub Codespaces](https://containers.dev) environment configured in `.devcontainer/devcontainer.json`.
1. Open the repository in VS Code.
2. When prompted, click **Reopen in Container** (or run `Remote-Containers: Reopen in Container` from the Command Palette).
3. The Dev Container automatically provisions the Rust toolchain (`cargo`, `rustfmt`, `clippy`, `rust-analyzer`), Git, GitHub CLI (`gh`), and preconfigured editor extensions.

### Local Development Prerequisites
If developing locally without Docker/Dev Containers, ensure you have:
- **Rust Toolchain**: Stable Rust installed via [rustup](https://rustup.rs/).
- **Cargo**: Rust's package manager and build system.

### Installation & Packaging
Every verified push to `main` automatically runs the [Head-of-Main Delivery workflow](.github/workflows/delivery.yml), computes a unique monotonic version (`<MAJOR>.<MINOR>.<PATCH>-r<COMMIT_COUNT>.g<SHORT_SHA>`, e.g., `0.1.0-r25.g87a20ea`), publishes an immutable versioned release (`v<VERSION>`), and refreshes the [`main-head` GitHub Release](https://github.com/sor4c/xtrc/releases/tag/main-head):

```bash
# 1. Install via Homebrew (using the delivered xtrc.rb formula pinned to its immutable v<VERSION> release)
curl -fsSL https://github.com/sor4c/xtrc/releases/download/main-head/xtrc.rb -o /tmp/xtrc.rb
brew install --formula /tmp/xtrc.rb

# 2. Install Debian package (.deb) on Debian / Ubuntu
DEB_URL="$(curl -fsSL https://api.github.com/repos/sor4c/xtrc/releases/tags/main-head | jq -r '.assets[] | select(.name | endswith(".deb")) | .browser_download_url')"
curl -fsSLO "${DEB_URL}"
sudo dpkg -i xtrc_*_amd64.deb

# 3. Install prebuilt binary via cargo-binstall
cargo binstall xtrc

# 4. Install portable archive (.tar.gz) directly
TAR_URL="$(curl -fsSL https://api.github.com/repos/sor4c/xtrc/releases/tags/main-head | jq -r '.assets[] | select(.name | endswith(".tar.gz")) | .browser_download_url')"
curl -fsSL "${TAR_URL}" | tar -xz
sudo install -m 755 xtrc-v*/xtrc /usr/local/bin/xtrc

# 5. Build and install from source via Cargo
cargo install --path .
```

### Version Policy
To guarantee that two head-of-`main` deliveries with different content **never** share the same version string:
- **Base Version (`MAJOR.MINOR.PATCH`)**: Declared in `Cargo.toml` following Semantic Versioning 2.0.0.
- **Head-of-`main` / Commit Builds (`<MAJOR>.<MINOR>.<PATCH>-r<COMMIT_COUNT>.g<SHORT_SHA>`)**: Computed deterministically by [`scripts/compute-version.sh`](scripts/compute-version.sh) and [`build.rs`](build.rs) (e.g., `0.1.0-r25.g87a20ea`).
- **Exact Tagged Releases (`v<MAJOR>.<MINOR>.<PATCH>`)**: Emit the clean `<MAJOR>.<MINOR>.<PATCH>` version string.
- **Immutable Release Tags**: Every head-of-`main` delivery creates a permanent, immutable GitHub Release tagged `v<VERSION>` (and refuses to overwrite an existing version tag), while updating the `main-head` alias to hold only the latest delivery's artifacts.

### Software Bill of Materials (SPDX 2.3 JSON SBOM)
Every delivery includes a deterministic **SPDX 2.3 JSON SBOM** (`xtrc.spdx.json`):
- Embedded inside the `.tar.gz` release archive and Homebrew doc directory (`xtrc.spdx.json`).
- Installed by the Debian `.deb` package at `/usr/share/doc/xtrc/xtrc.spdx.json`.
- Attached as a standalone asset on the [`main-head` GitHub Release](https://github.com/sor4c/xtrc/releases/tag/main-head) alongside `SHA256SUMS`.
- Regenerated locally at any time via `./scripts/generate-sbom.sh dist/xtrc.spdx.json`.

### Non-Functional Requirements (NFRs) & Continuous Performance Gates
`xtrc` enforces strict quantitative Non-Functional Requirements across every commit, pull request, and release via [`tests/perf/test_benchmarks.rs`](tests/perf/test_benchmarks.rs) and [`scripts/measure-perf.sh`](scripts/measure-perf.sh):
- **Sub-Second Latency (`REQ-NFR-PERF`)**: Full 1,000-file polyglot repository verification completes in `< 500ms`; self-repository CLI operations (`check --strict`, `matrix`, `tree`, `export`) complete in `< 200ms`.
- **Bounded Memory & Binary Sniffing (`REQ-NFR-MEM`)**: Binary assets are skipped via a 512-byte header probe in `< 25ms` without full-file buffering; 1,000-file workspace graph metadata memory remains `< 50 MiB`.
- **Linear $O(V + E)$ Scalability (`REQ-NFR-SCALE`)**: Multi-thousand-node DAG validation and cycle detection execute in linear $O(V + E)$ time (`< 100ms`), and literal `@trace` pre-filtering scans 1 MiB of non-matching source text in `< 50ms`.
- **Deterministic Multi-Core Output (`REQ-NFR-DETERM`)**: Parallel work-stealing traversal and scanning emit byte-for-byte identical outputs across runs.
- **CI/CD SLA Enforcement (`REQ-CI-PERF`)**: Both [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and [`.github/workflows/delivery.yml`](.github/workflows/delivery.yml) run `bash scripts/measure-perf.sh dist/perf-report.json`, publish `perf-report.json` and a Markdown latency summary, and fail the pipeline if any NFR threshold is breached.

### Build and Test Commands
```bash
# Build the project
cargo build

# Run unit, integration, and NFR benchmark tests
cargo test

# Check code formatting
cargo fmt --check

# Run linter checks
cargo clippy --all-targets --all-features -- -D warnings

# Run release-binary NFR performance measurement & SLA gate
bash scripts/measure-perf.sh dist/perf-report.json
```

### Contributing & Governance
`xtrc` follows a strict development governance model:
- **Issue-Driven Development**: All work begins with a documented issue.
- **Requirements Before Design**: Clarify behavioral requirements before designing solutions.
- **Intent Over Implementation**: Focus on verifiable outcomes rather than prescriptive mechanics.
- **Branch-First & Pull Request Workflow**: Never push directly to `main`; always work on a dedicated branch (`feat/<issue>-*`, `fix/<issue>-*`, etc.) and submit changes via Pull Requests.

For full guidelines on contributing, code standards, and PR workflows, please review [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

This project is licensed under the terms of the [MIT License](LICENSE).
