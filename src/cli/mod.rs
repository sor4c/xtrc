//! Command-line interface definitions and routing for xtrc.
//!
//! // @trace SPEC-DISC-INIT, SPEC-CLI-VERIFY, SPEC-CLI-MATRIX, SPEC-CLI-TREE, SPEC-CLI-EXPORT
//! // @trace SPEC-CLI-OUTPUT, SPEC-ERR-HIERARCHY, REQ-CLI-INIT, REQ-STRICT-GATE, REQ-CMD-MATRIX
//! // @trace REQ-CMD-TREE, REQ-CMD-EXPORT, REQ-EXIT-CODES, REQ-UNIX-STREAMS

pub mod export;
pub mod init;
pub mod matrix;
pub mod output;
pub mod pipeline;
pub mod tree;
pub mod verify;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

use crate::error::CliError;

/// Fast, plain-text traceability CLI and 7-layer DAG engine.
#[derive(Parser, Debug)]
#[command(
    name = "xtrc",
    author,
    version,
    about = "Fast, plain-text traceability CLI and 7-layer DAG engine",
    long_about = "Fast, plain-text traceability CLI written in Rust linking vision, strategy, epics, and user stories directly to technical specs, code, and automated tests."
)]
pub struct Cli {
    /// Verbose output flag.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override workspace root directory.
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    /// Override path to configuration file.
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Emit universal JSON payload to stdout.
    #[arg(long, global = true)]
    pub json: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands for xtrc.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new xtrc workspace with default configuration and directory scaffold.
    Init {
        /// Target directory path to initialize (defaults to current directory).
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Overwrite existing configuration if already present.
        #[arg(short, long)]
        force: bool,

        /// Suppress confirmation output on stdout.
        #[arg(short, long)]
        quiet: bool,
    },

    /// Verify bidirectional traceability integrity and compliance across all 7 layers.
    #[command(alias = "verify")]
    Check {
        /// Elevate warnings and coverage gaps to fatal failures (exit code 1).
        #[arg(short, long)]
        strict: bool,

        /// Emit universal machine-parseable JSON report to stdout.
        #[arg(long)]
        json: bool,

        /// Scope verification to a specific layer boundary.
        #[arg(short, long)]
        layer: Option<String>,

        /// Scope verification to the connected subgraph of a specific node identifier.
        #[arg(short, long)]
        id: Option<String>,

        /// Suppress non-essential summary display on stdout.
        #[arg(short, long)]
        quiet: bool,
    },

    /// Generate multidimensional traceability matrix across lifecycle layers.
    Matrix {
        /// Output format (table, markdown, csv, json).
        #[arg(short, long)]
        format: Option<String>,

        /// Filter matrix to only display items lacking forward code or test coverage.
        #[arg(short, long)]
        uncovered: bool,

        /// Filter matrix rows to a specific layer boundary.
        #[arg(short, long)]
        layer: Option<String>,

        /// Emit machine-parseable JSON matrix (alias for --format json).
        #[arg(long)]
        json: bool,
    },

    /// Visualize hierarchical traceability relationships and ancestors/descendants.
    Tree {
        /// Identifier of the root node to anchor the tree visualization.
        #[arg(value_name = "ID")]
        id: Option<String>,

        /// Traverse upward toward Vision ancestors (justification path).
        #[arg(short = 'r', long, conflicts_with = "down")]
        reverse: bool,

        /// Traverse downward toward Implementation and Tests (decomposition path, default).
        #[arg(long)]
        down: bool,

        /// Maximum recursion depth (0 = root only).
        #[arg(short = 'd', long, value_parser = clap::value_parser!(usize))]
        depth: Option<usize>,

        /// Emit machine-parseable JSON representation of the tree hierarchy.
        #[arg(long)]
        json: bool,
    },

