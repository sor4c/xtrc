//! Common pipeline coordinator for repository root discovery, file traversal,
//! document parsing, comment scanning, and in-memory DAG construction.
//!
//! // @trace SPEC-CLI-OUTPUT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-DAG-STORAGE, SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, SPEC-CI-PERF, SPEC-DEMO-WORKSPACE
//! // @trace REQ-CFG-DISCOVERY, REQ-IGNORE-RULES, REQ-DOC-PARSE, REQ-CODE-SCAN, REQ-TEST-SCAN, REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, REQ-CI-PERF, US-PERF-01, REQ-DEMO-SETUP, REQ-DEMO-RUNNER

use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::discovery::discover_workspace;
use crate::config::WorkspaceConfig;
use crate::error::{CliError, ParseError};
use crate::graph::model::TraceabilityGraph;
use crate::model::{LinkKind, TraceNode};
use crate::parser::document::parse_spec_file;
use crate::scanner::comments::scan_workspace_comments;
use crate::scanner::traversal::{categorize_files, walk_workspace};

/// Discovers repository root, loads workspace configuration, traverses ignore-aware files,
/// parses specification documents, scans polyglot comments, and builds the in-memory `TraceabilityGraph`.
pub fn load_workspace_graph(
    root_override: Option<&Path>,
    config_override: Option<&Path>,
) -> Result<(PathBuf, WorkspaceConfig, TraceabilityGraph), CliError> {
    let start_dir = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => return Err(CliError::Io(e)),
    };

    let (root, config) = discover_workspace(&start_dir, root_override, config_override)?;

    let all_files = walk_workspace(&root, &config)?;
    let categorized = categorize_files(&root, &all_files, &config);

    let all_nodes: Vec<TraceNode> = if categorized.specs.len() <= 2 {
        let mut nodes = Vec::new();
        for spec_file in &categorized.specs {
            nodes.extend(parse_spec_file(spec_file)?);
        }
        nodes
    } else {
        let per_spec: Result<Vec<Vec<TraceNode>>, ParseError> = categorized
            .specs
            .par_iter()
            .map(|spec_file| parse_spec_file(spec_file))
            .collect();
        per_spec?.into_iter().flatten().collect()
    };

    let mut all_links = Vec::new();

    // 1. Scan files matching code.include
    let code_links =
        scan_workspace_comments(&categorized.code, Some(&root), Some(LinkKind::SourceCode))?;
    all_links.extend(code_links);

    // 2. Scan files matching tests.include
    let test_links = scan_workspace_comments(
        &categorized.tests,
        Some(&root),
        Some(LinkKind::AutomatedTest),
    )?;
    all_links.extend(test_links);

    let graph = TraceabilityGraph::build(all_nodes, all_links)?;

    Ok((root, config, graph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_workspace_graph_empty() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let (root, config, graph) = load_workspace_graph(Some(temp.path()), None)?;
        assert_eq!(root, temp.path());
        assert!(config.project.name.is_empty() || config.project.name == "xtrc");
        assert_eq!(graph.node_count(), 0);
        Ok(())
    }
}
