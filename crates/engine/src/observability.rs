//! Higher-level observability surface for `arda-engine`.
//!
//! Aggregates `arda-core` loop observability and learning interop
//! primitives so downstream consumers only need to depend on
//! `arda-engine` instead of reaching into `arda-core` directly.

use arda_core::learning::LearningStore;
use arda_core::learning_adapter::build_learning_ledger_receipt;
use arda_core::loop_observability::{LatencyProbe, LoopObservabilityConfig};
use serde::Deserialize;
use serde::Serialize;

/// Aggregated observability status for the Arda engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineObservabilityStatus {
    pub loop_observability: LoopObservabilityConfig,
    pub learning: LearningReceiptStatus,
}

/// Learning receipt status from the current learning store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningReceiptStatus {
    pub schema_version: String,
    pub domain: String,
    pub consumer: String,
    pub retained_count: usize,
    pub ignored_count: usize,
    pub learning_path: String,
}

impl EngineObservabilityStatus {
    pub fn from_env_and_store(
        learning_path: impl AsRef<std::path::Path>,
        domain: impl AsRef<str>,
        consumer: impl AsRef<str>,
        min_observations: u64,
    ) -> Self {
        let loop_observability = LoopObservabilityConfig::from_env();
        let store = LearningStore::new(learning_path.as_ref());
        let learning = store.load();
        let receipt = build_learning_ledger_receipt(
            &learning,
            domain.as_ref(),
            consumer.as_ref(),
            min_observations,
        );

        Self {
            loop_observability,
            learning: LearningReceiptStatus {
                schema_version: receipt.schema_version.clone(),
                domain: receipt.domain,
                consumer: receipt.consumer,
                retained_count: receipt.retained.len(),
                ignored_count: receipt.ignored_count,
                learning_path: learning_path.as_ref().display().to_string(),
            },
        }
    }
}

impl Default for EngineObservabilityStatus {
    fn default() -> Self {
        Self {
            loop_observability: LoopObservabilityConfig::default(),
            learning: LearningReceiptStatus {
                schema_version: build_learning_ledger_receipt(
                    &arda_core::learning::LearningState::default(),
                    "",
                    "",
                    0,
                )
                .schema_version,
                domain: String::new(),
                consumer: String::new(),
                retained_count: 0,
                ignored_count: 0,
                learning_path: String::new(),
            },
        }
    }
}

/// Initialize a dummy latency probe for tick-level observability.
pub fn tick_probe(max_samples: usize) -> LatencyProbe {
    LatencyProbe::new(max_samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_is_conservative() {
        let status = EngineObservabilityStatus::default();
        assert!(!status.loop_observability.economy_snapshot_enabled);
        assert!(!status.loop_observability.latency_probe_enabled);
    }

    #[test]
    fn from_env_and_store_reads_default_learning_path() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("learn.json");
        LearningStore::new(&store_path)
            .save(&arda_core::learning::LearningState::default())
            .unwrap();

        let status =
            EngineObservabilityStatus::from_env_and_store(&store_path, "governance", "test", 1);
        assert!(status.learning.learning_path.ends_with("learn.json"));
        assert_eq!(status.learning.retained_count, 0);
    }
}
