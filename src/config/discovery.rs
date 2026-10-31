//! Upward repository root and configuration discovery engine.
//!
//! // @trace SPEC-DISC-DISCOVERY, REQ-CFG-DISCOVERY

use crate::config::WorkspaceConfig;
use crate::error::ConfigError;
use std::path::{Path, PathBuf};

/// Discovers the workspace root and loads the configuration.
///
/// Precedence:
/// 1. `config_override`: Explicit configuration path. Root is derived from `root_override`,
///    parent directory of the config file, or `start_dir`.
/// 2. `root_override`: Explicit workspace root. Loads `<root>/.xtrc.toml` if present,
///    or falls back to `WorkspaceConfig::default()`.
/// 3. Upward search: Starts at `start_dir` and checks each directory for `.xtrc.toml`, then `.git`.
///    Ascends until either boundary is found or filesystem root is reached.
pub fn discover_workspace(
    start_dir: &Path,
    root_override: Option<&Path>,
    config_override: Option<&Path>,
) -> Result<(PathBuf, WorkspaceConfig), ConfigError> {
    // 1. Explicit config override
    if let Some(cfg_path) = config_override {
        let config = WorkspaceConfig::load_from_file(cfg_path)?;
        let root = if let Some(r) = root_override {
            if !r.is_dir() {
                return Err(ConfigError::RootNotFound(r.to_path_buf()));
            }
            r.to_path_buf()
        } else if let Some(parent) = cfg_path.parent() {
            if parent.as_os_str().is_empty() {
                start_dir.to_path_buf()
            } else {
                parent.to_path_buf()
            }
        } else {
            start_dir.to_path_buf()
        };
        config.validate_relative_paths(&root)?;
        return Ok((root, config));
    }

    // 2. Explicit root override
    if let Some(root_path) = root_override {
        if !root_path.is_dir() {
            return Err(ConfigError::RootNotFound(root_path.to_path_buf()));
        }
        let candidate_config = root_path.join(".xtrc.toml");
        let config = if candidate_config.is_file() {
            WorkspaceConfig::load_from_file(&candidate_config)?
        } else {
            WorkspaceConfig::default()
        };
        config.validate_relative_paths(root_path)?;
        return Ok((root_path.to_path_buf(), config));
    }

    // 3. Upward directory search from start_dir
    let start_canonical = if start_dir.is_relative() {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(start_dir),
            Err(e) => {
                return Err(ConfigError::Io {
                    path: start_dir.to_path_buf(),
                    source: e,
                })
            }
        }
    } else {
        start_dir.to_path_buf()
    };

    let mut current = Some(start_canonical.as_path());
    while let Some(dir) = current {
        let candidate_config = dir.join(".xtrc.toml");
        if candidate_config.is_file() {
            let config = WorkspaceConfig::load_from_file(&candidate_config)?;
            config.validate_relative_paths(dir)?;
            return Ok((dir.to_path_buf(), config));
        }

        let candidate_git = dir.join(".git");
        if candidate_git.exists() {
            let config = WorkspaceConfig::default();
            config.validate_relative_paths(dir)?;
            return Ok((dir.to_path_buf(), config));
        }

        current = dir.parent();
    }

    Err(ConfigError::RootNotFound(start_dir.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discover_current_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let cfg_path = temp.path().join(".xtrc.toml");
        std::fs::write(&cfg_path, WorkspaceConfig::default_starter_toml())?;

        let (root, config) = discover_workspace(temp.path(), None, None)?;
        assert_eq!(root, temp.path());
        assert_eq!(config.project.name, "xtrc");
        Ok(())
    }

    #[test]
    fn test_discover_ascending_from_nested_subdir() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let cfg_path = temp.path().join(".xtrc.toml");
        std::fs::write(&cfg_path, WorkspaceConfig::default_starter_toml())?;

        let nested = temp.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested)?;

        let (root, config) = discover_workspace(&nested, None, None)?;
        assert_eq!(root, temp.path());
        assert_eq!(config.project.name, "xtrc");
        Ok(())
    }

    #[test]
    fn test_discover_git_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir)?;

        let (root, config) = discover_workspace(temp.path(), None, None)?;
        assert_eq!(root, temp.path());
        assert_eq!(config.project.name, "xtrc");
        Ok(())
    }

    #[test]
    fn test_config_override_precedence() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let custom_cfg = temp.path().join("custom.toml");
        let custom_content = r#"
        version = "1"
        [project]
        name = "custom-project"
        "#;
        std::fs::write(&custom_cfg, custom_content)?;

        let (root, config) = discover_workspace(temp.path(), None, Some(&custom_cfg))?;
        assert_eq!(root, temp.path());
        assert_eq!(config.project.name, "custom-project");
        Ok(())
    }

    #[test]
    fn test_root_override() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let sub = temp.path().join("sub_root");
        std::fs::create_dir_all(&sub)?;

        let (root, config) = discover_workspace(temp.path(), Some(&sub), None)?;
        assert_eq!(root, sub);
        assert_eq!(config.project.name, "xtrc");
        Ok(())
    }

    #[test]
    fn test_root_override_nonexistent_returns_root_not_found() {
        let nonexistent = Path::new("/nonexistent-xtrc-workspace-root-dir-123");
        let result = discover_workspace(Path::new("."), Some(nonexistent), None);
        match result {
            Err(ConfigError::RootNotFound(p)) => assert_eq!(p, nonexistent),
            other => panic!("Expected RootNotFound, got {other:?}"),
        }
    }
}
