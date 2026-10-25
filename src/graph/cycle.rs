//! 3-Color DFS cycle detection and circular dependency diagnostics.
//!
//! // @trace SPEC-DAG-CYCLE, SPEC-PERF-BOUNDS, REQ-NFR-PERF, REQ-CYCLE-DETECT, REQ-EXIT-CODES, US-CYCLE-01

use std::collections::BTreeMap;
use std::fmt;

use crate::error::GraphError;
use crate::graph::model::TraceabilityGraph;
use crate::model::NodeId;

/// State of a graph node during 3-color DFS traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeColor {
    /// Node has not been visited yet.
    White,
    /// Node is currently on the recursion call stack (active path).
    Gray,
    /// Node and all its reachable descendants have been fully explored.
    Black,
}

/// Represents a circular dependency loop chain detected in the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    /// Ordered sequence of node identifiers forming the cycle loop:
    /// `[v, ..., u, v]` where the last node equals the first node.
    pub nodes: Vec<NodeId>,
}

impl Cycle {
    /// Creates a new `Cycle` instance from a path slice and closing target node.
    pub fn new(mut path: Vec<NodeId>, back_target: NodeId) -> Self {
        path.push(back_target);
        Self { nodes: path }
    }

    /// Formats the cycle chain as a clean arrow-delimited string:
    /// `A -> B -> C -> A`.
    pub fn format_chain(&self) -> String {
        self.nodes
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Formats the cycle chain with file origin provenance:
    /// `A (docs/req.md:12) -> B (docs/req.md:45) -> A (docs/req.md:12)`.
    pub fn format_with_provenance(&self, graph: &TraceabilityGraph) -> String {
        self.nodes
            .iter()
            .map(|id| {
                if let Some(node) = graph.get_node(id) {
                    format!("{} ({})", id.as_str(), node.file_origin())
                } else {
                    id.as_str().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Returns the length of the cycle (number of hops, including the return edge).
    #[inline]
    pub fn len(&self) -> usize {
        if self.nodes.is_empty() {
            0
        } else {
            self.nodes.len().saturating_sub(1)
        }
    }

    /// Returns true if this cycle is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns true if this cycle is a self-referential loop (`A -> A`).
    #[inline]
    pub fn is_self_loop(&self) -> bool {
        self.nodes.len() == 2 && self.nodes.first() == self.nodes.last()
    }

    /// Returns the nodes slice.
    #[inline]
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }
}

impl fmt::Display for Cycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_chain())
    }
}

impl From<Cycle> for GraphError {
    fn from(cycle: Cycle) -> Self {
        GraphError::CycleDetected {
            cycle_chain: cycle.format_chain(),
        }
    }
}

/// 3-color Depth First Search cycle detector.
pub struct CycleDetector<'a> {
    graph: &'a TraceabilityGraph,
    colors: BTreeMap<NodeId, NodeColor>,
    call_stack: Vec<NodeId>,
    cycles: Vec<Cycle>,
}

impl<'a> CycleDetector<'a> {
    /// Creates a new `CycleDetector` initialized with the given graph.
    pub fn new(graph: &'a TraceabilityGraph) -> Self {
        Self {
            graph,
            colors: BTreeMap::new(),
            call_stack: Vec::new(),
            cycles: Vec::new(),
        }
    }

    /// Detects if any cycle exists in the graph.
    ///
    /// Returns `Ok(())` if the graph is a valid DAG.
    /// Returns `Err(GraphError::CycleDetected)` with the formatted cycle chain
    /// of the first deterministic cycle found.
    pub fn detect_cycles(&mut self) -> Result<(), GraphError> {
        for node_id in self.graph.all_node_ids() {
            if self.get_color(node_id) == NodeColor::White {
                if let Some(cycle) = self.dfs_find_first(node_id) {
                    return Err(GraphError::from(cycle));
                }
            }
        }

        Ok(())
    }

    /// Finds and collects all independent cycles across all graph components.
    pub fn find_all_cycles(&mut self) -> Vec<Cycle> {
        self.colors.clear();
        self.call_stack.clear();
        self.cycles.clear();

        for node_id in self.graph.all_node_ids() {
            if self.get_color(node_id) == NodeColor::White {
                self.dfs_collect_all(node_id);
            }
        }

        self.cycles.clone()
    }

    /// Gets the color of a node, defaulting to `White` if unrecorded.
    #[inline]
    fn get_color(&self, id: &NodeId) -> NodeColor {
        self.colors.get(id).copied().unwrap_or(NodeColor::White)
    }

