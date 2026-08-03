use arda_core::personal_ops::{
    CaptureContent, CaptureSource, InboxCapture, PersonalOpsEnvelope, PersonalOpsRecord,
};
use arda_engine::personal_ops::PersonalOpsLogStore;
use chrono::{TimeZone, Utc};
use std::path::PathBuf;
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
    PersonalOpsRecord::CaptureRecorded(arda_core::personal_ops::CaptureRecordedEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        capture,
    })
}

fn make_envelope(record: PersonalOpsRecord) -> PersonalOpsEnvelope<PersonalOpsRecord> {
    PersonalOpsEnvelope::new(record)
}

fn temp_root() -> PathBuf {
    let root = tempfile::tempdir().unwrap();
    root.into_path()
}

#[test]
fn append_then_load_roundtrips_envelopes() {
    let root = temp_root();
    let store = PersonalOpsLogStore::new(&root);
    let envelope = make_envelope(capture_event(make_capture()));

    store.append(&envelope).expect("append succeeds");

    let loaded = store.load_all().expect("load succeeds");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].schema_version, envelope.schema_version);
    assert_eq!(loaded[0].record.event_id(), envelope.record.event_id());
}

#[test]
fn load_all_returns_empty_when_log_missing() {
    let root = temp_root();
    let store = PersonalOpsLogStore::new(&root);
    let loaded = store.load_all().expect("missing file yields empty");
    assert!(loaded.is_empty());
}

#[test]
fn multiple_events_load_in_order() {
    let root = temp_root();
    let store = PersonalOpsLogStore::new(&root);

    let env1 = make_envelope(capture_event(make_capture()));
    let env2 = make_envelope(capture_event(make_capture()));
    store.append(&env1).unwrap();
    store.append(&env2).unwrap();

    let loaded = store.load_all().unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].record.event_id(), env1.record.event_id());
    assert_eq!(loaded[1].record.event_id(), env2.record.event_id());
}

#[test]
fn canonical_path_is_data_personal_events_jsonl() {
    let root = temp_root();
    let store = PersonalOpsLogStore::new(&root);
    assert_eq!(store.events_path, root.join("data/personal/events.jsonl"));
}
