use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use arda_orome::a2a_mesh::{
    CapabilityObservation, NodeEnrollment, NodeIdentity, ResourcePressureObservation,
};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
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

async fn start_manwe() -> (
    std::net::SocketAddr,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_chat = calls.clone();
    async fn providers() -> Json<Value> {
        Json(json!({
            "ok": true,
            "providers": [
                provider("profile-fast", "model-fast", "high"),
                provider("profile-review", "model-review", "medium")
            ]
        }))
    }
    async fn chat(
        State(calls): State<Arc<AtomicUsize>>,
        Json(body): Json<Value>,
    ) -> (StatusCode, HeaderMap, Json<Value>) {
        calls.fetch_add(1, Ordering::SeqCst);
        let model = body["model"].as_str().unwrap();
        let provider = if model == "model-fast" {
            "profile-fast"
        } else {
            "profile-review"
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-manwe-provider-id",
            HeaderValue::from_str(provider).unwrap(),
        );
        headers.insert("x-manwe-model-id", HeaderValue::from_str(model).unwrap());
        (
            StatusCode::OK,
            headers,
            Json(json!({
                "model": model,
                "choices": [{"message": {"role": "assistant", "content": format!("bounded result from {provider}")}}]
            })),
        )
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/providers", get(providers))
        .route("/v1/chat/completions", post(chat))
        .with_state(calls_for_chat);
    let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, calls, handle)
}

fn provider(id: &str, model: &str, quality: &str) -> Value {
    json!({
        "id": id,
        "provider_id": id,
        "access_tier": "local",
        "quality_band": quality,
        "enabled": true,
        "healthy": true,
        "in_cooldown": false,
        "operational_blocked": false,
        "intelligence_refreshed_at_utc": "2026-08-22T18:00:00Z",
        "hermes_bridge": {"persistent": false},
        "models": [{
            "id": model,
            "healthy": true,
            "in_cooldown": false,
            "capable_tasks": ["code", "reasoning"],
            "capabilities": {"tools": true, "structured_output": true},
            "cost_per_million_tokens_in": 0.0,
            "cost_per_million_tokens_out": 0.0
        }]
    })
}

async fn start_root(
    root: &TempDir,
    manwe_addr: std::net::SocketAddr,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_owned(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: format!("http://{manwe_addr}"),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator:placement-proof".to_owned(),
    };
    let (addr, handle) = serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .unwrap();
    (addr, shutdown, handle)
}

fn enrollment(node_id: &str) -> NodeEnrollment {
    let now = Utc::now();
    NodeEnrollment {
        schema_version: "arda.node-enrollment.v1".to_owned(),
        identity: NodeIdentity {
            schema_version: "arda.node-identity.v1".to_owned(),
            node_id: node_id.to_owned(),
            agent_id: format!("agent:{node_id}"),
            trust_domain: "home".to_owned(),
            enrollment_epoch: 1,
        },
        agent_card_url: format!("http://127.0.0.1:9/{node_id}/agent-card"),
        bearer_env: format!("UNUSED_{}", node_id.replace('-', "_")),
        allowed_capabilities: vec![
            "arda.cognition.worker.v1".to_owned(),
            "arda.cognition.critic.v1".to_owned(),
            "arda.cognition.adjudicator.v1".to_owned(),
        ],
        allowed_data_domains: vec!["system".to_owned()],
        issued_at: now - ChronoDuration::seconds(1),
        expires_at: now + ChronoDuration::minutes(10),
        revoked_at: None,
    }
}

fn observation(node_id: &str, provider_id: &str, pressure: f32) -> CapabilityObservation {
    let now = Utc::now();
    CapabilityObservation {
        schema_version: "arda.node-capability-observation.v1".to_owned(),
        observation_id: format!("observation:{node_id}:1"),
        node_id: node_id.to_owned(),
        capabilities: vec![
            "arda.cognition.worker.v1".to_owned(),
            "arda.cognition.critic.v1".to_owned(),
            "arda.cognition.adjudicator.v1".to_owned(),
            format!("manwe.provider:{provider_id}"),
        ],
        pressure: ResourcePressureObservation {
            cpu: pressure,
            memory: pressure,
            gpu: Some(pressure),
            queue_depth: 0,
        },
        observed_at: now,
        expires_at: now + ChronoDuration::minutes(5),
    }
}

