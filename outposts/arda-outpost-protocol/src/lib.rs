//! Shared outpost protocol types: observations, authority envelope, schema constants.

pub mod authority;
pub mod error;
pub mod observation;

pub use authority::AuthorityClass;
pub use error::OutpostProtocolError;
pub use observation::{OutpostObservation, ObservationClassification, ObservationScope};

pub const SCHEMA_VERSION: &str = "arda.outpost.observation.v1";
