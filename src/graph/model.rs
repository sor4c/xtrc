//! In-memory 7-layer Directed Acyclic Graph (DAG) data structure and queries.
//!
//! // @trace SPEC-DAG-STORAGE, SPEC-PERF-BOUNDS, REQ-DAG-MODEL, REQ-NFR-DETERM, REQ-NFR-ROBUST, REQ-EXIT-CODES, US-PERF-01

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::error::GraphError;
use crate::graph::cycle::{self, Cycle};
#[cfg(test)]
use crate::graph::validator::{self, ValidationOptions, ValidationReport};
use crate::model::{Layer, LinkKind, NodeId, TraceLink, TraceNode};

/// An in-memory 7-layer Directed Acyclic Graph tracking bidirectional relationships
/// across Vision, Strategy, Epic, UserStory, Requirement, TechnicalSpec, and Code/Test links.
///
/// Iteration and queries are guaranteed 100% deterministic via `BTreeMap` ordering.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TraceabilityGraph {
    /// Ingested specification nodes (Layers 1-6), indexed by `NodeId`.
    nodes: BTreeMap<NodeId, TraceNode>,

    /// Forward decomposition edges: parent -> list of child NodeIds (sorted).
    forward_edges: BTreeMap<NodeId, Vec<NodeId>>,

    /// Backward justification edges: child -> list of parent NodeIds (sorted).
    backward_edges: BTreeMap<NodeId, Vec<NodeId>>,

    /// Leaf code/test link attachments: target NodeId -> list of TraceLinks (sorted by path & line).
    links: BTreeMap<NodeId, Vec<TraceLink>>,

    /// Complete list of all ingested links (deterministic order).
    all_links: Vec<TraceLink>,
}

impl TraceabilityGraph {
    /// Creates a new empty `TraceabilityGraph`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs a new `TraceabilityGraph` from parsed nodes and scanned comment links.
    ///
    /// # Errors
    /// Returns `Err(GraphError::DuplicateIdentifier)` if two nodes declare the same `NodeId`,
    /// including exact file and line provenance for both occurrences.
    pub fn build(nodes: Vec<TraceNode>, links: Vec<TraceLink>) -> Result<Self, GraphError> {
        let mut node_map: BTreeMap<NodeId, TraceNode> = BTreeMap::new();

        // 1. Ingest nodes & enforce unique identifiers
        for node in nodes {
            if let Some(existing) = node_map.get(&node.id) {
                return Err(GraphError::DuplicateIdentifier {
                    id: node.id.to_string(),
                    path_a: existing.file_origin.path.clone(),
                    line_a: existing.file_origin.line,
                    path_b: node.file_origin.path.clone(),
                    line_b: node.file_origin.line,
                });
            }
            node_map.insert(node.id.clone(), node);
        }

        // 2. Build forward & backward adjacency maps using BTreeSet for deduplication and sorting
        let mut forward_temp: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        let mut backward_temp: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();

        // Ensure every registered node has an entry in both adjacency maps
        for id in node_map.keys() {
            forward_temp.entry(id.clone()).or_default();
            backward_temp.entry(id.clone()).or_default();
        }

        for node in node_map.values() {
            let parent_set = backward_temp.entry(node.id.clone()).or_default();
            for parent_id in &node.parents {
                parent_set.insert(parent_id.clone());
                forward_temp
                    .entry(parent_id.clone())
                    .or_default()
                    .insert(node.id.clone());
            }
        }

        // Convert sets to sorted vectors for O(1) slice access
        let forward_edges: BTreeMap<NodeId, Vec<NodeId>> = forward_temp
            .into_iter()
            .map(|(k, set)| (k, set.into_iter().collect()))
            .collect();

        let backward_edges: BTreeMap<NodeId, Vec<NodeId>> = backward_temp
            .into_iter()
            .map(|(k, set)| (k, set.into_iter().collect()))
            .collect();

        // 3. Ingest and sort leaf comment links
        let mut links_map: BTreeMap<NodeId, Vec<TraceLink>> = BTreeMap::new();
        for link in links {
            links_map
                .entry(link.target_id.clone())
                .or_default()
                .push(link);
        }

        for link_vec in links_map.values_mut() {
            link_vec.sort_by(|a, b| {
                a.source_file
                    .cmp(&b.source_file)
                    .then_with(|| a.line.cmp(&b.line))
                    .then_with(|| (a.kind as u8).cmp(&(b.kind as u8)))
            });
            link_vec.dedup();
        }

        let all_links: Vec<TraceLink> = links_map.values().flatten().cloned().collect();

        Ok(Self {
            nodes: node_map,
            forward_edges,
            backward_edges,
            links: links_map,
            all_links,
        })
    }

