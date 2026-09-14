use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use arda_orome::a2a_mesh::{
    CapabilityObservation, NodeEnrollment, NodeIdentity, ResourcePressureObservation, WorkEnvelope,
};
use axum::{
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

async fn start_peer(
    forged: bool,
) -> (
    std::net::SocketAddr,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound = listener.local_addr().unwrap();
    let rpc_url = format!("http://{bound}/a2a");
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_handler = calls.clone();
    let app = Router::new()
        .route(
            "/.well-known/agent-card.json",
            get(move || {
                let rpc_url = rpc_url.clone();
                async move {
                    Json(json!({
                        "name": "independent-peer",
                        "description": "Independent typed echo node",
                        "url": rpc_url,
                        "protocolVersion": "1.0",
                        "version": "1",
                        "capabilities": {"streaming": false},
                        "skills": [{
                            "id": "arda.echo.typed.v1",
                            "name": "Typed echo",
                            "description": "Echoes an Arda typed work envelope",
                            "tags": ["arda", "typed-echo"]
                        }]
                    }))
                }
            }),
        )
        .route(
            "/a2a",
            post(
                move |headers: HeaderMap, Json(request): Json<Value>| async move {
                    calls_for_handler.fetch_add(1, Ordering::SeqCst);
                    if headers
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        != Some("Bearer stage2-test-token")
                    {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"error": "unauthorized"})),
                        );
                    }
                    let context_id = request["params"]["message"]["contextId"]
                        .as_str()
                        .unwrap_or_default();
                    let envelope_id = request["id"].as_str().unwrap_or_default();
                    let response_id = if forged {
                        "forged-envelope"
                    } else {
                        envelope_id
                    };
                    (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0",
                            "id": response_id,
                            "result": {
                                "task": {
                                    "id": "peer-task-1",
                                    "contextId": context_id,
                                    "status": {"state": "TASK_STATE_COMPLETED"},
                                    "artifacts": [{
                                        "artifactId": "typed-echo-result",
                                        "parts": [{"text": "hello independent node"}]
                                    }]
                                }
                            }
                        })),
                    )
                },
            ),
        );
    let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (bound, calls, handle)
}

