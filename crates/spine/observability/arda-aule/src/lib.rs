#![recursion_limit = "1024"]
// sigil: ANKH
//! Observability home for the Arda runtime.
//!
//! This crate owns the observability surfaces for executors, governance
//! metrics, and operator-facing telemetry. It provides the contract,
//! readiness probe, and module export points for:
//!
//! - `prometheus/` — executive orchestrator pipeline and confidence scoring
//! - `ceo/` — orchestration brain for delegation and learning
//! - `cli/` — Arda CLI entrypoints for observability-aware operators
//! - `export_surface/` — interoperability contracts for agents and HUD
//!
//! # Example
//!
//! ```rust
//! use arda_aule::contract::contract;
//! use arda_aule::service::{ArdaAuleStatus, ObservabilityBrief};
//!
//! // Retrieves the observability contract for the crate family.
//! let observability_contract = contract();
//! assert_eq!(observability_contract.crate_name, "arda-aule");
//! assert_eq!(observability_contract.realm, "observability");
//!
//! // Computes readiness from the contract.
//! let status = ArdaAuleStatus::from_contract(&observability_contract);
//! assert!(status.governance_ready);
//!
//! // Builds an observability brief for reporting or export.
//! let brief = ObservabilityBrief::from_status(&status);
//! assert_eq!(brief.crate_name, "arda-aule");
//! ```
//!
//! # Governance Baselines
//!
//! Observability surfaces continue to honor Arda governance baselines:
//!
//! - **Triad Validation:** Three-way verification surfaced in telemetry
//! - **Bacon Lite:** Lightweight compliance emission for exporters
//! - **JouleWork:** Energy/work accounting in prometheus metrics
//! - **Love Equation:** Alignment telemetry fields
//! - **Soterion Trace:** Audit trail and traceability exports
//!
//! # Modules
//!
//! - [`contract`]: observability contract for Arda
//! - [`service`]: readiness probe and brief builders
//! - [`council`]: compatibility shim for existing council query types
//! - [`prometheus`]: executive orchestrator surface
//! - [`ceo`]: orchestration brain
//! - [`cli`]: operator CLI surface
//!
//! # See Also
//!
//! - [`ArdaAuleContract`]: The canonical observability contract
//! - [`ArdaAuleStatus`]: Readiness probe output
//! - [`ObservabilityBrief`]: Compact observability summary

pub mod contract;
pub mod council;
pub mod governance_metrics;
pub mod service;

#[cfg(feature = "telemetry")]
pub mod telemetry;

pub mod prometheus;
pub mod ceo;
pub mod cli;

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
