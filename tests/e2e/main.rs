//! End-to-end acceptance suite for the `xtrc` CLI, compiled as a single test binary.
//!
//! - `tier1_features`: each feature exercised in isolation.
//! - `tier2_boundaries`: limits, malformed input, cycles, broken links and exit codes.
//! - `tier3_combinations`: pairwise and composite CLI workflows.
//! - `tier4_real_world`: full repository lifecycles and CI gate scenarios.
//!
//! // @trace:test REQ-EXIT-CODES, REQ-UNIX-STREAMS

mod common;
mod tier1_features;
mod tier2_boundaries;
mod tier3_combinations;
mod tier4_real_world;
