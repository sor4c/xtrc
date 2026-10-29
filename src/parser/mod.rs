//! Specification document parsing and AST generation.
//!
//! // @trace SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE, SPEC-FMT-GRAMMAR

pub mod document;

pub use document::{
    parse_document, parse_document_file, parse_document_with_layers, parse_spec_content,
    parse_spec_content_with_layers, parse_spec_file, parse_spec_file_with_layers,
    DocumentFrontmatter, DocumentStatus, SpecDocument,
};
