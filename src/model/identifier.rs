//! Validated identifier grammar and layer parent constraint verification.
//!
//! // @trace SPEC-FMT-GRAMMAR, REQ-DOC-PARSE, REQ-BROKEN-LINK, US-DOC-01

use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::error::GraphError;
use crate::error::ParseError;
use crate::model::Layer;

/// Regular expression pattern string governing all valid `NodeId` tokens.
///
/// Pattern: `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`
#[cfg(test)]
const NODE_ID_REGEX_PATTERN: &str = r"^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$";

/// Domain errors encountered when validating identifier tokens or layer mappings.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// Dispatched when an identifier fails syntax or grammar rules.
    #[error("Identifier '{token}' violates grammar regex ^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$: {reason}")]
    InvalidFormat {
        /// The offending token string.
        token: String,
        /// Concrete diagnostic explanation.
        reason: String,
    },

    /// Dispatched when an identifier declares an unrecognized layer prefix.
    #[error(
        "Unknown layer prefix '{prefix}' in identifier '{token}'. Valid prefixes: {valid_prefixes}"
    )]
    UnknownPrefix {
        /// The full identifier token.
        token: String,
        /// The unrecognized prefix segment.
        prefix: String,
        /// Prefixes declared by the active layer model.
        valid_prefixes: String,
    },

    /// Dispatched when an unrecognized string is parsed as a Layer.
    #[error("Unrecognized layer '{0}'. Valid layers: Vision, Strategy, Epic, UserStory, Requirement, TechnicalSpec, SourceCode, AutomatedTest")]
    InvalidLayer(String),

    /// Dispatched when an unrecognized string is parsed as a LinkKind.
    #[error("Unrecognized link kind '{0}'. Valid kinds: source_code, automated_test")]
    InvalidLinkKind(String),
}

impl IdError {
    /// Converts this `IdError` into a `ParseError::GrammarViolation` with file provenance.
    pub fn into_parse_error(self, path: impl Into<PathBuf>, line: usize) -> ParseError {
        match self {
            IdError::InvalidFormat { token, reason } => ParseError::GrammarViolation {
                token,
                path: path.into(),
                line,
                reason,
            },
            IdError::UnknownPrefix {
                token,
                prefix,
                valid_prefixes,
            } => ParseError::GrammarViolation {
                token,
                path: path.into(),
                line,
                reason: format!(
                    "Unknown layer prefix '{prefix}'. Valid prefixes: {valid_prefixes}"
                ),
            },
            other => ParseError::GrammarViolation {
                token: other.to_string(),
                path: path.into(),
                line,
                reason: other.to_string(),
            },
        }
    }
}

impl From<IdError> for ParseError {
    fn from(err: IdError) -> Self {
        ParseError::GrammarViolation {
            token: match &err {
                IdError::InvalidFormat { token, .. } => token.clone(),
                IdError::UnknownPrefix { token, .. } => token.clone(),
                IdError::InvalidLayer(s) | IdError::InvalidLinkKind(s) => s.clone(),
            },
            path: PathBuf::new(),
            line: 0,
            reason: err.to_string(),
        }
    }
}

/// Validated, immutable traceability node identifier enforcing `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NodeId(String);

impl NodeId {
    /// Creates a new `NodeId`, validating that it conforms to `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`.
    ///
    /// Returns `Err(IdError::InvalidFormat)` if the candidate token fails validation.
    pub fn new(raw: impl Into<String>) -> Result<Self, IdError> {
        let s = raw.into();
        Self::validate_grammar(&s)?;
        Ok(Self(s))
    }

