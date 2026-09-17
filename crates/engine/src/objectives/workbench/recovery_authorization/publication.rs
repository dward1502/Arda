//! Admission of replayable terminal effects; no projection occurs before commit.
use super::*;
use crate::objectives::RecoveryPublication;
use crate::runs::RunStore;
use arda_core::run_graph::NodeId;

impl RecoveryAuthorization {
    pub(crate) fn apply_completed_publication(
        root: &std::path::Path,
        publication: &RecoveryPublication,
    ) -> Result<()> {
        anyhow::ensure!(
            publication.kind == "completion",
            "not a completion publication"
        );
        let prepared: PreparedResidentOutcome =
            serde_json::from_value(publication.payload["prepared"].clone())?;
        let item: ExplicitWorkbenchWorkItem =
            serde_json::from_value(publication.payload["item"].clone())?;
        let saved: Vec<StageReceipt> =
            serde_json::from_value(publication.payload["receipts"].clone())?;
        let outcome = item.completed_outcome(root, &publication.payload["run"])?;
        let mut expected = project_receipts(
            root,
            &item.run_id,
            &item.project_contract_digest,
            &item,
            &outcome,
        )?;
        for receipt in &mut expected {
            receipt.context_outcome_receipt_id = Some(prepared.context_outcome.receipt_id.clone());
            receipt.context_outcome_receipt_digest =
                Some(prepared.context_outcome.receipt_digest.clone());
            receipt.binding_digest = Some(receipt.computed_binding_digest()?);
        }
        anyhow::ensure!(
            saved == expected,
            "completion evidence changed before projection"
        );
        let memory = MnemosyneService::new(root.join("data/vaire"))?
            .with_contract_memory_root(root.join("core/state/memory"));
        apply_resident_context_outcome(&memory, &prepared)
    }

    pub(crate) fn commit_close_publication(&self) -> Result<()> {
        let guard = self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, _| {
                let material = self
                    .store
                    .validate_retained_recovery_material(&self.root, tx, grant)?;
                let item = canonical_work_item(grant, &material);
                let guard = RunStore::open(&self.root, grant.bindings.run_id.clone())?
                    .lock_recovery_mutation(
                        grant,
                        &grant.bindings.close_node_id,
                        self.lease.expires_ms.try_into()?,
                    )?;
                let graph = guard
                    .recover()?
                    .checkpoint
                    .context("Close checkpoint missing")?;
                let review_parent = graph
                    .nodes
                    .iter()
                    .find(|node| node.id == grant.bindings.review_node_id)
                    .and_then(|node| node.output_digest.as_deref())
                    .context("Close Review receipt missing")?;
                let request = item.close_request_body(review_parent)?;
                let receipt = item.close_receipt(review_parent)?;
                let publication = RecoveryPublication {
                    key: "close".into(),
                    kind: "close".into(),
                    node_id: grant.bindings.close_node_id.clone(),
                    payload: serde_json::json!({"item": item, "review_parent": review_parent,
                        "request": request, "receipt": receipt}),
                };
                self.store
                    .insert_recovery_publication_in(tx, grant, &self.lease, &publication)?;
                Ok(guard)
            },
        )?;
        drop(guard);
        Ok(())
    }

    pub(crate) fn commit_provider_publication(
        &self,
        node: &NodeId,
        request: &serde_json::Value,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.store
            .with_retained_recovery(
                &self.operator_id,
                &self.event_id,
                &self.lease,
                |tx, grant, retained| {
                    self.validate_provider_request_in(tx, grant, retained, node.as_str(), request)?;
                    let guarded = RunStore::open(&self.root, grant.bindings.run_id.clone())?
                        .lock_recovery_mutation(grant, node, self.lease.expires_ms.try_into()?)?;
                    let current = guarded.recover()?;
                    let latest_start = current
                        .events
                        .iter()
                        .rev()
                        .find(|event| {
                            event.node_id == *node
                                && matches!(
                                    event.kind,
                                    crate::runs::RunEventKind::NodeTransition {
                                        state: arda_core::run_graph::NodeState::Running,
                                        ..
                                    }
                                )
                        })
                        .context("provider publication has no admitted start")?;
                    anyhow::ensure!(
                        payload["request"] == *request,
                        "publication request changed"
                    );
                    anyhow::ensure!(
                        payload["start_key"].as_str()
                            == Some(latest_start.idempotency_key.as_str()),
                        "publication does not match current provider start"
                    );
                    if let Some(receipt) = payload.get("receipt") {
                        let receipt: crate::adapters::HermesExecutionReceipt =
                            serde_json::from_value(receipt.clone())?;
                        anyhow::ensure!(
                            receipt.has_valid_digest()?
                                && receipt.run_id == grant.bindings.run_id.as_str()
                                && receipt.node_id == node.as_str(),
                            "publication receipt binding changed"
                        );
                        let canonical = current
                            .checkpoint
                            .as_ref()
                            .context("checkpoint missing")?
                            .nodes
                            .iter()
                            .find(|value| value.id == *node)
                            .context("node missing")?;
                        anyhow::ensure!(
                            receipt.parent_receipts == canonical.parent_receipts
                                && receipt.project_contract_digest
                                    == grant.bindings.project_contract_digest,
                            "publication parent or contract changed"
                        );
                    } else {
                        anyhow::ensure!(
                            payload["error"]
                                .as_str()
                                .is_some_and(|error| !error.is_empty()),
                            "publication has neither outcome nor failure evidence"
                        );
                    }
                    self.store.insert_recovery_publication_in(
                        tx,
                        grant,
                        &self.lease,
                        &RecoveryPublication {
                            key: format!("{}:terminal", latest_start.idempotency_key),
                            kind: "provider-finalization".into(),
                            node_id: node.clone(),
                            payload,
                        },
                    )?;
                    Ok(guarded)
                },
            )
            .map(|_guard| ())
    }

    pub(crate) fn reconcile_publications(
        &self,
        apply: impl Fn(&RunStore, &RecoveryPublication) -> Result<()>,
    ) -> Result<()> {
        self.store.reconcile_recovery_publications(
            &self.root,
            &self.operator_id,
            &self.event_id,
            apply,
        )
    }
}
