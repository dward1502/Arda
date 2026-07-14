use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::contract_version;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LedgerKind {
    Goal,
    Plan,
    Task,
    Decision,
    Reflection,
    Memory,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub contract_version: String,
    pub id: String,
    pub ts: DateTime<Utc>,
    pub agent: String,
    pub kind: LedgerKind,
    pub payload: serde_json::Value,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl LedgerEntry {
    pub fn new(
        id: impl Into<String>,
        agent: impl Into<String>,
        kind: LedgerKind,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            contract_version: contract_version(),
            id: id.into(),
            ts: Utc::now(),
            agent: agent.into(),
            kind,
            payload,
            extensions: HashMap::new(),
        }
    }
}
