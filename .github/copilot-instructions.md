# GitHub Copilot Instructions for xtrc

These instructions guide GitHub Copilot (Inline Suggestions, Copilot Chat, and Copilot Edits) when working within the `xtrc` codebase.

---

## 1. Project Mission & Identity

**`xtrc`** is a fast, plain-text traceability CLI written in Rust. It tracks relationships across the entire development lifecycle, linking vision, strategy, epics, and user stories directly to technical specs, source code, and tests.

Key design principles of `xtrc`:
- **VCS-Native**: All traceability metadata, specifications, and links live directly in version control (Git). There are no external databases, servers, or cloud dependencies.
- **Plain-Text Driven**: Metadata is human-readable and plain-text (Markdown, YAML, TOML).
- **Sub-Second Performance**: CLI commands (`xtrc check`, `xtrc verify`, `xtrc matrix`, `xtrc tree`) operate with sub-second latency directly in the developer's terminal.
- **Bidirectional Alignment**: Ensures both forward traceability (requirements are fully implemented and tested) and backward traceability (every piece of code is justified by a stated requirement).

---

## 2. Core Development Rules

When assisting developers in this repository, GitHub Copilot MUST respect and enforce the **Three Core Development Rules**:

### Rule 1: Issue-Driven Development
- **No ungrounded code**: Never generate code, features, or architectural modifications in isolation.
- **Context requirement**: When asked to implement a feature or fix a bug, confirm or reference the relevant GitHub issue or requirement ID (e.g., `Fixes #12`, `REQ-TRACE-01`).
- **Traceability in commits & PRs**: Encourage linking every PR to a structured issue created via `.github/ISSUE_TEMPLATE/`.

### Rule 2: Requirements Before Design
- **Clarify the problem first**: Requirements describe the *what* and *why*; design describes the *how*.
- **Acceptance criteria**: Before drafting code or complex data structures, ensure high-level functional and non-functional requirements and acceptance criteria are documented.
- **Avoid premature architecture**: Derive technical design decisions strictly from validated requirements, not aesthetic preference or speculative future needs.

### Rule 3: Intent Over Implementation
- **Focus on outcomes**: Code suggestions must focus on fulfilling the stated user intent and behavioral contracts.
- **Preserve simplicity**: Do not introduce unnecessary abstraction layers, convoluted inheritance hierarchies, or excessive generic boilerplate when simple, idiomatic Rust achieves the intent.
- **Behavioral testing**: Propose tests that verify observable behavior and requirement contracts, not private internal implementation details.

### Branch-First & Pull Request Workflow (No Direct Pushes to `main`)
- **Never push directly to `main`**: Never commit or push directly to the `main` branch.
- **Always work on a branch**: Develop every change on a dedicated issue branch (`feat/<issue-id>-<slug>`, `fix/<issue-id>-<slug>`, `chore/<issue-id>-<slug>`, `docs/<issue-id>-<slug>`).
- **Always open Pull Requests**: Submit all changes via a GitHub Pull Request targeting `main` and ensure all CI verification, traceability, and NFR performance gates pass.

### Post-Merge CI and Issue Closure
- PR checks are not a substitute for checks on the merge commit. After merging, identify the exact merge commit SHA and wait until every required workflow triggered on `main` for that SHA has completed successfully.
- Do not use auto-closing PR phrases (`Closes #N`, `Fixes #N`, or `Resolves #N`) while post-merge checks are pending, because GitHub may close the issue immediately upon merge. Use `Relates to #N` and close the issue explicitly only after the post-merge checks pass.
- If a required post-merge workflow fails, keep or reopen the issue, investigate the failure, and submit the fix through a follow-up branch and PR. Verify the resulting merge commit's required workflows before closing the issue.
- If the workflow is pending, missing, or otherwise unverifiable, leave the issue open and describe the task as awaiting post-merge verification; do not report completion based only on PR checks.
- When asked to implement or deliver a GitHub issue end to end, follow the [issue-delivery skill](skills/issue-delivery/SKILL.md), invoke the `issue-reviewer` custom agent for a fresh-context read-only review, and use the `/implement-issue` prompt for the issue number when convenient.

### GitHub Advanced Security Findings
- Never merge while a finding or review thread from `github-advanced-security[bot]` is unresolved. Investigate and fix the finding, then resolve the conversation; do not dismiss or bypass it to unblock merging.
- Require the `Trusted Advanced Security Review Findings` check from the trusted `pull_request_target` workflow and GitHub's required-conversation-resolution branch protection. If either gate is absent, failing, or unverifiable, stop without merging. Do not use PR review-comment-triggered workflows for this gate because they can execute PR-controlled workflow code. After fixing and resolving a finding without a new push, post an issue comment on the PR containing `/security-check <current-head-sha>` to trigger a trusted recheck from the default branch.
- For the one-time PR that installs the trusted workflow, require conversation resolution, fix and resolve every bot finding, independently run the trusted checker on the current PR head, and make the new check required on `main` immediately after merge.

---

## 3. Rust Standards & Idiomatic Conventions

All Rust code suggested by Copilot must adhere to the following standards:

### Idiomatic Error Handling
- **No naked `unwrap()` or `expect()`**: Never suggest `.unwrap()` or `.expect()` in non-test production code.
- **Library modules**: Use custom, strongly-typed errors with [`thiserror`](https://docs.rs/thiserror).
- **CLI / Application layers**: Use [`anyhow`](https://docs.rs/anyhow) for context-rich error reporting with `.context("...")` or `with_context(...)`.
- **Invariants**: When handling optionals or results where an invariant guarantees success, handle the error gracefully or document the invariant rigorously.

### Code Style & Clippy Cleanliness
- **Rust Edition**: Use modern Rust (Edition 2021 or 2024).
- **Formatting**: Strictly follow `rustfmt` conventions.
- **Clippy Clean**: All code must pass `cargo clippy --all-targets --all-features -- -D warnings`. Do not propose code that triggers clippy warnings or uses `#[allow(...)]` to suppress legitimate lints.
- **Borrowing & Allocations**: Favor borrowed types (`&str`, `&[T]`, `&Path`) in function arguments over owned types (`String`, `Vec<T>`, `PathBuf`) unless ownership transfer is required.

### CLI & Unix Philosophy
- **Standard I/O separation**:
  - `stdout`: Machine-readable outputs (JSON, plain text, tabular matrices).
  - `stderr`: Progress indicators, diagnostic logs, warnings, and error messages.
- **Exit codes**: Return `0` on success, non-zero on failure.

### Documentation & Rustdoc
- Document all public items (`pub struct`, `pub enum`, `pub trait`, `pub fn`) using `///` Markdown docstrings.
- Include practical code examples (`# Examples`) in docstrings where appropriate.

### Test-Driven Co-Location
- Every function, parser, or domain model should be accompanied by co-located unit tests in a `#[cfg(test)] mod tests { ... }` module.
- Always include tests for edge cases: empty input, malformed syntax, boundary conditions, and broken traceability links.
