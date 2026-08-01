use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

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

fn contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture")
}

fn envelope(idempotency_key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-projects-1",
            "approval_id": "approval-projects-1",
            "ledger_writes": ["test-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-07-31T00:00:00Z"
        },
        "idempotency_key": idempotency_key
    })
}

#[tokio::test]
async fn project_routes_validate_attach_and_list_typed_contracts() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    let validation = client
        .post(format!("http://{bound}/v1/projects/validate"))
        .json(&json!({"contract": contract()}))
        .send()
        .await
        .expect("validate request");
    assert_eq!(validation.status(), 200);
    let validation: Value = validation.json().await.expect("validation body");
    assert_eq!(validation["valid"], true);
    assert_eq!(
        validation["project_id"],
        "550e8400-e29b-41d4-a716-446655440000"
    );

    let missing_envelope = client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract()}))
        .send()
        .await
        .expect("missing-envelope request");
    assert_eq!(missing_envelope.status(), 422);

    let mut blocked_envelope = envelope("attach-blocked");
    blocked_envelope["approval"]["decision"] = json!("policy_blocked");
    let blocked = client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": blocked_envelope}))
        .send()
        .await
        .expect("blocked approval request");
    assert_eq!(blocked.status(), 403);

    let attached = client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": envelope("attach-1")}))
        .send()
        .await
        .expect("attach request");
    assert_eq!(attached.status(), 201);

    let retry = client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": envelope("attach-1")}))
        .send()
        .await
        .expect("attach retry");
    assert_eq!(retry.status(), 200);

    let projects: Value = client
        .get(format!("http://{bound}/v1/projects"))
        .send()
        .await
        .expect("list request")
        .error_for_status()
        .expect("list status")
        .json()
        .await
        .expect("list body");
    assert_eq!(projects["projects"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        projects["projects"][0]["contract"]["identity"]["project_id"],
        "550e8400-e29b-41d4-a716-446655440000"
    );

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn attach_rejects_untyped_browser_command_fields_without_executing_them() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let response = reqwest::Client::new()
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract(),
            "envelope": envelope("attach-shell"),
            "shell": "touch should-never-exist"
        }))
        .send()
        .await
        .expect("attach request");
    assert_eq!(response.status(), 422);
    assert!(!root.path().join("should-never-exist").exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}
