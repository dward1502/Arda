use super::{AppendOutcome, RunEventDraft, RunStore, RunStoreError};
use arda_core::capability_composition::CapabilityComposition;
use arda_core::run_graph::{
    CapabilityCompositionReceipt, CompositionTrigger, DeterministicCompositionError, NodeId,
    NodeState, RunGraph, RunGraphError,
};
use arda_core::service_registry::{
    CapabilityRegistry, CapabilityRegistryError, CapabilityRuntimeState,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionOutcome {
    Applied { sequence: u64 },
    AlreadyApplied { sequence: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionExecutionOutcome {
    pub receipt: CapabilityCompositionReceipt,
    pub receipt_digest: String,
    pub sequence: u64,
}

pub fn compose_run_capabilities(
    store: &RunStore,
    graph: &RunGraph,
    composition: &CapabilityComposition,
    registry: &mut CapabilityRegistry,
    model_recommendations: &BTreeSet<String>,
    trigger: CompositionTrigger,
) -> Result<CompositionExecutionOutcome, CompositionExecutionError> {
    let prior = store.read_composition_receipt()?;
    match (prior.is_some(), trigger) {
        (true, CompositionTrigger::Initial) => {
            return Err(CompositionExecutionError::ReevaluationBoundaryRequired);
        }
        (false, CompositionTrigger::Initial) => {}
        (false, _) => return Err(CompositionExecutionError::InitialCompositionRequired),
        (true, _) => {}
    }
    let prior_receipt_digest = prior
        .as_ref()
        .map(CapabilityCompositionReceipt::digest)
        .transpose()?;
    let receipt = graph.deterministic_composition(
        composition,
        registry,
        model_recommendations,
        trigger,
        prior_receipt_digest,
    )?;
    if let Some(prior) = &prior {
        let boundary_observed = match trigger {
            CompositionTrigger::Initial => false,
            CompositionTrigger::HealthChanged => {
                receipt.registry_constraint_digest != prior.registry_constraint_digest
            }
            CompositionTrigger::OperatorAmendment => {
                receipt.composition_digest != prior.composition_digest
            }
            CompositionTrigger::Failure => {
                let recovered = store.recover()?;
                let last_composition_sequence = recovered
                    .events
                    .iter()
                    .rev()
                    .find(|event| {
                        matches!(
                            event.kind,
                            super::RunEventKind::CapabilityCompositionSelected { .. }
                        )
                    })
                    .map(|event| event.sequence)
                    .unwrap_or(0);
                recovered.events.iter().any(|event| {
                    event.sequence > last_composition_sequence
                        && matches!(
                            event.kind,
                            super::RunEventKind::NodeTransition {
                                state: NodeState::Failed
                            }
                        )
                })
            }
        };
        if !boundary_observed {
            return Err(CompositionExecutionError::BoundaryNotObserved(trigger));
        }
    }
    let receipt_digest = store.write_composition_receipt(&receipt)?;
    let outcome = store.append(RunEventDraft {
        node_id: NodeId::new("capability-composition")
            .expect("static composition node id is valid"),
        idempotency_key: format!("capability-composition:{receipt_digest}"),
        kind: super::RunEventKind::CapabilityCompositionSelected {
            composition_digest: receipt.composition_digest.clone(),
            trigger,
            selected_capability_count: receipt.selected_capabilities.len(),
        },
        receipt_digest: Some(receipt_digest.clone()),
    })?;
    let sequence = match outcome {
        AppendOutcome::Appended { sequence } | AppendOutcome::AlreadyApplied { sequence } => {
            sequence
        }
    };

    let selected = receipt
        .selected_capabilities
        .iter()
        .map(|capability| (capability.id.clone(), capability.version.clone()))
        .collect::<BTreeSet<_>>();
    let updates = registry
        .records()
        .map(|record| {
            let mut runtime = record.runtime;
            runtime.selected = selected.contains(&(
                record.declaration.id.clone(),
                record.declaration.version.clone(),
            ));
            (
                record.declaration.id.clone(),
                record.declaration.version.clone(),
                runtime,
            )
        })
        .collect::<Vec<(String, String, CapabilityRuntimeState)>>();
    for (id, version, runtime) in updates {
        registry.set_runtime_state(&id, &version, runtime)?;
    }

    Ok(CompositionExecutionOutcome {
        receipt,
        receipt_digest,
        sequence,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CompositionExecutionError {
    #[error(
        "initial capability composition already exists; an explicit re-evaluation boundary is required"
    )]
    ReevaluationBoundaryRequired,
    #[error("an initial capability composition is required before re-evaluation")]
    InitialCompositionRequired,
    #[error("composition re-evaluation boundary was not observed: {0:?}")]
    BoundaryNotObserved(CompositionTrigger),
    #[error("deterministic capability composition failed: {0}")]
    Deterministic(#[from] DeterministicCompositionError),
    #[error("run persistence failed: {0}")]
    Store(#[from] RunStoreError),
    #[error("capability registry update failed: {0}")]
    Registry(#[from] CapabilityRegistryError),
}

pub fn apply_transition_once(
    store: &RunStore,
    graph: &mut RunGraph,
    node_id: &NodeId,
    next: NodeState,
    idempotency_key: impl Into<String>,
    receipt_digest: Option<String>,
) -> Result<TransitionOutcome, RunStoreError> {
    let idempotency_key = idempotency_key.into();
    let draft = RunEventDraft {
        node_id: node_id.clone(),
        idempotency_key: idempotency_key.clone(),
        kind: super::RunEventKind::NodeTransition { state: next },
        receipt_digest: receipt_digest.clone(),
    };
    let recovered = store.recover()?;
    if recovered
        .applied_idempotency_keys
        .contains_key(&idempotency_key)
    {
        let outcome = store.append(draft)?;
        let current = graph
            .nodes
            .iter()
            .find(|node| node.id == *node_id)
            .map(|node| node.state)
            .ok_or_else(|| RunStoreError::Graph(RunGraphError::MissingNode(node_id.clone())))?;
        if current != next {
            graph
                .transition_node(node_id, next)
                .map_err(RunStoreError::Graph)?;
        }
        let sequence = match outcome {
            AppendOutcome::AlreadyApplied { sequence } => sequence,
            AppendOutcome::Appended { .. } => unreachable!("recovered key must already exist"),
        };
        if next == NodeState::Succeeded {
            project_succeeded_receipt(graph, node_id, sequence, receipt_digest.as_deref());
        }
        store.write_checkpoint(graph)?;
        return Ok(TransitionOutcome::AlreadyApplied { sequence });
    }

    let mut projected = graph.clone();
    projected
        .transition_node(node_id, next)
        .map_err(RunStoreError::Graph)?;
    let outcome = store.append(draft)?;
    *graph = projected;
    let sequence = match outcome {
        AppendOutcome::Appended { sequence } | AppendOutcome::AlreadyApplied { sequence } => {
            sequence
        }
    };
    if next == NodeState::Succeeded {
        project_succeeded_receipt(graph, node_id, sequence, receipt_digest.as_deref());
    }
    store.write_checkpoint(graph)?;
    Ok(match outcome {
        AppendOutcome::Appended { sequence } => TransitionOutcome::Applied { sequence },
        AppendOutcome::AlreadyApplied { sequence } => {
            TransitionOutcome::AlreadyApplied { sequence }
        }
    })
}

fn project_succeeded_receipt(
    graph: &mut RunGraph,
    node_id: &NodeId,
    sequence: u64,
    receipt_digest: Option<&str>,
) {
    let recovery_token = format!("{}:{}:{sequence}", graph.run_id.as_str(), node_id.as_str());
    let checkpoint_digest = format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "{}\n{}\n{sequence}\n{}",
                graph.run_id.as_str(),
                node_id.as_str(),
                receipt_digest.unwrap_or_default()
            )
            .as_bytes()
        )
    );
    if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == *node_id) {
        node.output_digest = receipt_digest.map(ToOwned::to_owned);
        node.checkpoint.sequence = sequence;
        node.checkpoint.recovery_token = Some(recovery_token);
        node.checkpoint.checkpoint_digest = Some(checkpoint_digest);
    }

    let Some(receipt_digest) = receipt_digest else {
        return;
    };
    let child_ids = graph
        .edges
        .iter_mut()
        .filter(|edge| edge.from == *node_id)
        .map(|edge| {
            edge.parent_receipt = Some(receipt_digest.to_owned());
            edge.to.clone()
        })
        .collect::<Vec<_>>();
    for child_id in child_ids {
        let parent_receipts = graph
            .edges
            .iter()
            .filter(|edge| edge.to == child_id)
            .filter_map(|edge| edge.parent_receipt.clone())
            .collect();
        if let Some(child) = graph.nodes.iter_mut().find(|node| node.id == child_id) {
            child.parent_receipts = parent_receipts;
        }
    }
}

impl From<RunGraphError> for RunStoreError {
    fn from(value: RunGraphError) -> Self {
        Self::Graph(value)
    }
}
