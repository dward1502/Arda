//! Shared outpost protocol types: observations, authority envelope, schema constants.

pub mod access;
pub mod authority;
pub mod error;
pub mod mirromere;
pub mod observation;
pub mod presence;
pub mod queue;
pub mod research;
pub mod research_beta;
pub mod watchlist;

pub use access::{
    NetworkPosture, OutpostAccessContract, OutpostEnrollment, OUTPOST_ACCESS_SCHEMA_VERSION,
};
pub use authority::AuthorityClass;
pub use error::OutpostProtocolError;
pub use mirromere::{
    MirromereAccessibility, MirromereAvailability, MirromereDisplayRole,
    MirromereEvidenceReference, MirromereFreshness, MirromereInteractionId, MirromerePresencePhase,
    MirromerePrivacyClass, MirromerePrivacyPolicy, MirromereReducedMotion, MirromereScene,
    MirromereSceneId, MirromereSlot, MirromereSlotContent, MirromereSourceMode,
    MirromereSurfaceProjection, MirromereSurfaceValidationError, MirromereTransitionPolicy,
    MirromereTransitionStyle, MirromereUrgency, MirromereVectorFieldKind,
    MirromereVisibilityCeiling, MIRROMERE_MAX_ACCESSIBILITY_BYTES, MIRROMERE_MAX_ATTENTION_BUDGET,
    MIRROMERE_MAX_PURPOSE_BYTES, MIRROMERE_MAX_SLOTS, MIRROMERE_MAX_TEXT_BYTES,
    MIRROMERE_MAX_TRANSITION_MS, MIRROMERE_MAX_VECTOR_SAMPLES, MIRROMERE_SURFACE_SCHEMA_VERSION,
};
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
    ContradictionPolicy, ResearchQuestion, ResearchQuestionSpec, ResearchWatchlist,
    WatchlistBudgets, WatchlistCadence, WatchlistError, WatchlistEvidenceRequirements,
    WatchlistNotificationPolicy, WatchlistSourcePolicy, WatchlistState, WATCHLIST_SCHEMA_VERSION,
};

pub const SCHEMA_VERSION: &str = "arda.outpost.observation.v1";
