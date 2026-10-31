//! Integration tests for 7-Layer Adjacency Graph Storage & Model.
//!
//! // @trace:test SPEC-DAG-STORAGE, REQ-DAG-MODEL, REQ-NFR-ROBUST, REQ-EXIT-CODES

use std::path::PathBuf;
use xtrc::error::GraphError;
use xtrc::graph::model::TraceabilityGraph;
use xtrc::model::{FileOrigin, Layer, LinkKind, NodeId, TraceLink, TraceNode};

fn make_origin(path: &str, line: usize) -> FileOrigin {
    FileOrigin::new(PathBuf::from(path), line)
}

fn make_node(id: &str, layer: Layer, parents: Vec<&str>) -> TraceNode {
    TraceNode::new(
        NodeId::new(id).expect("valid ID"),
        format!("Title {}", id),
        layer,
        parents
            .into_iter()
            .map(|p| NodeId::new(p).expect("valid parent ID"))
            .collect(),
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
    TraceNode::new(
        NodeId::new(id).expect("valid ID"),
        format!("Title {}", id),
        layer,
        parents
            .into_iter()
            .map(|p| NodeId::new(p).expect("valid parent ID"))
            .collect(),
        make_origin(path, line),
        "Description",
    )
}

fn make_link(target: &str, file: &str, line: usize, kind: LinkKind) -> TraceLink {
    TraceLink::new(NodeId::new(target).expect("valid target"), file, line, kind)
}

#[test]
fn test_empty_graph_construction() {
    let graph = TraceabilityGraph::build(vec![], vec![]).expect("empty build succeeds");
    assert!(graph.is_empty());
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.all_nodes().count(), 0);
    assert_eq!(graph.all_links().len(), 0);
}

#[test]
fn test_single_node_ingestion_and_queries() {
    let node = make_node("VIS-ROOT", Layer::Vision, vec![]);
    let id = NodeId::new("VIS-ROOT").expect("valid id");
    let graph = TraceabilityGraph::build(vec![node], vec![]).expect("build succeeds");

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

    let err = TraceabilityGraph::build(vec![node1, node2], vec![]).expect_err("must fail");
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
    let node1 = make_node_with_origin("REQ-AUTH-01", Layer::Requirement, vec![], "docs/a.md", 5);
    let node2 = make_node_with_origin("REQ-AUTH-01", Layer::Requirement, vec![], "docs/b.md", 15);

    let err = TraceabilityGraph::build(vec![node1, node2], vec![]).expect_err("must fail");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build succeeds");

    let vis = NodeId::new("VIS-01").expect("id");
    let strat = NodeId::new("STRAT-01").expect("id");
    let epic = NodeId::new("EPIC-01").expect("id");
    let us = NodeId::new("US-01").expect("id");
    let req = NodeId::new("REQ-01").expect("id");
    let spec = NodeId::new("SPEC-01").expect("id");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build succeeds");

    let join = NodeId::new("STRAT-JOIN").expect("id");
    let one = NodeId::new("VIS-ONE").expect("id");
    let two = NodeId::new("VIS-TWO").expect("id");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build succeeds");

    let hub = NodeId::new("VIS-HUB").expect("id");
    let a = NodeId::new("STRAT-A").expect("id");
    let b = NodeId::new("STRAT-B").expect("id");
    let c = NodeId::new("STRAT-C").expect("id");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build succeeds");

    let top = NodeId::new("VIS-TOP").expect("id");
    let l = NodeId::new("STRAT-L").expect("id");
    let r = NodeId::new("STRAT-R").expect("id");
    let bot = NodeId::new("EPIC-BOT").expect("id");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build succeeds");

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
    let graph = TraceabilityGraph::build(nodes, links).expect("build succeeds");

    let id = NodeId::new("REQ-01").expect("id");
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
    let graph = TraceabilityGraph::build(nodes, links).expect("build succeeds");

    let id = NodeId::new("REQ-01").expect("id");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build must not fail");

    let id = NodeId::new("REQ-01").expect("id");
    let missing = NodeId::new("US-MISSING").expect("id");
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
    let graph = TraceabilityGraph::build(vec![], links).expect("build must not fail");

    let missing = NodeId::new("REQ-MISSING").expect("id");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build succeeds");

    let vis = NodeId::new("VIS-01").expect("id");
    let spec = NodeId::new("SPEC-01").expect("id");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("large build succeeds");
    let elapsed = start.elapsed();

    assert_eq!(graph.node_count(), 1000);
    assert!(elapsed.as_millis() < 500, "took {:?}", elapsed);
}
