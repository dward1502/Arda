//! Exact retained-leaf execution, reachable only after authenticated ingress.
use super::*;
use crate::objectives::RecoveryAuthorization;
use crate::objectives::{ClaimedLeaf, ObjectiveStore};
mod publication;
pub(super) use publication::publish_provider;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(in crate::harness) enum RecoveryResult {
    Completed,
    CleanupOnly,
}
impl RecoveryResult {
    pub(in crate::harness) fn summary(&self) -> &'static str {
        match self {
            Self::Completed => "Retained stage completion verified and keeper cleanup reconciled; no objective control change requested.",
            Self::CleanupOnly => "Exact retained keeper cleanup reconciled after cancellation or supersession. No completion effects were replayed; any pending replay was durably suppressed. Earlier effects are preserved. No new execution or objective control change requested.",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::harness) async fn execute(
    state: &HarnessState,
    jobs: &crate::harness::RecoveryJobs,
    keeper: Arc<dyn crate::objectives::SnapshotAdmission>,
    operator_id: &str,
    event_id: &str,
    payload_digest: &str,
    objective_id: &str,
    leaf_id: &str,
    run_id: &str,
) -> Result<RecoveryResult, ApiError> {
    // The request owns only the response waiter. Once polled, recovery belongs
    // to the harness runtime and must finish even if that waiter is dropped.
    let state = state.clone();
    let operator_id = operator_id.to_owned();
    let event_id = event_id.to_owned();
    let payload_digest = payload_digest.to_owned();
    let objective_id = objective_id.to_owned();
    let leaf_id = leaf_id.to_owned();
    let run_id = run_id.to_owned();
    // Admission and keeper reconciliation perform bounded synchronous IO;
    // they must not block executor threads needed by the worker transport.
    let shutdown = jobs.shutdown();
    let key = serde_json::to_string(&(&state.workbench_root, &operator_id, &event_id))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    jobs.execute(key, payload_digest.clone(), async move {
        execute_owned(
            &state,
            shutdown,
            keeper,
            &operator_id,
            &event_id,
            &payload_digest,
            &objective_id,
            &leaf_id,
            &run_id,
        )
        .await
    })
    .await
}

#[allow(clippy::too_many_arguments)]
async fn execute_owned(
    state: &HarnessState,
    shutdown: crate::supervisor::Shutdown,
    keeper: Arc<dyn crate::objectives::SnapshotAdmission>,
    operator_id: &str,
    event_id: &str,
    payload_digest: &str,
    objective_id: &str,
    leaf_id: &str,
    run_id: &str,
) -> Result<RecoveryResult, ApiError> {
    if shutdown.is_triggered() {
        return Err(ApiError::stopping());
    }
    let store =
        ObjectiveStore::open_existing(state.workbench_root.join("data/arda/objectives.sqlite3"))
            .map_err(recovery_error)?
            .with_snapshot_admission(keeper);
    if store
        .cleanup_stopped_recovery_completion(
            &state.workbench_root,
            operator_id,
            event_id,
            payload_digest,
            objective_id,
            leaf_id,
            run_id,
        )
        .map_err(recovery_error)?
    {
        return Ok(RecoveryResult::CleanupOnly);
    }
    if store
        .reconcile_saved_completed_recovery(
            &state.workbench_root,
            operator_id,
            event_id,
            payload_digest,
            objective_id,
            leaf_id,
            run_id,
            |guard, intent| publication::apply_publication(state, guard, intent),
        )
        .map_err(recovery_error)?
    {
        return Ok(RecoveryResult::Completed);
    }
    let grant = store
        .prepare_recovery_admission(
            &state.workbench_root,
            operator_id,
            event_id,
            payload_digest,
            objective_id,
            leaf_id,
            run_id,
        )
        .map_err(recovery_error)?;
    store
        .with_recorded_recovery_admission(operator_id, event_id, |tx, saved| {
            store.validate_retained_recovery_material(&state.workbench_root, tx, saved)?;
            RunStore::open(&state.workbench_root, saved.bindings.run_id.clone())?
                .activate_recovery(saved.clone())?;
            Ok(())
        })
        .map_err(recovery_error)?;
    let remaining = i64::try_from(grant.expires_at_unix_ms)
        .map_err(|_| ApiError::conflict("recovery deadline overflow"))?
        .checked_sub(chrono::Utc::now().timestamp_millis())
        .filter(|remaining| *remaining > 0)
        .ok_or_else(|| ApiError::conflict("recovery window expired"))?;
    // Isolated subprocess qualification can expire a lease before its grant.
    #[cfg(test)]
    let remaining = if std::env::var_os("ARDA_TEST_RECOVERY_LEASE_CLOCK").is_some() {
        remaining.min(30_000)
    } else {
        remaining
    };
    let claim = store
        .claim_validated_retained_recovery(
            &state.workbench_root,
            operator_id,
            event_id,
            &format!("harness-recovery:{event_id}"),
            remaining,
        )
        .map_err(recovery_error)?;
    let authorization = Arc::new(
        RecoveryAuthorization::new(
            state.workbench_root.clone(),
            store.clone(),
            operator_id.to_owned(),
            event_id.to_owned(),
            &claim,
        )
        .map_err(recovery_error)?,
    );
    drive(state, shutdown, authorization, claim).await?;
    if !store
        .reconcile_saved_completed_recovery(
            &state.workbench_root,
            operator_id,
            event_id,
            payload_digest,
            objective_id,
            leaf_id,
            run_id,
            |guard, intent| publication::apply_publication(state, guard, intent),
        )
        .map_err(recovery_error)?
    {
        return Err(ApiError::conflict(
            "recovery completion is durable but keeper cleanup is pending",
        ));
    }
    Ok(RecoveryResult::Completed)
}

async fn drive(
    state: &HarnessState,
    shutdown: crate::supervisor::Shutdown,
    authorization: Arc<RecoveryAuthorization>,
    claim: ClaimedLeaf,
) -> Result<(), ApiError> {
    let item = authorization.canonical_item().map_err(recovery_error)?;
    for stage in ["verify", "review"] {
        let (_, graph) = load_run(state, &item.run_id)?;
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == stage)
            .ok_or_else(|| ApiError::conflict("retained stage missing"))?;
        if node.state == NodeState::Succeeded {
            continue;
        }
        let raw = authorization
            .prepare_provider_request(stage)
            .map_err(recovery_error)?;
        let Json(response) = execute_provider_node_authorized(
            state.clone(),
            item.run_id.clone(),
            stage.to_owned(),
            Some(axum::Extension(shutdown.clone())),
            raw,
            Some(Arc::clone(&authorization)),
        )
        .await?;
        if response.receipt.status != HermesReceiptStatus::Succeeded {
            return Err(ApiError::conflict(
                "retained provider stage did not succeed; no later stage was dispatched",
            ));
        }
    }
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    authorization
        .commit_close_publication()
        .map_err(recovery_error)?;
    authorization
        .reconcile_publications(|store, intent| {
            publication::apply_publication(state, store, intent)
        })
        .map_err(recovery_error)?;
    let (store, graph) = load_run(state, &item.run_id)?;
    let response = run_response(&store, graph)?;
    let run =
        serde_json::to_value(response).map_err(|error| ApiError::internal(error.to_string()))?;
    authorization
        .publish_completed(&claim, &run)
        .map_err(recovery_error)
}

pub(super) fn recovery_error(error: anyhow::Error) -> ApiError {
    ApiError::conflict(format!("retained recovery rejected: {error}"))
}
