//! 4-Stage Bidirectional Validator Engine verifying referential integrity,
//! root justification, forward coverage, and backward provenance to Vision.
//!
//! // @trace SPEC-DAG-VALIDATE, SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, REQ-NFR-PERF, REQ-NFR-SCALE, REQ-NFR-DETERM, REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST
//! // @trace REQ-ORPHAN-DETECT, REQ-STRICT-GATE, REQ-EXIT-CODES, US-VERIFY-01, US-ORPHAN-01, US-GATE-01, US-PERF-01

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

use crate::graph::cycle::detect_cycles;
use crate::graph::model::TraceabilityGraph;
use crate::model::{FileOrigin, Layer, LayerModel, LinkKind, NodeId, TraceNode};
use crate::report::{calculate_node_coverage, CoverageStatus, NodeCoverage};

/// Severity level of a validation violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViolationSeverity {
    /// Informational or advisory notice.
    Info,
    /// Warning notice (does not fail standard gate; elevated to error under --strict).
    Warning,
    /// Fatal error (always fails validation with exit code 1).
    Error,
}

/// The specific classification of a broken reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokenLinkKind {
    /// Parent reference declared in specification YAML/markdown frontmatter.
    ParentReference,
    /// `@trace` or `@trace:code` comment in source code.
    SourceCodeTrace,
    /// `@trace:test` comment in automated test file.
    AutomatedTestTrace,
}

impl BrokenLinkKind {
    /// Returns human-readable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParentReference => "parent reference",
            Self::SourceCodeTrace => "source code @trace",
            Self::AutomatedTestTrace => "automated test @trace:test",
        }
    }
}

/// A concrete integrity, coverage, or provenance violation detected during validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationViolation {
    /// Stage 1: Reference pointing to a non-existent node identifier.
    BrokenLink {
        /// File path and line number of the broken reference.
        origin: FileOrigin,
        /// The missing target identifier.
        target_id: NodeId,
        /// The source specification node ID declaring the parent, if applicable.
        #[serde(skip_serializing_if = "Option::is_none")]
        source_node: Option<NodeId>,
        /// Kind of link that was broken.
        kind: BrokenLinkKind,
        /// Human-readable diagnostic message.
        message: String,
    },

    /// Stage 2: Non-Vision node lacking parent justification links.
    OrphanNode {
        /// The orphan node identifier.
        id: NodeId,
        /// The hierarchy layer of the orphan node.
        layer: Layer,
        /// File origin where the orphan was declared.
        origin: FileOrigin,
        /// Human-readable diagnostic message.
        message: String,
    },

    /// Stage 3: Requirement node lacking child technical specifications.
    MissingSpec {
        /// The requirement identifier.
        id: NodeId,
        /// File origin where the requirement was declared.
        origin: FileOrigin,
        /// Human-readable diagnostic message.
        message: String,
    },

    /// Stage 3: Requirement or specification lacking source code implementation.
    MissingCode {
        /// The unfulfilled node identifier.
        id: NodeId,
        /// File origin where the node was declared.
        origin: FileOrigin,
        /// Human-readable diagnostic message.
        message: String,
    },

    /// Stage 3: Requirement or specification lacking automated test verification.
    MissingTest {
        /// The unverified node identifier.
        id: NodeId,
        /// File origin where the node was declared.
        origin: FileOrigin,
        /// Human-readable diagnostic message.
        message: String,
    },

    /// Stage 4: Node or leaf link cannot be traced upward to a Vision root.
    Unjustified {
        /// The unjustified node identifier.
        id: NodeId,
        /// The hierarchy layer of the unjustified node.
        layer: Layer,
        /// File origin of the node.
        origin: FileOrigin,
        /// Human-readable diagnostic message.
        message: String,
    },
}