    /// Recursive DFS returning immediately upon finding the first cycle.
    fn dfs_find_first(&mut self, current: &NodeId) -> Option<Cycle> {
        self.colors.insert(current.clone(), NodeColor::Gray);
        self.call_stack.push(current.clone());

        for neighbor in self.graph.children_of(current) {
            // Missing nodes that are broken parent references are skipped
            if !self.graph.has_node(neighbor) {
                continue;
            }

            match self.get_color(neighbor) {
                NodeColor::White => {
                    if let Some(cycle) = self.dfs_find_first(neighbor) {
                        return Some(cycle);
                    }
                }
                NodeColor::Gray => {
                    // Back-edge detected! Reconstruct cycle from call_stack
                    if let Some(pos) = self.call_stack.iter().position(|id| id == neighbor) {
                        let path = self.call_stack[pos..].to_vec();
                        return Some(Cycle::new(path, neighbor.clone()));
                    }
                }
                NodeColor::Black => {
                    // Cross-edge / diamond DAG: already verified acyclic
                }
            }
        }

        self.call_stack.pop();
        self.colors.insert(current.clone(), NodeColor::Black);
        None
    }

    /// Recursive DFS collecting all independent cycles without infinite looping.
    fn dfs_collect_all(&mut self, current: &NodeId) {
        self.colors.insert(current.clone(), NodeColor::Gray);
        self.call_stack.push(current.clone());

        for neighbor in self.graph.children_of(current) {
            if !self.graph.has_node(neighbor) {
                continue;
            }

            match self.get_color(neighbor) {
                NodeColor::White => {
                    self.dfs_collect_all(neighbor);
                }
                NodeColor::Gray => {
                    // Back-edge detected! Reconstruct cycle without recursing on neighbor
                    if let Some(pos) = self.call_stack.iter().position(|id| id == neighbor) {
                        let path = self.call_stack[pos..].to_vec();
                        let cycle = Cycle::new(path, neighbor.clone());
                        self.cycles.push(cycle);
                    }
                }
                NodeColor::Black => {
                    // Diamond DAG / cross-edge: acyclic
                }
            }
        }

        self.call_stack.pop();
        self.colors.insert(current.clone(), NodeColor::Black);
    }
}

/// Detects whether any circular dependencies exist in `graph`.
///
/// Returns `Ok(())` if the graph is a strict DAG.
/// Returns `Err(GraphError::CycleDetected)` on the first cycle discovered.
pub fn detect_cycles(graph: &TraceabilityGraph) -> Result<(), GraphError> {
    let mut detector = CycleDetector::new(graph);
    detector.detect_cycles()
}

