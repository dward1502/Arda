//! Apply exact committed effects. This module never calls a provider adapter.
use super::*;
use crate::objectives::RecoveryPublication;
use anyhow::Context;
#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;

pub(super) fn apply_publication(
    _state: &HarnessState,
    store: &RunStore,
    publication: &RecoveryPublication,
) -> anyhow::Result<()> {
    if publication.kind == "close" {
        let item: arda_aule::prometheus::autopilot::ExplicitWorkbenchWorkItem =
            serde_json::from_value(publication.payload["item"].clone())?;
        anyhow::ensure!(
            item.run_id == store.run_id().as_str() && publication.node_id.as_str() == "close",
            "Close target changed"
        );
        let parent = publication.payload["review_parent"]
            .as_str()
            .context("Close parent missing")?;
        let receipt = item.close_receipt(parent)?;
        anyhow::ensure!(
            publication.payload["receipt"] == receipt
                && publication.payload["request"] == item.close_request_body(parent)?,
            "Close payload changed"
        );
        if let Some(existing) = store.read_execution_receipt(&publication.node_id)? {
            anyhow::ensure!(
                existing == receipt,
                "Close receipt conflicts with committed publication"
            );
        }
        store.write_execution_receipt(&publication.node_id, &receipt)?;
        let request = serde_json::from_value(publication.payload["request"].clone())?;
        apply_close_projection(store, request)?;
        return Ok(());
    }
    apply_provider_publication(store, publication)
}

fn apply_provider_publication(
    store: &RunStore,
    publication: &RecoveryPublication,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        publication.kind == "provider-finalization",
        "unsupported recovery projection kind"
    );
    let payload = &publication.payload;
    let recovered = store.recover()?;
    let mut graph = recovered
        .checkpoint
        .context("publication checkpoint missing")?;
    let node_id = &publication.node_id;
    let start_key = payload["start_key"]
        .as_str()
        .context("publication start missing")?;
    anyhow::ensure!(
        recovered
            .events
            .iter()
            .any(|event| event.node_id == *node_id
                && event.idempotency_key == start_key
                && matches!(
                    event.kind,
                    RunEventKind::NodeTransition {
                        state: NodeState::Running
                    }
                )),
        "publication start is not in the canonical journal"
    );
    // Checkpoints are projections: the committed launch/terminal journal wins
    // across a crash between journal append and checkpoint replacement.
    if let Some(event) = recovered.events.iter().rev().find(|event| {
        event.node_id == *node_id && matches!(event.kind, RunEventKind::NodeTransition { .. })
    }) {
        if let RunEventKind::NodeTransition { state } = event.kind {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
                .context("publication node missing")?;
            node.state = state;
            if matches!(state, NodeState::Succeeded | NodeState::Failed) {
                node.output_digest = event.receipt_digest.clone();
            }
            node.checkpoint.sequence = payload["attempt"]
                .as_u64()
                .context("publication attempt missing")?;
        }
    }
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == *node_id)
        .context("publication node missing")?;
    if let Some(value) = payload.get("receipt") {
        let receipt: HermesExecutionReceipt = serde_json::from_value(value.clone())?;
        anyhow::ensure!(
            receipt.has_valid_digest()?
                && receipt.run_id == store.run_id().as_str()
                && receipt.node_id == node_id.as_str()
                && receipt.parent_receipts == node.parent_receipts,
            "committed receipt binding changed"
        );
        if let Some(existing) = store.read_execution_receipt(node_id)? {
            anyhow::ensure!(
                existing == *value,
                "canonical receipt conflicts with committed publication"
            );
        }
        if matches!(node.state, NodeState::Succeeded | NodeState::Failed) {
            anyhow::ensure!(
                node.output_digest.as_deref() == Some(receipt.receipt_digest.as_str()),
                "terminal digest conflicts with committed publication"
            );
        }
        store.write_execution_receipt(node_id, value)?;
        store.append_resource_usage(ResourceUsageDraft {
            idempotency_key: provider_usage_idempotency_key(
                store.run_id().as_str(),
                &receipt.idempotency_key,
            ),
            source: if receipt.usage.cost_measurement == CostMeasurement::Observed {
                ResourceMeasurementSource::Observed
            } else {
                ResourceMeasurementSource::DefaultFallback
            },
            provider_id: Some(
                receipt
                    .usage
                    .provider
                    .clone()
                    .unwrap_or_else(|| "unknown-provider".into()),
            ),
            local_joulework: 0.0,
            hosted_cost_usd: receipt.usage.estimated_cost_usd,
            hosted_requests: receipt.usage.api_calls,
            supersedes: None,
        })?;
        // A terminal journal append may have survived without its checkpoint.
        // Replay the canonical transition even when its state already matches:
        // this also restores outgoing receipts and the successful checkpoint.
        if let Some(event) = recovered.events.iter().rev().find(|event| {
            event.node_id == *node_id
                && matches!(
                    event.kind,
                    RunEventKind::NodeTransition {
                        state: NodeState::Succeeded
                    }
                )
        }) {
            apply_transition_once(
                store,
                &mut graph,
                node_id,
                NodeState::Succeeded,
                &event.idempotency_key,
                event.receipt_digest.clone(),
            )?;
        }
        finalize_provider_receipt(store, &mut graph, node_id, &receipt).map_err(api_error)?;
        let evidence = review_evidence_from_receipt(&receipt).map_err(api_error)?;
        project_review_evidence(
            store,
            node_id.as_str(),
            &receipt.idempotency_key,
            &receipt.receipt_digest,
            Some(&evidence),
        )
        .map_err(api_error)?;
    } else {
        anyhow::ensure!(
            payload["error"].as_str().is_some(),
            "publication outcome missing"
        );
        let error_key = payload["error_key"]
            .as_str()
            .context("publication error key missing")?;
        if node.state != NodeState::Failed {
            anyhow::ensure!(
                node.state == NodeState::Running,
                "failure conflicts with current terminal state"
            );
            apply_transition_once(
                store,
                &mut graph,
                node_id,
                NodeState::Failed,
                error_key,
                None,
            )?;
        }
    }
    store.write_checkpoint(&graph)?;
    Ok(())
}

