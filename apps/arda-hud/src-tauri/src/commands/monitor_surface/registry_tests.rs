use crate::commands::monitor_surface::registry::{
    parse_session_registry_document, session_registry_document_json, MonitorSessionRecord,
    MonitorSessionRegistryState, WorkstationHandoff, MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
};

fn base_record() -> MonitorSessionRecord {
    MonitorSessionRecord {
        slot_id: "monitor_1".to_string(),
        session_id: "session-web".to_string(),
        surface_session_id: "session-web".to_string(),
        owner: "agent-web".to_string(),
        kind: "web".to_string(),
        revision: 1,
        opened_at_utc: "2026-08-06T20:00:00.000Z".to_string(),
        lease_expires_at_utc: "2099-08-06T22:00:00.000Z".to_string(),
        content: serde_json::json!({
            "kind": "web",
            "url": "https://example.invalid/dashboard",
            "display": "capture_stream",
            "sandboxProfile": "default"
        }),
        playback: None,
        workstation_handoff: WorkstationHandoff {
            session_id: "session-web".to_string(),
            mode: "same_live_session".to_string(),
        },
        created_at_utc: "2026-08-06T20:00:00.000Z".to_string(),
        updated_at_utc: "2026-08-06T20:00:00.000Z".to_string(),
    }
}

#[test]
fn registry_stores_multiple_active_sessions() {
    let registry = MonitorSessionRegistryState::default();
    for index in 1..=5 {
        let mut record = base_record();
        record.slot_id = format!("monitor_{index}");
        record.owner = format!("agent-{index}");
        record.session_id = format!("session-{index}");
        record.surface_session_id = record.session_id.clone();
        record.workstation_handoff.session_id = record.session_id.clone();
        registry.insert_session(record);
    }
    let document = registry.claim_snapshot();
    assert_eq!(document.sessions.len(), 5);
    for index in 1..=5 {
        let active = registry
            .active_session(&format!("monitor_{index}"))
            .unwrap();
        assert_eq!(active.owner, format!("agent-{index}"));
    }
}

#[test]
fn registry_expiry_isolates_one_slot() {
    let registry = MonitorSessionRegistryState::default();
    let mut expired = base_record();
    expired.lease_expires_at_utc = "2020-01-01T00:00:00.000Z".to_string();
    registry.insert_session(expired);
    let mut live = base_record();
    live.slot_id = "monitor_2".to_string();
    live.owner = "agent-2".to_string();
    registry.insert_session(live);

    assert!(registry.active_session("monitor_1").is_none());
    assert!(registry.active_session("monitor_2").is_some());
}

#[test]
fn registry_json_round_trip_preserves_typed_descriptor_fields() {
    let registry = MonitorSessionRegistryState::default();
    let mut record = base_record();
    record.session_id = "session-roundtrip".to_string();
    record.surface_session_id = record.session_id.clone();
    record.workstation_handoff.session_id = record.session_id.clone();
    registry.insert_session(record);

    let snapshot = registry.claim_snapshot();
    let json = session_registry_document_json(&snapshot).unwrap();
    assert!(json.contains(MONITOR_SESSION_REGISTRY_SCHEMA_VERSION));

    let reparsed = parse_session_registry_document(Some(&json)).unwrap();
    assert_eq!(reparsed.sessions.len(), 1);
    let content = &reparsed.sessions["monitor_1"].content;
    assert_eq!(content["kind"], "web");
    assert_eq!(content["url"], "https://example.invalid/dashboard");
    assert_eq!(content["display"], "capture_stream");
}

#[test]
fn registry_restore_rejects_stale_schema_without_mutation() {
    let registry = MonitorSessionRegistryState::default();
    registry.insert_session(base_record());
    let mut stale = registry.claim_snapshot();
    stale.schema_version = "arda.monitor-session-registry.v0".to_string();

    assert!(registry.restore(stale).is_err());
    assert_eq!(registry.claim_snapshot().sessions.len(), 1);
}
