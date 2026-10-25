//! Structured domain error types for xtrc.
//!
//! // @trace SPEC-ERR-HIERARCHY, REQ-NFR-ROBUST, REQ-EXIT-CODES

use std::path::PathBuf;

/// Domain errors encountered during workspace configuration and root discovery.
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// Dispatched when neither `.xtrc.toml` nor `.git` is found during upward directory traversal.
    #[error("Could not find repository root or .git directory from '{0}'")]
    RootNotFound(PathBuf),

    /// Dispatched when a configuration file contains invalid TOML syntax.
    #[error("Invalid TOML syntax in '{path}': {source}")]
    InvalidToml {
        /// File path where syntax error occurred.
        path: PathBuf,
        /// The underlying TOML deserialization error.
        #[source]
        source: toml::de::Error,
    },

    /// Dispatched when parsing TOML string directly fails.
    #[error("Failed to parse TOML string: {0}")]
    ParseToml(#[source] toml::de::Error),

    /// Dispatched when serializing configuration to TOML fails.
    #[error("Failed to serialize TOML: {0}")]
    SerializeToml(#[source] toml::ser::Error),

    /// Dispatched when a path in the configuration is absolute instead of relative to root.
    #[error("Configured path '{0}' must be relative to workspace root")]
    InvalidRelativePath(PathBuf),

    /// Dispatched when a file system I/O error occurs while reading configuration.
    #[error("I/O error reading configuration '{path}': {source}")]
    Io {
        /// Path of the file being accessed.
        path: PathBuf,
        /// Underlying std I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Dispatched when attempting to initialize a workspace that already contains `.xtrc.toml` without `--force`.
    #[error("Workspace already initialized at '{path}'. Use --force to overwrite.")]
    AlreadyInitialized {
        /// Path to the existing configuration file.
        path: PathBuf,
    },
}

impl ConfigError {
    /// Returns the deterministic process exit code for this configuration error.
    pub fn exit_code(&self) -> i32 {
        2
    }
}

/// Domain errors encountered while parsing markdown specifications or code comments.
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    /// Dispatched when YAML frontmatter is missing, unclosed, or fails schema validation.
    #[error("Malformed YAML frontmatter in '{path}': {reason}")]
    InvalidFrontmatter {
        /// Path to the file with invalid frontmatter.
        path: PathBuf,
        /// Human-readable explanation of the validation failure.
        reason: String,
    },

    /// Dispatched when an identifier violates the grammar `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`.
    #[error("Invalid identifier token '{token}' at {path}:{line}: {reason}")]
    GrammarViolation {
        /// The invalid token string.
        token: String,
        /// File path where the token was found.
        path: PathBuf,
        /// 1-indexed line number where the token was found.
        line: usize,
        /// Human-readable reason for the grammar violation.
        reason: String,
    },

    /// Dispatched when file I/O fails during specification or comment reading.
    #[error("I/O error reading specification '{path}': {source}")]
    Io {
        /// File path being read.
        path: PathBuf,
        /// Underlying std I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl ParseError {
    /// Returns the deterministic process exit code for this parse error.
    pub fn exit_code(&self) -> i32 {
        2
    }
}

/// Domain errors encountered during graph construction, cycle detection, or validation.
#[derive(thiserror::Error, Debug)]
pub enum GraphError {
    /// Dispatched when two specification nodes declare the same identifier.
    #[error("Duplicate identifier '{id}' found in {path_a}:{line_a} and {path_b}:{line_b}")]
    DuplicateIdentifier {
        /// The colliding identifier.
        id: String,
        /// First file where identifier was declared.
        path_a: PathBuf,
        /// Line number in first file.
        line_a: usize,
        /// Second file where duplicate identifier was declared.
        path_b: PathBuf,
        /// Line number in second file.
        line_b: usize,
    },

    /// Dispatched when a link points to a non-existent identifier.
    #[error("Broken link in {origin}: target '{target_id}' does not exist")]
    BrokenLink {
        /// Provenance location of the broken link (e.g. `src/foo.rs:42`).
        origin: String,
        /// Target identifier that was not found.
        target_id: String,
    },

    /// Dispatched when a directed cycle is detected in the graph hierarchy.
    #[error("Cyclic dependency detected: {cycle_chain}")]
    CycleDetected {
        /// Formatted loop path showing the circular chain.
        cycle_chain: String,
    },

    /// Dispatched when a child references an ancestor in an inverted layer hierarchy.
    #[error("Layer inversion detected: child '{child_id}' ({child_layer}) cannot reference parent '{parent_id}' ({parent_layer})")]
    LayerInversion {
        /// Child identifier.
        child_id: String,
        /// Child layer name.
        child_layer: String,
        /// Parent identifier.
        parent_id: String,
        /// Parent layer name.
        parent_layer: String,
    },

    /// Dispatched when strict mode validation fails due to gaps or integrity violations.
    #[error("Traceability verification failed: {violations} violation(s) found")]
    VerificationFailed {
        /// Total number of violations.
        violations: usize,
    },
}

impl GraphError {
    /// Returns the deterministic process exit code for this graph error.
    pub fn exit_code(&self) -> i32 {
        1
    }
}

/// Top-level application and CLI errors.
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    /// Wrapped configuration error.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Wrapped parse error.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Wrapped graph error.
    #[error(transparent)]
    Graph(#[from] GraphError),

    /// Dispatched when attempting to initialize a workspace where configuration already exists without `--force`.
    #[error("Workspace already initialized at '{0}'. Use --force to overwrite.")]
    ConfigAlreadyExists(PathBuf),

    /// Requested node identifier was not found in the graph.
    #[error("Node '{id}' not found in traceability graph")]
    NodeNotFound {
        /// The missing identifier.
        id: String,
    },

    /// Invalid CLI arguments or flag combinations.
    #[error("Invalid CLI argument: {reason}")]
    InvalidArguments {
        /// Description of the argument issue.
        reason: String,
    },

    /// Unsupported output format requested.
    #[error("Unsupported output format '{format}'. Supported formats: table, markdown, csv, json")]
    InvalidFormat {
        /// The invalid format string requested.
        format: String,
    },

    /// General I/O error at the CLI boundary.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Verification gate failure.
    #[error("Traceability check failed with {violations} violation(s)")]
    VerificationFailure {
        /// Count of violations.
        violations: usize,
    },
}

impl CliError {
    /// Returns the deterministic Unix exit code for this CLI error.
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Config(err) => err.exit_code(),
            CliError::Parse(err) => err.exit_code(),
            CliError::Graph(err) => err.exit_code(),
            CliError::ConfigAlreadyExists(_) => 2,
            CliError::NodeNotFound { .. } => 1,
            CliError::VerificationFailure { .. } => 1,
            CliError::InvalidArguments { .. } => 2,
            CliError::InvalidFormat { .. } => 2,
            CliError::Io(_) => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_exit_codes() {
        let err = ConfigError::RootNotFound(PathBuf::from("/workspace"));
        assert_eq!(err.exit_code(), 2);
        assert!(err.to_string().contains("/workspace"));

        let cli_err = CliError::from(err);
        assert_eq!(cli_err.exit_code(), 2);

        let init_err = ConfigError::AlreadyInitialized {
            path: PathBuf::from("/workspace/.xtrc.toml"),
        };
        assert_eq!(init_err.exit_code(), 2);
        assert!(init_err.to_string().contains(".xtrc.toml"));

        let rel_err = ConfigError::InvalidRelativePath(PathBuf::from("/abs/path"));
        assert_eq!(rel_err.exit_code(), 2);
    }

    #[test]
    fn test_parse_error_exit_codes() {
        let err = ParseError::GrammarViolation {
            token: "invalid_id".to_string(),
            path: PathBuf::from("docs/test.md"),
            line: 12,
            reason: "Must be uppercase alphanumeric with hyphens".to_string(),
        };
        assert_eq!(err.exit_code(), 2);
        assert!(err.to_string().contains("invalid_id"));

        let cli_err = CliError::from(err);
        assert_eq!(cli_err.exit_code(), 2);
    }

    #[test]
    fn test_graph_error_exit_codes() {
        let err = GraphError::BrokenLink {
            origin: "src/main.rs:10".to_string(),
            target_id: "REQ-MISSING".to_string(),
        };
        assert_eq!(err.exit_code(), 1);
        assert!(err.to_string().contains("REQ-MISSING"));

        let cli_err = CliError::from(err);
        assert_eq!(cli_err.exit_code(), 1);

        let cycle_err = GraphError::CycleDetected {
            cycle_chain: "A -> B -> A".to_string(),
        };
        assert_eq!(cycle_err.exit_code(), 1);

        let inv_err = GraphError::LayerInversion {
            child_id: "VIS-1".to_string(),
            child_layer: "Vision".to_string(),
            parent_id: "REQ-1".to_string(),
            parent_layer: "Requirement".to_string(),
        };
        assert_eq!(inv_err.exit_code(), 1);
    }

    #[test]
    fn test_cli_error_exit_codes() {
        let err = CliError::NodeNotFound {
            id: "VIS-NONEXISTENT".to_string(),
        };
        assert_eq!(err.exit_code(), 1);

        let err = CliError::InvalidFormat {
            format: "xml".to_string(),
        };
        assert_eq!(err.exit_code(), 2);

        let err = CliError::ConfigAlreadyExists(PathBuf::from(".xtrc.toml"));
        assert_eq!(err.exit_code(), 2);
    }
}
