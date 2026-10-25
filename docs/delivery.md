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
- **Observable Contract & Behavior**: GitHub Actions workflow `.github/workflows/delivery.yml` triggers on pushes to `main`, runs all verification criteria first, and upon success builds and publishes the `.tar.gz` archive, `.deb` package, `xtrc.rb` formula, and `xtrc.spdx.json` SBOM to workflow artifacts, an immutable `v<VERSION>` GitHub Release, and the rolling `main-head` pointer.
- **Acceptance Criteria**: Delivery job depends strictly on passing verification; publishes `.deb`, `.tar.gz`, `xtrc.rb`, `SHA256SUMS`, and `xtrc.spdx.json` upon merge or push to `main`.

### REQ-VER-POLICY: Unique Monotonic Delivery Version Policy
- **ID**: REQ-VER-POLICY
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Prevent multiple head-of-`main` deliveries from sharing the same version string with different binary or package content.
- **Observable Contract & Behavior**: Every build and head-of-`main` delivery derives a unique, monotonic version `<MAJOR>.<MINOR>.<PATCH>-r<COMMIT_COUNT>.g<SHORT_SHA>` (or `<MAJOR>.<MINOR>.<PATCH>` when built on an exact `v<MAJOR>.<MINOR>.<PATCH>` release tag) and stamps it across `xtrc --version`, `xtrc.spdx.json`, `.deb`, `.tar.gz`, `xtrc.rb`, and an immutable `v<VERSION>` GitHub Release.
- **Acceptance Criteria**: Two distinct commits never produce the same delivery version string; `xtrc --version`, `xtrc.spdx.json`, `xtrc.rb`, `.deb`, and `.tar.gz` all report the identical unique version.

## 3. Layer 6: Technical Specifications (SPEC-*)

### SPEC-CI-VERIFY: GitHub Actions Continuous Verification Pipeline Contract
- **ID**: SPEC-CI-VERIFY
- **Parent**: REQ-CI-VERIFY, REQ-CI-PERF, US-CICD-01
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
    4. Automated test & NFR benchmark suite: `cargo test --all-targets --all-features`.
    5. Self-traceability gate: `cargo run --locked -- check --strict`.
    6. Release-binary performance measurement & NFR SLA gate: `bash scripts/measure-perf.sh dist/perf-report.json`.

### SPEC-CD-DELIVERY: GitHub Actions Head-of-Main Packaging and SPDX SBOM Delivery Contract
- **ID**: SPEC-CD-DELIVERY
- **Parent**: REQ-PKG-DIST, REQ-PKG-SBOM, REQ-CD-MAIN, REQ-VER-POLICY, REQ-CI-PERF, US-CICD-01
- **Type**: TechnicalSpec
- **Component Target**: .github/workflows/delivery.yml, scripts/generate-sbom.sh, scripts/generate-homebrew-formula.sh, Cargo.toml
- **Verification Target**: tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Workflow Path: `.github/workflows/delivery.yml`.
  - Concurrency: `concurrency: { group: delivery-main, cancel-in-progress: false }` to serialize delivery runs and eliminate release creation races.
  - Triggers: `push` to `main` branch and `workflow_dispatch`.
  - Job 1 (`verify`): Runs all mandatory quality and performance gates (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, `cargo run --locked -- check --strict`, `bash scripts/measure-perf.sh dist/perf-report.json`).
  - Job 2 (`deliver`, requires `verify`):
    1. Checks out full Git history (`fetch-depth: 0`) and computes unique version `VERSION="$(bash scripts/compute-version.sh)"`.
    2. Compiles optimized release binary (`XTRC_DELIVERY_VERSION="${VERSION}" cargo build --release --locked`) and verifies `./target/release/xtrc --version`.
    3. Measures release-binary performance and verifies NFR SLA fulfillment via `bash scripts/measure-perf.sh dist/perf-report.json ./target/release/xtrc`.
    4. Generates SPDX 2.3 JSON SBOM (`dist/xtrc.spdx.json`) stamped with `${VERSION}` via `scripts/generate-sbom.sh`.
    5. Builds Debian package (`dist/xtrc_${VERSION}_amd64.deb`) via `cargo deb --no-build --deb-version "${VERSION}"` (embedding `/usr/share/doc/xtrc/xtrc.spdx.json`).
    6. Packages release tarball (`dist/xtrc-v${VERSION}-x86_64-unknown-linux-gnu.tar.gz`) containing `xtrc`, `README.md`, `LICENSE`, and `xtrc.spdx.json`.
    7. Generates Homebrew package formula (`dist/xtrc.rb`) via `scripts/generate-homebrew-formula.sh` pinned to release tag `v${VERSION}`, plus `dist/SHA256SUMS`.
    8. Uploads `dist/*` (including `dist/perf-report.json`) as GitHub Actions workflow artifacts, creates an immutable `v${VERSION}` GitHub Release, and refreshes the rolling `main-head` release pointer.

### SPEC-VER-POLICY: Deterministic Git-Derived Version Policy Contract
- **ID**: SPEC-VER-POLICY
- **Parent**: REQ-VER-POLICY, US-CICD-01
- **Type**: TechnicalSpec
- **Component Target**: scripts/compute-version.sh, build.rs, src/cli/mod.rs
- **Verification Target**: tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Base Version Source: `package.version` in `Cargo.toml` (`<MAJOR>.<MINOR>.<PATCH>`).
  - Exact Tagged Releases: When `HEAD` matches git tag `v<MAJOR>.<MINOR>.<PATCH>`, the version is `<MAJOR>.<MINOR>.<PATCH>`.
  - Head-of-Main & Branch Builds: Otherwise, the version is `<MAJOR>.<MINOR>.<PATCH>-r<COMMIT_COUNT>.g<SHORT_SHA>` (where `<COMMIT_COUNT>` is `git rev-list --count HEAD` and `<SHORT_SHA>` is `git rev-parse --short=7 HEAD`).
  - Compile-Time Injection: `build.rs` exports `XTRC_BUILD_VERSION` consumed by `clap` in `src/cli/mod.rs` so `xtrc --version` always matches the packaged delivery version.