    /// Generate consolidated compliance audit export documentation.
    Export {
        /// Export format (markdown, json, csv).
        #[arg(short = 'f', long, default_value = "markdown")]
        format: String,

        /// Target output file path. If omitted, writes directly to stdout.
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
}

/// Runs the CLI application using arguments from `std::env::args_os`.
pub fn run_cli() -> Result<(), CliError> {
    run_cli_args(std::env::args_os())
}

/// Runs the CLI application using provided argument iterator.
pub fn run_cli_args<I, T>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(err) => {
            if err.use_stderr() {
                return Err(CliError::InvalidArguments {
                    reason: err.to_string(),
                });
            } else {
                print!("{err}");
                return Ok(());
            }
        }
    };

    match cli.command {
        Some(Commands::Init { path, force, quiet }) => init::run_init(init::InitOptions {
            target_path: path.as_deref(),
            force,
            quiet,
        }),
        Some(Commands::Check {
            strict,
            json,
            layer,
            id,
            quiet,
        }) => verify::run_verify(verify::VerifyOptions {
            root: cli.root.as_deref(),
            config_path: cli.config.as_deref(),
            strict,
            json: json || cli.json,
            layer,
            id,
            quiet,
        }),
        Some(Commands::Matrix {
            format,
            uncovered,
            layer,
            json,
        }) => matrix::run_matrix(matrix::MatrixOptions {
            root: cli.root.as_deref(),
            config_path: cli.config.as_deref(),
            format,
            uncovered,
            layer,
            json: json || cli.json,
        }),
        Some(Commands::Tree {
            id,
            reverse,
            down,
            depth,
            json,
        }) => tree::run_tree(tree::TreeOptions {
            root: cli.root.as_deref(),
            config_path: cli.config.as_deref(),
            id,
            reverse,
            down,
            depth,
            json: json || cli.json,
        }),
        Some(Commands::Export { format, output }) => export::run_export(export::ExportOptions {
            root: cli.root.as_deref(),
            config_path: cli.config.as_deref(),
            format,
            output,
        }),
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().map_err(CliError::Io)?;
            println!();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_init() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["xtrc", "init", "--force", "--quiet"];
        let cli = Cli::try_parse_from(args)?;
        match cli.command {
            Some(Commands::Init { force, quiet, path }) => {
                assert!(force);
                assert!(quiet);
                assert!(path.is_none());
            }
            _ => panic!("Expected Init subcommand"),
        }
        Ok(())
    }

    #[test]
    fn test_cli_parse_check() -> Result<(), Box<dyn std::error::Error>> {
        let args = [
            "xtrc",
            "check",
            "--strict",
            "--json",
            "--layer",
            "Requirement",
        ];
        let cli = Cli::try_parse_from(args)?;
        match cli.command {
            Some(Commands::Check {
                strict,
                json,
                layer,
                id,
                quiet,
            }) => {
                assert!(strict);
                assert!(json);
                assert_eq!(layer.as_deref(), Some("Requirement"));
                assert!(id.is_none());
                assert!(!quiet);
            }
            _ => panic!("Expected Check subcommand"),
        }
        Ok(())
    }

    #[test]
    fn test_cli_parse_matrix() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["xtrc", "matrix", "-f", "csv", "-u"];
        let cli = Cli::try_parse_from(args)?;
        match cli.command {
            Some(Commands::Matrix {
                format,
                uncovered,
                layer,
                json,
            }) => {
                assert_eq!(format.as_deref(), Some("csv"));
                assert!(uncovered);
                assert!(layer.is_none());
                assert!(!json);
            }
            _ => panic!("Expected Matrix subcommand"),
        }
        Ok(())
    }

    #[test]
    fn test_cli_parse_tree() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["xtrc", "tree", "REQ-01", "-r", "-d", "3"];
        let cli = Cli::try_parse_from(args)?;
        match cli.command {
            Some(Commands::Tree {
                id,
                reverse,
                down,
                depth,
                json,
            }) => {
                assert_eq!(id.as_deref(), Some("REQ-01"));
                assert!(reverse);
                assert!(!down);
                assert_eq!(depth, Some(3));
                assert!(!json);
            }
            _ => panic!("Expected Tree subcommand"),
        }
        Ok(())
    }

    #[test]
    fn test_cli_parse_export() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["xtrc", "export", "-f", "json", "-o", "audit.json"];
        let cli = Cli::try_parse_from(args)?;
        match cli.command {
            Some(Commands::Export { format, output }) => {
                assert_eq!(format, "json");
                assert_eq!(output, Some(PathBuf::from("audit.json")));
            }
            _ => panic!("Expected Export subcommand"),
        }
        Ok(())
    }

    #[test]
    fn test_cli_no_subcommand_prints_help_and_exits_zero() -> Result<(), Box<dyn std::error::Error>>
    {
        let args = ["xtrc"];
        let res = run_cli_args(args);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_cli_invalid_arg_returns_invalid_arguments_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let args = ["xtrc", "--invalid-flag-12345"];
        match run_cli_args(args) {
            Err(CliError::InvalidArguments { reason }) => {
                assert!(reason.contains("unexpected argument"));
            }
            Ok(()) => panic!("Expected InvalidArguments error, got Ok"),
            Err(other) => panic!("Expected InvalidArguments, got {other:?}"),
        }
        Ok(())
    }
}
