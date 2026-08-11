//! Higher-level observability surface for `arda-engine`.
//!
//! Aggregates `arda-core` loop observability and learning interop
//! primitives so downstream consumers only need to depend on
//! `arda-engine` instead of reaching into `arda-core` directly.

use crate::runs::{RunEvent, RunEventKind};
use arda_core::learning::LearningStore;
use arda_core::learning_adapter::build_learning_ledger_receipt;
use arda_core::loop_observability::{LatencyProbe, LoopObservabilityConfig};
use arda_core::run_graph::CompositionTrigger;
use arda_governance::metrics::global_governance_metrics;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

/// Stable correlation fields carried by every durable Workbench semantic event.
/// `run_id` is the trace-equivalent lineage and the event sequence is the
/// span-equivalent position within that lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeLineage {
    pub trace_id: String,
    pub span_id: String,
    pub run_id: String,
    pub node_id: String,
    pub event_sequence: u64,
    pub receipt_digest: Option<String>,
}

impl RuntimeLineage {
    pub fn from_run_event(event: &RunEvent) -> Self {
        let run_id = event.run_id.as_str().to_string();
        let node_id = event.node_id.as_str().to_string();
        Self {
            trace_id: run_id.clone(),
            span_id: format!("{node_id}:{}", event.sequence),
            run_id,
            node_id,
            event_sequence: event.sequence,
            receipt_digest: event.receipt_digest.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCompositionObservation {
    pub run_id: String,
    pub event_sequence: u64,
    pub composition_digest: String,
    pub receipt_digest: String,
    pub trigger: CompositionTrigger,
    pub selected_capability_count: usize,
}

impl CapabilityCompositionObservation {
    pub fn from_run_event(event: &RunEvent) -> Option<Self> {
        let RunEventKind::CapabilityCompositionSelected {
            composition_digest,
            trigger,
            selected_capability_count,
        } = &event.kind
        else {
            return None;
        };
        Some(Self {
            run_id: event.run_id.as_str().to_string(),
            event_sequence: event.sequence,
            composition_digest: composition_digest.clone(),
            receipt_digest: event.receipt_digest.clone()?,
            trigger: *trigger,
            selected_capability_count: *selected_capability_count,
        })
    }
}

/// Finite Stage 5/U3 operating budgets. Values are mirrored into release
/// evidence with measured observations; this type is the machine-readable
/// runtime contract, not a source of synthetic measurements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalReliabilityBudgets {
    pub startup_latency_ms_max: u64,
    pub idle_rss_peak_mib_max: u64,
    pub ui_interaction_latency_ms_max: u64,
    pub event_projection_latency_ms_max: u64,
    pub recovery_latency_ms_max: u64,
    pub diagnostic_bundle_bytes_max: u64,
    pub protected_state_growth_files_max: u64,
    pub protected_state_growth_bytes_max: u64,
}

impl Default for OperationalReliabilityBudgets {
    fn default() -> Self {
        Self {
            startup_latency_ms_max: 2_000,
            idle_rss_peak_mib_max: 512,
            ui_interaction_latency_ms_max: 100,
            event_projection_latency_ms_max: 1_000,
            recovery_latency_ms_max: 1_000,
            diagnostic_bundle_bytes_max: 1_048_576,
            protected_state_growth_files_max: 1_000,
            protected_state_growth_bytes_max: 67_108_864,
        }
    }
}

/// Aggregated observability status for the Arda engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineObservabilityStatus {
    pub loop_observability: LoopObservabilityConfig,
    pub learning: LearningReceiptStatus,
    /// Read-only projection; `arda-governance` remains the sole counter owner.
    pub governance_counters: Vec<EngineGovernanceCounter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineGovernanceCounter {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: u64,
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
        let governance_counters = global_governance_metrics()
            .snapshot()
            .counters
            .into_iter()
            .map(|counter| EngineGovernanceCounter {
                name: counter.name,
                labels: counter.labels,
                value: counter.value,
            })
            .collect();

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
            governance_counters,
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
            governance_counters: global_governance_metrics()
                .snapshot()
                .counters
                .into_iter()
                .map(|counter| EngineGovernanceCounter {
                    name: counter.name,
                    labels: counter.labels,
                    value: counter.value,
                })
                .collect(),
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
        assert_eq!(
            serde_json::to_value(&status).unwrap()["governance_counters"],
            serde_json::json!([])
        );
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

    #[test]
    fn runtime_lineage_projects_run_event_without_minting_new_authority() {
        use crate::runs::RunEventKind;
        use arda_core::run_graph::{NodeId, RunId};

        let event = RunEvent {
            schema_version: RunEvent::SCHEMA_VERSION.to_string(),
            sequence: 7,
            run_id: RunId::new("run-u3-lineage").unwrap(),
            node_id: NodeId::new("verify").unwrap(),
            idempotency_key: "verify-u3".to_string(),
            kind: RunEventKind::ResultProjected,
            receipt_digest: Some("sha256:receipt".to_string()),
            recorded_at_unix_ms: 1,
        };

        let lineage = RuntimeLineage::from_run_event(&event);
        assert_eq!(lineage.trace_id, "run-u3-lineage");
        assert_eq!(lineage.span_id, "verify:7");
        assert_eq!(lineage.receipt_digest.as_deref(), Some("sha256:receipt"));
    }

    #[test]
    fn operational_budgets_are_finite_and_serialize_with_named_boundaries() {
        let budgets = OperationalReliabilityBudgets::default();
        let value = serde_json::to_value(&budgets).unwrap();
        assert_eq!(value["startup_latency_ms_max"], 2_000);
        assert_eq!(value["event_projection_latency_ms_max"], 1_000);
        assert_eq!(value["protected_state_growth_bytes_max"], 67_108_864);
    }
}
