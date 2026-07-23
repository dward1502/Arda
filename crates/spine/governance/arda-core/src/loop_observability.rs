//! Loop observability helpers for GEN3.
//!
//! These types are additive metadata only: they do not change dispatch
//! semantics or append-only ledger output. They expose loop-economy
//! summary data, decision-latency probes, and bounded env/config knobs
//! so external observability tooling can consume `arda-core` state
//! without requiring behavior changes in the dispatcher.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{env, time::Instant};

pub use crate::loop_economy::{build_snapshot, write_snapshot, BidSpread, LoopEconomySnapshot};
pub use crate::loop_economy::snapshot_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLatencyProbe {
    pub probe_kind: DecisionLatencyKind,
    pub started_at_utc: String,
    pub finished_at_utc: String,
    pub elapsed_nanos: u128,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DecisionLatencyKind {
    #[default]
    LoopTick,
    LedgerAppend,
    EconomySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopObservabilityConfig {
    #[serde(default)]
    pub economy_snapshot_enabled: bool,
    pub latency_probe_enabled: bool,
    pub max_latency_samples: usize,
}

impl Default for LoopObservabilityConfig {
    fn default() -> Self {
        Self {
            economy_snapshot_enabled: false,
            latency_probe_enabled: false,
            max_latency_samples: 64,
        }
    }
}

impl LoopObservabilityConfig {
    pub fn from_env() -> Self {
        Self {
            economy_snapshot_enabled: env::var("ARDA_LOOP_ECONOMY_SNAPSHOTS")
                .ok()
                .map(|v| !v.eq_ignore_ascii_case("0"))
                .unwrap_or_default(),
            latency_probe_enabled: env::var("ARDA_LOOP_LATENCY_PROBES")
                .ok()
                .map(|v| !v.eq_ignore_ascii_case("0"))
                .unwrap_or_default(),
            max_latency_samples: env::var("ARDA_LOOP_MAX_LATENCY_SAMPLES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(64),
        }
    }
}

#[derive(Debug)]
pub struct LatencyProbe {
    kind: DecisionLatencyKind,
    started_at: Instant,
    samples: Vec<DecisionLatencyProbe>,
    max_samples: usize,
}

impl LatencyProbe {
    pub fn new(max_samples: usize) -> Self {
        Self {
            kind: DecisionLatencyKind::default(),
            started_at: Instant::now(),
            samples: Vec::new(),
            max_samples,
        }
    }

    pub fn with_kind(&mut self, kind: DecisionLatencyKind) -> &mut Self {
        self.kind = kind;
        self
    }

    pub fn sample(&mut self) -> Option<DecisionLatencyProbe> {
        let elapsed = self.started_at.elapsed();
        let probe = DecisionLatencyProbe {
            probe_kind: self.kind,
            started_at_utc: Utc::now().to_rfc3339(),
            finished_at_utc: Utc::now().to_rfc3339(),
            elapsed_nanos: elapsed.as_nanos(),
        };
        self.samples.push(probe);
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
        self.samples.last().cloned()
    }

    pub fn samples(&self) -> &[DecisionLatencyProbe] {
        &self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateRoot;
    use crate::contract::{Decision, DecisionClass, PhilosopherVerdict, TriadOutcome, TriadVerdict};

    #[test]
    fn observability_config_defaults_are_conservative() {
        let config = LoopObservabilityConfig::default();
        assert!(!config.economy_snapshot_enabled);
        assert!(!config.latency_probe_enabled);
        assert_eq!(config.max_latency_samples, 64);
    }

    #[test]
    fn env_knobs_toggle_observability_features() {
        unsafe {
            std::env::set_var("ARDA_LOOP_ECONOMY_SNAPSHOTS", "1");
            std::env::set_var("ARDA_LOOP_LATENCY_PROBES", "0");
            std::env::set_var("ARDA_LOOP_MAX_LATENCY_SAMPLES", "8");
        }
        let config = LoopObservabilityConfig::from_env();
        assert!(config.economy_snapshot_enabled);
        assert!(!config.latency_probe_enabled);
        assert_eq!(config.max_latency_samples, 8);
        unsafe {
            std::env::remove_var("ARDA_LOOP_ECONOMY_SNAPSHOTS");
            std::env::remove_var("ARDA_LOOP_LATENCY_PROBES");
            std::env::remove_var("ARDA_LOOP_MAX_LATENCY_SAMPLES");
        }
    }

    #[test]
    fn economy_snapshot_round_trip_preserves_snapshot_shape() {
        let dir = tempfile::tempdir().unwrap();
        let state = StateRoot::new(dir.path().join("core/state"));

        let mut decision = Decision::new(
            "econ_1",
            DecisionClass::Dispatch,
            "task_1",
            "cheap",
            "economy probe",
            TriadOutcome {
                verdict: TriadVerdict::Pass,
                aurelius: PhilosopherVerdict {
                    verdict: TriadVerdict::Pass,
                    reason: None,
                },
                bacon: PhilosopherVerdict {
                    verdict: TriadVerdict::Pass,
                    reason: None,
                },
                sun_tzu: PhilosopherVerdict {
                    verdict: TriadVerdict::Pass,
                    reason: None,
                },
            },
        );
        decision.joule_estimate = 3.5;
        crate::ledger::Ledger::new(state.root().join("ledger"))
            .unwrap()
            .append(&decision)
            .unwrap();

        let snapshot = write_snapshot(&state).unwrap();
        assert_eq!(snapshot.decisions_today, 1);
        assert!((snapshot.total_joules_today - 3.5).abs() < 1e-9);
        assert!(snapshot_path(&state).exists());
    }

    #[test]
    fn latency_probe_records_bounded_samples() {
        let mut probe = LatencyProbe::new(2);
        probe.with_kind(DecisionLatencyKind::LoopTick)
            .sample()
            .unwrap();
        probe.sample();
        assert_eq!(probe.samples().len(), 2);
    }
}
