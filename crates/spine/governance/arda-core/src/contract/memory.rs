use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::contract_version;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Episodic,
    Semantic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Active,
    Decayed,
    Revoked,
    Promoted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub contract_version: String,
    pub id: String,
    pub kind: MemoryKind,
    pub agent: String,
    pub content: String,
    pub salience: f64,
    pub evidence_count: u32,
    pub state: MemoryState,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl MemoryRecord {
    pub fn new(
        id: impl Into<String>,
        kind: MemoryKind,
        agent: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            contract_version: contract_version(),
            id: id.into(),
            kind,
            agent: agent.into(),
            content: content.into(),
            salience: 0.5,
            evidence_count: 1,
            state: MemoryState::Active,
            created_at: now,
            last_seen_at: now,
            extensions: HashMap::new(),
        }
    }
}
