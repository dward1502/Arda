use arda_core::run_graph::{
    NodeId, NodeKind, NodeState, RunGraph, RunId, WorkerRole, WorkerRouteClass,
};
use arda_vaire::ContextAssembly;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive},
    response::Sse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::adapters::{
    AdapterCancellation, CostMeasurement, HermesAdapter, HermesExecutionReceipt, HermesNodeTask,
    HermesReceiptStatus,
};
use crate::runs::{
    apply_transition_once, project_worker_progress, schedule_ready_workers, AppendOutcome,
    ResourceMeasurementSource, ResourceUsageDraft, RunEvent, RunEventDraft, RunEventKind, RunStore,
    WorkerAvailability, WorkerLimits, WorkerProgressState, WorkerUsage,
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

/// Live provider cancellation handles keyed by `run_id/node_id`.
///
/// The durable run journal remains authoritative across restart. This registry
/// exists only long enough to propagate an authenticated cancellation into the
/// currently running Hermes child process.
static ACTIVE_PROVIDER_CANCELLATIONS: LazyLock<Mutex<HashMap<String, AdapterCancellation>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ACTIVE_PROVIDER_ROUTES: LazyLock<Mutex<HashMap<String, WorkerRouteClass>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRunRequest {
    project_id: String,
    #[serde(default)]
    expected_project_contract_digest: Option<String>,
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
    #[serde(default)]
    context_assembly: Option<ContextAssembly>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
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
    worker_progress: BTreeMap<String, WorkerProgressState>,
    recovery_diagnostics: Option<RecoveryDiagnostics>,
}

#[derive(Debug, Serialize)]
pub struct RecoveryDiagnostics {
    failure_owner: String,
    failed_node_id: String,
    failure_reason: String,
    last_valid_state: Option<LastValidRunState>,
    safe_recovery_action: String,
    post_recovery_receipt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LastValidRunState {
    node_id: String,
    state: NodeState,
    receipt_digest: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunEventsResponse {
    events: Vec<RunEvent>,
}

#[derive(Debug, Serialize)]
pub struct RunListResponse {
    runs: Vec<RunResponse>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteProviderNodeResponse {
    run: RunResponse,
    receipt: HermesExecutionReceipt,
}

fn mark_current_run(root: &FsPath, run_id: &str) -> Result<(), ApiError> {
    let path = root.join(crate::operator_projection::CURRENT_RUNS_PATH);
    let mut run_ids = match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let value: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
                ApiError::internal(format!("failed to parse current-run registry: {error}"))
            })?;
            if value["schema_version"] != "arda.workbench.current-runs.v1" {
                return Err(ApiError::internal(
                    "unsupported current-run registry version",
                ));
            }
            value["run_ids"]
                .as_array()
                .ok_or_else(|| ApiError::internal("current-run registry requires run_ids"))?
                .iter()
                .filter_map(|value| value.as_str())
                .map(str::to_owned)
                .collect::<std::collections::BTreeSet<_>>()
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Default::default(),
        Err(error) => {
            return Err(ApiError::internal(format!(
                "failed to read current-run registry: {error}"
            )));
        }
    };
    run_ids.insert(run_id.to_owned());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            ApiError::internal(format!("failed to create current-run registry: {error}"))
        })?;
    }
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": "arda.workbench.current-runs.v1",
        "run_ids": run_ids,
    }))
    .map_err(|error| ApiError::internal(format!("serialize current-run registry: {error}")))?;
    std::fs::write(&temporary, bytes)
        .and_then(|_| std::fs::rename(&temporary, &path))
        .map_err(|error| ApiError::internal(format!("write current-run registry: {error}")))
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
    let attached_digest = contract_digest(&attached.contract)?;
    if request
        .expected_project_contract_digest
        .as_deref()
        .is_some_and(|expected| expected != attached_digest)
    {
        return Err(ApiError::conflict(format!(
            "project `{}` contract changed before run planning",
            request.project_id
        )));
    }

    let store =
        RunStore::open(&state.workbench_root, request.graph.run_id.clone()).map_err(store_error)?;
    let recovered = store.recover().map_err(store_error)?;
    if let Some(existing) = recovered.checkpoint {
        if recovered
            .applied_idempotency_keys
            .contains_key(&request.envelope.idempotency_key)
        {
            mark_current_run(&state.workbench_root, existing.run_id.as_str())?;
            return Ok((StatusCode::OK, Json(run_response(&store, existing)?)));
        }
        return Err(ApiError::conflict(format!(
            "run `{}` already exists",
            request.graph.run_id.as_str()
        )));
    }

    let mut graph = request.graph;
    graph.provenance.project_contract_digest = attached_digest;
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
        mark_current_run(&state.workbench_root, graph.run_id.as_str())?;
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
    if request.reason.trim().is_empty() {
        return Err(ApiError::bad_request("cancellation reason cannot be empty"));
    }
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    cancel_active_provider_run(&id).await;
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
    if graph
        .nodes
        .iter()
        .find(|node| node.id.as_str() == node_id)
        .is_some_and(|node| node.worker.is_some())
    {
        return Err(ApiError::conflict(format!(
            "provider-owned node `{node_id}` must complete through provider execution"
        )));
    }
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
        require_matching_projected_evidence(&store, request.evidence.as_ref())?;
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
    let is_verification = node.kind == NodeKind::Verify;
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
    let verification_failed = is_verification
        && request.evidence.as_ref().is_none_or(|evidence| {
            evidence.tests.is_empty()
                || evidence
                    .tests
                    .iter()
                    .any(|test| test.status != TestStatus::Passed)
        });
    apply_transition_once(
        &store,
        &mut graph,
        &node_id,
        if verification_failed {
            NodeState::Failed
        } else {
            NodeState::Succeeded
        },
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

fn provider_instructions(kind: NodeKind, declared_checks: &[String]) -> String {
    if kind == NodeKind::Review {
        format!(
            "Work only inside the attached project root. Do not commit or modify project files. Independently inspect the implementation and durable verification evidence without rerunning the declared checks, and report named defects. For an intermediate run-graph node, judge only this node's objective and evidence; do not require downstream whole-objective deliverables such as synthesis, repair backlogs, operator outcomes, or joined closure. Fail rather than approve unsupported completion. For read-only source evidence, exported tool output digests authenticate the actual calls and must not equal source content digests because they hash different envelopes. Treat absence of mutating tool calls under read-only authority as the no-modification evidence. Require a context_use_receipt only when supplied by the governed capsule. Declared checks already covered by the verification receipt: {}",
            declared_checks.join("; ")
        )
    } else if kind == NodeKind::Inspect {
        format!(
            "Work only inside the attached project root. Do not commit or modify project files. Use the file tools to read at least one relevant project file and return material file-tool evidence supporting the bounded inspection. Do not run the declared checks; the independent verifier owns project-native check execution. Declared checks reserved for verification: {}",
            declared_checks.join("; ")
        )
    } else {
        format!(
            "Work only inside the attached project root. Do not commit. Execute every declared check exactly as printed before any optional exploratory command, and make no changes outside the objective. In test_evidence, reference the terminal tool call that ran the exact declared command; never reference an exploratory command. If a declared check succeeds, do not substitute ls, pwd, or inspection output for its evidence. Declared checks: {}",
            declared_checks.join("; ")
        )
    }
}

fn dependency_receipt_instructions(
    store: &RunStore,
    graph: &RunGraph,
    node_id: &NodeId,
) -> Result<Option<String>, ApiError> {
    let node = graph
        .nodes
        .iter()
        .find(|candidate| candidate.id == *node_id)
        .ok_or_else(|| ApiError::internal("provider node disappeared while assembling evidence"))?;
    let mut receipts = Vec::new();
    for parent_digest in &node.parent_receipts {
        let Some(parent) = graph
            .nodes
            .iter()
            .find(|candidate| candidate.output_digest.as_deref() == Some(parent_digest.as_str()))
        else {
            continue;
        };
        let Some(value) = store
            .read_execution_receipt(&parent.id)
            .map_err(store_error)?
        else {
            continue;
        };
        let receipt: HermesExecutionReceipt = serde_json::from_value(value).map_err(|error| {
            ApiError::internal(format!(
                "stored parent execution receipt is invalid: {error}"
            ))
        })?;
        if receipt.receipt_digest != *parent_digest
            || !receipt.has_valid_digest().map_err(|error| {
                ApiError::internal(format!(
                    "stored parent execution receipt digest could not be checked: {error}"
                ))
            })?
        {
            return Err(ApiError::conflict(
                "stored parent execution receipt failed canonical digest verification",
            ));
        }
        receipts.push(serde_json::json!({
            "node_id": receipt.node_id,
            "receipt_digest": receipt.receipt_digest,
            "status": receipt.status,
            "summary": receipt.summary,
            "tool_evidence": receipt.tool_evidence,
            "test_evidence": receipt.test_evidence,
            "artifacts": receipt.artifacts,
        }));
    }
    if receipts.is_empty() {
        return Ok(None);
    }
    let payload = serde_json::to_string(&receipts).map_err(|error| {
        ApiError::internal(format!(
            "serialize canonical parent receipt evidence: {error}"
        ))
    })?;
    Ok(Some(format!(
        " Canonical parent execution receipt payloads (loaded from the durable run store and digest-validated before dispatch): {payload}"
    )))
}

fn provider_executes_declared_checks(kind: NodeKind) -> bool {
    !matches!(kind, NodeKind::Inspect | NodeKind::Review)
}

fn provider_usage_idempotency_key(run_id: &str, receipt_key: &str) -> String {
    format!("{run_id}:{receipt_key}:provider-usage")
}

fn provider_ready_idempotency_key(node_key: &str, attempt: u64) -> String {
    format!("{node_key}:provider-ready:{attempt}")
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

    // Serialize setup and terminal projection with the Workbench mutation lock,
    // but release it while Hermes runs so an authenticated cancel request can
    // propagate to the live child process.
    let mut _mutation_guard = Some(WORKBENCH_MUTATIONS.lock().await);
    let (mut store, mut graph) = load_run(&state, &id)?;
    let node_id = NodeId::new(node_id)
        .map_err(|error| ApiError::bad_request(format!("invalid node id: {error}")))?;
    let mut node = graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| ApiError::not_found(format!("node `{}` was not found", node_id.as_str())))?
        .clone();
    if !is_provider_execution_kind(node.kind) {
        return Err(ApiError::conflict(format!(
            "node `{}` is not an inspect, execute, verify, or review provider worker",
            node_id.as_str()
        )));
    }
    if node.kind == NodeKind::Review
        && !node.worker.as_ref().is_some_and(|worker| {
            matches!(
                worker.role,
                WorkerRole::SecurityPrivacyCritic | WorkerRole::ImplementationRiskCritic
            )
        })
    {
        return Err(ApiError::conflict(format!(
            "review node `{}` requires an independent critic worker",
            node_id.as_str()
        )));
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
    let attached_digest = contract_digest(&attached.contract)?;
    require_current_project_contract(
        &project_id,
        &graph.provenance.project_contract_digest,
        &attached_digest,
    )?;
    let project_root = state
        .workbench_root
        .join(attached.contract.workspace.root.as_str());

    require_succeeded_dependencies(&graph, &node_id)?;
    enforce_worker_admission(&state, &store, &graph, &node_id).await?;
    let approval_receipt = node
        .parent_receipts
        .first()
        .cloned()
        .ok_or_else(|| ApiError::conflict("provider execution requires an approval receipt"))?;
    let config_path = provider_config_path(&state.workbench_root);
    let environment = std::env::vars().collect::<BTreeMap<_, _>>();
    let adapter = HermesAdapter::load(&config_path, &project_root, &project_root, &environment)
        .map_err(|error| {
            ApiError::internal(format!(
                "failed to load Workbench provider adapter at {}: {error}",
                config_path.display()
            ))
        })?;
    let mut ready_node = graph
        .nodes
        .iter()
        .find(|candidate| candidate.id == node_id)
        .expect("provider node remains present")
        .clone();
    ready_node.state = NodeState::Ready;
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
    let mut instructions = provider_instructions(ready_node.kind, &declared_checks);
    if let Some(receipt_instructions) = dependency_receipt_instructions(&store, &graph, &node_id)? {
        instructions.push_str(&receipt_instructions);
    }
    let task = HermesNodeTask {
        run_id: graph.run_id.clone(),
        node: ready_node,
        objective: request.objective.trim().to_string(),
        instructions,
        checks: if provider_executes_declared_checks(node.kind) {
            attached
                .contract
                .checks
                .iter()
                .map(|check| check.id.clone())
                .collect()
        } else {
            Vec::new()
        },
        check_commands: if provider_executes_declared_checks(node.kind) {
            check_commands
        } else {
            BTreeMap::new()
        },
        project_contract_digest: graph.provenance.project_contract_digest.clone(),
        context_assembly: request.context_assembly.clone(),
    };
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
        if receipt.project_contract_digest != task.project_contract_digest
            || receipt.parent_receipts != task.node.parent_receipts
        {
            return Err(ApiError::conflict(
                "stored provider receipt does not match current contract and parent authority",
            ));
        }
        let requested_context = request.context_assembly.as_ref().map(|assembly| {
            (
                assembly.capsule.capsule_id.as_str(),
                assembly.capsule.capsule_digest.as_str(),
                assembly.use_receipt.receipt_ref(),
            )
        });
        let receipt_context = match (
            receipt.context_capsule_id.as_deref(),
            receipt.context_capsule_digest.as_deref(),
            receipt.context_use_receipt_ref.as_deref(),
        ) {
            (None, None, None) => None,
            (Some(id), Some(digest), Some(use_receipt_ref)) => {
                Some((id, digest, use_receipt_ref.to_string()))
            }
            _ => {
                return Err(ApiError::conflict(
                    "stored provider receipt has incomplete context authority",
                ));
            }
        };
        if requested_context != receipt_context {
            return Err(ApiError::conflict(
                "stored provider receipt does not match the requested context capsule authority",
            ));
        }
        adapter
            .validate_stored_receipt_authority(&task, &receipt)
            .map_err(|error| {
                ApiError::conflict(format!(
                    "stored provider receipt failed current authority binding: {error}"
                ))
            })?;
        adapter.preflight(&task).map_err(|error| {
            ApiError::conflict(format!("provider task failed bounded preflight: {error}"))
        })?;
        receipt
    } else {
        adapter.preflight(&task).map_err(|error| {
            ApiError::conflict(format!("provider task failed bounded preflight: {error}"))
        })?;
        let cancellation_key = format!("{id}/{}", node_id.as_str());
        if node.state == NodeState::Running {
            if ACTIVE_PROVIDER_CANCELLATIONS
                .lock()
                .await
                .contains_key(&cancellation_key)
            {
                return Err(ApiError::conflict(format!(
                    "node `{}` already has an active provider worker",
                    node_id.as_str()
                )));
            } else {
                apply_transition_once(
                    &store,
                    &mut graph,
                    &node_id,
                    NodeState::Failed,
                    format!(
                        "{}:provider-restart-death:{}",
                        node.idempotency_key, node.checkpoint.sequence
                    ),
                    node.input_digest.clone(),
                )
                .map_err(store_error)?;
                node.state = NodeState::Failed;
            }
        }
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
        let attempt = graph
            .nodes
            .iter()
            .find(|candidate| candidate.id == node_id)
            .map(|candidate| candidate.checkpoint.sequence + 1)
            .expect("provider node remains present");
        if attempt > u64::from(node.retry.max_attempts) {
            return Err(ApiError::conflict(format!(
                "node `{}` exhausted provider attempts",
                node_id.as_str()
            )));
        }
        if node.state != NodeState::Ready {
            apply_transition_once(
                &store,
                &mut graph,
                &node_id,
                NodeState::Ready,
                provider_ready_idempotency_key(&node.idempotency_key, attempt),
                Some(approval_receipt),
            )
            .map_err(store_error)?;
        }
        graph
            .nodes
            .iter_mut()
            .find(|candidate| candidate.id == node_id)
            .expect("provider node remains present")
            .checkpoint
            .sequence = attempt;
        apply_transition_once(
            &store,
            &mut graph,
            &node_id,
            NodeState::Running,
            format!("{}:provider-running:{attempt}", node.idempotency_key),
            node.input_digest.clone(),
        )
        .map_err(store_error)?;
        let cancellation = AdapterCancellation::new();
        ACTIVE_PROVIDER_CANCELLATIONS
            .lock()
            .await
            .insert(cancellation_key.clone(), cancellation.clone());
        if let Some(worker) = node.worker.as_ref() {
            ACTIVE_PROVIDER_ROUTES
                .lock()
                .await
                .insert(cancellation_key.clone(), worker.route_class);
        }
        drop(_mutation_guard.take());

        let execution = adapter.execute(&task, cancellation).await;
        ACTIVE_PROVIDER_CANCELLATIONS
            .lock()
            .await
            .remove(&cancellation_key);
        ACTIVE_PROVIDER_ROUTES
            .lock()
            .await
            .remove(&cancellation_key);
        _mutation_guard = Some(WORKBENCH_MUTATIONS.lock().await);

        // Cancellation may have updated the journal while the child was
        // running. Reload so that durable terminal state wins over a late child
        // result and cannot be overwritten by provider finalization.
        let (reloaded_store, reloaded_graph) = load_run(&state, &id)?;
        store = reloaded_store;
        graph = reloaded_graph;
        if graph
            .nodes
            .iter()
            .any(|candidate| candidate.state == NodeState::Cancelled)
        {
            return Err(ApiError::conflict(format!(
                "run `{id}` was cancelled while provider execution was active"
            )));
        }
        let receipt = match execution {
            Ok(receipt) => receipt,
            Err(error) => {
                let current = graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == node_id)
                    .cloned()
                    .ok_or_else(|| ApiError::internal("provider node disappeared after failure"))?;
                if current.state == NodeState::Running {
                    apply_transition_once(
                        &store,
                        &mut graph,
                        &node_id,
                        NodeState::Failed,
                        format!(
                            "{}:provider-error:{}",
                            current.idempotency_key, current.checkpoint.sequence
                        ),
                        None,
                    )
                    .map_err(store_error)?;
                }
                return Err(ApiError::internal(format!(
                    "Workbench provider execution failed: {error}"
                )));
            }
        };
        let value = serde_json::to_value(&receipt).map_err(|error| {
            ApiError::internal(format!("failed to serialize provider receipt: {error}"))
        })?;
        store
            .write_execution_receipt(&node_id, &value)
            .map_err(store_error)?;
        store
            .append_resource_usage(ResourceUsageDraft {
                idempotency_key: provider_usage_idempotency_key(&id, &receipt.idempotency_key),
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
            })
            .map_err(|error| {
                ApiError::internal(format!("resource usage persistence failed: {error}"))
            })?;
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

async fn cancel_active_provider_run(run_id: &str) {
    let prefix = format!("{run_id}/");
    let active = ACTIVE_PROVIDER_CANCELLATIONS
        .lock()
        .await
        .iter()
        .filter(|(key, _)| key.starts_with(&prefix))
        .map(|(_, cancellation)| cancellation.clone())
        .collect::<Vec<_>>();
    for cancellation in active {
        cancellation.cancel();
    }
}

async fn enforce_worker_admission(
    state: &HarnessState,
    store: &RunStore,
    graph: &RunGraph,
    node_id: &NodeId,
) -> Result<(), ApiError> {
    let Some(worker) = graph
        .nodes
        .iter()
        .find(|node| node.id == *node_id)
        .and_then(|node| node.worker.as_ref())
    else {
        return Ok(());
    };
    let config_path = std::env::var_os("ARDA_WORKER_LIMITS_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            state
                .workbench_root
                .join("config/runtime/worker_orchestration.toml")
        });
    let limits = match std::fs::read_to_string(&config_path) {
        Ok(raw) => WorkerLimits::from_toml_str(&raw).map_err(|error| {
            ApiError::internal(format!(
                "invalid worker admission policy at {}: {error}",
                config_path.display()
            ))
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => WorkerLimits::default(),
        Err(error) => {
            return Err(ApiError::internal(format!(
                "worker admission policy could not be read at {}: {error}",
                config_path.display()
            )));
        }
    };
    let active_routes = ACTIVE_PROVIDER_ROUTES.lock().await;
    let active_total_workers = active_routes.len();
    let active_local_workers = active_routes
        .values()
        .filter(|route| **route == WorkerRouteClass::Local)
        .count();
    let active_hosted_workers = active_routes
        .values()
        .filter(|route| **route == WorkerRouteClass::Hosted)
        .count();
    drop(active_routes);

    let now_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let daily = store
        .resource_rollup_since(now_unix_ms.saturating_sub(86_400_000), None)
        .map_err(|error| ApiError::internal(format!("daily resource rollup failed: {error}")))?;
    let mut spent_cost_usd = 0.0;
    for candidate in &graph.nodes {
        let Some(value) = store
            .read_execution_receipt(&candidate.id)
            .map_err(store_error)?
        else {
            continue;
        };
        let receipt: HermesExecutionReceipt = serde_json::from_value(value).map_err(|error| {
            ApiError::internal(format!("stored provider receipt is invalid: {error}"))
        })?;
        spent_cost_usd += receipt.usage.estimated_cost_usd;
    }
    let usage = WorkerUsage {
        active_total_workers,
        active_local_workers,
        active_hosted_workers,
        spent_cost_usd,
        spent_joules: 0.0,
        cycle_spent_cost_usd: spent_cost_usd,
        cycle_spent_joules: 0.0,
        daily_spent_cost_usd: daily.hosted_cost_usd,
    };
    let mut availability = WorkerAvailability::default();
    if worker.route_class == WorkerRouteClass::Local {
        let target = format!("{}/healthz", state.manwe_url.trim_end_matches('/'));
        availability.local_worker_available = state
            .client
            .get(target)
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success());
    }
    availability.local_thermal_ok = std::env::var("ARDA_LOCAL_THERMAL_OK")
        .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    if let Ok(routes) = std::env::var("ARDA_DEGRADED_WORKER_ROUTES") {
        availability.degraded_routes = routes
            .split(',')
            .map(str::trim)
            .filter(|route| !route.is_empty())
            .map(str::to_string)
            .collect();
    }
    let decision = schedule_ready_workers(graph, &limits, &usage, &availability, now_unix_ms);
    if decision.selected.iter().any(|selected| selected == node_id) {
        return Ok(());
    }
    let reason = decision
        .blocked
        .iter()
        .chain(decision.queued.iter())
        .find(|blocked| blocked.node_id == *node_id)
        .map(|blocked| format!("{:?}", blocked.reason))
        .unwrap_or_else(|| "not selected by deterministic scheduler".into());
    Err(ApiError::conflict(format!(
        "worker `{}` was not admitted: {reason}",
        node_id.as_str()
    )))
}

#[cfg(test)]
mod active_provider_cancellation_tests {
    use super::*;

    #[tokio::test]
    async fn run_cancellation_signals_registered_provider_child() {
        let cancellation = AdapterCancellation::new();
        let mut signal = cancellation.subscribe();
        ACTIVE_PROVIDER_CANCELLATIONS
            .lock()
            .await
            .insert("run-live/execute".into(), cancellation);

        cancel_active_provider_run("run-live").await;
        signal.changed().await.expect("cancellation signal");
        assert!(*signal.borrow());

        ACTIVE_PROVIDER_CANCELLATIONS
            .lock()
            .await
            .remove("run-live/execute");
    }

    #[test]
    fn provider_execution_rejects_project_contract_replacement_after_planning() {
        let result = require_current_project_contract(
            "550e8400-e29b-41d4-a716-446655440000",
            "sha256:planned",
            "sha256:replacement",
        );

        assert!(result.is_err());
    }

    #[test]
    fn provider_execution_accepts_read_only_inspection_workers() {
        assert!(is_provider_execution_kind(NodeKind::Inspect));
    }
}

fn is_provider_execution_kind(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Inspect | NodeKind::Execute | NodeKind::Verify | NodeKind::Review
    )
}

