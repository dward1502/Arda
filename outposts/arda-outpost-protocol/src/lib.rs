//! Shared outpost protocol types: observations, authority envelope, schema constants.

pub mod authority;
pub mod error;
pub mod observation;
pub mod presence;
pub mod queue;
pub mod research;
pub mod research_beta;
pub mod watchlist;

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
pub use research::{
    validate_research_chain, AcknowledgementReceipt, DispatchDisposition,
    ExternalObservationReceipt, PersistedResearchChain, ResearchCursor, ResearchDispatch,
    ResearchReceiptError, ResearchReceiptLedger, ResearchSuggestion, ResearchSuggestionLedger,
    ADVISORY_RESEARCH_AUTHORITY, RESEARCH_SCHEMA_VERSION,
};
pub use research_beta::{
    disabled_watchlist_templates, inspect_untrusted_content, ContentInspection, ResearchBetaPolicy,
    WatchlistTemplate, WatchlistTemplateCategory, PROPOSAL_ONLY_AUTHORITY,
    RESEARCH_BETA_POLICY_SCHEMA,
};
pub use watchlist::{
    ContradictionPolicy, ResearchQuestion, ResearchWatchlist, WatchlistBudgets, WatchlistCadence,
    WatchlistError, WatchlistEvidenceRequirements, WatchlistNotificationPolicy,
    WatchlistSourcePolicy, WatchlistState, WATCHLIST_SCHEMA_VERSION,
};

pub const SCHEMA_VERSION: &str = "arda.outpost.observation.v1";
