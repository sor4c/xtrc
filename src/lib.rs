//! xtrc - Fast, Plain-Text Traceability CLI & 7-Layer DAG Engine in Rust.
//!
//! // @trace SPEC-PERF-BOUNDS, SPEC-ERR-HIERARCHY, REQ-NFR-ROBUST, REQ-EXIT-CODES
//! // @trace SPEC-FMT-GRAMMAR, SPEC-FMT-FRONTMATTER, SPEC-SCAN-COMMENTS
//! // @trace REQ-DOC-PARSE, REQ-CODE-SCAN, REQ-TEST-SCAN
//! // @trace SPEC-DAG-STORAGE, SPEC-DAG-CYCLE, SPEC-DAG-VALIDATE, REQ-DAG-MODEL
//! // @trace REQ-CYCLE-DETECT, REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST
//! // @trace REQ-ORPHAN-DETECT, REQ-STRICT-GATE
//! // @trace SPEC-CLI-VERIFY, SPEC-CLI-MATRIX, SPEC-CLI-TREE, SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT
//! // @trace REQ-CLI-INIT, REQ-CMD-MATRIX, REQ-CMD-TREE, REQ-CMD-EXPORT, REQ-FMT-JSON, REQ-UNIX-STREAMS
//! // @trace SPEC-CI-VERIFY, SPEC-CD-DELIVERY, SPEC-VER-POLICY, REQ-PKG-DIST, REQ-PKG-SBOM, REQ-CI-VERIFY, REQ-CD-MAIN, REQ-VER-POLICY

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod cli;
pub mod config;
pub mod error;
pub mod graph;
pub mod model;
pub mod parser;
pub mod scanner;

pub use cli::output::{
    MatrixFormat, OutputFormat, StreamEnvironment, TabularFormatter, UniversalEnvelope,
};
pub use cli::pipeline::load_workspace_graph;
pub use cli::{Cli, Commands};
pub use config::WorkspaceConfig;
pub use error::{CliError, ConfigError, GraphError, ParseError};
pub use graph::{
    detect_cycles, find_all_cycles, validate_graph, BrokenLinkKind, Cycle, CycleDetector,
    EpicCoverage, NodeColor, TraceabilityGraph, ValidationOptions, ValidationReport,
    ValidationStats, ValidationViolation, ViolationSeverity,
};
pub use model::{
    validate_parent_relationship, FileOrigin, IdError, Layer, LinkKind, NodeId, TraceLink,
    TraceNode, NODE_ID_REGEX_PATTERN,
};
pub use parser::{
    parse_document, parse_document_file, parse_spec_content, parse_spec_file, DocumentFrontmatter,
    DocumentStatus, SpecDocument,
};
pub use scanner::{
    classify_link_kind, clean_id_token, contains_trace_marker_bytes, is_test_file_path,
    scan_comment_line, scan_comments_file, scan_comments_text, scan_workspace_comments,
};
