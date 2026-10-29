//! Domain model representing the 7-layer traceability DAG, nodes, links, and origins.
//!
//! // @trace SPEC-FMT-GRAMMAR, REQ-DOC-PARSE, REQ-DAG-MODEL, US-DOC-01

pub mod identifier;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub use identifier::{validate_parent_relationship, IdError, NodeId, NODE_ID_REGEX_PATTERN};

/// The distinct tiers in the 7-layer traceability lifecycle hierarchy.
///
/// Order reflects upward justification and downward decomposition:
/// Vision (1) -> Strategy (2) -> Epic (3) -> UserStory (4) -> Requirement (5)
/// -> TechnicalSpec (6) -> SourceCode (7) & AutomatedTest (8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Layer 1: High-level product vision and North Star principles.
    Vision,
    /// Layer 2: Strategic themes and initiatives realizing Vision.
    Strategy,
    /// Layer 3: Epics decomposing strategic themes into deliverable milestones.
    Epic,
    /// Layer 4: User stories describing user persona value.
    UserStory,
    /// Layer 5: Functional and non-functional requirements.
    Requirement,
    /// Layer 6: Technical specifications detailing architectural realization.
    TechnicalSpec,
    /// Layer 7a: Source code implementation links.
    SourceCode,
    /// Layer 7b: Automated verification test links.
    AutomatedTest,
    /// A position in a user-declared hierarchy.
    Custom(u16),
}

impl Layer {
    /// Returns the rank representing the hierarchy order.
    #[inline]
    pub fn rank(&self) -> u16 {
        match self {
            Layer::Vision => 1,
            Layer::Strategy => 2,
            Layer::Epic => 3,
            Layer::UserStory => 4,
            Layer::Requirement => 5,
            Layer::TechnicalSpec => 6,
            Layer::SourceCode => 7,
            Layer::AutomatedTest => 8,
            Layer::Custom(rank) => *rank,
        }
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
            Layer::Custom(_) => "Custom",
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
            Layer::Custom(_) => None,
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

    /// Returns true if this layer is the first position in its hierarchy.
    #[inline]
    pub fn is_root(&self) -> bool {
        matches!(self, Layer::Vision | Layer::Custom(1))
    }

    /// Returns true if this layer is a leaf layer (SourceCode or AutomatedTest).
    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self, Layer::SourceCode | Layer::AutomatedTest)
    }

    /// Returns true if this layer represents a specification document node rather than a leaf link.
    #[inline]
    pub fn is_specification(&self) -> bool {
        !self.is_leaf()
    }

    /// Returns every higher specification layer that may parent this layer.
    pub fn allowed_parent_layers(&self) -> Vec<Layer> {
        let last_specification_rank = if self.is_leaf() {
            6
        } else {
            self.rank().saturating_sub(1)
        };
        (1..=last_specification_rank)
            .map(|rank| match (self, rank) {
                (Layer::Custom(_), rank) => Layer::Custom(rank),
                (_, 1) => Layer::Vision,
                (_, 2) => Layer::Strategy,
                (_, 3) => Layer::Epic,
                (_, 4) => Layer::UserStory,
                (_, 5) => Layer::Requirement,
                (_, 6) => Layer::TechnicalSpec,
                _ => Layer::Custom(rank),
            })
            .collect()
    }

    /// Returns true if `parent_layer` is permitted as a parent for a node in `self`.
    #[inline]
    pub fn can_have_parent_layer(&self, parent_layer: Layer) -> bool {
        self.allowed_parent_layers().contains(&parent_layer)
    }
}

impl PartialOrd for Layer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Layer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank()
            .cmp(&other.rank())
            .then_with(|| matches!(self, Layer::Custom(_)).cmp(&matches!(other, Layer::Custom(_))))
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layer::Custom(rank) => write!(f, "Layer{rank}"),
            _ => write!(f, "{}", self.as_str()),
        }
    }
}

impl FromStr for Layer {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace(['-', '_', ' '], "");
        if let Some(rank) = normalized.strip_prefix("layer") {
            if let Ok(rank) = rank.parse::<u16>() {
                if rank > 0 && rank < u16::MAX - 1 {
                    return Ok(Layer::Custom(rank));
                }
            }
        }
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

impl Serialize for Layer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Layer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Layer::from_str(&value).map_err(de::Error::custom)
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

/// A user-configurable traceability layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerDefinition {
    /// Human-readable layer name.
    pub name: String,
    /// Uppercase identifier prefix assigned to this layer.
    pub prefix: String,
}

