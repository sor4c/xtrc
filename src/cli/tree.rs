//! Subcommand handler for `xtrc tree` visualizing traceability hierarchy trees.
//!
//! // @trace SPEC-CLI-TREE, SPEC-PERF-BOUNDS, REQ-NFR-PERF, REQ-CMD-TREE, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use std::fmt::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cli::output::{write_buffered_stdout, StreamEnvironment, UniversalEnvelope};
use crate::cli::pipeline::load_workspace_graph_ext;
use crate::error::{CliError, GraphError};
use crate::graph::model::TraceabilityGraph;
use crate::graph::validator::{validate_graph, ValidationOptions};
use crate::model::{NodeId, TraceNode};

/// Options configuring `xtrc tree` execution.
#[derive(Debug, Clone, Default)]
pub struct TreeOptions<'a> {
    /// Workspace root directory override.
    pub root: Option<&'a Path>,
    /// Configuration file override.
    pub config_path: Option<&'a Path>,
    /// Optional starting node identifier.
    pub id: Option<String>,
    /// Traverse upward toward Vision ancestors (justification path).
    pub reverse: bool,
    /// Traverse downward toward Implementation and Tests (decomposition path, default).
    pub down: bool,
    /// Maximum recursion depth (0 = root only).
    pub depth: Option<usize>,
    /// Emit machine-parseable JSON tree to stdout.
    pub json: bool,
    /// Verbose diagnostic output.
    pub verbose: bool,
}

/// JSON representation of a node in the tree hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNodeJson {
    /// Node identifier.
    pub id: String,
    /// Node title.
    pub title: String,
    /// Node lifecycle layer.
    pub layer: String,
    /// Parent identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<String>,
    /// Child specification nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeNodeJson>,
    /// Leaf source code and test verification links.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<TreeLinkJson>,
}

/// JSON representation of a leaf trace link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeLinkJson {
    /// Link classification kind ("source_code" or "automated_test").
    pub kind: String,
    /// Target node identifier.
    pub target_id: String,
    /// Originating source file.
    pub source_file: String,
    /// 1-indexed line number.
    pub line: usize,
}

/// JSON payload structure for `tree --json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreePayload {
    /// Hierarchical tree nodes.
    pub tree: Vec<TreeNodeJson>,
}

/// Executes tree visualization.
pub fn run_tree(options: TreeOptions<'_>) -> Result<(), CliError> {
    // Probe stream environment and disable ANSI color codes when piped
    let _env = StreamEnvironment::probe();

    let (_root, config, graph) =
        load_workspace_graph_ext(options.root, options.config_path, options.verbose)?;

    // 1. Resolve anchor node if specified
    let starting_nodes: Vec<&TraceNode> = match options.id {
        Some(ref s) => {
            if s.trim().is_empty() {
                return Err(CliError::InvalidArguments {
                    reason: "The ID argument cannot be empty".to_string(),
                });
            }
            let node_id = NodeId::new(s).map_err(|e| CliError::InvalidArguments {
                reason: format!("Invalid node ID '{s}': {e}"),
            })?;
            match graph.get_node(&node_id) {
                Some(node) => vec![node],
                None => {
                    return Err(CliError::NodeNotFound {
                        id: node_id.to_string(),
                    });
                }
            }
        }
        None => {
            if options.reverse {
                graph.leaves()
            } else {
                graph.roots()
            }
        }
    };

    let mut cycle_detected = false;

    // 2. Render JSON or Text
    if options.json {
        let mut tree_nodes = Vec::new();
        for root in &starting_nodes {
            let mut visited = Vec::new();
            let json_node = build_json_tree(
                &graph,
                root,
                options.reverse,
                0,
                options.depth,
                &mut visited,
                &mut cycle_detected,
            );
            tree_nodes.push(json_node);
        }

        let val_report = validate_graph(
            &graph,
            &ValidationOptions {
                require_test_coverage: config.validation.require_test_coverage,
                ..Default::default()
            },
        );
        let status = if !cycle_detected && val_report.is_success(false) {
            "pass"
        } else {
            "fail"
        };

        let envelope = UniversalEnvelope::new(
            "tree",
            status,
            val_report.statistics,
            TreePayload { tree: tree_nodes },
        );
        envelope.print_stdout()?;
    } else {
        let mut output_buf = String::with_capacity(4096);
        for (i, root) in starting_nodes.iter().enumerate() {
            if i > 0 {
                output_buf.push('\n');
            }
            let _ = writeln!(output_buf, "{}: {} ({})", root.id, root.title, root.layer);
            if options.depth != Some(0) {
                let mut walker = TreeWalker {
                    graph: &graph,
                    reverse: options.reverse,
                    max_depth: options.depth,
                    path_stack: vec![root.id.clone()],
                    cycle_detected: false,
                    out: &mut output_buf,
                };
                walker.render_branches(root, 1, "");
                cycle_detected |= walker.cycle_detected;
            }
        }
        write_buffered_stdout(&output_buf)?;
    }

    // 3. Cycle defect checking
    if cycle_detected || graph.detect_cycles().is_err() {
        let cycles = graph.find_all_cycles();
        for cycle in &cycles {
            eprintln!(
                "[ERROR] Cyclic dependency detected: {}",
                cycle.format_with_provenance(&graph)
            );
        }
        let chain = if let Some(first) = cycles.first() {
            first.format_with_provenance(&graph)
        } else {
            "cyclic dependency detected".to_string()
        };
        return Err(CliError::Graph(GraphError::CycleDetected {
            cycle_chain: chain,
        }));
    }

    Ok(())
}

