//! Integration test suite for identifier grammar, layer taxonomy, and parent validation.
//!
//! // @trace SPEC-FMT-GRAMMAR, REQ-DOC-PARSE, REQ-BROKEN-LINK, US-DOC-01

use std::str::FromStr;
use xtrc::error::GraphError;
use xtrc::model::{
    validate_parent_relationship, FileOrigin, Layer, LinkKind, NodeId, TraceLink, TraceNode,
};

#[test]
fn test_integration_full_7_layer_dag_linkage() {
    let vis_id = NodeId::from_str("VIS-VCS-NATIVE").expect("valid ID");
    let strat_id = NodeId::from_str("STRAT-PLAIN-TEXT").expect("valid ID");
    let epic_id = NodeId::from_str("EPIC-WORKSPACE").expect("valid ID");
    let us_id = NodeId::from_str("US-DOC-01").expect("valid ID");
    let req_id = NodeId::from_str("REQ-DOC-PARSE").expect("valid ID");
    let spec_id = NodeId::from_str("SPEC-FMT-GRAMMAR").expect("valid ID");

    // Verify each sequential parent relationship is valid
    assert!(
        validate_parent_relationship(&strat_id, Layer::Strategy, &vis_id, Layer::Vision).is_ok()
    );
    assert!(
        validate_parent_relationship(&epic_id, Layer::Epic, &strat_id, Layer::Strategy).is_ok()
    );
    assert!(validate_parent_relationship(&us_id, Layer::UserStory, &epic_id, Layer::Epic).is_ok());
    assert!(
        validate_parent_relationship(&req_id, Layer::Requirement, &us_id, Layer::UserStory).is_ok()
    );
    assert!(validate_parent_relationship(
        &spec_id,
        Layer::TechnicalSpec,
        &req_id,
        Layer::Requirement
    )
    .is_ok());
    assert!(
        validate_parent_relationship(&spec_id, Layer::TechnicalSpec, &us_id, Layer::UserStory)
            .is_ok()
    );

    // Verify invalid inverted relationships fail with LayerInversion
    match validate_parent_relationship(&vis_id, Layer::Vision, &strat_id, Layer::Strategy) {
        Err(GraphError::LayerInversion {
            child_id,
            parent_id,
            ..
        }) => {
            assert_eq!(child_id, "VIS-VCS-NATIVE");
            assert_eq!(parent_id, "STRAT-PLAIN-TEXT");
        }
        other => panic!("Expected LayerInversion, got {:?}", other),
    }

    match validate_parent_relationship(&req_id, Layer::Requirement, &spec_id, Layer::TechnicalSpec)
    {
        Err(GraphError::LayerInversion {
            child_id,
            parent_id,
            ..
        }) => {
            assert_eq!(child_id, "REQ-DOC-PARSE");
            assert_eq!(parent_id, "SPEC-FMT-GRAMMAR");
        }
        other => panic!("Expected LayerInversion, got {:?}", other),
    }
}

#[test]
fn test_integration_trace_node_and_link_provenance() {
    let req_id = NodeId::new("REQ-DOC-PARSE").expect("valid ID");
    let us_id = NodeId::new("US-DOC-01").expect("valid ID");

    let node = TraceNode::new(
        req_id.clone(),
        "Plain-Text Specification Document Parsing",
        Layer::Requirement,
        vec![us_id],
        FileOrigin::new("docs/requirements.md", 286),
        "Parses markdown with YAML frontmatter",
    );

    assert_eq!(node.id().as_str(), "REQ-DOC-PARSE");
    assert_eq!(node.file_origin().to_string(), "docs/requirements.md:286");

    let code_link = TraceLink::new(
        req_id.clone(),
        "src/model/identifier.rs",
        1,
        LinkKind::SourceCode,
    );
    assert_eq!(code_link.kind(), LinkKind::SourceCode);
    assert_eq!(
        code_link.file_origin().to_string(),
        "src/model/identifier.rs:1"
    );

    let test_link = TraceLink::new(
        req_id,
        "tests/model/test_identifier.rs",
        1,
        LinkKind::AutomatedTest,
    );
    assert_eq!(test_link.kind(), LinkKind::AutomatedTest);
    assert_eq!(
        test_link.file_origin().to_string(),
        "tests/model/test_identifier.rs:1"
    );
}
