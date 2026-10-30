//! Subcommand handler for `xtrc matrix` generating multidimensional traceability matrices.
//!
//! // @trace SPEC-CLI-MATRIX, REQ-CMD-MATRIX, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use std::path::Path;
use std::str::FromStr;

use crate::cli::output::{
    is_terminal_stdout, render_aligned_table, render_csv, render_markdown_table,
    write_buffered_stdout, MatrixFormat, StreamEnvironment,
};
use crate::cli::pipeline::load_workspace_graph_ext;
use crate::error::CliError;
use crate::graph::validator::{validate_graph, ValidationOptions};
use crate::model::Layer;
use crate::render::render_json;
use crate::report::{Report, ReportMatrixRow};

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
    /// Verbose diagnostic output.
    pub verbose: bool,
}

/// Structured row representation of a traceability matrix entry.
/// Structured row representation of a traceability matrix entry.
pub type MatrixRow = ReportMatrixRow;

/// Executes matrix generation and rendering.
pub fn run_matrix(options: MatrixOptions<'_>) -> Result<(), CliError> {
    // Probe stream environment and disable ANSI color codes when piped
    let _env = StreamEnvironment::probe();

    let (_root, config, graph) =
        load_workspace_graph_ext(options.root, options.config_path, options.verbose)?;

    // 1. Resolve target layer
    let target_layer = match options.layer {
        Some(ref s) => config
            .layers
            .parse_layer(s)
            .or_else(|| {
                Layer::from_str(s)
                    .ok()
                    .filter(|layer| config.layers.is_default() || layer.is_leaf())
            })
            .ok_or_else(|| CliError::InvalidArguments {
                reason: format!(
                    "Invalid layer '{s}'. Valid layers: {}",
                    config
                        .layers
                        .layers
                        .iter()
                        .map(|layer| layer.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?,
        None => config
            .layers
            .layer_at(config.layers.coverage_index())
            .unwrap_or(Layer::Requirement),
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

    // 3. Build the canonical report once; matrix rows reuse its computed coverage.
    let validation = validate_graph(
        &graph,
        &ValidationOptions {
            require_test_coverage: config.validation.require_test_coverage,
            layer_model: config.layers.clone(),
            ..Default::default()
        },
    );
    let mut report = Report::from_graph(&graph, validation, "matrix", &config.layers);
    let target_name = config
        .layers
        .name_for_layer(target_layer)
        .unwrap_or("Layer");
    report.rows = report
        .nodes
        .iter()
        .filter(|node| {
            node.layer == target_name
                && (!options.uncovered || node.coverage_status.as_str() != "Covered")
        })
        .map(|node| ReportMatrixRow {
            id: node.id.clone(),
            layer: node.layer.clone(),
            title: node.title.clone(),
            parents: node.parents.clone(),
            specs: node.specs.clone(),
            code_links_count: node.code_links_count,
            test_links_count: node.test_links_count,
            status: node.coverage_status.as_str().to_string(),
        })
        .collect();
    report.total_rows = report.rows.len();
    let rows = &report.rows;

    // 4. Render output to stdout
    let layer_index = usize::from(target_layer.rank()).saturating_sub(1);
    let parent_header = layer_index
        .checked_sub(1)
        .and_then(|index| config.layers.layers.get(index))
        .map(|layer| format!("Parent {}", layer.name))
        .unwrap_or_else(|| "Parents".to_string());
    let child_header = config
        .layers
        .layers
        .get(layer_index + 1)
        .map(|layer| format!("Child {}", layer.name))
        .unwrap_or_else(|| "Children".to_string());
    let id_header = format!("{target_name} ID");
    let headers = [
        id_header.as_str(),
        "Title",
        parent_header.as_str(),
        child_header.as_str(),
        "Code Links",
        "Test Links",
        "Status",
    ];

    match format {
        MatrixFormat::Json => {
            let json = render_json(&report).map_err(|e| CliError::InvalidArguments {
                reason: format!("JSON serialization error: {e}"),
            })?;
            write_buffered_stdout(&format!("{json}\n"))?;
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
        MatrixFormat::Html => {
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
            let mut html = String::from("<table><thead><tr>");
            for h in &headers {
                html.push_str(&format!("<th>{h}</th>"));
            }
            html.push_str("</tr></thead><tbody>");
            for row in &row_cells {
                html.push_str("<tr>");
                for cell in row {
                    html.push_str(&format!(
                        "<td>{}</td>",
                        cell.replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                    ));
                }
                html.push_str("</tr>");
            }
            html.push_str("</tbody></table>\n");
            write_buffered_stdout(&html)?;
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
            layer: "Requirement".to_string(),
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
