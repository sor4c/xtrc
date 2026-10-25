//! Adversarial and Empirical Stress Tests for Graph Model and Cycle Detection.
//!
//! // @trace:test SPEC-DAG-STORAGE, SPEC-DAG-CYCLE, REQ-DAG-MODEL, REQ-CYCLE-DETECT

use std::path::PathBuf;
use xtrc::error::GraphError;
use xtrc::graph::{detect_cycles, find_all_cycles, TraceabilityGraph};
use xtrc::model::{FileOrigin, Layer, LinkKind, NodeId, TraceLink, TraceNode};

fn make_origin(path: &str, line: usize) -> FileOrigin {
    FileOrigin::new(PathBuf::from(path), line)
}

fn make_node(id: &str, layer: Layer, parents: Vec<&str>) -> TraceNode {
    let node_id = match NodeId::new(id) {
        Ok(nid) => nid,
        Err(e) => panic!("Invalid node ID '{}': {:?}", id, e),
    };
    let parent_ids: Vec<NodeId> = parents
        .into_iter()
        .map(|p| match NodeId::new(p) {
            Ok(nid) => nid,
            Err(e) => panic!("Invalid parent ID '{}': {:?}", p, e),
        })
        .collect();

    TraceNode::new(
        node_id,
        format!("Adversarial Node {}", id),
        layer,
        parent_ids,
        make_origin("docs/adversarial.md", 10),
        "Adversarial stress test node",
    )
}

fn make_node_with_origin(
    id: &str,
    layer: Layer,
    parents: Vec<&str>,
    path: &str,
    line: usize,
) -> TraceNode {
    let node_id = match NodeId::new(id) {
        Ok(nid) => nid,
        Err(e) => panic!("Invalid node ID '{}': {:?}", id, e),
    };
    let parent_ids: Vec<NodeId> = parents
        .into_iter()
        .map(|p| match NodeId::new(p) {
            Ok(nid) => nid,
            Err(e) => panic!("Invalid parent ID '{}': {:?}", p, e),
        })
        .collect();

    TraceNode::new(
        node_id,
        format!("Adversarial Node {}", id),
        layer,
        parent_ids,
        make_origin(path, line),
        "Adversarial stress test node",
    )
}

fn make_link(target: &str, file: &str, line: usize, kind: LinkKind) -> TraceLink {
    let target_id = match NodeId::new(target) {
        Ok(nid) => nid,
        Err(e) => panic!("Invalid target ID '{}': {:?}", target, e),
    };
    TraceLink::new(target_id, file, line, kind)
}

// -----------------------------------------------------------------------------
// 1. Diamond DAGs and Complex Lattices (Zero False Positives)
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_diamond_dag_variants_zero_false_positives() {
    // Standard Diamond: A -> B, A -> C, B -> D, C -> D
    // (B has parent A, C has parent A, D has parents B and C)
    let standard_diamond = vec![
        make_node("VIS-A", Layer::Vision, vec![]),
        make_node("STRAT-B", Layer::Strategy, vec!["VIS-A"]),
        make_node("STRAT-C", Layer::Strategy, vec!["VIS-A"]),
        make_node("EPIC-D", Layer::Epic, vec!["STRAT-B", "STRAT-C"]),
    ];
    let graph = match TraceabilityGraph::build(standard_diamond, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build standard diamond graph: {:?}", e),
    };
    assert!(detect_cycles(&graph).is_ok());
    assert_eq!(find_all_cycles(&graph).len(), 0);

    // Transitive Bypass Diamond: A -> B -> C and direct A -> C
    // (B has parent A, C has parents A and B)
    let bypass_diamond = vec![
        make_node("VIS-ROOT", Layer::Vision, vec![]),
        make_node("STRAT-MID", Layer::Strategy, vec!["VIS-ROOT"]),
        make_node("EPIC-BOT", Layer::Epic, vec!["VIS-ROOT", "STRAT-MID"]),
    ];
    let graph = match TraceabilityGraph::build(bypass_diamond, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build bypass diamond graph: {:?}", e),
    };
    assert!(detect_cycles(&graph).is_ok());
    assert_eq!(find_all_cycles(&graph).len(), 0);

    // Wide Multi-Branch Diamond: 1 root -> 50 intermediate nodes -> 1 leaf
    let mut wide_diamond = Vec::with_capacity(52);
    wide_diamond.push(make_node("VIS-ROOT", Layer::Vision, vec![]));
    let mut mid_ids = Vec::with_capacity(50);
    for i in 0..50 {
        let mid_id = format!("STRAT-M{:02}", i);
        mid_ids.push(mid_id.clone());
        wide_diamond.push(make_node(&mid_id, Layer::Strategy, vec!["VIS-ROOT"]));
    }
    let mid_id_refs: Vec<&str> = mid_ids.iter().map(|s| s.as_str()).collect();
    wide_diamond.push(make_node("EPIC-LEAF", Layer::Epic, mid_id_refs));

    let graph = match TraceabilityGraph::build(wide_diamond, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build wide diamond graph: {:?}", e),
    };
    assert!(detect_cycles(&graph).is_ok());
    assert_eq!(find_all_cycles(&graph).len(), 0);
}

