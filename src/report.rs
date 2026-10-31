//! Deterministic, command-independent traceability report data.
//!
//! // @trace SPEC-CLI-OUTPUT, REQ-FMT-JSON, REQ-NFR-DETERM

use serde::{Deserialize, Serialize};

use crate::graph::model::TraceabilityGraph;
use crate::graph::validator::ValidationReport;
pub use crate::graph::validator::{
    BrokenLinkKind, EpicCoverage, ValidationStats, ValidationViolation, ViolationSeverity,
};
use crate::model::LayerModel;
pub use crate::model::{FileOrigin, Layer, LinkKind, NodeId};

/// Shared JSON schema version for all report output.
pub(crate) const REPORT_SCHEMA_VERSION: &str = "1.0.0";

/// Coverage state assigned once to each node when a report is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageStatus {
    /// The node has the configured implementation and test evidence.
    #[serde(rename = "Covered")]
    Covered,
    /// The node has neither implementation nor test evidence.
    #[serde(rename = "Uncovered")]
    Uncovered,
    /// The node has test evidence but lacks implementation evidence.
    #[serde(rename = "Missing Code")]
    MissingCode,
    /// The node has implementation evidence but lacks test evidence.
    #[serde(rename = "Missing Test")]
    MissingTest,
}

impl CoverageStatus {
    /// Returns the stable display label used by report renderers.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Covered => "Covered",
            Self::Uncovered => "Uncovered",
            Self::MissingCode => "Missing Code",
            Self::MissingTest => "Missing Test",
        }
    }

    fn from_evidence(has_code: bool, has_test: bool, require_test_coverage: bool) -> Self {
        if !require_test_coverage {
            return if has_code {
                Self::Covered
            } else {
                Self::MissingCode
            };
        }
        match (has_code, has_test) {
            (true, true) => Self::Covered,
            (false, false) => Self::Uncovered,
            (false, true) => Self::MissingCode,
            (true, false) => Self::MissingTest,
        }
    }
}

/// Computed coverage evidence for a graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NodeCoverage {
    /// Covered node identifier.
    pub node_id: NodeId,
    /// Whether code evidence exists on this node or its immediate child specs.
    pub has_code: bool,
    /// Whether test evidence exists on this node or its immediate child specs.
    pub has_test: bool,
    /// Total code links on this node and its immediate child specs.
    pub code_links_count: usize,
    /// Total test links on this node and its immediate child specs.
    pub test_links_count: usize,
    /// Canonical coverage classification.
    pub status: CoverageStatus,
}

/// Computes per-node coverage once, using the configured immediate child layer.
pub(crate) fn calculate_node_coverage(
    graph: &TraceabilityGraph,
    layers: &LayerModel,
    require_test_coverage: bool,
) -> Vec<NodeCoverage> {
    graph
        .all_nodes()
        .map(|node| {
            let child_layer = layers.layer_at(usize::from(node.layer.rank()));
            let child_ids: Vec<_> = graph
                .children_of(&node.id)
                .iter()
                .filter(|child_id| {
                    graph
                        .get_node(child_id)
                        .is_some_and(|child| Some(child.layer) == child_layer)
                })
                .collect();
            let code_links_count = graph.code_links_for(&node.id).len()
                + child_ids
                    .iter()
                    .map(|id| graph.code_links_for(id).len())
                    .sum::<usize>();
            let test_links_count = graph.test_links_for(&node.id).len()
                + child_ids
                    .iter()
                    .map(|id| graph.test_links_for(id).len())
                    .sum::<usize>();
            let has_code = code_links_count > 0;
            let has_test = test_links_count > 0;
            NodeCoverage {
                node_id: node.id.clone(),
                has_code,
                has_test,
                code_links_count,
                test_links_count,
                status: CoverageStatus::from_evidence(has_code, has_test, require_test_coverage),
            }
        })
        .collect()
}

/// Link evidence attached to a specification node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportLink {
    /// Identifier of the linked specification node.
    pub target_id: String,
    /// Whether this link is implementation or automated-test evidence.
    pub kind: LinkKind,
    /// Repository-relative source file path.
    pub source_file: String,
    /// One-based line number of the trace marker.
    pub line: usize,
}

/// A specification node and its computed coverage state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportNode {
    /// Node identifier.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Configured layer name.
    pub layer: String,
    /// Direct parent identifiers.
    pub parents: Vec<String>,
    /// Direct child identifiers.
    pub children: Vec<String>,
    /// Immediate child specification identifiers.
    pub specs: Vec<String>,
    /// File and line where this node was declared.
    pub source_origin: String,
    /// Long-form description.
    pub description: String,
    /// Source code links, formatted as `path:line`.
    pub code_links: Vec<String>,
    /// Automated test links, formatted as `path:line`.
    pub test_links: Vec<String>,
    /// Counts include evidence on the node and its immediate child specification nodes.
    pub code_links_count: usize,
    /// Counts include evidence on the node and its immediate child specification nodes.
    pub test_links_count: usize,
    /// Computed coverage state, shared by every report view.
    pub coverage_status: CoverageStatus,
}

