//! Domain model representing the 7-layer traceability DAG, nodes, links, and origins.
//!
//! // @trace SPEC-FMT-GRAMMAR, REQ-DOC-PARSE, REQ-DAG-MODEL, US-DOC-01

pub mod identifier;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub use identifier::{validate_parent_relationship, IdError, NodeId, NODE_ID_REGEX_PATTERN};

/// The distinct tiers in the 7-layer traceability lifecycle hierarchy.
///
/// Order reflects upward justification and downward decomposition:
/// Vision (1) -> Strategy (2) -> Epic (3) -> UserStory (4) -> Requirement (5)
/// -> TechnicalSpec (6) -> SourceCode (7) & AutomatedTest (8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Layer {
    /// Layer 1: High-level product vision and North Star principles.
    Vision = 1,
    /// Layer 2: Strategic themes and initiatives realizing Vision.
    Strategy = 2,
    /// Layer 3: Epics decomposing strategic themes into deliverable milestones.
    Epic = 3,
    /// Layer 4: User stories describing user persona value.
    UserStory = 4,
    /// Layer 5: Functional and non-functional requirements.
    Requirement = 5,
    /// Layer 6: Technical specifications detailing architectural realization.
    TechnicalSpec = 6,
    /// Layer 7a: Source code implementation links.
    SourceCode = 7,
    /// Layer 7b: Automated verification test links.
    AutomatedTest = 8,
}

impl Layer {
    /// Returns the integer rank (1 to 8) representing the hierarchy order.
    #[inline]
    pub fn rank(&self) -> u8 {
        *self as u8
    }

    /// Returns the canonical display name of the layer.
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            Layer::Vision => "Vision",
            Layer::Strategy => "Strategy",
            Layer::Epic => "Epic",
            Layer::UserStory => "UserStory",
            Layer::Requirement => "Requirement",
            Layer::TechnicalSpec => "TechnicalSpec",
            Layer::SourceCode => "SourceCode",
            Layer::AutomatedTest => "AutomatedTest",
        }
    }

    /// Returns the standard specification identifier prefix for this layer, if applicable.
    #[inline]
    pub fn prefix(&self) -> Option<&'static str> {
        match self {
            Layer::Vision => Some("VIS"),
            Layer::Strategy => Some("STRAT"),
            Layer::Epic => Some("EPIC"),
            Layer::UserStory => Some("US"),
            Layer::Requirement => Some("REQ"),
            Layer::TechnicalSpec => Some("SPEC"),
            Layer::SourceCode | Layer::AutomatedTest => None,
        }
    }

    /// Attempts to match a string prefix (e.g. "VIS", "REQ") to a standard specification layer.
    pub fn from_prefix(prefix: &str) -> Option<Layer> {
        match prefix {
            "VIS" => Some(Layer::Vision),
            "STRAT" => Some(Layer::Strategy),
            "EPIC" => Some(Layer::Epic),
            "US" => Some(Layer::UserStory),
            "REQ" => Some(Layer::Requirement),
            "SPEC" => Some(Layer::TechnicalSpec),
            _ => None,
        }
    }

    /// Returns true if this layer is the root layer (Vision), which has no parents.
    #[inline]
    pub fn is_root(&self) -> bool {
        matches!(self, Layer::Vision)
    }

    /// Returns true if this layer is a leaf layer (SourceCode or AutomatedTest).
    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self, Layer::SourceCode | Layer::AutomatedTest)
    }

    /// Returns true if this layer represents a specification document node (Layers 1-6).
    #[inline]
    pub fn is_specification(&self) -> bool {
        !self.is_leaf()
    }

    /// Returns the slice of allowed parent layers for nodes residing in this layer.
    ///
    /// - Vision: No parents allowed.
    /// - Strategy: Only Vision.
    /// - Epic: Only Strategy.
    /// - UserStory: Only Epic.
    /// - Requirement: Only UserStory.
    /// - TechnicalSpec: Requirement or UserStory.
    /// - SourceCode: TechnicalSpec or Requirement.
    /// - AutomatedTest: TechnicalSpec, Requirement, or UserStory.
    pub fn allowed_parent_layers(&self) -> &'static [Layer] {
        match self {
            Layer::Vision => &[],
            Layer::Strategy => &[Layer::Vision],
            Layer::Epic => &[Layer::Strategy],
            Layer::UserStory => &[Layer::Epic],
            Layer::Requirement => &[Layer::UserStory],
            Layer::TechnicalSpec => &[Layer::Requirement, Layer::UserStory],
            Layer::SourceCode => &[Layer::TechnicalSpec, Layer::Requirement],
            Layer::AutomatedTest => &[Layer::TechnicalSpec, Layer::Requirement, Layer::UserStory],
        }
    }

    /// Returns true if `parent_layer` is permitted as a parent for a node in `self`.
    #[inline]
    pub fn can_have_parent_layer(&self, parent_layer: Layer) -> bool {
        self.allowed_parent_layers().contains(&parent_layer)
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Layer {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace(['-', '_', ' '], "");
        match normalized.as_str() {
            "vision" | "vis" | "1" => Ok(Layer::Vision),
            "strategy" | "strat" | "2" => Ok(Layer::Strategy),
            "epic" | "3" => Ok(Layer::Epic),
            "userstory" | "story" | "us" | "4" => Ok(Layer::UserStory),
            "requirement" | "req" | "5" => Ok(Layer::Requirement),
            "technicalspec" | "spec" | "6" => Ok(Layer::TechnicalSpec),
            "sourcecode" | "source" | "code" | "7" => Ok(Layer::SourceCode),
            "automatedtest" | "test" | "tests" | "8" => Ok(Layer::AutomatedTest),
            _ => Err(IdError::InvalidLayer(s.to_string())),
        }
    }
}

