// sigil: ANKH
//! Core blueprint module for the Annunimas Council agent.
//!
//! This module defines the contract, council seats, and governance baseline
//! that all new sovereign agents should replicate. It provides the foundational
//! structure for implementing governance-consistent agentic crates in the
//! Annunimas system.
//!
//! # Governance Baselines
//!
//! Every sovereign agent must implement these governance requirements:
//!
//! - **Triad Validation:** Three-way verification of decisions
//! - **Bacon Lite:** Lightweight compliance framework
//! - **JouleWork:** Energy/work accounting
//! - **Love Equation:** Alignment verification
//! - **Soterion Trace:** Audit trail and traceability
//!
//! # Continuity Baselines
//!
//! State preservation requirements for agent reliability:
//!
//! - **Task Ledger:** Historical task tracking
//! - **Memory Checkpoint:** State snapshots
//! - **ARDA Visibility:** HUD integration
//!
//! # Example
//! # Example
//! ```rust
//! use annunimas_council::contract::contract;
//! use annunimas_council::contract::AnnunimasCouncilContract;
//! use annunimas_council::{council::{CouncilQuery, CouncilSeat, QueryMode}, service};
//!
//! // Get the canonical contract
//! let c: AnnunimasCouncilContract = contract();
//! assert_eq!(c.crate_name, "annunimas-council");
//! assert_eq!(c.realm, "command");
//!
//! // Check governance readiness
//! let status = service::status();
//! assert!(status.governance_ready);
//!
//! // Build a council brief for a query
//! let query = CouncilQuery {
//!     mode: QueryMode::FullCouncil,
//!     seats: vec![],
//!     prompt: "Should we ship this feature?".into(),
//! };
//! let brief = service::build_brief(&query);
//! assert_eq!(brief.participating_seats.len(), 7);
//! assert!(brief.escalation_required);
//! ```
//!
//! # Blueprint Usage
//!
//! This crate is designed as a blueprint for new agentic crates. When creating
//! a new sovereign agent, replicate this structure:
//!
//! 1. Define your crate's contract in `src/contract.rs`
//! 2. Implement service status in `src/service.rs`
//! 3. Add governance smoke tests in `tests/contract_smoke.rs`
//! 4. Export required state to `core/state/<crate-name>.json`
//!
//! # Modules
//!
//! - [`contract`]: Defines the governance and continuity baselines
//! - [`council`]: Council seat definitions and query modes
//! - [`service`]: Service status and brief building utilities
//!
//! # Dependencies
//!
//! - `serde` + `serde_json`: Serialization
//! - `chrono`: Date/time handling
//!
//! # See Also
//!
//! - [`AnnunimasCouncilContract`]: The canonical governance contract
//! - [`GovernanceBaseline`]: Required governance checks
//! - [`ContinuityBaseline`]: State preservation requirements
//! - [`CouncilSeat`]: Available council roles
//! - [`QueryMode`]: Council deliberation modes

pub mod contract;
pub mod council;
pub mod service;

/// Returns the identity string of this crate.
///
/// # Example
///
/// ```
/// use annunimas_council::crate_identity;
///
/// assert_eq!(crate_identity(), "annunimas-council");
/// ```
pub fn crate_identity() -> &'static str {
    "annunimas-council"
}
