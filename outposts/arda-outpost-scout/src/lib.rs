pub mod error;
pub mod observation;
pub mod suggestion;
pub mod survey;

pub use arda_outpost_protocol::{AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation};
pub use error::{ScoutError, Result};
pub use observation::{CrateObservation, CrateStatus, SurveyReport};
pub use survey::survey_repo;