#[test]
fn test_adversarial_grid_lattice_dag_zero_false_positives() {
    // 8x8 Grid Lattice: 64 nodes, where node (r, c) decomposes to (r+1, c) and (r, c+1)
    // Every cell (r, c) for r>0 or c>0 has parents (r-1, c) and (r, c-1).
    // This creates hundreds of overlapping, converging directed paths without cycles.
    const ROWS: usize = 8;
    const COLS: usize = 8;
    let mut nodes = Vec::with_capacity(ROWS * COLS);

    for r in 0..ROWS {
        for c in 0..COLS {
            let id = format!("NODE-R{}C{}", r, c);
            let mut parents = Vec::new();
            let p_up = (r > 0).then(|| format!("NODE-R{}C{}", r - 1, c));
            let p_left = (c > 0).then(|| format!("NODE-R{}C{}", r, c - 1));

            if let Some(ref p) = p_up {
                parents.push(p.as_str());
            }
            if let Some(ref p) = p_left {
                parents.push(p.as_str());
            }

            nodes.push(make_node(&id, Layer::Vision, parents));
        }
    }

    let graph = match TraceabilityGraph::build(nodes, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build grid lattice graph: {:?}", e),
    };
    assert_eq!(graph.node_count(), ROWS * COLS);
    assert!(
        detect_cycles(&graph).is_ok(),
        "Grid lattice DAG must be acyclic"
    );
    assert_eq!(find_all_cycles(&graph).len(), 0);
}

// -----------------------------------------------------------------------------
// 2. Self-Referential Loops (A -> A)
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_self_referential_loop() {
    let node = make_node("VIS-SELF", Layer::Vision, vec!["VIS-SELF"]);
    let graph = match TraceabilityGraph::build(vec![node], vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build self-referential graph: {:?}", e),
    };

    let result = detect_cycles(&graph);
    assert!(result.is_err());
    match result {
        Err(GraphError::CycleDetected { cycle_chain }) => {
            assert_eq!(cycle_chain, "VIS-SELF -> VIS-SELF");
        }
        other => panic!("Expected CycleDetected, got {:?}", other),
    }

    let cycles = find_all_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert!(cycles[0].is_self_loop());
    assert_eq!(cycles[0].len(), 1);
    assert_eq!(cycles[0].format_chain(), "VIS-SELF -> VIS-SELF");
}

