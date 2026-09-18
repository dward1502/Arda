//! Canonical run-side evidence for recovery. This does not authenticate an
//! operator or validate the objective store, captured capability or Vairë policy.
use super::{RecoveryBindings, RunEvent, RunEventKind, RunStore};
use crate::adapters::{HermesExecutionReceipt, HermesReceiptStatus};
use anyhow::{ensure, Context, Result};
use arda_core::run_graph::{AuthorityClass, NodeId, NodeKind, NodeState, RunGraph};
use sha2::{Digest, Sha256};
#[cfg(test)]
pub(crate) mod tests;

impl RunStore {
    /// Read the canonical checkpoint, journal and preserved Execute receipt.
    /// Call under the objective writer fence; this is evidence, not admission.
    pub fn validate_recovery_run_evidence(&self, bindings: &RecoveryBindings) -> Result<RunGraph> {
        ensure!(
            self.run_id() == &bindings.run_id,
            "recovery run identity differs"
        );
        let recovered = self.recover()?;
        let graph = recovered
            .checkpoint
            .context("recovery has no canonical checkpoint")?;
        let value = self
            .read_execution_receipt(&NodeId::new("execute")?)?
            .context("recovery has no canonical execution receipt")?;
        let receipt: HermesExecutionReceipt = serde_json::from_value(value)?;
        validate_history(&graph, &recovered.events, &receipt, bindings)?;
        Ok(graph)
    }
}

fn validate_history(
    graph: &RunGraph,
    events: &[RunEvent],
    execute_receipt: &HermesExecutionReceipt,
    bindings: &RecoveryBindings,
) -> Result<()> {
    ensure!(
        !events.iter().any(|e| matches!(
            e.kind,
            RunEventKind::NodeTransition {
                state: NodeState::Cancelled
            } | RunEventKind::Cancelled { .. }
        )),
        "cancelled run cannot recover"
    );
    validate_historical_lineage(graph, events, execute_receipt, bindings)
}

