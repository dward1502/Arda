//! Shared outpost protocol types: observations, authority envelope, schema constants.

pub mod authority;
pub mod error;
pub mod observation;
pub mod queue;

pub use authority::AuthorityClass;
pub use error::OutpostProtocolError;
pub use observation::{AgentFeedback, OutpostObservation, ObservationClassification, ObservationScope};
pub use queue::{OutpostQueue, OutpostQueueError, generate_queue, consume_queue};

pub const SCHEMA_VERSION: &str = "arda.outpost.observation.v1";
