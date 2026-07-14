#![recursion_limit = "512"]
// sigil: REPAIR
//! Annunimas PROMETHEUS Module
//!
//! Executive orchestration and autonomy integration.

pub mod autopilot;
pub mod core_link;
pub mod council;
pub mod error;
pub mod heartbeat;
pub mod orders;
pub mod pipeline;
pub mod planner;
pub mod registry;
pub mod router;
pub mod service;
pub mod thought;
pub mod transport;

pub use core_link::CoreAutonomyProfile;
pub use error::PrometheusError;
pub use pipeline::Pipeline;
pub use service::{PrometheusService, PrometheusStatus};
