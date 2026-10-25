//! Handler for the `xtrc init` command.
//!
//! // @trace SPEC-DISC-INIT, REQ-CLI-INIT

use crate::config::WorkspaceConfig;
use crate::error::CliError;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_tmp_filename() -> String {
    let pid = std::process::id();
    let thread_raw = format!("{:?}", std::thread::current().id());
    let thread_clean: String = thread_raw.chars().filter(|c| c.is_alphanumeric()).collect();
    let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    format!(".xtrc.toml.tmp.{pid}_{thread_clean}_{nanos}_{counter}")
}

/// Options configuring workspace initialization.
#[derive(Debug, Clone, Default)]
pub struct InitOptions<'a> {
    /// Target directory path where workspace is initialized (defaults to current working directory).
    pub target_path: Option<&'a Path>,

    /// Whether to overwrite an existing `.xtrc.toml` file.
    pub force: bool,

    /// Whether to suppress confirmation messages on standard output.
    pub quiet: bool,
}

/// Executes workspace initialization.
///
/// Creates `.xtrc.toml` with default configuration and initializes `docs/` starter directory.
/// Fails with exit code 2 (`CliError::ConfigAlreadyExists`) if `.xtrc.toml` exists without `--force`.
/// Uses atomic write via temporary file rename to prevent corrupt or partial initialization.
pub fn run_init(options: InitOptions<'_>) -> Result<(), CliError> {
    let target_dir: PathBuf = match options.target_path {
        Some(p) => p.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => return Err(CliError::Io(e)),
        },
    };

    if !target_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            return Err(CliError::Io(e));
        }
    }

    let config_path = target_dir.join(".xtrc.toml");
    if config_path.exists() && !options.force {
        return Err(CliError::ConfigAlreadyExists(config_path));
    }

    let docs_dir = target_dir.join("docs");
    if !docs_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&docs_dir) {
            return Err(CliError::Io(e));
        }
    }

    let content = WorkspaceConfig::default_starter_toml();
    let tmp_path = target_dir.join(generate_tmp_filename());
    if let Err(e) = std::fs::write(&tmp_path, content) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(CliError::Io(e));
    }

    if let Err(e) = std::fs::rename(&tmp_path, &config_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(CliError::Io(e));
    }

    if !options.quiet {
        println!("Initialized xtrc workspace at {}", target_dir.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_basic() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let opts = InitOptions {
            target_path: Some(temp.path()),
            force: false,
            quiet: true,
        };
        assert!(run_init(opts).is_ok());
        assert!(temp.path().join(".xtrc.toml").is_file());
        assert!(temp.path().join("docs").is_dir());
        Ok(())
    }

    #[test]
    fn test_init_already_exists_fails_without_force() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        std::fs::write(temp.path().join(".xtrc.toml"), "existing")?;

        let opts = InitOptions {
            target_path: Some(temp.path()),
            force: false,
            quiet: true,
        };
        let err = run_init(opts);
        match err {
            Err(CliError::ConfigAlreadyExists(_)) => (),
            other => panic!("Expected ConfigAlreadyExists, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_init_overwrites_with_force() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        std::fs::write(temp.path().join(".xtrc.toml"), "old content")?;

        let opts = InitOptions {
            target_path: Some(temp.path()),
            force: true,
            quiet: true,
        };
        assert!(run_init(opts).is_ok());
        let content = std::fs::read_to_string(temp.path().join(".xtrc.toml"))?;
        assert!(content.contains("[project]"));
        Ok(())
    }

    #[test]
    fn test_tmp_filename_uniqueness() {
        let f1 = generate_tmp_filename();
        let f2 = generate_tmp_filename();
        assert_ne!(f1, f2);
        assert!(f1.starts_with(".xtrc.toml.tmp."));
        assert!(f2.starts_with(".xtrc.toml.tmp."));
    }

    #[test]
    fn test_init_concurrent_in_separate_dirs() -> Result<(), Box<dyn std::error::Error>> {
        use std::thread;

        let handles: Vec<_> = (0..8)
            .map(|_| {
                thread::spawn(|| {
                    let temp = TempDir::new().map_err(|e| e.to_string())?;
                    let opts = InitOptions {
                        target_path: Some(temp.path()),
                        force: false,
                        quiet: true,
                    };
                    run_init(opts).map_err(|e| e.to_string())?;
                    if !temp.path().join(".xtrc.toml").is_file() {
                        return Err("Config file missing".to_string());
                    }
                    Ok(())
                })
            })
            .collect();

        for handle in handles {
            let res = handle.join().map_err(|e| format!("Join error: {e:?}"))?;
            res.map_err(|e| format!("Thread error: {e}"))?;
        }
        Ok(())
    }
}
