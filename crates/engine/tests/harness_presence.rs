//! Integration tests for the presence harness surface.
//!
//! Verifies:
//! - `/v1/presence/snapshot` returns a deterministic, versioned projection
//! - `/v1/presence/events` returns SSE with `text/event-stream`
//! - Loopback callers are authorized
//! - Remote callers without enrolled outpost identity / `presence.read`
//!   capability receive 401; with proper credentials receive 200.

use std::{sync::Arc, time::Duration};

use reqwest::Client;
use tokio::sync::Notify;

use arda_aule::presence_projection::{ProjectionInputs, ServicePresence};
use arda_engine::harness::presence::HarnessPresenceState;
use arda_engine::harness::{serve, HarnessState};
use arda_outpost_protocol::{
    presence::{HealthState, LifecycleState, ResourcePressure},
    NetworkPosture, OutpostAccessContract, OutpostEnrollment, OUTPOST_ACCESS_SCHEMA_VERSION,
};

#[tokio::test]
async fn presence_snapshot_returns_versioned_projection() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state();
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = client();
    let response = client
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .send()
        .await
        .expect("send snapshot request");
    let status = response.status();
    let body = response.text().await.expect("body");
    println!("snapshot status={status} body={body}");
    assert!(status.is_success(), "snapshot status: {}", status);
    let snapshot: serde_json::Value = serde_json::from_str(&body).expect("snapshot json");

    assert_eq!(snapshot["schema_version"], "arda.harness.presence.v1");
    assert_eq!(
        snapshot["snapshot"]["schema_version"],
        "arda.runtime-presence.v1"
    );
    assert!(snapshot["snapshot_sequence"].as_u64().unwrap() >= 1);
    assert!(!snapshot["generated_at"].as_str().unwrap().is_empty());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn presence_snapshot_publishes_updated_live_inputs() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state();
    let presence = state.presence_inputs.clone();
    presence
        .update_inputs(ProjectionInputs {
            services: vec![ServicePresence {
                id: "service-manwe".to_string(),
                label: "Manwe".to_string(),
                lifecycle: LifecycleState::Active,
                health: HealthState::Healthy,
                confidence: 0.9,
                freshness_seconds: 1,
                resource_pressure: ResourcePressure {
                    cpu: 0.1,
                    memory: 0.2,
                    provider: 0.0,
                },
                run_id: None,
                task_id: None,
                source_receipt_refs: vec!["receipt:manwe".to_string()],
            }],
            agents: Vec::new(),
            edges: Vec::new(),
            source_receipt_refs: vec!["receipt:manwe".to_string()],
        })
        .await;
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let snapshot: serde_json::Value = client()
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .send()
        .await
        .expect("send snapshot request")
        .json()
        .await
        .expect("snapshot json");

    assert_eq!(snapshot["snapshot"]["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["snapshot"]["nodes"][0]["id"], "service-manwe");
    assert!(snapshot["snapshot"]["projection_id"]
        .as_str()
        .unwrap()
        .starts_with("arda-runtime-presence-"));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn presence_events_returns_sse_stream() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state();
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = client();
    let response = client
        .get(format!("http://{bound}/v1/presence/events"))
        .send()
        .await
        .expect("send events request");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn remote_presence_without_enrollment_is_unauthorized() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state();
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build client");

    let response = client
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "192.168.1.50")
        .send()
        .await
        .expect("send remote request");

    assert_eq!(response.status(), 401);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn remote_presence_with_valid_citadel_capability_is_authorized() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state_with_access(false, &["presence.read"], "10.0.0.5");
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = client();
    let snapshot: serde_json::Value = client
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "10.0.0.5")
        .header(reqwest::header::AUTHORIZATION, "Bearer test-secret")
        .send()
        .await
        .expect("send remote request")
        .error_for_status()
        .expect("remote status")
        .json()
        .await
        .expect("remote body");

    assert_eq!(snapshot["schema_version"], "arda.harness.presence.v1");

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn remote_presence_with_wrong_capability_is_unauthorized() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state_with_access(false, &["presence.write"], "10.0.0.5");
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = client();
    let response = client
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "10.0.0.5")
        .header(reqwest::header::AUTHORIZATION, "Bearer test-secret")
        .send()
        .await
        .expect("send remote request");

    assert_eq!(response.status(), 401);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn revoked_remote_presence_enrollment_is_unauthorized() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state_with_access(true, &["presence.read"], "10.0.0.5");
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let response = client()
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "10.0.0.5")
        .header(reqwest::header::AUTHORIZATION, "Bearer test-secret")
        .send()
        .await
        .expect("send remote request");

    assert_eq!(response.status(), 401);
    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn remote_presence_from_disallowed_network_is_unauthorized() {
    let harness_addr = "127.0.0.1:0".parse().unwrap();
    let state = harness_state_with_access(false, &["presence.read"], "10.0.0.5");
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let response = client()
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "10.0.0.6")
        .header(reqwest::header::AUTHORIZATION, "Bearer test-secret")
        .send()
        .await
        .expect("send remote request");

    assert_eq!(response.status(), 401);
    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

fn harness_state() -> HarnessState {
    harness_state_with_presence(HarnessPresenceState::default())
}

fn harness_state_with_access(
    revoked: bool,
    capabilities: &[&str],
    allowed_ip: &str,
) -> HarnessState {
    let contract = OutpostAccessContract {
        schema_version: OUTPOST_ACCESS_SCHEMA_VERSION.to_string(),
        enrollments: vec![OutpostEnrollment {
            outpost_id: "node-pi5-citadel-avatar".to_string(),
            bearer_env: "TEST_PRESENCE_BEARER".to_string(),
            capabilities: capabilities.iter().map(|value| value.to_string()).collect(),
            revoked,
            network_posture: NetworkPosture {
                allow_forwarded: true,
                allowed_ips: vec![allowed_ip.parse().expect("allowed IP")],
            },
        }],
    };
    let presence = HarnessPresenceState::from_access_contract(contract, |name| {
        (name == "TEST_PRESENCE_BEARER").then(|| "test-secret".to_string())
    })
    .expect("access contract");
    harness_state_with_presence(presence)
}

fn harness_state_with_presence(presence_inputs: HarnessPresenceState) -> HarnessState {
    HarnessState {
        harness_addr: "127.0.0.1:7878".to_string(),
        child_pids: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: Duration::from_secs(5),
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: Duration::from_secs(2),
        presence_inputs,
        workbench_root: std::env::temp_dir(),
        operator_id: "operator-0".to_string(),
    }
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build client")
}
