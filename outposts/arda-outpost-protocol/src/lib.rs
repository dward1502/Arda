//! Shared outpost protocol types: observations, authority envelope, schema constants.

pub mod authority;
pub mod error;
pub mod observation;
pub mod queue;

pub use authority::AuthorityClass;
pub use error::OutpostProtocolError;
pub use observation::{
    AgentFeedback, ObservationClassification, ObservationScope, OutpostObservation,
};
pub use queue::{consume_queue, generate_queue, OutpostQueue, OutpostQueueError};

pub const SCHEMA_VERSION: &str = "arda.outpost.observation.v1";
