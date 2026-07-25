#![cfg(feature = "full-cli")]
pub mod autopilot;
pub mod core_link;
pub mod council;
pub mod error;
pub mod heartbeat;
pub mod orders;
pub mod planner;
pub mod registry;
pub mod router;
pub mod service;
pub mod thought;
pub mod transport;

pub use error::PrometheusError;
pub use heartbeat::{select_heartbeat_mode, HeartbeatMode, HeartbeatState};
pub use orders::{EscalationEvent, EscalationStatus, OrderStatus, OrderStore};
pub use planner::{run as run_planner, PlanPass};
pub use registry::{AgentRosterSnapshot, AgentStatus};
pub use router::Router;
pub use service::{ContextEngineeringPolicy, PrometheusService, PrometheusStatus};
pub use thought::ThoughtLedger;
