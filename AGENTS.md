# Autonomous Agent Instructions & Repository Governance for xtrc

This file establishes the mandatory operational rules, quality gates, and engineering standards for all autonomous software engineering agents (including Google Antigravity, Gemini, Teamwork, Claude Code, Devin, and Codex) operating within the `xtrc` repository.

---

## 1. Persona & Operational Scope

You are an autonomous software engineering agent contributing to **`xtrc`**.

`xtrc` is a fast, plain-text traceability CLI written in Rust. It tracks relationships across the complete software development lifecycle, linking vision, strategy, epics, and user stories directly to technical specs, source code, and automated tests.

`xtrc` operates purely inside version control (Git), eliminating heavyweight databases and web silos so developers can verify project alignment instantly right from their terminal.

---

## 2. Non-Negotiable Governance Rules

All autonomous agents MUST adhere strictly to the **Three Core Development Rules**:

### Rule 1: Issue-Driven Development
- Every functional code change, bug fix, or refactor must be grounded in an existing GitHub Issue or requirement ID.
- Before generating code or modifying existing files, verify that an issue exists describing the motivation, requirements, and scope.
- Reference the issue number or requirement ID in commit messages and pull request descriptions (e.g., `Fixes #10` or `Relates to #15`).
- Unsolicited, ungrounded code modifications are strictly prohibited.

### Rule 2: Requirements Before Design
- Requirements define the problem space (*what* the system must do and *why*); design defines the solution space (*how*).
- Thoroughly inspect, clarify, and document high-level functional requirements and verifiable acceptance criteria before proposing architecture, modules, or implementations.
- Design decisions must be directly derived from validated requirements.

### Rule 3: Intent Over Implementation
- Focus on the intended outcome, user value, and behavioral contracts rather than prescriptive mechanisms.
- Choose clean, idiomatic, and minimal Rust solutions that directly satisfy the stated intent without introducing unnecessary abstraction layers.
- Write tests that verify observable behavioral outcomes, not private internal implementation details.

### Branch-First & Pull Request Mandate (No Direct Pushes to `main`)
- **Never push directly to `main`**: Committing or pushing directly to the `main` branch (`git push origin main`) is strictly prohibited.
- **Always work on a dedicated branch**: Every task, feature, bug fix, or documentation update must be developed on an isolated branch named `<type>/<issue-id>-<short-description>` (e.g., `feat/15-nfr-ci-perf-gate`, `fix/20-parser-edge-case`).
- **Always use Pull Requests**: Push your feature/task branch to the remote repository and open a GitHub Pull Request targeting `main` that links to the originating issue and passes all CI/CD verification and performance gates.

### Post-Merge CI and Issue Closure (Mandatory)
- PR checks validate the PR head, not necessarily the merge commit. After merging, identify the exact merge commit SHA and wait for every required workflow triggered on `main` for that SHA to complete successfully before declaring the task complete.
- Do not put auto-closing keywords such as `Closes #N`, `Fixes #N`, or `Resolves #N` in a PR description while post-merge checks are pending; GitHub may close the issue as soon as the PR merges. Use a non-closing reference such as `Relates to #N`, then close the issue explicitly only after post-merge CI succeeds.
- If any required post-merge workflow fails, keep the issue open or reopen it immediately. Diagnose the failure, fix it on a follow-up branch and PR, and verify all required workflows on the follow-up merge commit before closing the issue.
- If the post-merge workflow has not started, is still running, or its result cannot be verified, leave the issue open and report verification as pending. Do not equate green PR checks with successful post-merge verification.
- For requests to implement or deliver a GitHub issue end to end, use the [issue-delivery workflow](.github/skills/issue-delivery/SKILL.md). It requires a fresh-context review with the `issue-reviewer` agent and covers version/SBOM/docs impact, PR and merge gates, exact-SHA post-merge verification, issue closure, and returning the checkout to `main`.