/// Category of a leaf comment traceability link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// Implementation evidence in production source code (`@trace` or `@trace:code`).
    SourceCode,
    /// Automated test evidence verifying a requirement (`@trace:test`).
    AutomatedTest,
}

impl LinkKind {
    /// Returns the string tag name.
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::SourceCode => "source_code",
            LinkKind::AutomatedTest => "automated_test",
        }
    }

    /// Maps this link kind to its corresponding `Layer`.
    #[inline]
    pub fn to_layer(&self) -> Layer {
        match self {
            LinkKind::SourceCode => Layer::SourceCode,
            LinkKind::AutomatedTest => Layer::AutomatedTest,
        }
    }
}

impl fmt::Display for LinkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for LinkKind {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace(['-', ' '], "_");
        match normalized.as_str() {
            "source_code" | "code" | "src" => Ok(LinkKind::SourceCode),
            "automated_test" | "test" | "tests" => Ok(LinkKind::AutomatedTest),
            _ => Err(IdError::InvalidLinkKind(s.to_string())),
        }
    }
}

impl From<LinkKind> for Layer {
    fn from(kind: LinkKind) -> Self {
        kind.to_layer()
    }
}

/// Provenance location tracking file path and 1-indexed line number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileOrigin {
    /// Path to the file, relative to repository root.
    pub path: PathBuf,
    /// 1-indexed line number in the source file.
    pub line: usize,
}

impl FileOrigin {
    /// Creates a new `FileOrigin`.
    pub fn new(path: impl Into<PathBuf>, line: usize) -> Self {
        Self {
            path: path.into(),
            line,
        }
    }

    /// Returns the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the 1-indexed line number.
    pub fn line(&self) -> usize {
        self.line
    }
}

impl fmt::Display for FileOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.path.display(), self.line)
    }
}

/// A structured node in the traceability graph (Layers 1-6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceNode {
    /// Unique validated node identifier.
    pub id: NodeId,
    /// Human-readable title of the node.
    pub title: String,
    /// Hierarchy layer category.
    pub layer: Layer,
    /// Direct upstream parent identifiers.
    pub parents: Vec<NodeId>,
    /// File and line provenance where this node was declared.
    pub file_origin: FileOrigin,
    /// Long-form description or narrative content.
    pub description: String,
    /// Additional parsed key-value metadata attributes.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, String>,
}

impl TraceNode {
    /// Creates a new `TraceNode`.
    pub fn new(
        id: NodeId,
        title: impl Into<String>,
        layer: Layer,
        parents: Vec<NodeId>,
        file_origin: FileOrigin,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            layer,
            parents,
            file_origin,
            description: description.into(),
            attributes: HashMap::new(),
        }
    }

    /// Attaches arbitrary key-value metadata attributes to this node.
    pub fn with_attributes(mut self, attributes: HashMap<String, String>) -> Self {
        self.attributes = attributes;
        self
    }

    /// Returns the node identifier.
    pub fn id(&self) -> &NodeId {
        &self.id
    }

    /// Returns the title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the layer.
    pub fn layer(&self) -> Layer {
        self.layer
    }

    /// Returns the slice of parent identifiers.
    pub fn parents(&self) -> &[NodeId] {
        &self.parents
    }

    /// Returns the provenance file origin.
    pub fn file_origin(&self) -> &FileOrigin {
        &self.file_origin
    }

    /// Returns the long-form description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the attributes map.
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

/// A leaf traceability link extracted from source code or automated test annotations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceLink {
    /// The target specification node identifier being implemented or verified.
    pub target_id: NodeId,
    /// Source file path where the `@trace` comment was scanned.
    pub source_file: PathBuf,
    /// 1-indexed line number in the source file.
    pub line: usize,
    /// Classification of the link (SourceCode vs AutomatedTest).
    pub kind: LinkKind,
}

impl TraceLink {
    /// Creates a new `TraceLink`.
    pub fn new(
        target_id: NodeId,
        source_file: impl Into<PathBuf>,
        line: usize,
        kind: LinkKind,
    ) -> Self {
        Self {
            target_id,
            source_file: source_file.into(),
            line,
            kind,
        }
    }

    /// Returns the target node identifier.
    pub fn target_id(&self) -> &NodeId {
        &self.target_id
    }

    /// Returns the source file path.
    pub fn source_file(&self) -> &Path {
        &self.source_file
    }