/// Matrix row projection of canonical report node data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportMatrixRow {
    /// Item identifier.
    pub id: String,
    /// Configured lifecycle layer name.
    pub layer: String,
    /// Item title.
    pub title: String,
    /// Upward parent identifiers.
    pub parents: Vec<String>,
    /// Downward child specification identifiers.
    pub specs: Vec<String>,
    /// Total source code implementation links.
    pub code_links_count: usize,
    /// Total automated test verification links.
    pub test_links_count: usize,
    /// Forward coverage state, copied from the canonical node record.
    pub status: String,
}

/// Hierarchical node representation used by the `tree` JSON view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportTreeNode {
    /// Node identifier.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Lifecycle layer.
    pub layer: String,
    /// Parent identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<String>,
    /// Child specification nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ReportTreeNode>,
    /// Attached code and test evidence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<ReportTreeLink>,
}

/// Leaf link representation used by the `tree` JSON view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportTreeLink {
    /// Link classification.
    pub kind: String,
    /// Target node identifier.
    pub target_id: String,
    /// Source file.
    pub source_file: String,
    /// One-based line number.
    pub line: usize,
}

/// Canonical report envelope used by all machine-readable command output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Version of the shared report schema.
    pub schema_version: String,
    /// Command that produced this report.
    pub command: String,
    /// Overall validation outcome.
    pub status: String,
    /// Summary metrics computed by graph validation.
    pub statistics: ValidationStats,
    /// Detailed node records with canonical coverage status.
    pub nodes: Vec<ReportNode>,
    /// All scanned implementation and test links.
    pub links: Vec<ReportLink>,
    /// Validation violations and coverage gaps.
    pub violations: Vec<ValidationViolation>,
    /// Per-epic coverage breakdown.
    pub epic_coverage: Vec<EpicCoverage>,
    /// Matrix projection. Empty unless populated by the matrix command.
    pub rows: Vec<ReportMatrixRow>,
    /// Number of matrix rows in the current matrix view.
    pub total_rows: usize,
    /// Tree projection. Empty unless populated by the tree command.
    pub tree: Vec<ReportTreeNode>,
    /// Stable title for export renderers.
    pub title: String,
    /// Name of the configured coverage layer.
    pub coverage_layer_name: String,
    /// Name of the configured child-specification layer, when present.
    pub spec_layer_name: Option<String>,
    /// Whether the built-in layer naming convention is active.
    pub default_layer_names: bool,
}

impl Report {
    /// Builds the shared report from a graph and its single validation result.
    pub(crate) fn from_graph(
        graph: &TraceabilityGraph,
        validation: ValidationReport,
        command: &str,
        layers: &LayerModel,
    ) -> Self {
        let nodes: Vec<ReportNode> = graph
            .all_nodes()
            .zip(validation.node_coverage.iter())
            .map(|(node, coverage)| {
                let child_layer = layers.layer_at(usize::from(node.layer.rank()));
                let child_ids: Vec<_> = graph
                    .children_of(&node.id)
                    .iter()
                    .filter(|child_id| {
                        graph
                            .get_node(child_id)
                            .is_some_and(|child| Some(child.layer) == child_layer)
                    })
                    .collect();
                let links_for_node = graph.links_for(&node.id);
                ReportNode {
                    id: node.id.to_string(),
                    title: node.title.clone(),
                    layer: layers
                        .name_for_layer(node.layer)
                        .unwrap_or(node.layer.as_str())
                        .to_string(),
                    parents: graph
                        .parents_of(&node.id)
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    children: graph
                        .children_of(&node.id)
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    specs: child_ids.iter().map(|id| id.to_string()).collect(),
                    source_origin: node.file_origin.to_string(),
                    description: node.description.clone(),
                    code_links: links_for_node
                        .iter()
                        .filter(|link| link.kind == LinkKind::SourceCode)
                        .map(|link| format!("{}:{}", link.source_file.display(), link.line))
                        .collect(),
                    test_links: links_for_node
                        .iter()
                        .filter(|link| link.kind == LinkKind::AutomatedTest)
                        .map(|link| format!("{}:{}", link.source_file.display(), link.line))
                        .collect(),
                    code_links_count: coverage.code_links_count,
                    test_links_count: coverage.test_links_count,
                    coverage_status: coverage.status,
                }
            })
            .collect();

        let links = graph
            .all_links()
            .iter()
            .map(|link| ReportLink {
                target_id: link.target_id.to_string(),
                kind: link.kind,
                source_file: link.source_file.display().to_string(),
                line: link.line,
            })
            .collect();

        Self {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            command: command.to_string(),
            status: validation.status.clone(),
            statistics: validation.statistics,
            nodes,
            links,
            violations: validation.violations,
            epic_coverage: validation.epic_coverage,
            rows: Vec::new(),
            total_rows: 0,
            tree: Vec::new(),
            title: "Traceability Compliance Audit Report".to_string(),
            coverage_layer_name: layers
                .layer_at(layers.coverage_index())
                .and_then(|layer| layers.name_for_layer(layer))
                .unwrap_or("Coverage")
                .to_string(),
            spec_layer_name: layers
                .layer_at(layers.coverage_index() + 1)
                .and_then(|layer| layers.name_for_layer(layer))
                .map(str::to_string),
            default_layer_names: layers.is_default(),
        }
    }
}
