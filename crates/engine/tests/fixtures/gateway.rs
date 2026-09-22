use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use arda_engine::objectives::ObjectiveStore;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

pub const GATEWAY_CAPABILITY: &str = "test-hermes-gateway-capability";

pub fn gateway_client() -> reqwest::Client {
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

pub async fn start_research_harness_at(
    root: &std::path::Path,
    warden_scout_url: Option<String>,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    std::env::set_var("ARDA_HERMES_GATEWAY_CAPABILITY", GATEWAY_CAPABILITY);
    ObjectiveStore::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        research_store_policy: arda_engine::harness::ResearchStorePolicy::Isolated,
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.to_path_buf(),
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

pub fn gateway_message(message_id: &str, text: &str) -> Value {
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
