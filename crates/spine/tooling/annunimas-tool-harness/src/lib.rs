// sigil: ANKH
//! Blueprint surface for governed Annunimas tool invocation.
//!
//! `annunimas-tool-harness` defines the contract and validation primitives for
//! tool metadata, invocation envelopes, idempotency, and governance posture. It
//! is not yet the canonical mutating-tool runtime boundary.

pub mod contract;
pub mod service;
pub mod types;

pub fn crate_identity() -> &'static str {
    "annunimas-tool-harness"
}
