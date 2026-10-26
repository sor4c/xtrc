//! Integration tests for the 4-Stage Bidirectional Validator Engine.
//!
//! // @trace:test SPEC-DAG-VALIDATE, REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST
//! // @trace:test REQ-ORPHAN-DETECT, REQ-STRICT-GATE, REQ-EXIT-CODES, US-VERIFY-01, US-ORPHAN-01, US-GATE-01

use std::path::PathBuf;
use xtrc::graph::{
    validate_graph, BrokenLinkKind, TraceabilityGraph, ValidationOptions, ValidationViolation,
};
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
            .map(|p| NodeId::new(p).expect("valid parent"))
            .collect(),
        make_origin("docs/spec.md", 10),
        "Description",
    )
}

fn make_link(target: &str, file: &str, line: usize, kind: LinkKind) -> TraceLink {
    TraceLink::new(NodeId::new(target).expect("valid target"), file, line, kind)
}

#[test]
fn test_stage1_broken_parent_link() {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("REQ-01", Layer::Requirement, vec!["US-NON-EXISTENT"]),
    ];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
    let report = graph.validate(false);

    assert_eq!(report.statistics.broken_links, 1);
    assert!(!report.is_success(false));
    assert_eq!(report.exit_code(false), 1);
    assert_eq!(report.status, "fail");
    assert!(report.violations.iter().any(|v| matches!(v,
        ValidationViolation::BrokenLink { target_id, kind: BrokenLinkKind::ParentReference, .. }
        if target_id.as_str() == "US-NON-EXISTENT"
    )));
}

#[test]
fn test_stage1_broken_code_link() {
    let nodes = vec![make_node("VIS-01", Layer::Vision, vec![])];
    let links = vec![make_link(
        "REQ-MISSING",
        "src/foo.rs",
        42,
        LinkKind::SourceCode,
    )];
    let graph = TraceabilityGraph::build(nodes, links).expect("build graph");
    let report = graph.validate(false);

    assert_eq!(report.statistics.broken_links, 1);
    assert_eq!(report.exit_code(false), 1);
    assert!(report.violations.iter().any(|v| matches!(v,
        ValidationViolation::BrokenLink { target_id, kind: BrokenLinkKind::SourceCodeTrace, .. }
        if target_id.as_str() == "REQ-MISSING"
    )));
}

#[test]
fn test_stage1_broken_test_link() {
    let nodes = vec![make_node("VIS-01", Layer::Vision, vec![])];
    let links = vec![make_link(
        "SPEC-MISSING",
        "tests/foo.rs",
        15,
        LinkKind::AutomatedTest,
    )];
    let graph = TraceabilityGraph::build(nodes, links).expect("build graph");
    let report = graph.validate(false);

    assert_eq!(report.statistics.broken_links, 1);
    assert_eq!(report.exit_code(false), 1);
    assert!(report.violations.iter().any(|v| matches!(v,
        ValidationViolation::BrokenLink { target_id, kind: BrokenLinkKind::AutomatedTestTrace, .. }
        if target_id.as_str() == "SPEC-MISSING"
    )));
}

#[test]
fn test_stage2_vision_root_exempt_from_orphan() {
    let nodes = vec![make_node("VIS-01", Layer::Vision, vec![])];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
    let report = graph.validate(true);

    assert_eq!(report.statistics.orphan_nodes, 0);
    assert!(report.is_success(true));
    assert_eq!(report.exit_code(true), 0);
}

#[test]
fn test_stage2_orphan_requirement_standard_vs_strict() {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("REQ-ORPHAN", Layer::Requirement, vec![]),
    ];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

    // Standard mode: warning only -> exit code 0
    let rep_default = graph.validate(false);
    assert_eq!(rep_default.statistics.orphan_nodes, 1);
    assert!(rep_default.is_success(false));
    assert_eq!(rep_default.exit_code(false), 0);
    assert_eq!(rep_default.status, "pass");

    // Strict mode: fatal error -> exit code 1
    let rep_strict = graph.validate(true);
    assert_eq!(rep_strict.statistics.orphan_nodes, 1);
    assert!(!rep_strict.is_success(true));
    assert_eq!(rep_strict.exit_code(true), 1);
    assert_eq!(rep_strict.status, "fail");
}

#[test]
fn test_stage3_full_coverage_100_percent() {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
        make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
        make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
        make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
        make_node("SPEC-01", Layer::TechnicalSpec, vec!["REQ-01"]),
    ];
    let links = vec![
        make_link("SPEC-01", "src/foo.rs", 10, LinkKind::SourceCode),
        make_link("SPEC-01", "tests/foo.rs", 20, LinkKind::AutomatedTest),
    ];
    let graph = TraceabilityGraph::build(nodes, links).expect("build graph");
    let report = graph.validate(true);

    assert_eq!(report.statistics.requirements_count, 1);
    assert_eq!(report.statistics.covered_nodes, 1);
    assert_eq!(report.statistics.coverage_percent, 100.0);
    assert_eq!(report.statistics.broken_links, 0);
    assert_eq!(report.statistics.orphan_nodes, 0);
    assert!(report.is_success(true));
    assert_eq!(report.exit_code(true), 0);
}