async fn start_root(
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
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:7171".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator:mesh-proof".into(),
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

fn configure_a2a_node(root: &TempDir, node_id: &str, bearer_env: &str) {
    std::fs::create_dir_all(root.path().join("config")).unwrap();
    std::fs::write(
        root.path().join("config/a2a-node.toml"),
        format!(
            r#"schema_version = "arda.a2a-node-config.v1"
node_id = "{node_id}"
agent_id = "agent:{node_id}"
trust_domain = "home"
capabilities = ["arda.echo.typed.v1"]
allowed_data_domains = ["system"]
inbound_bearer_env = "{bearer_env}"
"#
        ),
    )
    .unwrap();
}

fn enrollment(node_id: &str, card_url: String, bearer_env: &str) -> NodeEnrollment {
    let now = Utc::now();
    NodeEnrollment {
        schema_version: "arda.node-enrollment.v1".into(),
        identity: NodeIdentity {
            schema_version: "arda.node-identity.v1".into(),
            node_id: node_id.into(),
            agent_id: format!("agent:{node_id}"),
            trust_domain: "home".into(),
            enrollment_epoch: 1,
        },
        agent_card_url: card_url,
        bearer_env: bearer_env.into(),
        allowed_capabilities: vec!["arda.echo.typed.v1".into()],
        allowed_data_domains: vec!["system".into()],
        issued_at: now - ChronoDuration::seconds(1),
        expires_at: now + ChronoDuration::minutes(10),
        revoked_at: None,
    }
}

fn observation(node_id: &str, ttl_ms: i64) -> CapabilityObservation {
    let now = Utc::now();
    CapabilityObservation {
        schema_version: "arda.node-capability-observation.v1".into(),
        observation_id: format!("observation:{node_id}:1"),
        node_id: node_id.into(),
        capabilities: vec!["arda.echo.typed.v1".into()],
        pressure: ResourcePressureObservation {
            cpu: 0.1,
            memory: 0.1,
            gpu: None,
            queue_depth: 0,
        },
        observed_at: now,
        expires_at: now + ChronoDuration::milliseconds(ttl_ms),
    }
}

fn envelope(id: &str) -> WorkEnvelope {
    let now = Utc::now();
    WorkEnvelope {
        schema_version: "arda.work-envelope.v1".into(),
        envelope_id: id.into(),
        objective_id: std::env::var("STAGE_A2A_OBJECTIVE_ID")
            .unwrap_or_else(|_| "objective:mesh-proof".into()),
        run_id: format!("run:{id}"),
        worker_id: "worker:root".into(),
        capability: "arda.echo.typed.v1".into(),
        data_domain: "system".into(),
        payload: json!({"kind": "typed_echo", "text": "hello independent node"}),
        issued_at: now,
        expires_at: now + ChronoDuration::minutes(2),
        nonce: format!("nonce:{id}"),
        route_trace: vec!["node-root".into()],
        max_hops: 3,
    }
}

#[tokio::test]
async fn two_root_harness_nodes_exchange_and_receipt_both_sides() {
    let root = TempDir::new().unwrap();
    let peer = TempDir::new().unwrap();
    let bearer_env = format!("ARDA_MESH_NODE_TOKEN_{}", uuid::Uuid::new_v4().simple());
    std::env::set_var(&bearer_env, "stage2-node-token");
    configure_a2a_node(&peer, "node-peer", &bearer_env);

    let (peer_addr, peer_shutdown, peer_handle) = start_root(&peer).await;
    let (root_addr, root_shutdown, root_handle) = start_root(&root).await;
    let client = reqwest::Client::new();
    let card_url = format!("http://{peer_addr}/.well-known/agent-card.json");

    client
        .post(format!("http://{root_addr}/v1/mesh/enroll"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&enrollment("node-peer", card_url, &bearer_env))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("http://{root_addr}/v1/mesh/observations"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&observation("node-peer", 60_000))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let response = client
        .post(format!("http://{root_addr}/v1/mesh/dispatch"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&envelope("envelope:two-roots"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["receipt"]["target_node_id"], "node-peer");
    assert_eq!(body["result"]["status"]["state"], "TASK_STATE_COMPLETED");

    let peer_projection: Value = client
        .get(format!("http://{peer_addr}/v1/mesh"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(peer_projection["receipts"].as_array().unwrap().len(), 1);
    assert_eq!(
        peer_projection["receipts"][0]["envelope_id"],
        "envelope:two-roots"
    );

    if let Ok(path) = std::env::var("STAGE_A2A_EVIDENCE_PATH") {
        let artifact = json!({
            "schema_version": "arda.digital-organism.a2a-proof.v1",
            "objective_id": envelope("envelope:two-roots").objective_id,
            "transport": "linux-foundation-a2a-jsonrpc-http",
            "root_address": root_addr.to_string(),
            "peer_address": peer_addr.to_string(),
            "dispatch": body,
            "peer_projection": peer_projection,
        });
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }

    root_shutdown.notify_waiters();
    peer_shutdown.notify_waiters();
    root_handle.await.unwrap();
    peer_handle.await.unwrap();
    std::env::remove_var(&bearer_env);
}

#[tokio::test]
async fn independent_nodes_exchange_authenticated_typed_task_and_project_offline_truth() {
    let root = TempDir::new().unwrap();
    let bearer_env = format!("ARDA_MESH_TEST_TOKEN_{}", uuid::Uuid::new_v4().simple());
    std::env::set_var(&bearer_env, "stage2-test-token");

    let (peer_addr, peer_calls, peer_handle) = start_peer(false).await;
    let (root_addr, shutdown, root_handle) = start_root(&root).await;
    let client = reqwest::Client::new();
    client
        .post(format!("http://{root_addr}/v1/mesh/enroll"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&enrollment(
            "node-peer",
            format!("http://{peer_addr}/.well-known/agent-card.json"),
            &bearer_env,
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("http://{root_addr}/v1/mesh/observations"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&observation("node-peer", 750))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let response: Value = client
        .post(format!("http://{root_addr}/v1/mesh/dispatch"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&envelope("envelope:real-transport:1"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response["receipt"]["status"], "completed");
    assert_eq!(response["receipt"]["a2a_task_id"], "peer-task-1");
    assert_eq!(response["receipt"]["target_node_id"], "node-peer");

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    let projection: Value = client
        .get(format!("http://{root_addr}/v1/mesh"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projection["peers"][0]["availability"], "offline");
    assert_eq!(peer_calls.load(Ordering::SeqCst), 1);
    client
        .post(format!("http://{root_addr}/v1/mesh/node-peer/revoke"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&json!({"reason": "transport proof complete"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let revoked: Value = client
        .get(format!("http://{root_addr}/v1/mesh"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(revoked["peers"][0]["availability"], "revoked");
    assert_eq!(projection["receipts"].as_array().unwrap().len(), 1);

    shutdown.notify_waiters();
    root_handle.await.unwrap();
    peer_handle.abort();
    std::env::remove_var(&bearer_env);
}

#[tokio::test]
async fn forged_completion_is_rejected_without_recording_a_receipt() {
    let root = TempDir::new().unwrap();
    let bearer_env = format!("ARDA_MESH_TEST_TOKEN_{}", uuid::Uuid::new_v4().simple());
    std::env::set_var(&bearer_env, "stage2-test-token");
    let (peer_addr, peer_calls, peer_handle) = start_peer(true).await;
    let (root_addr, shutdown, root_handle) = start_root(&root).await;
    let client = reqwest::Client::new();
    client
        .post(format!("http://{root_addr}/v1/mesh/enroll"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&enrollment(
            "node-forged",
            format!("http://{peer_addr}/.well-known/agent-card.json"),
            &bearer_env,
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("http://{root_addr}/v1/mesh/observations"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&observation("node-forged", 5_000))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let rejected = client
        .post(format!("http://{root_addr}/v1/mesh/dispatch"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&envelope("envelope:forged:1"))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_GATEWAY);

    let projection: Value = client
        .get(format!("http://{root_addr}/v1/mesh"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(projection["receipts"].as_array().unwrap().is_empty());
    assert_eq!(peer_calls.load(Ordering::SeqCst), 1);

    shutdown.notify_waiters();
    root_handle.await.unwrap();
    peer_handle.abort();
    std::env::remove_var(&bearer_env);
}

#[tokio::test]
async fn concurrent_replays_produce_one_transport_exchange() {
    let root = TempDir::new().unwrap();
    let bearer_env = format!("ARDA_MESH_TEST_TOKEN_{}", uuid::Uuid::new_v4().simple());
    std::env::set_var(&bearer_env, "stage2-test-token");
    let (peer_addr, peer_calls, peer_handle) = start_peer(false).await;
    let (root_addr, shutdown, root_handle) = start_root(&root).await;
    let client = reqwest::Client::new();
    client
        .post(format!("http://{root_addr}/v1/mesh/enroll"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&enrollment(
            "node-replay",
            format!("http://{peer_addr}/.well-known/agent-card.json"),
            &bearer_env,
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("http://{root_addr}/v1/mesh/observations"))
        .header("x-arda-operator-id", "operator:mesh-proof")
        .json(&observation("node-replay", 5_000))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let mut attempts = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let client = client.clone();
        let work = envelope("envelope:replay:1");
        attempts.spawn(async move {
            client
                .post(format!("http://{root_addr}/v1/mesh/dispatch"))
                .header("x-arda-operator-id", "operator:mesh-proof")
                .json(&work)
                .send()
                .await
                .unwrap()
                .status()
        });
    }
    let mut completed = 0;
    let mut replayed = 0;
    while let Some(result) = attempts.join_next().await {
        match result.unwrap() {
            StatusCode::OK => completed += 1,
            StatusCode::CONFLICT => replayed += 1,
            status => panic!("unexpected dispatch status {status}"),
        }
    }
    assert_eq!(completed, 1);
    assert_eq!(replayed, 19);
    assert_eq!(peer_calls.load(Ordering::SeqCst), 1);

    shutdown.notify_waiters();
    root_handle.await.unwrap();
    peer_handle.abort();
    std::env::remove_var(&bearer_env);
}