impl ValidationViolation {
    /// Returns the severity level of this violation given the `strict` setting.
    pub fn severity(&self, strict: bool) -> ViolationSeverity {
        match self {
            Self::BrokenLink { .. } => ViolationSeverity::Error,
            Self::OrphanNode { .. } => {
                if strict {
                    ViolationSeverity::Error
                } else {
                    ViolationSeverity::Warning
                }
            }
            Self::MissingCode { .. } | Self::MissingTest { .. } | Self::Unjustified { .. } => {
                if strict {
                    ViolationSeverity::Error
                } else {
                    ViolationSeverity::Warning
                }
            }
            Self::MissingSpec { .. } => ViolationSeverity::Warning,
        }
    }

    /// Returns `true` if this violation causes command failure under the given strictness.
    pub fn is_fatal(&self, strict: bool) -> bool {
        match self {
            Self::BrokenLink { .. } => true,
            Self::OrphanNode { .. } => strict,
            Self::MissingCode { .. } => strict,
            Self::MissingTest { .. } => strict,
            Self::Unjustified { .. } => strict,
            Self::MissingSpec { .. } => false,
        }
    }

    /// Returns the provenance file origin of this violation.
    pub fn origin(&self) -> &FileOrigin {
        match self {
            Self::BrokenLink { origin, .. } => origin,
            Self::OrphanNode { origin, .. } => origin,
            Self::MissingSpec { origin, .. } => origin,
            Self::MissingCode { origin, .. } => origin,
            Self::MissingTest { origin, .. } => origin,
            Self::Unjustified { origin, .. } => origin,
        }
    }

    /// Returns the primary identifier associated with this violation.
    pub fn primary_id(&self) -> &NodeId {
        match self {
            Self::BrokenLink { target_id, .. } => target_id,
            Self::OrphanNode { id, .. } => id,
            Self::MissingSpec { id, .. } => id,
            Self::MissingCode { id, .. } => id,
            Self::MissingTest { id, .. } => id,
            Self::Unjustified { id, .. } => id,
        }
    }

    /// Returns the diagnostic message.
    pub fn message(&self) -> &str {
        match self {
            Self::BrokenLink { message, .. } => message,
            Self::OrphanNode { message, .. } => message,
            Self::MissingSpec { message, .. } => message,
            Self::MissingCode { message, .. } => message,
            Self::MissingTest { message, .. } => message,
            Self::Unjustified { message, .. } => message,
        }
    }
}

/// Numerical metrics and summary statistics computed during validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Total count of specification nodes across Layers 1-6.
    pub total_nodes: usize,
    /// Total count of nodes in the layer directly below the coverage layer, if configured.
    pub specs_count: usize,
    /// Total count of nodes in the configured coverage layer.
    pub requirements_count: usize,
    /// Total count of coverage-layer nodes fulfilled by code and tests.
    pub covered_nodes: usize,
    /// Total count of source code implementation links.
    pub code_links: usize,
    /// Total count of automated test verification links.
    pub test_links: usize,
    /// Total count of broken parent and trace references.
    #[serde(alias = "broken_link_count")]
    pub broken_links: usize,
    /// Total count of circular dependency loops.
    #[serde(alias = "cycle_count")]
    pub cycles_count: usize,
    /// Total count of orphan nodes lacking parents.
    #[serde(alias = "orphan_count")]
    pub orphan_nodes: usize,
    /// Downward requirement completion percentage (0.0 to 100.0).
    #[serde(alias = "coverage_percentage")]
    pub coverage_percent: f64,
}

/// Per-Epic forward coverage breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpicCoverage {
    /// Epic identifier.
    pub epic_id: NodeId,
    /// Total requirement nodes descending from this Epic.
    pub total_requirements: usize,
    /// Fulfilled requirement nodes descending from this Epic.
    pub covered_requirements: usize,
    /// Coverage percentage for this Epic.
    pub coverage_percent: f64,
}

/// Validation result produced by the 4-stage validation engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// High-level pass/fail outcome status ("pass" or "fail").
    pub status: String,
    /// Summary numerical metrics.
    pub statistics: ValidationStats,
    /// List of discovered integrity violations and coverage gaps (deterministic order).
    pub violations: Vec<ValidationViolation>,
    /// Per-node coverage data shared with every report renderer.
    pub node_coverage: Vec<NodeCoverage>,
    /// Per-epic coverage decomposition breakdown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub epic_coverage: Vec<EpicCoverage>,
}

