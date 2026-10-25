---
document_type: specification
id: SPEC-DELIV-001
title: Continuous Verification, Packaging, and SPDX SBOM Delivery Specification
version: 1.0.0
status: approved
governance_tracking:
  rule_1_issue: "#7"
  rule_2: "Requirements Before Design"
  rule_3: "Intent Over Implementation"
traceability_layers:
  - User Stories
  - Requirements
  - Technical Specs
schema_version: "1.0.0"
---

# Continuous Verification, Packaging, and SPDX SBOM Delivery Specification

## 1. Layer 4: User Stories (US-*)

### US-CICD-01: Automated Verification, Packaging, and SBOM Delivery
- **ID**: US-CICD-01
- **Parent**: EPIC-VERIFY
- **Type**: User Story
- **Intent & Value**: As a software engineer and release consumer, I want every commit verified automatically in CI and every verified change on `main` packaged into easily installable artifacts accompanied by an SPDX JSON SBOM.
- **Observable Contract & Behavior**: Pushing commits to any branch runs all formatting, linting, test, and traceability verification checks; pushing verified changes to `main` automatically builds installable packages (`.deb`, `.tar.gz`, Cargo metadata) and an SPDX JSON SBOM.
- **Acceptance Criteria**: CI fails on any verification error; head-of-main delivery publishes binary packages and SPDX JSON SBOM only after all verification gates pass.

## 2. Layer 5: Requirements (REQ-*)

### REQ-PKG-DIST: Installable Package Deliverables
- **ID**: REQ-PKG-DIST
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Enable users and automation environments to install the `xtrc` CLI effortlessly using standard system and Rust package managers.
- **Observable Contract & Behavior**: Provides standard Rust package metadata (`Cargo.toml`, `cargo-binstall`) and produces Homebrew formulas (`xtrc.rb`), Debian packages (`.deb`), and prebuilt binary archives (`.tar.gz`) installing the `xtrc` binary and SBOM documentation to standard system paths.
- **Acceptance Criteria**: `Cargo.toml` defines valid `[package.metadata.deb]` and `[package.metadata.binstall]` sections; delivery pipeline emits `xtrc.rb`, `.deb`, and `.tar.gz` artifacts alongside SHA-256 checksums.

### REQ-PKG-SBOM: SPDX JSON Software Bill of Materials in Every Delivery
- **ID**: REQ-PKG-SBOM
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide supply-chain transparency and compliance auditability by including a machine-readable Software Bill of Materials with every delivery.
- **Observable Contract & Behavior**: Generates an SPDX 2.3 JSON Software Bill of Materials (`xtrc.spdx.json`) capturing exact crate dependencies and licenses for each packaged delivery.
- **Acceptance Criteria**: Every delivery bundle and release includes a valid SPDX JSON file (`xtrc.spdx.json`).

### REQ-CI-VERIFY: Automated Branch and Pull Request Verification Workflow
- **ID**: REQ-CI-VERIFY
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Enforce repository quality gates automatically on every remote commit across `main` and feature branches.
- **Observable Contract & Behavior**: GitHub Actions workflow `.github/workflows/ci.yml` triggers on pushes to any branch and on pull requests, executing `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, and `cargo run -- check --strict`.
- **Acceptance Criteria**: Workflow executes automatically on all branch pushes and pull requests and fails if any local verification check returns a non-zero exit code.

### REQ-CD-MAIN: Automated Head-of-Main Delivery Workflow
- **ID**: REQ-CD-MAIN
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Deliver up-to-date, verified packages and SBOM artifacts automatically whenever changes land on the `main` branch.
- **Observable Contract & Behavior**: GitHub Actions workflow `.github/workflows/delivery.yml` triggers on pushes to `main`, runs all verification criteria first, and upon success builds and publishes the `.tar.gz` archive, `.deb` package, and `xtrc.spdx.json` SBOM to workflow artifacts and the rolling `main-head` GitHub Release.
- **Acceptance Criteria**: Delivery job depends strictly on passing verification; publishes `.deb`, `.tar.gz`, `SHA256SUMS`, and `xtrc.spdx.json` upon merge or push to `main`.

## 3. Layer 6: Technical Specifications (SPEC-*)

### SPEC-CI-VERIFY: GitHub Actions Continuous Verification Pipeline Contract
- **ID**: SPEC-CI-VERIFY
- **Parent**: REQ-CI-VERIFY, US-CICD-01
- **Type**: TechnicalSpec
- **Component Target**: .github/workflows/ci.yml
- **Verification Target**: tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Workflow Path: `.github/workflows/ci.yml`.
  - Triggers: `push` on all branches (`"**"`) and `pull_request`.
  - Steps:
    1. Checkout repository and provision stable Rust toolchain with `rustfmt` and `clippy`.
    2. Formatting gate: `cargo fmt --check`.
    3. Linter gate: `cargo clippy --all-targets --all-features -- -D warnings`.
    4. Automated test suite: `cargo test --all-targets --all-features`.
    5. Self-traceability gate: `cargo run -- check --strict`.

### SPEC-CD-DELIVERY: GitHub Actions Head-of-Main Packaging and SPDX SBOM Delivery Contract
- **ID**: SPEC-CD-DELIVERY
- **Parent**: REQ-PKG-DIST, REQ-PKG-SBOM, REQ-CD-MAIN, US-CICD-01
- **Type**: TechnicalSpec
- **Component Target**: .github/workflows/delivery.yml, scripts/generate-sbom.sh, scripts/generate-homebrew-formula.sh, Cargo.toml
- **Verification Target**: tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Workflow Path: `.github/workflows/delivery.yml`.
  - Triggers: `push` to `main` branch and `workflow_dispatch`.
  - Job 1 (`verify`): Runs all mandatory quality gates (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, `cargo run -- check --strict`).
  - Job 2 (`deliver`, requires `verify`):
    1. Compiles optimized release binary (`cargo build --release --locked`).
    2. Generates SPDX 2.3 JSON SBOM (`dist/xtrc.spdx.json`) via `scripts/generate-sbom.sh`.
    3. Builds Debian package (`dist/xtrc_<version>_amd64.deb`) via `cargo-deb` (embedding `/usr/share/doc/xtrc/xtrc.spdx.json`).
    4. Packages release tarball (`dist/xtrc-v<version>-x86_64-unknown-linux-gnu.tar.gz`) containing `xtrc`, `README.md`, `LICENSE`, and `xtrc.spdx.json`.
    5. Generates Homebrew package formula (`dist/xtrc.rb`) via `scripts/generate-homebrew-formula.sh` with computed tarball SHA-256 checksum, plus `dist/SHA256SUMS`.
    6. Uploads `dist/*` as GitHub Actions workflow artifacts and publishes to the rolling `main-head` GitHub Release.