#[test]
fn test_stage3_direct_code_and_test_without_spec_is_fulfilled() {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
        make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
        make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
        make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
    ];
    let links = vec![
        make_link("REQ-01", "src/foo.rs", 10, LinkKind::SourceCode),
        make_link("REQ-01", "tests/foo.rs", 20, LinkKind::AutomatedTest),
    ];
    let graph = TraceabilityGraph::build(nodes, links).expect("build graph");
    let report = graph.validate(true);

    assert_eq!(report.statistics.requirements_count, 1);
    assert_eq!(report.statistics.covered_nodes, 1);
    assert_eq!(report.statistics.coverage_percent, 100.0);
    assert!(report.is_success(true));
    assert_eq!(report.exit_code(true), 0);
}

#[test]
fn test_stage3_requirement_missing_test_fails_strict() {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
        make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
        make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
        make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
    ];
    let links = vec![make_link("REQ-01", "src/foo.rs", 10, LinkKind::SourceCode)];
    let graph = TraceabilityGraph::build(nodes, links).expect("build graph");

    let rep_default = graph.validate(false);
    assert_eq!(rep_default.statistics.coverage_percent, 0.0);
    assert_eq!(rep_default.exit_code(false), 0);

    let rep_strict = graph.validate(true);
    assert_eq!(rep_strict.statistics.coverage_percent, 0.0);
    assert_eq!(rep_strict.exit_code(true), 1);
}

#[test]
fn test_stage3_empty_graph_has_100_percent_coverage() {
    let graph = TraceabilityGraph::build(vec![], vec![]).expect("build graph");
    let report = graph.validate(true);

    assert_eq!(report.statistics.requirements_count, 0);
    assert_eq!(report.statistics.covered_nodes, 0);
    assert_eq!(report.statistics.coverage_percent, 100.0);
    assert!(report.is_success(true));
    assert_eq!(report.exit_code(true), 0);
}

#[test]
fn test_stage4_unjustified_island_detected() {
    let nodes = vec![
        make_node("VIS-ROOT", Layer::Vision, vec![]),
        make_node("STRAT-ISLAND", Layer::Strategy, vec![]),
        make_node("EPIC-ISLAND", Layer::Epic, vec!["STRAT-ISLAND"]),
    ];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");
    let report = graph.validate(true);

    assert!(!report.is_success(true));
    assert!(report.violations.iter().any(|v| matches!(v,
        ValidationViolation::Unjustified { id, .. } if id.as_str() == "EPIC-ISLAND"
    )));
}

#[test]
fn test_json_serialization_matches_cli_schema() -> Result<(), Box<dyn std::error::Error>> {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("STRAT-01", Layer::Strategy, vec!["VIS-01"]),
        make_node("EPIC-01", Layer::Epic, vec!["STRAT-01"]),
        make_node("US-01", Layer::UserStory, vec!["EPIC-01"]),
        make_node("REQ-01", Layer::Requirement, vec!["US-01"]),
        make_node("SPEC-01", Layer::TechnicalSpec, vec!["REQ-01"]),
    ];
    let links = vec![
        make_link("SPEC-01", "src/foo.rs", 10, LinkKind::SourceCode),
        make_link("SPEC-01", "tests/foo.rs", 20, LinkKind::AutomatedTest),
    ];
    let graph = TraceabilityGraph::build(nodes, links)?;
    let report = graph.validate(true);

    let json_str = report.to_json()?;
    let val: serde_json::Value = serde_json::from_str(&json_str)?;

    assert_eq!(val["schema_version"], "1.0.0");
    assert_eq!(val["command"], "check");
    assert_eq!(val["status"], "pass");
    assert_eq!(val["statistics"]["total_nodes"], 6);
    assert_eq!(val["statistics"]["specs_count"], 1);
    assert_eq!(val["statistics"]["broken_links"], 0);
    assert_eq!(val["statistics"]["cycles_count"], 0);
    assert_eq!(val["statistics"]["orphan_nodes"], 0);
    assert_eq!(val["statistics"]["coverage_percent"], 100.0);
    assert!(val["violations"].as_array().expect("array").is_empty());
    Ok(())
}

#[test]
fn test_validation_layer_filter() {
    let nodes = vec![
        make_node("VIS-01", Layer::Vision, vec![]),
        make_node("STRAT-ORPHAN", Layer::Strategy, vec![]),
        make_node("REQ-ORPHAN", Layer::Requirement, vec![]),
    ];
    let graph = TraceabilityGraph::build(nodes, vec![]).expect("build graph");

    let opt_req = ValidationOptions {
        strict: true,
        require_test_coverage: true,
        layer_filter: Some(Layer::Requirement),
        id_filter: None,
    };
    let report = validate_graph(&graph, &opt_req);
    assert_eq!(report.statistics.orphan_nodes, 1);
    assert!(report.violations.iter().any(|v| match v {
        ValidationViolation::OrphanNode { id, .. } => id.as_str() == "REQ-ORPHAN",
        _ => false,
    }));
}
