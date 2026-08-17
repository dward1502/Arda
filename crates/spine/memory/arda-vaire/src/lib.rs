pub mod error;
pub mod persona;
pub mod retrieval_eval;
pub mod schema;
pub mod service;
pub mod significance;
pub mod transport;

pub use schema::{
    CONTINUITY_SCHEMA_VERSION, EPISODIC_SCHEMA_VERSION, LEGACY_EPISODIC_SCHEMA_VERSION,
    PERSONA_SCHEMA_ID, PERSONA_SCHEMA_VERSION,
};
pub use service::{
    continuity::{
        ContinuityPrivacyClass, ContinuityProvenance, ContinuityRecord, SurfaceHistoryEntry,
        VAIRE_CONTINUITY_SCHEMA_VERSION,
    },
    governed::{
        ApprovedKnowledgeDelta, GovernedKnowledgeReceipt, GOVERNED_KNOWLEDGE_SCHEMA_VERSION,
    },
    InformantEvent, MnemosyneService,
};
