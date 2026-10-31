//! Markdown specification document and YAML frontmatter parser.
//!
//! // @trace SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE, SPEC-FMT-GRAMMAR, SPEC-PERF-BOUNDS, REQ-NFR-PERF

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;
use crate::model::{FileOrigin, Layer, LayerModel, NodeId, TraceNode};

/// Allowed lifecycle statuses for specification documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    /// Initial drafting state.
    Draft,
    /// Formally reviewed and approved.
    Approved,
    /// Deprecated or superseded.
    Deprecated,
}

impl DocumentStatus {
    /// Parses status case-insensitively.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "draft" => Some(Self::Draft),
            "approved" => Some(Self::Approved),
            "deprecated" => Some(Self::Deprecated),
            _ => None,
        }
    }

    /// Returns lowercase canonical representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Deprecated => "deprecated",
        }
    }
}

/// Typed representation of document-level YAML frontmatter metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentFrontmatter {
    /// Specification document type (e.g., "requirements_specification", "design_specification").
    pub document_type: String,

    /// Unique document-level identifier (e.g., "REQ-SPEC-001", "DES-SPEC-001").
    pub id: String,

    /// Human-readable document title.
    pub title: String,

    /// Specification semantic version string (e.g., "1.0.0").
    pub version: String,

    /// Document lifecycle status (draft, approved, deprecated).
    pub status: String,

    /// Optional extra unstructured frontmatter metadata (e.g. governance_tracking, schema_version).
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Parsed Markdown specification document containing frontmatter and extracted child nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecDocument {
    /// Deserialized document frontmatter metadata.
    pub frontmatter: DocumentFrontmatter,

    /// Path to the specification document file.
    pub path: PathBuf,

    /// Extracted child traceability nodes.
    pub nodes: Vec<TraceNode>,
}

/// Internal builder used to accumulate child node lines and metadata.
struct NodeBuilder<'a> {
    id: NodeId,
    title: String,
    line: usize,
    parents: Vec<NodeId>,
    explicit_type: Option<String>,
    attributes: HashMap<String, String>,
    body_lines: Vec<&'a str>,
}

impl NodeBuilder<'_> {
    fn finalize(self, path: &Path, layer_model: &LayerModel) -> Result<TraceNode, ParseError> {
        if self.id.prefix().len() < 2 {
            return Err(ParseError::GrammarViolation {
                token: self.id.to_string(),
                path: path.to_path_buf(),
                line: self.line,
                reason: format!(
                    "Identifier prefix '{}' is invalid: prefix must contain at least two characters",
                    self.id.prefix()
                ),
            });
        }

        let layer = if let Some(ref t) = self.explicit_type {
            let normalized = t.trim().to_lowercase();
            if let Some(layer) = layer_model.parse_layer(t) {
                layer
            } else if !layer_model.is_default() {
                match self.id.infer_layer_with_model(layer_model) {
                    Ok(layer) => layer,
                    Err(err) => {
                        return Err(ParseError::GrammarViolation {
                            token: self.id.to_string(),
                            path: path.to_path_buf(),
                            line: self.line,
                            reason: format!("Unrecognized layer type '{}' and {}", t, err),
                        });
                    }
                }
            } else {
                match normalized.as_str() {
                    "vision" | "vis" => Layer::Vision,
                    "strategy" | "strat" => Layer::Strategy,
                    "epic" => Layer::Epic,
                    "user story" | "userstory" | "user-story" | "user_story" | "story" | "us" => {
                        Layer::UserStory
                    }
                    "requirement"
                    | "req"
                    | "functional requirement"
                    | "non-functional requirement"
                    | "nfr" => Layer::Requirement,
                    "technical specification"
                    | "technical spec"
                    | "technicalspec"
                    | "technicalspecification"
                    | "technical-spec"
                    | "technical_spec"
                    | "specification"
                    | "spec" => Layer::TechnicalSpec,
                    _ => match Layer::from_str(t) {
                        Ok(l) if l.is_specification() => l,
                        _ => match self.id.infer_layer_with_model(layer_model) {
                            Ok(l) if l.is_specification() => l,
                            Err(err) => {
                                return Err(ParseError::GrammarViolation {
                                    token: self.id.to_string(),
                                    path: path.to_path_buf(),
                                    line: self.line,
                                    reason: format!("Unrecognized layer type '{}' and {}", t, err),
                                });
                            }
                            Ok(l) => {
                                return Err(ParseError::GrammarViolation {
                                token: self.id.to_string(),
                                path: path.to_path_buf(),
                                line: self.line,
                                reason: format!(
                                    "Unrecognized layer type '{}' and leaf layer '{}' is not permitted in specification documents",
                                    t, l
                                ),
                            });
                            }
                        },
                    },
                }
            }
        } else {
            match self.id.infer_layer_with_model(layer_model) {
                Ok(l) => l,
                Err(err) => {
                    return Err(ParseError::GrammarViolation {
                        token: self.id.to_string(),
                        path: path.to_path_buf(),
                        line: self.line,
                        reason: format!("Cannot infer layer from identifier: {}", err),
                    });
                }
            }
        };

        let file_origin = FileOrigin::new(path.to_path_buf(), self.line);
        let description = self.body_lines.join("\n").trim().to_string();

        let mut node = TraceNode::new(
            self.id,
            self.title,
            layer,
            self.parents,
            file_origin,
            description,
        );
        node = node.with_attributes(self.attributes);
        Ok(node)
    }
}