async fn publish_profiles(root_addr: std::net::SocketAddr) {
    let client = reqwest::Client::new();
    for (node, provider, pressure) in [
        ("node-fast", "profile-fast", 0.1),
        ("node-review", "profile-review", 0.15),
    ] {
        client
            .post(format!("http://{root_addr}/v1/mesh/enroll"))
            .header("x-arda-operator-id", "operator:placement-proof")
            .json(&enrollment(node))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        client
            .post(format!("http://{root_addr}/v1/mesh/observations"))
            .header("x-arda-operator-id", "operator:placement-proof")
            .json(&observation(node, provider, pressure))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
}

#[tokio::test]
async fn one_objective_executes_across_two_live_capability_profiles() {
    let root = TempDir::new().unwrap();
    let (manwe_addr, calls, manwe_handle) = start_manwe().await;
    let (root_addr, shutdown, root_handle) = start_root(&root, manwe_addr).await;
    publish_profiles(root_addr).await;

    let response = reqwest::Client::new()
        .post(format!("http://{root_addr}/v1/adaptive-placement/objectives"))
        .header("x-arda-operator-id", "operator:placement-proof")
        .json(&json!({
            "objective_id": "objective:stage3-two-profile-proof",
            "objective": "Review a proposed Rust queue change for implementation and operational risk",
            "data_domain": "system",
            "task_kind": "code",
            "material_unresolved_risks": true,
            "unresolved_disagreement": true,
            "requires_tools": true,
            "requires_structured_output": true,
            "max_cost_usd": 0.01,
            "execute": true
        }))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["composition"]["worker_count"], 3);
    assert_eq!(body["placements"].as_array().unwrap().len(), 3);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let profiles = body["capability_profiles_used"].as_array().unwrap();
    assert_eq!(profiles.len(), 2);
    let roles = body["placements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["role"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(roles, vec!["worker", "critic", "adjudicator"]);
    assert!(body["placements"].as_array().unwrap().iter().all(|row| {
        row["execution"]["status"] == "completed"
            && row["execution"]["lifetime_status"] == "task_terminated_after_receipt"
            && row["sources"]
                .as_array()
                .is_some_and(|sources| sources.len() == 3)
    }));

    shutdown.notify_waiters();
    root_handle.await.unwrap();
    manwe_handle.abort();
}

#[tokio::test]
async fn deterministic_objective_composes_zero_model_workers() {
    let root = TempDir::new().unwrap();
    let (manwe_addr, calls, manwe_handle) = start_manwe().await;
    let (root_addr, shutdown, root_handle) = start_root(&root, manwe_addr).await;
    let body: Value = reqwest::Client::new()
        .post(format!(
            "http://{root_addr}/v1/adaptive-placement/objectives"
        ))
        .header("x-arda-operator-id", "operator:placement-proof")
        .json(&json!({
            "objective_id": "objective:deterministic",
            "objective": "Compute a stable digest",
            "data_domain": "system",
            "deterministic_tool_suffices": true,
            "execute": true
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["composition"]["worker_count"], 0);
    assert_eq!(body["placements"].as_array().unwrap().len(), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    shutdown.notify_waiters();
    root_handle.await.unwrap();
    manwe_handle.abort();
}

#[tokio::test]
async fn bounded_composition_adds_roles_only_for_declared_escalation() {
    let root = TempDir::new().unwrap();
    let (manwe_addr, calls, manwe_handle) = start_manwe().await;
    let (root_addr, shutdown, root_handle) = start_root(&root, manwe_addr).await;
    publish_profiles(root_addr).await;
    let client = reqwest::Client::new();
    for (objective_id, material_risk, expected_roles) in [
        ("objective:ordinary", false, 1),
        ("objective:material-risk", true, 2),
    ] {
        let body: Value = client
            .post(format!(
                "http://{root_addr}/v1/adaptive-placement/objectives"
            ))
            .header("x-arda-operator-id", "operator:placement-proof")
            .json(&json!({
                "objective_id": objective_id,
                "objective": "Assess one bounded implementation question",
                "data_domain": "system",
                "task_kind": "code",
                "material_unresolved_risks": material_risk,
                "requires_tools": true,
                "requires_structured_output": true,
                "execute": false
            }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(body["composition"]["worker_count"], expected_roles);
        assert_eq!(body["placements"].as_array().unwrap().len(), expected_roles);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    shutdown.notify_waiters();
    root_handle.await.unwrap();
    manwe_handle.abort();
}

#[tokio::test]
#[ignore = "requires the live local Manwe service and real inference nodes"]
async fn live_stage3_objective_uses_core_and_beelink_without_workflow_changes() {
    let root = TempDir::new().unwrap();
    let live_manwe: std::net::SocketAddr = "127.0.0.1:7171".parse().unwrap();
    let (root_addr, shutdown, root_handle) = start_root(&root, live_manwe).await;
    let client = reqwest::Client::new();
    for (node, provider, pressure) in [
        ("node-core-live", "edge_core", 0.08),
        ("node-beelink-live", "edge_beelink_light", 0.12),
    ] {
        client
            .post(format!("http://{root_addr}/v1/mesh/enroll"))
            .header("x-arda-operator-id", "operator:placement-proof")
            .json(&enrollment(node))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        client
            .post(format!("http://{root_addr}/v1/mesh/observations"))
            .header("x-arda-operator-id", "operator:placement-proof")
            .json(&observation(node, provider, pressure))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    let objective_id = std::env::var("STAGE_ADAPTIVE_OBJECTIVE_ID")
        .unwrap_or_else(|_| "objective:digital-organism-s3-live-proof".to_owned());
    let objective = std::env::var("STAGE_ADAPTIVE_OBJECTIVE").unwrap_or_else(|_| {
        "Review the Stage 3 adaptive placement slice for implementation correctness and unresolved operational risk".to_owned()
    });
    let response = client
        .post(format!(
            "http://{root_addr}/v1/adaptive-placement/objectives"
        ))
        .header("x-arda-operator-id", "operator:placement-proof")
        .json(&json!({
            "objective_id": objective_id,
            "objective": objective,
            "data_domain": "system",
            "task_kind": "code",
            "material_unresolved_risks": true,
            "unresolved_disagreement": true,
            "requires_tools": true,
            "requires_structured_output": true,
            "max_cost_usd": 0.01,
            "execute": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["capability_profiles_used"].as_array().unwrap().len(),
        2
    );
    assert!(body["placements"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["execution"]["status"] == "completed"));
    let evidence_path = std::env::var("STAGE3_EVIDENCE_PATH")
        .expect("STAGE3_EVIDENCE_PATH is required for live proof");
    std::fs::write(&evidence_path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    println!("stage3 live evidence: {evidence_path}");

    shutdown.notify_waiters();
    root_handle.await.unwrap();
}
