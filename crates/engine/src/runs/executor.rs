use super::{AppendOutcome, RunEventDraft, RunStore, RunStoreError};
use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunGraphError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionOutcome {
    Applied { sequence: u64 },
    AlreadyApplied { sequence: u64 },
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
    let recovered = store.recover()?;
    if let Some(sequence) = recovered.applied_idempotency_keys.get(&idempotency_key) {
        return Ok(TransitionOutcome::AlreadyApplied {
            sequence: *sequence,
        });
    }

    graph
        .transition_node(node_id, next)
        .map_err(RunStoreError::Graph)?;
    let outcome = store.append(RunEventDraft {
        node_id: node_id.clone(),
        idempotency_key,
        kind: super::RunEventKind::NodeTransition { state: next },
        receipt_digest,
    })?;
    store.write_checkpoint(graph)?;
    Ok(match outcome {
        AppendOutcome::Appended { sequence } => TransitionOutcome::Applied { sequence },
        AppendOutcome::AlreadyApplied { sequence } => {
            TransitionOutcome::AlreadyApplied { sequence }
        }
    })
}

impl From<RunGraphError> for RunStoreError {
    fn from(value: RunGraphError) -> Self {
        Self::Graph(value)
    }
}
