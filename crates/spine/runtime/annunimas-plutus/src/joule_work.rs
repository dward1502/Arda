// sigil: REPAIR
use annunimas_core::JouleWorkMeasurementSource;
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
    pub average_confidence: f64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct JouleWorkTracker {
    entries: Arc<RwLock<Vec<JouleWork>>>,
}

impl JouleWorkTracker {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
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
            task_id,
            unit,
            measurement_source,
            measurement_confidence: measurement_confidence.clamp(0.0, 1.0),
        };
        self.track(jw).await;
    }

    pub async fn summary(&self) -> JouleWorkSummary {
        let entries = self.entries.read().await;

        let mut by_unit: HashMap<JouleWorkUnit, f64> = HashMap::new();
        let mut by_agent: HashMap<String, f64> = HashMap::new();
        let mut by_source: HashMap<JouleWorkMeasurementSource, f64> = HashMap::new();
        let mut total = 0.0;
        let mut observed_total = 0.0;
        let mut default_fallback_total = 0.0;
        let mut confidence_total = 0.0;
        let mut confidence_samples = 0usize;

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
            average_confidence: if confidence_samples > 0 {
                confidence_total / confidence_samples as f64
            } else {
                0.0
            },
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
                "average_confidence": summary.average_confidence,
                "autonomy_truth_warning": summary.default_fallback_total > 0.0,
                "default_fallback_autonomy_truth": false
            },
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
                    unit: JouleWorkUnit::Reasoning,
                    measurement_source: JouleWorkMeasurementSource::DefaultFallback,
                    measurement_confidence: 0.0,
                });
            }
        }
        let mut entries = self.entries.write().await;
        *entries = restored;
    }
}

impl Default for JouleWorkTracker {
    fn default() -> Self {
        Self::new()
    }
}
