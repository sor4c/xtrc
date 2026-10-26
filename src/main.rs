//! xtrc CLI binary entry point.
//!
//! // @trace SPEC-CLI-OUTPUT, SPEC-ERR-HIERARCHY, SPEC-VER-BUMP, REQ-UNIX-STREAMS, REQ-EXIT-CODES, REQ-NFR-ROBUST, REQ-VER-BUMP-PATCH

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match xtrc::cli::run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            match &err {
                xtrc::error::CliError::InvalidArguments { reason } => {
                    if reason.starts_with("error:") || reason.starts_with("Error:") {
                        if reason.ends_with('\n') {
                            eprint!("{reason}");
                        } else {
                            eprintln!("{reason}");
                        }
                    } else if reason.ends_with('\n') {
                        eprint!("Error: {reason}");
                    } else {
                        eprintln!("Error: {reason}");
                    }
                }
                _ => {
                    eprintln!("Error: {err}");
                }
            }
            let code = err.exit_code();
            ExitCode::from(code as u8)
        }
    }
}