fn apply_close_projection(store: &RunStore, request: CompleteRunNodeRequest) -> anyhow::Result<()> {
    let recovered = store.recover()?;
    let mut graph = recovered.checkpoint.context("Close checkpoint missing")?;
    let node_id = NodeId::new("close")?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .context("Close node missing")?;
    anyhow::ensure!(
        node.kind == NodeKind::Close && node.worker.is_none(),
        "Close authority changed"
    );
    anyhow::ensure!(
        graph
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .all(|edge| {
                graph
                    .nodes
                    .iter()
                    .find(|parent| parent.id == edge.from)
                    .is_some_and(|parent| {
                        parent.state == NodeState::Succeeded
                            && parent.output_digest == edge.parent_receipt
                            && edge
                                .parent_receipt
                                .as_ref()
                                .is_some_and(|digest| node.parent_receipts.contains(digest))
                    })
            }),
        "Close dependencies changed"
    );
    let key = &request.envelope.idempotency_key;
    let steps = [
        (NodeState::Ready, format!("{key}:ready")),
        (NodeState::Running, format!("{key}:running")),
        (NodeState::Succeeded, key.clone()),
    ];
    let mut latest = None;
    for (index, (state, key)) in steps.iter().enumerate() {
        if recovered.applied_idempotency_keys.contains_key(key) {
            store.append(RunEventDraft {
                node_id: node_id.clone(),
                idempotency_key: key.clone(),
                kind: RunEventKind::NodeTransition { state: *state },
                receipt_digest: Some(request.receipt_digest.clone()),
            })?;
            latest = Some(index);
        }
    }
    if let Some(index) = latest {
        graph
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .unwrap()
            .state = steps[index].0;
    }
    for (state, key) in steps.iter().skip(latest.unwrap_or(0)) {
        apply_transition_once(
            store,
            &mut graph,
            &node_id,
            *state,
            key,
            Some(request.receipt_digest.clone()),
        )?;
    }
    Ok(())
}

fn api_error(error: ApiError) -> anyhow::Error {
    anyhow::anyhow!("recovery projection failed: {error:?}")
}

pub(in crate::harness::runs) fn publish_provider(
    state: &HarnessState,
    authorization: &RecoveryAuthorization,
    store: &RunStore,
    node: &NodeId,
    request: &serde_json::Value,
    payload: serde_json::Value,
) -> Result<Json<ExecuteProviderNodeResponse>, ApiError> {
    // Attempt-addressed evidence is not the canonical receipt and cannot grant
    // a transition. It survives an expired or rolled-back intent admission.
    store
        .write_recovery_outcome_evidence(node, &payload)
        .map_err(recovery_error)?;

    if payload.get("receipt").is_none() {
        // An adapter error has no successful effects to publish. Retain its
        // immutable evidence, then project only the exact-start failure under
        // the same live authority fence. Replay can finish a partial projection.
        with_provider_mutation(
            &state.workbench_root,
            store,
            Some(authorization),
            node,
            request,
            |guard| {
                let current = guard.recover().map_err(store_error)?;
                let latest = current
                    .events
                    .iter()
                    .rev()
                    .find(|event| {
                        event.node_id == *node
                            && matches!(
                                event.kind,
                                RunEventKind::NodeTransition {
                                    state: NodeState::Running
                                }
                            )
                    })
                    .ok_or_else(|| ApiError::conflict("provider failure has no admitted start"))?;
                if payload["request"] != *request
                    || payload["start_key"].as_str() != Some(latest.idempotency_key.as_str())
                    || !payload["error"]
                        .as_str()
                        .is_some_and(|error| !error.is_empty())
                {
                    return Err(ApiError::conflict(
                        "provider failure does not match current start",
                    ));
                }
                apply_provider_publication(
                    guard,
                    &RecoveryPublication {
                        key: format!("{}:terminal", latest.idempotency_key),
                        kind: "provider-finalization".into(),
                        node_id: node.clone(),
                        payload,
                    },
                )
                .map_err(recovery_error)
            },
        )?;
        return Err(ApiError::conflict(
            "provider failed; exact-start failure evidence retained",
        ));
    }
    authorization
        .commit_provider_publication(node, request, payload)
        .map_err(recovery_error)?;
    authorization
        .reconcile_publications(|guard, publication| apply_publication(state, guard, publication))
        .map_err(recovery_error)?;
    let graph = store
        .recover()
        .map_err(store_error)?
        .checkpoint
        .ok_or_else(|| ApiError::internal("recovery checkpoint missing after publication"))?;
    let receipt = store
        .read_execution_receipt(node)
        .map_err(store_error)?
        .ok_or_else(|| {
            ApiError::conflict("provider failed; committed failure evidence retained")
        })?;
    let receipt: HermesExecutionReceipt =
        serde_json::from_value(receipt).map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(ExecuteProviderNodeResponse {
        run: run_response(store, graph)?,
        receipt,
    }))
}
