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