    /// Returns the 1-indexed line number.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the link kind.
    pub fn kind(&self) -> LinkKind {
        self.kind
    }

    /// Returns the provenance `FileOrigin` of this link.
    pub fn file_origin(&self) -> FileOrigin {
        FileOrigin::new(self.source_file.clone(), self.line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_taxonomy_ordering_and_ranks() {
        assert_eq!(Layer::Vision.rank(), 1);
        assert_eq!(Layer::Strategy.rank(), 2);
        assert_eq!(Layer::Epic.rank(), 3);
        assert_eq!(Layer::UserStory.rank(), 4);
        assert_eq!(Layer::Requirement.rank(), 5);
        assert_eq!(Layer::TechnicalSpec.rank(), 6);
        assert_eq!(Layer::SourceCode.rank(), 7);
        assert_eq!(Layer::AutomatedTest.rank(), 8);

        assert!(Layer::Vision < Layer::Strategy);
        assert!(Layer::Strategy < Layer::Epic);
        assert!(Layer::Epic < Layer::UserStory);
        assert!(Layer::UserStory < Layer::Requirement);
        assert!(Layer::Requirement < Layer::TechnicalSpec);
        assert!(Layer::TechnicalSpec < Layer::SourceCode);
        assert!(Layer::SourceCode < Layer::AutomatedTest);
    }

    #[test]
    fn test_layer_prefix_lookups() {
        assert_eq!(Layer::Vision.prefix(), Some("VIS"));
        assert_eq!(Layer::Strategy.prefix(), Some("STRAT"));
        assert_eq!(Layer::Epic.prefix(), Some("EPIC"));
        assert_eq!(Layer::UserStory.prefix(), Some("US"));
        assert_eq!(Layer::Requirement.prefix(), Some("REQ"));
        assert_eq!(Layer::TechnicalSpec.prefix(), Some("SPEC"));
        assert_eq!(Layer::SourceCode.prefix(), None);
        assert_eq!(Layer::AutomatedTest.prefix(), None);

        assert_eq!(Layer::from_prefix("VIS"), Some(Layer::Vision));
        assert_eq!(Layer::from_prefix("STRAT"), Some(Layer::Strategy));
        assert_eq!(Layer::from_prefix("EPIC"), Some(Layer::Epic));
        assert_eq!(Layer::from_prefix("US"), Some(Layer::UserStory));
        assert_eq!(Layer::from_prefix("REQ"), Some(Layer::Requirement));
        assert_eq!(Layer::from_prefix("SPEC"), Some(Layer::TechnicalSpec));
        assert_eq!(Layer::from_prefix("UNKNOWN"), None);
    }

    #[test]
    fn test_layer_from_str_and_display() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Layer::from_str("Vision")?, Layer::Vision);
        assert_eq!(Layer::from_str("vision")?, Layer::Vision);
        assert_eq!(Layer::from_str("vis")?, Layer::Vision);
        assert_eq!(Layer::from_str("user-story")?, Layer::UserStory);
        assert_eq!(Layer::from_str("technical_spec")?, Layer::TechnicalSpec);
        assert_eq!(Layer::from_str("req")?, Layer::Requirement);
        assert_eq!(Layer::from_str("code")?, Layer::SourceCode);
        assert_eq!(Layer::from_str("test")?, Layer::AutomatedTest);

        assert!(Layer::from_str("nonexistent").is_err());
        assert_eq!(Layer::Requirement.to_string(), "Requirement");
        Ok(())
    }

    #[test]
    fn test_file_origin_display() {
        let origin = FileOrigin::new("src/main.rs", 42);
        assert_eq!(origin.to_string(), "src/main.rs:42");
        assert_eq!(origin.path(), Path::new("src/main.rs"));
        assert_eq!(origin.line(), 42);
    }

    #[test]
    fn test_trace_node_and_link_creation() -> Result<(), Box<dyn std::error::Error>> {
        let id = NodeId::new("REQ-DOC-PARSE")?;
        let parent_id = NodeId::new("US-DOC-01")?;
        let origin = FileOrigin::new("docs/requirements.md", 286);

        let mut attrs = HashMap::new();
        attrs.insert("status".to_string(), "approved".to_string());

        let node = TraceNode::new(
            id.clone(),
            "Plain-Text Specification Document Parsing",
            Layer::Requirement,
            vec![parent_id],
            origin,
            "Ingest markdown with YAML frontmatter",
        )
        .with_attributes(attrs);

        assert_eq!(node.id().as_str(), "REQ-DOC-PARSE");
        assert_eq!(node.layer(), Layer::Requirement);
        assert_eq!(node.parents().len(), 1);
        assert_eq!(
            node.attributes().get("status"),
            Some(&"approved".to_string())
        );

        let link = TraceLink::new(id, "src/parser/document.rs", 15, LinkKind::SourceCode);
        assert_eq!(link.target_id().as_str(), "REQ-DOC-PARSE");
        assert_eq!(link.file_origin().to_string(), "src/parser/document.rs:15");
        assert_eq!(link.kind(), LinkKind::SourceCode);
        Ok(())
    }
}
