//! Server-derived admission; callers authenticate the operator before entry.
use super::*;
use crate::runs::RunStore;
use arda_core::run_graph::{NodeId, NodeState, RunId};
use sha2::{Digest, Sha256};
use std::path::Path;

impl ObjectiveStore {
    /// Read-only authority replay followed by exact-leaf keeper cleanup. This
    /// path deliberately does not renew a window or acquire an execution lease.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reconcile_saved_completed_recovery(
        &self,
        root: &Path,
        operator_id: &str,
        event_id: &str,
        payload_digest: &str,
        objective_id: &str,
        leaf_id: &str,
        run_id: &str,
        apply: impl Fn(&RunStore, &crate::objectives::RecoveryPublication) -> Result<()>,
    ) -> Result<bool> {
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(grant) = read_admission(&tx, operator_id, event_id)? else {
            return Ok(false);
        };
        anyhow::ensure!(
            grant.authenticated_payload_digest == format!("sha256:{payload_digest}")
                && grant.bindings.objective_id == objective_id
                && grant.bindings.leaf_id == leaf_id
                && grant.bindings.run_id.as_str() == run_id,
            "recovery replay target or payload changed"
        );
        let complete: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM leaves WHERE id=?1 AND objective_id=?2 AND stage='complete')",
                params![leaf_id, objective_id], |row| row.get(0))?;
        if !complete {
            tx.commit()?;
            self.reconcile_recovery_publications(root, operator_id, event_id, apply)?;
            return Ok(false);
        }
        use crate::objectives::ReceiptStage;
        let mut receipts = Vec::new();
        for stage in [
            ReceiptStage::Execute,
            ReceiptStage::Verify,
            ReceiptStage::Review,
            ReceiptStage::Close,
        ] {
            receipts.push(tx.query_row(
                "SELECT contract,digest,predecessor_digest,run_path,provider,model,started_at_ms,
                 completed_at_ms,verdict,context_outcome_receipt_id,context_outcome_receipt_digest,binding_digest
                 FROM stage_receipts WHERE leaf_id=?1 AND stage=?2",
                params![leaf_id, stage.as_str()], |row| Ok(StageReceipt {
                    stage, contract: row.get(0)?, digest: row.get(1)?, predecessor_digest: row.get(2)?,
                    run_path: row.get(3)?, provider: row.get(4)?, model: row.get(5)?,
                    started_at_ms: row.get(6)?, completed_at_ms: row.get(7)?, verdict: row.get(8)?,
                    context_outcome_receipt_id: row.get(9)?, context_outcome_receipt_digest: row.get(10)?,
                    binding_digest: row.get(11)?,
                }))?);
        }
        tx.commit()?;
        self.reconcile_recovery_completion(root, operator_id, event_id, |publication| {
            crate::objectives::RecoveryAuthorization::apply_completed_publication(root, publication)
        })?;
        self.reconcile_completed_recovery(operator_id, event_id, &receipts, |_, saved, receipts| {
            anyhow::ensure!(saved == &grant, "completed recovery grant changed");
            let run = RunStore::open(root, saved.bindings.run_id.clone())?;
            let graph = run.validate_recovery_run_evidence(&saved.bindings)?;
            anyhow::ensure!(run.recover()?.events.iter().any(|event| matches!(&event.kind,
                crate::runs::RunEventKind::RecoveryActivated { grant: activation } if activation.as_ref() == saved)),
                "completed recovery activation changed");
            let mut parent = None;
            for (stage, receipt) in ["execute", "verify", "review", "close"].into_iter().zip(receipts) {
                let value = run.read_execution_receipt(&NodeId::new(stage)?)?.context("completed stage receipt missing")?;
                let actual: crate::adapters::HermesExecutionReceipt = serde_json::from_value(value)?;
                anyhow::ensure!(actual.has_valid_digest()? && actual.receipt_digest == receipt.digest
                    && actual.run_id == run_id && actual.node_id == stage
                    && actual.project_contract_digest == saved.bindings.project_contract_digest
                    && actual.status == crate::adapters::HermesReceiptStatus::Succeeded,
                    "completed stage receipt changed");
                if let Some(parent) = parent {
                    anyhow::ensure!(actual.parent_receipts == vec![parent], "completed stage parent changed");
                }
                anyhow::ensure!(graph.nodes.iter().any(|node| node.id.as_str() == stage
                    && node.state == NodeState::Succeeded && node.output_digest.as_deref() == Some(&receipt.digest)),
                    "completed stage graph changed");
                parent = Some(receipt.digest.clone());
            }
            Ok(())
        })
    }

    /// Resolve an immutable saved admission first. A replay cannot provide a
    /// replacement clock, budget, context, failure or retained capability.
    #[allow(clippy::too_many_arguments)] // Keep authenticated and canonical identity components explicit.
    pub fn prepare_recovery_admission(
        &self,
        root: &std::path::Path,
        operator_id: &str,
        event_id: &str,
        payload_digest: &str,
        objective_id: &str,
        leaf_id: &str,
        run_id: &str,
    ) -> Result<RecoveryGrant> {
        anyhow::ensure!(
            payload_digest.len() == 64
                && payload_digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "authenticated payload digest must be bare lowercase SHA-256"
        );
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(saved) = read_admission(&tx, operator_id, event_id)? {
            anyhow::ensure!(
                saved.bindings.objective_id == objective_id
                    && saved.bindings.leaf_id == leaf_id
                    && saved.bindings.run_id.as_str() == run_id
                    && saved.authenticated_payload_digest == format!("sha256:{payload_digest}"),
                "saved recovery admission differs from authenticated command"
            );
            tx.commit()?;
            return Ok(saved);
        }
        let (revision, generation, contract, context_json, snapshot_json):
            (i64, i64, String, String, String) = tx.query_row(
            "SELECT o.revision,o.stop_generation,p.contract_digest,c.assembly_json,s.capability_json
             FROM objectives o JOIN leaves l ON l.objective_id=o.id
             JOIN objective_projects p ON p.objective_id=o.id AND p.project_id=l.project_id
             JOIN resident_context_bindings c ON c.run_id=l.execution_run_id
             JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
             WHERE o.id=?1 AND o.operator_id=?2 AND o.state='paused'
               AND o.approved_revision=o.revision AND l.id=?3 AND l.execution_run_id=?4
               AND l.authority='read_only' AND c.deleted_by_operator_ms IS NULL",
            params![objective_id, operator_id, leaf_id, run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;
        let context: arda_vaire::ContextAssembly = serde_json::from_str(&context_json)?;
        let snapshot: crate::objectives::RetainedSnapshot = serde_json::from_str(&snapshot_json)?;
        let run_id = RunId::new(run_id)?;
        let run_store = crate::runs::RunStore::open(root, run_id.clone())?;
        let recovered = run_store.recover()?;
        let graph = recovered
            .checkpoint
            .context("missing retained checkpoint")?;
        let verify = graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "verify")
            .context("missing retained Verify")?;
        let review = graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "review")
            .context("missing retained Review")?;
        anyhow::ensure!(verify.state == NodeState::Failed, "Verify is not failed");
        let prior = recovered
            .events
            .iter()
            .filter(|event| {
                event.node_id == verify.id
                    && matches!(
                        event.kind,
                        crate::runs::RunEventKind::NodeTransition {
                            state: NodeState::Running
                        }
                    )
            })
            .count() as u64;
        anyhow::ensure!(
            prior == u64::from(verify.retry.max_attempts),
            "Verify budget is not exhausted"
        );
        let failed = recovered
            .events
            .iter()
            .rev()
            .find(|event| {
                event.node_id == verify.id
                    && matches!(
                        event.kind,
                        crate::runs::RunEventKind::NodeTransition {
                            state: NodeState::Failed
                        }
                    )
            })
            .context("missing retained Verify failure")?;
        let execute = graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "execute")
            .and_then(|node| node.output_digest.clone())
            .context("missing preserved Execute receipt")?;
        let now = u64::try_from(chrono::Utc::now().timestamp_millis())?;
        let grant = RecoveryGrant {
            authenticated_event_id: event_id.to_owned(),
            authenticated_payload_digest: format!("sha256:{payload_digest}"),
            activated_at_unix_ms: now,
            expires_at_unix_ms: now
                .checked_add(crate::runs::RECOVERY_WINDOW_MS)
                .context("recovery deadline overflow")?,
            bindings: crate::runs::RecoveryBindings {
                objective_id: objective_id.to_owned(),
                objective_revision: revision,
                stop_generation: u64::try_from(generation)?,
                leaf_id: leaf_id.to_owned(),
                run_id,
                failed_event_sequence: failed.sequence,
                failed_event_digest: format!(
                    "sha256:{:x}",
                    Sha256::digest(serde_json::to_vec(failed)?)
                ),
                execute_receipt_digest: execute,
                project_contract_digest: contract,
                context_capsule_digest: context.capsule.capsule_digest,
                context_use_receipt_digest: context.use_receipt.receipt_digest,
                retained_authority_digest: format!(
                    "sha256:{:x}",
                    Sha256::digest(serde_json::to_vec(&snapshot)?)
                ),
                verify_node_id: verify.id.clone(),
                review_node_id: review.id.clone(),
                close_node_id: NodeId::new("close")?,
                prior_verify_starts: prior,
                provider_start_ceilings: std::collections::BTreeMap::from([
                    (
                        "verify".to_owned(),
                        prior.checked_add(1).context("Verify ceiling overflow")?,
                    ),
                    ("review".to_owned(), u64::from(review.retry.max_attempts)),
                ]),
            },
        };
        check_gateway_binding(&tx, &grant)?;
        self.validate_retained_recovery_material(root, &tx, &grant)?;
        tx.commit()?;
        self.record_recovery_admission_intent(operator_id, &grant, |tx, saved| {
            self.validate_retained_recovery_material(root, tx, saved)
                .map(|_| ())
        })
    }
}