### GitHub Advanced Security Findings (Mandatory)
- Never merge a PR while any finding or review thread from `github-advanced-security[bot]` is unresolved. Fix the issue and resolve its conversation; never dismiss or bypass a finding just to make a PR mergeable.
- The `Trusted Advanced Security Review Findings` PR check from the trusted `pull_request_target` workflow and GitHub's required-conversation-resolution branch protection are mandatory merge gates. If either is missing, failing, skipped unexpectedly, or unverifiable, stop and do not merge. Do not use PR review-comment-triggered workflows for this gate because they can execute PR-controlled workflow code. After fixing and resolving a finding without a new push, dispatch the trusted workflow for the current PR number and head SHA: `gh workflow run security-review-gate.yml --repo sor4c/xtrc --ref main -f pr_number=<number> -f head_sha=<head-sha>`.
- For the one-time PR that installs the trusted workflow, require conversation resolution, fix and resolve all bot findings, and independently run the trusted checker against the current PR head before merging; immediately make the new check required on `main`. This bootstrap does not permit unresolved findings or a merge before direct verification.
- Treat bot findings as security reports requiring investigation even when CI is green. Confirm the fix in code and tests, and record the resolution in the PR.

---

## 3. Mandatory Quality Gates & Verification

Before reporting completion or submitting a Pull Request, agents MUST execute and verify that all of the following quality gates pass with exit code `0`:

1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Linting & Diagnostics**:
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
3. **Automated Test & NFR Benchmark Suite**:
   ```bash
   cargo test --all-targets --all-features
   ```
4. **Strict Bidirectional Traceability**:
   ```bash
   cargo run --locked -- check --strict
   ```
5. **Performance & NFR SLA Measurement**:
   ```bash
   bash scripts/measure-perf.sh
   ```

### Strict Integrity Mandate (Zero Tolerance for Bypassing Checks)
Agents are strictly prohibited from taking shortcuts:
- **No hardcoding**: Do not hardcode test results, expected outputs, or verification strings to make tests pass.
- **No dummy or facade implementations**: All implementations must maintain real state and execute genuine logic.
- **No silencing linter warnings**: Do not add `#![allow(...)]` or `#[allow(...)]` attributes to bypass compiler or Clippy checks without explicit user consent.
- **No weakening tests**: Never comment out failing tests, delete assertions, or modify test expectations to match buggy behavior.
- **No fabricated verification**: Never claim checks passed without executing the commands and inspecting their verbatim output.

---

## 4. Workspace Hygiene

Agents must keep the repository clean and professional:
- **No clutter in tracked paths**: Do not leave scratch files, debug logs, temporary scripts, or ad-hoc binaries in tracked repository directories.
- **Scratch directory**: If scratch files or intermediate logs are necessary during execution, write them to temporary directories or paths ignored by `.gitignore`.
- **Metadata directory isolation**: The `.agents/teamwork/` directory is reserved exclusively for agent collaboration metadata (plans, progress, handoffs). **NEVER** place source code, tests, or application assets inside `.agents/teamwork/`.
- **Pre-handoff cleanup**: Verify that `git status` reports only intended modifications before finalizing any task.

---

## 5. Rust Standards & Coding Guidelines

- **Rust Toolchain**: Target modern Rust (Edition 2021 / 2024).
- **Error Handling**:
  - Strictly **no naked `.unwrap()` or `.expect()`** in production codepaths.
  - In internal modules and libraries, define domain-specific error enums using [`thiserror`](https://docs.rs/thiserror).
  - In CLI entry points and command handlers, use [`anyhow`](https://docs.rs/anyhow) with `.context("...")` to provide user-actionable error messages.
- **CLI Behavior**:
  - Print structured, machine-parseable data to `stdout`.
  - Print logs, progress spinners, diagnostics, and errors to `stderr`.
  - Use standard Unix exit codes (`0` for success, non-zero for failure).
- **Documentation**:
  - All public modules, structs, enums, traits, and functions must be documented with `///` rustdoc docstrings.
- **Test Co-Location**:
  - Co-locate unit tests within `mod tests` in the same file as the implementation.
  - Cover edge cases, error conditions, and boundary values.
- **Traceability Integration**:
  - Annotate source code and tests with traceability markers (e.g., `// @trace REQ-...`) linking directly back to specifications and issues.
