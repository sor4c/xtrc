//! Subcommand handler for `xtrc check` (and `xtrc verify` alias).
//!
//! // @trace SPEC-CLI-VERIFY, REQ-STRICT-GATE, REQ-EXIT-CODES, REQ-UNIX-STREAMS

use std::path::Path;
use std::str::FromStr;

use crate::cli::output::StreamEnvironment;
use crate::cli::pipeline::load_workspace_graph;
use crate::error::CliError;
use crate::graph::validator::{validate_graph, ValidationOptions};
use crate::model::{Layer, NodeId};

/// Options configuring `xtrc check` / `xtrc verify` execution.
#[derive(Debug, Clone, Default)]
pub struct VerifyOptions<'a> {
    /// Workspace root directory override.
    pub root: Option<&'a Path>,
    /// Configuration file override.
    pub config_path: Option<&'a Path>,
    /// Elevate warnings and coverage gaps to fatal failures (exit code 1).
    pub strict: bool,
    /// Emit universal machine-parseable JSON report to stdout.
    pub json: bool,
    /// Scope verification to a specific layer boundary.
    pub layer: Option<String>,
    /// Scope verification to the connected subgraph of a specific node identifier.
    pub id: Option<String>,
    /// Suppress non-essential summary display on stdout.
    pub quiet: bool,
}

/// Executes the bidirectional verification pipeline.
pub fn run_verify(options: VerifyOptions<'_>) -> Result<(), CliError> {
    // Probe stream environment and disable ANSI color codes when piped
    let _env = StreamEnvironment::probe();

    let (_root, config, graph) = load_workspace_graph(options.root, options.config_path)?;

    // 1. Parse layer filter if requested
    let layer_filter = match options.layer {
        Some(ref s) => {
            let parsed = Layer::from_str(s).map_err(|e| CliError::InvalidArguments {
                reason: format!("Invalid layer '{s}': {e}"),
            })?;
            Some(parsed)
        }
        None => None,
    };

    // 2. Parse and validate ID filter if requested
    let id_filter = match options.id {
        Some(ref s) => {
            if s.trim().is_empty() {
                return Err(CliError::InvalidArguments {
                    reason: "The --id option cannot be empty".to_string(),
                });
            }
            let node_id = NodeId::new(s).map_err(|e| CliError::InvalidArguments {
                reason: format!("Invalid node ID '{s}': {e}"),
            })?;
            if !graph.has_node(&node_id) {
                return Err(CliError::NodeNotFound {
                    id: node_id.to_string(),
                });
            }
            Some(node_id)
        }
        None => None,
    };

    // 3. Configure validation options
    let is_strict = options.strict || config.validation.strict;
    let validation_options = ValidationOptions {
        strict: is_strict,
        layer_filter,
        id_filter,
    };

    // 4. Execute 4-stage validation
    let report = validate_graph(&graph, &validation_options);

    // 5. Detect and report circular dependencies to stderr
    let cycles = graph.find_all_cycles();
    for cycle in &cycles {
        eprintln!(
            "[ERROR] Cyclic dependency detected: {}",
            cycle.format_with_provenance(&graph)
        );
    }

    // 6. Stream diagnostic warnings and errors to stderr
    let diagnostics = report.format_diagnostics_stderr(is_strict);
    if !diagnostics.is_empty() {
        eprint!("{diagnostics}");
    }

    // 7. Stream structured report exclusively to stdout
    if options.json {
        let json_str = report
            .to_json()
            .map_err(|e| CliError::Io(std::io::Error::other(e)))?;
        println!("{json_str}");
    } else if !options.quiet {
        println!("{}", report.format_terminal_summary());
    }

    // 8. Determine exit code contract
    if !report.is_success(is_strict) || !cycles.is_empty() {
        let fatal_count = report
            .violations
            .iter()
            .filter(|v| v.is_fatal(is_strict))
            .count()
            + cycles.len();
        return Err(CliError::VerificationFailure {
            violations: fatal_count,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_options_default() {
        let opts = VerifyOptions::default();
        assert!(!opts.strict);
        assert!(!opts.json);
        assert!(!opts.quiet);
        assert!(opts.layer.is_none());
        assert!(opts.id.is_none());
    }
}
