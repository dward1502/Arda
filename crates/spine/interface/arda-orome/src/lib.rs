#![recursion_limit = "512"]
//! sigil: REPAIR
//! arda-orome — Arda’s resident comms bridge.
//!
//! Merged surface: resident messaging + A2A/A2H protocol types.

pub mod comm;
pub mod message;
pub use comm::{
  Attachment, A2HMessage, AuthPayload, Channel as A2HChannel, ClarifyPayload, CommError,
  CommGovernanceMetadata, HumanResponse, InboundMessage, MessageQueue, NotifyPayload,
  OutboundMessage, Priority, ResponseAction, StatusPayload,
};
pub use message::{A2AMessage, A2AMessageType, Envelope};