// -----------------------------------------------------------------------------
// 3. Exact Reconstructed Loop Strings (2-node, 3-node, 10-node cycle chains)
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_2_node_cycle_exact_reconstruction() {
    // Forward edges: NODE-A -> NODE-B and NODE-B -> NODE-A
    // NODE-B has parent NODE-A, and NODE-A has parent NODE-B.
    let nodes = vec![
        make_node("NODE-A", Layer::Vision, vec!["NODE-B"]),
        make_node("NODE-B", Layer::Vision, vec!["NODE-A"]),
    ];
    let graph = match TraceabilityGraph::build(nodes, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build 2-node cycle graph: {:?}", e),
    };

    let result = detect_cycles(&graph);
    assert!(result.is_err());
    match result {
        Err(GraphError::CycleDetected { cycle_chain }) => {
            // Because NODE-A < NODE-B alphabetically, traversal starts at NODE-A.
            // Child of NODE-A is NODE-B; child of NODE-B is NODE-A (Gray).
            // Reconstructed stack slice: NODE-A -> NODE-B -> NODE-A.
            assert_eq!(cycle_chain, "NODE-A -> NODE-B -> NODE-A");
        }
        other => panic!("Expected CycleDetected, got {:?}", other),
    }

    let cycles = find_all_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].format_chain(), "NODE-A -> NODE-B -> NODE-A");
    assert_eq!(cycles[0].len(), 2);
}

#[test]
fn test_adversarial_3_node_cycle_exact_reconstruction() {
    // Forward edges: NODE-A -> NODE-B -> NODE-C -> NODE-A
    // NODE-B has parent NODE-A
    // NODE-C has parent NODE-B
    // NODE-A has parent NODE-C
    let nodes = vec![
        make_node("NODE-A", Layer::Vision, vec!["NODE-C"]),
        make_node("NODE-B", Layer::Vision, vec!["NODE-A"]),
        make_node("NODE-C", Layer::Vision, vec!["NODE-B"]),
    ];
    let graph = match TraceabilityGraph::build(nodes, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build 3-node cycle graph: {:?}", e),
    };

    let result = detect_cycles(&graph);
    assert!(result.is_err());
    match result {
        Err(GraphError::CycleDetected { cycle_chain }) => {
            assert_eq!(cycle_chain, "NODE-A -> NODE-B -> NODE-C -> NODE-A");
        }
        other => panic!("Expected CycleDetected, got {:?}", other),
    }

    let cycles = find_all_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(
        cycles[0].format_chain(),
        "NODE-A -> NODE-B -> NODE-C -> NODE-A"
    );
    assert_eq!(cycles[0].len(), 3);
}

#[test]
fn test_adversarial_10_node_cycle_exact_reconstruction() {
    // 10 nodes: NODE-00 -> NODE-01 -> ... -> NODE-09 -> NODE-00
    // Node i has parent (i - 1 + 10) % 10.
    // Forward decomposition edges are (i - 1 + 10) % 10 -> i.
    // Specifically:
    // NODE-00 parent is NODE-09 (so edge NODE-09 -> NODE-00)
    // NODE-01 parent is NODE-00 (so edge NODE-00 -> NODE-01)
    // ...
    // NODE-09 parent is NODE-08 (so edge NODE-08 -> NODE-09)
    let mut nodes = Vec::with_capacity(10);
    for i in 0..10 {
        let id = format!("NODE-{:02}", i);
        let parent = if i == 0 {
            "NODE-09".to_string()
        } else {
            format!("NODE-{:02}", i - 1)
        };
        nodes.push(make_node(&id, Layer::Vision, vec![&parent]));
    }

    let graph = match TraceabilityGraph::build(nodes, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build 10-node cycle graph: {:?}", e),
    };

    let expected_chain = "NODE-00 -> NODE-01 -> NODE-02 -> NODE-03 -> NODE-04 -> NODE-05 -> NODE-06 -> NODE-07 -> NODE-08 -> NODE-09 -> NODE-00";

    let result = detect_cycles(&graph);
    assert!(result.is_err());
    match result {
        Err(GraphError::CycleDetected { cycle_chain }) => {
            assert_eq!(cycle_chain, expected_chain);
        }
        other => panic!("Expected CycleDetected, got {:?}", other),
    }

    let cycles = find_all_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].format_chain(), expected_chain);
    assert_eq!(cycles[0].len(), 10);
}

