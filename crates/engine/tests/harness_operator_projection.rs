use arda_core::operator_projection::OperatorProjection;
use arda_engine::harness::presence::HarnessPresenceState;
use arda_engine::harness::{
    self, HarnessState, DEFAULT_HARNESS_ADDR, DEFAULT_MANWE_PROXY_TIMEOUT,
    DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

fn base_state(workbench_root: PathBuf) -> HarnessState {
    HarnessState {
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
        workbench_root,
        operator_id: "operator-0".to_string(),
    }
}

fn write_run(root: &Path, body: &str) {
    let directory = root.join("data/runs/run-api");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("checkpoint.json"), body).unwrap();
}

fn valid_run() -> &'static str {
    r#"{
  "schema_version": "arda.run-graph.v1",
  "run_id": "run-api",
  "objective_id": "objective-api",
  "nodes": [{
    "id": "execute",
    "kind": "execute",
    "state": "running",
    "authority": "read_only",
    "budget": { "max_joules": 10.0, "max_cost_usd": 0.0 },
    "retry": { "max_attempts": 1 },
    "timeout_ms": 60000,
    "idempotency_key": "run-api-execute",
    "input_digest": "objective:objective-api",
    "output_digest": null,
    "parent_receipts": [],
    "checkpoint": { "sequence": 0, "recovery_token": null, "checkpoint_digest": null }
  }],
  "edges": [],
  "provenance": {
    "project_contract_digest": "project:api",
    "created_by": "harness-test",
    "parent_receipts": []
  }
}"#
}

async fn start(root: &Path) -> (String, Arc<Notify>, tokio::task::JoinHandle<()>) {
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        base_state(root.to_path_buf()),
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (format!("http://{bound}"), shutdown, handle)
}

#[tokio::test]
async fn canonical_operator_projection_endpoint_publishes_and_preserves_live_ids() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), valid_run());
    let (base, shutdown, handle) = start(root.path()).await;

    let response = reqwest::get(format!("{base}/v1/operator-projection"))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let projection: Value = response.json().await.unwrap();
    assert_eq!(projection["authority"], "read_only");
    assert_eq!(projection["freshness"], "fresh");
    assert_eq!(projection["objectives"][0]["objective_id"], "objective-api");
    assert_eq!(projection["runs"][0]["run_id"], "run-api");
    assert_eq!(projection["runs"][0]["status"], "running");

    let persisted =
        std::fs::read_to_string(root.path().join("core/state/operator_projection.json")).unwrap();
    let persisted: Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(projection["schema_version"], persisted["schema_version"]);
    assert_eq!(projection["authority"], persisted["authority"]);
    assert_eq!(projection["objectives"], persisted["objectives"]);
    assert_eq!(projection["runs"], persisted["runs"]);
    assert_eq!(projection["dependencies"], persisted["dependencies"]);
    OperatorProjection::from_json_str(&serde_json::to_string(&projection).unwrap()).unwrap();
    OperatorProjection::from_json_str(&serde_json::to_string(&persisted).unwrap()).unwrap();

    let mutation = reqwest::Client::new()
        .post(format!("{base}/v1/operator-projection"))
        .json(&json!({"run_id": "different"}))
        .send()
        .await
        .unwrap();
    assert_eq!(mutation.status(), 405);

    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn operator_projection_endpoint_exposes_unavailable_and_invalid_source_states() {
    let root = tempfile::tempdir().unwrap();
    let (base, shutdown, handle) = start(root.path()).await;

    let missing = reqwest::get(format!("{base}/v1/operator-projection"))
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    let missing_body: Value = missing.json().await.unwrap();
    assert_eq!(missing_body["state"], "unavailable");

    write_run(root.path(), "{not-json");
    let invalid = reqwest::get(format!("{base}/v1/operator-projection"))
        .await
        .unwrap();
    assert_eq!(invalid.status(), 422);
    let invalid_body: Value = invalid.json().await.unwrap();
    assert_eq!(invalid_body["state"], "failed");
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn harness_publishes_projection_for_file_consumers_without_an_api_read() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), valid_run());
    let (_base, shutdown, handle) = start(root.path()).await;
    let output = root.path().join("core/state/operator_projection.json");

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if output.is_file() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("publisher must materialize the HUD handoff without an API request");

    let projection = std::fs::read_to_string(output).unwrap();
    let projection = OperatorProjection::from_json_str(&projection).unwrap();
    assert_eq!(projection.runs[0].run_id, "run-api");

    shutdown.notify_waiters();
    handle.await.unwrap();
}
