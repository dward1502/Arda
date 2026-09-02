use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use arda_engine::objectives::{ObjectiveState, ObjectiveStore};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

const PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const GATEWAY_CAPABILITY: &str = "test-hermes-gateway-capability";

fn gateway_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "x-arda-gateway-capability",
        GATEWAY_CAPABILITY.parse().expect("test capability header"),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("gateway client")
}

async fn start_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    std::env::set_var("ARDA_HERMES_GATEWAY_CAPABILITY", GATEWAY_CAPABILITY);
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "discord-user-1".to_string(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().expect("loopback")),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (bound, shutdown, handle)
}

async fn start_capability_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    std::env::set_var("ARDA_HERMES_GATEWAY_CAPABILITY", GATEWAY_CAPABILITY);
    start_harness(root).await
}

fn mutation_envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-operator-test",
            "approval_id": "approval-operator-test",
            "ledger_writes": ["test-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": Utc::now().to_rfc3339()
        },
        "idempotency_key": key
    })
}

fn gateway_message(message_id: &str, text: &str) -> Value {
    let timestamp = Utc::now().to_rfc3339();
    json!({
        "operator": {
            "operator_id": "discord-user-1",
            "authenticated": true,
            "authentication_method": "gateway_identity",
            "authenticated_at": timestamp
        },
        "adapter_id": "hermes-discord-default",
        "event": {
            "text": text,
            "message_type": "text",
            "user_id": "discord-user-1",
            "user_name": "operator",
            "source": {
                "platform": "discord",
                "chat_id": "discord-dm-1",
                "chat_type": "dm",
                "thread_id": null,
                "message_id": message_id
            },
            "message_id": message_id,
            "media_urls": [],
            "media_types": [],
            "timestamp": timestamp,
            "prompt_response": null
        }
    })
}

fn created_objective_id(response: &Value) -> &str {
    response["evidence_refs"]
        .as_array()
        .expect("evidence refs")
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|reference| reference.strip_prefix("arda://objectives/"))
        .find(|objective_id| !objective_id.contains('/'))
        .expect("resident objective reference")
}

#[tokio::test]
async fn gateway_capability_is_required_before_operator_ingestion() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_capability_harness(&root).await;
    let endpoint = format!("http://{bound}/v1/operator/messages");
    let body = gateway_message("discord-capability-rejection", "arda status");

    let missing = reqwest::Client::new()
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .expect("missing capability response");
    assert_eq!(missing.status(), 403);

    let wrong = reqwest::Client::new()
        .post(&endpoint)
        .header("x-arda-gateway-capability", "wrong-capability")
        .json(&body)
        .send()
        .await
        .expect("wrong capability response");
    assert_eq!(wrong.status(), 403);
    assert!(!root
        .path()
        .join("core/state/orome/operator-session/operator_sessions.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

fn approval_graph(run_id: &str, node_id: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": "approval",
            "state": "pending",
            "authority": "human_approval",
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 1000,
            "idempotency_key": format!("node-{run_id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": [],
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
        }],
        "edges": [],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "operator-test",
            "parent_receipts": []
        }
    })
}

fn execution_graph(run_id: &str, node_id: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": "execute",
            "state": "pending",
            "authority": "read_only",
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 1000,
            "idempotency_key": format!("node-{run_id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": [],
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
        }],
        "edges": [],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "operator-test",
            "parent_receipts": []
        }
    })
}

async fn attach_and_plan(client: &reqwest::Client, bound: std::net::SocketAddr, run_id: &str) {
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract, "envelope": mutation_envelope("attach-operator-test")}))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": approval_graph(run_id, "approval"),
            "envelope": mutation_envelope(&format!("plan-{run_id}"))
        }))
        .send()
        .await
        .expect("plan")
        .error_for_status()
        .expect("plan status");
}

