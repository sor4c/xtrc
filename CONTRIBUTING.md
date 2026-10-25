# Contributing to xtrc

Thank you for your interest in contributing to **`xtrc`**!

`xtrc` is a fast, plain-text traceability CLI written in Rust. It tracks relationships across the complete software development lifecycle, linking vision, strategy, epics, and user stories directly to technical specs, architecture, source code, and tests. By storing all traceability metadata directly inside version control (Git), `xtrc` eliminates heavy database silos, synchronization lag, and vendor lock-in.

Because `xtrc` is built to bring engineering rigor and traceability to software projects, we hold our own development process to the highest standard. Whether you are contributing bug fixes, feature enhancements, documentation, or tooling improvements, please follow this guide.

---

## The Three Core Development Rules

All contributions—whether authored by human engineers or AI assistants—must strictly adhere to our three foundational development principles:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   The 3 Core Development Rules                         │
│                                                                        │
│  1. Issue-Driven Development    ──►  No code without context & problem │
│  2. Requirements Before Design ──►  Define "what & why" before "how"   │
│  3. Intent Over Implementation ──►  Focus on contracts & outcomes      │
└────────────────────────────────────────────────────────────────────────┘
```

### Rule 1: Issue-Driven Development
> **"Every line of code traces back to an issue."**

- **Context and Motivation First**: No pull request or code change may be created in isolation. Every change must originate from an existing, well-scoped GitHub issue that details the motivation, background, and definition of done.
- **Search Before Creating**: Check existing issues and discussions before opening a new issue to avoid duplicate efforts.
- **Use Structured Templates**: Issues must be submitted using one of our governed templates in `.github/ISSUE_TEMPLATE/` (`feature_proposal.yml`, `bug_report.yml`, or `task.yml`). Blank, unformatted issues are disabled.
- **Explicit Traceability Links**: Pull requests must explicitly reference their originating issue in their title and body (e.g., `Fixes #42` or `Relates to #88`).
- **No Unsolicited Drive-By PRs**: Speculative refactorings or ungrounded architectural changes submitted without prior issue alignment will be closed.

### Rule 2: Requirements Before Design
> **"Understand the problem space before committing to the solution space."**

- **Define Requirements First**: Before designing data structures, choosing dependencies, or writing implementation logic, clearly formulate the high-level functional and non-functional requirements and verifiable acceptance criteria.
- **Requirements Drive Design**: Technical design choices, architecture patterns, and CLI flag semantics must be directly derived from validated requirements, not from aesthetic preference or tool bias.
- **Synchronous Evolution**: If you uncover new edge cases or requirements during implementation, update the issue or specification document first, re-align on the requirements, and then adjust the code.

### Rule 3: Intent Over Implementation
> **"Specify the desired outcome and contract, not prescriptive mechanisms."**

- **Outcome-Oriented Specifications**: High-level requirements and issues must clearly specify *what* observable behavior the system must exhibit and *why*, rather than micromanaging private internal implementation details.
- **Preserve Engineering Autonomy**: Developers and AI pairing assistants should have the freedom to select the most idiomatic, performant, and maintainable Rust patterns that satisfy the stated intent.
- **Behavioral Testing**: Automated tests must verify external observable behaviors, interface contracts, and boundary conditions, rather than tightly coupling to transient private data structures or internal helper methods.

---

## Contributor Lifecycle

The path from an idea to merged code follows a transparent, six-stage lifecycle:

```
[ 1. Issue ] ──► [ 2. Requirements ] ──► [ 3. Branch ] ──► [ 4. Devcontainer ] ──► [ 5. Verify ] ──► [ 6. Pull Request ]
```

### 1. Issue Creation & Triage
- Open an issue using the appropriate template:
  - **Feature Proposal** (`.github/ISSUE_TEMPLATE/feature_proposal.yml`): For new features, CLI commands, or traceability capabilities.
  - **Bug Report** (`.github/ISSUE_TEMPLATE/bug_report.yml`): For deviations between expected intent and actual behavior.
  - **Technical Task** (`.github/ISSUE_TEMPLATE/task.yml`): For refactoring, dependencies, documentation, or CI/CD tooling.
- Discuss and refine the issue with maintainers during triage.

### 2. Requirements Definition & Acceptance Criteria
- Collaborate on the issue thread to ensure high-level functional requirements and acceptance criteria are unambiguous.
- Define what constitutes success (e.g., specific CLI exit codes, output formats, test scenarios).

### 3. Branching Strategy
- Create a new branch off the latest `main` branch.
- Use structured branch naming conventions:
  - `feat/<issue-id>-<short-description>` (e.g., `feat/42-graph-query`)
  - `fix/<issue-id>-<short-description>` (e.g., `fix/88-yaml-parse-panic`)
  - `chore/<issue-id>-<short-description>` (e.g., `chore/101-upgrade-clippy`)
  - `docs/<issue-id>-<short-description>` (e.g., `docs/15-contributing-guide`)

### 4. Development Environment (Dev Container)
- We recommend using our turnkey **Rust Dev Container** (`.devcontainer/devcontainer.json`).
  - Open the project in VS Code and select **"Reopen in Container"**.
  - The container provisions the required Rust toolchain (`cargo`, `rustfmt`, `clippy`, `rust-analyzer`), debugging utilities (`lldb`), GitHub CLI (`gh`), and pre-configured extension settings.
- If developing locally on your host system:
  - Install Rust 1.80+ (stable toolchain) via `rustup`.
  - Install required components: `rustup component add rustfmt clippy rust-analyzer`.

