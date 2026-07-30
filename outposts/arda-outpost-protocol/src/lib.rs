//! Shared outpost protocol types: observations, authority envelope, schema constants.

pub mod authority;
pub mod error;
pub mod observation;
pub mod presence;
pub mod queue;

pub use authority::AuthorityClass;
pub use error::OutpostProtocolError;
pub use observation::{
    AgentFeedback, ObservationClassification, ObservationScope, OutpostObservation,
};
pub use presence::{
    DegradedReason, HealthState, LifecycleState, PresenceEdge, PresenceEdgeType, PresenceNode,
    PresenceNodeKind, RedactionClass, ResourcePressure, RuntimePresenceProjection,
    SceneDisposition, SceneState, RUNTIME_PRESENCE_SCHEMA_VERSION,
};
pub use queue::{consume_queue, generate_queue, OutpostQueue, OutpostQueueError};

pub const SCHEMA_VERSION: &str = "arda.outpost.observation.v1";
