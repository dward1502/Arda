use arda_core::run_graph::{NodeId, NodeKind, NodeState, RunGraph, RunId};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive},
    response::Sse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Component, Path as FsPath, PathBuf};
use std::time::Duration;

use crate::adapters::{
    AdapterCancellation, HermesAdapter, HermesExecutionReceipt, HermesNodeTask, HermesReceiptStatus,
};
use crate::runs::{
    apply_transition_once, AppendOutcome, RunEvent, RunEventDraft, RunEventKind, RunStore,
};

use super::{
    projects::{
        contract_digest, find_attached_project, require_loopback, ApiError, MutationEnvelope,
        WORKBENCH_MUTATIONS,
    },
    HarnessState,
};

const MAX_REVIEW_ITEMS: usize = 128;
const MAX_REVIEW_PATH_BYTES: usize = 4096;
const MAX_REVIEW_TEXT_BYTES: usize = 64 * 1024;

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
pub struct CompleteRunNodeRequest {
    envelope: MutationEnvelope,
    receipt_digest: String,
    #[serde(default)]
    evidence: Option<RunReviewEvidence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteProviderNodeRequest {
    envelope: MutationEnvelope,
    objective: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunReviewEvidence {
    changes: Vec<ChangeEvidence>,
    tests: Vec<TestEvidence>,
    provider_receipt: Option<ProviderReceiptEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeEvidence {
    path: String,
    status: ChangeStatus,
    additions: u64,
    deletions: u64,
    #[serde(default)]
    diff: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestEvidence {
    name: String,
    status: TestStatus,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(default)]
    details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    Passed,
    Failed,
    Running,
    NotRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderReceiptEvidence {
    provider: String,
    model: String,
    adapter: String,
    receipt_digest: String,
    summary: String,
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
    review: RunReviewEvidence,
}

#[derive(Debug, Serialize)]
pub struct RunEventsResponse {
    events: Vec<RunEvent>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteProviderNodeResponse {
    run: RunResponse,
    receipt: HermesExecutionReceipt,
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
    let attached = find_attached_project(&state.workbench_root, &request.project_id)?;

    let store =
        RunStore::open(&state.workbench_root, request.graph.run_id.clone()).map_err(store_error)?;
    let recovered = store.recover().map_err(store_error)?;
    if let Some(existing) = recovered.checkpoint {
        if recovered
            .applied_idempotency_keys
            .contains_key(&request.envelope.idempotency_key)
        {
            return Ok((StatusCode::OK, Json(run_response(&store, existing)?)));
        }
        return Err(ApiError::conflict(format!(
            "run `{}` already exists",
            request.graph.run_id.as_str()
        )));
    }

    let mut graph = request.graph;
    graph.provenance.project_contract_digest = contract_digest(&attached.contract)?;
    let plan_node_id = graph
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Plan)
        .map(|node| node.id.clone());
    let plan_receipt = plan_node_id
        .as_ref()
        .and_then(|node_id| {
            graph
                .edges
                .iter()
                .find(|edge| edge.from == *node_id)
                .and_then(|edge| edge.parent_receipt.clone())
        })
        .unwrap_or_else(|| request.envelope.approval.approval_id.clone());
    let outcome = store
        .append(RunEventDraft {
            node_id: plan_node_id.clone().unwrap_or(first_node),
            idempotency_key: request.envelope.idempotency_key.clone(),
            kind: RunEventKind::Planned {
                project_id: request.project_id,
                approval_id: request.envelope.approval.approval_id.clone(),
            },
            receipt_digest: Some(plan_receipt.clone()),
        })
        .map_err(store_error)?;
    if matches!(outcome, AppendOutcome::Appended { .. }) {
        if let Some(plan_node_id) = plan_node_id {
            for (suffix, next) in [
                ("ready", NodeState::Ready),
                ("running", NodeState::Running),
                ("succeeded", NodeState::Succeeded),
            ] {
                apply_transition_once(
                    &store,
                    &mut graph,
                    &plan_node_id,
                    next,
                    format!("{}:plan:{suffix}", request.envelope.idempotency_key),
                    Some(plan_receipt.clone()),
                )
                .map_err(store_error)?;
            }
        }
    }
    store.write_checkpoint(&graph).map_err(store_error)?;
    let status = match outcome {
        AppendOutcome::Appended { .. } => StatusCode::CREATED,
        AppendOutcome::AlreadyApplied { .. } => StatusCode::OK,
    };
    Ok((status, Json(run_response(&store, graph)?)))
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

pub(super) async fn complete_run_node(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path((id, node_id)): Path<(String, String)>,
    Json(request): Json<CompleteRunNodeRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    validate_sha256_digest(&request.receipt_digest, "receipt digest")?;
    if let Some(evidence) = request.evidence.as_ref() {
        validate_review_evidence(evidence, &request.receipt_digest)?;
    }

    let _guard = WORKBENCH_MUTATIONS.lock().await;
    let (store, mut graph) = load_run(&state, &id)?;
    let recovered = store.recover().map_err(store_error)?;
    if recovered
        .applied_idempotency_keys
        .contains_key(&request.envelope.idempotency_key)
    {
        let existing_output = graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == node_id)
            .and_then(|node| node.output_digest.as_deref());
        if existing_output != Some(request.receipt_digest.as_str()) {
            return Err(ApiError::conflict(format!(
                "idempotency key is already bound to output receipt `{}`",
                existing_output.unwrap_or("missing")
            )));
        }
        project_review_evidence(
            &store,
            &node_id,
            &request.envelope.idempotency_key,
            &request.receipt_digest,
            request.evidence.as_ref(),
        )?;
        return Ok(Json(run_response(&store, graph)?));
    }

    let node_id = NodeId::new(node_id)
        .map_err(|error| ApiError::bad_request(format!("invalid node id: {error}")))?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| ApiError::not_found(format!("node `{}` was not found", node_id.as_str())))?;
    if !matches!(
        node.kind,
        NodeKind::Execute | NodeKind::Verify | NodeKind::Review | NodeKind::Close
    ) {
        return Err(ApiError::conflict(format!(
            "node `{}` cannot be completed through the operator receipt endpoint",
            node_id.as_str()
        )));
    }
    if matches!(
        node.state,
        NodeState::Pending | NodeState::Blocked | NodeState::Failed
    ) {
        let dependencies_succeeded =
            graph
                .edges
                .iter()
                .filter(|edge| edge.to == node_id)
                .all(|edge| {
                    graph
                        .nodes
                        .iter()
                        .find(|candidate| candidate.id == edge.from)
                        .is_some_and(|parent| {
                            parent.state == NodeState::Succeeded
                                && parent.output_digest == edge.parent_receipt
                                && edge
                                    .parent_receipt
                                    .as_ref()
                                    .is_some_and(|receipt| node.parent_receipts.contains(receipt))
                        })
                });
        if !dependencies_succeeded {
            return Err(ApiError::conflict(format!(
                "node `{}` has incomplete dependencies",
                node_id.as_str()
            )));
        }
        apply_transition_once(
            &store,
            &mut graph,
            &node_id,
            NodeState::Ready,
            format!("{}:ready", request.envelope.idempotency_key),
            Some(request.receipt_digest.clone()),
        )
        .map_err(store_error)?;
    }
    let state = graph
        .nodes
        .iter()
        .find(|candidate| candidate.id == node_id)
        .map(|candidate| candidate.state)
        .expect("completion node was validated");
    if state != NodeState::Ready {
        return Err(ApiError::conflict(format!(
            "node `{}` must be ready before completion; current state is {state:?}",
            node_id.as_str()
        )));
    }

    apply_transition_once(
        &store,
        &mut graph,
        &node_id,
        NodeState::Running,
        format!("{}:running", request.envelope.idempotency_key),
        Some(request.receipt_digest.clone()),
    )
    .map_err(store_error)?;
    apply_transition_once(
        &store,
        &mut graph,
        &node_id,
        NodeState::Succeeded,
        request.envelope.idempotency_key.clone(),
        Some(request.receipt_digest.clone()),
    )
    .map_err(store_error)?;
    project_review_evidence(
        &store,
        node_id.as_str(),
        &request.envelope.idempotency_key,
        &request.receipt_digest,
        request.evidence.as_ref(),
    )?;

    Ok(Json(run_response(&store, graph)?))
}

pub(super) async fn execute_provider_node(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path((id, node_id)): Path<(String, String)>,
    Json(request): Json<ExecuteProviderNodeRequest>,
) -> Result<Json<ExecuteProviderNodeResponse>, ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    if request.objective.trim().is_empty() {
        return Err(ApiError::bad_request("provider objective cannot be empty"));
    }

    // Serialize this first production slice with the existing Workbench mutation
    // lock. The durable receipt is written before terminal projection, so a
    // retry after an interrupted response reuses provider evidence rather than
    // issuing a second model call.
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    let (store, mut graph) = load_run(&state, &id)?;
    let node_id = NodeId::new(node_id)
        .map_err(|error| ApiError::bad_request(format!("invalid node id: {error}")))?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| ApiError::not_found(format!("node `{}` was not found", node_id.as_str())))?
        .clone();
    if node.kind != NodeKind::Execute {
        return Err(ApiError::conflict(format!(
            "node `{}` is not a provider-executable node",
            node_id.as_str()
        )));
    }

    let receipt = if let Some(value) = store
        .read_execution_receipt(&node_id)
        .map_err(store_error)?
    {
        let receipt: HermesExecutionReceipt = serde_json::from_value(value).map_err(|error| {
            ApiError::internal(format!("stored provider receipt is invalid: {error}"))
        })?;
        if receipt.run_id != id
            || receipt.node_id != node_id.as_str()
            || receipt.idempotency_key != node.idempotency_key
        {
            return Err(ApiError::conflict(
                "stored provider receipt does not match the requested run node",
            ));
        }
        if !receipt.has_valid_digest().map_err(|error| {
            ApiError::internal(format!(
                "stored provider receipt digest could not be checked: {error}"
            ))
        })? {
            return Err(ApiError::conflict(
                "stored provider receipt failed canonical digest verification",
            ));
        }
        receipt
    } else {
        if !matches!(
            node.state,
            NodeState::Pending | NodeState::Ready | NodeState::Blocked | NodeState::Failed
        ) {
            return Err(ApiError::conflict(format!(
                "node `{}` cannot start provider execution from state {:?}",
                node_id.as_str(),
                node.state
            )));
        }
        require_succeeded_dependencies(&graph, &node_id)?;
        let approval_receipt =
            node.parent_receipts.first().cloned().ok_or_else(|| {
                ApiError::conflict("provider execution requires an approval receipt")
            })?;
        if node.state != NodeState::Ready {
            apply_transition_once(
                &store,
                &mut graph,
                &node_id,
                NodeState::Ready,
                format!("{}:provider-ready", node.idempotency_key),
                Some(approval_receipt),
            )
            .map_err(store_error)?;
        }

        let recovered = store.recover().map_err(store_error)?;
        let project_id = recovered
            .events
            .iter()
            .find_map(|event| match &event.kind {
                RunEventKind::Planned { project_id, .. } => Some(project_id.clone()),
                _ => None,
            })
            .ok_or_else(|| ApiError::internal("run journal has no planned project identity"))?;
        let attached = find_attached_project(&state.workbench_root, &project_id)?;
        let project_root = state
            .workbench_root
            .join(attached.contract.workspace.root.as_str());
        let config_path = provider_config_path(&state.workbench_root);
        let environment = std::env::vars().collect::<BTreeMap<_, _>>();
        let adapter = HermesAdapter::load(&config_path, &project_root, &project_root, &environment)
            .map_err(|error| {
                ApiError::internal(format!(
                    "failed to load Workbench provider adapter at {}: {error}",
                    config_path.display()
                ))
            })?;
        let ready_node = graph
            .nodes
            .iter()
            .find(|candidate| candidate.id == node_id)
            .expect("provider node remains present")
            .clone();
        let check_commands = attached
            .contract
            .checks
            .iter()
            .map(|check| {
                let command = attached.contract.command(&check.command);
                let invocation = command
                    .map(|command| {
                        std::iter::once(command.program.as_str())
                            .chain(command.args.iter().map(String::as_str))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_else(|| check.command.clone());
                (check.id.clone(), invocation)
            })
            .collect::<BTreeMap<_, _>>();
        let declared_checks = check_commands
            .iter()
            .map(|(id, command)| format!("{id}: {command}"))
            .collect::<Vec<_>>();
        let task = HermesNodeTask {
            run_id: graph.run_id.clone(),
            node: ready_node,
            objective: request.objective.trim().to_string(),
            instructions: format!(
                "Work only inside the attached project root. Do not commit. Run every declared check and make no changes outside the objective. Declared checks: {}",
                declared_checks.join("; ")
            ),
            checks: attached
                .contract
                .checks
                .iter()
                .map(|check| check.id.clone())
                .collect(),
            check_commands,
            project_contract_digest: graph.provenance.project_contract_digest.clone(),
        };
        let receipt = adapter
            .execute(&task, AdapterCancellation::new())
            .await
            .map_err(|error| {
                ApiError::internal(format!("Workbench provider execution failed: {error}"))
            })?;
        let value = serde_json::to_value(&receipt).map_err(|error| {
            ApiError::internal(format!("failed to serialize provider receipt: {error}"))
        })?;
        store
            .write_execution_receipt(&node_id, &value)
            .map_err(store_error)?;
        receipt
    };

    finalize_provider_receipt(&store, &mut graph, &node_id, &receipt)?;
    let evidence = review_evidence_from_receipt(&receipt)?;
    project_review_evidence(
        &store,
        node_id.as_str(),
        &receipt.idempotency_key,
        &receipt.receipt_digest,
        Some(&evidence),
    )?;
    Ok(Json(ExecuteProviderNodeResponse {
        run: run_response(&store, graph)?,
        receipt,
    }))
}

fn provider_config_path(root: &std::path::Path) -> PathBuf {
    std::env::var_os("ARDA_HERMES_ADAPTER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("config/adapters/hermes-workbench.toml"))
}

fn require_succeeded_dependencies(graph: &RunGraph, node_id: &NodeId) -> Result<(), ApiError> {
    let complete = graph
        .edges
        .iter()
        .filter(|edge| edge.to == *node_id)
        .all(|edge| {
            graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.from)
                .is_some_and(|parent| {
                    parent.state == NodeState::Succeeded
                        && parent.output_digest == edge.parent_receipt
                        && edge.parent_receipt.is_some()
                })
        });
    if complete {
        Ok(())
    } else {
        Err(ApiError::conflict(format!(
            "node `{}` has incomplete approval dependencies",
            node_id.as_str()
        )))
    }
}

fn finalize_provider_receipt(
    store: &RunStore,
    graph: &mut RunGraph,
    node_id: &NodeId,
    receipt: &HermesExecutionReceipt,
) -> Result<(), ApiError> {
    let state = graph
        .nodes
        .iter()
        .find(|node| node.id == *node_id)
        .map(|node| node.state)
        .ok_or_else(|| ApiError::not_found("provider node disappeared during execution"))?;
    if state == NodeState::Ready {
        apply_transition_once(
            store,
            graph,
            node_id,
            NodeState::Running,
            format!("{}:provider-running", receipt.idempotency_key),
            Some(receipt.receipt_digest.clone()),
        )
        .map_err(store_error)?;
    }
    let current = graph
        .nodes
        .iter()
        .find(|node| node.id == *node_id)
        .map(|node| node.state)
        .expect("provider node remains present");
    let terminal = match receipt.status {
        HermesReceiptStatus::Succeeded => NodeState::Succeeded,
        HermesReceiptStatus::Failed => NodeState::Failed,
        HermesReceiptStatus::Cancelled => NodeState::Cancelled,
    };
    if current == NodeState::Running {
        apply_transition_once(
            store,
            graph,
            node_id,
            terminal,
            receipt.idempotency_key.clone(),
            Some(receipt.receipt_digest.clone()),
        )
        .map_err(store_error)?;
    } else if current != terminal {
        return Err(ApiError::conflict(format!(
            "provider receipt is terminal as {terminal:?}, but node is {current:?}"
        )));
    }
    Ok(())
}

fn review_evidence_from_receipt(
    receipt: &HermesExecutionReceipt,
) -> Result<RunReviewEvidence, ApiError> {
    let provider = receipt
        .usage
        .provider
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::internal("provider receipt omitted provider identity"))?;
    let model = receipt
        .usage
        .model
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::internal("provider receipt omitted model identity"))?;
    let tests = receipt
        .test_evidence
        .iter()
        .map(|test| TestEvidence {
            name: test.command.clone(),
            status: if test.exit_code == 0 {
                TestStatus::Passed
            } else {
                TestStatus::Failed
            },
            duration_ms: None,
            details: Some(format!(
                "check_id={} exit_code={} output_digest={}",
                test.check_id, test.exit_code, test.output_digest
            )),
        })
        .collect();
    Ok(RunReviewEvidence {
        changes: Vec::new(),
        tests,
        provider_receipt: Some(ProviderReceiptEvidence {
            provider,
            model,
            adapter: receipt.adapter.clone(),
            receipt_digest: receipt.receipt_digest.clone(),
            summary: receipt.summary.clone(),
        }),
    })
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

pub(super) async fn stream_run_events(
    State(state): State<HarnessState>,
    Path(id): Path<String>,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let (store, _) = load_run(&state, &id)?;
    let stream = async_stream::stream! {
        let mut next_sequence = 1_u64;
        let mut ticker = tokio::time::interval(Duration::from_millis(250));
        loop {
            ticker.tick().await;
            match store.recover() {
                Ok(recovered) => {
                    for run_event in recovered.events {
                        if run_event.sequence < next_sequence {
                            continue;
                        }
                        next_sequence = run_event.sequence.saturating_add(1);
                        match Event::default().event("run_event").json_data(&run_event) {
                            Ok(event) => yield Ok(event),
                            Err(error) => {
                                tracing::warn!("harness: run event serialization failed: {error}");
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!("harness: run event stream recovery failed: {error}");
                }
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10))))
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
    let review = store
        .read_result()
        .map_err(store_error)?
        .map(serde_json::from_value)
        .transpose()
        .map_err(store_error)?
        .unwrap_or_default();
    Ok(RunResponse {
        graph,
        events,
        review,
    })
}

fn project_review_evidence(
    store: &RunStore,
    node_id: &str,
    idempotency_key: &str,
    receipt_digest: &str,
    evidence: Option<&RunReviewEvidence>,
) -> Result<(), ApiError> {
    let Some(evidence) = evidence else {
        return Ok(());
    };
    validate_review_evidence(evidence, receipt_digest)?;
    let mut review: RunReviewEvidence = store
        .read_result()
        .map_err(store_error)?
        .map(serde_json::from_value)
        .transpose()
        .map_err(store_error)?
        .unwrap_or_default();
    for change in &evidence.changes {
        review.changes.retain(|current| current.path != change.path);
        review.changes.push(change.clone());
    }
    for test in &evidence.tests {
        review.tests.retain(|current| current.name != test.name);
        review.tests.push(test.clone());
    }
    if let Some(provider_receipt) = &evidence.provider_receipt {
        review.provider_receipt = Some(provider_receipt.clone());
    }
    store
        .write_result(&serde_json::to_value(&review).map_err(store_error)?)
        .map_err(store_error)?;
    let event_node_id = NodeId::new(node_id.to_owned())
        .map_err(|error| ApiError::bad_request(format!("invalid node id: {error}")))?;
    store
        .append(RunEventDraft {
            node_id: event_node_id,
            idempotency_key: format!("{idempotency_key}:review-projected"),
            kind: RunEventKind::ResultProjected,
            receipt_digest: Some(receipt_digest.to_owned()),
        })
        .map_err(store_error)?;
    Ok(())
}

fn validate_review_evidence(
    evidence: &RunReviewEvidence,
    receipt_digest: &str,
) -> Result<(), ApiError> {
    if evidence.changes.len() > MAX_REVIEW_ITEMS || evidence.tests.len() > MAX_REVIEW_ITEMS {
        return Err(ApiError::bad_request(format!(
            "review evidence cannot contain more than {MAX_REVIEW_ITEMS} changes or tests"
        )));
    }
    for change in &evidence.changes {
        let path = change.path.trim();
        let is_bounded_relative_path = !path.is_empty()
            && path.len() <= MAX_REVIEW_PATH_BYTES
            && FsPath::new(path)
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !is_bounded_relative_path {
            return Err(ApiError::bad_request(
                "change evidence path must be a bounded relative path without traversal",
            ));
        }
        if change
            .diff
            .as_ref()
            .is_some_and(|diff| diff.len() > MAX_REVIEW_TEXT_BYTES)
        {
            return Err(ApiError::bad_request("change evidence diff is too large"));
        }
    }
    for test in &evidence.tests {
        if test.name.trim().is_empty() || test.name.len() > MAX_REVIEW_PATH_BYTES {
            return Err(ApiError::bad_request(
                "test evidence name must be non-empty and bounded",
            ));
        }
        if test
            .details
            .as_ref()
            .is_some_and(|details| details.len() > MAX_REVIEW_TEXT_BYTES)
        {
            return Err(ApiError::bad_request("test evidence details are too large"));
        }
    }
    if let Some(provider_receipt) = &evidence.provider_receipt {
        if [
            &provider_receipt.provider,
            &provider_receipt.model,
            &provider_receipt.adapter,
            &provider_receipt.summary,
        ]
        .iter()
        .any(|value| value.trim().is_empty() || value.len() > MAX_REVIEW_TEXT_BYTES)
        {
            return Err(ApiError::bad_request(
                "provider receipt evidence fields must be non-empty and bounded",
            ));
        }
        validate_sha256_digest(&provider_receipt.receipt_digest, "provider receipt digest")?;
        if provider_receipt.receipt_digest != receipt_digest {
            return Err(ApiError::conflict(
                "provider review evidence does not match the completion receipt digest",
            ));
        }
    }
    Ok(())
}

fn validate_sha256_digest(value: &str, label: &str) -> Result<(), ApiError> {
    let valid = value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    });
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "{label} must be a lowercase SHA-256 digest"
        )))
    }
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
