//! In-memory 7-layer Directed Acyclic Graph (DAG) and validation engine.
//!
//! // @trace SPEC-DAG-STORAGE, SPEC-DAG-CYCLE, SPEC-DAG-VALIDATE, REQ-DAG-MODEL

pub mod cycle;
pub mod model;
pub mod validator;

#[cfg(test)]
#[path = "internal_tests/mod.rs"]
mod integration_tests;
