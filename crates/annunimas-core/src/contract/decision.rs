use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::contract_version;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionClass {
    Dispatch,
    Governance,
    Budget,
    Retire,
    Bid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriadVerdict {
    Pass,
    Conditional,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilosopherVerdict {
    pub verdict: TriadVerdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadOutcome {
    pub verdict: TriadVerdict,
    pub aurelius: PhilosopherVerdict,
    pub bacon: PhilosopherVerdict,
    pub sun_tzu: PhilosopherVerdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub contract_version: String,
    pub id: String,
    pub decided_at: DateTime<Utc>,
    pub decision_class: DecisionClass,
    pub subject_id: String,
    pub options_considered: Vec<String>,
    pub chosen: String,
    pub rationale: String,
    pub triad: TriadOutcome,
    pub love_score: f64,
    pub resonance: f64,
    pub joule_estimate: f64,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl Decision {
    pub fn new(
        id: impl Into<String>,
        decision_class: DecisionClass,
        subject_id: impl Into<String>,
        chosen: impl Into<String>,
        rationale: impl Into<String>,
        triad: TriadOutcome,
    ) -> Self {
        Self {
            contract_version: contract_version(),
            id: id.into(),
            decided_at: Utc::now(),
            decision_class,
            subject_id: subject_id.into(),
            options_considered: Vec::new(),
            chosen: chosen.into(),
            rationale: rationale.into(),
            triad,
            love_score: 0.0,
            resonance: 0.0,
            joule_estimate: 0.0,
            extensions: HashMap::new(),
        }
    }
}
