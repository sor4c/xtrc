//! Common pipeline coordinator for repository root discovery, file traversal,
//! document parsing, comment scanning, and in-memory DAG construction.
//!
//! // @trace SPEC-CLI-OUTPUT, SPEC-DISC-DISCOVERY, SPEC-DISC-TRAVERSAL, SPEC-DAG-STORAGE
//! // @trace REQ-CFG-DISCOVERY, REQ-IGNORE-RULES, REQ-DOC-PARSE, REQ-CODE-SCAN, REQ-TEST-SCAN

use std::path::{Path, PathBuf};

use crate::config::discovery::discover_workspace;
use crate::config::WorkspaceConfig;
use crate::error::CliError;
use crate::graph::model::TraceabilityGraph;
use crate::model::LinkKind;
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

    let mut all_nodes = Vec::new();
    for spec_file in &categorized.specs {
        let nodes = parse_spec_file(spec_file)?;
        all_nodes.extend(nodes);
    }

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

    // 3. Scan remaining un-categorized workspace text files (polyglot scripts, migrations, etc.)
    let mut other_files = Vec::new();
    for file in &all_files {
        if !categorized.specs.contains(file)
            && !categorized.code.contains(file)
            && !categorized.tests.contains(file)
        {
            other_files.push(file.clone());
        }
    }
    if !other_files.is_empty() {
        let other_links = scan_workspace_comments(&other_files, Some(&root), None)?;
        all_links.extend(other_links);
    }

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
