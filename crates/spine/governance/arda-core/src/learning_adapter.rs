//! GEN3 learning/memory adaptation helpers.
//!
//! This module exposes `LearningState` concepts as additive,
//! serializable adaptation records so external memory surfaces can
//! consume Arda governance learning without changing append-only
//! auditability in `arda-core`.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub use crate::learning::{LearningState, OutcomeStats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDomainAdaptation {
    pub domain: String,
    pub consumer: String,
    pub retained: Vec<DomainRetainedInsight>,
    pub ignored: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRetainedInsight {
    pub agent: String,
    pub task_type: String,
    pub success_rate: f64,
    pub observations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningLedgerReceipt {
    pub schema_version: String,
    pub domain: String,
    pub consumer: String,
    pub retained: Vec<DomainRetainedInsight>,
    pub ignored_count: usize,
}

impl LearningLedgerReceipt {
    pub const SCHEMA_VERSION: &'static str = "arda.learning.memory_adaptation.v1";
}

fn split_key(key: &str) -> Option<(&str, &str)> {
    key.split_once("::")
}

pub fn adapt_learning_to_domain(
    learning: &LearningState,
    domain: &str,
    consumer: &str,
    min_observations: u64,
) -> MemoryDomainAdaptation {
    let mut retained = Vec::new();
    let mut ignored = Vec::new();

    for (key, stats) in &learning.stats {
        if stats.attempts < min_observations {
            ignored.push(key.clone());
            continue;
        }

        let Some((agent, task_type)) = split_key(key) else {
            ignored.push(key.clone());
            continue;
        };

        retained.push(DomainRetainedInsight {
            agent: agent.to_string(),
            task_type: task_type.to_string(),
            success_rate: stats.success_rate(),
            observations: stats.attempts,
        });
    }

    retained.sort_by(|a, b| match b.success_rate.partial_cmp(&a.success_rate) {
        Some(ord) => ord,
        None => Ordering::Equal,
    });

    MemoryDomainAdaptation {
        domain: domain.to_string(),
        consumer: consumer.to_string(),
        retained,
        ignored,
    }
}

pub fn build_learning_ledger_receipt(
    learning: &LearningState,
    domain: &str,
    consumer: &str,
    min_observations: u64,
) -> LearningLedgerReceipt {
    let adaptation = adapt_learning_to_domain(learning, domain, consumer, min_observations);

    LearningLedgerReceipt {
        schema_version: LearningLedgerReceipt::SCHEMA_VERSION.to_string(),
        domain: adaptation.domain.clone(),
        consumer: adaptation.consumer.clone(),
        retained: adaptation.retained.clone(),
        ignored_count: adaptation.ignored.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapt_learning_to_domain_filters_and_ranks_by_success_rate() {
        let mut learning = LearningState::default();
        learning.observe("a", "ops", true, 1.0, 1.0);
        learning.observe("a", "ops", true, 1.0, 1.0);
        learning.observe("a", "ops", true, 1.0, 1.0);
        learning.observe("b", "ops", true, 1.0, 1.0);
        learning.observe("c", "ops", false, 1.0, 1.0);

        let adapted = adapt_learning_to_domain(&learning, "governance", "test-consumer", 2);
        assert_eq!(adapted.retained.len(), 1);
        assert_eq!(adapted.retained[0].agent, "a");
        assert_eq!(adapted.retained[0].success_rate, 1.0);
        assert_eq!(adapted.ignored.len(), 2);
    }

    #[test]
    fn adapt_learning_to_domain_survives_malformed_key() {
        let mut learning = LearningState::default();
        learning.stats.insert(
            "malformed_key".into(),
            OutcomeStats {
                attempts: 2,
                successes: 1,
                avg_duration_secs: 1.0,
                avg_joules: 1.0,
            },
        );

        let adapted = adapt_learning_to_domain(&learning, "governance", "test-consumer", 1);
        assert!(adapted.retained.is_empty());
        assert_eq!(adapted.ignored.len(), 1);
    }

    #[test]
    fn build_learning_ledger_receipt_survives_round_trip() {
        let mut learning = LearningState::default();
        learning.observe("a", "ops", true, 1.0, 1.0);
        learning.observe("a", "ops", true, 1.0, 1.0);

        let receipt = build_learning_ledger_receipt(&learning, "governance", "test-consumer", 2);
        assert_eq!(
            receipt.schema_version,
            LearningLedgerReceipt::SCHEMA_VERSION
        );
        assert_eq!(receipt.retained.len(), 1);
        let json = serde_json::to_string(&receipt).unwrap();
        let back: LearningLedgerReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.consumer, "test-consumer");
        assert_eq!(back.retained[0].agent, "a");
    }
}
