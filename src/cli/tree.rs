//! Subcommand handler for `xtrc tree` visualizing traceability hierarchy trees.
//!
//! // @trace SPEC-CLI-TREE, SPEC-PERF-BOUNDS, REQ-NFR-PERF, REQ-CMD-TREE, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use std::fmt::Write as _;
use std::path::Path;

use crate::cli::output::{write_buffered_stdout, StreamEnvironment};
use crate::cli::pipeline::load_workspace_graph_ext;
use crate::error::{CliError, GraphError};
use crate::graph::model::TraceabilityGraph;
use crate::graph::validator::{validate_graph, ValidationOptions};
use crate::model::{NodeId, TraceNode};
use crate::render::render_json;
use crate::report::{Report, ReportTreeLink, ReportTreeNode};

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
pub type TreeNodeJson = ReportTreeNode;
/// JSON representation of a leaf trace link.
pub type TreeLinkJson = ReportTreeLink;

/// Executes tree visualization.
pub fn run_tree(options: TreeOptions<'_>) -> Result<(), CliError> {
    // Probe stream environment and disable ANSI color codes when piped
    let _env = StreamEnvironment::probe();

    let (_root, config, graph) =
        load_workspace_graph_ext(options.root, options.config_path, options.verbose)?;
    let reverse = options.reverse && !options.down;
    let validation = validate_graph(
        &graph,
        &ValidationOptions {
            require_test_coverage: config.validation.require_test_coverage,
            layer_model: config.layers.clone(),
            ..Default::default()
        },
    );
    let mut report = Report::from_graph(&graph, validation, "tree", &config.layers);

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
            if reverse {
                graph.leaves()
            } else {
                graph.roots()
            }
        }
    };

    let mut cycle_detected = false;
    let mut tree_nodes = Vec::new();
    let builder = TreeJsonBuilder {
        graph: &graph,
        reverse,
        max_depth: options.depth,
        layers: &config.layers,
    };
    for root in &starting_nodes {
        let mut visited = Vec::new();
        let json_node = builder.build(root, 0, &mut visited, &mut cycle_detected);
        tree_nodes.push(json_node);
    }
    report.tree = tree_nodes;
    if cycle_detected {
        report.status = "fail".to_string();
    }

    // 2. Render JSON or Text
    if options.json {
        let json = render_json(&report).map_err(|e| CliError::InvalidArguments {
            reason: format!("JSON serialization error: {e}"),
        })?;
        write_buffered_stdout(&format!("{json}\n"))?;
    } else {
        write_buffered_stdout(&render_tree_text(&report))?;
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

struct TreeJsonBuilder<'g, 'm> {
    graph: &'g TraceabilityGraph,
    reverse: bool,
    max_depth: Option<usize>,
    layers: &'m crate::model::LayerModel,
}

fn render_tree_text(report: &Report) -> String {
    let mut output = String::with_capacity(4096);
    for (index, root) in report.tree.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let _ = writeln!(output, "{}: {} ({})", root.id, root.title, root.layer);
        render_tree_branches(root, "", &mut output);
    }
    output
}

fn render_tree_branches(node: &TreeNodeJson, prefix: &str, output: &mut String) {
    let total = node.children.len() + node.links.len();
    let mut item_index = 0;
    for child in &node.children {
        item_index += 1;
        let is_last = item_index == total;
        let branch = if is_last { "└── " } else { "├── " };
        let next_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };
        let _ = writeln!(
            output,
            "{prefix}{branch}{}: {} ({})",
            child.id, child.title, child.layer
        );
        render_tree_branches(child, &next_prefix, output);
    }

    for kind in ["source_code", "automated_test"] {
        for link in node.links.iter().filter(|link| link.kind == kind) {
            item_index += 1;
            let is_last = item_index == total;
            let branch = if is_last { "└── " } else { "├── " };
            let label = if kind == "source_code" {
                "code"
            } else {
                "test"
            };
            let _ = writeln!(
                output,
                "{prefix}{branch}[{label}] {}:{}",
                link.source_file, link.line
            );
        }
    }
}

impl TreeJsonBuilder<'_, '_> {
    fn build(
        &self,
        current: &TraceNode,
        current_depth: usize,
        path_stack: &mut Vec<NodeId>,
        cycle_detected: &mut bool,
    ) -> TreeNodeJson {
        let mut children_json = Vec::new();
        let mut links_json = Vec::new();

        if let Some(limit) = self.max_depth {
            if current_depth >= limit {
                return TreeNodeJson {
                    id: current.id.to_string(),
                    title: current.title.clone(),
                    layer: self
                        .layers
                        .name_for_layer(current.layer)
                        .unwrap_or(current.layer.as_str())
                        .to_string(),
                    parents: current.parents.iter().map(|p| p.to_string()).collect(),
                    children: children_json,
                    links: links_json,
                };
            }
        }

        path_stack.push(current.id.clone());

        if self.reverse {
            let parents = self.graph.parents_of(&current.id);
            for parent_id in parents {
                if path_stack.contains(parent_id) {
                    *cycle_detected = true;
                    continue;
                }
                if let Some(parent_node) = self.graph.get_node(parent_id) {
                    let child =
                        self.build(parent_node, current_depth + 1, path_stack, cycle_detected);
                    children_json.push(child);
                }
            }
        } else {
            let children = self.graph.children_of(&current.id);
            for child_id in children {
                if path_stack.contains(child_id) {
                    *cycle_detected = true;
                    continue;
                }
                if let Some(child_node) = self.graph.get_node(child_id) {
                    let child =
                        self.build(child_node, current_depth + 1, path_stack, cycle_detected);
                    children_json.push(child);
                }
            }

            for link in self.graph.links_for(&current.id) {
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
            layer: self
                .layers
                .name_for_layer(current.layer)
                .unwrap_or(current.layer.as_str())
                .to_string(),
            parents: current.parents.iter().map(|p| p.to_string()).collect(),
            children: children_json,
            links: links_json,
        }
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
