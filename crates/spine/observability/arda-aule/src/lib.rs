// sigil: ANKH
//! Core blueprint module for the Arda Council agent.
//!
//! This module defines the contract, council seats, and governance baseline
//! that all new sovereign agents should replicate. It provides the foundational
//! structure for implementing governance-consistent agentic crates in the
//! Arda system.
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
//! use arda_aule::contract::contract;
//! use arda_aule::contract::ArdaCouncilContract;
//! use arda_aule::{council::{CouncilQuery, CouncilSeat, QueryMode}, service};
//!
//! // Get the canonical contract
//! let c: ArdaCouncilContract = contract();
//! assert_eq!(c.crate_name, "arda-aule");
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
//! - [`ArdaCouncilContract`]: The canonical governance contract
//! - [`GovernanceBaseline`]: Required governance checks
//! - [`ContinuityBaseline`]: State preservation requirements
//! - [`CouncilSeat`]: Available council roles
//! - [`QueryMode`]: Council deliberation modes

pub mod contract;
pub mod council;
pub mod governance_metrics;
pub mod service;

pub use governance_metrics::render_governance_prometheus;

#[cfg(test)]
mod phase5_observability_tests {
    use super::render_governance_prometheus;
    use arda_governance::{
        GovernanceCounterSnapshot, GovernanceHistogramBucket, GovernanceHistogramSnapshot,
        GovernanceMetricsSnapshot,
    };
    use std::collections::BTreeMap;

    #[test]
    fn active_aule_surface_renders_governance_prometheus_snapshot() {
        let snapshot = GovernanceMetricsSnapshot {
            collection_mode: "library_owned_in_process_caller_exposed".to_string(),
            owns_http_server: false,
            bacon_lite_writer: None,
            counters: vec![GovernanceCounterSnapshot {
                name: "arda_governance_triad_validations_total".to_string(),
                labels: BTreeMap::from([
                    ("verdict".to_string(), "pass".to_string()),
                    ("policy_version".to_string(), "current".to_string()),
                    ("scorer_version".to_string(), "current".to_string()),
                    ("review_mode".to_string(), "heuristic_local".to_string()),
                ]),
                value: 1,
            }],
            histograms: vec![GovernanceHistogramSnapshot {
                name: "arda_governance_resonance".to_string(),
                count: 1,
                sum: 0.8,
                buckets: vec![GovernanceHistogramBucket {
                    upper_bound: 1.0,
                    cumulative_count: 1,
                }],
            }],
        };
        let text = render_governance_prometheus(&snapshot);
        assert!(text.contains("arda_governance_triad_validations_total"));
        assert!(text.contains("policy_version=\"current\""));
        assert!(text.contains("# TYPE arda_governance_resonance histogram"));
    }
}

/// Returns the identity string of this crate.
///
/// # Example
///
/// ```
/// use arda_aule::crate_identity;
///
/// assert_eq!(crate_identity(), "arda-aule");
/// ```
pub fn crate_identity() -> &'static str {
    "arda-aule"
}
