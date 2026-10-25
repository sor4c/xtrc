//! Subcommand handler for `xtrc matrix` generating multidimensional traceability matrices.
//!
//! // @trace SPEC-CLI-MATRIX, REQ-CMD-MATRIX, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::cli::output::{
    is_terminal_stdout, render_aligned_table, render_csv, render_markdown_table,
    write_buffered_stdout, MatrixFormat, StreamEnvironment, UniversalEnvelope,
};
use crate::cli::pipeline::load_workspace_graph;
use crate::error::CliError;
use crate::graph::validator::{validate_graph, ValidationOptions};
use crate::model::Layer;

/// Options configuring `xtrc matrix` execution.
#[derive(Debug, Clone, Default)]
pub struct MatrixOptions<'a> {
    /// Workspace root directory override.
    pub root: Option<&'a Path>,
    /// Configuration file override.
    pub config_path: Option<&'a Path>,
    /// Output presentation format string (table, markdown, csv, json).
    pub format: Option<String>,
    /// Filter matrix to only display items lacking forward code or test coverage.
    pub uncovered: bool,
    /// Filter matrix rows to a specific layer boundary.
    pub layer: Option<String>,
    /// Emit universal JSON matrix to stdout (alias for --format json).
    pub json: bool,
}

/// Structured row representation of a traceability matrix entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixRow {
    /// Item identifier.
    pub id: String,
    /// Item title.
    pub title: String,
    /// Upward parent identifiers.
    pub parents: Vec<String>,
    /// Downward child specification identifiers.
    pub specs: Vec<String>,
    /// Total source code implementation links.
    pub code_links_count: usize,
    /// Total automated test verification links.
    pub test_links_count: usize,
    /// Forward coverage status label ("Covered", "Uncovered", "Missing Code", "Missing Test").
    pub status: String,
}

/// JSON payload structure for `matrix --json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixPayload {
    /// Total rows emitted.
    pub total_rows: usize,
    /// Matrix rows.
    pub rows: Vec<MatrixRow>,
}

/// Executes matrix generation and rendering.
pub fn run_matrix(options: MatrixOptions<'_>) -> Result<(), CliError> {
    // Probe stream environment and disable ANSI color codes when piped
    let _env = StreamEnvironment::probe();

    let (_root, _config, graph) = load_workspace_graph(options.root, options.config_path)?;

    // 1. Resolve target layer
    let target_layer = match options.layer {
        Some(ref s) => Layer::from_str(s).map_err(|e| CliError::InvalidArguments {
            reason: format!("Invalid layer '{s}': {e}"),
        })?,
        None => Layer::Requirement,
    };

    // 2. Resolve output format
    let format = if options.json {
        MatrixFormat::Json
    } else {
        match options.format {
            Some(ref s) => MatrixFormat::from_str(s)?,
            None => {
                if is_terminal_stdout() {
                    MatrixFormat::Table
                } else {
                    MatrixFormat::Markdown
                }
            }
        }
    };

    // 3. Assemble matrix rows
    let nodes = graph.nodes_by_layer(target_layer);
    let mut rows: Vec<MatrixRow> = Vec::new();

    for node in nodes {
        let parents: Vec<String> = graph
            .parents_of(&node.id)
            .iter()
            .map(|p| p.to_string())
            .collect();

        let child_specs: Vec<&crate::model::TraceNode> = graph
            .children_of(&node.id)
            .iter()
            .filter_map(|cid| graph.get_node(cid))
            .filter(|n| n.layer == Layer::TechnicalSpec)
            .collect();

        let specs: Vec<String> = child_specs.iter().map(|n| n.id.to_string()).collect();

        let direct_code = graph.code_links_for(&node.id).len();
        let spec_code: usize = child_specs
            .iter()
            .map(|spec| graph.code_links_for(&spec.id).len())
            .sum();
        let total_code = direct_code + spec_code;

        let direct_test = graph.test_links_for(&node.id).len();
        let spec_test: usize = child_specs
            .iter()
            .map(|spec| graph.test_links_for(&spec.id).len())
            .sum();
        let total_test = direct_test + spec_test;

        let status = match (total_code > 0, total_test > 0) {
            (true, true) => "Covered",
            (false, false) => "Uncovered",
            (false, true) => "Missing Code",
            (true, false) => "Missing Test",
        };

        if options.uncovered && status == "Covered" {
            continue;
        }

        rows.push(MatrixRow {
            id: node.id.to_string(),
            title: node.title.clone(),
            parents,
            specs,
            code_links_count: total_code,
            test_links_count: total_test,
            status: status.to_string(),
        });
    }

    // 4. Render output to stdout
    let headers = [
        "Requirement ID",
        "Title",
        "Parent US",
        "Specs",
        "Code Links",
        "Test Links",
        "Status",
    ];

    match format {
        MatrixFormat::Json => {
            let val_report = validate_graph(&graph, &ValidationOptions::default());
            let envelope = UniversalEnvelope::new(
                "matrix",
                if val_report.is_success(false) {
                    "pass"
                } else {
                    "fail"
                },
                val_report.statistics,
                MatrixPayload {
                    total_rows: rows.len(),
                    rows,
                },
            );
            envelope.print_stdout()?;
        }
        MatrixFormat::Markdown => {
            let row_cells: Vec<Vec<String>> = rows
                .iter()
                .map(|r| {
                    vec![
                        r.id.clone(),
                        r.title.clone(),
                        if r.parents.is_empty() {
                            "-".to_string()
                        } else {
                            r.parents.join(", ")
                        },
                        if r.specs.is_empty() {
                            "-".to_string()
                        } else {
                            r.specs.join(", ")
                        },
                        r.code_links_count.to_string(),
                        r.test_links_count.to_string(),
                        r.status.clone(),
                    ]
                })
                .collect();
            let table = render_markdown_table(&headers, &row_cells);
            write_buffered_stdout(&table)?;
        }
        MatrixFormat::Csv => {
            let row_cells: Vec<Vec<String>> = rows
                .iter()
                .map(|r| {
                    vec![
                        r.id.clone(),
                        r.title.clone(),
                        if r.parents.is_empty() {
                            String::new()
                        } else {
                            r.parents.join("; ")
                        },
                        if r.specs.is_empty() {
                            String::new()
                        } else {
                            r.specs.join("; ")
                        },
                        r.code_links_count.to_string(),
                        r.test_links_count.to_string(),
                        r.status.clone(),
                    ]
                })
                .collect();
            let csv = render_csv(&headers, &row_cells);
            write_buffered_stdout(&csv)?;
        }
        MatrixFormat::Table => {
            let row_cells: Vec<Vec<String>> = rows
                .iter()
                .map(|r| {
                    vec![
                        r.id.clone(),
                        r.title.clone(),
                        if r.parents.is_empty() {
                            "-".to_string()
                        } else {
                            r.parents.join(", ")
                        },
                        if r.specs.is_empty() {
                            "-".to_string()
                        } else {
                            r.specs.join(", ")
                        },
                        r.code_links_count.to_string(),
                        r.test_links_count.to_string(),
                        r.status.clone(),
                    ]
                })
                .collect();
            let table = render_aligned_table(&headers, &row_cells);
            write_buffered_stdout(&table)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_row_creation() {
        let row = MatrixRow {
            id: "REQ-01".to_string(),
            title: "Verification".to_string(),
            parents: vec!["US-01".to_string()],
            specs: vec!["SPEC-01".to_string()],
            code_links_count: 1,
            test_links_count: 1,
            status: "Covered".to_string(),
        };
        assert_eq!(row.status, "Covered");
        assert_eq!(row.code_links_count, 1);
    }
}
