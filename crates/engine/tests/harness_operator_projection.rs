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
    }
}

fn fixture() -> &'static str {
    include_str!("../../../spec/operator-projection/v1/fixtures/valid-operator-projection.json")
}

fn publish(root: &Path, body: &str) {
    let path = root.join("core/state/operator_projection.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
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
async fn canonical_operator_projection_endpoint_preserves_ids_state_and_authority() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), fixture());
    let (base, shutdown, handle) = start(root.path()).await;

    let response = reqwest::get(format!("{base}/v1/operator-projection"))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let projection: Value = response.json().await.unwrap();
    let canonical: Value = serde_json::from_str(fixture()).unwrap();
    assert_eq!(projection, canonical, "CLI/API must return the canonical handoff without rewriting IDs, state, freshness, or provenance");
    assert_eq!(projection["projection_id"], "projection-p9-fixture");
    assert_eq!(projection["authority"], "read_only");
    assert_eq!(projection["freshness"], "fresh");
    assert_eq!(projection["objectives"][0]["objective_id"], "objective-p9");
    assert_eq!(projection["runs"][0]["run_id"], "run-p9");
    assert_eq!(projection["runs"][0]["status"], "running");

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
async fn operator_projection_endpoint_exposes_unavailable_and_invalid_states() {
    let root = tempfile::tempdir().unwrap();
    let (base, shutdown, handle) = start(root.path()).await;

    let missing = reqwest::get(format!("{base}/v1/operator-projection"))
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    let missing_body: Value = missing.json().await.unwrap();
    assert_eq!(missing_body["state"], "unavailable");

    publish(
        root.path(),
        &fixture().replace("arda.operator-projection.v1", "arda.operator-projection.v0"),
    );
    let invalid = reqwest::get(format!("{base}/v1/operator-projection"))
        .await
        .unwrap();
    assert_eq!(invalid.status(), 422);
    let invalid_body: Value = invalid.json().await.unwrap();
    assert_eq!(invalid_body["state"], "failed");

    shutdown.notify_waiters();
    handle.await.unwrap();
}
