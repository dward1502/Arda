use arda_core::run_graph::{NodeId, NodeKind, NodeState, RunGraph, RunId};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::runs::{
    apply_transition_once, AppendOutcome, RunEvent, RunEventDraft, RunEventKind, RunStore,
};

use super::{
    projects::{
        find_attached_project, require_loopback, ApiError, MutationEnvelope, WORKBENCH_MUTATIONS,
    },
    HarnessState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRunRequest {
    project_id: String,
    graph: RunGraph,
    envelope: MutationEnvelope,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproveRunRequest {
    node_id: String,
    envelope: MutationEnvelope,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRunRequest {
    reason: String,
    envelope: MutationEnvelope,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    graph: RunGraph,
    events: Vec<RunEvent>,
}

#[derive(Debug, Serialize)]
pub struct RunEventsResponse {
    events: Vec<RunEvent>,
}

pub(super) async fn plan_run(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<PlanRunRequest>,
) -> Result<(StatusCode, Json<RunResponse>), ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    validate_run_id(request.graph.run_id.as_str())?;
    request
        .graph
        .validate()
        .map_err(|error| ApiError::bad_request(format!("invalid run graph: {error}")))?;
    let first_node = request
        .graph
        .nodes
        .first()
        .ok_or_else(|| ApiError::bad_request("run graph must contain at least one node"))?
        .id
        .clone();
    find_attached_project(&state.workbench_root, &request.project_id)?;

    let store =
        RunStore::open(&state.workbench_root, request.graph.run_id.clone()).map_err(store_error)?;
    let recovered = store.recover().map_err(store_error)?;
    if let Some(existing) = recovered.checkpoint {
        if recovered
            .applied_idempotency_keys
            .contains_key(&request.envelope.idempotency_key)
            && existing == request.graph
        {
            return Ok((StatusCode::OK, Json(run_response(&store, existing)?)));
        }
        return Err(ApiError::conflict(format!(
            "run `{}` already exists",
            request.graph.run_id.as_str()
        )));
    }

    let outcome = store
        .append(RunEventDraft {
            node_id: first_node,
            idempotency_key: request.envelope.idempotency_key,
            kind: RunEventKind::Planned {
                project_id: request.project_id,
                approval_id: request.envelope.approval.approval_id.clone(),
            },
            receipt_digest: Some(request.envelope.approval.approval_id),
        })
        .map_err(store_error)?;
    store
        .write_checkpoint(&request.graph)
        .map_err(store_error)?;
    let status = match outcome {
        AppendOutcome::Appended { .. } => StatusCode::CREATED,
        AppendOutcome::AlreadyApplied { .. } => StatusCode::OK,
    };
    Ok((status, Json(run_response(&store, request.graph)?)))
}

pub(super) async fn approve_run(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(request): Json<ApproveRunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    let (store, mut graph) = load_run(&state, &id)?;
    let recovered = store.recover().map_err(store_error)?;
    if recovered
        .applied_idempotency_keys
        .contains_key(&request.envelope.idempotency_key)
    {
        return Ok(Json(run_response(&store, graph)?));
    }

    let node_id = NodeId::new(request.node_id.clone())
        .map_err(|error| ApiError::bad_request(format!("invalid node id: {error}")))?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| ApiError::not_found(format!("node `{}` was not found", node_id.as_str())))?;
    if node.kind != NodeKind::Approval {
        return Err(ApiError::conflict(format!(
            "node `{}` is not an approval node",
            node_id.as_str()
        )));
    }

    loop {
        let state = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.state)
            .expect("approval node was validated");
        let (next, suffix) = match state {
            NodeState::Pending | NodeState::Blocked | NodeState::Failed => {
                (NodeState::Ready, "ready")
            }
            NodeState::Ready => (NodeState::Running, "running"),
            NodeState::Running => (NodeState::Succeeded, "approved"),
            NodeState::Succeeded => {
                return Err(ApiError::conflict(format!(
                    "approval node `{}` already succeeded under another idempotency key",
                    node_id.as_str()
                )));
            }
            NodeState::Cancelled | NodeState::Superseded => {
                return Err(ApiError::conflict(format!(
                    "approval node `{}` is terminal in state {state:?}",
                    node_id.as_str()
                )));
            }
        };
        let key = if next == NodeState::Succeeded {
            request.envelope.idempotency_key.clone()
        } else {
            format!("{}:approval:{suffix}", request.envelope.idempotency_key)
        };
        apply_transition_once(
            &store,
            &mut graph,
            &node_id,
            next,
            key,
            Some(request.envelope.approval.approval_id.clone()),
        )
        .map_err(store_error)?;
        if next == NodeState::Succeeded {
            break;
        }
    }

    Ok(Json(run_response(&store, graph)?))
}