/// Parses the YAML frontmatter from lines starting at index 0.
///
/// Returns the validated `DocumentFrontmatter` and the 1-indexed line number where the body begins.
fn parse_frontmatter(
    content: &str,
    path: &Path,
) -> Result<(DocumentFrontmatter, usize), ParseError> {
    if content.trim().is_empty() {
        return Err(ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: "Specification file is empty".to_string(),
        });
    }

    let trimmed_start = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut line_iter = trimmed_start.split_inclusive('\n');

    let first_line = match line_iter.next() {
        Some(line) if line.trim() == "---" => line,
        _ => {
            return Err(ParseError::InvalidFrontmatter {
                path: path.to_path_buf(),
                reason: "Specification file must begin with '---' YAML frontmatter fence"
                    .to_string(),
            });
        }
    };

    let yaml_start_byte = first_line.len();
    let mut current_byte = yaml_start_byte;
    let mut closing_info = None;

    for (idx_after_first, line) in line_iter.enumerate() {
        if line.trim() == "---" {
            closing_info = Some((idx_after_first + 1, current_byte));
            break;
        }
        current_byte += line.len();
    }

    let (closing_line_idx, yaml_end_byte) = match closing_info {
        Some(info) => info,
        None => {
            return Err(ParseError::InvalidFrontmatter {
                path: path.to_path_buf(),
                reason: "Unclosed YAML frontmatter: missing closing '---' fence".to_string(),
            });
        }
    };

    let yaml_slice = &trimmed_start[yaml_start_byte..yaml_end_byte];

    let mut frontmatter: DocumentFrontmatter =
        serde_yaml::from_str(yaml_slice).map_err(|err| ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: format!("YAML deserialization error: {err}"),
        })?;

    // Validate required fields
    if frontmatter.document_type.trim().is_empty() {
        return Err(ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: "Missing or empty required frontmatter field 'document_type'".to_string(),
        });
    }

    if frontmatter.id.trim().is_empty() {
        return Err(ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: "Missing or empty required frontmatter field 'id'".to_string(),
        });
    }

    if frontmatter.title.trim().is_empty() {
        return Err(ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: "Missing or empty required frontmatter field 'title'".to_string(),
        });
    }

    if frontmatter.version.trim().is_empty() {
        return Err(ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: "Missing or empty required frontmatter field 'version'".to_string(),
        });
    }

    if frontmatter.status.trim().is_empty() {
        return Err(ParseError::InvalidFrontmatter {
            path: path.to_path_buf(),
            reason: "Missing or empty required frontmatter field 'status'".to_string(),
        });
    }

    let status_enum = match DocumentStatus::from_str_loose(&frontmatter.status) {
        Some(s) => s,
        None => {
            return Err(ParseError::InvalidFrontmatter {
                path: path.to_path_buf(),
                reason: format!(
                    "Invalid document status '{}'. Expected 'draft', 'approved', or 'deprecated'",
                    frontmatter.status
                ),
            });
        }
    };
    frontmatter.status = status_enum.as_str().to_string();

    let body_start_line = closing_line_idx + 2; // 1-indexed, line after closing fence
    Ok((frontmatter, body_start_line))
}

