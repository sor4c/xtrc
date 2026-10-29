//! Workspace configuration representation and serialization for xtrc.
//!
//! // @trace SPEC-DISC-INIT, SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY, REQ-CLI-INIT

use crate::error::ConfigError;
use crate::model::LayerModel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod discovery;
pub use discovery::{discover_workspace, find_project_root};

/// Root configuration structure matching the `.xtrc.toml` file schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Schema format version (default: "1").
    #[serde(default = "default_version")]
    pub version: String,

    /// Ordered specification layers. Omitted configurations use the built-in hierarchy.
    #[serde(default, skip_serializing_if = "is_default_layer_model")]
    pub layers: LayerModel,

    /// Project metadata configuration.
    #[serde(default)]
    pub project: ProjectConfig,

    /// Specification file scanning rules.
    #[serde(default)]
    pub specs: SpecsConfig,

    /// Source code scanning rules.
    #[serde(default)]
    pub code: CodeConfig,

    /// Automated test scanning rules.
    #[serde(default)]
    pub tests: TestsConfig,

    /// Traceability graph validation settings.
    #[serde(default)]
    pub validation: ValidationConfig,
}

/// Project metadata configuration section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Human-readable project name.
    #[serde(default = "default_project_name")]
    pub name: String,

    /// Project root path relative to configuration file.
    #[serde(default = "default_project_root")]
    pub root: String,
}

/// Specification file glob patterns and exclusion rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecsConfig {
    /// Glob patterns for specification documents.
    #[serde(default = "default_specs_include")]
    pub include: Vec<String>,

    /// Glob patterns for specification paths to ignore.
    #[serde(default = "default_specs_exclude")]
    pub exclude: Vec<String>,
}

/// Source code file glob patterns and extension filters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeConfig {
    /// Glob patterns for source code files.
    #[serde(default = "default_code_include")]
    pub include: Vec<String>,

    /// File extensions to include during code scanning.
    #[serde(default = "default_code_extensions")]
    pub extensions: Vec<String>,
}

/// Automated test file glob patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestsConfig {
    /// Glob patterns for test files.
    #[serde(default = "default_tests_include")]
    pub include: Vec<String>,
}

/// Traceability validation policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Whether strict validation mode is enabled (treating warnings as failures).
    #[serde(default = "default_validation_strict")]
    pub strict: bool,

    /// Whether requirements must link to automated test cases.
    #[serde(default = "default_validation_require_test_coverage")]
    pub require_test_coverage: bool,
}

fn default_version() -> String {
    "1".to_string()
}

fn is_default_layer_model(model: &LayerModel) -> bool {
    model.is_default()
}

fn default_project_name() -> String {
    "xtrc".to_string()
}

fn default_project_root() -> String {
    ".".to_string()
}

fn default_specs_include() -> Vec<String> {
    vec!["docs/**/*.md".to_string()]
}

fn default_specs_exclude() -> Vec<String> {
    vec![".agents/**".to_string(), "target/**".to_string()]
}

fn default_code_include() -> Vec<String> {
    vec!["src/**/*.rs".to_string(), "lib/**/*.*".to_string()]
}

fn default_code_extensions() -> Vec<String> {
    vec![
        ".rs".to_string(),
        ".py".to_string(),
        ".go".to_string(),
        ".ts".to_string(),
        ".js".to_string(),
        ".c".to_string(),
        ".cpp".to_string(),
    ]
}

fn default_tests_include() -> Vec<String> {
    vec!["tests/**/*.*".to_string()]
}

fn default_validation_strict() -> bool {
    false
}

fn default_validation_require_test_coverage() -> bool {
    true
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            layers: LayerModel::default(),
            project: ProjectConfig::default(),
            specs: SpecsConfig::default(),
            code: CodeConfig::default(),
            tests: TestsConfig::default(),
            validation: ValidationConfig::default(),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: default_project_name(),
            root: default_project_root(),
        }
    }
}

impl Default for SpecsConfig {
    fn default() -> Self {
        Self {
            include: default_specs_include(),
            exclude: default_specs_exclude(),
        }
    }
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            include: default_code_include(),
            extensions: default_code_extensions(),
        }
    }
}

impl Default for TestsConfig {
    fn default() -> Self {
        Self {
            include: default_tests_include(),
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict: default_validation_strict(),
            require_test_coverage: default_validation_require_test_coverage(),
        }
    }
}

