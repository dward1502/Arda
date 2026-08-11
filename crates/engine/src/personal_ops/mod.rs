//! Personal operations service: append-only event log and projection.
//!
//! Reuses arda-core's personal-ops contract types and projection builder,
//! backed by a JSONL event log store.

pub mod calendar;
pub mod proactive_cycle;
pub mod store;

pub use arda_core::personal_ops_projection::{build_projection, PersonalOpsProjection};
pub use proactive_cycle::{
    DeliveryPermit, DeliveryReceipt, PersistedEvaluation, ProactiveCycleError,
    ProactiveCycleProjection, ProactiveCycleStore, ProactiveEvaluation, ProactiveEvaluationStatus,
};
pub use store::{LoadError, PersonalOpsLogStore};