### 5. Local Verification & Quality Gates
Before committing or submitting a PR, you MUST run and pass all local verification checks:
```bash
# 1. Format check
cargo fmt --check

# 2. Strict linter check (zero warnings permitted)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Test suite
cargo test --all-targets --all-features
```

### 6. Pull Request Submission
- Create a pull request targeting `main`.
- Fill out the PR description:
  - Reference the originating issue (e.g., `Closes #42`).
  - Summarize the intent of the change and how requirements are fulfilled.
  - List the verification steps taken and check off (`- [x]`) all fulfilled acceptance criteria.
- Ensure all CI workflow checks pass. Address reviewer feedback constructively.

### 7. Version & Delivery Policy
- **Base Version (`MAJOR.MINOR.PATCH`)**: Managed in `Cargo.toml` per Semantic Versioning 2.0.0.
- **Unique Delivery Version (`<MAJOR>.<MINOR>.<PATCH>-r<COMMIT_COUNT>.g<SHORT_SHA>`)**: Computed automatically by `scripts/compute-version.sh` and `build.rs` so that every commit and head-of-`main` delivery has a unique, monotonically increasing version across `xtrc --version`, `xtrc.spdx.json`, `.deb`, `.tar.gz`, `xtrc.rb`, and its immutable `v<VERSION>` GitHub Release.
- Never overwrite an existing `v<VERSION>` release tag with different content.

---

## Rust Quality Standards & Conventions

To maintain high software reliability, maintainability, and safety across `xtrc`, all Rust contributions must conform to the following standards:

### 1. Idiomatic Rust & Code Hygiene
- Target the **Rust 2021 / 2024 Edition**.
- Follow official Rust API Guidelines ([`rust-lang.github.io/api-guidelines`](https://rust-lang.github.io/api-guidelines/)).
- All code must be strictly formatted with `cargo fmt`.
- Code must pass `cargo clippy --all-targets --all-features -- -D warnings` with zero warnings. Suppressing warnings with `#[allow(...)]` or `#![allow(...)]` is strictly prohibited unless approved by maintainers with documented justification.
- Prefer borrowed references (`&str`, `&[T]`, `&Path`) over owned allocations (`String`, `Vec<T>`, `PathBuf`) in function arguments where ownership is not needed.

### 2. Strict Error Handling (No Naked Unwrap)
- **Strictly No Naked `.unwrap()` or `.expect()` in Production Code**:
  - Panics in a command-line tool degrade user trust and corrupt terminal sessions.
  - Use pattern matching, `?` operator propagation, or `unwrap_or_else` defaults.
  - In unit tests (`#[cfg(test)]`), `.unwrap()` and `.expect()` are permitted where test clarity is enhanced.
  - If handling an invariant where an `Option` or `Result` is mathematically guaranteed to be populated, use `.expect("INVARIANT: <reason>")` with an explanatory comment proving why it cannot fail.
- **Error Types**:
  - **Internal / Domain Libraries**: Define custom, structured, strongly-typed error enums using [`thiserror`](https://docs.rs/thiserror).
  - **CLI Binary / Application Layer**: Use [`anyhow::Result`](https://docs.rs/anyhow) for rich context propagation, annotating failures with `.context("...")` or `.with_context(|| ...)`.

### 3. CLI Design & Unix Philosophy
`xtrc` follows standard Unix CLI design principles:
- **Standard Output (`stdout`)**: Exclusively reserved for intended program output (e.g., formatted tables, graph trees, plain-text lines, or `--json` structured data).
- **Standard Error (`stderr`)**: Reserved for progress spinners, diagnostic information, operational logs, warnings, and error messages.
- **Exit Codes**:
  - `0`: Success (including clean alignment checks where no errors are found).
  - `1`: Traceability violations, verification failures, or unfulfilled requirements.
  - `2`: CLI argument or syntax errors.
  - `>2`: Fatal system or I/O errors.
- Ensure commands support non-interactive pipes (e.g., `xtrc check | grep FAIL`).

### 4. Documentation & Rustdoc
- All public items (`pub struct`, `pub enum`, `pub trait`, `pub fn`, `pub mod`) must be documented using `///` Markdown docstrings.
- Docstrings should clearly state the intent, parameters, return values, and include `# Examples`, `# Errors`, and `# Panics` sections where relevant.
- Traceability Annotations: Annotate code items with their corresponding requirement identifiers (e.g., `/// Traceability: [REQ-CLI-01]`).

### 5. Comprehensive Testing
- **Co-Located Unit Tests**: Place unit tests in a `mod tests` block within the same file as the tested functionality to verify internal component behavior.
- **Integration Tests**: Place end-to-end command and workflow tests in the `tests/` directory.
- **Boundary & Negative Testing**: Test empty inputs, invalid formats, missing files, broken links, and permission errors in addition to the happy path.
- **Behavioral Integrity**: Never fabricate test data or alter test assertions to artificially mask bugs.

---

## AI Assistant Pairing (GitHub Copilot & Antigravity)

We encourage contributors to use AI coding assistants such as GitHub Copilot and Google Antigravity / Gemini agents. When doing so:
- Ensure the assistant is aware of our instructions:
  - `.github/copilot-instructions.md` for GitHub Copilot.
  - `AGENTS.md` and `.gemini/rules/repository-governance.md` for autonomous agents.
- Remember: **The human contributor is 100% accountable** for any code generated by an AI assistant.
- Verify that AI-generated code strictly complies with the 3 Core Development Rules, does not introduce naked `unwrap()` calls, and passes all quality gates.

---

## Code of Conduct

We are dedicated to providing a welcoming, inclusive, and harassment-free environment for everyone. Please be respectful, constructive, and collaborative in all issue discussions, code reviews, and communications.
