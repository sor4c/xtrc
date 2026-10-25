//! xtrc CLI binary entry point.
//!
//! // @trace SPEC-CLI-OUTPUT, SPEC-ERR-HIERARCHY, REQ-UNIX-STREAMS, REQ-EXIT-CODES, REQ-NFR-ROBUST

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match xtrc::cli::run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            match &err {
                xtrc::error::CliError::InvalidArguments { reason } => {
                    if reason.ends_with('\n') {
                        eprint!("{reason}");
                    } else {
                        eprintln!("{reason}");
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
