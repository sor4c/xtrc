//! In-memory 7-layer Directed Acyclic Graph (DAG) and validation engine.
//!
//! // @trace SPEC-DAG-STORAGE, SPEC-DAG-CYCLE, SPEC-DAG-VALIDATE, REQ-DAG-MODEL

pub mod cycle;
pub mod model;
pub mod validator;

pub use cycle::{detect_cycles, find_all_cycles, Cycle, CycleDetector, NodeColor};
pub use model::TraceabilityGraph;
pub use validator::{
    validate_graph, BrokenLinkKind, EpicCoverage, ValidationOptions, ValidationReport,
    ValidationStats, ValidationViolation, ViolationSeverity,
};
