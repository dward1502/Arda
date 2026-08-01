use super::{AppendOutcome, RunEventDraft, RunStore, RunStoreError};
use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunGraphError};
use sha2::{Digest, Sha256};

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
        receipt_digest: receipt_digest.clone(),
    })?;
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
