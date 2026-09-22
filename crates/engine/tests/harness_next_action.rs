use arda_engine::harness::{
    self, presence::HarnessPresenceState, HarnessState, DEFAULT_MANWE_PROXY_TIMEOUT,
    DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::Value;
#[path = "fixtures/objective_agenda.rs"]
mod objective_agenda;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

fn state(root: &std::path::Path) -> HarnessState {
    HarnessState {
        research_store_policy: arda_engine::harness::ResearchStorePolicy::Isolated,
        harness_addr: "127.0.0.1:7878".to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".to_string(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.to_path_buf(),
        operator_id: "operator:mythos".to_string(),
    }
}

fn write_objective(root: &std::path::Path) {
    objective_agenda::seed(
        root,
        "operator-current",
        "operator:mythos",
        "pending_approval",
        90,
    );
}

async fn read_next_action(root: &std::path::Path) -> Value {
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state(root),
        shutdown.clone(),
    )
    .await
    .unwrap();
    let response = reqwest::Client::new()
        .get(format!("http://{bound}/v1/next-action"))
        .header("x-arda-operator-id", "operator:mythos")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value = response.json().await.unwrap();
    shutdown.notify_waiters();
    handle.await.unwrap();
    value
}

#[tokio::test]
async fn next_action_endpoint_uses_configured_identity_and_survives_restart() {
    let root = tempfile::tempdir().unwrap();
    write_objective(root.path());

    let before = read_next_action(root.path()).await;
    let after = read_next_action(root.path()).await;

    assert_eq!(before["schema_version"], "arda.next-action.v1");
    assert_eq!(before["selected"]["id"], "operator-current");
    assert_eq!(before["selected"]["source_kind"], "objective");
    assert_eq!(after["selected"]["id"], "operator-current");
}
