//! Integration tests for the presence harness surface.
//!
//! Verifies:
//! - `/v1/presence/snapshot` returns a deterministic, versioned projection
//! - `/v1/presence/events` returns SSE with `text/event-stream`
//! - Loopback callers are authorized
//! - Remote callers without enrolled outpost identity / `presence.read`
//!   capability receive 401; with proper credentials receive 200.

use std::{sync::Arc, time::Duration};

use axum::{Router, routing::get};
use reqwest::Client;
use tokio::sync::Notify;

use arda_engine::harness::{HarnessState, serve};
use arda_engine::harness::presence::{PresenceRouter, HarnessPresenceState};

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
    assert_eq!(snapshot["snapshot"]["schema_version"], "arda.runtime-presence.v1");
    assert!(snapshot["snapshot_sequence"].as_u64().unwrap() >= 1);
    assert!(snapshot["generated_at"].as_str().unwrap().len() > 0);

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
    let state = harness_state();
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = client();
    let snapshot: serde_json::Value = client
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "10.0.0.5")
        .header(
            reqwest::header::AUTHORIZATION,
            "Bearer citadel-outpost-1:presence.read",
        )
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
    let state = harness_state();
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(Some(harness_addr), state, shutdown.clone())
        .await
        .expect("start harness");

    let client = client();
    let response = client
        .get(format!("http://{bound}/v1/presence/snapshot"))
        .header("x-forwarded-for", "10.0.0.5")
        .header(
            reqwest::header::AUTHORIZATION,
            "Bearer citadel-outpost-1:presence.write",
        )
        .send()
        .await
        .expect("send remote request");

    assert_eq!(response.status(), 401);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

fn harness_state() -> HarnessState {
    HarnessState {
        harness_addr: "127.0.0.1:7878".to_string(),
        child_pids: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: Duration::from_secs(5),
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: Duration::from_secs(2),
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: std::env::temp_dir(),
    }
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build client")
}
