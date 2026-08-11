use crate::commands::monitor_surface::registry::{
    MonitorSessionRecord, SessionRegistryDocument, WorkstationHandoff,
    MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
};
use crate::commands::monitor_surface::typed::TypedMonitorSurfaceState;

fn record(slot_id: &str, owner: &str, content: serde_json::Value) -> MonitorSessionRecord {
    let session_id = format!("session-{slot_id}");
    let kind = content["kind"].as_str().unwrap().to_string();
    MonitorSessionRecord {
        slot_id: slot_id.to_string(),
        session_id: session_id.clone(),
        surface_session_id: session_id.clone(),
        owner: owner.to_string(),
        kind,
        revision: 1,
        opened_at_utc: chrono::Utc::now().to_rfc3339(),
        lease_expires_at_utc: (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
        content,
        playback: None,
        workstation_handoff: WorkstationHandoff {
            session_id,
            mode: "same_live_session".to_string(),
        },
        created_at_utc: chrono::Utc::now().to_rfc3339(),
        updated_at_utc: chrono::Utc::now().to_rfc3339(),
    }
}

#[test]
fn typed_state_preserves_five_distinct_descriptors() {
    let state = TypedMonitorSurfaceState::new();
    let descriptors = [
        serde_json::json!({"kind": "web", "url": "https://example.invalid"}),
        serde_json::json!({"kind": "youtube", "videoId": "dQw4w9WgXcQ"}),
        serde_json::json!({"kind": "document", "source": {"kind": "local", "path": "docs/a.md"}, "documentKind": "markdown"}),
        serde_json::json!({"kind": "terminal", "sessionId": "terminal-main", "readOnly": true}),
        serde_json::json!({"kind": "remote_session", "sessionId": "remote-main", "streamUrl": "https://example.invalid/live.m3u8", "transport": "hls"}),
    ];

    for (index, descriptor) in descriptors.into_iter().enumerate() {
        state
            .claim_session(record(
                &format!("monitor_{}", index + 1),
                &format!("agent-{}", index + 1),
                descriptor,
            ))
            .unwrap();
    }

    let snapshot = state.snapshot();
    assert_eq!(snapshot.sessions.len(), 5);
    assert_eq!(snapshot.sessions["monitor_1"].content["kind"], "web");
    assert_eq!(
        snapshot.sessions["monitor_2"].content["videoId"],
        "dQw4w9WgXcQ"
    );
    assert_eq!(
        snapshot.sessions["monitor_3"].content["documentKind"],
        "markdown"
    );
    assert_eq!(
        snapshot.sessions["monitor_4"].content["sessionId"],
        "terminal-main"
    );
    assert_eq!(snapshot.sessions["monitor_5"].content["transport"], "hls");
}

#[test]
fn typed_state_conflict_and_wrong_owner_release_do_not_mutate() {
    let state = TypedMonitorSurfaceState::new();
    state
        .claim_session(record(
            "monitor_1",
            "agent-one",
            serde_json::json!({"kind": "web", "url": "https://example.invalid"}),
        ))
        .unwrap();

    let conflict = state.claim_session(record(
        "monitor_1",
        "agent-two",
        serde_json::json!({"kind": "terminal", "sessionId": "other"}),
    ));
    assert!(conflict.is_err());
    assert!(state.release_session("monitor_1", "agent-two").is_err());
    assert_eq!(state.snapshot().sessions.len(), 1);
    assert_eq!(state.snapshot().sessions["monitor_1"].owner, "agent-one");
}

#[test]
fn typed_state_refresh_preserves_descriptor_and_advances_revision() {
    let state = TypedMonitorSurfaceState::new();
    let session = record(
        "monitor_1",
        "agent-one",
        serde_json::json!({"kind": "terminal", "sessionId": "terminal-main", "readOnly": true}),
    );
    let content = session.content.clone();
    state.claim_session(session).unwrap();
    state
        .refresh_session("monitor_1", "agent-one", 2, 120)
        .unwrap();

    let refreshed = &state.snapshot().sessions["monitor_1"];
    assert_eq!(refreshed.revision, 2);
    assert_eq!(refreshed.content, content);
}

#[test]
fn typed_state_restore_rejects_stale_schema() {
    let state = TypedMonitorSurfaceState::new();
    let stale = SessionRegistryDocument {
        schema_version: "arda.monitor-session-registry.v0".to_string(),
        updated_at_utc: chrono::Utc::now().to_rfc3339(),
        sessions: Default::default(),
    };
    assert!(state.restore(stale).is_err());
    assert_eq!(
        state.snapshot().schema_version,
        MONITOR_SESSION_REGISTRY_SCHEMA_VERSION
    );
}

#[test]
fn typed_state_restarts_from_durable_five_session_registry() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("monitor-session-registry.json");
    let descriptors = [
        serde_json::json!({"kind": "web", "url": "https://example.invalid"}),
        serde_json::json!({"kind": "youtube", "videoId": "dQw4w9WgXcQ"}),
        serde_json::json!({"kind": "document", "source": {"kind": "local", "path": "docs/a.md"}, "documentKind": "markdown"}),
        serde_json::json!({"kind": "terminal", "sessionId": "terminal-main", "readOnly": true}),
        serde_json::json!({"kind": "remote_session", "sessionId": "remote-main", "streamUrl": "https://example.invalid/live.m3u8", "transport": "hls"}),
    ];

    {
        let state = TypedMonitorSurfaceState::with_persistence_path(path.clone()).unwrap();
        for (index, descriptor) in descriptors.into_iter().enumerate() {
            state
                .claim_session(record(
                    &format!("monitor_{}", index + 1),
                    &format!("agent-{}", index + 1),
                    descriptor,
                ))
                .unwrap();
        }
    }

    let restarted = TypedMonitorSurfaceState::with_persistence_path(path).unwrap();
    let snapshot = restarted.snapshot();
    assert_eq!(snapshot.sessions.len(), 5);
    for index in 1..=5 {
        let slot = format!("monitor_{index}");
        assert_eq!(snapshot.sessions[&slot].slot_id, slot);
        assert_eq!(snapshot.sessions[&slot].owner, format!("agent-{index}"));
        assert_eq!(
            snapshot.sessions[&slot].workstation_handoff.session_id,
            format!("session-monitor_{index}")
        );
    }
}

#[test]
fn typed_state_rejects_corrupt_durable_registry() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("monitor-session-registry.json");
    std::fs::write(&path, b"not-json").unwrap();

    let error = TypedMonitorSurfaceState::with_persistence_path(path).unwrap_err();
    assert!(error.contains("parse durable monitor session registry"));
}
