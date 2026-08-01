mod executor;
mod recovery;
mod store;

pub use executor::{apply_transition_once, TransitionOutcome};
pub use recovery::RecoveredRun;
pub use store::{AppendOutcome, RunEvent, RunEventDraft, RunEventKind, RunStore, RunStoreError};