impl WorkspaceConfig {
    /// Deserializes a WorkspaceConfig from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(content).map_err(ConfigError::ParseToml)?;
        config.validate_layers()?;
        Ok(config)
    }

    /// Serializes the WorkspaceConfig to a formatted TOML string.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::SerializeToml)
    }

    /// Loads and parses a WorkspaceConfig from a file path.
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let config: Self = toml::from_str(&content).map_err(|e| ConfigError::InvalidToml {
            path: path.to_path_buf(),
            source: e,
        })?;
        config.validate_layers()?;
        Ok(config)
    }

    fn validate_layers(&self) -> Result<(), ConfigError> {
        self.layers
            .validate()
            .map_err(ConfigError::InvalidLayerModel)
    }

    /// Validates that internal paths are relative to repository root.
    pub fn validate_relative_paths(&self, _root: &Path) -> Result<(), ConfigError> {
        if Path::new(&self.project.root).is_absolute() {
            return Err(ConfigError::InvalidRelativePath(PathBuf::from(
                &self.project.root,
            )));
        }
        for pattern in self
            .specs
            .include
            .iter()
            .chain(self.specs.exclude.iter())
            .chain(self.code.include.iter())
            .chain(self.tests.include.iter())
        {
            if Path::new(pattern).is_absolute() {
                return Err(ConfigError::InvalidRelativePath(PathBuf::from(pattern)));
            }
        }
        Ok(())
    }

    /// Canonical starter TOML string matching SPEC-DISC-INIT.
    pub fn default_starter_toml() -> &'static str {
        r#"version = "1"

[project]
name = "xtrc"
root = "."

[specs]
include = ["docs/**/*.md"]
exclude = [".agents/**", "target/**"]

[code]
include = ["src/**/*.rs", "lib/**/*.*"]
extensions = [".rs", ".py", ".go", ".ts", ".js", ".c", ".cpp"]

[tests]
include = ["tests/**/*.*"]

[validation]
strict = false
require_test_coverage = true
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_matches_spec() {
        let config = WorkspaceConfig::default();
        assert_eq!(config.version, "1");
        assert_eq!(config.project.name, "xtrc");
        assert_eq!(config.project.root, ".");
        assert_eq!(config.specs.include, vec!["docs/**/*.md"]);
        assert_eq!(config.specs.exclude, vec![".agents/**", "target/**"]);
        assert_eq!(config.code.include, vec!["src/**/*.rs", "lib/**/*.*"]);
        assert!(config.code.extensions.contains(&".rs".to_string()));
        assert_eq!(config.tests.include, vec!["tests/**/*.*"]);
        assert!(!config.validation.strict);
        assert!(config.validation.require_test_coverage);
    }

    #[test]
    fn test_starter_toml_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let starter = WorkspaceConfig::default_starter_toml();
        let parsed = WorkspaceConfig::from_toml(starter)?;
        assert_eq!(parsed, WorkspaceConfig::default());
        Ok(())
    }

    #[test]
    fn test_partial_toml_deserialization_uses_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let partial = r#"
        version = "1"
        [project]
        name = "my-project"
        "#;
        let parsed = WorkspaceConfig::from_toml(partial)?;
        assert_eq!(parsed.project.name, "my-project");
        assert_eq!(parsed.project.root, ".");
        assert_eq!(parsed.specs.include, vec!["docs/**/*.md"]);
        Ok(())
    }

    #[test]
    fn test_configured_layers_parse_and_validate() -> Result<(), Box<dyn std::error::Error>> {
        let config = WorkspaceConfig::from_toml(
            r#"
layers = [{ name = "Requirement", prefix = "REQ" }]
"#,
        )?;
        assert_eq!(config.layers.layers.len(), 1);
        assert_eq!(config.layers.layers[0].name, "Requirement");
        assert_eq!(config.layers.layers[0].prefix, "REQ");
        assert_eq!(
            config.layers.layer_for_prefix("REQ"),
            Some(crate::model::Layer::Custom(1))
        );
        assert_eq!(WorkspaceConfig::from_toml(&config.to_toml()?)?, config);
        Ok(())
    }

    #[test]
    fn test_invalid_configured_layers_are_rejected() {
        let empty = WorkspaceConfig::from_toml("layers = []");
        assert!(empty.is_err());

        let invalid_prefix =
            WorkspaceConfig::from_toml(r#"layers = [{ name = "Requirement", prefix = "req" }]"#);
        assert!(invalid_prefix.is_err());
    }

    #[test]
    fn test_validate_relative_paths() {
        let mut config = WorkspaceConfig::default();
        assert!(config.validate_relative_paths(Path::new("/root")).is_ok());

        config.project.root = "/absolute/root".to_string();
        assert!(config.validate_relative_paths(Path::new("/root")).is_err());
    }
}
