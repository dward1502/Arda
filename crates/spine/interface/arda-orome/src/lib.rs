#![recursion_limit = "512"]
//! sigil: REPAIR
//! arda-orome — Arda’s resident comms bridge.
//!
//! Merged surface: resident messaging + A2A/A2H protocol types.

#[cfg(feature = "service-runtime")]
pub mod agent;
pub mod comm;
pub mod commercial;
#[cfg(feature = "service-runtime")]
pub mod context_cache;
#[cfg(feature = "service-runtime")]
pub mod context_enrichment;
#[cfg(feature = "service-runtime")]
pub mod discord_health;
#[cfg(feature = "service-runtime")]
pub mod discord_safe_message;
pub mod governance;
pub mod grpc;
#[cfg(any(test, feature = "service-runtime"))]
pub mod intent;
#[cfg(feature = "service-runtime")]
pub mod mcp;
pub mod message;
#[cfg(test)]
pub mod message_retry_expiry;
#[cfg(feature = "service-runtime")]
pub mod mnemosyne_integration;
pub mod personal_reminder;
#[cfg(feature = "service-runtime")]
pub mod protocol;
pub mod provider;
#[cfg(any(test, feature = "service-runtime"))]
pub mod registry;
#[cfg(test)]
pub mod router;
#[cfg(feature = "service-runtime")]
pub mod service;
pub mod types;
pub use comm::{
    A2HMessage, Attachment, AuthPayload, Channel as A2HChannel, ClarifyPayload, CommError,
    CommGovernanceMetadata, HumanResponse, MessageQueue, NotifyPayload, Priority, ResponseAction,
    StatusPayload,
};
pub use governance::GovernanceHooks;
pub use message::{A2AMessage, A2AMessageType, Envelope};
pub use types::{
    BoardroomManweRouteEvidence, BoardroomOracleLink, BoardroomPost, BoardroomQuorumDecision,
    BoardroomQuorumPacket, BoardroomTriadScores, CommsEvent, CommsEventRisk, CommsEventType,
    CommsEventVisibility, CouncilCommandSeat, CouncilDiscussionNote, CouncilDiscussionProjection,
    CouncilDiscussionPromotion, IntentResult, InterruptionDisposition, InterruptionEnvelope,
    InterruptionLedgerDecision, InterruptionMessage, LocalCouncilSummaryFallbackMetadata,
    LocalCouncilSummaryRoute, ManweRouteHint, OperatingRoomEvent, OperatingRoomEventKind,
    PersonalReminderDeliveryState, PersonalReminderReceipt, PersonalReminderRequest,
    PromotionState, SubagentCompletionPacket, SubagentCompletionProjection, TaskApprovalEnvelope,
    TaskApprovalPacket, TaskApprovalProjection, TaskApprovalProposal,
};

#[cfg(all(test, feature = "service-runtime"))]
pub(crate) static HERMES_PROVIDER_SEND_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
