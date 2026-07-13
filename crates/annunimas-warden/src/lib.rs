// sigil: REPAIR
//! Annunimas Warden Module
//!
//! Runtime monitoring and security for containers.

pub mod alerts;
pub mod crypto;
pub mod error;
pub mod foreign;
pub mod monitor;
pub mod podman;
pub mod scoring;

pub use error::{GateError, GateResult};
pub use monitor::{audit_container, evaluate_execution_harness, ExecutionHarnessPolicy};
pub use scoring::{AutonomyReadinessScorer, DefaultGateScorer, GateScorer, OperationalRiskScorer};