impl ValidationReport {
    /// Returns `true` if the validation passed successfully according to `strict` mode.
    pub fn is_success(&self, strict: bool) -> bool {
        if self.statistics.broken_links > 0 || self.statistics.cycles_count > 0 {
            return false;
        }
        if strict {
            if self.statistics.orphan_nodes > 0 || self.statistics.coverage_percent < 100.0 {
                return false;
            }
            if self.violations.iter().any(|v| v.is_fatal(true)) {
                return false;
            }
        }
        true
    }

    /// Returns the deterministic process exit code: `0` for success, `1` for violation.
    #[inline]
    pub fn exit_code(&self, strict: bool) -> i32 {
        if self.is_success(strict) {
            0
        } else {
            1
        }
    }
}

/// Options configuring validation engine behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationOptions {
    /// Elevate warnings and partial coverage to fatal errors (exit code 1).
    pub strict: bool,
    /// Whether nodes in the coverage layer must link to automated test cases.
    pub require_test_coverage: bool,
    /// Optional filter to validate only nodes in a specific Layer.
    pub layer_filter: Option<Layer>,
    /// Optional filter to validate only the subgraph connected to `NodeId`.
    pub id_filter: Option<NodeId>,
    /// Ordered hierarchy used for root, requirement, and specification layers.
    pub layer_model: LayerModel,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            strict: false,
            require_test_coverage: true,
            layer_filter: None,
            id_filter: None,
            layer_model: LayerModel::default(),
        }
    }
}