/// Recursion state for rendering a text tree: fixed traversal settings plus the
/// active ancestor path (for cycle detection), whether a cycle was seen, and a shared output buffer.
struct TreeWalker<'g, 'b> {
    graph: &'g TraceabilityGraph,
    reverse: bool,
    max_depth: Option<usize>,
    path_stack: Vec<NodeId>,
    cycle_detected: bool,
    out: &'b mut String,
}

impl TreeWalker<'_, '_> {
    fn render_branches(&mut self, current: &TraceNode, current_depth: usize, prefix: &str) {
        let graph = self.graph;
        let reverse = self.reverse;
        let max_depth = self.max_depth;
        if let Some(limit) = max_depth {
            if current_depth > limit {
                return;
            }
        }

        if reverse {
            // Upward: visit parents
            let parents = graph.parents_of(&current.id);
            let total = parents.len();
            for (idx, parent_id) in parents.iter().enumerate() {
                let is_last = idx + 1 == total;
                let branch = if is_last { "└── " } else { "├── " };
                let next_prefix = if is_last {
                    format!("{prefix}    ")
                } else {
                    format!("{prefix}│   ")
                };

                if self.path_stack.contains(parent_id) {
                    self.cycle_detected = true;
                    let mut chain: Vec<String> =
                        self.path_stack.iter().map(|id| id.to_string()).collect();
                    chain.push(parent_id.to_string());
                    let _ = writeln!(
                        self.out,
                        "{prefix}{branch}{parent_id} (cycle: {})",
                        chain.join(" -> ")
                    );
                    continue;
                }

                if let Some(parent_node) = graph.get_node(parent_id) {
                    let _ = writeln!(
                        self.out,
                        "{prefix}{branch}{}: {} ({})",
                        parent_node.id, parent_node.title, parent_node.layer
                    );
                    self.path_stack.push(parent_id.clone());
                    self.render_branches(parent_node, current_depth + 1, &next_prefix);
                    self.path_stack.pop();
                } else {
                    let _ = writeln!(self.out, "{prefix}{branch}{parent_id} [missing]");
                }
            }
        } else {
            // Downward: visit spec children and leaf code/test links
            let children = graph.children_of(&current.id);
            let code_links = graph.code_links_for(&current.id);
            let test_links = graph.test_links_for(&current.id);
            let total_items = children.len() + code_links.len() + test_links.len();

            let mut item_idx = 0usize;

            // 1. Spec children
            for child_id in children {
                item_idx += 1;
                let is_last = item_idx == total_items;
                let branch = if is_last { "└── " } else { "├── " };
                let next_prefix = if is_last {
                    format!("{prefix}    ")
                } else {
                    format!("{prefix}│   ")
                };

                if self.path_stack.contains(child_id) {
                    self.cycle_detected = true;
                    let mut chain: Vec<String> =
                        self.path_stack.iter().map(|id| id.to_string()).collect();
                    chain.push(child_id.to_string());
                    let _ = writeln!(
                        self.out,
                        "{prefix}{branch}{child_id} (cycle: {})",
                        chain.join(" -> ")
                    );
                    continue;
                }

                if let Some(child_node) = graph.get_node(child_id) {
                    let _ = writeln!(
                        self.out,
                        "{prefix}{branch}{}: {} ({})",
                        child_node.id, child_node.title, child_node.layer
                    );
                    self.path_stack.push(child_id.clone());
                    self.render_branches(child_node, current_depth + 1, &next_prefix);
                    self.path_stack.pop();
                } else {
                    let _ = writeln!(self.out, "{prefix}{branch}{child_id} [missing]");
                }
            }

            // 2. Code links
            for link in code_links {
                item_idx += 1;
                let is_last = item_idx == total_items;
                let branch = if is_last { "└── " } else { "├── " };
                let _ = writeln!(
                    self.out,
                    "{prefix}{branch}[code] {}:{}",
                    link.source_file.display(),
                    link.line
                );
            }

            // 3. Test links
            for link in test_links {
                item_idx += 1;
                let is_last = item_idx == total_items;
                let branch = if is_last { "└── " } else { "├── " };
                let _ = writeln!(
                    self.out,
                    "{prefix}{branch}[test] {}:{}",
                    link.source_file.display(),
                    link.line
                );
            }
        }
    }
}