// -----------------------------------------------------------------------------
// 4. Multi-Cycle Graph Determinism
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_multi_cycle_determinism_across_shuffled_inputs() {
    // Graph containing 3 distinct cycles:
    // Cycle A: CYC-A1 -> CYC-A2 -> CYC-A1
    // Cycle M: CYC-M1 -> CYC-M2 -> CYC-M3 -> CYC-M1
    // Cycle Z: CYC-Z1 -> CYC-Z2 -> CYC-Z1
    let make_nodes = || {
        vec![
            make_node("CYC-A1", Layer::Vision, vec!["CYC-A2"]),
            make_node("CYC-A2", Layer::Vision, vec!["CYC-A1"]),
            make_node("CYC-M1", Layer::Vision, vec!["CYC-M3"]),
            make_node("CYC-M2", Layer::Vision, vec!["CYC-M1"]),
            make_node("CYC-M3", Layer::Vision, vec!["CYC-M2"]),
            make_node("CYC-Z1", Layer::Vision, vec!["CYC-Z2"]),
            make_node("CYC-Z2", Layer::Vision, vec!["CYC-Z1"]),
        ]
    };

    // 1. In standard order:
    let graph1 = match TraceabilityGraph::build(make_nodes(), vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build multi-cycle graph 1: {:?}", e),
    };
    let first_detected = match detect_cycles(&graph1) {
        Err(GraphError::CycleDetected { cycle_chain }) => cycle_chain,
        other => panic!("Expected cycle, got {:?}", other),
    };
    assert_eq!(first_detected, "CYC-A1 -> CYC-A2 -> CYC-A1");

    let all_cycles1 = find_all_cycles(&graph1);
    assert_eq!(all_cycles1.len(), 3);
    assert_eq!(all_cycles1[0].format_chain(), "CYC-A1 -> CYC-A2 -> CYC-A1");
    assert_eq!(
        all_cycles1[1].format_chain(),
        "CYC-M1 -> CYC-M2 -> CYC-M3 -> CYC-M1"
    );
    assert_eq!(all_cycles1[2].format_chain(), "CYC-Z1 -> CYC-Z2 -> CYC-Z1");

    // 2. In reversed and permuted orders:
    for shift in 1..7 {
        let mut nodes = make_nodes();
        nodes.rotate_left(shift);
        let graph = match TraceabilityGraph::build(nodes, vec![]) {
            Ok(g) => g,
            Err(e) => panic!("Failed to build shifted graph: {:?}", e),
        };

        // detect_cycles must remain 100% deterministic regardless of insertion order
        let detected = match detect_cycles(&graph) {
            Err(GraphError::CycleDetected { cycle_chain }) => cycle_chain,
            other => panic!("Expected cycle, got {:?}", other),
        };
        assert_eq!(
            detected, first_detected,
            "detect_cycles must be deterministic across node permutations"
        );

        let all_cycles = find_all_cycles(&graph);
        assert_eq!(all_cycles.len(), 3);
        assert_eq!(all_cycles[0].format_chain(), "CYC-A1 -> CYC-A2 -> CYC-A1");
        assert_eq!(
            all_cycles[1].format_chain(),
            "CYC-M1 -> CYC-M2 -> CYC-M3 -> CYC-M1"
        );
        assert_eq!(all_cycles[2].format_chain(), "CYC-Z1 -> CYC-Z2 -> CYC-Z1");
    }
}

