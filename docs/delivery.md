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
- **Observable Contract & Behavior**: Pushing commits to any branch runs all formatting, linting, test, traceability, and native platform build checks; pushing verified changes to `main` automatically builds Linux, macOS, and Windows release archives and supported Linux `.deb` packages with an SPDX JSON SBOM.
- **Acceptance Criteria**: CI fails on any verification or supported-target build error; head-of-main delivery publishes all declared platform artifacts and the SPDX JSON SBOM only after all required gates pass.

## 2. Layer 5: Requirements (REQ-*)

### REQ-PKG-DIST: Installable Package Deliverables
- **ID**: REQ-PKG-DIST
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Enable users and automation environments to install the `xtrc` CLI effortlessly using standard system and Rust package managers.
- **Observable Contract & Behavior**: Provides prebuilt binaries for Linux x86_64 (`x86_64-unknown-linux-gnu`) and ARM64 (`aarch64-unknown-linux-gnu`), macOS Intel (`x86_64-apple-darwin`) and Apple Silicon (`aarch64-apple-darwin`), and Windows x86_64 (`x86_64-pc-windows-msvc`). Release assets use `.tar.gz` archives for Linux/macOS, `.zip` for Windows, plus a Windows `.tar.gz` archive for cargo-binstall; Linux deliveries include Debian packages for amd64 and arm64. Every archive contains the executable, README, license, and SPDX SBOM. The Homebrew formula selects the correct native macOS archive.
- **Acceptance Criteria**: `Cargo.toml` defines valid `[package.metadata.deb]` and `[package.metadata.binstall]` sections; delivery emits the five declared target archives, two supported Linux `.deb` packages, a native macOS `xtrc.rb` formula, and SHA-256 checksums.

### REQ-PKG-SBOM: SPDX JSON Software Bill of Materials in Every Delivery
- **ID**: REQ-PKG-SBOM
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Provide supply-chain transparency and compliance auditability by including a machine-readable Software Bill of Materials with every delivery.
- **Observable Contract & Behavior**: Generates an SPDX 2.3 JSON Software Bill of Materials (`xtrc.spdx.json`) capturing exact crate dependencies and licenses for each packaged delivery.
- **Acceptance Criteria**: Every platform archive and release includes the same valid, version-matched SPDX JSON file (`xtrc.spdx.json`).

### REQ-CI-VERIFY: Automated Branch and Pull Request Verification Workflow
- **ID**: REQ-CI-VERIFY
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Enforce repository quality gates automatically on every remote commit across `main` and feature branches.
- **Observable Contract & Behavior**: GitHub Actions workflow `.github/workflows/ci.yml` triggers on pushes to any branch, version tags, and pull requests. It runs `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, `cargo run -- check --strict`, and native release build/smoke checks for all declared targets.
- **Acceptance Criteria**: Workflow executes automatically on all branch pushes, version tags, and pull requests and fails if any quality gate or supported native target check returns a non-zero exit code.

### REQ-CD-MAIN: Automated Head-of-Main Delivery Workflow
- **ID**: REQ-CD-MAIN
- **Parent**: US-CICD-01
- **Type**: Functional Requirement
- **Intent & Value**: Deliver up-to-date, verified packages and SBOM artifacts automatically whenever changes land on the `main` branch.
- **Observable Contract & Behavior**: GitHub Actions workflow `.github/workflows/delivery.yml` triggers on pushes to `main`, version tags, and manual dispatch. After verification, it computes one delivery version and SBOM, builds all supported target archives in parallel on native runners, and publishes Linux `.deb` packages, a macOS Homebrew formula, checksums, and performance report as workflow artifacts and releases.
- **Acceptance Criteria**: Delivery artifacts are published only after verification and all five native builds succeed; releases contain the two Linux `.deb` packages, five platform archives (plus the Windows cargo-binstall tarball), `xtrc.rb`, `SHA256SUMS`, `perf-report.json`, and `xtrc.spdx.json`. Tagged releases remain immutable and verified pushes to `main` refresh `main-head`.

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
  - Triggers: `push` on all branches (`"**"`), version tags (`v*`), and `pull_request`.
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
- **Component Target**: .github/workflows/ci.yml, .github/workflows/delivery.yml, scripts/generate-sbom.sh, scripts/generate-homebrew-formula.sh, Cargo.toml, README.md
- **Verification Target**: tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Workflow Path: `.github/workflows/delivery.yml`.
  - Concurrency: `concurrency: { group: delivery-main, cancel-in-progress: false }` to serialize delivery runs and eliminate release creation races.
  - Triggers: successful completion of the `Verify` workflow and `workflow_dispatch`. CI runs on all branches, pull requests, and `v*` tags; delivery ignores unsuccessful runs and does not repeat the full quality gates. Manual dispatch builds and uploads artifacts only; publication is reserved for successful verification runs on `main` or exact version tags.
  - Job 1 (`prepare`, requires successful `Verify`): Checks out the verified commit with full history, computes the unique delivery version once, generates and validates the version-stamped SPDX 2.3 SBOM, and uploads the SBOM for all target builds.
  - Job 2 (`build`, requires `prepare`): Runs five parallel native builds on Ubuntu x86_64/ARM64, macOS Intel/Apple Silicon, and Windows x86_64 runners. Each build verifies its Rust host target, compiles with `XTRC_DELIVERY_VERSION="${VERSION}"`, checks `xtrc --version`, and runs `xtrc --root examples/demo check --strict`. Linux jobs build `.deb` packages for amd64 and arm64; x86_64 Linux measures the optimized release binary against the performance SLA. Each target archive includes the native executable, README, LICENSE, and shared SBOM. Windows emits ZIP and tar.gz archives.
  - Job 3 (`publish`, requires `prepare` and all target builds): Downloads all target outputs, generates a Homebrew formula with separate Intel and Apple Silicon URLs and SHA-256 hashes, generates `SHA256SUMS`, uploads all artifacts, then creates the immutable `v${VERSION}` GitHub Release and refreshes the rolling `main-head` release pointer for `main`.
  - The native target matrix is also built and smoke-tested in parallel with Linux quality gates on every CI push and pull request. CI does not collect usage telemetry. macOS binaries are unsigned unless signing/notarization credentials are configured separately.

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
