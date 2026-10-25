//! Subcommand handler for `xtrc export` generating compliance audit documentation.
//!
//! // @trace SPEC-CLI-EXPORT, REQ-CMD-EXPORT, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::cli::output::{
    escape_csv_field, write_buffered_stdout, StreamEnvironment, UniversalEnvelope,
};
use crate::cli::pipeline::load_workspace_graph;
use crate::error::CliError;
use crate::graph::model::TraceabilityGraph;
use crate::graph::validator::{validate_graph, ValidationOptions};
use crate::model::Layer;

static ATOMIC_EXPORT_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Supported output formats for `export`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Consolidated Markdown documentation.
    Markdown,
    /// Universal JSON document.
    Json,
    /// Comma-separated values document conforming to RFC 4180.
    Csv,
}

impl FromStr for ExportFormat {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => Err(CliError::InvalidFormat {
                format: other.to_string(),
            }),
        }
    }
}

/// Options configuring `xtrc export` execution.
#[derive(Debug, Clone)]
pub struct ExportOptions<'a> {
    /// Workspace root directory override.
    pub root: Option<&'a Path>,
    /// Configuration file override.
    pub config_path: Option<&'a Path>,
    /// Export presentation format (markdown, json, csv).
    pub format: String,
    /// Target file path to write. If omitted, emits to stdout.
    pub output: Option<PathBuf>,
}

impl<'a> Default for ExportOptions<'a> {
    fn default() -> Self {
        Self {
            root: None,
            config_path: None,
            format: "markdown".to_string(),
            output: None,
        }
    }
}

/// JSON payload representation for `export --format json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPayload {
    /// Title of the audit document.
    pub title: String,
    /// Layered specification nodes with link details.
    pub nodes: Vec<ExportNodeJson>,
}

/// Detailed node information for export audit document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportNodeJson {
    /// Node identifier.
    pub id: String,
    /// Node title.
    pub title: String,
    /// Node layer.
    pub layer: String,
    /// Parent identifiers.
    pub parents: Vec<String>,
    /// Child identifiers.
    pub children: Vec<String>,
    /// Provenance file and line origin.
    pub source_origin: String,
    /// Description body.
    pub description: String,
    /// Code links.
    pub code_links: Vec<String>,
    /// Test links.
    pub test_links: Vec<String>,
}

/// Executes export generation and file writing.
pub fn run_export(options: ExportOptions<'_>) -> Result<(), CliError> {
    // Probe stream environment and disable ANSI color codes when piped
    let _env = StreamEnvironment::probe();

    let format = ExportFormat::from_str(&options.format)?;

    let (_root, _config, graph) = load_workspace_graph(options.root, options.config_path)?;

    let val_report = validate_graph(&graph, &ValidationOptions::default());

    // Generate output content string according to selected format
    let content = match format {
        ExportFormat::Markdown => generate_markdown_export(&graph, &val_report.statistics),
        ExportFormat::Csv => generate_csv_export(&graph),
        ExportFormat::Json => {
            let mut export_nodes = Vec::new();
            for node in graph.all_nodes() {
                let parents: Vec<String> = graph
                    .parents_of(&node.id)
                    .iter()
                    .map(|p| p.to_string())
                    .collect();
                let children: Vec<String> = graph
                    .children_of(&node.id)
                    .iter()
                    .map(|c| c.to_string())
                    .collect();
                let code_links: Vec<String> = graph
                    .code_links_for(&node.id)
                    .iter()
                    .map(|l| format!("{}:{}", l.source_file.display(), l.line))
                    .collect();
                let test_links: Vec<String> = graph
                    .test_links_for(&node.id)
                    .iter()
                    .map(|l| format!("{}:{}", l.source_file.display(), l.line))
                    .collect();

                export_nodes.push(ExportNodeJson {
                    id: node.id.to_string(),
                    title: node.title.clone(),
                    layer: node.layer.to_string(),
                    parents,
                    children,
                    source_origin: node.file_origin.to_string(),
                    description: node.description.clone(),
                    code_links,
                    test_links,
                });
            }

            let status = if val_report.is_success(false) {
                "pass"
            } else {
                "fail"
            };

            let envelope = UniversalEnvelope::new(
                "export",
                status,
                val_report.statistics,
                ExportPayload {
                    title: "Traceability Compliance Audit Report".to_string(),
                    nodes: export_nodes,
                },
            );

            serde_json::to_string_pretty(&envelope).map_err(|e| CliError::InvalidArguments {
                reason: format!("JSON serialization error: {e}"),
            })? + "\n"
        }
    };

    // Output delivery: atomic write to file if `-o` supplied, else print to stdout
    if let Some(ref target_path) = options.output {
        let parent = target_path.parent().unwrap_or(Path::new("."));
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(CliError::Io)?;
        }

        let file_name = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("export");
        let temp_name = format!(
            ".{}.tmp.{}_{}",
            file_name,
            std::process::id(),
            ATOMIC_EXPORT_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let temp_path = parent.join(temp_name);

        if let Err(e) = std::fs::write(&temp_path, &content) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(CliError::Io(e));
        }

        if let Err(e) = std::fs::rename(&temp_path, target_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(CliError::Io(e));
        }
    } else {
        write_buffered_stdout(&content)?;
    }

    Ok(())
}

