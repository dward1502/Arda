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
                WHERE authenticated_event_id=?1 AND applied_at_ms IS NULL)",
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
        operator: &str,
        event: &str,
        apply: impl FnOnce(&RecoveryPublication) -> Result<()>,
    ) -> Result<()> {
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let grant = read_admission(&tx, operator, event)?.context("recovery admission missing")?;
        let row: Option<(String, String, String)> = tx
            .query_row(
                "SELECT payload_json,payload_digest,grant_digest FROM recovery_publications
             WHERE authenticated_event_id=?1 AND publication_key='completion' AND kind='completion'
             AND applied_at_ms IS NULL",
                [event],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((payload, payload_digest, grant_digest)) = row {
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
            let receipts: Vec<StageReceipt> =
                serde_json::from_value(publication.payload["receipts"].clone())?;
            anyhow::ensure!(receipts.len() == 4, "completion chain is incomplete");
            for receipt in &receipts {
                let matches:bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM stage_receipts
                    WHERE leaf_id=?1 AND stage=?2 AND digest=?3 AND run_path=?4 AND binding_digest=?5)",
                    params![grant.bindings.leaf_id,receipt.stage.as_str(),receipt.digest,receipt.run_path,receipt.binding_digest],|r|r.get(0))?;
                anyhow::ensure!(matches, "completion stage evidence changed");
            }
            apply(&publication)?;
            tx.execute(
                "UPDATE recovery_publications SET applied_at_ms=?2
                WHERE authenticated_event_id=?1 AND publication_key='completion'",
                params![event, Utc::now().timestamp_millis()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
