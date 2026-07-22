#![recursion_limit = "512"]
//! sigil: REPAIR
//! arda-orome — Arda’s resident comms bridge.
//!
//! Merged surface: resident messaging + A2A/A2H protocol types.

#[cfg(test)]
pub mod intent;
#[cfg(test)]
pub mod message_retry_expiry;
#[cfg(test)]
pub mod router;
#[cfg(test)]
pub mod registry;
pub mod comm;
pub mod grpc;
pub mod message;
pub mod provider;
pub mod types;
pub use comm::{
    A2HMessage, Attachment, AuthPayload, Channel as A2HChannel, ClarifyPayload, CommError,
    CommGovernanceMetadata, HumanResponse, InboundMessage, MessageQueue, NotifyPayload,
    OutboundMessage, Priority, ResponseAction, StatusPayload,
};
pub use message::{A2AMessage, A2AMessageType, Envelope};