/// Parses a bullet line in the form `- **Key**: Value` or `* **Key**: Value`.
fn parse_bullet_item(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))?;

    let rest = rest.trim_start();
    let after_bold_open = rest.strip_prefix("**")?;
    let (key, after_bold_close) = after_bold_open.split_once("**:")?;
    Some((key.trim(), after_bold_close.trim()))
}

/// Determines whether a heading string looks like an attempted specification node ID.
fn is_attempted_node_id(candidate: &str) -> bool {
    let upper = candidate.to_ascii_uppercase();
    let prefixes = [
        "VIS-", "STRAT-", "EPIC-", "US-", "REQ-", "SPEC-", "VIS_", "STRAT_", "EPIC_", "US_",
        "REQ_", "SPEC_",
    ];
    for p in prefixes {
        if upper.starts_with(p) {
            return true;
        }
    }
    false
}

/// Returns true if markdown content contains any heading that defines or attempts to define a spec node.
pub fn has_spec_headings(content: &str) -> bool {
    let trimmed_start = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut in_code_block = false;

    for line in trimmed_start.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with("```") || trimmed_line.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        let is_indented_code = line.starts_with("    ") || line.starts_with('\t');
        if !is_indented_code && trimmed_line.starts_with('#') {
            let hash_count = trimmed_line.bytes().take_while(|&b| b == b'#').count();
            if hash_count == 3 || hash_count == 4 {
                let remainder = trimmed_line[hash_count..].trim_start();
                let (id_part, _) = match remainder.split_once(':') {
                    Some((id_str, _)) => (id_str.trim(), ""),
                    None => (remainder.trim(), ""),
                };
                if NodeId::is_valid(id_part) || is_attempted_node_id(id_part) {
                    return true;
                }
            }
        }
    }
    false
}

/// Returns true if content begins with YAML frontmatter fence `---`.
pub fn has_frontmatter(content: &str) -> bool {
    let trimmed_start = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut line_iter = trimmed_start.split_inclusive('\n');
    matches!(line_iter.next(), Some(line) if line.trim() == "---")
}

/// Parses Markdown specification content string into a structured `SpecDocument` AST.
#[cfg(test)]
pub fn parse_document(content: &str, path: &Path) -> Result<SpecDocument, ParseError> {
    parse_document_with_layers(content, path, &LayerModel::default())
}

