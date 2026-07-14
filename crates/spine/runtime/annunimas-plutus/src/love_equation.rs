// sigil: REPAIR
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoveScore {
    pub agent_a: String,
    pub agent_b: String,
    pub value: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoveConfig {
    pub resonance_weight: f64,
    pub attention_weight: f64,
    pub reciprocity_weight: f64,
}

impl Default for LoveConfig {
    fn default() -> Self {
        Self {
            resonance_weight: 0.4,
            attention_weight: 0.3,
            reciprocity_weight: 0.3,
        }
    }
}

pub struct LoveEquation {
    config: LoveConfig,
    relationships: HashMap<(String, String), LoveScore>,
}

impl LoveEquation {
    pub fn new() -> Self {
        Self {
            config: LoveConfig::default(),
            relationships: HashMap::new(),
        }
    }

    pub fn with_config(config: LoveConfig) -> Self {
        Self {
            config,
            relationships: HashMap::new(),
        }
    }

    pub fn calculate(
        &self,
        _agent_a: &str,
        _agent_b: &str,
        resonance: f64,
        attention: f64,
        reciprocity: f64,
    ) -> f64 {
        let r_weighted = resonance * self.config.resonance_weight;
        let a_weighted = attention * self.config.attention_weight;
        let rec_weighted = reciprocity * self.config.reciprocity_weight;

        (r_weighted + a_weighted + rec_weighted).clamp(0.0, 1.0)
    }

    pub fn record_relationship(
        &mut self,
        agent_a: impl Into<String>,
        agent_b: impl Into<String>,
        value: f64,
    ) {
        let agent_a = agent_a.into();
        let agent_b = agent_b.into();
        let score = LoveScore {
            agent_a: agent_a.clone(),
            agent_b: agent_b.clone(),
            value,
            timestamp: chrono::Utc::now(),
        };

        let key = if agent_a < agent_b {
            (agent_a, agent_b)
        } else {
            (agent_b, agent_a)
        };

        self.relationships.insert(key, score);
    }

    pub fn get_relationship(&self, agent_a: &str, agent_b: &str) -> Option<&LoveScore> {
        if agent_a < agent_b {
            self.relationships
                .get_key_value(&(agent_a.to_owned(), agent_b.to_owned()))
                .map(|(_, score)| score)
        } else {
            self.relationships
                .get_key_value(&(agent_b.to_owned(), agent_a.to_owned()))
                .map(|(_, score)| score)
        }
    }

    pub fn all_relationships(&self) -> Vec<&LoveScore> {
        self.relationships.values().collect()
    }

    pub fn top_relationships(&self, n: usize) -> Vec<&LoveScore> {
        let mut scores: Vec<_> = self.relationships.values().collect();
        scores.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores.into_iter().take(n).collect()
    }

    pub fn snapshot(&self, top_n: usize) -> serde_json::Value {
        json!({
            "relationships_total": self.relationships.len(),
            "relationships": self
                .all_relationships()
                .into_iter()
                .map(|score| json!({
                    "agent_a": score.agent_a,
                    "agent_b": score.agent_b,
                    "value": score.value,
                    "timestamp": score.timestamp,
                }))
                .collect::<Vec<_>>(),
            "top_relationships": self
                .top_relationships(top_n)
                .into_iter()
                .map(|score| json!({
                    "agent_a": score.agent_a,
                    "agent_b": score.agent_b,
                    "value": score.value,
                    "timestamp": score.timestamp,
                }))
                .collect::<Vec<_>>(),
        })
    }

    pub fn restore_from_snapshot(&mut self, snapshot: &serde_json::Value) {
        self.relationships.clear();
        let Some(rows) = snapshot.get("relationships").and_then(|v| v.as_array()) else {
            return;
        };
        for row in rows {
            let Some(agent_a) = row.get("agent_a").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(agent_b) = row.get("agent_b").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(value) = row.get("value").and_then(|v| v.as_f64()) else {
                continue;
            };
            self.record_relationship(agent_a, agent_b, value);
        }
    }
}

impl Default for LoveEquation {
    fn default() -> Self {
        Self::new()
    }
}
