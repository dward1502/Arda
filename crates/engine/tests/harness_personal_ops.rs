use arda_core::personal_ops::{
    CaptureContent, CaptureRecordedEvent, CaptureSource, EvidenceClass, InboxCapture,
    ItemClassifiedEvent, PersonalItemKind, PersonalOpsEnvelope, PersonalOpsRecord,
};
use arda_engine::harness::presence::HarnessPresenceState;
use arda_engine::harness::{
    self, HarnessState, DEFAULT_HARNESS_ADDR, DEFAULT_MANWE_PROXY_TIMEOUT,
    DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use arda_engine::personal_ops::PersonalOpsLogStore;
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

fn captured_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 30, 12, 0, 0).unwrap()
}

fn make_capture() -> InboxCapture {
    InboxCapture {
        capture_id: Uuid::new_v4(),
        captured_at: captured_at(),
        source: CaptureSource::Text,
        content: CaptureContent {
            text: Some("Call the transplant coordinator".to_owned()),
            audio_reference: None,
        },
        attachments: Vec::new(),
        project_id: None,
        priority: None,
        due_at: None,
    }
}

fn capture_event(capture: InboxCapture) -> PersonalOpsRecord {
    PersonalOpsRecord::CaptureRecorded(CaptureRecordedEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        capture,
    })
}

fn classify_event(
    item_id: Uuid,
    kind: PersonalItemKind,
    evidence: EvidenceClass,
) -> PersonalOpsRecord {
    PersonalOpsRecord::ItemClassified(ItemClassifiedEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        item_id,
        kind,
        evidence_class: evidence,
        confidence: None,
        rationale: None,
    })
}

fn make_envelope(record: PersonalOpsRecord) -> PersonalOpsEnvelope<PersonalOpsRecord> {
    PersonalOpsEnvelope {
        schema_version: "arda.personal-ops.v1".to_owned(),
        record,
    }
}