fn generate_markdown_export(
    graph: &TraceabilityGraph,
    stats: &crate::graph::validator::ValidationStats,
) -> String {
    let mut out = String::new();
    out.push_str("# Traceability Compliance Audit Report\n\n");
    out.push_str("## Executive Summary\n\n");
    out.push_str("| Metric | Value |\n");
    out.push_str("| --- | --- |\n");
    out.push_str(&format!(
        "| Total Specification Nodes | {} |\n",
        stats.total_nodes
    ));
    out.push_str(&format!(
        "| Technical Specifications (Layer 6) | {} |\n",
        stats.specs_count
    ));
    out.push_str(&format!(
        "| Requirements (Layer 5) | {} |\n",
        stats.requirements_count
    ));
    out.push_str(&format!(
        "| Covered Requirements | {} |\n",
        stats.covered_nodes
    ));
    out.push_str(&format!(
        "| Source Code Implementation Links | {} |\n",
        stats.code_links
    ));
    out.push_str(&format!(
        "| Automated Test Verification Links | {} |\n",
        stats.test_links
    ));
    out.push_str(&format!("| Broken References | {} |\n", stats.broken_links));
    out.push_str(&format!("| Cycle Loops | {} |\n", stats.cycles_count));
    out.push_str(&format!("| Orphan Items | {} |\n", stats.orphan_nodes));
    out.push_str(&format!(
        "| Requirement Coverage Rate | {:.1}% |\n\n",
        stats.coverage_percent
    ));

    out.push_str("## 7-Layer Specification Traceability\n\n");

    let layers = [
        Layer::Vision,
        Layer::Strategy,
        Layer::Epic,
        Layer::UserStory,
        Layer::Requirement,
        Layer::TechnicalSpec,
    ];

    for layer in layers {
        let nodes = graph.nodes_by_layer(layer);
        if nodes.is_empty() {
            continue;
        }

        out.push_str(&format!("### {layer}\n\n"));

        for node in nodes {
            out.push_str(&format!("#### {}: {}\n\n", node.id, node.title));
            out.push_str(&format!("- **ID**: {}\n", node.id));
            out.push_str(&format!("- **Layer**: {}\n", node.layer));

            let parents = graph.parents_of(&node.id);
            if parents.is_empty() {
                out.push_str("- **Parents**: None\n");
            } else {
                let p_strs: Vec<String> = parents.iter().map(|p| p.to_string()).collect();
                out.push_str(&format!("- **Parents**: {}\n", p_strs.join(", ")));
            }

            let children = graph.children_of(&node.id);
            if children.is_empty() {
                out.push_str("- **Children**: None\n");
            } else {
                let c_strs: Vec<String> = children.iter().map(|c| c.to_string()).collect();
                out.push_str(&format!("- **Children**: {}\n", c_strs.join(", ")));
            }

            out.push_str(&format!("- **Source File**: {}\n", node.file_origin));

            let code_links = graph.code_links_for(&node.id);
            if !code_links.is_empty() {
                let c_strs: Vec<String> = code_links
                    .iter()
                    .map(|l| format!("{}:{}", l.source_file.display(), l.line))
                    .collect();
                out.push_str(&format!(
                    "- **Implementation Links**: {}\n",
                    c_strs.join(", ")
                ));
            }

            let test_links = graph.test_links_for(&node.id);
            if !test_links.is_empty() {
                let t_strs: Vec<String> = test_links
                    .iter()
                    .map(|l| format!("{}:{}", l.source_file.display(), l.line))
                    .collect();
                out.push_str(&format!(
                    "- **Verification Links**: {}\n",
                    t_strs.join(", ")
                ));
            }

            if !node.description.is_empty() {
                out.push_str(&format!("\n{}\n", node.description));
            }

            out.push('\n');
        }
    }

    out
}

fn generate_csv_export(graph: &TraceabilityGraph) -> String {
    let mut out = String::new();
    out.push_str("ID,Title,Layer,Parents,Children,Code Links,Test Links,Coverage Status\n");

    for node in graph.all_nodes() {
        let parents = graph.parents_of(&node.id);
        let p_str = if parents.is_empty() {
            String::new()
        } else {
            let p_strs: Vec<String> = parents.iter().map(|p| p.to_string()).collect();
            p_strs.join("; ")
        };

        let children = graph.children_of(&node.id);
        let c_str = if children.is_empty() {
            String::new()
        } else {
            let c_strs: Vec<String> = children.iter().map(|c| c.to_string()).collect();
            c_strs.join("; ")
        };

        let code_count = graph.code_links_for(&node.id).len();
        let test_count = graph.test_links_for(&node.id).len();
        let status = match (code_count > 0, test_count > 0) {
            (true, true) => "Covered",
            (false, false) => "Uncovered",
            (false, true) => "Missing Code",
            (true, false) => "Missing Test",
        };

        let row = [
            escape_csv_field(node.id.as_str()),
            escape_csv_field(&node.title),
            escape_csv_field(&node.layer.to_string()),
            escape_csv_field(&p_str),
            escape_csv_field(&c_str),
            code_count.to_string(),
            test_count.to_string(),
            status.to_string(),
        ];
        out.push_str(&row.join(","));
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_parsing() {
        assert_eq!(
            ExportFormat::from_str("markdown").ok(),
            Some(ExportFormat::Markdown)
        );
        assert_eq!(
            ExportFormat::from_str("md").ok(),
            Some(ExportFormat::Markdown)
        );
        assert_eq!(
            ExportFormat::from_str("json").ok(),
            Some(ExportFormat::Json)
        );
        assert_eq!(ExportFormat::from_str("csv").ok(), Some(ExportFormat::Csv));
        assert!(ExportFormat::from_str("pdf").is_err());
    }
}
