use futures_util::StreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Emitter;

const DEFAULT_HARNESS_URL: &str = "http://127.0.0.1:7878";
pub const WORKBENCH_RUN_EVENT: &str = "arda://workbench-run-event";
pub const WORKBENCH_STREAM_ERROR: &str = "arda://workbench-stream-error";

#[derive(Default)]
pub struct WorkbenchEventStreamState {
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectValidation {
    pub valid: bool,
    pub project_id: Option<String>,
    pub root: Option<String>,
    pub effective_permissions: Vec<String>,
    pub provider_posture: Option<String>,
    pub project_checks: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskApproval {
    pub schema_version: String,
    pub proposal_id: String,
    pub approval_id: String,
    pub ledger_writes: Vec<String>,
    pub decision: String,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationEnvelope {
    pub approval: TaskApproval,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedProject {
    pub contract: Value,
    pub approval_id: String,
    pub proposal_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_joules: f64,
    pub max_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub sequence: u64,
    pub recovery_token: Option<String>,
    pub checkpoint_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunNode {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub authority: String,
    pub budget: Budget,
    pub retry: RetryPolicy,
    pub timeout_ms: u64,
    pub idempotency_key: String,
    pub input_digest: Option<String>,
    pub output_digest: Option<String>,
    pub parent_receipts: Vec<String>,
    pub checkpoint: CheckpointMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub parent_receipt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunProvenance {
    pub project_contract_digest: String,
    pub created_by: String,
    pub parent_receipts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunGraph {
    pub schema_version: String,
    pub run_id: String,
    pub objective_id: String,
    pub nodes: Vec<RunNode>,
    pub edges: Vec<RunEdge>,
    pub provenance: RunProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub graph: RunGraph,
    pub events: Vec<Value>,
    pub review: RunReviewEvidence,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunReviewEvidence {
    pub changes: Vec<ChangeEvidence>,
    pub tests: Vec<TestEvidence>,
    pub provider_receipt: Option<ProviderReceiptEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvidence {
    pub path: String,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEvidence {
    pub name: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderReceiptEvidence {
    pub provider: String,
    pub model: String,
    pub adapter: String,
    pub receipt_digest: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRunRequest {
    pub project_id: String,
    pub graph: RunGraph,
    pub envelope: MutationEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveRunRequest {
    pub run_id: String,
    pub node_id: String,
    pub envelope: MutationEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRunNodeRequest {
    pub run_id: String,
    pub node_id: String,
    pub receipt_digest: String,
    pub envelope: MutationEnvelope,
    pub evidence: Option<RunReviewEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteProviderNodeRequest {
    pub run_id: String,
    pub node_id: String,
    pub objective: String,
    pub envelope: MutationEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteProviderNodeResponse {
    pub run: RunRecord,
    pub receipt: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRunRequest {
    pub run_id: String,
    pub reason: String,
    pub envelope: MutationEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEventsResponse {
    pub events: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct HarnessValidation {
    valid: bool,
    project_id: String,
}

#[derive(Serialize)]
struct ContractRequest<'a> {
    contract: &'a Value,
}

#[derive(Serialize)]
struct AttachRequest<'a> {
    contract: &'a Value,
    envelope: &'a MutationEnvelope,
}

#[derive(Serialize)]
struct ApproveRequest<'a> {
    node_id: &'a str,
    envelope: &'a MutationEnvelope,
}

#[derive(Serialize)]
struct CompleteRequest<'a> {
    receipt_digest: &'a str,
    envelope: &'a MutationEnvelope,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<&'a RunReviewEvidence>,
}

#[derive(Serialize)]
struct ExecuteProviderRequest<'a> {
    objective: &'a str,
    envelope: &'a MutationEnvelope,
}

#[derive(Serialize)]
struct CancelRequest<'a> {
    reason: &'a str,
    envelope: &'a MutationEnvelope,
}

fn harness_url() -> String {
    std::env::var("ARDA_HARNESS_URL")
        .unwrap_or_else(|_| DEFAULT_HARNESS_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn endpoint(path: &str) -> String {
    format!("{}{path}", harness_url())
}

fn checked_id<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(value)
}

fn read_contract(path: &str) -> Result<(PathBuf, Value), String> {
    let path = PathBuf::from(path);
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read project contract: {error}"))?;
    let contract = serde_json::from_str(&raw)
        .map_err(|error| format!("Project contract is not valid JSON: {error}"))?;
    Ok((path, contract))
}

fn strings_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn contract_projection(path: &Path, contract: &Value) -> ProjectValidation {
    let mut errors = Vec::new();
    let schema = contract
        .get("schema_version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if schema != "arda.project-contract.v1" {
        errors.push("schema_version must be arda.project-contract.v1".to_string());
    }

    let project_id = contract
        .pointer("/identity/project_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    if project_id.is_none() {
        errors.push("identity.project_id is required".to_string());
    }

    let root = contract
        .pointer("/workspace/root")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    if root.is_none() {
        errors.push("workspace.root is required".to_string());
    }

    let effective_permissions = contract
        .get("permissions")
        .and_then(Value::as_object)
        .map(|permissions| {
            permissions
                .iter()
                .map(|(name, value)| {
                    format!(
                        "{name}:{}",
                        match value {
                            Value::String(value) => value.clone(),
                            Value::Bool(value) => value.to_string(),
                            Value::Object(value) => value
                                .get("allow")
                                .or_else(|| value.get("write"))
                                .map(Value::to_string)
                                .unwrap_or_else(|| "configured".to_string()),
                            _ => "configured".to_string(),
                        }
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let provider_posture = contract
        .pointer("/provider_posture/mode")
        .or_else(|| contract.get("provider_posture"))
        .or_else(|| contract.pointer("/runtime/adapter"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let project_checks = contract
        .get("checks")
        .and_then(Value::as_array)
        .map(|checks| {
            checks
                .iter()
                .filter_map(|check| check.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| strings_at(contract, "/project_checks"));

    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        errors.push("project contract must be a JSON file".to_string());
    }

    ProjectValidation {
        valid: errors.is_empty(),
        project_id,
        root,
        effective_permissions,
        provider_posture,
        project_checks,
        errors,
    }
}

async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
    let status = response.status();
    if status.is_success() {
        return response
            .json::<T>()
            .await
            .map_err(|error| format!("Harness returned an invalid response: {error}"));
    }
    let detail = response.text().await.unwrap_or_default();
    Err(format!("Harness request failed ({status}): {detail}"))
}

async fn post_json<B: Serialize + ?Sized, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let response = reqwest::Client::new()
        .post(endpoint(path))
        .json(body)
        .send()
        .await
        .map_err(|error| format!("Unable to reach the ARDA harness: {error}"))?;
    decode(response).await
}

async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let response = reqwest::Client::new()
        .get(endpoint(path))
        .send()
        .await
        .map_err(|error| format!("Unable to reach the ARDA harness: {error}"))?;
    decode(response).await
}

#[tauri::command]
pub async fn validate_project_contract(path: String) -> Result<ProjectValidation, String> {
    let (path_buf, contract) = read_contract(&path)?;
    let mut projection = contract_projection(&path_buf, &contract);
    if !projection.valid {
        return Ok(projection);
    }

    let response: HarnessValidation = post_json(
        "/v1/projects/validate",
        &ContractRequest {
            contract: &contract,
        },
    )
    .await?;
    projection.valid = response.valid;
    projection.project_id = Some(response.project_id);
    Ok(projection)
}

#[tauri::command]
pub async fn attach_project_contract(
    path: String,
    envelope: MutationEnvelope,
) -> Result<AttachedProject, String> {
    let (_, contract) = read_contract(&path)?;
    post_json(
        "/v1/projects/attach",
        &AttachRequest {
            contract: &contract,
            envelope: &envelope,
        },
    )
    .await
}

#[tauri::command]
pub async fn plan_workbench_run(request: PlanRunRequest) -> Result<RunRecord, String> {
    post_json("/v1/runs/plan", &request).await
}

#[tauri::command]
pub async fn approve_workbench_run(request: ApproveRunRequest) -> Result<RunRecord, String> {
    let run_id = checked_id(&request.run_id, "run_id")?;
    post_json(
        &format!("/v1/runs/{run_id}/approve"),
        &ApproveRequest {
            node_id: &request.node_id,
            envelope: &request.envelope,
        },
    )
    .await
}

#[tauri::command]
pub async fn complete_workbench_run_node(
    request: CompleteRunNodeRequest,
) -> Result<RunRecord, String> {
    let run_id = checked_id(&request.run_id, "run_id")?;
    let node_id = checked_id(&request.node_id, "node_id")?;
    if request.receipt_digest.trim().is_empty() {
        return Err("receipt_digest is required".to_string());
    }
    let body = CompleteRequest {
        receipt_digest: &request.receipt_digest,
        envelope: &request.envelope,
        evidence: request.evidence.as_ref(),
    };
    post_json(
        &format!("/v1/runs/{run_id}/nodes/{node_id}/complete"),
        &body,
    )
    .await
}

#[tauri::command]
pub async fn execute_workbench_provider_node(
    request: ExecuteProviderNodeRequest,
) -> Result<ExecuteProviderNodeResponse, String> {
    let run_id = checked_id(&request.run_id, "run_id")?;
    let node_id = checked_id(&request.node_id, "node_id")?;
    if request.objective.trim().is_empty() {
        return Err("provider objective is required".to_string());
    }
    post_json(
        &format!("/v1/runs/{run_id}/nodes/{node_id}/execute-provider"),
        &ExecuteProviderRequest {
            objective: request.objective.trim(),
            envelope: &request.envelope,
        },
    )
    .await
}

#[tauri::command]
pub async fn cancel_workbench_run(request: CancelRunRequest) -> Result<RunRecord, String> {
    let run_id = checked_id(&request.run_id, "run_id")?;
    post_json(
        &format!("/v1/runs/{run_id}/cancel"),
        &CancelRequest {
            reason: &request.reason,
            envelope: &request.envelope,
        },
    )
    .await
}

#[tauri::command]
pub async fn get_workbench_run(run_id: String) -> Result<RunRecord, String> {
    let run_id = checked_id(&run_id, "run_id")?;
    get_json(&format!("/v1/runs/{run_id}")).await
}

#[tauri::command]
pub async fn get_workbench_run_events(run_id: String) -> Result<RunEventsResponse, String> {
    let run_id = checked_id(&run_id, "run_id")?;
    get_json(&format!("/v1/runs/{run_id}/events")).await
}

fn take_sse_events(buffer: &mut Vec<u8>) -> Vec<Value> {
    let mut events = Vec::new();
    while let Some(end) = buffer.windows(2).position(|window| window == b"\n\n") {
        let frame: Vec<u8> = buffer.drain(..end + 2).collect();
        let Ok(frame) = std::str::from_utf8(&frame) else {
            continue;
        };
        let data = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        if let Ok(event) = serde_json::from_str(&data) {
            events.push(event);
        }
    }
    events
}

#[tauri::command]
pub async fn start_workbench_run_event_stream(
    app: tauri::AppHandle,
    state: tauri::State<'_, WorkbenchEventStreamState>,
    run_id: String,
) -> Result<(), String> {
    let run_id = checked_id(&run_id, "run_id")?.to_string();
    let response = reqwest::Client::new()
        .get(endpoint(&format!("/v1/runs/{run_id}/events/stream")))
        .send()
        .await
        .map_err(|error| format!("Unable to reach the ARDA run event stream: {error}"))?
        .error_for_status()
        .map_err(|error| format!("ARDA run event stream rejected the request: {error}"))?;
    let mut stream = response.bytes_stream();
    let task = tauri::async_runtime::spawn(async move {
        let mut buffer = Vec::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk) => {
                    buffer.extend_from_slice(&chunk);
                    for event in take_sse_events(&mut buffer) {
                        if app.emit(WORKBENCH_RUN_EVENT, event).is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    let _ = app.emit(
                        WORKBENCH_STREAM_ERROR,
                        serde_json::json!({"runId": run_id, "error": error.to_string()}),
                    );
                    return;
                }
            }
        }
    });

    let mut current = state
        .task
        .lock()
        .map_err(|_| "workbench event stream lock was poisoned".to_string())?;
    if let Some(previous) = current.replace(task) {
        previous.abort();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_fixture(value: &Value) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("arda-workbench-{unique}.json"));
        fs::write(&path, serde_json::to_vec(value).expect("fixture JSON")).expect("fixture write");
        path
    }

    #[test]
    fn projects_validation_contract_before_harness_submission() {
        let path = write_fixture(&json!({
            "schema_version": "arda.project-contract.v1",
            "identity": {"project_id": "project-1"},
            "workspace": {"root": "/tmp/project"},
            "permissions": {"authority": "approval_required", "network": {"allow": false}},
            "runtime": {"adapter": "cargo"},
            "checks": [{"id": "test", "command": "test"}]
        }));
        let (_, contract) = read_contract(path.to_str().expect("path string")).expect("contract");
        let result = contract_projection(&path, &contract);
        fs::remove_file(path).expect("fixture cleanup");

        assert!(result.valid);
        assert_eq!(result.project_id.as_deref(), Some("project-1"));
        assert!(result
            .effective_permissions
            .contains(&"authority:approval_required".to_string()));
        assert!(result
            .effective_permissions
            .contains(&"network:false".to_string()));
        assert_eq!(result.provider_posture.as_deref(), Some("cargo"));
        assert_eq!(result.project_checks, ["test"]);
    }

    #[test]
    fn rejects_missing_identity_before_any_mutation() {
        let path = Path::new("contract.json");
        let result =
            contract_projection(path, &json!({"schema_version": "arda.project-contract.v1"}));
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("project_id")));
        assert!(result.errors.iter().any(|error| error.contains("root")));
    }

    #[test]
    fn run_route_segments_cannot_be_frontend_supplied_paths() {
        assert_eq!(checked_id("run-1", "run_id"), Ok("run-1"));
        assert!(checked_id("../shell", "run_id").is_err());
        assert!(checked_id("run/approve", "run_id").is_err());
    }

    #[test]
    fn attach_payload_has_only_typed_contract_and_envelope_fields() {
        let contract = json!({"schema_version": "arda.project-contract.v1"});
        let envelope = MutationEnvelope {
            approval: TaskApproval {
                schema_version: "arda.orome.task_approval.v1".into(),
                proposal_id: "proposal-1".into(),
                approval_id: "approval-1".into(),
                ledger_writes: vec![],
                decision: "policy_safe".into(),
                created_at_utc: "2026-07-31T00:00:00Z".into(),
            },
            idempotency_key: "attach-1".into(),
        };
        let payload = serde_json::to_value(AttachRequest {
            contract: &contract,
            envelope: &envelope,
        })
        .expect("serialize payload");
        assert_eq!(payload.as_object().map(|value| value.len()), Some(2));
        assert!(payload.get("shell").is_none());
        assert!(payload.get("command").is_none());
    }

    #[test]
    fn completion_payload_contains_only_receipt_and_typed_envelope() {
        let envelope = MutationEnvelope {
            approval: TaskApproval {
                schema_version: "arda.orome.task_approval.v1".into(),
                proposal_id: "proposal-1".into(),
                approval_id: "approval-1".into(),
                ledger_writes: vec![],
                decision: "policy_safe".into(),
                created_at_utc: "2026-07-31T00:00:00Z".into(),
            },
            idempotency_key: "complete-1".into(),
        };
        let payload = serde_json::to_value(CompleteRequest {
            receipt_digest: "sha256:receipt",
            envelope: &envelope,
            evidence: None,
        })
        .expect("serialize completion payload");
        assert_eq!(payload.as_object().map(|value| value.len()), Some(2));
        assert_eq!(payload["receipt_digest"], "sha256:receipt");
        assert!(payload.get("shell").is_none());
        assert!(payload.get("command").is_none());
    }

    #[test]
    fn provider_execution_payload_contains_only_objective_and_typed_envelope() {
        let envelope = MutationEnvelope {
            approval: TaskApproval {
                schema_version: "arda.orome.task_approval.v1".into(),
                proposal_id: "proposal-1".into(),
                approval_id: "approval-1".into(),
                ledger_writes: vec![],
                decision: "policy_safe".into(),
                created_at_utc: "2026-07-31T00:00:00Z".into(),
            },
            idempotency_key: "execute-provider-1".into(),
        };
        let payload = serde_json::to_value(ExecuteProviderRequest {
            objective: "Apply the approved bounded change.",
            envelope: &envelope,
        })
        .expect("serialize provider execution payload");
        assert_eq!(payload.as_object().map(|value| value.len()), Some(2));
        assert_eq!(payload["objective"], "Apply the approved bounded change.");
        assert!(payload.get("shell").is_none());
        assert!(payload.get("adapter_config").is_none());
    }

    #[test]
    fn sse_parser_handles_chunk_boundaries_and_ignores_non_data_frames() {
        let mut buffer = b"event: run_event\ndata: {\"sequence\":1}\n\nevent: run_".to_vec();
        assert_eq!(take_sse_events(&mut buffer), [json!({"sequence": 1})]);
        buffer.extend_from_slice(b"event\ndata: {\"sequence\":2}\n\n: keep-alive\n\n");
        assert_eq!(take_sse_events(&mut buffer), [json!({"sequence": 2})]);
        assert!(buffer.is_empty());
    }
}
