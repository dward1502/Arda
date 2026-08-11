// sigil: REPAIR
use arda_core::JouleWorkMeasurementSource;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleWork {
    pub amount: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub agent_id: String,
    pub task_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    pub unit: JouleWorkUnit,
    #[serde(default)]
    pub measurement_source: JouleWorkMeasurementSource,
    #[serde(default)]
    pub measurement_confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum JouleWorkUnit {
    Compute,
    Network,
    Storage,
    Attention,
    Reasoning,
}

impl JouleWorkUnit {
    pub fn multiplier(&self) -> f64 {
        match self {
            JouleWorkUnit::Compute => 1.0,
            JouleWorkUnit::Network => 0.5,
            JouleWorkUnit::Storage => 0.3,
            JouleWorkUnit::Attention => 1.5,
            JouleWorkUnit::Reasoning => 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleWorkSummary {
    pub total: f64,
    pub by_unit: HashMap<JouleWorkUnit, f64>,
    pub by_agent: HashMap<String, f64>,
    pub by_source: HashMap<JouleWorkMeasurementSource, f64>,
    pub observed_total: f64,
    pub default_fallback_total: f64,
    pub synthetic_restoration_total: f64,
    pub average_confidence: f64,
    pub by_run: HashMap<String, JouleWorkRunSummary>,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JouleWorkRunSummary {
    pub total: f64,
    pub by_source: HashMap<JouleWorkMeasurementSource, f64>,
    pub average_confidence: f64,
    pub autonomy_truth_allowed: bool,
    #[serde(default)]
    pub measurement_count: usize,
    #[serde(default)]
    pub continuity_only: bool,
}

#[derive(Clone)]
pub struct JouleWorkTracker {
    entries: Arc<RwLock<Vec<JouleWork>>>,
    restored_runs: Arc<RwLock<HashMap<String, JouleWorkRunSummary>>>,
}

impl JouleWorkTracker {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            restored_runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn track(&self, jw: JouleWork) {
        if !jw.amount.is_finite() {
            return;
        }
        let mut entries = self.entries.write().await;
        entries.push(jw);
    }

    pub async fn track_work(
        &self,
        agent_id: impl Into<String>,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) {
        self.track_work_with_source(
            agent_id,
            amount,
            unit,
            task_id,
            JouleWorkMeasurementSource::DefaultFallback,
            0.0,
        )
        .await;
    }

    pub async fn track_work_with_source(
        &self,
        agent_id: impl Into<String>,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
        measurement_source: JouleWorkMeasurementSource,
        measurement_confidence: f64,
    ) {
        if !amount.is_finite() {
            return;
        }
        let jw = JouleWork {
            amount: amount * unit.multiplier(),
            timestamp: chrono::Utc::now(),
            agent_id: agent_id.into(),
            run_id: task_id.clone(),
            task_id,
            unit,
            measurement_source,
            measurement_confidence: measurement_confidence.clamp(0.0, 1.0),
        };
        self.track(jw).await;
    }

    pub async fn summary(&self) -> JouleWorkSummary {
        let entries = self.entries.read().await;
        let restored_runs = self.restored_runs.read().await;

        let mut by_unit: HashMap<JouleWorkUnit, f64> = HashMap::new();
        let mut by_agent: HashMap<String, f64> = HashMap::new();
        let mut by_source: HashMap<JouleWorkMeasurementSource, f64> = HashMap::new();
        let mut total = 0.0;
        let mut observed_total = 0.0;
        let mut default_fallback_total = 0.0;
        let mut synthetic_restoration_total = 0.0;
        let mut confidence_total = 0.0;
        let mut confidence_samples = 0usize;
        let mut by_run = restored_runs.clone();
        let mut run_confidence = by_run
            .iter()
            .map(|(run_id, run)| {
                (
                    run_id.clone(),
                    (
                        run.average_confidence * run.measurement_count as f64,
                        run.measurement_count,
                    ),
                )
            })
            .collect::<HashMap<String, (f64, usize)>>();

        for entry in entries.iter() {
            if !entry.amount.is_finite() {
                continue;
            }
            *by_unit.entry(entry.unit).or_insert(0.0) += entry.amount;
            *by_agent.entry(entry.agent_id.clone()).or_insert(0.0) += entry.amount;
            *by_source.entry(entry.measurement_source).or_insert(0.0) += entry.amount;
            total += entry.amount;
            confidence_total += entry.measurement_confidence.clamp(0.0, 1.0);
            confidence_samples += 1;
            if entry.measurement_source.is_observed() {
                observed_total += entry.amount;
            }
            if entry.measurement_source == JouleWorkMeasurementSource::DefaultFallback {
                default_fallback_total += entry.amount;
            }
            if entry.measurement_source == JouleWorkMeasurementSource::SyntheticRestoration {
                synthetic_restoration_total += entry.amount;
            }
            if let Some(run_id) = &entry.run_id {
                let run = by_run
                    .entry(run_id.clone())
                    .or_insert_with(|| JouleWorkRunSummary {
                        total: 0.0,
                        by_source: HashMap::new(),
                        average_confidence: 0.0,
                        autonomy_truth_allowed: true,
                        measurement_count: 0,
                        continuity_only: false,
                    });
                run.total += entry.amount;
                *run.by_source.entry(entry.measurement_source).or_insert(0.0) += entry.amount;
                run.autonomy_truth_allowed &= entry.measurement_source.is_autonomy_truth();
                run.measurement_count += 1;
                let confidence = run_confidence.entry(run_id.clone()).or_insert((0.0, 0));
                confidence.0 += entry.measurement_confidence.clamp(0.0, 1.0);
                confidence.1 += 1;
            }
        }
        for (run_id, run) in &mut by_run {
            let (total_confidence, samples) = run_confidence[run_id];
            run.average_confidence = total_confidence / samples as f64;
        }

        let period_start = entries
            .first()
            .map(|e| e.timestamp)
            .unwrap_or_else(chrono::Utc::now);
        let period_end = entries
            .last()
            .map(|e| e.timestamp)
            .unwrap_or_else(chrono::Utc::now);

        JouleWorkSummary {
            total,
            by_unit,
            by_agent,
            by_source,
            observed_total,
            default_fallback_total,
            synthetic_restoration_total,
            average_confidence: if confidence_samples > 0 {
                confidence_total / confidence_samples as f64
            } else {
                0.0
            },
            by_run,
            period_start,
            period_end,
        }
    }

    pub async fn agent_total(&self, agent_id: &str) -> f64 {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.agent_id == agent_id)
            .map(|e| e.amount)
            .sum()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        let mut restored_runs = self.restored_runs.write().await;
        restored_runs.clear();
    }

    pub async fn status_snapshot(&self) -> serde_json::Value {
        let summary = self.summary().await;
        json!({
            "total": summary.total,
            "by_unit": summary
                .by_unit
                .iter()
                .map(|(unit, amount)| json!({"unit": format!("{unit:?}"), "amount": amount}))
                .collect::<Vec<_>>(),
            "by_agent": summary.by_agent,
            "by_source": summary
                .by_source
                .iter()
                .map(|(source, amount)| {
                    json!({
                        "source": serde_json::to_value(source)
                            .unwrap_or_else(|_| json!("unknown")),
                        "amount": amount
                    })
                })
                .collect::<Vec<_>>(),
            "measurement_metadata": {
                "observed_total": summary.observed_total,
                "default_fallback_total": summary.default_fallback_total,
                "synthetic_restoration_total": summary.synthetic_restoration_total,
                "average_confidence": summary.average_confidence,
                "autonomy_truth_warning": summary.default_fallback_total > 0.0
                    || summary.synthetic_restoration_total > 0.0,
                "default_fallback_autonomy_truth": false,
                "synthetic_restoration_autonomy_truth": false
            },
            "runs": summary.by_run,
            "period_start": summary.period_start,
            "period_end": summary.period_end,
        })
    }

    /// Restore from a summary snapshot.
    ///
    /// Current snapshots contain aggregate `by_agent` totals, not the original
    /// per-entry ledger. Rehydrated rows are therefore explicitly synthetic and
    /// suitable for continuity totals only, not audit-grade entry fidelity.
    pub async fn restore_from_snapshot(&self, snapshot: &serde_json::Value) {
        let mut restored = Vec::new();
        let mut restored_runs = snapshot
            .get("runs")
            .cloned()
            .and_then(|runs| {
                serde_json::from_value::<HashMap<String, JouleWorkRunSummary>>(runs).ok()
            })
            .unwrap_or_default();
        for run in restored_runs.values_mut() {
            run.autonomy_truth_allowed = false;
            run.continuity_only = true;
        }
        let now = chrono::Utc::now();
        if let Some(by_agent) = snapshot.get("by_agent").and_then(|v| v.as_object()) {
            for (agent_id, amount) in by_agent {
                let Some(amount) = amount.as_f64().filter(|amount| amount.is_finite()) else {
                    continue;
                };
                restored.push(JouleWork {
                    amount,
                    timestamp: now,
                    agent_id: agent_id.clone(),
                    task_id: None,
                    run_id: None,
                    unit: JouleWorkUnit::Reasoning,
                    measurement_source: JouleWorkMeasurementSource::SyntheticRestoration,
                    measurement_confidence: 0.0,
                });
            }
        }
        let mut entries = self.entries.write().await;
        *entries = restored;
        let mut stored_runs = self.restored_runs.write().await;
        *stored_runs = restored_runs;
    }
}

impl Default for JouleWorkTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "operator-scale provenance and multiplier regression"]
    async fn operator_scale_session_preserves_unit_and_source_invariants() {
        let tracker = JouleWorkTracker::new();
        let units = [
            JouleWorkUnit::Compute,
            JouleWorkUnit::Network,
            JouleWorkUnit::Storage,
            JouleWorkUnit::Attention,
            JouleWorkUnit::Reasoning,
        ];

        for batch in 0..1_000 {
            for unit in units {
                tracker
                    .track_work_with_source(
                        "operator-scale-agent",
                        1.0,
                        unit,
                        Some(format!("observed-{batch}-{unit:?}")),
                        JouleWorkMeasurementSource::RuntimeTimer,
                        1.0,
                    )
                    .await;
                tracker
                    .track_work_with_source(
                        "operator-scale-agent",
                        1.0,
                        unit,
                        Some(format!("fallback-{batch}-{unit:?}")),
                        JouleWorkMeasurementSource::DefaultFallback,
                        0.0,
                    )
                    .await;
            }
        }

        let summary = tracker.summary().await;
        assert!((summary.total - 10_600.0).abs() < 1e-6);
        assert!((summary.observed_total - 5_300.0).abs() < 1e-6);
        assert!((summary.default_fallback_total - 5_300.0).abs() < 1e-6);
        assert!((summary.average_confidence - 0.5).abs() < 1e-9);
        for unit in units {
            let expected = unit.multiplier() * 2_000.0;
            assert!((summary.by_unit[&unit] - expected).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn run_projection_persists_source_confidence_and_autonomy_truth() {
        let tracker = JouleWorkTracker::new();
        tracker
            .track_work_with_source(
                "agent",
                2.0,
                JouleWorkUnit::Compute,
                Some("observed-run".to_string()),
                JouleWorkMeasurementSource::RuntimeTimer,
                0.9,
            )
            .await;
        tracker
            .track_work_with_source(
                "agent",
                3.0,
                JouleWorkUnit::Compute,
                Some("fallback-run".to_string()),
                JouleWorkMeasurementSource::DefaultFallback,
                0.2,
            )
            .await;

        let snapshot = tracker.status_snapshot().await;
        assert_eq!(snapshot["runs"]["observed-run"]["average_confidence"], 0.9);
        assert_eq!(
            snapshot["runs"]["observed-run"]["autonomy_truth_allowed"],
            true
        );
        assert_eq!(snapshot["runs"]["observed-run"]["continuity_only"], false);
        assert_eq!(snapshot["runs"]["fallback-run"]["average_confidence"], 0.2);
        assert_eq!(
            snapshot["runs"]["fallback-run"]["autonomy_truth_allowed"],
            false
        );
    }

    #[tokio::test]
    async fn restored_aggregate_is_explicitly_synthetic_and_not_autonomy_truth() {
        let tracker = JouleWorkTracker::new();
        tracker
            .restore_from_snapshot(&json!({"by_agent": {"agent": 5.0}}))
            .await;

        let summary = tracker.summary().await;
        assert_eq!(summary.synthetic_restoration_total, 5.0);
        assert_eq!(summary.observed_total, 0.0);
        assert!(summary
            .by_source
            .contains_key(&JouleWorkMeasurementSource::SyntheticRestoration));
    }

    #[tokio::test]
    async fn restored_run_metadata_retains_provenance_but_loses_autonomy_authority() {
        let tracker = JouleWorkTracker::new();
        tracker
            .restore_from_snapshot(&json!({
                "by_agent": {},
                "runs": {
                    "run-1": {
                        "total": 2.0,
                        "by_source": {"runtime_timer": 2.0},
                        "average_confidence": 0.9,
                        "autonomy_truth_allowed": true,
                        "measurement_count": 1,
                        "continuity_only": false
                    }
                }
            }))
            .await;

        let summary = tracker.summary().await;
        let restored = &summary.by_run["run-1"];
        assert_eq!(restored.average_confidence, 0.9);
        assert_eq!(
            restored.by_source[&JouleWorkMeasurementSource::RuntimeTimer],
            2.0
        );
        assert!(restored.continuity_only);
        assert!(!restored.autonomy_truth_allowed);
    }
}
