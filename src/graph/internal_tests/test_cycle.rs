//! Integration tests for 3-Color DFS Cycle Detection.
//!
//! // @trace:test SPEC-DAG-CYCLE, REQ-CYCLE-DETECT, REQ-EXIT-CODES, US-CYCLE-01

use std::path::PathBuf;
use xtrc::error::GraphError;
use xtrc::graph::cycle::{detect_cycles, find_all_cycles};
use xtrc::graph::model::TraceabilityGraph;
use xtrc::model::{FileOrigin, Layer, NodeId, TraceNode};

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

#[test]
fn test_cycle_empty_graph() {
    let graph = TraceabilityGraph::build(vec![], vec![]).expect("build graph");
    assert!(detect_cycles(&graph).is_ok());
    assert_eq!(find_all_cycles(&graph).len(), 0);
}

#[test]
fn test_cycle_single_node() {
    let nodes = vec![make_node("VIS-01", Layer::Vision, vec![])];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
    assert!(detect_cycles(&graph).is_ok());
    assert_eq!(find_all_cycles(&graph).len(), 0);
}

#[test]
fn test_cycle_self_referential_loop() {
    let nodes = vec![make_node("VIS-SELF", Layer::Vision, vec!["VIS-SELF"])];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

    let res = detect_cycles(&graph);
    assert!(res.is_err());
    match res {
        Err(GraphError::CycleDetected { cycle_chain }) => {
            assert!(cycle_chain.contains("->"), "Chain should contain arrows");
            assert!(cycle_chain.starts_with(cycle_chain.split(" -> ").last().unwrap_or("")));
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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

    let res = detect_cycles(&graph);
    assert!(res.is_err());
}

#[test]
fn test_cycle_cross_layer() {
    let nodes = vec![
        make_node("VIS-A", Layer::Vision, vec!["REQ-B"]),
        make_node("REQ-B", Layer::Requirement, vec!["VIS-A"]),
    ];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

    let first = detect_cycles(&graph)
        .expect_err("cycle expected")
        .to_string();
    for _ in 0..50 {
        let next = detect_cycles(&graph)
            .expect_err("cycle expected")
            .to_string();
        assert_eq!(first, next);
    }
}

#[test]
fn test_cycle_provenance_formatting() {
    let nodes = vec![
        make_node_with_origin("VIS-A", Layer::Vision, vec!["VIS-B"], "docs/a.md", 12),
        make_node_with_origin("VIS-B", Layer::Vision, vec!["VIS-A"], "docs/b.md", 45),
    ];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

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
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

    assert!(detect_cycles(&graph).is_ok());
}
