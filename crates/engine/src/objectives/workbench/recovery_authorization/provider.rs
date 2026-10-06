//! Canonical payload validation and synchronous, journal-fenced provider launch.
use super::*;

impl RecoveryAuthorization {
    /// Build and durably bind only a stage of the saved canonical item.
    pub fn prepare_provider_request(&self, node_id: &str) -> Result<serde_json::Value> {
        self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, retained| {
                anyhow::ensure!(
                    node_id == "verify" || node_id == "review",
                    "unsupported recovery stage"
                );
                let material = self
                    .store
                    .validate_retained_recovery_material(&self.root, tx, grant)?;
                let item = canonical_work_item(grant, &material);
                let node = material
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.id.as_str() == node_id)
                    .context("missing recovery stage")?;
                let projected = item
                    .stage_context(&self.root, node_id, node.parent_receipts.clone())?
                    .context("missing retained context")?;
                let memory = MnemosyneService::new(self.root.join("data/vaire"))?
                    .with_contract_memory_root(self.root.join("core/state/memory"));
                let context = memory.bind_run_stage_context_for_recovery(
                    &material.context,
                    node_id,
                    &projected.use_receipt.purpose,
                    node.parent_receipts.clone(),
                    u128::try_from(Utc::now().timestamp_millis())?,
                    u128::from(grant.activated_at_unix_ms)..u128::from(grant.expires_at_unix_ms),
                )?;
                let mut request = item.provider_request_body(&self.root, node_id, Some(context))?;
                request["expected_retained_lease"] = serde_json::to_value(&retained.lease)?;
                request["recovery_event_id"] = serde_json::json!(self.event_id);
                self.validate_provider_request_in(tx, grant, retained, node_id, &request)?;
                Ok(request)
            },
        )
    }

    /// Close has no provider start. Persist its canonical receipt before the
    /// graph transition, with the same stop/lease fence covering both writes.
    pub fn with_close_mutation<T>(
        &self,
        operation: impl FnOnce(&crate::runs::RunStore, serde_json::Value) -> T,
    ) -> Result<T> {
        self.store
            .with_retained_recovery(
                &self.operator_id,
                &self.event_id,
                &self.lease,
                |tx, grant, _| {
                    let material = self
                        .store
                        .validate_retained_recovery_material(&self.root, tx, grant)?;
                    let close = material
                        .graph
                        .nodes
                        .iter()
                        .find(|node| node.id == grant.bindings.close_node_id)
                        .context("missing recovery Close")?;
                    anyhow::ensure!(
                        close.worker.is_none(),
                        "recovery Close cannot own a provider"
                    );
                    let review = material
                        .graph
                        .nodes
                        .iter()
                        .find(|node| node.id == grant.bindings.review_node_id)
                        .context("missing recovery Review")?;
                    anyhow::ensure!(
                        review.state == arda_core::run_graph::NodeState::Succeeded,
                        "Review is not complete"
                    );
                    let parent = review
                        .output_digest
                        .as_deref()
                        .context("Review receipt missing")?;
                    anyhow::ensure!(
                        close.parent_receipts == [parent.to_owned()],
                        "Close lineage differs from Review"
                    );
                    let item = canonical_work_item(grant, &material);
                    let request = item.close_request_body(parent)?;
                    let guarded =
                        crate::runs::RunStore::open(&self.root, grant.bindings.run_id.clone())?
                            .lock_recovery_mutation(
                                grant,
                                &grant.bindings.close_node_id,
                                self.lease.expires_ms.try_into()?,
                            )?;
                    item.persist_close_receipt(&self.root, parent, &self.store)?;
                    let result = operation(&guarded, request);
                    Ok((guarded, result))
                },
            )
            .map(|(_guard, result)| result)
    }

    /// Keep stop generation, exact lease and current context policy fenced
    /// through a synchronous provider projection. Never await in the callback.
    pub fn with_provider_mutation<T>(
        &self,
        node_id: &str,
        request: &serde_json::Value,
        operation: impl FnOnce(&crate::runs::RunStore) -> T,
    ) -> Result<T> {
        self.store
            .with_retained_recovery(
                &self.operator_id,
                &self.event_id,
                &self.lease,
                |tx, grant, retained| {
                    self.validate_provider_request_in(tx, grant, retained, node_id, request)?;
                    let guarded =
                        crate::runs::RunStore::open(&self.root, grant.bindings.run_id.clone())?
                            .lock_recovery_mutation(
                                grant,
                                &arda_core::run_graph::NodeId::new(node_id)?,
                                retained.lease.expires_ms.try_into()?,
                            )?;
                    let result = operation(&guarded);
                    Ok((guarded, result))
                },
            )
            .map(|(_guard, result)| result)
    }

    /// Resolve a provider's immutable overlay through the same canonical fence
    /// used at dispatch. This does not consume a start or replace the context.
    pub fn provider_binding(
        &self,
        node_id: &str,
        request: &serde_json::Value,
    ) -> Result<(RecoveryGrant, crate::objectives::RetainedExecution)> {
        self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, retained| {
                self.validate_provider_request_in(tx, grant, retained, node_id, request)?;
                Ok((grant.clone(), retained.clone()))
            },
        )
    }

    /// Auxiliary delimiters keep the objective and canonical run cancellation
    /// fences held, but cannot spend another Chat start.
    pub fn dispatch_provider_auxiliary<T>(
        &self,
        request: &serde_json::Value,
        retained: &crate::objectives::RetainedExecution,
        start: &crate::runs::RunEventDraft,
        expected_node: &arda_core::run_graph::RunNode,
        send: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, current| {
                anyhow::ensure!(
                    retained.snapshot == current.snapshot && retained.lease == current.lease,
                    "recovery transport differs from current captured binding"
                );
                self.validate_provider_request_in(
                    tx,
                    grant,
                    current,
                    start.node_id.as_str(),
                    request,
                )?;
                self.validate_provider_node_in(tx, grant, expected_node)?;
                let canonical_store =
                    crate::runs::RunStore::open(&self.root, grant.bindings.run_id.clone())?;
                canonical_store.with_recovery_auxiliary_before(
                    grant,
                    start,
                    u64::try_from(current.lease.expires_ms)?,
                    send,
                )?
            },
        )
    }

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
        self.launch_provider_request_checked(request, retained, run_store, draft, None, send)
    }

    pub(crate) fn launch_provider_request_checked<T>(
        &self,
        request: &serde_json::Value,
        retained: &crate::objectives::RetainedExecution,
        run_store: &crate::runs::RunStore,
        draft: crate::runs::RunEventDraft,
        expected_node: Option<&arda_core::run_graph::RunNode>,
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
                if let Some(expected) = expected_node {
                    self.validate_provider_node_in(tx, grant, expected)?;
                }
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

    fn validate_provider_node_in(
        &self,
        tx: &rusqlite::Transaction<'_>,
        grant: &RecoveryGrant,
        expected: &arda_core::run_graph::RunNode,
    ) -> Result<()> {
        let material = self
            .store
            .validate_retained_recovery_material(&self.root, tx, grant)?;
        let current = material
            .graph
            .nodes
            .iter()
            .find(|node| node.id == expected.id)
            .context("recovery provider node missing")?;
        let identity = |node: &arda_core::run_graph::RunNode| -> Result<serde_json::Value> {
            let mut value = serde_json::to_value(node)?;
            let fields = value.as_object_mut().context("node must be an object")?;
            // State and checkpoint are mutable journal projection, not authority.
            fields.remove("state");
            fields.remove("checkpoint");
            Ok(value)
        };
        anyhow::ensure!(
            identity(current)? == identity(expected)?,
            "recovery dispatch node authority changed after construction"
        );
        Ok(())
    }

    pub(super) fn validate_provider_request_in(
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
