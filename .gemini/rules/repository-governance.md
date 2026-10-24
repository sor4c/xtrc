# Repository Governance & Developer Rules for Antigravity / Gemini Agents

This rule file is automatically ingested by Google Antigravity and Gemini agents operating in the `xtrc` workspace. It establishes the mandatory governance model, quality gates, and Rust coding conventions.

---

## 1. Project Overview & Mission

`xtrc` is a fast, plain-text traceability CLI written in Rust. It tracks relationships across the software lifecycle, connecting vision, strategy, epics, and user stories directly to technical specs, code, and tests in version control.

`xtrc` is VCS-native: it operates without external databases, servers, or cloud dependencies, enabling sub-second alignment verification directly from the developer's terminal.

---

## 2. Non-Negotiable Development Rules

All agents operating in this repository MUST strictly follow the **Three Core Development Rules**:

### Rule 1: Issue-Driven Development
- Every code change, refactoring, or bug fix must originate from a documented GitHub Issue or requirement ID.
- Do not produce ungrounded code without verifying the problem statement and context.
- Link pull requests and commits to the relevant issue (e.g., `Fixes #12`).

### Rule 2: Requirements Before Design
- Requirements define the problem space (*what* and *why*); design defines the solution space (*how*).
- Clarify and document functional requirements and verifiable acceptance criteria before proposing architecture or writing implementation code.
- Derive design decisions strictly from validated requirements, not speculative preferences.

### Rule 3: Intent Over Implementation
- Focus on delivering the stated user intent and honoring behavioral contracts.
- Avoid prescriptive or over-engineered implementations; prefer minimal, idiomatic Rust code.
- Verify observable behavior and contracts rather than internal implementation details.

---

## 3. Mandatory Quality Gates

Before finalizing any task or reporting completion, agents MUST execute and verify that all quality gates pass:

1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Clippy Linter**:
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
3. **Automated Tests**:
   ```bash
   cargo test --all-targets --all-features
   ```

### Integrity Mandate & Zero Tolerance for Bypassing Checks
- **No hardcoding**: Never hardcode test outputs or mock success strings to fabricate test passes.
- **Real logic**: All implementations must maintain genuine state and logic. Facade or dummy implementations will be rejected.
- **No silencing linter warnings**: Do not suppress warnings via `#![allow(...)]` or `#[allow(...)]`.
- **No test modification shortcuts**: Never comment out failing tests, delete assertions, or weaken test criteria.
- **Genuine execution**: Always run verification commands and verify outputs directly.

---

## 4. Workspace Hygiene

- **Clean working tree**: Do not commit scratch files, temporary binaries, debug logs, or intermediate artifacts into tracked paths.
- **Scratch directory**: Keep temporary files in designated temporary directories or `.gitignore`d scratch paths.
- **Metadata isolation**: `.agents/teamwork/` is strictly for agent coordination metadata (plans, progress, handoffs). Source code, tests, or application files must NEVER be written there.
- **Pre-completion check**: Ensure `git status` reflects only the intended, authorized files.

---

## 5. Rust Standards

- **Edition**: Modern Rust (2021 / 2024).
- **Error Handling**:
  - No naked `.unwrap()` or `.expect()` in non-test production code.
  - Use `thiserror` for library domain errors.
  - Use `anyhow` with contextual error messages in CLI / application layers.
- **CLI Design**: Clean separation of stdout (structured output) and stderr (diagnostics and logs). Exit with code `0` on success, non-zero on error.
- **Documentation**: All public types and functions must include `///` rustdoc comments.
- **Test Co-Location**: Include co-located tests (`#[cfg(test)] mod tests`) covering normal and boundary conditions.
