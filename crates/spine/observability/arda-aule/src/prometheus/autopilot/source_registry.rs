#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Read-only Arandur source registry contracts for universal objective discovery.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::prometheus::queue_authority::canonical_project_task_queue;

pub const SOURCE_REGISTRY_CONTRACT: &str = "arda.arandur.source_registry.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArandurSourceType {
    HumanApproval,
    ObjectiveInbox,
    ArandurRecommendation,
    CanonicalQueue,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceDescriptor {
    pub contract: String,
    pub source_type: ArandurSourceType,
    pub path: PathBuf,
    pub record_id_field: String,
    pub read_only_discovery: bool,
    pub canonical_queue_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceRegistry {
    pub contract: String,
    pub sources: Vec<SourceDescriptor>,
}

impl SourceRegistry {
    pub fn arandur_default(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let queue_path = canonical_project_task_queue(root);
        Self::arandur_with_queue(root, queue_path)
    }

    pub fn arandur_with_queue(root: impl AsRef<Path>, queue_path: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            contract: SOURCE_REGISTRY_CONTRACT.into(),
            sources: vec![
                SourceDescriptor {
                    contract: "arda.h2a.approvals.v1".into(),
                    source_type: ArandurSourceType::HumanApproval,
                    path: root.join("data/comm/h2a.jsonl"),
                    record_id_field: "objective.id".into(),
                    read_only_discovery: true,
                    canonical_queue_mutation_allowed: false,
                },
                SourceDescriptor {
                    contract: "arda.prometheus.objective_inbox.v1".into(),
                    source_type: ArandurSourceType::ObjectiveInbox,
                    path: root.join("core/projects/objectives/inbox.jsonl"),
                    record_id_field: "id".into(),
                    read_only_discovery: true,
                    canonical_queue_mutation_allowed: false,
                },
                SourceDescriptor {
                    contract: "arda.arandur.recommendations.v1".into(),
                    source_type: ArandurSourceType::ArandurRecommendation,
                    path: root.join("data/arandur/recommendations.jsonl"),
                    record_id_field: "recommendation_id".into(),
                    read_only_discovery: true,
                    canonical_queue_mutation_allowed: false,
                },
                SourceDescriptor {
                    contract: "arda.canonical_task_queue.v1".into(),
                    source_type: ArandurSourceType::CanonicalQueue,
                    path: queue_path.as_ref().to_path_buf(),
                    record_id_field: "source_record_id|id".into(),
                    read_only_discovery: true,
                    canonical_queue_mutation_allowed: false,
                },
            ],
        }
    }

    pub fn active_sources(&self) -> impl Iterator<Item = &SourceDescriptor> {
        self.sources
            .iter()
            .filter(|source| source.read_only_discovery)
    }

    pub fn by_contract(&self, contract: &str) -> Option<&SourceDescriptor> {
        self.sources
            .iter()
            .find(|source| source.contract == contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arandur_default_declares_universal_discovery_sources() {
        let registry = SourceRegistry::arandur_default("/arda");

        assert_eq!(registry.contract, SOURCE_REGISTRY_CONTRACT);
        assert_eq!(registry.active_sources().count(), 4);
        assert!(
            registry
                .by_contract("arda.canonical_task_queue.v1")
                .is_some()
        );
        assert!(
            registry
                .sources
                .iter()
                .all(|source| !source.canonical_queue_mutation_allowed)
        );
    }

    #[test]
    fn canonical_queue_source_declares_effective_record_key() {
        let registry = SourceRegistry::arandur_default("/arda");
        let queue = registry
            .by_contract("arda.canonical_task_queue.v1")
            .expect("canonical queue source should exist");

        assert_eq!(queue.record_id_field, "source_record_id|id");
    }
}