/// Parses Markdown specification content using the configured layer hierarchy.
pub fn parse_document_with_layers(
    content: &str,
    path: &Path,
    layer_model: &LayerModel,
) -> Result<SpecDocument, ParseError> {
    let (frontmatter, body_start_line) = parse_frontmatter(content, path)?;

    let trimmed_start = content.strip_prefix('\u{feff}').unwrap_or(content);

    let mut in_code_block = false;
    let mut current_node: Option<NodeBuilder<'_>> = None;
    let mut nodes: Vec<TraceNode> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();

    let start_idx = body_start_line.saturating_sub(1);
    for (line_idx, line) in trimmed_start.lines().enumerate().skip(start_idx) {
        let line_number = line_idx + 1;
        let trimmed_line = line.trim();

        // 1. Code block fence detection
        if trimmed_line.starts_with("```") || trimmed_line.starts_with("~~~") {
            in_code_block = !in_code_block;
            if let Some(ref mut builder) = current_node {
                builder.body_lines.push(line);
            }
            continue;
        }

        if in_code_block {
            if let Some(ref mut builder) = current_node {
                builder.body_lines.push(line);
            }
            continue;
        }

        // 2. Headings (when not in code block)
        // Indented with 4+ spaces is treated as code block / list continuation
        let is_indented_code = line.starts_with("    ") || line.starts_with('\t');
        if !is_indented_code && trimmed_line.starts_with('#') {
            // Count leading '#' characters
            let hash_count = trimmed_line.bytes().take_while(|&b| b == b'#').count();
            let remainder = trimmed_line[hash_count..].trim_start();

            if hash_count == 1 || hash_count == 2 {
                // Section break at H1 or H2: finalize current node if any
                if let Some(builder) = current_node.take() {
                    match builder.finalize(path, layer_model) {
                        Ok(n) => nodes.push(n),
                        Err(e) => errors.push(e),
                    }
                }
                continue;
            }

            if hash_count == 3 || hash_count == 4 {
                // Check if this heading defines a child node: `### <ID>: <Title>` or `### <ID>`
                let (id_part, title_part) = match remainder.split_once(':') {
                    Some((id_str, title_str)) => (id_str.trim(), title_str.trim()),
                    None => (remainder.trim(), ""),
                };

                if NodeId::is_valid(id_part) {
                    if let Some(builder) = current_node.take() {
                        match builder.finalize(path, layer_model) {
                            Ok(n) => nodes.push(n),
                            Err(e) => errors.push(e),
                        }
                    }

                    let node_id = match NodeId::new(id_part) {
                        Ok(id) => id,
                        Err(err) => {
                            errors.push(err.into_parse_error(path, line_number));
                            continue;
                        }
                    };

                    current_node = Some(NodeBuilder {
                        id: node_id,
                        title: title_part.to_string(),
                        line: line_number,
                        parents: Vec::new(),
                        explicit_type: None,
                        attributes: HashMap::new(),
                        body_lines: Vec::new(),
                    });
                    continue;
                } else if is_attempted_node_id(id_part) {
                    // Malformed attempted specification node ID
                    let validation_err = NodeId::validate_grammar(id_part);
                    let reason = match validation_err {
                        Err(e) => e.to_string(),
                        Ok(()) => "Invalid identifier".to_string(),
                    };
                    errors.push(ParseError::GrammarViolation {
                        token: id_part.to_string(),
                        path: path.to_path_buf(),
                        line: line_number,
                        reason,
                    });
                    continue;
                } else {
                    // Regular markdown H3/H4 heading (e.g. `### 8.1 Forward Requirements...`)
                    if let Some(ref mut builder) = current_node {
                        builder.body_lines.push(line);
                    }
                    continue;
                }
            }
        }

        // 3. Bullet metadata item parsing (when inside an active node)
        if let Some(ref mut builder) = current_node {
            if let Some((key, val)) = parse_bullet_item(line) {
                if key.eq_ignore_ascii_case("id") {
                    // Verify ID matches heading ID
                    if val != builder.id.as_str() {
                        errors.push(ParseError::GrammarViolation {
                            token: val.to_string(),
                            path: path.to_path_buf(),
                            line: line_number,
                            reason: format!(
                                "ID bullet point '{}' does not match heading identifier '{}'",
                                val, builder.id
                            ),
                        });
                    }
                } else if key.eq_ignore_ascii_case("parent")
                    || key.eq_ignore_ascii_case("parents")
                    || key.eq_ignore_ascii_case("parent requirement")
                    || key.eq_ignore_ascii_case("parent requirements")
                {
                    if !val.eq_ignore_ascii_case("none") && !val.is_empty() {
                        let unbracketed = val.trim_start_matches('[').trim_end_matches(']');
                        for part in unbracketed.split(',') {
                            let clean_part = part.trim().trim_matches(|c| c == '\'' || c == '"');
                            if clean_part.is_empty() || clean_part.eq_ignore_ascii_case("none") {
                                continue;
                            }
                            match NodeId::new(clean_part) {
                                Ok(parent_id) => {
                                    if !builder.parents.contains(&parent_id) {
                                        builder.parents.push(parent_id);
                                    }
                                }
                                Err(err) => {
                                    errors.push(err.into_parse_error(path, line_number));
                                }
                            }
                        }
                    }
                } else if key.eq_ignore_ascii_case("type") || key.eq_ignore_ascii_case("layer") {
                    builder.explicit_type = Some(val.to_string());
                }

                builder.attributes.insert(key.to_string(), val.to_string());
            }

            builder.body_lines.push(line);
        }
    }

    if let Some(builder) = current_node.take() {
        match builder.finalize(path, layer_model) {
            Ok(n) => nodes.push(n),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        if errors.len() == 1 {
            return Err(errors.remove(0));
        } else {
            return Err(ParseError::Multiple(errors));
        }
    }

    Ok(SpecDocument {
        frontmatter,
        path: path.to_path_buf(),
        nodes,
    })
}

/// Parses specification content using the configured layer hierarchy.
pub fn parse_spec_content_with_layers(
    content: &str,
    path: &Path,
    layer_model: &LayerModel,
) -> Result<Vec<TraceNode>, ParseError> {
    let doc = parse_document_with_layers(content, path, layer_model)?;
    Ok(doc.nodes)
}

/// Reads a specification file from disk and parses it into a structured `SpecDocument` AST.
#[cfg(test)]
pub fn parse_document_file(path: &Path) -> Result<SpecDocument, ParseError> {
    let content = fs::read_to_string(path).map_err(|err| ParseError::Io {
        path: path.to_path_buf(),
        source: err,
    })?;
    parse_document(&content, path)
}

/// Primary M2 ↔ M3 interface: reads a specification file from disk and returns extracted child `TraceNode`s.
#[cfg(test)]
pub fn parse_spec_file(path: &Path) -> Result<Vec<TraceNode>, ParseError> {
    let doc = parse_document_file(path)?;
    Ok(doc.nodes)
}

/// Reads and parses a specification file using the configured layer hierarchy.
pub fn parse_spec_file_with_layers(
    path: &Path,
    layer_model: &LayerModel,
) -> Result<Vec<TraceNode>, ParseError> {
    let content = fs::read_to_string(path).map_err(|err| ParseError::Io {
        path: path.to_path_buf(),
        source: err,
    })?;
    parse_spec_content_with_layers(&content, path, layer_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_valid() -> Result<(), Box<dyn std::error::Error>> {
        let content = r#"---
document_type: requirements_specification
id: REQ-SPEC-001
title: xtrc Foundational Requirements Specification
version: 1.0.0
status: approved
---
# Content here
"#;
        let (fm, body_start) = parse_frontmatter(content, Path::new("test.md"))?;
        assert_eq!(fm.document_type, "requirements_specification");
        assert_eq!(fm.id, "REQ-SPEC-001");
        assert_eq!(fm.title, "xtrc Foundational Requirements Specification");
        assert_eq!(fm.version, "1.0.0");
        assert_eq!(fm.status, "approved");
        assert_eq!(body_start, 8);
        Ok(())
    }

    #[test]
    fn test_parse_frontmatter_missing_fences() {
        let content = "document_type: req\n# Content";
        let err = parse_frontmatter(content, Path::new("test.md"));
        assert!(err.is_err());

        let unclosed = "---\ndocument_type: req\n# Content";
        let err = parse_frontmatter(unclosed, Path::new("test.md"));
        assert!(err.is_err());
    }

    #[test]
    fn test_parse_frontmatter_empty() {
        let err = parse_frontmatter("", Path::new("test.md"));
        assert!(err.is_err());
    }

    #[test]
    fn test_parse_frontmatter_invalid_status() {
        let content = r#"---
document_type: req
id: REQ-01
title: Title
version: 1.0.0
status: invalid_status
---
"#;
        let err = parse_frontmatter(content, Path::new("test.md"));
        assert!(err.is_err());
    }

    #[test]
    fn test_parse_document_single_node() -> Result<(), Box<dyn std::error::Error>> {
        let content = r#"---
document_type: requirements_specification
id: REQ-SPEC-001
title: Requirements
version: 1.0.0
status: approved
---

## Section 1

### REQ-CLI-INIT: Workspace Initialization
- **ID**: REQ-CLI-INIT
- **Parent**: US-INIT-01
- **Type**: Functional Requirement
- **Intent**: Scaffolds workspace

Some long body text here.
"#;
        let doc = parse_document(content, Path::new("docs/req.md"))?;
        assert_eq!(doc.nodes.len(), 1);
        let node = &doc.nodes[0];
        assert_eq!(node.id().as_str(), "REQ-CLI-INIT");
        assert_eq!(node.title(), "Workspace Initialization");
        assert_eq!(node.layer(), Layer::Requirement);
        assert_eq!(node.parents().len(), 1);
        assert_eq!(node.parents()[0].as_str(), "US-INIT-01");
        assert_eq!(node.file_origin().line(), 11);
        assert!(node.description().contains("Some long body text here."));
        Ok(())
    }

    #[test]
    fn test_parse_bracketed_and_multiple_parents() -> Result<(), Box<dyn std::error::Error>> {
        let content = r#"---
document_type: design_specification
id: DES-SPEC-001
title: Design
version: 1.0.0
status: approved
---

### SPEC-DAG-VALIDATE: Validation Engine
- **ID**: SPEC-DAG-VALIDATE
- **Parent Requirements**: REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST
- **Parents**: [US-DOC-01, US-CODE-01]
- **Target**: src/graph/validator.rs
"#;
        let doc = parse_document(content, Path::new("docs/design.md"))?;
        assert_eq!(doc.nodes.len(), 1);
        let node = &doc.nodes[0];
        assert_eq!(node.id().as_str(), "SPEC-DAG-VALIDATE");
        assert_eq!(node.layer(), Layer::TechnicalSpec);
        // Should have 5 distinct parents
        assert_eq!(node.parents().len(), 5);
        assert!(node
            .parents()
            .iter()
            .any(|p| p.as_str() == "REQ-BROKEN-LINK"));
        assert!(node.parents().iter().any(|p| p.as_str() == "US-DOC-01"));
        Ok(())
    }

    #[test]
    fn test_code_blocks_ignore_headings() -> Result<(), Box<dyn std::error::Error>> {
        let content = r#"---
document_type: design_specification
id: DES-SPEC-001
title: Design
version: 1.0.0
status: approved
---

### SPEC-DISC-INIT: Init Scaffold
- **ID**: SPEC-DISC-INIT
- **Parent**: REQ-CLI-INIT

```toml
# This is a comment
### NOT-A-NODE: Fake Title
key = "value"
```

After code block.
"#;
        let doc = parse_document(content, Path::new("docs/design.md"))?;
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].id().as_str(), "SPEC-DISC-INIT");
        assert!(doc.nodes[0]
            .description()
            .contains("### NOT-A-NODE: Fake Title"));
        Ok(())
    }

    #[test]
    fn test_id_mismatch_fails() {
        let content = r#"---
document_type: req
id: REQ-SPEC-001
title: Requirements
version: 1.0.0
status: approved
---

### REQ-01: Title
- **ID**: REQ-02
- **Parent**: None
"#;
        let res = parse_document(content, Path::new("docs/req.md"));
        assert!(res.is_err());
    }

    #[test]
    fn test_custom_layer_prefix_and_type_are_parsed() -> Result<(), Box<dyn std::error::Error>> {
        let model = LayerModel {
            layers: vec![crate::model::LayerDefinition {
                name: "Requirement".to_string(),
                prefix: "REQ".to_string(),
            }],
        };
        let content = r#"---
document_type: requirements_specification
id: REQ-SPEC-001
title: Requirements
version: 1.0.0
status: approved
---

### REQ-01: One requirement
- **Type**: Requirement
- **Parent**: None
"#;
        let doc = parse_document_with_layers(content, Path::new("docs/req.md"), &model)?;
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].layer, Layer::Custom(1));
        Ok(())
    }

    #[test]
    fn test_custom_layer_unknown_prefix_diagnostic_lists_valid_prefixes() {
        let model = LayerModel {
            layers: vec![crate::model::LayerDefinition {
                name: "Requirement".to_string(),
                prefix: "REQ".to_string(),
            }],
        };
        let content = r#"---
document_type: requirements_specification
id: REQ-SPEC-001
title: Requirements
version: 1.0.0
status: approved
---

### OTHER-01: Unknown layer
"#;
        let error = parse_document_with_layers(content, Path::new("docs/req.md"), &model);
        assert!(error.is_err());
        assert!(error
            .err()
            .is_some_and(|error| error.to_string().contains("Valid prefixes: REQ")));
    }
}
