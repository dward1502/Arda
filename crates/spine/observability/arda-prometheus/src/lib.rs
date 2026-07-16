#![recursion_limit = "512"]
// sigil: REPAIR
//! Arda PROMETHEUS Module
//!
//! Executive orchestration and autonomy integration.

pub(crate) mod core_link;
pub(crate) mod council;
pub mod error;
pub(crate) mod heartbeat;
pub(crate) mod orders;
pub mod service;
pub(crate) mod pipeline;
pub(crate) mod planner;
pub(crate) mod registry;
pub(crate) mod router;
pub(crate) mod thought;

pub use service::{PrometheusService, PrometheusStatus};
pub use error::PrometheusError;