/// Finds and returns all distinct cycles in `graph`.
pub fn find_all_cycles(graph: &TraceabilityGraph) -> Vec<Cycle> {
    let mut detector = CycleDetector::new(graph);
    detector.find_all_cycles()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileOrigin, Layer, NodeId, TraceLink, TraceNode};
    use std::path::PathBuf;

    fn make_origin(path: &str, line: usize) -> FileOrigin {
        FileOrigin::new(PathBuf::from(path), line)
    }

    fn make_id(s: &str) -> NodeId {
        match NodeId::new(s) {
            Ok(id) => id,
            Err(err) => panic!("Invalid test NodeId '{s}': {err}"),
        }
    }

    fn build_graph(nodes: Vec<TraceNode>, links: Vec<TraceLink>) -> TraceabilityGraph {
        match TraceabilityGraph::build(nodes, links) {
            Ok(g) => g,
            Err(e) => panic!("build failed: {:?}", e),
        }
    }

    fn make_node(id: &str, layer: Layer, parents: Vec<&str>) -> TraceNode {
        let parent_ids: Vec<NodeId> = parents.into_iter().map(make_id).collect();
        TraceNode::new(
            make_id(id),
            format!("Title {}", id),
            layer,
            parent_ids,
            make_origin("docs/spec.md", 10),
            "Description",
        )
    }

    fn make_node_with_origin(
        id: &str,
        layer: Layer,
        parents: Vec<&str>,
        path: &str,
        line: usize,
    ) -> TraceNode {
        let parent_ids: Vec<NodeId> = parents.into_iter().map(make_id).collect();
        TraceNode::new(
            make_id(id),
            format!("Title {}", id),
            layer,
            parent_ids,
            make_origin(path, line),
            "Description",
        )
    }

    #[test]
    fn test_cycle_empty_graph() {
        let graph = build_graph(vec![], vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_single_node() {
        let nodes = vec![make_node("VIS-01", Layer::Vision, vec![])];
        let graph = build_graph(nodes, vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_linear_chain() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
            make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
            make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
            make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
            make_node("SPEC-01", Layer::TechnicalSpec, vec!["REQ-01"]),
        ];
        let graph = build_graph(nodes, vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_branching_tree() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("STRAT-A", Layer::Strategy, vec!["VIS-01"]),
            make_node("STRAT-B", Layer::Strategy, vec!["VIS-01"]),
            make_node("EPIC-A1", Layer::Epic, vec!["STRAT-A"]),
            make_node("EPIC-A2", Layer::Epic, vec!["STRAT-A"]),
            make_node("EPIC-B1", Layer::Epic, vec!["STRAT-B"]),
        ];
        let graph = build_graph(nodes, vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_convergent_multi_parent() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
            make_node("STRAT-02", Layer::Strategy, vec!["VIS-01"]),
            make_node("EPIC-01", Layer::Epic, vec!["STRAT-01", "STRAT-02"]),
        ];
        let graph = build_graph(nodes, vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_diamond_dag() {
        let nodes = vec![
            make_node("VIS-TOP", Layer::Vision, vec![]),
            make_node("STRAT-L", Layer::Strategy, vec!["VIS-TOP"]),
            make_node("STRAT-R", Layer::Strategy, vec!["VIS-TOP"]),
            make_node("EPIC-BOT", Layer::Epic, vec!["STRAT-L", "STRAT-R"]),
        ];
        let graph = build_graph(nodes, vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_lattice_dag() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("STRAT-1", Layer::Strategy, vec!["VIS-01"]),
            make_node("STRAT-2", Layer::Strategy, vec!["VIS-01"]),
            make_node("EPIC-1", Layer::Epic, vec!["STRAT-1", "STRAT-2"]),
            make_node("EPIC-2", Layer::Epic, vec!["STRAT-1", "STRAT-2"]),
            make_node("US-1", Layer::UserStory, vec!["EPIC-1", "EPIC-2"]),
        ];
        let graph = build_graph(nodes, vec![]);
        assert!(detect_cycles(&graph).is_ok());
        assert_eq!(find_all_cycles(&graph).len(), 0);
    }

    #[test]
    fn test_cycle_self_referential_loop() {
        let nodes = vec![make_node("VIS-SELF", Layer::Vision, vec!["VIS-SELF"])];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
        match res {
            Err(GraphError::CycleDetected { cycle_chain }) => {
                assert_eq!(cycle_chain, "VIS-SELF -> VIS-SELF");
            }
            other => panic!("Expected CycleDetected, got {:?}", other),
        }

        let cycles = find_all_cycles(&graph);
        assert_eq!(cycles.len(), 1);
        assert!(cycles[0].is_self_loop());
    }

    #[test]
    fn test_cycle_direct_2_node() {
        let nodes = vec![
            make_node("VIS-A", Layer::Vision, vec!["VIS-B"]),
            make_node("VIS-B", Layer::Vision, vec!["VIS-A"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
        match res {
            Err(GraphError::CycleDetected { cycle_chain }) => {
                assert!(
                    cycle_chain == "VIS-A -> VIS-B -> VIS-A"
                        || cycle_chain == "VIS-B -> VIS-A -> VIS-B",
                    "Unexpected chain: {}",
                    cycle_chain
                );
            }
            other => panic!("Expected CycleDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_cycle_indirect_3_node() {
        let nodes = vec![
            make_node("VIS-A", Layer::Vision, vec!["VIS-C"]),
            make_node("VIS-B", Layer::Vision, vec!["VIS-A"]),
            make_node("VIS-C", Layer::Vision, vec!["VIS-B"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
        match res {
            Err(GraphError::CycleDetected { cycle_chain }) => {
                assert!(cycle_chain.contains("->"), "Chain should contain arrows");
                let last_part = match cycle_chain.split(" -> ").last() {
                    Some(p) => p,
                    None => panic!("empty chain"),
                };
                assert!(cycle_chain.starts_with(last_part));
            }
            other => panic!("Expected CycleDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_cycle_large_ring() {
        let mut nodes = Vec::with_capacity(10);
        for i in 0..10 {
            let id = format!("NODE-{:02}", i);
            let prev = if i == 0 {
                "NODE-09".to_string()
            } else {
                format!("NODE-{:02}", i - 1)
            };
            nodes.push(make_node(&id, Layer::Vision, vec![&prev]));
        }
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
        let cycles = find_all_cycles(&graph);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 10);
    }

    #[test]
    fn test_cycle_with_entry_prefix() {
        let nodes = vec![
            make_node("VIS-ROOT", Layer::Vision, vec![]),
            make_node("STRAT-X", Layer::Strategy, vec!["VIS-ROOT"]),
            make_node("EPIC-A", Layer::Epic, vec!["STRAT-X", "EPIC-C"]),
            make_node("EPIC-B", Layer::Epic, vec!["EPIC-A"]),
            make_node("EPIC-C", Layer::Epic, vec!["EPIC-B"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
        match res {
            Err(GraphError::CycleDetected { cycle_chain }) => {
                assert!(
                    !cycle_chain.contains("VIS-ROOT"),
                    "Prefix VIS-ROOT must be trimmed: {}",
                    cycle_chain
                );
                assert!(
                    !cycle_chain.contains("STRAT-X"),
                    "Prefix STRAT-X must be trimmed: {}",
                    cycle_chain
                );
                assert!(cycle_chain.contains("EPIC-A"));
                assert!(cycle_chain.contains("EPIC-B"));
                assert!(cycle_chain.contains("EPIC-C"));
            }
            other => panic!("Expected CycleDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_cycle_nested_in_dag() {
        let nodes = vec![
            make_node("VIS-ROOT", Layer::Vision, vec![]),
            make_node("STRAT-VALID", Layer::Strategy, vec!["VIS-ROOT"]),
            make_node("EPIC-1", Layer::Epic, vec!["STRAT-VALID", "EPIC-2"]),
            make_node("EPIC-2", Layer::Epic, vec!["EPIC-1"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
    }

    #[test]
    fn test_cycle_multiple_independent() {
        let nodes = vec![
            make_node("VIS-A1", Layer::Vision, vec!["VIS-B1"]),
            make_node("VIS-B1", Layer::Vision, vec!["VIS-A1"]),
            make_node("VIS-A2", Layer::Vision, vec!["VIS-B2"]),
            make_node("VIS-B2", Layer::Vision, vec!["VIS-A2"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let cycles = find_all_cycles(&graph);
        assert_eq!(cycles.len(), 2);
    }

    #[test]
    fn test_cycle_disconnected_component() {
        let nodes = vec![
            make_node("VIS-ROOT", Layer::Vision, vec![]),
            make_node("STRAT-01", Layer::Strategy, vec!["VIS-ROOT"]),
            make_node("VIS-X1", Layer::Vision, vec!["VIS-X2"]),
            make_node("VIS-X2", Layer::Vision, vec!["VIS-X1"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
    }

    #[test]
    fn test_cycle_cross_layer() {
        let nodes = vec![
            make_node("VIS-A", Layer::Vision, vec!["REQ-B"]),
            make_node("REQ-B", Layer::Requirement, vec!["VIS-A"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let res = detect_cycles(&graph);
        assert!(res.is_err());
    }

    #[test]
    fn test_cycle_determinism() {
        let nodes = vec![
            make_node("VIS-Z1", Layer::Vision, vec!["VIS-Z2"]),
            make_node("VIS-Z2", Layer::Vision, vec!["VIS-Z1"]),
            make_node("VIS-A1", Layer::Vision, vec!["VIS-A2"]),
            make_node("VIS-A2", Layer::Vision, vec!["VIS-A1"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let first = match detect_cycles(&graph) {
            Ok(_) => panic!("cycle expected"),
            Err(e) => e.to_string(),
        };
        for _ in 0..50 {
            let next = match detect_cycles(&graph) {
                Ok(_) => panic!("cycle expected"),
                Err(e) => e.to_string(),
            };
            assert_eq!(first, next);
        }
    }

    #[test]
    fn test_cycle_provenance_formatting() {
        let nodes = vec![
            make_node_with_origin("VIS-A", Layer::Vision, vec!["VIS-B"], "docs/a.md", 12),
            make_node_with_origin("VIS-B", Layer::Vision, vec!["VIS-A"], "docs/b.md", 45),
        ];
        let graph = build_graph(nodes, vec![]);

        let cycles = find_all_cycles(&graph);
        assert_eq!(cycles.len(), 1);
        let formatted = cycles[0].format_with_provenance(&graph);
        assert!(formatted.contains("docs/a.md:12"));
        assert!(formatted.contains("docs/b.md:45"));
    }

    #[test]
    fn test_cycle_missing_target_resilience() {
        let nodes = vec![
            make_node("VIS-A", Layer::Vision, vec!["VIS-MISSING"]),
            make_node("VIS-B", Layer::Vision, vec![]),
        ];
        let graph = build_graph(nodes, vec![]);

        assert!(detect_cycles(&graph).is_ok());
    }
}
