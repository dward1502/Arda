use crate::commands::monitor_surface::contract::{is_session_active, MonitorSurfaceContractState};
use crate::commands::monitor_surface::registry::{
    MonitorSessionRecord, WorkstationHandoff, MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
};
use std::sync::{Arc, Barrier};

const DEFAULT_LEASE_SECS: u64 = 7200;

fn base_record() -> MonitorSessionRecord {
    MonitorSessionRecord {
        slot_id: "monitor_1".to_string(),
        session_id: "session-web".to_string(),
        surface_session_id: "session-web".to_string(),
        owner: "agent-web".to_string(),
        kind: "web".to_string(),
        revision: 1,
        opened_at_utc: "2026-08-06T20:00:00.000Z".to_string(),
        lease_expires_at_utc: (chrono::Utc::now()
            + chrono::Duration::seconds(DEFAULT_LEASE_SECS as i64))
        .to_rfc3339(),
        content: serde_json::json!({
            "kind": "web",
            "url": "https://example.invalid/dashboard",
            "display": "capture_stream",
            "sandboxProfile": "default"
        }),
        workstation_handoff: WorkstationHandoff {
            session_id: "session-web".to_string(),
            mode: "same_live_session".to_string(),
        },
        created_at_utc: "2026-08-06T20:00:00.000Z".to_string(),
        updated_at_utc: "2026-08-06T20:00:00.000Z".to_string(),
    }
}

#[test]
fn contract_claims_five_independent_monitor_sessions() {
    let state = MonitorSurfaceContractState::new();
    for index in 1..=5 {
        let mut record = base_record();
        record.slot_id = format!("monitor_{index}");
        record.owner = format!("agent-{index}");
        record.session_id = format!("session-{index}");
        record.surface_session_id = record.session_id.clone();
        record.workstation_handoff.session_id = record.session_id.clone();
        state.claim_session(record).unwrap();
    }

    let snapshot = state.session_registry();
    assert_eq!(snapshot.sessions.len(), 5);
    for index in 1..=5 {
        assert_eq!(
            snapshot.sessions[&format!("monitor_{index}")].owner,
            format!("agent-{index}")
        );
    }
}

#[test]
fn contract_serializes_concurrent_claims_to_one_active_owner() {
    let state = Arc::new(MonitorSurfaceContractState::new());
    let barrier = Arc::new(Barrier::new(6));
    let mut handles = Vec::new();

    for index in 1..=5 {
        let state = Arc::clone(&state);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut record = base_record();
            record.owner = format!("concurrent-agent-{index}");
            record.session_id = format!("concurrent-session-{index}");
            record.surface_session_id = record.session_id.clone();
            record.workstation_handoff.session_id = record.session_id.clone();
            barrier.wait();
            state.claim_session(record)
        }));
    }
    barrier.wait();

    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(state.session_registry().sessions.len(), 1);
}

#[test]
fn contract_keeps_concurrent_release_and_reclaim_atomic() {
    let state = Arc::new(MonitorSurfaceContractState::new());
    state.claim_session(base_record()).unwrap();
    let barrier = Arc::new(Barrier::new(3));

    let release_state = Arc::clone(&state);
    let release_barrier = Arc::clone(&barrier);
    let release = std::thread::spawn(move || {
        release_barrier.wait();
        release_state.release_session("monitor_1", "agent-web")
    });

    let claim_state = Arc::clone(&state);
    let claim_barrier = Arc::clone(&barrier);
    let claim = std::thread::spawn(move || {
        let mut record = base_record();
        record.owner = "replacement-agent".to_string();
        record.session_id = "replacement-session".to_string();
        record.surface_session_id = record.session_id.clone();
        record.workstation_handoff.session_id = record.session_id.clone();
        claim_barrier.wait();
        claim_state.claim_session(record)
    });

    barrier.wait();
    assert!(release.join().unwrap().is_ok());
    let claim_result = claim.join().unwrap();
    let snapshot = state.session_registry();
    assert!(snapshot.sessions.len() <= 1);
    if claim_result.is_ok() {
        assert_eq!(snapshot.sessions["monitor_1"].owner, "replacement-agent");
    } else {
        assert!(snapshot.sessions.is_empty());
    }
}

#[test]
fn contract_rejects_unknown_slot_and_revision_conflict() {
    let state = MonitorSurfaceContractState::new();
    let record = base_record();
    state.claim_session(record.clone()).unwrap();

    let mut invalid = record.clone();
    invalid.slot_id = "view_desk_l".to_string();
    assert!(state
        .claim_session(invalid)
        .unwrap_err()
        .contains("not a canonical monitor slot"));

    assert!(state
        .refresh_session(&record.slot_id, &record.owner, 0, 60)
        .unwrap_err()
        .contains("revision conflict"));
}

#[test]
fn contract_wrong_owner_release_does_not_mutate() {
    let state = MonitorSurfaceContractState::new();
    let record = base_record();
    state.claim_session(record.clone()).unwrap();

    assert!(state
        .release_session(&record.slot_id, "other-owner")
        .unwrap_err()
        .contains("owned by"));
    assert_eq!(state.session_registry().sessions.len(), 1);
    assert_eq!(
        state.active_session(&record.slot_id).unwrap().owner,
        record.owner
    );

    let document = state
        .release_session(&record.slot_id, &record.owner)
        .unwrap();
    assert!(!document.sessions.contains_key(&record.slot_id));
}

#[test]
fn contract_refresh_preserves_content_and_advances_revision() {
    let state = MonitorSurfaceContractState::new();
    let record = base_record();
    state.claim_session(record.clone()).unwrap();

    let refreshed = state
        .refresh_session(&record.slot_id, &record.owner, record.revision + 1, 10)
        .unwrap();
    assert_eq!(refreshed.revision, record.revision + 1);
    assert_eq!(refreshed.content, record.content);
    assert!(is_session_active(
        &state.active_session(&record.slot_id).unwrap()
    ));
}

#[test]
fn contract_snapshot_and_restore_round_trip() {
    let state = MonitorSurfaceContractState::new();
    state.claim_session(base_record()).unwrap();

    let snapshot = state.active_snapshot();
    let json = state.session_json().unwrap();
    assert!(json.contains(MONITOR_SESSION_REGISTRY_SCHEMA_VERSION));

    let fresh = MonitorSurfaceContractState::new();
    fresh.restore(snapshot).unwrap();
    assert_eq!(
        fresh.active_session("monitor_1").unwrap().owner,
        "agent-web"
    );
}

#[test]
fn contract_rejects_stale_schema_on_restore() {
    let state = MonitorSurfaceContractState::new();
    let mut document = state.active_snapshot();
    document.schema_version = "arda.monitor-session-registry.v0".to_string();
    assert!(state
        .restore(document)
        .unwrap_err()
        .contains("schema version"));
}

#[test]
fn contract_rejects_invalid_content_without_mutation() {
    let state = MonitorSurfaceContractState::new();
    let mut record = base_record();
    record.content = serde_json::json!({"url": "https://example.invalid"});
    assert!(state.claim_session(record).is_err());
    assert!(state.session_registry().sessions.is_empty());
}

#[test]
fn is_session_active_returns_false_after_expiry() {
    let record = MonitorSessionRecord {
        lease_expires_at_utc: (chrono::Utc::now() - chrono::Duration::seconds(60)).to_rfc3339(),
        ..base_record()
    };
    assert!(!is_session_active(&record));
}
