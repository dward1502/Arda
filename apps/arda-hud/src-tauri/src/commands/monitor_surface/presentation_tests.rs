use super::presentation::{present, PresentationRequest};
use super::typed::TypedMonitorSurfaceState;
use serde_json::json;

fn request(id: &str) -> PresentationRequest {
    serde_json::from_value(json!({"requestId":id,"ambientAllowed":true,"operation":"show","source":{"kind":"web","url":"https://example.org/"}})).unwrap()
}

#[test]
fn presentation_readiness_requires_successful_restore() {
    let state = TypedMonitorSurfaceState::new();
    assert!(!state.presentation_ready());
    state.restore(state.snapshot()).unwrap();
    assert!(state.presentation_ready());
    let original = present(&state, request("restored")).unwrap().session;
    state
        .restore(TypedMonitorSurfaceState::new().snapshot())
        .unwrap();
    assert_eq!(state.snapshot().sessions[&original.slot_id], original);
}

#[test]
fn presentation_requires_explicit_ambient_permission() {
    let state = TypedMonitorSurfaceState::new();
    let mut req = request("privacy");
    req.ambient_allowed = false;
    assert!(present(&state, req).unwrap_err().contains("ambient"));
    assert!(state.snapshot().sessions.is_empty());
}

#[test]
fn presentation_replay_preserves_native_session_and_does_not_steal() {
    let state = TypedMonitorSurfaceState::new();
    let first = present(&state, request("one")).unwrap();
    let replay = present(&state, request("one")).unwrap();
    assert_eq!(first.session, replay.session);
    assert_eq!(first.state, "published");
    assert_ne!(first.state, "rendered");
    let second = present(&state, request("two")).unwrap();
    assert_ne!(first.session.slot_id, second.session.slot_id);
    let mut occupied = request("three");
    occupied.slot_id = Some(first.session.slot_id);
    assert!(present(&state, occupied).unwrap_err().contains("occupied"));
    for id in ["three", "four", "five"] {
        present(&state, request(id)).unwrap();
    }
    assert!(present(&state, request("six"))
        .unwrap_err()
        .contains("occupied"));
    assert_eq!(state.snapshot().sessions.len(), 5);
}

#[test]
fn presentation_rejects_identity_injection_and_unsafe_sources() {
    assert!(serde_json::from_value::<PresentationRequest>(json!({"requestId":"x","ambientAllowed":true,"owner":"operator","operation":"show","source":{"kind":"web","url":"https://example.org"}})).is_err());
    let state = TypedMonitorSurfaceState::new();
    for url in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "http://127.0.0.1:7878",
        "https://user:pass@example.org",
        "https://localhost/",
    ] {
        let req = serde_json::from_value(json!({"requestId":"unsafe","ambientAllowed":true,"operation":"show","source":{"kind":"web","url":url}})).unwrap();
        assert!(present(&state, req).is_err(), "{url}");
    }
}

#[test]
fn presentation_replay_with_changed_content_fails() {
    let state = TypedMonitorSurfaceState::new();
    present(&state, request("one")).unwrap();
    let changed = serde_json::from_value(json!({"requestId":"one","ambientAllowed":true,"operation":"show","source":{"kind":"web","url":"https://example.com/"}})).unwrap();
    assert!(present(&state, changed).unwrap_err().contains("different"));
}
