//! Integration test suites for the 7-layer graph engine, cycle detector, and validator.
//!
//! // @trace:test SPEC-DAG-STORAGE, SPEC-DAG-CYCLE, SPEC-DAG-VALIDATE, REQ-DAG-MODEL
//! // @trace:test REQ-CYCLE-DETECT, REQ-BROKEN-LINK, REQ-FORWARD-COV, REQ-BACKWARD-JUST
//! // @trace:test REQ-ORPHAN-DETECT, REQ-STRICT-GATE

pub mod test_adversarial;
pub mod test_cycle;
pub mod test_model;
pub mod test_validator;