#[tokio::test]
async fn authenticated_gateway_capture_is_durable_and_duplicate_safe() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let body = gateway_message(
        "discord-capture-1",
        "arda capture buy transplant-safe groceries",
    );

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("capture")
        .error_for_status()
        .expect("capture status")
        .json()
        .await
        .expect("capture body");
    assert!(response["summary"]
        .as_str()
        .is_some_and(|text| text.starts_with("Captured")));

    let inbox: Value = client
        .get(format!("http://{bound}/v1/personal/inbox"))
        .header("x-arda-operator-id", "discord-user-1")
        .send()
        .await
        .expect("inbox")
        .error_for_status()
        .expect("inbox status")
        .json()
        .await
        .expect("inbox body");
    assert_eq!(
        inbox["inbox"][0]["content"],
        "buy transplant-safe groceries"
    );

    let duplicate = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("duplicate");
    assert_eq!(duplicate.status(), 409);

    let mut other_chat = body.clone();
    other_chat["event"]["source"]["chat_id"] = json!("discord-dm-2");
    client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&other_chat)
        .send()
        .await
        .expect("same platform message id in another chat")
        .error_for_status()
        .expect("chat-scoped message id");
    let inbox: Value = client
        .get(format!("http://{bound}/v1/personal/inbox"))
        .header("x-arda-operator-id", "discord-user-1")
        .send()
        .await
        .expect("inbox after second chat")
        .error_for_status()
        .expect("inbox after second chat status")
        .json()
        .await
        .expect("inbox after second chat body");
    assert_eq!(inbox["inbox"].as_array().map(Vec::len), Some(2));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_context_returns_the_canonical_cross_domain_next_action() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-context-objective")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    let objective: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-context-objective",
            &format!("arda objective {PROJECT_ID} Review Arda against the operator vision"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective body");

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-context-next", "arda context"))
        .send()
        .await
        .expect("context")
        .error_for_status()
        .expect("context status")
        .json()
        .await
        .expect("context body");

    assert!(response["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Review Arda against the operator vision")));
    assert_eq!(
        response["evidence_refs"][1],
        format!("arda://objectives/{}", created_objective_id(&objective))
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objectives_reads_the_resident_objective_store() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-objectives-list")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objectives-create",
            &format!("arda objective {PROJECT_ID} Resume deferred repair"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective body");

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-objectives-1", "arda objectives"))
        .send()
        .await
        .expect("objectives")
        .error_for_status()
        .expect("objectives status")
        .json()
        .await
        .expect("objectives body");

    let summary = response["summary"].as_str().expect("summary");
    assert!(summary.contains("Objectives: 1"));
    assert!(summary.contains("authority=resident_objective_store"));
    assert!(summary.contains("[pending_approval]"));
    assert!(summary.contains("text=Resume deferred repair"));
    assert_eq!(
        response["evidence_refs"][2],
        format!("arda://objectives/{}", created_objective_id(&created))
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_controls_mutate_only_resident_objective_store() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-resident-controls")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-controls-objective",
            &format!("arda objective {PROJECT_ID} Original operator objective"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective response");
    let objective_id = created_objective_id(&created).to_owned();
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let task_id = store.list_leaves(&objective_id).expect("objective leaves")[0]
        .id
        .clone();

    for (message_id, command, expected) in [
        (
            "discord-pause-task",
            format!("arda pause-task {task_id} {objective_id} operator requested pause"),
            format!("Paused resident objective {objective_id}: operator requested pause"),
        ),
        (
            "discord-resume-task",
            format!("arda resume-task {task_id} {objective_id} operator requested resume"),
            format!("Resumed resident objective {objective_id}: operator requested resume"),
        ),
        (
            "discord-reprioritize-task",
            format!("arda reprioritize {task_id} {objective_id} critical urgent operator priority"),
            format!("Reprioritized {task_id} to 100: urgent operator priority"),
        ),
        (
            "discord-revise-objective",
            format!(
                "arda revise-objective {task_id} {objective_id} Revised operator objective --reason operator corrected scope"
            ),
            format!(
                "Revised resident objective {objective_id}; fresh approval is required: operator corrected scope"
            ),
        ),
        (
            "discord-approve-objective",
            format!("arda approve-objective {task_id} {objective_id} operator accepts revision"),
            format!("Approved resident objective {objective_id}: operator accepts revision"),
        ),
    ] {
        let response = client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, &command))
            .send()
            .await
            .expect("control");
        let status = response.status();
        let body = response.text().await.expect("control body");
        assert!(status.is_success(), "{command}: {status} {body}");
        let response: Value = serde_json::from_str(&body).expect("control JSON");
        assert_eq!(response["summary"], expected);
    }

    let approved = store
        .objective(&objective_id)
        .expect("read controlled objective")
        .expect("controlled objective");
    assert_eq!(approved.state, ObjectiveState::Approved);
    assert_eq!(approved.text, "Revised operator objective");
    assert_eq!(approved.priority, 100);
    assert_eq!(approved.revision, 2);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
    let operator_rows = fs::read_to_string(
        root.path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl"),
    )
    .unwrap();
    let operator_event_count = operator_rows.lines().count();

    let rejected_reapproval = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-reapprove-objective",
            &format!(
                "arda approve-objective {task_id} {objective_id} duplicate approval must fail"
            ),
        ))
        .send()
        .await
        .expect("rejected reapproval");
    assert_eq!(rejected_reapproval.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join("core/state/orome/operator-session/operator_sessions.jsonl"),
        )
        .unwrap()
        .lines()
        .count(),
        operator_event_count,
        "rejected resident mutation must not append an operator session event"
    );

    let wrong_lineage_command =
        format!("arda cancel-task {task_id} wrong-objective must not cancel");
    let wrong_lineage = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-task-wrong-objective",
            &wrong_lineage_command,
        ))
        .send()
        .await
        .expect("wrong-lineage cancellation");
    assert_eq!(wrong_lineage.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join("core/state/orome/operator-session/operator_sessions.jsonl"),
        )
        .unwrap()
        .lines()
        .count(),
        operator_event_count,
        "rejected canonical preflight must not append an operator session event"
    );
    let cancelled: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-task",
            &format!(
                "arda cancel-task {task_id} {objective_id} operator no longer wants this objective"
            ),
        ))
        .send()
        .await
        .expect("cancel task")
        .error_for_status()
        .expect("cancel task status")
        .json()
        .await
        .expect("cancel task body");
    assert_eq!(
        cancelled["summary"],
        format!(
            "Cancelled resident objective {objective_id}: operator no longer wants this objective"
        )
    );
    assert_eq!(
        store
            .objective(&objective_id)
            .expect("read cancelled objective")
            .expect("cancelled objective")
            .state,
        ObjectiveState::Cancelled
    );
    let operator_rows = fs::read_to_string(
        root.path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl"),
    )
    .unwrap();
    let last_operator_row: Value =
        serde_json::from_str(operator_rows.lines().last().unwrap()).unwrap();
    assert_eq!(last_operator_row["operation"], "cancel");

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_multi_project_objective_preserves_all_attached_project_authorities() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let first: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    let mut second = first.clone();
    let second_id = "550e8400-e29b-41d4-a716-446655440001";
    second["identity"]["project_id"] = Value::String(second_id.into());
    second["identity"]["name"] = Value::String("second-real-project".into());
    for (contract, key) in [
        (first, "attach-multi-objective-first"),
        (second, "attach-multi-objective-second"),
    ] {
        client
            .post(format!("http://{bound}/v1/projects/attach"))
            .json(&json!({
                "contract": contract,
                "envelope": mutation_envelope(key)
            }))
            .send()
            .await
            .expect("attach")
            .error_for_status()
            .expect("attach status");
    }

    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-multi-project-objective",
            &format!(
                "arda objective {PROJECT_ID},{second_id} inspect both real projects and join the evidence"
            ),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("multi-project objective status")
        .json()
        .await
        .expect("objective response");
    let objective_id = created_objective_id(&created);
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let objective = store
        .objective(objective_id)
        .expect("read objective")
        .expect("created objective");
    assert_eq!(objective.project_ids, vec![PROJECT_ID, second_id]);
    let leaves = store.list_leaves(objective_id).expect("objective leaves");
    assert_eq!(leaves.len(), 3, "two leaves plus dependent join");
    assert!(leaves.iter().any(|leaf| leaf.id.ends_with("-join")));
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objective_approval_schedules_and_can_cancel_before_first_claim() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-objective-control")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");

    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objective-control",
            &format!("arda objective {PROJECT_ID} create a disposable acceptance artifact"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective response");
    let objective_id = created_objective_id(&created).to_owned();
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let task_id = store.list_leaves(&objective_id).expect("objective leaves")[0]
        .id
        .clone();

    for (message_id, command) in [
        (
            "discord-objective-control-revise",
            format!(
                "arda revise-objective {task_id} {objective_id} create only a disposable acceptance artifact --reason bound the acceptance scope"
            ),
        ),
        (
            "discord-objective-control-approve",
            format!(
                "arda approve-objective {task_id} {objective_id} approve bounded disposable acceptance"
            ),
        ),
    ] {
        client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, &command))
            .send()
            .await
            .expect("objective control")
            .error_for_status()
            .expect("objective control status");
    }

    let approved = store
        .objective(&objective_id)
        .expect("read approved objective")
        .expect("approved objective");
    assert_eq!(approved.state, ObjectiveState::Approved);
    assert_eq!(approved.revision, 2);
    assert_eq!(
        approved.text,
        "create only a disposable acceptance artifact"
    );

    let cancelled: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objective-control-cancel",
            &format!("arda cancel-task {task_id} {objective_id} acceptance scenario cleanup"),
        ))
        .send()
        .await
        .expect("cancel unclaimed task")
        .error_for_status()
        .expect("cancel unclaimed task status")
        .json()
        .await
        .unwrap();
    assert_eq!(
        cancelled["summary"],
        format!("Cancelled resident objective {objective_id}: acceptance scenario cleanup")
    );
    let terminal = store
        .objective(&objective_id)
        .expect("read cancelled objective")
        .expect("cancelled objective");
    assert_eq!(terminal.state, ObjectiveState::Cancelled);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_research_command_persists_question_without_creating_commitment() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let body = gateway_message(
        "discord-research-1",
        "arda research practical x402 earning opportunities",
    );

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("research")
        .error_for_status()
        .expect("research status")
        .json()
        .await
        .expect("research body");

    assert!(response["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Research question")));
    let registry: Value = serde_json::from_str(
        &fs::read_to_string(root.path().join("data/workbench/research/questions.json"))
            .expect("question registry"),
    )
    .expect("question registry json");
    assert_eq!(registry["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        registry["records"][0]["question"],
        "practical x402 earning opportunities"
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());

    let duplicate = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("duplicate research");
    assert_eq!(duplicate.status(), 409);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn authenticated_gateway_approval_cancel_and_resume_use_canonical_runs() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();

    attach_and_plan(&client, bound, "phone-approve").await;
    let approved: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-approve-1",
            "arda approve phone-approve approval",
        ))
        .send()
        .await
        .expect("approve")
        .error_for_status()
        .expect("approve status")
        .json()
        .await
        .expect("approve body");
    assert_eq!(approved["run_id"], "phone-approve");
    let run: Value = client
        .get(format!("http://{bound}/v1/runs/phone-approve"))
        .send()
        .await
        .expect("approved run")
        .error_for_status()
        .expect("approved run status")
        .json()
        .await
        .expect("approved run body");
    assert_eq!(run["graph"]["nodes"][0]["state"], "succeeded");

    attach_and_plan(&client, bound, "phone-cancel").await;
    client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-1",
            "arda cancel phone-cancel operator requested stop",
        ))
        .send()
        .await
        .expect("cancel")
        .error_for_status()
        .expect("cancel status");
    let resumed: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-resume-1",
            "arda resume phone-cancel",
        ))
        .send()
        .await
        .expect("resume")
        .error_for_status()
        .expect("resume status")
        .json()
        .await
        .expect("resume body");
    assert!(resumed["summary"]
        .as_str()
        .is_some_and(|text| text.contains("approval=cancelled")));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objective_context_status_and_result_use_canonical_state() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    attach_and_plan(&client, bound, "phone-status").await;

    let objective: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objective-1",
            &format!("arda objective {PROJECT_ID} finish the operator bridge"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective body");
    assert!(objective["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains(PROJECT_ID)));
    let personal_ledger = std::fs::read_to_string(root.path().join("data/personal/events.jsonl"))
        .expect("personal ledger");
    assert!(personal_ledger.contains(PROJECT_ID));
    let objective_id = created_objective_id(&objective);
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let resident = store
        .objective(objective_id)
        .expect("read resident objective")
        .expect("resident objective");
    assert_eq!(resident.text, "finish the operator bridge");
    assert_eq!(resident.state, ObjectiveState::PendingApproval);
    assert_eq!(resident.project_ids, vec![PROJECT_ID.to_owned()]);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
    assert!(objective["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Execution still requires review")));

    let context: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-context-1", "arda context"))
        .send()
        .await
        .expect("context")
        .error_for_status()
        .expect("context status")
        .json()
        .await
        .expect("context body");
    assert!(context["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("finish the operator bridge")));

    let status: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-status-1", "arda status"))
        .send()
        .await
        .expect("status")
        .error_for_status()
        .expect("status status")
        .json()
        .await
        .expect("status body");
    assert!(status["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("awaiting approval: 1")));

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": execution_graph("phone-result", "execute"),
            "envelope": mutation_envelope("plan-phone-result")
        }))
        .send()
        .await
        .expect("plan result")
        .error_for_status()
        .expect("plan result status");
    let receipt_digest = format!("sha256:{}", "a".repeat(64));
    client
        .post(format!(
            "http://{bound}/v1/runs/phone-result/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": mutation_envelope("complete-phone-result"),
            "receipt_digest": receipt_digest,
            "evidence": {
                "changes": [{
                    "path": "artifacts/report.md",
                    "status": "added",
                    "additions": 4,
                    "deletions": 0
                }],
                "tests": [{"name": "operator-result-check", "status": "passed"}]
            }
        }))
        .send()
        .await
        .expect("complete result")
        .error_for_status()
        .expect("complete result status");
    let result: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-result-1",
            "arda result phone-result",
        ))
        .send()
        .await
        .expect("result")
        .error_for_status()
        .expect("result status")
        .json()
        .await
        .expect("result body");
    assert!(result["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Verified tests: 1/1")));
    assert!(result["evidence_refs"].as_array().is_some_and(|refs| {
        refs.iter().any(|reference| {
            reference
                .as_str()
                .is_some_and(|reference| reference.ends_with("files/artifacts/report.md"))
        })
    }));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_reject_and_revise_consume_scoped_decisions() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();

    for (run_id, message_id, command, expected_operation) in [
        (
            "phone-reject",
            "discord-reject-1",
            "arda reject phone-reject approval scope is too broad",
            "reject",
        ),
        (
            "phone-revise",
            "discord-revise-1",
            "arda revise phone-revise approval narrow the requested change",
            "revise",
        ),
    ] {
        attach_and_plan(&client, bound, run_id).await;
        client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, command))
            .send()
            .await
            .expect("decision")
            .error_for_status()
            .expect("decision status");
        let run: Value = client
            .get(format!("http://{bound}/v1/runs/{run_id}"))
            .send()
            .await
            .expect("decision run")
            .error_for_status()
            .expect("decision run status")
            .json()
            .await
            .expect("decision run body");
        assert_eq!(run["graph"]["nodes"][0]["state"], "cancelled");
        let ledger = std::fs::read_to_string(
            root.path()
                .join("core/state/orome/operator-session/operator_sessions.jsonl"),
        )
        .expect("operator ledger");
        let event: Value = serde_json::from_str(ledger.lines().last().expect("decision event"))
            .expect("decision json");
        assert_eq!(event["operation"], expected_operation);
        assert_eq!(event["approval"]["single_use_state"], "consumed");
    }

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_reminder_acknowledgement_requires_a_delivered_attempt() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let capture: Value = client
        .post(format!("http://{bound}/v1/personal/captures"))
        .header("x-arda-operator-id", "discord-user-1")
        .header("idempotency-key", "reminder-owner-capture")
        .json(&json!({
            "operator_id": "discord-user-1",
            "text": "Reminder ownership fixture"
        }))
        .send()
        .await
        .expect("capture request")
        .error_for_status()
        .expect("capture status")
        .json()
        .await
        .expect("capture body");
    let item_id = capture["capture_id"]
        .as_str()
        .expect("canonical capture id")
        .to_string();
    let reminder_id = uuid::Uuid::new_v4().to_string();

    let out_of_order = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-ack-early",
            &format!("arda acknowledge {reminder_id}"),
        ))
        .send()
        .await
        .expect("out-of-order acknowledgement");
    assert_eq!(out_of_order.status(), 409);

    client
        .post(format!("http://{bound}/v1/personal/reminders/attempt"))
        .header("x-arda-operator-id", "discord-user-1")
        .header("idempotency-key", "reminder-attempt-1")
        .json(&json!({
            "operator_id": "discord-user-1",
            "item_id": item_id,
            "reminder_id": reminder_id,
            "state": "delivered",
            "provider_message_id": "provider-reminder-1"
        }))
        .send()
        .await
        .expect("reminder attempt")
        .error_for_status()
        .expect("reminder attempt status");
    let acknowledged: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-ack-1",
            &format!("arda acknowledge {reminder_id}"),
        ))
        .send()
        .await
        .expect("acknowledgement")
        .error_for_status()
        .expect("acknowledgement status")
        .json()
        .await
        .expect("acknowledgement body");
    assert!(acknowledged["summary"]
        .as_str()
        .is_some_and(|summary| summary.ends_with("acknowledged.")));

    let duplicate_terminal = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-defer-late",
            &format!("arda defer {reminder_id}"),
        ))
        .send()
        .await
        .expect("late defer");
    assert_eq!(duplicate_terminal.status(), 409);

    let deferred_reminder_id = uuid::Uuid::new_v4().to_string();
    client
        .post(format!("http://{bound}/v1/personal/reminders/attempt"))
        .header("x-arda-operator-id", "discord-user-1")
        .header("idempotency-key", "reminder-attempt-2")
        .json(&json!({
            "operator_id": "discord-user-1",
            "item_id": item_id,
            "reminder_id": deferred_reminder_id,
            "state": "delivered",
            "provider_message_id": "provider-reminder-2"
        }))
        .send()
        .await
        .expect("second reminder attempt")
        .error_for_status()
        .expect("second reminder attempt status");
    let deferred: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-defer-1",
            &format!("arda defer {deferred_reminder_id}"),
        ))
        .send()
        .await
        .expect("defer")
        .error_for_status()
        .expect("defer status")
        .json()
        .await
        .expect("defer body");
    assert!(deferred["summary"]
        .as_str()
        .is_some_and(|summary| summary.ends_with("deferred.")));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_private_capture_rejects_group_audience_without_mutation() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let mut message = gateway_message("group-private-capture", "arda capture private medical note");
    message["event"]["source"]["chat_type"] = json!("group");

    let response = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .expect("send group capture");
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(
        !root.path().join("data/personal/events.jsonl").exists(),
        "wrong-audience capture must not mutate personal state"
    );
    assert!(
        !root
            .path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl")
            .exists(),
        "wrong-audience capture must be rejected before bridge ingestion"
    );

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_operator_endpoint_rejects_unauthenticated_identity() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let mut body = gateway_message("discord-denied-1", "arda capture denied");
    body["operator"]["authenticated"] = json!(false);

    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("denied");
    assert_eq!(denied.status(), 403);

    let mut forged = gateway_message("discord-forged-1", "arda objectives");
    forged["operator"]["operator_id"] = json!("forged-operator");
    forged["event"]["user_id"] = json!("forged-operator");
    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&forged)
        .send()
        .await
        .expect("forged identity");
    assert_eq!(denied.status(), 403);

    let mut stale = gateway_message("discord-stale-auth", "arda objectives");
    stale["operator"]["authenticated_at"] = json!("1970-01-01T00:00:00Z");
    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&stale)
        .send()
        .await
        .expect("stale authentication");
    assert_eq!(denied.status(), 403);

    let mut malformed = gateway_message("discord-malformed-auth", "arda objectives");
    malformed["operator"]["authenticated_at"] = json!("not-a-timestamp");
    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&malformed)
        .send()
        .await
        .expect("malformed authentication");
    assert_eq!(denied.status(), 403);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_council_query_projects_tension_and_decision_without_approval() {
    let root = TempDir::new().expect("root");
    let council_dir = root.path().join("data/runs/council-run-1");
    fs::create_dir_all(&council_dir).expect("council run directory");
    fs::write(
        council_dir.join("council-run.json"),
        include_bytes!("../../../spec/council-run/v1/fixtures/valid-independent-disagreement.json"),
    )
    .expect("council fixture");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-council-1",
            "arda council council-run-1",
        ))
        .send()
        .await
        .expect("council query")
        .error_for_status()
        .expect("council query status")
        .json()
        .await
        .expect("council query body");
    let summary = response["summary"].as_str().expect("summary");
    assert!(summary.contains("Material tension:"));
    assert!(summary.contains("Decision requested:"));
    assert!(summary.contains("operator approval has not been granted"));
    assert_eq!(
        response["evidence_refs"]
            .as_array()
            .unwrap()
            .last()
            .unwrap(),
        "arda://runs/council-run-1/council"
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}