/// Executes the 4-stage bidirectional validation engine against `graph`.
pub fn validate_graph(graph: &TraceabilityGraph, options: &ValidationOptions) -> ValidationReport {
    let mut violations: Vec<ValidationViolation> = Vec::new();
    let coverage_index = options.layer_model.coverage_index();
    let coverage_layer = options.layer_model.layer_at(coverage_index);
    let node_coverage =
        calculate_node_coverage(graph, &options.layer_model, options.require_test_coverage);
    let coverage_by_id: std::collections::BTreeMap<_, _> = node_coverage
        .iter()
        .map(|coverage| (&coverage.node_id, coverage))
        .collect();
    let spec_layer = (coverage_index + 1 < options.layer_model.layers.len())
        .then(|| options.layer_model.layer_at(coverage_index + 1))
        .flatten();

    // -------------------------------------------------------------------------
    // STAGE 1: Broken Links (Referential Integrity)
    // -------------------------------------------------------------------------
    let mut broken_links_count = 0usize;

    // 1a. Audit declared parents in all specification nodes
    for node in graph.all_nodes() {
        for parent_id in &node.parents {
            if !graph.has_node(parent_id) {
                broken_links_count += 1;
                violations.push(ValidationViolation::BrokenLink {
                    origin: node.file_origin.clone(),
                    target_id: parent_id.clone(),
                    source_node: Some(node.id.clone()),
                    kind: BrokenLinkKind::ParentReference,
                    message: format!(
                        "Broken link in {}: node '{}' references non-existent parent '{}'",
                        node.file_origin, node.id, parent_id
                    ),
                });
            }
        }
    }

    // 1b. Audit all scanned leaf links (@trace and @trace:test comments)
    for link in graph.all_links() {
        if !graph.has_node(&link.target_id) {
            broken_links_count += 1;
            let kind = match link.kind {
                LinkKind::SourceCode => BrokenLinkKind::SourceCodeTrace,
                LinkKind::AutomatedTest => BrokenLinkKind::AutomatedTestTrace,
            };
            violations.push(ValidationViolation::BrokenLink {
                origin: link.file_origin(),
                target_id: link.target_id.clone(),
                source_node: None,
                kind,
                message: format!(
                    "Broken link in {}:{}: {} references non-existent target '{}'",
                    link.source_file.display(),
                    link.line,
                    kind.as_str(),
                    link.target_id
                ),
            });
        }
    }

    // -------------------------------------------------------------------------
    // STAGE 2: Orphan Nodes (Root Justification)
    // -------------------------------------------------------------------------
    let mut orphan_count = 0usize;
    let mut orphan_ids: BTreeSet<&NodeId> = BTreeSet::new();

    for node in graph.all_nodes() {
        // Layer::Vision is the only valid root layer exempt from parent requirement
        if node.layer.is_root() {
            continue;
        }

        // Apply layer filter if present
        if let Some(filtered_layer) = options.layer_filter {
            if node.layer != filtered_layer {
                continue;
            }
        }

        // Check if node has no declared parents.
        // If it declares parent(s) that are missing, that was already recorded as a BrokenLink in Stage 1.
        if node.parents.is_empty() {
            orphan_count += 1;
            orphan_ids.insert(&node.id);
            violations.push(ValidationViolation::OrphanNode {
                id: node.id.clone(),
                layer: node.layer,
                origin: node.file_origin.clone(),
                message: format!(
                    "Orphan node '{}' ({}) at {}: non-root specification item lacks parent justification",
                    node.id,
                    display_layer_name(&options.layer_model, node.layer),
                    node.file_origin
                ),
            });
        }
    }

    // -------------------------------------------------------------------------
    // STAGE 3: Forward Coverage (Downward Decomposition Completeness)
    // -------------------------------------------------------------------------
    let mut req_nodes: Vec<(&TraceNode, &NodeCoverage)> = graph
        .all_nodes()
        .zip(node_coverage.iter())
        .filter(|(node, _)| Some(node.layer) == coverage_layer)
        .collect();

    // If id_filter is set, scope requirements to connected subgraph in O(V + E)
    if let Some(ref target_id) = options.id_filter {
        let target_descendants = graph.descendants_of(target_id);
        let target_ancestors = graph.ancestors_of(target_id);
        req_nodes.retain(|(node, _)| {
            node.id == *target_id
                || target_descendants.contains(&node.id)
                || target_ancestors.contains(&node.id)
        });
    }

    let requirements_count = req_nodes.len();
    let mut covered_nodes = 0usize;

    for (req, coverage) in &req_nodes {
        let children = graph.children_of(&req.id);
        let spec_children: Vec<&TraceNode> = children
            .iter()
            .filter_map(|id| graph.get_node(id))
            .filter(|n| Some(n.layer) == spec_layer)
            .collect();

        // 3a. Child Spec Check
        if spec_layer.is_some() && spec_children.is_empty() {
            let coverage_name = coverage_layer
                .map(|layer| display_layer_name(&options.layer_model, layer))
                .unwrap_or("Requirement");
            let spec_name = spec_layer
                .and_then(|layer| options.layer_model.name_for_layer(layer))
                .unwrap_or("Technical Specification");
            violations.push(ValidationViolation::MissingSpec {
                id: req.id.clone(),
                origin: req.file_origin.clone(),
                message: format!(
                    "{} '{}' at {} lacks child {}",
                    coverage_name, req.id, req.file_origin, spec_name
                ),
            });
        }

        // 3b. Code Links Check (Direct or via child specs)
        let has_code = coverage.has_code;

        if !has_code {
            let coverage_name = coverage_layer
                .map(|layer| display_layer_name(&options.layer_model, layer))
                .unwrap_or("Requirement");
            violations.push(ValidationViolation::MissingCode {
                id: req.id.clone(),
                origin: req.file_origin.clone(),
                message: format!(
                    "{} '{}' at {} missing source code implementation",
                    coverage_name, req.id, req.file_origin
                ),
            });
        }

        // 3c. Test Links Check (Direct or via child specs)
        let has_test = coverage.has_test;

        if options.require_test_coverage && !has_test {
            let coverage_name = coverage_layer
                .map(|layer| display_layer_name(&options.layer_model, layer))
                .unwrap_or("Requirement");
            violations.push(ValidationViolation::MissingTest {
                id: req.id.clone(),
                origin: req.file_origin.clone(),
                message: format!(
                    "{} '{}' at {} missing automated test verification",
                    coverage_name, req.id, req.file_origin
                ),
            });
        }

        if coverage.status == CoverageStatus::Covered {
            covered_nodes += 1;
        }
    }

    let coverage_percent = if requirements_count == 0 {
        100.0
    } else {
        let raw = (covered_nodes as f64 / requirements_count as f64) * 100.0;
        (raw * 10.0).round() / 10.0
    };

    // -------------------------------------------------------------------------
    // STAGE 4: Backward Justification (Upward Provenance to Vision)
    // -------------------------------------------------------------------------
    let vision_reachable = compute_vision_reachable(graph);
    for node in graph.all_nodes() {
        if node.layer.is_root() {
            continue;
        }

        // Check if node can reach any Vision root via precomputed O(V + E) set
        if !vision_reachable.contains(&node.id) && !orphan_ids.contains(&node.id) {
            violations.push(ValidationViolation::Unjustified {
                id: node.id.clone(),
                layer: node.layer,
                origin: node.file_origin.clone(),
                message: format!(
                    "Node '{}' ({}) at {} has no directed provenance path to a root layer",
                    node.id,
                    display_layer_name(&options.layer_model, node.layer),
                    node.file_origin
                ),
            });
        }
    }

    // -------------------------------------------------------------------------
    // Cycle Detection Integration
    // -------------------------------------------------------------------------
    let cycles_count = if detect_cycles(graph).is_err() { 1 } else { 0 };

    // Deterministic sorting of all collected violations
    violations.sort_by(|a, b| {
        a.origin()
            .path
            .cmp(&b.origin().path)
            .then_with(|| a.origin().line.cmp(&b.origin().line))
            .then_with(|| a.primary_id().cmp(b.primary_id()))
    });

    let specs_count = graph
        .all_nodes()
        .filter(|n| Some(n.layer) == spec_layer)
        .count();

    let code_links = graph
        .all_links()
        .iter()
        .filter(|l| l.kind == LinkKind::SourceCode)
        .count();

    let test_links = graph
        .all_links()
        .iter()
        .filter(|l| l.kind == LinkKind::AutomatedTest)
        .count();

    let stats = ValidationStats {
        total_nodes: graph.node_count(),
        specs_count,
        requirements_count,
        covered_nodes,
        code_links,
        test_links,
        broken_links: broken_links_count,
        cycles_count,
        orphan_nodes: orphan_count,
        coverage_percent,
    };

    let is_pass = if options.strict {
        broken_links_count == 0
            && cycles_count == 0
            && orphan_count == 0
            && coverage_percent == 100.0
            && !violations.iter().any(|v| v.is_fatal(true))
    } else {
        broken_links_count == 0 && cycles_count == 0
    };

    // Calculate per-epic coverage
    let mut epic_coverage: Vec<EpicCoverage> = Vec::new();
    let summary_layer = coverage_index
        .checked_sub(2)
        .and_then(|index| options.layer_model.layer_at(index));
    for epic in graph.all_nodes().filter(|n| Some(n.layer) == summary_layer) {
        let descendant_reqs: Vec<NodeId> = graph
            .descendants_of(&epic.id)
            .into_iter()
            .filter(|id| {
                graph
                    .get_node(id)
                    .is_some_and(|n| Some(n.layer) == coverage_layer)
            })
            .collect();

        if !descendant_reqs.is_empty() {
            let total_reqs = descendant_reqs.len();
            let mut covered_reqs = 0;
            for req_id in &descendant_reqs {
                let coverage = coverage_by_id.get(req_id);
                let is_covered =
                    coverage.is_some_and(|coverage| coverage.status == CoverageStatus::Covered);
                if is_covered {
                    covered_reqs += 1;
                }
            }

            let epic_pct = (covered_reqs as f64 / total_reqs as f64) * 100.0;
            epic_coverage.push(EpicCoverage {
                epic_id: epic.id.clone(),
                total_requirements: total_reqs,
                covered_requirements: covered_reqs,
                coverage_percent: (epic_pct * 10.0).round() / 10.0,
            });
        }
    }

    ValidationReport {
        status: if is_pass {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        statistics: stats,
        violations,
        node_coverage,
        epic_coverage,
    }
}

fn display_layer_name(model: &LayerModel, layer: Layer) -> &str {
    if model.is_default() {
        layer.as_str()
    } else {
        model.name_for_layer(layer).unwrap_or(layer.as_str())
    }
}

/// Computes the set of all nodes in `graph` that possess a valid directed path
/// terminating at a `Layer::Vision` root in a single $O(V + E)$ forward BFS pass.
fn compute_vision_reachable(graph: &TraceabilityGraph) -> BTreeSet<NodeId> {
    let mut visited: BTreeSet<NodeId> = BTreeSet::new();
    let mut queue: VecDeque<&NodeId> = VecDeque::new();

    for node in graph.all_nodes() {
        if node.layer.is_root() && visited.insert(node.id.clone()) {
            queue.push_back(&node.id);
        }
    }

    while let Some(current_id) = queue.pop_front() {
        for child_id in graph.children_of(current_id) {
            if graph.has_node(child_id) && visited.insert(child_id.clone()) {
                queue.push_back(child_id);
            }
        }
    }

    visited
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TraceLink;
    use std::path::PathBuf;

    fn make_origin(path: &str, line: usize) -> FileOrigin {
        FileOrigin::new(PathBuf::from(path), line)
    }

    fn make_id(s: &str) -> NodeId {
        match NodeId::new(s) {
            Ok(id) => id,
            Err(e) => panic!("invalid node id '{}': {}", s, e),
        }
    }

    fn build_graph(nodes: Vec<TraceNode>, links: Vec<TraceLink>) -> TraceabilityGraph {
        match TraceabilityGraph::build(nodes, links) {
            Ok(g) => g,
            Err(e) => panic!("failed to build graph in test: {}", e),
        }
    }

    fn make_node(id: &str, layer: Layer, parents: Vec<&str>) -> TraceNode {
        TraceNode::new(
            make_id(id),
            format!("Title {}", id),
            layer,
            parents.into_iter().map(make_id).collect(),
            make_origin("docs/spec.md", 10),
            "Description",
        )
    }

    fn make_link(target: &str, file: &str, line: usize, kind: LinkKind) -> TraceLink {
        TraceLink::new(make_id(target), file, line, kind)
    }

    #[test]
    fn test_stage1_broken_parent_link() {
        let nodes = vec![
            make_node("VIS-01", Layer::Vision, vec![]),
            make_node("REQ-01", Layer::Requirement, vec!["US-NON-EXISTENT"]),
        ];
        let graph = build_graph(nodes, vec![]);
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
        let graph = build_graph(nodes, links);
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
        let graph = build_graph(nodes, links);
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
        let graph = build_graph(nodes, vec![]);
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
        let graph = build_graph(nodes, vec![]);

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
        let graph = build_graph(nodes, links);
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
        let graph = build_graph(nodes, links);
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
        let graph = build_graph(nodes, links);

        let rep_default = graph.validate(false);
        assert_eq!(rep_default.statistics.coverage_percent, 0.0);
        assert_eq!(rep_default.exit_code(false), 0);

        let rep_strict = graph.validate(true);
        assert_eq!(rep_strict.statistics.coverage_percent, 0.0);
        assert_eq!(rep_strict.exit_code(true), 1);

        // When require_test_coverage = false, code without tests is covered
        let rep_no_test_req = graph.validate_with_options(&ValidationOptions {
            strict: true,
            require_test_coverage: false,
            ..Default::default()
        });
        assert_eq!(rep_no_test_req.statistics.coverage_percent, 100.0);
        assert!(rep_no_test_req.is_success(true));
        assert_eq!(rep_no_test_req.exit_code(true), 0);
    }

    #[test]
    fn test_stage3_empty_graph_has_100_percent_coverage() {
        let graph = build_graph(vec![], vec![]);
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
        let graph = build_graph(nodes, vec![]);
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
        let validation = graph.validate(true);
        let report =
            crate::report::Report::from_graph(&graph, validation, "check", &LayerModel::default());

        let json_str = serde_json::to_string_pretty(&report)?;
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
        let violations_arr = match val["violations"].as_array() {
            Some(arr) => arr,
            None => panic!("expected violations array in JSON output"),
        };
        assert!(violations_arr.is_empty());
        Ok(())
    }
}