fn require_current_project_contract(
    project_id: &str,
    planned_digest: &str,
    live_digest: &str,
) -> Result<(), ApiError> {
    if live_digest == planned_digest {
        Ok(())
    } else {
        Err(ApiError::conflict(format!(
            "project `{project_id}` contract changed after run planning"
        )))
    }
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

pub(super) async fn list_runs(
    State(state): State<HarnessState>,
) -> Result<Json<RunListResponse>, ApiError> {
    let root = state.workbench_root.join("data/runs");
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Json(RunListResponse { runs: Vec::new() }));
        }
        Err(error) => {
            return Err(ApiError::internal(format!(
                "failed to list run journals at {}: {error}",
                root.display()
            )));
        }
    };
    let mut ids = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("checkpoint.json").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|id| validate_run_id(id).is_ok())
        .collect::<Vec<_>>();
    ids.sort();
    let mut runs = Vec::with_capacity(ids.len());
    for id in ids {
        let (store, graph) = load_run(&state, &id)?;
        runs.push(run_response(&store, graph)?);
    }
    Ok(Json(RunListResponse { runs }))
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
    let worker_progress = project_worker_progress(&graph);
    let recovery_diagnostics = project_recovery_diagnostics(&graph, &events, &review);
    Ok(RunResponse {
        graph,
        events,
        review,
        worker_progress,
        recovery_diagnostics,
    })
}

