// sigil: REPAIR
//! Shared learning primitives for autonomous adaptation.
//!
//! Owned by `annunimas-core` so Prometheus, Hades, Athena, and CLI
//! can consume the same state without pulling in the autopilot crate.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutcomeStats {
    pub attempts: u64,
    pub successes: u64,
    pub avg_duration_secs: f64,
    pub avg_joules: f64,
}

impl OutcomeStats {
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.5
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }

    pub fn observe(&mut self, success: bool, duration_secs: f64, joules: f64) {
        let n = self.attempts as f64;
        self.attempts += 1;
        if success {
            self.successes += 1;
        }
        self.avg_duration_secs =
            (self.avg_duration_secs * n + duration_secs) / (self.attempts as f64);
        self.avg_joules = (self.avg_joules * n + joules) / (self.attempts as f64);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningState {
    /// keyed by `agent::task_type`
    pub stats: BTreeMap<String, OutcomeStats>,
}

impl LearningState {
    fn key(agent: &str, task_type: &str) -> String {
        format!("{agent}::{task_type}")
    }

    pub fn observe(&mut self, agent: &str, task_type: &str, success: bool, dur: f64, joules: f64) {
        self.stats
            .entry(Self::key(agent, task_type))
            .or_default()
            .observe(success, dur, joules);
    }

    pub fn routing_bias(&self, agent: &str, task_type: &str) -> f64 {
        self.stats
            .get(&Self::key(agent, task_type))
            .map(|s| s.success_rate())
            .unwrap_or(0.5)
    }

    pub fn best_agent(&self, task_type: &str) -> Option<String> {
        self.stats
            .iter()
            .filter_map(|(k, s)| {
                let (agent, tt) = k.split_once("::")?;
                if tt == task_type && s.attempts >= 3 {
                    Some((agent.to_string(), s.success_rate()))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(a, _)| a)
    }
}

pub struct LearningStore {
    path: PathBuf,
}

impl LearningStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> LearningState {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, state: &LearningState) -> std::io::Result<()> {
        if let Some(p) = self.path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let data = serde_json::to_vec_pretty(state).map_err(std::io::Error::other)?;
        std::fs::write(&self.path, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_best_agent_for_type() {
        let mut s = LearningState::default();
        for _ in 0..5 {
            s.observe("a", "ops", true, 1.0, 1.0);
        }
        for _ in 0..5 {
            s.observe("b", "ops", false, 1.0, 1.0);
        }
        assert_eq!(s.best_agent("ops").as_deref(), Some("a"));
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learn.json");
        let store = LearningStore::new(&path);
        let mut st = LearningState::default();
        st.observe("a", "ops", true, 2.0, 3.0);
        store.save(&st).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.stats.len(), 1);
    }

    #[test]
    fn routing_bias_defaults_without_evidence() {
        let s = LearningState::default();
        assert_eq!(s.routing_bias("z", "ops"), 0.5);
    }

    #[test]
    fn routing_bias_reflects_observed_success_rate() {
        let mut s = LearningState::default();
        for _ in 0..3 {
            s.observe("k", "burden", true, 0.5, 1.0);
        }
        assert_eq!(s.routing_bias("k", "burden"), 1.0);
    }

    #[test]
    fn observe_only_updates_matching_key() {
        let mut s = LearningState::default();
        s.observe("a", "ops", true, 1.0, 1.0);
        s.observe("b", "ops", false, 1.0, 1.0);
        assert_eq!(s.stats.len(), 2);
        assert_eq!(s.stats["a::ops"].attempts, 1);
        assert_eq!(s.stats["a::ops"].successes, 1);
        assert_eq!(s.stats["b::ops"].attempts, 1);
        assert_eq!(s.stats["b::ops"].successes, 0);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum GateState {
    #[serde(rename = "pending")]
    #[default]
    Pending,
    #[serde(rename = "approved")]
    Approved { by: Vec<String>, at: String },
    #[serde(rename = "rejected")]
    Rejected { reason: String, at: String },
}

impl GateState {
    pub fn is_approved(&self) -> bool {
        matches!(self, GateState::Approved { .. })
    }

    pub fn is_closed(&self) -> bool {
        matches!(
            self,
            GateState::Approved { .. } | GateState::Rejected { .. }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningLoopLifecyclePacket {
    pub id: String,
    #[serde(default)]
    pub proposal_id: String,
    pub packet_kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub created_at_utc: String,
    #[serde(default)]
    pub requester: String,
    pub phase: String,
    pub gate_state: GateState,
}

impl LearningLoopLifecyclePacket {
    pub fn approve(mut self, by: Vec<String>) -> anyhow::Result<Self> {
        if self.gate_state.is_closed() {
            anyhow::bail!(
                "lifecycle packet {} is already closed (gate_state={:?})",
                self.id,
                self.gate_state
            );
        }
        self.gate_state = GateState::Approved {
            by,
            at: Utc::now().to_rfc3339(),
        };
        Ok(self)
    }

    pub fn reject(mut self, reason: String) -> anyhow::Result<Self> {
        if self.gate_state.is_closed() {
            anyhow::bail!(
                "lifecycle packet {} is already closed (gate_state={:?})",
                self.id,
                self.gate_state
            );
        }
        self.gate_state = GateState::Rejected {
            reason,
            at: Utc::now().to_rfc3339(),
        };
        Ok(self)
    }

    pub fn into_receipt(self) -> LifecycleReceipt {
        let (verdict, notes) = match &self.gate_state {
            GateState::Approved { by, at } => {
                let verdict = format!("approved_by={} at={}", by.join(","), at);
                let notes = "lifecycle gate closed by HADES review".to_owned();
                (verdict, notes)
            }
            GateState::Rejected { reason, at } => {
                let verdict = format!("rejected at={} reason={}", at, reason);
                let notes = "lifecycle gate denied; mutation rejected".to_owned();
                (verdict, notes)
            }
            GateState::Pending => (
                "pending".to_owned(),
                "gate still open; awaiting HADES lifecycle review".to_owned(),
            ),
        };

        LifecycleReceipt {
            proposal_id: self.proposal_id.clone(),
            verdict,
            approval_packet_path: String::new(),
            mutated_locally: false,
            processed_at_utc: Utc::now().to_rfc3339(),
            notes,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningTaskProposal {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source_delta_id: String,
    #[serde(default)]
    pub risk_level: String,
    #[serde(default)]
    pub risk_class: String,
    #[serde(default)]
    pub proposed_at: String,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl Default for LearningLoopLifecyclePacket {
    fn default() -> Self {
        Self {
            id: String::new(),
            proposal_id: String::new(),
            packet_kind: String::new(),
            payload: serde_json::Value::Null,
            created_at_utc: String::new(),
            requester: String::new(),
            phase: String::new(),
            gate_state: GateState::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningLoopVerdict {
    pub proposal_id: String,
    pub verdict: String,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub queried_at_utc: String,
    #[serde(default)]
    pub requested_by: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecyclePacket {
    pub id: String,
    #[serde(default)]
    pub proposal_id: String,
    pub packet_kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub created_at_utc: String,
    #[serde(default)]
    pub requester: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleReceipt {
    pub proposal_id: String,
    pub verdict: String,
    #[serde(default)]
    pub approval_packet_path: String,
    #[serde(default)]
    pub mutated_locally: bool,
    #[serde(default)]
    pub processed_at_utc: String,
    #[serde(default)]
    pub notes: String,
}

impl LearningTaskProposal {
    pub fn is_low_risk(&self) -> bool {
        self.risk_class == "low"
    }
}
