use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

const MANIFEST: &str = r#"
schema_version = "arda.organism-manifest.v1"
organism_id = "arda:mythos:primary"
display_name = "Arda"
mission = "Compose bounded nodes into one governed digital organism."
operator_id = "operator:mythos"
privacy_domains = ["personal", "business", "system"]
accepted_transports = ["in_process_rust", "arda_harness_http", "hermes_plugin_hook", "linux_foundation_a2a", "mcp", "manwe_openai_api", "systemd_or_engine_adapter", "outpost_protocol"]
enabled_transports = ["in_process_rust", "arda_harness_http", "hermes_plugin_hook", "manwe_openai_api", "systemd_or_engine_adapter", "outpost_protocol"]

[authorities]
objective = "arda-core"
run = "arda-engine"
node = "arda-engine+arda-outpost-protocol"
session = "hermes-agent"
agent = "hermes-agent+a2a-agent-card"
semantic_envelope = "arda-orome"
a2a_wire = "hermes-a2a"
model_route = "manwe"
memory = "arda-vaire"
evidence = "arda-varda"
governance = "arda-governance"
projection = "arda-aule"

[contract_versions]
organism_manifest = "arda.organism-manifest.v1"
organism_context = "arda.organism-context.v1"
organism_outcome = "arda.organism-outcome.v1"
"#;

async fn start(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    std::fs::create_dir_all(root.path().join("config")).unwrap();
    std::fs::write(root.path().join("config/organism.toml"), MANIFEST).unwrap();
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:7171".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator:mythos".into(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .unwrap();
    (bound, shutdown, handle)
}

#[tokio::test]
async fn manifest_endpoint_is_authenticated_stable_and_restart_safe() {
    let root = TempDir::new().unwrap();
    let client = reqwest::Client::new();
    let (bound, shutdown, handle) = start(&root).await;

    let unauthorized = client
        .get(format!("http://{bound}/v1/organism/manifest"))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), reqwest::StatusCode::FORBIDDEN);

    let first: serde_json::Value = client
        .get(format!("http://{bound}/v1/organism/manifest"))
        .header("x-arda-operator-id", "operator:mythos")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        first["schema_version"],
        "arda.organism-manifest-response.v1"
    );
    assert_eq!(first["manifest"]["organism_id"], "arda:mythos:primary");
    assert!(first["manifest_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));

    shutdown.notify_waiters();
    handle.await.unwrap();
    let (restarted, restarted_shutdown, restarted_handle) = start(&root).await;
    let second: serde_json::Value = client
        .get(format!("http://{restarted}/v1/organism/manifest"))
        .header("x-arda-operator-id", "operator:mythos")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first, second);

    restarted_shutdown.notify_waiters();
    restarted_handle.await.unwrap();
}
