use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

const PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

async fn start_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().expect("loopback address")),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (bound, shutdown, handle)
}

fn envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-runs-1",
            "approval_id": "approval-runs-1",
            "ledger_writes": ["test-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-07-31T00:00:00Z"
        },
        "idempotency_key": key
    })
}

fn contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture")
}

fn graph(run_id: &str, node_id: &str, kind: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": kind,
            "state": "pending",
            "authority": if kind == "approval" { "human_approval" } else { "read_only" },
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
            "created_by": "integration-test",
            "parent_receipts": []
        }
    })
}

async fn attach(client: &reqwest::Client, bound: std::net::SocketAddr) {
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": envelope("attach-for-runs")}))
        .send()
        .await
        .expect("attach request")
        .error_for_status()
        .expect("attach status");
}

async fn plan(
    client: &reqwest::Client,
    bound: std::net::SocketAddr,
    run_id: &str,
    node_id: &str,
    kind: &str,
) -> reqwest::Response {
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph(run_id, node_id, kind),
            "envelope": envelope(&format!("plan-{run_id}"))
        }))
        .send()
        .await
        .expect("plan request")
}

#[tokio::test]
async fn run_routes_plan_approve_read_and_expose_canonical_events() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let planned = plan(&client, bound, "run-approve", "approval-node", "approval").await;
    assert_eq!(planned.status(), 201);
    let retry = plan(&client, bound, "run-approve", "approval-node", "approval").await;
    assert_eq!(retry.status(), 200);

    let approved: Value = client
        .post(format!("http://{bound}/v1/runs/run-approve/approve"))
        .json(&json!({
            "node_id": "approval-node",
            "envelope": envelope("approve-run-approve")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status")
        .json()
        .await
        .expect("approve body");
    assert_eq!(approved["graph"]["nodes"][0]["state"], "succeeded");

    let run: Value = client
        .get(format!("http://{bound}/v1/runs/run-approve"))
        .send()
        .await
        .expect("get run")
        .error_for_status()
        .expect("get run status")
        .json()
        .await
        .expect("get run body");
    assert_eq!(run["graph"]["run_id"], "run-approve");

    let events: Value = client
        .get(format!("http://{bound}/v1/runs/run-approve/events"))
        .send()
        .await
        .expect("get events")
        .error_for_status()
        .expect("events status")
        .json()
        .await
        .expect("events body");
    assert!(events["events"]
        .as_array()
        .is_some_and(|events| events.len() >= 4));
    assert!(root
        .path()
        .join("data/runs/run-approve/events.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn cancel_is_idempotent_and_mutations_require_typed_envelopes() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    assert_eq!(
        plan(&client, bound, "run-cancel", "plan-node", "plan")
            .await
            .status(),
        201
    );

    let missing_envelope = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&json!({"reason": "operator requested"}))
        .send()
        .await
        .expect("missing-envelope request");
    assert_eq!(missing_envelope.status(), 422);

    let cancel_body = json!({
        "reason": "operator requested",
        "envelope": envelope("cancel-run-cancel")
    });
    let cancelled: Value = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel request")
        .error_for_status()
        .expect("cancel status")
        .json()
        .await
        .expect("cancel body");
    assert_eq!(cancelled["graph"]["nodes"][0]["state"], "cancelled");

    let retry = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel retry");
    assert_eq!(retry.status(), 200);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}
