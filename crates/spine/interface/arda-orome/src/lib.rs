#![recursion_limit = "512"]
//! sigil: REPAIR
//! arda-orome — Arda’s resident comms bridge.
//!
//! Merged surface: resident messaging + A2A/A2H protocol types.

pub mod comm;
pub mod governance;
pub mod grpc;
#[cfg(test)]
pub mod intent;
pub mod message;
#[cfg(test)]
pub mod message_retry_expiry;
pub mod provider;
#[cfg(test)]
pub mod registry;
#[cfg(test)]
pub mod router;
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
    PromotionState, SubagentCompletionPacket, SubagentCompletionProjection, TaskApprovalEnvelope,
    TaskApprovalPacket, TaskApprovalProjection, TaskApprovalProposal,
};
