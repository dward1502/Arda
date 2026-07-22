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
        let mut relationships = self.relationships.values().collect::<Vec<_>>();
        relationships.sort_by(|a, b| (&a.agent_a, &a.agent_b).cmp(&(&b.agent_a, &b.agent_b)));
        relationships
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
            let Ok(score) = serde_json::from_value::<LoveScore>(row.clone()) else {
                continue;
            };
            if !score.value.is_finite() {
                continue;
            }
            let key = if score.agent_a < score.agent_b {
                (score.agent_a.clone(), score.agent_b.clone())
            } else {
                (score.agent_b.clone(), score.agent_a.clone())
            };
            self.relationships.insert(key, score);
        }
    }
}

impl Default for LoveEquation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{LoveConfig, LoveEquation};

    #[test]
    fn custom_weights_and_bounds_are_applied() {
        let equation = LoveEquation::with_config(LoveConfig {
            resonance_weight: 0.5,
            attention_weight: 0.25,
            reciprocity_weight: 0.25,
        });
        assert_eq!(equation.calculate("a", "b", 1.0, 0.0, 0.0), 0.5);
        assert_eq!(equation.calculate("a", "b", 2.0, 2.0, 2.0), 1.0);
        assert_eq!(equation.calculate("a", "b", -1.0, -1.0, -1.0), 0.0);
    }

    #[test]
    fn snapshot_restore_preserves_relationship_timestamp() {
        let mut equation = LoveEquation::new();
        equation.record_relationship("varda", "manwe", 0.8);
        let snapshot = equation.snapshot(10);
        let timestamp = snapshot["relationships"][0]["timestamp"].clone();

        let mut restored = LoveEquation::new();
        restored.restore_from_snapshot(&snapshot);
        let restored_snapshot = restored.snapshot(10);

        assert_eq!(
            restored_snapshot["relationships"][0]["timestamp"],
            timestamp
        );
        assert!(restored.get_relationship("manwe", "varda").is_some());
    }
}
