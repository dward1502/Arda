//! Canonical payload validation and synchronous, journal-fenced provider launch.
use super::*;

impl RecoveryAuthorization {
    pub fn validate_provider_request(
        &self,
        node_id: &str,
        request: &serde_json::Value,
    ) -> Result<()> {
        self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, retained| {
                self.validate_provider_request_in(tx, grant, retained, node_id, request)
            },
        )
    }

    /// Invoke only for Chat, with the adapter's exact captured binding. The
    /// transport owns the sent marker and must clean up a sent+error outcome.
    /// Neither validation nor an AlreadyApplied result permits a second send.
    pub fn launch_provider_request<T>(
        &self,
        request: &serde_json::Value,
        retained: &crate::objectives::RetainedExecution,
        run_store: &crate::runs::RunStore,
        draft: crate::runs::RunEventDraft,
        send: impl FnOnce() -> Result<T>,
    ) -> Result<crate::runs::AppendOutcome> {
        anyhow::ensure!(
            run_store.run_id().as_str() == self.lease.run_id,
            "recovery launch journal differs from claimed run"
        );
        self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, current| {
                // Derive the actual launch journal from the same canonical root
                // as material validation. A same-ID copied journal is not the
                // authority that owns this run's spent starts/cancellation lock.
                let canonical_store =
                    crate::runs::RunStore::open(&self.root, grant.bindings.run_id.clone())?;
                anyhow::ensure!(
                    run_store
                        .events_path()
                        .parent()
                        .context("launch journal directory missing")?
                        .canonicalize()?
                        == canonical_store
                            .events_path()
                            .parent()
                            .context("canonical journal directory missing")?
                            .canonicalize()?,
                    "recovery launch journal is not the canonical run journal"
                );
                anyhow::ensure!(
                    retained.snapshot == current.snapshot && retained.lease == current.lease,
                    "recovery transport differs from current captured binding"
                );
                self.validate_provider_request_in(
                    tx,
                    grant,
                    current,
                    draft.node_id.as_str(),
                    request,
                )?;
                let (outcome, sent) = canonical_store.with_recovery_start_before(
                    grant,
                    draft,
                    u64::try_from(current.lease.expires_ms)?,
                    send,
                )?;
                // An error after sending must propagate through the outer
                // SQLite fence without erasing the transport's sent marker.
                sent.transpose()?;
                Ok(outcome)
            },
        )
    }

    fn validate_provider_request_in(
        &self,
        tx: &rusqlite::Transaction<'_>,
        grant: &RecoveryGrant,
        retained: &crate::objectives::RetainedExecution,
        node_id: &str,
        request: &serde_json::Value,
    ) -> Result<()> {
        let material = self
            .store
            .validate_retained_recovery_material(&self.root, tx, grant)?;
        anyhow::ensure!(
            material.snapshot == retained.snapshot,
            "recovery material differs from the fenced snapshot"
        );
        anyhow::ensure!(
            (node_id == "verify" && node_id == grant.bindings.verify_node_id.as_str())
                || (node_id == "review" && node_id == grant.bindings.review_node_id.as_str()),
            "recovery provider request must target its bound Verify or Review stage"
        );
        let node = material
            .graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == node_id)
            .context("recovery provider node missing")?;
        let item = canonical_work_item(grant, &material);
        let context = item
            .stage_context(&self.root, node_id, node.parent_receipts.clone())?
            .context("recovery provider context missing")?;
        let mut expected =
            item.provider_request_body(&self.root, node_id, Some(context.clone()))?;
        expected["expected_retained_lease"] = serde_json::to_value(&retained.lease)?;
        expected["recovery_event_id"] = serde_json::json!(self.event_id);
        anyhow::ensure!(
            &expected == request,
            "recovery provider request differs from canonical stage payload"
        );
        // A matching projection alone is not a durable context/use receipt.
        // Require the exact stage assembly already retained by Vaire.
        let memory = MnemosyneService::new(self.root.join("data/vaire"))?
            .with_contract_memory_root(self.root.join("core/state/memory"));
        memory.validate_context_assembly_for_recovery(
            &context,
            u128::try_from(chrono::Utc::now().timestamp_millis())?,
            u128::from(grant.activated_at_unix_ms),
            u128::from(grant.expires_at_unix_ms),
        )?;
        Ok(())
    }
}
