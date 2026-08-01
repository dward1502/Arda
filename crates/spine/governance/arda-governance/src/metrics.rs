//! In-process governance instrumentation.
//!
//! Metric collection is library-owned so every scorer records consistently, but
//! transport is caller-owned: this crate exposes snapshots and never starts an
//! HTTP server. `arda-aule` is responsible for Prometheus exposition.

use crate::{
    BaconLiteEvent, BaconLiteWriterCounters, EnvironmentalAdvisory, EnvironmentalCoherence,
    GateOutcome, GovernanceReviewMode, JouleWorkProfile, LoveDynamicsScore, LoveEquationScore,
    MeasurementQuality, ResonanceScore, SignalFreshness, SignalHealth, TriadResult,
    BACON_LITE_POLICY_VERSION, ENVIRONMENTAL_POLICY_VERSION, GOVERNANCE_CHAIN_POLICY_VERSION,
    JOULEWORK_POLICY_VERSION, LOVE_EQUATION_POLICY_VERSION, RESONANCE_POLICY_VERSION,
    TRIAD_POLICY_VERSION,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

const HISTOGRAM_BOUNDS: [f64; 6] = [0.10, 0.25, 0.50, 0.75, 0.90, 1.0];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CounterKey {
    name: String,
    labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct HistogramState {
    count: u64,
    sum: f64,
    buckets: [u64; HISTOGRAM_BOUNDS.len()],
}

#[derive(Debug, Default)]
struct MetricsState {
    counters: BTreeMap<CounterKey, u64>,
    histograms: BTreeMap<String, HistogramState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GovernanceCounterSnapshot {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GovernanceHistogramBucket {
    pub upper_bound: f64,
    pub cumulative_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GovernanceHistogramSnapshot {
    pub name: String,
    pub count: u64,
    pub sum: f64,
    pub buckets: Vec<GovernanceHistogramBucket>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GovernanceMetricsSnapshot {
    pub collection_mode: String,
    pub owns_http_server: bool,
    pub bacon_lite_writer: Option<BaconLiteWriterCounters>,
    pub counters: Vec<GovernanceCounterSnapshot>,
    pub histograms: Vec<GovernanceHistogramSnapshot>,
}

impl GovernanceMetricsSnapshot {
    pub fn counter_value(&self, name: &str, labels: &BTreeMap<String, String>) -> u64 {
        self.counters
            .iter()
            .find(|counter| counter.name == name && &counter.labels == labels)
            .map(|counter| counter.value)
            .unwrap_or(0)
    }

    pub fn histogram(&self, name: &str) -> Option<&GovernanceHistogramSnapshot> {
        self.histograms
            .iter()
            .find(|histogram| histogram.name == name)
    }

    pub fn label_values(&self, label: &str) -> BTreeSet<String> {
        self.counters
            .iter()
            .filter_map(|counter| counter.labels.get(label).cloned())
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct GovernanceMetrics {
    state: Mutex<MetricsState>,
}

impl GovernanceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_triad(&self, result: &TriadResult) {
        let version = version_class(&result.policy_version, TRIAD_POLICY_VERSION);
        let scorer_version = version_class(&result.chain_version, GOVERNANCE_CHAIN_POLICY_VERSION);
        let review_mode = review_mode_label(result.review_mode);
        self.increment(
            "arda_governance_triad_validations_total",
            labels(&[
                ("verdict", verdict_label(result.passed)),
                ("policy_version", version),
                ("scorer_version", scorer_version),
                ("review_mode", review_mode),
            ]),
        );
        for (lens, outcome) in [
            ("aurelius", result.aurelius),
            ("bacon", result.bacon),
            ("sun_tzu", result.sun_tzu),
        ] {
            self.increment(
                "arda_governance_triad_lens_outcomes_total",
                labels(&[
                    ("lens", lens),
                    ("outcome", outcome_label(outcome)),
                    ("policy_version", version),
                    ("scorer_version", scorer_version),
                    ("review_mode", review_mode),
                ]),
            );
        }
    }

    pub fn observe_bacon_lite(&self, event: &BaconLiteEvent) {
        self.increment(
            "arda_governance_bacon_lite_total",
            labels(&[
                ("verdict", verdict_label(event.passed)),
                (
                    "policy_version",
                    version_class(&event.policy_version, BACON_LITE_POLICY_VERSION),
                ),
                (
                    "scorer_version",
                    version_class(&event.scorer_version, TRIAD_POLICY_VERSION),
                ),
                ("review_mode", review_mode_label(event.review_mode)),
            ]),
        );
    }

    pub fn observe_resonance(&self, score: &ResonanceScore) {
        let _version = version_class(&score.policy_version, RESONANCE_POLICY_VERSION);
        self.observe_histogram("arda_governance_resonance", score.value / 100.0);
    }

    pub fn observe_love_proxy(&self, score: &LoveEquationScore) {
        let _version = version_class(&score.policy_version, LOVE_EQUATION_POLICY_VERSION);
        self.observe_histogram("arda_governance_love_proxy", score.score);
    }

    pub fn observe_love_dynamics(&self, score: &LoveDynamicsScore) {
        self.observe_histogram(
            "arda_governance_love_dynamics_projected_empathy",
            score.projected_empathy,
        );
    }

    pub fn observe_joule_honesty(&self, profile: &JouleWorkProfile) {
        let _version = version_class(&profile.policy_version, JOULEWORK_POLICY_VERSION);
        self.observe_histogram("arda_governance_joule_honesty", profile.honesty_ratio);
    }

    pub fn observe_environmental(&self, coherence: &EnvironmentalCoherence) {
        let version = version_class(&coherence.policy_version, ENVIRONMENTAL_POLICY_VERSION);
        self.increment(
            "arda_governance_environmental_assessments_total",
            labels(&[
                ("advisory", environmental_advisory_label(coherence.advisory)),
                ("policy_version", version),
            ]),
        );
        self.observe_histogram(
            "arda_governance_environmental_coherence",
            coherence.score / 100.0,
        );
        for envelope in &coherence.signals {
            self.increment(
                "arda_governance_environmental_signals_total",
                labels(&[
                    ("source", environmental_source_label(envelope.source)),
                    ("health", signal_health_label(&envelope.health)),
                    ("freshness", signal_freshness_label(envelope.freshness)),
                    (
                        "measurement_quality",
                        measurement_quality_label(envelope.measurement_quality),
                    ),
                    ("policy_version", version),
                ]),
            );
        }
    }

    pub fn snapshot(&self) -> GovernanceMetricsSnapshot {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        GovernanceMetricsSnapshot {
            collection_mode: "library_owned_in_process_caller_exposed".to_string(),
            owns_http_server: false,
            bacon_lite_writer: crate::global_bacon_lite_counters(),
            counters: state
                .counters
                .iter()
                .map(|(key, value)| GovernanceCounterSnapshot {
                    name: key.name.clone(),
                    labels: key.labels.clone(),
                    value: *value,
                })
                .collect(),
            histograms: state
                .histograms
                .iter()
                .map(|(name, histogram)| GovernanceHistogramSnapshot {
                    name: name.clone(),
                    count: histogram.count,
                    sum: histogram.sum,
                    buckets: HISTOGRAM_BOUNDS
                        .iter()
                        .zip(histogram.buckets)
                        .map(
                            |(upper_bound, cumulative_count)| GovernanceHistogramBucket {
                                upper_bound: *upper_bound,
                                cumulative_count,
                            },
                        )
                        .collect(),
                })
                .collect(),
        }
    }

    fn increment(&self, name: &str, labels: BTreeMap<String, String>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *state
            .counters
            .entry(CounterKey {
                name: name.to_string(),
                labels,
            })
            .or_default() += 1;
    }

    fn observe_histogram(&self, name: &str, value: f64) {
        let value = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let histogram = state.histograms.entry(name.to_string()).or_default();
        histogram.count += 1;
        histogram.sum += value;
        for (index, upper_bound) in HISTOGRAM_BOUNDS.iter().enumerate() {
            if value <= *upper_bound {
                histogram.buckets[index] += 1;
            }
        }
    }
}

pub fn global_governance_metrics() -> &'static GovernanceMetrics {
    static METRICS: OnceLock<GovernanceMetrics> = OnceLock::new();
    METRICS.get_or_init(GovernanceMetrics::new)
}

fn labels(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn verdict_label(passed: bool) -> &'static str {
    if passed {
        "pass"
    } else {
        "fail"
    }
}

fn outcome_label(outcome: GateOutcome) -> &'static str {
    match outcome {
        GateOutcome::Pass => "pass",
        GateOutcome::Fail => "fail",
        GateOutcome::Conditional => "conditional",
    }
}

fn review_mode_label(mode: GovernanceReviewMode) -> &'static str {
    match mode {
        GovernanceReviewMode::HeuristicLocal => "heuristic_local",
        GovernanceReviewMode::IndependentAgent => "independent_agent",
        GovernanceReviewMode::HumanReviewed => "human_reviewed",
        GovernanceReviewMode::ConsensusReceipted => "consensus_receipted",
    }
}

fn environmental_advisory_label(advisory: EnvironmentalAdvisory) -> &'static str {
    match advisory {
        EnvironmentalAdvisory::Supportive => "supportive",
        EnvironmentalAdvisory::Neutral => "neutral",
        EnvironmentalAdvisory::Caution => "caution",
    }
}

fn environmental_source_label(source: crate::GovernanceSignalSource) -> &'static str {
    match source {
        crate::GovernanceSignalSource::Audio => "audio",
        crate::GovernanceSignalSource::Vision => "vision",
        crate::GovernanceSignalSource::Solar => "solar",
    }
}

fn signal_health_label(health: &SignalHealth) -> &'static str {
    match health {
        SignalHealth::Healthy => "healthy",
        SignalHealth::Degraded { .. } => "degraded",
        SignalHealth::Unavailable { .. } => "unavailable",
    }
}

fn signal_freshness_label(freshness: SignalFreshness) -> &'static str {
    match freshness {
        SignalFreshness::Fresh => "fresh",
        SignalFreshness::Stale => "stale",
        SignalFreshness::Unknown => "unknown",
    }
}

fn measurement_quality_label(quality: MeasurementQuality) -> &'static str {
    match quality {
        MeasurementQuality::Measured => "measured",
        MeasurementQuality::Derived => "derived",
        MeasurementQuality::Defaulted => "defaulted",
        MeasurementQuality::Unavailable => "unavailable",
    }
}

fn version_class(version: &str, current: &str) -> &'static str {
    if version == current {
        "current"
    } else if version.contains("v1") {
        "legacy"
    } else {
        "other"
    }
}
