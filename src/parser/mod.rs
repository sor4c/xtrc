//! Specification document parsing and AST generation.
//!
//! // @trace SPEC-FMT-FRONTMATTER, REQ-DOC-PARSE, SPEC-FMT-GRAMMAR

pub mod document;

pub use document::{
    parse_document, parse_document_file, parse_spec_content, parse_spec_file, DocumentFrontmatter,
    DocumentStatus, SpecDocument,
};
