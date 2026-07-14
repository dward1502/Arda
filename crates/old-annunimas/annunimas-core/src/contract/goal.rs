use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::contract_version;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Paused,
    Achieved,
    Abandoned,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub contract_version: String,
    pub id: String,
    pub title: String,
    pub intent: String,
    pub owner_agent: String,
    pub status: GoalStatus,
    pub priority: GoalPriority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joule_budget_per_day: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl Goal {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        intent: impl Into<String>,
        owner_agent: impl Into<String>,
        priority: GoalPriority,
    ) -> Self {
        let now = Utc::now();
        Self {
            contract_version: contract_version(),
            id: id.into(),
            title: title.into(),
            intent: intent.into(),
            owner_agent: owner_agent.into(),
            status: GoalStatus::Active,
            priority,
            joule_budget_per_day: None,
            created_at: now,
            updated_at: now,
            extensions: HashMap::new(),
        }
    }
}
