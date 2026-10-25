//! Workspace scanning, ignore-aware filesystem traversal, and asset inspection.
//!
//! // @trace SPEC-DISC-TRAVERSAL, SPEC-SCAN-COMMENTS, REQ-IGNORE-RULES, REQ-CODE-SCAN, REQ-TEST-SCAN

pub mod comments;
pub mod traversal;

pub use comments::{
    classify_link_kind, clean_id_token, is_test_file_path, scan_comment_line, scan_comments_file,
    scan_comments_text, scan_workspace_comments,
};
pub use traversal::{
    build_glob_set, categorize_files, create_walk_builder, glob_match, is_binary_buffer,
    is_binary_file, matches_any_glob, should_skip_entry, sniff_binary, walk_workspace,
    walk_workspace_sync, CategorizedFiles, BINARY_SNIFF_BUFFER_SIZE,
};