// -----------------------------------------------------------------------------
// 5. Duplicate Node IDs (Fail-Fast Verification)
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_duplicate_node_id_fails_fast_with_provenance() {
    // Test duplicate ID within the same file at different lines
    let node1 = make_node_with_origin(
        "REQ-AUTH-01",
        Layer::Requirement,
        vec![],
        "docs/reqs.md",
        25,
    );
    let node2 = make_node_with_origin(
        "REQ-AUTH-01",
        Layer::Requirement,
        vec![],
        "docs/reqs.md",
        80,
    );

    let result = TraceabilityGraph::build(vec![node1, node2], vec![]);
    match result {
        Err(GraphError::DuplicateIdentifier {
            id,
            path_a,
            line_a,
            path_b,
            line_b,
        }) => {
            assert_eq!(id, "REQ-AUTH-01");
            assert_eq!(path_a, PathBuf::from("docs/reqs.md"));
            assert_eq!(line_a, 25);
            assert_eq!(path_b, PathBuf::from("docs/reqs.md"));
            assert_eq!(line_b, 80);
        }
        other => panic!("Expected DuplicateIdentifier, got {:?}", other),
    }

    // Test duplicate ID across different files and different layers
    let node_vis = make_node_with_origin(
        "ITEM-COLLIDE-01",
        Layer::Vision,
        vec![],
        "docs/vision.md",
        5,
    );
    let node_spec = make_node_with_origin(
        "ITEM-COLLIDE-01",
        Layer::TechnicalSpec,
        vec![],
        "docs/spec.md",
        110,
    );

    let result2 = TraceabilityGraph::build(vec![node_vis, node_spec], vec![]);
    match result2 {
        Err(GraphError::DuplicateIdentifier {
            id,
            path_a,
            line_a,
            path_b,
            line_b,
        }) => {
            assert_eq!(id, "ITEM-COLLIDE-01");
            assert_eq!(path_a, PathBuf::from("docs/vision.md"));
            assert_eq!(line_a, 5);
            assert_eq!(path_b, PathBuf::from("docs/spec.md"));
            assert_eq!(line_b, 110);
        }
        other => panic!("Expected DuplicateIdentifier, got {:?}", other),
    }

    // Verify fail-fast: even with links present, duplicate check aborts before links are attached
    let link = make_link("ITEM-COLLIDE-01", "src/main.rs", 42, LinkKind::SourceCode);
    let node_a = make_node_with_origin("ITEM-COLLIDE-01", Layer::Vision, vec![], "docs/v.md", 1);
    let node_b = make_node_with_origin("ITEM-COLLIDE-01", Layer::Vision, vec![], "docs/v.md", 2);

    let result3 = TraceabilityGraph::build(vec![node_a, node_b], vec![link]);
    assert!(matches!(
        result3,
        Err(GraphError::DuplicateIdentifier { .. })
    ));
}

// -----------------------------------------------------------------------------
// 6. Deep Cycle Chains & High-Order Scalability Stress
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_deep_cycle_chain_scalability() {
    // 100-node ring cycle: tests recursion depth and call-stack slicing
    const COUNT: usize = 100;
    let mut nodes = Vec::with_capacity(COUNT);
    for i in 0..COUNT {
        let id = format!("NODE-{:03}", i);
        let parent = if i == 0 {
            format!("NODE-{:03}", COUNT - 1)
        } else {
            format!("NODE-{:03}", i - 1)
        };
        nodes.push(make_node(&id, Layer::Vision, vec![&parent]));
    }

    let graph = match TraceabilityGraph::build(nodes, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build 100-node cycle graph: {:?}", e),
    };

    let result = detect_cycles(&graph);
    assert!(result.is_err());
    let cycles = find_all_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), COUNT);
    assert_eq!(cycles[0].nodes().first(), cycles[0].nodes().last());
}

// -----------------------------------------------------------------------------
// 7. Cycle Detection with Broken Links and Disconnected Islands
// -----------------------------------------------------------------------------

#[test]
fn test_adversarial_cycle_coexisting_with_broken_links() {
    // Node A points to missing target; Node B & C form a 2-node cycle.
    let nodes = vec![
        make_node("NODE-A", Layer::Vision, vec!["NON-EXISTENT-TARGET"]),
        make_node("NODE-B", Layer::Vision, vec!["NODE-C"]),
        make_node("NODE-C", Layer::Vision, vec!["NODE-B"]),
    ];

    let graph = match TraceabilityGraph::build(nodes, vec![]) {
        Ok(g) => g,
        Err(e) => panic!("Failed to build graph with broken links: {:?}", e),
    };

    // Broken links do not crash cycle detection and do not prevent cycle discovery
    let result = detect_cycles(&graph);
    assert!(result.is_err());
    match result {
        Err(GraphError::CycleDetected { cycle_chain }) => {
            assert_eq!(cycle_chain, "NODE-B -> NODE-C -> NODE-B");
        }
        other => panic!("Expected CycleDetected, got {:?}", other),
    }
}