/// Ordered specification layers used to classify identifiers and define parent edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LayerModel {
    /// Layers ordered from root to leaf, excluding code and test link kinds.
    pub layers: Vec<LayerDefinition>,
}

impl LayerModel {
    /// Returns true if this model exactly matches the built-in layer definitions.
    pub fn is_default(&self) -> bool {
        const DEFAULT_LAYERS: [(&str, &str); 6] = [
            ("Vision", "VIS"),
            ("Strategy", "STRAT"),
            ("Epic", "EPIC"),
            ("User Story", "US"),
            ("Requirement", "REQ"),
            ("Technical Specification", "SPEC"),
        ];
        self.layers.len() == DEFAULT_LAYERS.len()
            && self
                .layers
                .iter()
                .zip(DEFAULT_LAYERS)
                .all(|(layer, (name, prefix))| layer.name == name && layer.prefix == prefix)
    }

    /// Returns the configured layer index used for requirement coverage.
    ///
    /// The default model keeps its historical Requirement layer. Other models
    /// prefer a layer named Requirement or using the `REQ` prefix, then fall
    /// back to the final configured layer.
    pub fn coverage_index(&self) -> usize {
        self.layers
            .iter()
            .position(|layer| {
                layer.prefix == "REQ"
                    || matches!(
                        layer
                            .name
                            .chars()
                            .filter(|character| character.is_ascii_alphanumeric())
                            .collect::<String>()
                            .to_ascii_lowercase()
                            .as_str(),
                        "req" | "requirement" | "requirements"
                    )
            })
            .unwrap_or_else(|| self.layers.len().saturating_sub(1))
    }

    /// Returns the configured layer for an identifier prefix.
    pub fn layer_for_prefix(&self, prefix: &str) -> Option<Layer> {
        self.layers
            .iter()
            .position(|layer| layer.prefix == prefix)
            .and_then(|index| self.layer_at(index))
    }

    /// Returns a configured layer by its name or prefix.
    pub fn parse_layer(&self, value: &str) -> Option<Layer> {
        self.layers
            .iter()
            .position(|layer| {
                layer.name.eq_ignore_ascii_case(value) || layer.prefix.eq_ignore_ascii_case(value)
            })
            .and_then(|index| self.layer_at(index))
    }

    /// Returns a configured layer's display name, if it belongs to this model.
    pub fn name_for_layer(&self, layer: Layer) -> Option<&str> {
        if self.is_default() {
            return self
                .layer_for_prefix(layer.prefix()?)
                .filter(|configured| *configured == layer)
                .map(|_| layer.as_str());
        }
        self.layers
            .get(usize::from(layer.rank()).checked_sub(1)?)
            .map(|definition| definition.name.as_str())
    }

    /// Returns every configured parent layer permitted for a node at `layer`.
    ///
    /// Code and test leaf kinds may link directly to any configured specification layer.
    pub fn allowed_parent_layers(&self, layer: Layer) -> Vec<Layer> {
        if layer == Layer::SourceCode || layer == Layer::AutomatedTest {
            return (0..self.layers.len())
                .filter_map(|index| self.layer_at(index))
                .collect();
        }
        let Some(index) = self
            .layers
            .iter()
            .enumerate()
            .find_map(|(index, _)| (self.layer_at(index) == Some(layer)).then_some(index))
        else {
            return Vec::new();
        };
        (0..index)
            .filter_map(|parent_index| self.layer_at(parent_index))
            .collect()
    }

    /// Returns true if `parent` is an earlier configured specification layer than `child`.
    pub fn can_have_parent_layer(&self, child: Layer, parent: Layer) -> bool {
        self.allowed_parent_layers(child).contains(&parent)
    }

