use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::contract_version;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReflectionOutcome {
    Success,
    Partial,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub contract_version: String,
    pub id: String,
    pub task_id: String,
    pub plan_id: String,
    pub completed_at: DateTime<Utc>,
    pub outcome: ReflectionOutcome,
    pub score: f64,
    pub narrative: String,
    pub joule_estimated: f64,
    pub joule_actual: f64,
    #[serde(default)]
    pub lessons_emitted: Vec<String>,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl Reflection {
    pub fn new(
        id: impl Into<String>,
        task_id: impl Into<String>,
        plan_id: impl Into<String>,
        outcome: ReflectionOutcome,
        score: f64,
    ) -> Self {
        Self {
            contract_version: contract_version(),
            id: id.into(),
            task_id: task_id.into(),
            plan_id: plan_id.into(),
            completed_at: Utc::now(),
            outcome,
            score,
            narrative: String::new(),
            joule_estimated: 0.0,
            joule_actual: 0.0,
            lessons_emitted: Vec::new(),
            extensions: HashMap::new(),
        }
    }

    pub fn honesty_delta(&self) -> f64 {
        self.joule_actual - self.joule_estimated
    }
}