fn project_recovery_diagnostics(
    graph: &RunGraph,
    events: &[RunEvent],
    review: &RunReviewEvidence,
) -> Option<RecoveryDiagnostics> {
    let failure_event = events.iter().rev().find(|event| {
        matches!(
            event.kind,
            RunEventKind::NodeTransition {
                state: NodeState::Failed
            }
        )
    })?;
    let failed_node = graph
        .nodes
        .iter()
        .find(|node| node.id == failure_event.node_id)?;
    let failed_index = graph
        .nodes
        .iter()
        .position(|node| node.id == failed_node.id)?;
    let last_valid_state = graph.nodes[..failed_index]
        .iter()
        .rev()
        .find(|node| node.state == NodeState::Succeeded)
        .map(|node| LastValidRunState {
            node_id: node.id.as_str().to_owned(),
            state: node.state,
            receipt_digest: node.output_digest.clone(),
        });
    let failure_reason = match failed_node.kind {
        NodeKind::Verify => review
            .tests
            .iter()
            .find(|test| test.status == TestStatus::Failed)
            .and_then(|test| test.details.clone())
            .map(|details| format!("Project-native verification failed: {details}"))
            .unwrap_or_else(|| {
                "Project-native verification did not produce passing evidence.".into()
            }),
        NodeKind::Execute => review
            .provider_receipt
            .as_ref()
            .map(|receipt| receipt.summary.clone())
            .unwrap_or_else(|| {
                "Execution failed before a successful provider receipt was recorded.".into()
            }),
        _ => format!(
            "The {} node entered the failed state.",
            failed_node.id.as_str()
        ),
    };
    let safe_recovery_action = match failed_node.kind {
        NodeKind::Verify => "Correct the failing project check, then retry verification; review remains blocked until passing evidence is durable.",
        NodeKind::Execute => "Inspect the provider receipt and retry only within the node's declared attempt and approval bounds.",
        _ => "Inspect the durable run events and retry only through the node's declared authority boundary.",
    }
    .to_owned();
    let post_recovery_receipt = (failed_node.state == NodeState::Succeeded)
        .then(|| failed_node.output_digest.clone())
        .flatten();

    Some(RecoveryDiagnostics {
        failure_owner: format!("arda-engine/workbench.{}", node_kind_name(failed_node.kind)),
        failed_node_id: failed_node.id.as_str().to_owned(),
        failure_reason,
        last_valid_state,
        safe_recovery_action,
        post_recovery_receipt,
    })
}