fn base_state(workbench_root: std::path::PathBuf) -> HarnessState {
    HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
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

#[tokio::test]
async fn personal_ops_projection_endpoint_returns_empty_when_no_events() {
    let root = tempfile::tempdir().unwrap();
    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let (bound, harness_handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");

    let response: Value = reqwest::get(format!("http://{bound}/v1/personal-ops/projection"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["schema_version"], "arda.harness.personal-ops.v1");
    assert_eq!(response["projection"]["event_count"], 0);
    assert_eq!(response["projection"]["inbox"].as_array().unwrap().len(), 0);
    assert_eq!(response["projection"]["today"].as_array().unwrap().len(), 0);

    shutdown.notify_waiters();
    harness_handle.await.unwrap();
}

#[tokio::test]
async fn personal_ops_projection_endpoint_reflects_appended_events() {
    let root = tempfile::tempdir().unwrap();
    let store = PersonalOpsLogStore::new(root.path());
    let capture = make_capture();
    let capture_id = capture.capture_id;

    let envelopes = vec![
        make_envelope(capture_event(capture)),
        make_envelope(classify_event(
            capture_id,
            PersonalItemKind::Task,
            EvidenceClass::Inferred,
        )),
    ];

    for env in &envelopes {
        store.append(env).unwrap();
    }

    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let (bound, harness_handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");

    let response: Value = reqwest::get(format!("http://{bound}/v1/personal-ops/projection"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["projection"]["event_count"], 2);
    assert_eq!(response["projection"]["inbox"].as_array().unwrap().len(), 0);
    assert_eq!(response["projection"]["today"].as_array().unwrap().len(), 1);

    shutdown.notify_waiters();
    harness_handle.await.unwrap();
}

#[tokio::test]
async fn capture_endpoint_creates_capture_and_appears_in_inbox() {
    let root = tempfile::tempdir().unwrap();
    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let (bound, harness_handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");

    let resp = reqwest::Client::new()
        .post(format!("http://{bound}/v1/personal/captures"))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "text": "Test capture from endpoint"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let inbox: Value = reqwest::get(format!("http://{bound}/v1/personal/inbox"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inbox["inbox"].as_array().unwrap().len(), 1);
    assert_eq!(
        inbox["inbox"][0]["content"].as_str().unwrap(),
        "Test capture from endpoint"
    );

    shutdown.notify_waiters();
    harness_handle.await.unwrap();
}

#[tokio::test]
async fn reminder_attempt_and_acknowledge_flow_updates_projection() {
    let root = tempfile::tempdir().unwrap();
    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let (bound, harness_handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");

    // Create a capture
    let create_resp = reqwest::Client::new()
        .post(format!("http://{bound}/v1/personal/captures"))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "text": "Take medication"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), 201);
    let capture_id = create_resp.json::<serde_json::Value>().await.unwrap()["capture_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Classify as a reminder
    let classify_resp = reqwest::Client::new()
        .post(format!(
            "http://{bound}/v1/personal/items/{capture_id}/classify"
        ))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "item_id": capture_id,
            "kind": "reminder",
            "evidence_class": "operator_authored"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(classify_resp.status(), 201);

    // Record a reminder attempt with state "delivered"
    let reminder_uuid = uuid::Uuid::new_v4().to_string();
    let attempt_resp = reqwest::Client::new()
        .post(format!("http://{bound}/v1/personal/reminders/attempt"))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "item_id": capture_id,
            "reminder_id": reminder_uuid,
            "state": "delivered",
            "provider_message_id": "msg-123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(attempt_resp.status(), 201);

    // Projection should show the reminder state on the item
    let proj: Value = reqwest::get(format!("http://{bound}/v1/personal-ops/projection"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(proj["projection"]["event_count"], 3);
    let today_items = proj["projection"]["today"].as_array().unwrap();
    assert_eq!(today_items.len(), 1);
    let item = &today_items[0];
    assert_eq!(item["reminder_state"]["attempt_count"].as_u64().unwrap_or(0), 1);

    // Acknowledge the reminder
    let ack_resp = reqwest::Client::new()
        .post(format!(
            "http://{bound}/v1/personal/reminders/{reminder_uuid}/acknowledge"
        ))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "state": "acknowledged",
            "receipt_reference": "ack-ref-456"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(ack_resp.status(), 201);

    // Projection should now show 4 events (capture + classify + attempt + ack)
    let proj: Value = reqwest::get(format!("http://{bound}/v1/personal-ops/projection"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(proj["projection"]["event_count"], 4);

    shutdown.notify_waiters();
    harness_handle.await.unwrap();
}

#[tokio::test]
async fn reminder_attempt_rejects_non_loopback_bind() {
    let root = tempfile::tempdir().unwrap();
    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let result = harness::serve(
        Some("0.0.0.0:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("loopback"));
}

#[tokio::test]
async fn classify_and_complete_flow_updates_projection() {
    let root = tempfile::tempdir().unwrap();
    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let (bound, harness_handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");

    // Create a capture
    let create_resp = reqwest::Client::new()
        .post(format!("http://{bound}/v1/personal/captures"))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "text": "Write the plan"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), 201);
    let capture_id = create_resp.json::<serde_json::Value>().await.unwrap()["capture_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Inbox should have 1 item
    let inbox: Value = reqwest::get(format!("http://{bound}/v1/personal/inbox"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inbox["inbox"].as_array().unwrap().len(), 1);

    // Classify it
    let classify_resp = reqwest::Client::new()
        .post(format!(
            "http://{bound}/v1/personal/items/{capture_id}/classify"
        ))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "item_id": capture_id,
            "kind": "task",
            "evidence_class": "operator_authored"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(classify_resp.status(), 201);

    // After classification, inbox should be empty
    let inbox: Value = reqwest::get(format!("http://{bound}/v1/personal/inbox"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inbox["inbox"].as_array().unwrap().len(), 0);

    // Complete it
    let complete_resp = reqwest::Client::new()
        .post(format!(
            "http://{bound}/v1/personal/items/{capture_id}/complete"
        ))
        .json(&serde_json::json!({
            "operator_id": "operator-0"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(complete_resp.status(), 201);

    // Projection should show 1 completed item
    let proj: Value = reqwest::get(format!("http://{bound}/v1/personal-ops/projection"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(proj["projection"]["completed"].as_array().unwrap().len(), 1);
    assert_eq!(proj["projection"]["event_count"], 3);

    shutdown.notify_waiters();
    harness_handle.await.unwrap();
}

#[tokio::test]
async fn resume_and_brief_endpoints_work() {
    let root = tempfile::tempdir().unwrap();
    let state = base_state(root.path().to_path_buf());
    let shutdown = Arc::new(Notify::new());
    let (bound, harness_handle) = harness::serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");

    // POST a capture
    let create_resp = reqwest::Client::new()
        .post(format!("http://{bound}/v1/personal/captures"))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "text": "Finish report"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), 201);
    let capture_id = create_resp.json::<serde_json::Value>().await.unwrap()["capture_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Classify to move from inbox to today
    let classify_resp = reqwest::Client::new()
        .post(format!(
            "http://{bound}/v1/personal/items/{capture_id}/classify"
        ))
        .json(&serde_json::json!({
            "operator_id": "operator-0",
            "item_id": capture_id,
            "kind": "task",
            "evidence_class": "operator_authored"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(classify_resp.status(), 201);

    // Resume endpoint
    let resume: Value = reqwest::get(format!("http://{bound}/v1/personal/resume"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resume["schema_version"], "arda.harness.personal-ops.v1");
    assert!(resume["resume"]["summary"].as_str().unwrap().len() > 0);
    assert_eq!(resume["resume"]["inbox_count"], 0);
    assert_eq!(resume["resume"]["today_count"], 1);

    // Brief endpoint
    let brief: Value = reqwest::get(format!("http://{bound}/v1/personal/briefs/today"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(brief["schema_version"], "arda.harness.personal-ops.v1");
    assert_eq!(brief["brief"]["today"].as_array().unwrap().len(), 1);

    shutdown.notify_waiters();
    harness_handle.await.unwrap();
}