    /// Performs strict grammar validation on candidate token string without constructing an instance.
    ///
    /// Implemented via pure zero-allocation string analysis conforming strictly to
    /// `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$` with zero panics or unsafe unwrapping.
    pub fn validate_grammar(token: &str) -> Result<(), IdError> {
        if token.is_empty() {
            return Err(IdError::InvalidFormat {
                token: token.to_string(),
                reason: "Identifier cannot be empty".to_string(),
            });
        }

        if token.starts_with('-') || token.ends_with('-') {
            return Err(IdError::InvalidFormat {
                token: token.to_string(),
                reason: "Identifier cannot start or end with a hyphen".to_string(),
            });
        }

        let mut segments = token.split('-');
        let first = match segments.next() {
            Some(seg) if !seg.is_empty() => seg,
            _ => {
                return Err(IdError::InvalidFormat {
                    token: token.to_string(),
                    reason: "First segment cannot be empty".to_string(),
                });
            }
        };

        // First character of prefix segment must be ASCII uppercase letter A-Z
        let mut chars = first.chars();
        match chars.next() {
            Some(c) if c.is_ascii_uppercase() => {}
            Some(c) => {
                return Err(IdError::InvalidFormat {
                    token: token.to_string(),
                    reason: format!(
                        "First character must be uppercase ASCII letter A-Z, found '{c}'"
                    ),
                });
            }
            None => {
                return Err(IdError::InvalidFormat {
                    token: token.to_string(),
                    reason: "Prefix segment cannot be empty".to_string(),
                });
            }
        }

        // Remaining characters in first segment must be ASCII uppercase or ASCII digit
        for c in chars {
            if !c.is_ascii_uppercase() && !c.is_ascii_digit() {
                return Err(IdError::InvalidFormat {
                    token: token.to_string(),
                    reason: format!(
                        "Invalid character '{c}' in prefix segment: only uppercase ASCII letters and digits allowed"
                    ),
                });
            }
        }

        // Must have at least one subsequent segment, each non-empty and containing only [A-Z0-9]
        let mut subsequent_count = 0usize;
        for segment in segments {
            subsequent_count += 1;
            if segment.is_empty() {
                return Err(IdError::InvalidFormat {
                    token: token.to_string(),
                    reason: "Consecutive hyphens are not permitted (empty segment)".to_string(),
                });
            }
            for c in segment.chars() {
                if !c.is_ascii_uppercase() && !c.is_ascii_digit() {
                    return Err(IdError::InvalidFormat {
                        token: token.to_string(),
                        reason: format!(
                            "Invalid character '{c}' in segment '{segment}': only uppercase ASCII letters and digits allowed"
                        ),
                    });
                }
            }
        }

        if subsequent_count == 0 {
            return Err(IdError::InvalidFormat {
                token: token.to_string(),
                reason: "Identifier must contain at least one hyphen-separated segment".to_string(),
            });
        }

        Ok(())
    }

    /// Returns true if candidate string is a valid `NodeId`.
    #[inline]
    pub fn is_valid(raw: &str) -> bool {
        Self::validate_grammar(raw).is_ok()
    }

    /// Returns the underlying string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extracts the prefix segment preceding the first hyphen.
    ///
    /// For example:
    /// - `"REQ-DOC-PARSE"` -> `"REQ"`
    /// - `"VIS-VCS-NATIVE"` -> `"VIS"`
    /// - `"SPEC-FMT-GRAMMAR"` -> `"SPEC"`
    #[inline]
    pub fn prefix(&self) -> &str {
        match self.0.split_once('-') {
            Some((prefix, _)) => prefix,
            None => &self.0,
        }
    }

    /// Infers the lifecycle `Layer` from the identifier's standard prefix.
    ///
    /// Returns `Err(IdError::UnknownPrefix)` if the prefix is not one of the standard
    /// specification layer prefixes (`VIS`, `STRAT`, `EPIC`, `US`, `REQ`, `SPEC`).
    pub fn infer_layer(&self) -> Result<Layer, IdError> {
        self.infer_layer_with_model(&crate::model::LayerModel::default())
    }