fn build_json_tree(
    graph: &TraceabilityGraph,
    current: &TraceNode,
    reverse: bool,
    current_depth: usize,
    max_depth: Option<usize>,
    path_stack: &mut Vec<NodeId>,
    cycle_detected: &mut bool,
) -> TreeNodeJson {
    let mut children_json = Vec::new();
    let mut links_json = Vec::new();

    if let Some(limit) = max_depth {
        if current_depth >= limit {
            return TreeNodeJson {
                id: current.id.to_string(),
                title: current.title.clone(),
                layer: current.layer.to_string(),
                parents: current.parents.iter().map(|p| p.to_string()).collect(),
                children: children_json,
                links: links_json,
            };
        }
    }

    path_stack.push(current.id.clone());

    if reverse {
        let parents = graph.parents_of(&current.id);
        for parent_id in parents {
            if path_stack.contains(parent_id) {
                *cycle_detected = true;
                continue;
            }
            if let Some(parent_node) = graph.get_node(parent_id) {
                let child = build_json_tree(
                    graph,
                    parent_node,
                    reverse,
                    current_depth + 1,
                    max_depth,
                    path_stack,
                    cycle_detected,
                );
                children_json.push(child);
            }
        }
    } else {
        let children = graph.children_of(&current.id);
        for child_id in children {
            if path_stack.contains(child_id) {
                *cycle_detected = true;
                continue;
            }
            if let Some(child_node) = graph.get_node(child_id) {
                let child = build_json_tree(
                    graph,
                    child_node,
                    reverse,
                    current_depth + 1,
                    max_depth,
                    path_stack,
                    cycle_detected,
                );
                children_json.push(child);
            }
        }

        for link in graph.links_for(&current.id) {
            links_json.push(TreeLinkJson {
                kind: link.kind.to_string(),
                target_id: link.target_id.to_string(),
                source_file: link.source_file.to_string_lossy().to_string(),
                line: link.line,
            });
        }
    }

    path_stack.pop();

    TreeNodeJson {
        id: current.id.to_string(),
        title: current.title.clone(),
        layer: current.layer.to_string(),
        parents: current.parents.iter().map(|p| p.to_string()).collect(),
        children: children_json,
        links: links_json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_options_default() {
        let opts = TreeOptions::default();
        assert!(opts.id.is_none());
        assert!(!opts.reverse);
        assert!(!opts.down);
        assert!(opts.depth.is_none());
        assert!(!opts.json);
    }
}