pub(super) async fn cancel_run(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(request): Json<CancelRunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    if request.reason.trim().is_empty() {
        return Err(ApiError::bad_request("cancellation reason cannot be empty"));
    }
    let (store, mut graph) = load_run(&state, &id)?;
    let recovered = store.recover().map_err(store_error)?;
    if recovered
        .applied_idempotency_keys
        .contains_key(&request.envelope.idempotency_key)
    {
        return Ok(Json(run_response(&store, graph)?));
    }

    let cancellable: Vec<NodeId> = graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.state,
                NodeState::Pending
                    | NodeState::Ready
                    | NodeState::Blocked
                    | NodeState::Running
                    | NodeState::Failed
            )
        })
        .map(|node| node.id.clone())
        .collect();
    let receipt_node = cancellable
        .first()
        .cloned()
        .or_else(|| graph.nodes.first().map(|node| node.id.clone()))
        .ok_or_else(|| ApiError::conflict("run has no nodes to cancel"))?;
    if cancellable.is_empty() {
        return Err(ApiError::conflict("run has no cancellable nodes"));
    }

    for node_id in cancellable {
        apply_transition_once(
            &store,
            &mut graph,
            &node_id,
            NodeState::Cancelled,
            format!(
                "{}:cancel:{}",
                request.envelope.idempotency_key,
                node_id.as_str()
            ),
            Some(request.envelope.approval.approval_id.clone()),
        )
        .map_err(store_error)?;
    }
    store
        .append(RunEventDraft {
            node_id: receipt_node,
            idempotency_key: request.envelope.idempotency_key,
            kind: RunEventKind::Cancelled {
                reason: request.reason,
            },
            receipt_digest: Some(request.envelope.approval.approval_id),
        })
        .map_err(store_error)?;
    store.write_checkpoint(&graph).map_err(store_error)?;

    Ok(Json(run_response(&store, graph)?))
}

pub(super) async fn get_run(
    State(state): State<HarnessState>,
    Path(id): Path<String>,
) -> Result<Json<RunResponse>, ApiError> {
    let (store, graph) = load_run(&state, &id)?;
    Ok(Json(run_response(&store, graph)?))
}

pub(super) async fn get_run_events(
    State(state): State<HarnessState>,
    Path(id): Path<String>,
) -> Result<Json<RunEventsResponse>, ApiError> {
    let (store, _) = load_run(&state, &id)?;
    let recovered = store.recover().map_err(store_error)?;
    Ok(Json(RunEventsResponse {
        events: recovered.events,
    }))
}

fn load_run(state: &HarnessState, id: &str) -> Result<(RunStore, RunGraph), ApiError> {
    validate_run_id(id)?;
    let run_id = RunId::new(id)
        .map_err(|error| ApiError::bad_request(format!("invalid run id: {error}")))?;
    let checkpoint = state
        .workbench_root
        .join("data/runs")
        .join(id)
        .join("checkpoint.json");
    if !checkpoint.is_file() {
        return Err(ApiError::not_found(format!("run `{id}` was not found")));
    }
    let store = RunStore::open(&state.workbench_root, run_id).map_err(store_error)?;
    let graph = store
        .recover()
        .map_err(store_error)?
        .checkpoint
        .ok_or_else(|| ApiError::not_found(format!("run `{id}` has no checkpoint")))?;
    Ok((store, graph))
}

fn run_response(store: &RunStore, graph: RunGraph) -> Result<RunResponse, ApiError> {
    let events = store.recover().map_err(store_error)?.events;
    Ok(RunResponse { graph, events })
}

fn validate_run_id(id: &str) -> Result<(), ApiError> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "run id must be 1-128 ASCII letters, digits, '.', '_' or '-'",
        ))
    }
}

fn store_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(format!("run store error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::validate_run_id;

    #[test]
    fn run_ids_cannot_escape_the_run_store() {
        assert!(validate_run_id("run-1").is_ok());
        assert!(validate_run_id("../outside").is_err());
        assert!(validate_run_id("run/child").is_err());
    }
}
