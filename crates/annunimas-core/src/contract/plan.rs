use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::contract_version;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Ready,
    Dispatched,
    Done,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub intent: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub contract_version: String,
    pub id: String,
    pub goal_id: String,
    pub summary: String,
    pub steps: Vec<PlanStep>,
    pub status: PlanStatus,
    #[serde(default)]
    pub lessons_consulted: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl Plan {
    pub fn new(
        id: impl Into<String>,
        goal_id: impl Into<String>,
        summary: impl Into<String>,
        steps: Vec<PlanStep>,
    ) -> Self {
        let now = Utc::now();
        Self {
            contract_version: contract_version(),
            id: id.into(),
            goal_id: goal_id.into(),
            summary: summary.into(),
            steps,
            status: PlanStatus::Draft,
            lessons_consulted: Vec::new(),
            created_at: now,
            updated_at: now,
            extensions: HashMap::new(),
        }
    }
}
