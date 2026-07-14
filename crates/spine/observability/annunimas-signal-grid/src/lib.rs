// sigil: ANKH
//! Blueprint surface for Annunimas communication signal routing.
//!
//! `annunimas-signal-grid` does not own live Hermes transport. It defines the
//! stable contract, governance checks, and example routing decisions needed to
//! evaluate whether a future signal grid should remain a projection surface or
//! graduate into an active routing crate.

pub mod contract;
pub mod pipeline;
pub mod service;

pub fn crate_identity() -> &'static str {
    "annunimas-signal-grid"
}