    /// Looks up a node by its identifier.
    #[inline]
    pub fn get_node(&self, id: &NodeId) -> Option<&TraceNode> {
        self.nodes.get(id)
    }

    /// Returns `true` if the graph contains a node with the given identifier.
    #[inline]
    pub fn has_node(&self, id: &NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    /// Returns the number of specification nodes in the graph.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if the graph contains no specification nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns an iterator over all specification nodes in deterministic order.
    #[inline]
    pub fn all_nodes(&self) -> impl Iterator<Item = &TraceNode> {
        self.nodes.values()
    }

    /// Returns an iterator over all node identifiers in deterministic order.
    #[inline]
    pub fn all_node_ids(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.keys()
    }

    /// Returns a borrowed slice of direct parent identifiers for `id`.
    ///
    /// If `id` has no parents or is not found, returns an empty slice `&[]`.
    #[inline]
    pub fn parents_of(&self, id: &NodeId) -> &[NodeId] {
        self.backward_edges.get(id).map_or(&[], |v| v.as_slice())
    }

    /// Returns a borrowed slice of direct child identifiers for `id`.
    ///
    /// If `id` has no children or is not found, returns an empty slice `&[]`.
    #[inline]
    pub fn children_of(&self, id: &NodeId) -> &[NodeId] {
        self.forward_edges.get(id).map_or(&[], |v| v.as_slice())
    }

    /// Returns a borrowed slice of leaf comment links pointing to `id`.
    ///
    /// If `id` has no links or is not found, returns an empty slice `&[]`.
    #[inline]
    pub fn links_for(&self, id: &NodeId) -> &[TraceLink] {
        self.links.get(id).map_or(&[], |v| v.as_slice())
    }

    /// Returns all source code implementation links (`@trace` / `@trace:code`) for `id`.
    pub fn code_links_for(&self, id: &NodeId) -> Vec<&TraceLink> {
        self.links_for(id)
            .iter()
            .filter(|link| link.kind == LinkKind::SourceCode)
            .collect()
    }

    /// Returns all automated test verification links (`@trace:test`) for `id`.
    pub fn test_links_for(&self, id: &NodeId) -> Vec<&TraceLink> {
        self.links_for(id)
            .iter()
            .filter(|link| link.kind == LinkKind::AutomatedTest)
            .collect()
    }

    /// Returns `true` if `id` has at least one source code implementation link.
    pub fn has_code_link(&self, id: &NodeId) -> bool {
        self.links_for(id)
            .iter()
            .any(|link| link.kind == LinkKind::SourceCode)
    }

    /// Returns `true` if `id` has at least one automated test verification link.
    pub fn has_test_link(&self, id: &NodeId) -> bool {
        self.links_for(id)
            .iter()
            .any(|link| link.kind == LinkKind::AutomatedTest)
    }

    /// Returns all specification nodes residing in the specified `Layer`, in deterministic order.
    pub fn nodes_by_layer(&self, layer: Layer) -> Vec<&TraceNode> {
        self.nodes
            .values()
            .filter(|node| node.layer == layer)
            .collect()
    }

    /// Returns a borrowed slice of all ingested leaf links across the workspace.
    #[inline]
    pub fn all_links(&self) -> &[TraceLink] {
        &self.all_links
    }

    /// Returns all root nodes in the graph (all `Layer::Vision` nodes, or nodes with zero parents).
    pub fn roots(&self) -> Vec<&TraceNode> {
        self.nodes
            .values()
            .filter(|node| node.layer.is_root() || self.parents_of(&node.id).is_empty())
            .collect()
    }

    /// Returns all leaf specification nodes with no downward children.
    pub fn leaves(&self) -> Vec<&TraceNode> {
        self.nodes
            .values()
            .filter(|node| self.children_of(&node.id).is_empty())
            .collect()
    }

    /// Resolves direct parents of `id` into borrowed `&TraceNode` references.
    ///
    /// Silently skips any parent IDs that do not exist in the graph (broken links).
    pub fn parent_nodes_of(&self, id: &NodeId) -> Vec<&TraceNode> {
        self.parents_of(id)
            .iter()
            .filter_map(|parent_id| self.get_node(parent_id))
            .collect()
    }

    /// Resolves direct children of `id` into borrowed `&TraceNode` references.
    ///
    /// Silently skips any child IDs that do not exist in the graph.
    pub fn child_nodes_of(&self, id: &NodeId) -> Vec<&TraceNode> {
        self.children_of(id)
            .iter()
            .filter_map(|child_id| self.get_node(child_id))
            .collect()
    }

    /// Returns all transitive ancestor identifiers of `id` (upward justification traversal).
    pub fn ancestors_of(&self, id: &NodeId) -> BTreeSet<NodeId> {
        let mut ancestors = BTreeSet::new();
        let mut queue = VecDeque::new();

        for parent in self.parents_of(id) {
            queue.push_back(parent.clone());
        }

        while let Some(current) = queue.pop_front() {
            if ancestors.insert(current.clone()) {
                for next_parent in self.parents_of(&current) {
                    if !ancestors.contains(next_parent) {
                        queue.push_back(next_parent.clone());
                    }
                }
            }
        }

        ancestors
    }

    /// Returns all transitive descendant identifiers of `id` (downward decomposition traversal).
    pub fn descendants_of(&self, id: &NodeId) -> BTreeSet<NodeId> {
        let mut descendants = BTreeSet::new();
        let mut queue = VecDeque::new();

        for child in self.children_of(id) {
            queue.push_back(child.clone());
        }

        while let Some(current) = queue.pop_front() {
            if descendants.insert(current.clone()) {
                for next_child in self.children_of(&current) {
                    if !descendants.contains(next_child) {
                        queue.push_back(next_child.clone());
                    }
                }
            }
        }

        descendants
    }

    /// Detects whether any circular dependencies exist in the graph.
    ///
    /// Returns `Ok(())` if the graph is a strict DAG.
    /// Returns `Err(GraphError::CycleDetected)` on the first cycle discovered.
    pub fn detect_cycles(&self) -> Result<(), GraphError> {
        cycle::detect_cycles(self)
    }

    /// Finds and returns all distinct cycles in the graph.
    pub fn find_all_cycles(&self) -> Vec<Cycle> {
        cycle::find_all_cycles(self)
    }

    /// Validates graph referential integrity, orphan nodes, forward coverage, and backward justification.
    #[cfg(test)]
    pub(crate) fn validate(&self, strict: bool) -> ValidationReport {
        validator::validate_graph(
            self,
            &ValidationOptions {
                strict,
                ..Default::default()
            },
        )
    }

    /// Validates graph with explicit options (e.g. strict, layer_filter, id_filter).
    #[cfg(test)]
    pub(crate) fn validate_with_options(&self, options: &ValidationOptions) -> ValidationReport {
        validator::validate_graph(self, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileOrigin;
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

    fn make_link(target: &str, file: &str, line: usize, kind: LinkKind) -> TraceLink {
        TraceLink::new(make_id(target), file, line, kind)
    }

    #[test]
    fn test_empty_graph_construction() {
        let graph = build_graph(vec![], vec![]);
        assert!(graph.is_empty());
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.all_nodes().count(), 0);
        assert_eq!(graph.all_links().len(), 0);
    }

    #[test]
    fn test_single_node_ingestion_and_queries() {
        let node = make_node("VIS-ROOT", Layer::Vision, vec![]);
        let id = make_id("VIS-ROOT");
        let graph = build_graph(vec![node], vec![]);

        assert!(!graph.is_empty());
        assert_eq!(graph.node_count(), 1);
        assert!(graph.has_node(&id));
        assert!(graph.get_node(&id).is_some());
        assert_eq!(graph.parents_of(&id).len(), 0);
        assert_eq!(graph.children_of(&id).len(), 0);
        assert_eq!(graph.links_for(&id).len(), 0);
        assert_eq!(graph.nodes_by_layer(Layer::Vision).len(), 1);
        assert_eq!(graph.nodes_by_layer(Layer::Requirement).len(), 0);
        assert_eq!(graph.roots().len(), 1);
        assert_eq!(graph.leaves().len(), 1);
    }

    #[test]
    fn test_duplicate_id_same_file_fails_with_exact_provenance() {
        let node1 = make_node_with_origin(
            "REQ-AUTH-01",
            Layer::Requirement,
            vec![],
            "docs/reqs.md",
            10,
        );
        let node2 = make_node_with_origin(
            "REQ-AUTH-01",
            Layer::Requirement,
            vec![],
            "docs/reqs.md",
            30,
        );

        let err = match TraceabilityGraph::build(vec![node1, node2], vec![]) {
            Ok(_) => panic!("Expected DuplicateIdentifier error"),
            Err(e) => e,
        };
        match err {
            GraphError::DuplicateIdentifier {
                id,
                path_a,
                line_a,
                path_b,
                line_b,
            } => {
                assert_eq!(id, "REQ-AUTH-01");
                assert_eq!(path_a, PathBuf::from("docs/reqs.md"));
                assert_eq!(line_a, 10);
                assert_eq!(path_b, PathBuf::from("docs/reqs.md"));
                assert_eq!(line_b, 30);
            }
            other => panic!("Expected DuplicateIdentifier, got {:?}", other),
        }
    }

    #[test]
    fn test_duplicate_id_different_files_fails() {
        let node1 =
            make_node_with_origin("REQ-AUTH-01", Layer::Requirement, vec![], "docs/a.md", 5);
        let node2 =
            make_node_with_origin("REQ-AUTH-01", Layer::Requirement, vec![], "docs/b.md", 15);

        let err = match TraceabilityGraph::build(vec![node1, node2], vec![]) {
            Ok(_) => panic!("Expected DuplicateIdentifier error"),
            Err(e) => e,
        };
        match err {
            GraphError::DuplicateIdentifier {
                id,
                path_a,
                line_a,
                path_b,
                line_b,
            } => {
                assert_eq!(id, "REQ-AUTH-01");
                assert_eq!(path_a, PathBuf::from("docs/a.md"));
                assert_eq!(line_a, 5);
                assert_eq!(path_b, PathBuf::from("docs/b.md"));
                assert_eq!(line_b, 15);
            }
            other => panic!("Expected DuplicateIdentifier, got {:?}", other),
        }
    }

    #[test]
    fn test_multi_layer_linear_hierarchy() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
            make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
            make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
            make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
            make_node("SPEC-01", Layer::TechnicalSpec, vec!["REQ-01"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let vis = make_id("VIS-01");
        let strat = make_id("STRAT-01");
        let epic = make_id("EPIC-01");
        let us = make_id("US-01");
        let req = make_id("REQ-01");
        let spec = make_id("SPEC-01");

        assert_eq!(graph.parents_of(&vis), &[]);
        assert_eq!(graph.children_of(&vis), std::slice::from_ref(&strat));

        assert_eq!(graph.parents_of(&strat), std::slice::from_ref(&vis));
        assert_eq!(graph.children_of(&strat), std::slice::from_ref(&epic));

        assert_eq!(graph.parents_of(&epic), std::slice::from_ref(&strat));
        assert_eq!(graph.children_of(&epic), std::slice::from_ref(&us));

        assert_eq!(graph.parents_of(&us), std::slice::from_ref(&epic));
        assert_eq!(graph.children_of(&us), std::slice::from_ref(&req));

        assert_eq!(graph.parents_of(&req), std::slice::from_ref(&us));
        assert_eq!(graph.children_of(&req), std::slice::from_ref(&spec));

        assert_eq!(graph.parents_of(&spec), std::slice::from_ref(&req));
        assert_eq!(graph.children_of(&spec), &[]);
    }

    #[test]
    fn test_multi_parent_join_node() {
        let nodes = vec![
            make_node("VIS-ONE", Layer::Vision, vec![]),
            make_node("VIS-TWO", Layer::Vision, vec![]),
            make_node("STRAT-JOIN", Layer::Strategy, vec!["VIS-TWO", "VIS-ONE"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let join = make_id("STRAT-JOIN");
        let one = make_id("VIS-ONE");
        let two = make_id("VIS-TWO");

        assert_eq!(graph.parents_of(&join), &[one.clone(), two.clone()]);
        assert_eq!(graph.children_of(&one), std::slice::from_ref(&join));
        assert_eq!(graph.children_of(&two), std::slice::from_ref(&join));
    }

    #[test]
    fn test_fan_out_single_parent_many_children() {
        let nodes = vec![
            make_node("VIS-HUB", Layer::Vision, vec![]),
            make_node("STRAT-C", Layer::Strategy, vec!["VIS-HUB"]),
            make_node("STRAT-A", Layer::Strategy, vec!["VIS-HUB"]),
            make_node("STRAT-B", Layer::Strategy, vec!["VIS-HUB"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let hub = make_id("VIS-HUB");
        let a = make_id("STRAT-A");
        let b = make_id("STRAT-B");
        let c = make_id("STRAT-C");

        assert_eq!(graph.children_of(&hub), &[a, b, c]);
    }

    #[test]
    fn test_diamond_dag_topology() {
        let nodes = vec![
            make_node("VIS-TOP", Layer::Vision, vec![]),
            make_node("STRAT-L", Layer::Strategy, vec!["VIS-TOP"]),
            make_node("STRAT-R", Layer::Strategy, vec!["VIS-TOP"]),
            make_node("EPIC-BOT", Layer::Epic, vec!["STRAT-L", "STRAT-R"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let top = make_id("VIS-TOP");
        let l = make_id("STRAT-L");
        let r = make_id("STRAT-R");
        let bot = make_id("EPIC-BOT");

        assert_eq!(graph.children_of(&top), &[l.clone(), r.clone()]);
        assert_eq!(graph.parents_of(&bot), &[l, r]);
    }

    #[test]
    fn test_deterministic_iteration_order() {
        let nodes = vec![
            make_node("REQ-03", Layer::Requirement, vec![]),
            make_node("REQ-01", Layer::Requirement, vec![]),
            make_node("REQ-02", Layer::Requirement, vec![]),
        ];
        let graph = build_graph(nodes, vec![]);

        let ids: Vec<String> = graph.all_nodes().map(|n| n.id.to_string()).collect();
        assert_eq!(ids, vec!["REQ-01", "REQ-02", "REQ-03"]);
    }

    #[test]
    fn test_link_attachment_and_filtering() {
        let nodes = vec![make_node("REQ-01", Layer::Requirement, vec![])];
        let links = vec![
            make_link("REQ-01", "src/core.rs", 12, LinkKind::SourceCode),
            make_link("REQ-01", "tests/core_test.rs", 45, LinkKind::AutomatedTest),
        ];
        let graph = build_graph(nodes, links);

        let id = make_id("REQ-01");
        assert_eq!(graph.links_for(&id).len(), 2);
        assert_eq!(graph.code_links_for(&id).len(), 1);
        assert_eq!(graph.test_links_for(&id).len(), 1);
        assert!(graph.has_code_link(&id));
        assert!(graph.has_test_link(&id));
    }

    #[test]
    fn test_link_sorting_and_deduplication() {
        let nodes = vec![make_node("REQ-01", Layer::Requirement, vec![])];
        let links = vec![
            make_link("REQ-01", "src/b.rs", 20, LinkKind::SourceCode),
            make_link("REQ-01", "src/a.rs", 10, LinkKind::SourceCode),
            make_link("REQ-01", "src/a.rs", 10, LinkKind::SourceCode),
            make_link("REQ-01", "src/a.rs", 5, LinkKind::SourceCode),
        ];
        let graph = build_graph(nodes, links);

        let id = make_id("REQ-01");
        let links_res = graph.links_for(&id);
        assert_eq!(links_res.len(), 3);
        assert_eq!(links_res[0].source_file, PathBuf::from("src/a.rs"));
        assert_eq!(links_res[0].line, 5);
        assert_eq!(links_res[1].source_file, PathBuf::from("src/a.rs"));
        assert_eq!(links_res[1].line, 10);
        assert_eq!(links_res[2].source_file, PathBuf::from("src/b.rs"));
        assert_eq!(links_res[2].line, 20);
    }

    #[test]
    fn test_dangling_parent_reference_stored_non_fatal() {
        let nodes = vec![make_node("REQ-01", Layer::Requirement, vec!["US-MISSING"])];
        let graph = build_graph(nodes, vec![]);

        let id = make_id("REQ-01");
        let missing = make_id("US-MISSING");
        assert_eq!(graph.parents_of(&id), std::slice::from_ref(&missing));
        assert!(!graph.has_node(&missing));
    }

    #[test]
    fn test_dangling_code_link_stored_non_fatal() {
        let links = vec![make_link(
            "REQ-MISSING",
            "src/foo.rs",
            10,
            LinkKind::SourceCode,
        )];
        let graph = build_graph(vec![], links);

        let missing = make_id("REQ-MISSING");
        assert_eq!(graph.links_for(&missing).len(), 1);
        assert_eq!(graph.all_links().len(), 1);
    }

    #[test]
    fn test_transitive_ancestors_and_descendants() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
            make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
            make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
            make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
            make_node("SPEC-01", Layer::TechnicalSpec, vec!["REQ-01"]),
        ];
        let graph = build_graph(nodes, vec![]);

        let vis = make_id("VIS-01");
        let spec = make_id("SPEC-01");

        let ancestors = graph.ancestors_of(&spec);
        assert_eq!(ancestors.len(), 5);
        assert!(ancestors.contains(&vis));

        let descendants = graph.descendants_of(&vis);
        assert_eq!(descendants.len(), 5);
        assert!(descendants.contains(&spec));
    }

    #[test]
    fn test_large_scale_chain_performance() {
        let mut nodes = Vec::with_capacity(1000);
        nodes.push(make_node("VIS-0000", Layer::Vision, vec![]));
        for i in 1..1000 {
            let id_str = format!("REQ-{:04}", i);
            let parent_str = if i == 1 {
                "VIS-0000".to_string()
            } else {
                format!("REQ-{:04}", i - 1)
            };
            nodes.push(make_node(&id_str, Layer::Requirement, vec![&parent_str]));
        }

        let start = std::time::Instant::now();
        let graph = build_graph(nodes, vec![]);
        let elapsed = start.elapsed();

        assert_eq!(graph.node_count(), 1000);
        assert!(elapsed.as_millis() < 500, "took {:?}", elapsed);
    }
}
