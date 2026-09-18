//! Completion and its cross-store effects share a single durable SQL decision.
use super::*;
use crate::objectives::snapshot_protocol::Lease;

impl ObjectiveStore {
    pub(crate) fn commit_recovery_completion(
        &self,
        operator: &str,
        event: &str,
        lease: &Lease,
        prepare: impl FnOnce(
            &Transaction<'_>,
            &RecoveryGrant,
        ) -> Result<(RecoveryPublication, RunStore)>,
    ) -> Result<()> {
        let guard = self.with_retained_recovery(operator, event, lease, |tx, grant, _| {
            let (publication, guard) = prepare(tx, grant)?;
            anyhow::ensure!(
                publication.kind == "completion"
                    && publication.key == "completion"
                    && publication.node_id == grant.bindings.close_node_id
                    && guard.run_id() == &grant.bindings.run_id,
                "completion publication target changed"
            );
            let receipts: Vec<StageReceipt> =
                serde_json::from_value(publication.payload["receipts"].clone())?;
            use crate::objectives::ReceiptStage;
            anyhow::ensure!(
                receipts.iter().map(|r| r.stage).eq([
                    ReceiptStage::Execute,
                    ReceiptStage::Verify,
                    ReceiptStage::Review,
                    ReceiptStage::Close
                ]) && receipts[0].digest == grant.bindings.execute_receipt_digest,
                "completion publication chain changed"
            );
            let pending: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM recovery_publications
                WHERE authenticated_event_id=?1 AND applied_at_ms IS NULL
                AND NOT EXISTS(SELECT 1 FROM recovery_completion_suppressions s
                    WHERE s.authenticated_event_id=recovery_publications.authenticated_event_id
                      AND s.publication_key=recovery_publications.publication_key))",
                [event],
                |row| row.get(0),
            )?;
            anyhow::ensure!(!pending, "earlier recovery publication is pending");
            self.insert_recovery_publication_in(tx, grant, lease, &publication)?;
            for receipt in &receipts {
                Self::record_stage_receipt_in(
                    tx,
                    &grant.bindings.leaf_id,
                    &lease.owner,
                    lease.generation,
                    receipt,
                    Utc::now().timestamp_millis(),
                )?;
            }
            Ok(guard)
        })?;
        drop(guard);
        self.notify_change();
        Ok(())
    }

    /// Historical SQL completion authorizes only its exact remaining projections.
    pub(crate) fn reconcile_recovery_completion(
        &self,
        root: &std::path::Path,
        operator: &str,
        event: &str,
        apply: impl FnOnce(&RecoveryPublication) -> Result<()>,
    ) -> Result<()> {
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let grant = read_admission(&tx, operator, event)?.context("recovery admission missing")?;
        let suppressed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM recovery_completion_suppressions WHERE authenticated_event_id=?1)", [event], |r|r.get(0))?;
        anyhow::ensure!(
            !suppressed,
            "completion effects were durably suppressed; cleanup only"
        );
        let row: Option<(String, String, String, i64, String, i64, String)> = tx
            .query_row(
                "SELECT payload_json,payload_digest,grant_digest,lease_generation,lease_owner,lease_expires_ms,node_id FROM recovery_publications
             WHERE authenticated_event_id=?1 AND publication_key='completion' AND kind='completion'
             AND applied_at_ms IS NULL",
                [event],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .optional()?;
        // Hold the journal cancellation fence through SQL acknowledgement.
        let mut journal_guard = None;
        if let Some((payload, payload_digest, grant_digest, generation, owner, expiry, node)) = row
        {
            anyhow::ensure!(
                node == grant.bindings.close_node_id.as_str(),
                "completion publication node changed"
            );
            let publication = RecoveryPublication {
                key: "completion".into(),
                kind: "completion".into(),
                node_id: grant.bindings.close_node_id.clone(),
                payload: serde_json::from_str(&payload)?,
            };
            anyhow::ensure!(
                digest(&publication)? == payload_digest && digest(&grant)? == grant_digest,
                "completion publication was altered"
            );
            let complete: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM leaves WHERE id=?1
                AND objective_id=?2 AND execution_run_id=?3 AND stage='complete')",
                params![
                    grant.bindings.leaf_id,
                    grant.bindings.objective_id,
                    grant.bindings.run_id.as_str()
                ],
                |r| r.get(0),
            )?;
            anyhow::ensure!(complete, "completion intent has no committed stage chain");
            let current: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM leaves l
                 JOIN objectives o ON o.id=l.objective_id
                 JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
                 JOIN retained_snapshot_lease_intents i ON i.leaf_id=l.id AND i.generation=l.attempt
                 WHERE l.id=?1 AND l.execution_run_id=?2 AND l.attempt=?3
                 AND s.committed_generation=?3 AND i.lease_owner=?4 AND i.lease_expires_ms=?5
                 AND i.recovery_event_id=?6 AND l.lease_owner IS NULL AND l.lease_expires_ms IS NULL
                 AND o.operator_id=?7 AND o.state='paused' AND o.revision=?8
                 AND o.approved_revision=o.revision AND o.stop_generation=?9)",
                params![grant.bindings.leaf_id, grant.bindings.run_id.as_str(), generation,
                    owner, expiry, event, operator, grant.bindings.objective_revision,
                    i64::try_from(grant.bindings.stop_generation)?],
                |row| row.get(0),
            )?;
            anyhow::ensure!(
                current,
                "completion projection control or lease was superseded"
            );
            let receipts: Vec<StageReceipt> =
                serde_json::from_value(publication.payload["receipts"].clone())?;
            anyhow::ensure!(receipts.len() == 4, "completion chain is incomplete");
            for receipt in &receipts {
                let matches:bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM stage_receipts
                    WHERE leaf_id=?1 AND stage=?2 AND digest=?3 AND run_path=?4 AND binding_digest=?5)",
                    params![grant.bindings.leaf_id,receipt.stage.as_str(),receipt.digest,receipt.run_path,receipt.binding_digest],|r|r.get(0))?;
                anyhow::ensure!(matches, "completion stage evidence changed");
            }
            journal_guard = Some(
                RunStore::open(root, grant.bindings.run_id.clone())?
                    .lock_recovery_reconciliation(&grant, &grant.bindings.close_node_id)?,
            );
            apply(&publication)?;
            tx.execute(
                "UPDATE recovery_publications SET applied_at_ms=?2
                WHERE authenticated_event_id=?1 AND publication_key='completion'",
                params![event, Utc::now().timestamp_millis()],
            )?;
        }
        tx.commit()?;
        drop(journal_guard);
        Ok(())
    }
}