    /// Returns the layer at a zero-based index.
    pub fn layer_at(&self, index: usize) -> Option<Layer> {
        if self.is_default() {
            return match index {
                0 => Some(Layer::Vision),
                1 => Some(Layer::Strategy),
                2 => Some(Layer::Epic),
                3 => Some(Layer::UserStory),
                4 => Some(Layer::Requirement),
                5 => Some(Layer::TechnicalSpec),
                _ => None,
            };
        }
        u16::try_from(index.checked_add(1)?)
            .ok()
            .filter(|rank| *rank < u16::MAX - 1)
            .map(Layer::Custom)
    }

    /// Returns a concise list of valid prefixes for diagnostics.
    pub fn valid_prefixes(&self) -> String {
        self.layers
            .iter()
            .map(|layer| layer.prefix.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Validates layer names, prefixes, and hierarchy size.
    pub fn validate(&self) -> Result<(), String> {
        if self.layers.is_empty() {
            return Err("at least one layer must be configured".to_string());
        }
        if self.layers.len() >= usize::from(u16::MAX - 1) {
            return Err("the configured layer hierarchy is too large".to_string());
        }
        let mut names = std::collections::BTreeSet::new();
        let mut prefixes = std::collections::BTreeSet::new();
        for layer in &self.layers {
            if layer.name.trim().is_empty() {
                return Err("layer names cannot be empty".to_string());
            }
            if layer.prefix.len() < 2
                || !layer.prefix.as_bytes()[0].is_ascii_uppercase()
                || !layer
                    .prefix
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            {
                return Err(format!("invalid layer prefix '{}'", layer.prefix));
            }
            if !names.insert(layer.name.to_ascii_lowercase()) {
                return Err(format!("duplicate layer name '{}'", layer.name));
            }
            if !prefixes.insert(layer.prefix.as_str()) {
                return Err(format!("duplicate layer prefix '{}'", layer.prefix));
            }
        }
        Ok(())
    }
}

impl Default for LayerModel {
    fn default() -> Self {
        Self {
            layers: vec![
                LayerDefinition {
                    name: "Vision".to_string(),
                    prefix: "VIS".to_string(),
                },
                LayerDefinition {
                    name: "Strategy".to_string(),
                    prefix: "STRAT".to_string(),
                },
                LayerDefinition {
                    name: "Epic".to_string(),
                    prefix: "EPIC".to_string(),
                },
                LayerDefinition {
                    name: "User Story".to_string(),
                    prefix: "US".to_string(),
                },
                LayerDefinition {
                    name: "Requirement".to_string(),
                    prefix: "REQ".to_string(),
                },
                LayerDefinition {
                    name: "Technical Specification".to_string(),
                    prefix: "SPEC".to_string(),
                },
            ],
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
    fn test_custom_layer_hierarchy_and_coverage_layer() -> Result<(), Box<dyn std::error::Error>> {
        let model = LayerModel {
            layers: vec![
                LayerDefinition {
                    name: "Goal".to_string(),
                    prefix: "GOAL".to_string(),
                },
                LayerDefinition {
                    name: "Requirement".to_string(),
                    prefix: "REQ".to_string(),
                },
                LayerDefinition {
                    name: "Technical Specification".to_string(),
                    prefix: "SPEC".to_string(),
                },
            ],
        };
        let root = model
            .layer_at(0)
            .ok_or_else(|| std::io::Error::other("configured root layer missing"))?;
        let requirement = model
            .layer_at(1)
            .ok_or_else(|| std::io::Error::other("configured requirement layer missing"))?;
        let spec = model
            .layer_at(2)
            .ok_or_else(|| std::io::Error::other("configured spec layer missing"))?;

        assert_eq!(model.coverage_index(), 1);
        assert!(requirement.can_have_parent_layer(root));
        assert!(spec.can_have_parent_layer(root));
        assert!(!root.can_have_parent_layer(requirement));
        assert!(model.can_have_parent_layer(spec, requirement));
        assert!(model.can_have_parent_layer(Layer::SourceCode, spec));
        assert_eq!(model.name_for_layer(requirement), Some("Requirement"));
        Ok(())
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
    fn test_layer_serialization_remains_readable() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            serde_json::to_string(&Layer::Requirement)?,
            "\"Requirement\""
        );
        let custom = Layer::Custom(1);
        let serialized = serde_json::to_string(&custom)?;
        assert_eq!(serialized, "\"Layer1\"");
        assert_eq!(serde_json::from_str::<Layer>(&serialized)?, custom);
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