    /// Infers the layer using the supplied ordered layer model.
    pub fn infer_layer_with_model(
        &self,
        model: &crate::model::LayerModel,
    ) -> Result<Layer, IdError> {
        match model.layer_for_prefix(self.prefix()) {
            Some(layer) => Ok(layer),
            None => Err(IdError::UnknownPrefix {
                token: self.0.clone(),
                prefix: self.prefix().to_string(),
                valid_prefixes: model.valid_prefixes(),
            }),
        }
    }

    /// Returns the inferred `Layer` if prefix matches a standard specification layer, or `None`.
    #[inline]
    pub fn layer(&self) -> Option<Layer> {
        self.infer_layer().ok()
    }
}

impl Deref for NodeId {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for NodeId {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for NodeId {
    type Err = IdError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for NodeId {
    type Error = IdError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&str> for NodeId {
    type Error = IdError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<NodeId> for String {
    #[inline]
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// Validates that a child node of `child_layer` can legally declare a parent of `parent_layer`.
///
/// Returns `Ok(())` if the relationship is valid per the 7-layer hierarchy rules:
/// - Vision: 0 parents (cannot reference any parent).
/// - Strategy: Only Vision parents.
/// - Epic: Only Strategy parents.
/// - UserStory: Only Epic parents.
/// - Requirement: Only UserStory parents.
/// - TechnicalSpec: Requirement or UserStory parents.
/// - SourceCode: TechnicalSpec or Requirement parents.
/// - AutomatedTest: TechnicalSpec, Requirement, or UserStory parents.
///
/// If violated, returns `Err(GraphError::LayerInversion)`.
#[cfg(test)]
pub fn validate_parent_relationship(
    child_id: &NodeId,
    child_layer: Layer,
    parent_id: &NodeId,
    parent_layer: Layer,
) -> Result<(), GraphError> {
    if child_layer.is_root() {
        return Err(GraphError::LayerInversion {
            child_id: child_id.to_string(),
            child_layer: child_layer.to_string(),
            parent_id: parent_id.to_string(),
            parent_layer: parent_layer.to_string(),
        });
    }

    if !child_layer.can_have_parent_layer(parent_layer) {
        return Err(GraphError::LayerInversion {
            child_id: child_id.to_string(),
            child_layer: child_layer.to_string(),
            parent_id: parent_id.to_string(),
            parent_layer: parent_layer.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_node_ids() -> Result<(), Box<dyn std::error::Error>> {
        let valid_cases = [
            "VIS-VCS-NATIVE",
            "STRAT-PLAIN-TEXT",
            "EPIC-WORKSPACE",
            "US-DOC-01",
            "REQ-DOC-PARSE",
            "SPEC-FMT-GRAMMAR",
            "A-B",
            "A1-B2-C3",
            "REQ-1-A",
            "X-Y-Z",
            "V2-MIGRATION-STEP1",
            "LONG-ID-WITH-MULTIPLE-HYPHEN-SEPARATED-SEGMENTS-12345",
        ];

        for id_str in valid_cases {
            assert!(
                NodeId::is_valid(id_str),
                "Expected '{}' to be valid",
                id_str
            );
            let node_id = NodeId::new(id_str)?;
            assert_eq!(node_id.as_str(), id_str);
            assert_eq!(node_id.to_string(), id_str);
        }

        Ok(())
    }

    #[test]
    fn test_invalid_node_ids() {
        let invalid_cases = [
            ("", "empty"),
            ("REQ", "no hyphen segment"),
            ("req-01", "lowercase prefix"),
            ("Req-01", "mixed case prefix"),
            ("REQ-auth", "lowercase segment"),
            ("-REQ-01", "leading hyphen"),
            ("REQ-01-", "trailing hyphen"),
            ("REQ--01", "consecutive hyphens"),
            ("1-REQ", "starts with digit"),
            ("1REQ-01", "prefix starts with digit"),
            ("REQ_01", "contains underscore"),
            ("REQ-AUTH_01", "segment contains underscore"),
            (" REQ-01", "leading space"),
            ("REQ-01 ", "trailing space"),
            ("REQ - 01", "spaces around hyphen"),
            ("REQ.01", "contains dot"),
            ("REQ-01;", "contains semicolon"),
            ("REQ:01", "contains colon"),
            ("RÉQ-01", "non-ascii unicode character"),
        ];

        for (id_str, reason) in invalid_cases {
            assert!(
                !NodeId::is_valid(id_str),
                "Expected '{}' ({}) to be invalid",
                id_str,
                reason
            );
            assert!(NodeId::new(id_str).is_err());
        }
    }

    #[test]
    fn test_regex_parity_verification() -> Result<(), Box<dyn std::error::Error>> {
        let re = match regex::Regex::new(NODE_ID_REGEX_PATTERN) {
            Ok(r) => r,
            Err(e) => return Err(Box::new(e)),
        };

        let test_corpus = [
            ("VIS-01", true),
            ("STRAT-THEME-1", true),
            ("EPIC-WORKSPACE", true),
            ("US-AUTH-01", true),
            ("REQ-DOC-PARSE", true),
            ("SPEC-FMT-GRAMMAR", true),
            ("A-B", true),
            ("A1-B2", true),
            ("A-1-B-2", true),
            ("", false),
            ("A", false),
            ("REQ", false),
            ("req-01", false),
            ("-REQ-01", false),
            ("REQ-01-", false),
            ("REQ--01", false),
            ("1-REQ", false),
            ("REQ_01", false),
            ("REQ 01", false),
            ("REQ.01", false),
            ("REQ-01;", false),
        ];

        for (token, expected) in test_corpus {
            assert_eq!(
                re.is_match(token),
                expected,
                "Regex mismatch for '{}'",
                token
            );
            assert_eq!(
                NodeId::is_valid(token),
                expected,
                "Grammar validator mismatch for '{}'",
                token
            );
        }
        Ok(())
    }

    #[test]
    fn test_layer_prefix_inference() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(NodeId::new("VIS-VCS-NATIVE")?.infer_layer()?, Layer::Vision);
        assert_eq!(
            NodeId::new("STRAT-PLAIN-TEXT")?.infer_layer()?,
            Layer::Strategy
        );
        assert_eq!(NodeId::new("EPIC-WORKSPACE")?.infer_layer()?, Layer::Epic);
        assert_eq!(NodeId::new("US-DOC-01")?.infer_layer()?, Layer::UserStory);
        assert_eq!(
            NodeId::new("REQ-DOC-PARSE")?.infer_layer()?,
            Layer::Requirement
        );
        assert_eq!(
            NodeId::new("SPEC-FMT-GRAMMAR")?.infer_layer()?,
            Layer::TechnicalSpec
        );

        let custom_id = NodeId::new("CUSTOM-FEATURE-1")?;
        assert_eq!(custom_id.prefix(), "CUSTOM");
        assert!(custom_id.infer_layer().is_err());
        assert_eq!(custom_id.layer(), None);
        Ok(())
    }

    #[test]
    fn test_layer_parent_constraint_rules() -> Result<(), Box<dyn std::error::Error>> {
        let vis = NodeId::new("VIS-01")?;
        let strat = NodeId::new("STRAT-01")?;
        let epic = NodeId::new("EPIC-01")?;
        let us = NodeId::new("US-01")?;
        let req = NodeId::new("REQ-01")?;
        let spec = NodeId::new("SPEC-01")?;

        // 1. Vision cannot have parents
        assert!(
            validate_parent_relationship(&vis, Layer::Vision, &strat, Layer::Strategy).is_err()
        );
        assert!(validate_parent_relationship(&vis, Layer::Vision, &vis, Layer::Vision).is_err());

        // A node may reference any earlier layer in the hierarchy.
        assert!(validate_parent_relationship(&strat, Layer::Strategy, &vis, Layer::Vision).is_ok());
        assert!(validate_parent_relationship(&strat, Layer::Strategy, &epic, Layer::Epic).is_err());
        assert!(
            validate_parent_relationship(&strat, Layer::Strategy, &strat, Layer::Strategy).is_err()
        );

        // 3. Epic can parent to any higher layer.
        assert!(validate_parent_relationship(&epic, Layer::Epic, &strat, Layer::Strategy).is_ok());
        assert!(validate_parent_relationship(&epic, Layer::Epic, &vis, Layer::Vision).is_ok());
        assert!(validate_parent_relationship(&epic, Layer::Epic, &us, Layer::UserStory).is_err());

        // 4. UserStory can parent to any higher layer.
        assert!(validate_parent_relationship(&us, Layer::UserStory, &epic, Layer::Epic).is_ok());
        assert!(
            validate_parent_relationship(&us, Layer::UserStory, &strat, Layer::Strategy).is_ok()
        );
        assert!(
            validate_parent_relationship(&us, Layer::UserStory, &req, Layer::Requirement).is_err()
        );

        // 5. Requirement can parent to any higher layer.
        assert!(
            validate_parent_relationship(&req, Layer::Requirement, &us, Layer::UserStory).is_ok()
        );
        assert!(validate_parent_relationship(&req, Layer::Requirement, &epic, Layer::Epic).is_ok());
        assert!(validate_parent_relationship(
            &req,
            Layer::Requirement,
            &spec,
            Layer::TechnicalSpec
        )
        .is_err());

        // 6. TechnicalSpec can parent to any higher layer.
        assert!(validate_parent_relationship(
            &spec,
            Layer::TechnicalSpec,
            &req,
            Layer::Requirement
        )
        .is_ok());
        assert!(
            validate_parent_relationship(&spec, Layer::TechnicalSpec, &us, Layer::UserStory)
                .is_ok()
        );
        assert!(
            validate_parent_relationship(&spec, Layer::TechnicalSpec, &epic, Layer::Epic).is_ok()
        );
        assert!(
            validate_parent_relationship(&spec, Layer::TechnicalSpec, &vis, Layer::Vision).is_ok()
        );

        // 7. Leaf links
        let code_leaf = NodeId::new("CODE-01")?;
        assert!(validate_parent_relationship(
            &code_leaf,
            Layer::SourceCode,
            &spec,
            Layer::TechnicalSpec
        )
        .is_ok());
        assert!(validate_parent_relationship(
            &code_leaf,
            Layer::SourceCode,
            &req,
            Layer::Requirement
        )
        .is_ok());
        assert!(
            validate_parent_relationship(&code_leaf, Layer::SourceCode, &epic, Layer::Epic).is_ok()
        );

        let test_leaf = NodeId::new("TEST-01")?;
        assert!(validate_parent_relationship(
            &test_leaf,
            Layer::AutomatedTest,
            &spec,
            Layer::TechnicalSpec
        )
        .is_ok());
        assert!(validate_parent_relationship(
            &test_leaf,
            Layer::AutomatedTest,
            &req,
            Layer::Requirement
        )
        .is_ok());
        assert!(validate_parent_relationship(
            &test_leaf,
            Layer::AutomatedTest,
            &us,
            Layer::UserStory
        )
        .is_ok());
        assert!(validate_parent_relationship(
            &test_leaf,
            Layer::AutomatedTest,
            &vis,
            Layer::Vision
        )
        .is_ok());
        Ok(())
    }

    #[test]
    fn test_serde_json_roundtrip_and_validation() -> Result<(), Box<dyn std::error::Error>> {
        let original = NodeId::new("REQ-DOC-PARSE")?;
        let json_str = serde_json::to_string(&original)?;
        assert_eq!(json_str, "\"REQ-DOC-PARSE\"");

        let deserialized: NodeId = serde_json::from_str(&json_str)?;
        assert_eq!(deserialized, original);

        let invalid_json = "\"invalid_id\"";
        let err = serde_json::from_str::<NodeId>(invalid_json);
        assert!(err.is_err());
        Ok(())
    }
}
