//! xtrc - Fast, Plain-Text Traceability CLI & 7-Layer DAG Engine in Rust.
//!
//! // @trace SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, SPEC-CI-PERF, SPEC-ERR-HIERARCHY
//! // @trace REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, REQ-NFR-ROBUST, REQ-CI-PERF, REQ-EXIT-CODES, US-PERF-01
//! // @trace SPEC-FMT-GRAMMAR, SPEC-FMT-FRONTMATTER, SPEC-SCAN-COMMENTS
//! // @trace REQ-DOC-PARSE, REQ-CODE-SCAN, REQ-TEST-SCAN
//! // @trace SPEC-DAG-STORAGE, SPEC-DAG-CYCLE, SPEC-DAG-VALIDATE, REQ-DAG-MODEL
//! // @trace REQ-CYCLE-DETECT, REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST
//! // @trace REQ-ORPHAN-DETECT, REQ-STRICT-GATE
//! // @trace SPEC-CLI-VERIFY, SPEC-CLI-MATRIX, SPEC-CLI-TREE, SPEC-CLI-EXPORT, SPEC-CLI-OUTPUT
//! // @trace REQ-CLI-INIT, REQ-CMD-MATRIX, REQ-CMD-TREE, REQ-CMD-EXPORT, REQ-FMT-JSON, REQ-UNIX-STREAMS
//! // @trace SPEC-CI-VERIFY, SPEC-CD-DELIVERY, SPEC-VER-POLICY, REQ-PKG-DIST, REQ-PKG-SBOM, REQ-CI-VERIFY, REQ-CD-MAIN, REQ-VER-POLICY

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

mod cli;
pub mod config;
mod error;
mod graph;
mod model;
mod parser;
mod render;
pub mod report;
mod scanner;

pub use cli::run_cli;
pub use error::{CliError, ConfigError, GraphError, ParseError};

use config::WorkspaceConfig;
use std::path::{Path, PathBuf};

use graph::model::TraceabilityGraph;
use graph::validator::{validate_graph, ValidationOptions};
use report::Report;

/// A loaded traceability workspace.
pub struct Workspace {
    root: PathBuf,
    config: WorkspaceConfig,
    graph: TraceabilityGraph,
}

impl Workspace {
    /// Returns the root directory of the loaded workspace.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the workspace configuration.
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }
}

/// Loads a workspace, its configuration, and its traceability graph.
///
/// # Errors
///
/// Returns an error when workspace discovery, parsing, scanning, or graph construction fails.
pub fn load_workspace(
    root_override: Option<&Path>,
    config_override: Option<&Path>,
) -> Result<Workspace, CliError> {
    let (root, config, graph) =
        cli::pipeline::load_workspace_graph(root_override, config_override)?;
    Ok(Workspace {
        root,
        config,
        graph,
    })
}

/// Validates a loaded workspace and returns its canonical report.
///
/// # Errors
///
/// Returns [`CliError::VerificationFailure`] when the workspace fails its configured validation
/// policy or contains a cycle.
pub fn check(workspace: &Workspace) -> Result<Report, CliError> {
    let strict = workspace.config.validation.strict;
    let mut validation = validate_graph(
        &workspace.graph,
        &ValidationOptions {
            strict,
            require_test_coverage: workspace.config.validation.require_test_coverage,
            layer_model: workspace.config.layers.clone(),
            ..Default::default()
        },
    );
    let cycles = workspace.graph.find_all_cycles();
    let empty_graph_is_fatal = strict && workspace.graph.is_empty();
    if empty_graph_is_fatal {
        validation.status = "fail".to_string();
    }
    let passed = validation.is_success(strict);

    let report = Report::from_graph(
        &workspace.graph,
        validation,
        "check",
        &workspace.config.layers,
    );
    if !passed || !cycles.is_empty() || empty_graph_is_fatal {
        let violations = report
            .violations
            .iter()
            .filter(|violation| violation.is_fatal(strict))
            .count()
            + cycles.len()
            + usize::from(empty_graph_is_fatal);
        return Err(CliError::VerificationFailure { violations });
    }

    Ok(report)
}

#[cfg(test)]
extern crate self as xtrc;

#[cfg(test)]
#[path = "config/discovery_internal_tests.rs"]
mod config_discovery_tests;

#[cfg(test)]
#[path = "model/identifier_internal_tests.rs"]
mod model_identifier_tests;

#[cfg(test)]
#[path = "parser/document_internal_tests.rs"]
mod parser_document_tests;

#[cfg(test)]
#[path = "scanner/comments_internal_tests.rs"]
mod scanner_comments_tests;

#[cfg(test)]
#[path = "scanner/traversal_internal_tests.rs"]
mod scanner_traversal_tests;

#[cfg(test)]
#[path = "report/internal_benchmarks.rs"]
mod report_benchmarks;