/// Immutable evidence only; confers no execution or success-projection authority.
pub(super) fn validate_historical_lineage(
    graph: &RunGraph,
    events: &[RunEvent],
    execute_receipt: &HermesExecutionReceipt,
    bindings: &RecoveryBindings,
) -> Result<()> {
    graph.validate()?;
    ensure!(
        graph.run_id == bindings.run_id
            && graph.objective_id.as_str() == bindings.leaf_id
            && graph.provenance.project_contract_digest == bindings.project_contract_digest,
        "recovery graph authority differs"
    );
    let node = |id: &str| {
        graph
            .nodes
            .iter()
            .find(|n| n.id.as_str() == id)
            .with_context(|| format!("recovery graph lacks {id}"))
    };
    let approval = node("approval")?;
    let execute = node("execute")?;
    let verify = node(bindings.verify_node_id.as_str())?;
    let review = node(bindings.review_node_id.as_str())?;
    let close = node(bindings.close_node_id.as_str())?;
    let review_ceiling = *bindings
        .provider_start_ceilings
        .get(review.id.as_str())
        .context("recovery lacks review ceiling")?;
    ensure!(
        bindings.provider_start_ceilings.len() == 2
            && bindings
                .provider_start_ceilings
                .get(verify.id.as_str())
                .copied()
                == bindings.prior_verify_starts.checked_add(1),
        "recovery start ceilings differ"
    );
    ensure!(
        approval.kind == NodeKind::Approval
            && approval.authority == AuthorityClass::HumanApproval
            && approval.state == NodeState::Succeeded
            && approval.output_digest.is_some(),
        "recovery requires successful approval"
    );
    ensure!(
        execute.kind == NodeKind::Inspect
            && execute.authority == AuthorityClass::ReadOnly
            && execute.state == NodeState::Succeeded
            && execute.output_digest.as_deref() == Some(bindings.execute_receipt_digest.as_str()),
        "recovery must preserve successful read-only execution"
    );
    ensure!(
        verify.kind == NodeKind::Verify
            && verify.authority == AuthorityClass::Verify
            && u64::from(verify.retry.max_attempts) == bindings.prior_verify_starts
            && review.kind == NodeKind::Review
            && review.authority == AuthorityClass::ReadOnly
            && u64::from(review.retry.max_attempts) == review_ceiling
            && close.kind == NodeKind::Close
            && close.authority == AuthorityClass::ReadOnly,
        "recovery stage kinds, authority or original ceilings differ"
    );
    ensure!(
        execute_receipt.has_valid_digest()?
            && execute_receipt.status == HermesReceiptStatus::Succeeded
            && execute_receipt.usage.completed
            && !execute_receipt.usage.failed
            && execute_receipt.receipt_digest == bindings.execute_receipt_digest
            && execute_receipt.run_id == graph.run_id.as_str()
            && execute_receipt.node_id == execute.id.as_str()
            && execute_receipt.idempotency_key == execute.idempotency_key
            && execute_receipt.project_contract_digest == bindings.project_contract_digest
            && execute_receipt.parent_receipts == execute.parent_receipts
            && execute
                .parent_receipts
                .contains(approval.output_digest.as_ref().unwrap()),
        "canonical execution receipt or approval lineage differs"
    );

    let failure = events
        .iter()
        .find(|e| e.sequence == bindings.failed_event_sequence)
        .context("bound verification failure is missing")?;
    ensure!(
        failure.run_id == graph.run_id
            && failure.node_id == verify.id
            && matches!(
                failure.kind,
                RunEventKind::NodeTransition {
                    state: NodeState::Failed
                }
            )
            && failure.idempotency_key
                == format!(
                    "{}:provider-error:{}",
                    verify.idempotency_key, bindings.prior_verify_starts
                )
            && format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(failure)?))
                == bindings.failed_event_digest,
        "bound verification failure differs"
    );
    let running = |event: &&RunEvent| {
        matches!(
            event.kind,
            RunEventKind::NodeTransition {
                state: NodeState::Running
            }
        )
    };
    ensure!(
        !events.iter().any(|event| event.sequence < failure.sequence
            && ((event.node_id == verify.id
                && matches!(
                    event.kind,
                    RunEventKind::NodeTransition {
                        state: NodeState::Succeeded
                    }
                ))
                || ([&review.id, &close.id].contains(&&event.node_id)
                    && matches!(
                        event.kind,
                        RunEventKind::NodeTransition {
                            state: NodeState::Running | NodeState::Succeeded
                        }
                    )))),
        "recovery history already advanced beyond unfinished verification"
    );
    for start in events.iter().filter(running) {
        ensure!(
            !events.iter().any(|prior| prior.node_id == start.node_id
                && prior.sequence < start.sequence
                && matches!(
                    prior.kind,
                    RunEventKind::NodeTransition {
                        state: NodeState::Succeeded
                    }
                )),
            "recovery history restarts an already successful node"
        );
    }
    let verify_starts: Vec<_> = events
        .iter()
        .filter(|e| e.node_id == verify.id)
        .filter(running)
        .collect();
    ensure!(
        verify_starts
            .iter()
            .filter(|e| e.sequence < failure.sequence)
            .count() as u64
            == bindings.prior_verify_starts,
        "historical verification start count differs"
    );
    ensure!(
        verify_starts.len() as u64
            <= bindings
                .prior_verify_starts
                .checked_add(1)
                .context("verification ceiling overflow")?,
        "recovery verification ceiling exceeded"
    );
    ensure!(
        events
            .iter()
            .filter(|e| e.node_id == review.id)
            .filter(running)
            .count() as u64
            <= review_ceiling,
        "review ceiling exceeded"
    );
    ensure!(
        events
            .iter()
            .filter(|e| e.node_id == execute.id)
            .filter(running)
            .count()
            == 1
            && events.iter().any(|e| e.node_id == execute.id
                && e.sequence < failure.sequence
                && matches!(
                    e.kind,
                    RunEventKind::NodeTransition {
                        state: NodeState::Succeeded
                    }
                )
                && e.receipt_digest.as_deref() == Some(bindings.execute_receipt_digest.as_str())),
        "execution journal does not preserve its single successful start"
    );
    for start in verify_starts
        .iter()
        .filter(|e| e.sequence > failure.sequence)
    {
        ensure!(events.iter().any(|event| event.sequence > failure.sequence && event.sequence < start.sequence
            && matches!(&event.kind, RunEventKind::RecoveryActivated { grant } if &grant.bindings == bindings)),
            "additional verification start lacks its bound activation");
    }
    Ok(())
}
