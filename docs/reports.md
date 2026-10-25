---
document_type: specification
id: SPEC-REP-001
title: Traceability Compliance Reporting and Multi-Format Export Specification
version: 1.0.0
status: approved
governance_tracking:
  rule_1_issue: "#19"
  rule_2: "Requirements Before Design"
  rule_3: "Intent Over Implementation"
traceability_layers:
  - User Stories
  - Requirements
  - Technical Specs
schema_version: "1.0.0"
---

# Traceability Compliance Reporting and Multi-Format Export Specification

## 1. Layer 4: User Stories (US-*)

### US-REPORT-01: Multi-Format Traceability Compliance Reporting
- **ID**: US-REPORT-01
- **Parent**: EPIC-QUERY, EPIC-CLI-CONTRACT
- **Type**: User Story
- **Intent & Value**: As a software engineer, quality auditor, or compliance officer, I want `xtrc` to generate comprehensive traceability reports in both human-readable (HTML, Markdown) and machine-readable (JSON, CSV) formats so that I can inspect, verify, and publish project compliance status across developer environments and audit pipelines.
- **Observable Contract & Behavior**: Generates complete compliance reports covering all 7 lifecycle layers with summary statistics, validation status, and detailed node linkages, emitted to standard output or saved to a file via `-o, --output <PATH>`.
- **Acceptance Criteria**: Supports `markdown`, `json`, `csv`, and `html` export formats with sub-second execution, deterministic content, and zero external runtime dependencies.

## 2. Layer 5: Requirements (REQ-*)

### REQ-REPORT-HTML: Standalone Human-Readable HTML Compliance Report
- **ID**: REQ-REPORT-HTML
- **Parent**: US-REPORT-01, REQ-CMD-EXPORT
- **Type**: Functional Requirement
- **Intent & Value**: Provide a self-contained, publication-ready visual report for browser inspection and audit documentation without external web assets or cloud services.
- **Observable Contract & Behavior**: `xtrc export --format html` produces a valid HTML5 document embedding executive metrics, status badges (`PASS`/`FAIL`), and detailed 7-layer node tables with inlined CSS styling and zero external font or script dependencies.
- **Acceptance Criteria**: The generated HTML is completely self-contained, parses as valid HTML5, reflects accurate graph statistics and node linkage, and passes automated verification.

### REQ-REPORT-MACHINE: Machine-Readable Universal JSON and CSV Export
- **ID**: REQ-REPORT-MACHINE
- **Parent**: US-REPORT-01, REQ-FMT-JSON
- **Type**: Functional Requirement
- **Intent & Value**: Allow external automation tools, security scanners, and compliance pipelines to ingest and process traceability graphs programmatically.
- **Observable Contract & Behavior**: Emits structured JSON conforming to the universal `UniversalEnvelope` schema (`schema_version: "1.0.0"`) and RFC 4180-compliant CSV tables with properly escaped cells.
- **Acceptance Criteria**: Output conforms strictly to the respective format schemas and exits with code `0`.

## 3. Layer 6: Technical Specifications (SPEC-*)

### SPEC-REPORT-HTML: Self-Contained HTML5 Traceability Export Engine Contract
- **ID**: SPEC-REPORT-HTML
- **Parent**: REQ-REPORT-HTML, REQ-REPORT-MACHINE, US-REPORT-01
- **Type**: TechnicalSpec
- **Component Target**: src/cli/export.rs, src/cli/output.rs, src/cli/mod.rs
- **Verification Target**: tests/cli/test_export.rs, tests/e2e/tier4_real_world.rs
- **Technical Specification**:
  - Format Identifier: `html` / `htm` (case-insensitive).
  - Output Structure:
    1. HTML5 Doctype: `<!DOCTYPE html>`.
    2. Embedded CSS: Responsive CSS containing styling for metrics grid, status badges (`.pass`, `.fail`), layer hierarchy tables, and code/test links.
    3. Executive Summary: Cards for Total Nodes, Specs, Requirements, Covered Requirements, Code Links, Test Links, Broken References, Cycles, Orphans, and Coverage Rate.
    4. Compliance Status Badge: Prominent `PASS` (green) or `FAIL` (red) pill based on `val_report.is_success(false)`.
    5. Layer Breakdown: Iterates across `Vision`, `Strategy`, `Epic`, `UserStory`, `Requirement`, and `TechnicalSpec`, rendering each node's ID, title, parents, children, origin, description, and code/test links.
  - Delivery: Written atomically to `-o <PATH>` via temporary file swap or written to locked standard output via `write_buffered_stdout`.