fn node_kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Inspect => "inspect",
        NodeKind::Retrieve => "retrieve",
        NodeKind::Research => "research",
        NodeKind::Plan => "plan",
        NodeKind::Approval => "approval",
        NodeKind::Execute => "execute",
        NodeKind::Verify => "verify",
        NodeKind::Review => "review",
        NodeKind::Compensate => "compensate",
        NodeKind::Close => "close",
    }
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

fn require_matching_projected_evidence(
    store: &RunStore,
    evidence: Option<&RunReviewEvidence>,
) -> Result<(), ApiError> {
    let Some(evidence) = evidence else {
        return Ok(());
    };
    let projected: RunReviewEvidence = store
        .read_result()
        .map_err(store_error)?
        .map(serde_json::from_value)
        .transpose()
        .map_err(store_error)?
        .unwrap_or_default();
    let changes_match = evidence.changes.iter().all(|expected| {
        projected
            .changes
            .iter()
            .any(|current| current.path == expected.path && current == expected)
    });
    let tests_match = evidence.tests.iter().all(|expected| {
        projected
            .tests
            .iter()
            .any(|current| current.name == expected.name && current == expected)
    });
    let provider_matches = evidence
        .provider_receipt
        .as_ref()
        .is_none_or(|expected| projected.provider_receipt.as_ref() == Some(expected));
    if !changes_match || !tests_match || !provider_matches {
        return Err(ApiError::conflict(
            "idempotent completion replay evidence differs from the durable projected evidence",
        ));
    }
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
    use super::{
        provider_executes_declared_checks, provider_instructions, provider_ready_idempotency_key,
        provider_usage_idempotency_key, validate_run_id,
    };
    use arda_core::run_graph::NodeKind;

    #[test]
    fn provider_usage_is_scoped_to_the_run() {
        assert_eq!(
            provider_usage_idempotency_key("run-b", "task-execute"),
            "run-b:task-execute:provider-usage"
        );
    }

    #[test]
    fn provider_ready_transition_is_scoped_to_the_attempt() {
        assert_ne!(
            provider_ready_idempotency_key("task-execute", 1),
            provider_ready_idempotency_key("task-execute", 2)
        );
    }

    #[test]
    fn read_only_inspection_instructions_require_file_evidence_without_terminal_checks() {
        let instructions = provider_instructions(NodeKind::Inspect, &["test: cargo test".into()]);

        assert!(instructions.contains("read at least one relevant project file"));
        assert!(instructions.contains("material file-tool evidence"));
        assert!(!instructions.contains("Execute every declared check"));
        assert!(!provider_executes_declared_checks(NodeKind::Inspect));
    }

    #[test]
    fn review_instructions_interpret_read_only_source_evidence_without_false_digest_equality() {
        let instructions = provider_instructions(NodeKind::Review, &[]);

        assert!(instructions.contains("must not equal source content digests"));
        assert!(instructions.contains("absence of mutating tool calls"));
        assert!(instructions.contains("context_use_receipt only when supplied"));
        assert!(instructions.contains("judge only this node's objective and evidence"));
    }

    #[test]
    fn run_ids_cannot_escape_the_run_store() {
        assert!(validate_run_id("run-1").is_ok());
        assert!(validate_run_id("../outside").is_err());
        assert!(validate_run_id("run/child").is_err());
    }
}
