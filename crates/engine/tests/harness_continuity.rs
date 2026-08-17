use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use chrono::{Duration, Utc};
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
    let state = HarnessState {
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
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator-1".into(),
    };
    let shutdown = Arc::new(Notify::new());
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .unwrap();
    (bound, shutdown, handle)
}

fn continuity_event(key: &str) -> Value {
    let now = Utc::now();
    json!({
        "operator": {
            "operator_id": "operator-1",
            "authenticated": true,
            "authentication_method": "gateway_identity",
            "authenticated_at": now.to_rfc3339()
        },
        "event": {
            "schema_version": "arda.continuity-event.v1",
            "event_id": "continuity-event-1",
            "session_lineage_id": "lineage-1",
            "current_session_id": "session-1",
            "surface_id": "discord:private-chat",
            "platform": "discord",
            "chat_id": "private-chat",
            "thread_id": null,
            "privacy_class": "personal_device",
            "authorized_domains": ["system"],
            "requested_domains": ["system"],
            "topic_refs": ["topic:phase-2"],
            "commitment_refs": ["commitment:finish-phase-2"],
            "memory_scope_refs": ["vaire:scope:system-continuity"],
            "observed_at": now.to_rfc3339(),
            "expires_at": (now + Duration::minutes(15)).to_rfc3339(),
            "idempotency_key": key
        }
    })
}

fn handoff() -> Value {
    let now = Utc::now();
    json!({
        "operator": {
            "operator_id": "operator-1",
            "authenticated": true,
            "authentication_method": "gateway_identity",
            "authenticated_at": now.to_rfc3339()
        },
        "handoff": {
            "schema_version": "arda.surface-handoff.v1",
            "handoff_id": "handoff-1",
            "operator_ref": "operator-1",
            "session_lineage_id": "lineage-1",
            "current_session_id": "session-1",
            "source_surface_id": "discord:private-chat",
            "destination_surface_id": "desktop:arda-hud",
            "topic_refs": ["topic:phase-2"],
            "commitment_refs": ["commitment:finish-phase-2"],
            "memory_scope_refs": ["vaire:scope:system-continuity"],
            "authorized_domains": ["system"],
            "requested_domains": ["system"],
            "privacy_class": "personal_device",
            "consent": {"state": "requested", "requesting_actor": "operator-1"},
            "state": "requested",
            "issued_at": now.to_rfc3339(),
            "expires_at": (now + Duration::minutes(15)).to_rfc3339(),
            "accepted_at": null,
            "idempotency_key": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "reason": "continue here",
            "error": null,
            "receipt_refs": []
        }
    })
}

#[tokio::test]
async fn continuity_endpoints_enforce_identity_replay_and_transitions() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    let key = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let event = continuity_event(key);
    let created = client
        .post(format!("http://{bound}/v1/continuity/events"))
        .json(&event)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 201);
    let replay: Value = client
        .post(format!("http://{bound}/v1/continuity/events"))
        .json(&event)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(replay["replayed"], true);

    let mut altered = continuity_event(key);
    altered["event"]["surface_id"] = json!("discord:other-chat");
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/continuity/events"))
            .json(&altered)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );

    let mut unauthorized =
        continuity_event("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    unauthorized["operator"]["operator_id"] = json!("other-operator");
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/continuity/events"))
            .json(&unauthorized)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );

    let prepared: Value = client
        .post(format!("http://{bound}/v1/handoffs"))
        .json(&handoff())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(prepared["handoff"]["state"], "prepared");

    let accept_body = json!({
        "operator_ref": "operator-1",
        "idempotency_key": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
    });
    let accepted: Value = client
        .post(format!("http://{bound}/v1/handoffs/handoff-1/accept"))
        .json(&accept_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(accepted["handoff"]["state"], "accepted");

    let accepted_replay: Value = client
        .post(format!("http://{bound}/v1/handoffs/handoff-1/accept"))
        .json(&accept_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(accepted_replay["replayed"], true);
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/handoffs/handoff-1/accept"))
            .json(&json!({
                "operator_ref": "operator-1",
                "idempotency_key": "sha256:9999999999999999999999999999999999999999999999999999999999999999"
            }))
            .send()
            .await
            .unwrap()
            .status(),
        409
    );

    let fetched: Value = client
        .get(format!("http://{bound}/v1/handoffs/handoff-1"))
        .header("x-arda-operator-id", "operator-1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched["handoff"]["state"], "accepted");

    let session: Value = client
        .get(format!("http://{bound}/v1/continuity/sessions/lineage-1"))
        .header("x-arda-operator-id", "operator-1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(session["session_lineage_id"], "lineage-1");
    assert!(session.get("transcript").is_none());

    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn continuity_rejects_unknown_expired_and_cross_domain_payloads() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    let mut unknown =
        continuity_event("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    unknown["event"]["schema_version"] = json!("arda.continuity-event.v0");
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/continuity/events"))
            .json(&unknown)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );

    let mut expired =
        continuity_event("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    expired["event"]["expires_at"] = json!((Utc::now() - Duration::seconds(1)).to_rfc3339());
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/continuity/events"))
            .json(&expired)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );

    let mut escalated = handoff();
    escalated["handoff"]["requested_domains"] = json!(["system", "business"]);
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/handoffs"))
            .json(&escalated)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );

    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn continuity_projection_is_safe_for_empty_and_shared_surfaces() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    let empty: Value = client
        .get(format!("http://{bound}/v1/continuity/projection"))
        .header("x-arda-operator-id", "operator-1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(empty["active"], false);

    let mut shared =
        continuity_event("sha256:1212121212121212121212121212121212121212121212121212121212121212");
    shared["event"]["privacy_class"] = json!("shared_room");
    shared["event"]["surface_id"] = json!("discord:shared-room");
    client
        .post(format!("http://{bound}/v1/continuity/events"))
        .json(&shared)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let projection: Value = client
        .get(format!("http://{bound}/v1/continuity/projection"))
        .header("x-arda-operator-id", "operator-1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projection["active"], true);
    assert_eq!(projection["surface_id"], "discord:shared-room");
    assert_eq!(projection["private_refs_withheld"], true);
    assert_eq!(projection["topic_refs"], json!([]));
    assert_eq!(projection["commitment_refs"], json!([]));
    assert_eq!(projection["memory_scope_refs"], json!([]));

    shutdown.notify_waiters();
    handle.await.unwrap();
}
