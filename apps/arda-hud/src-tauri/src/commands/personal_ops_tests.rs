use super::*;
use chrono::TimeZone;
use serde_json::json;

#[test]
fn personal_ops_frontend_intents_cannot_supply_operator_or_idempotency_authority() {
    let capture = serde_json::from_value::<CaptureIntent>(json!({
        "text": "Call the coordinator",
        "operatorId": "browser-operator",
        "idempotencyKey": "browser-key"
    }));
    assert!(capture.is_err());

    let classify = serde_json::from_value::<ClassificationIntent>(json!({
        "itemId": "item-1",
        "kind": "task",
        "evidenceClass": "inferred",
        "operatorId": "browser-operator"
    }));
    assert!(classify.is_err());
}

#[test]
fn rust_owned_personal_ops_idempotency_retries_pending_and_scopes_operators() {
    let first = pending_idempotency_key("operator-a", "capture", "Buy tea");
    let retry = pending_idempotency_key("operator-a", "capture", "Buy tea");
    let other_operator = pending_idempotency_key("operator-b", "capture", "Buy tea");
    assert_eq!(first, retry);
    assert_ne!(first, other_operator);
    assert!(first.starts_with("personal-ops-capture-"));
    complete_pending_mutation("operator-a", "capture", "Buy tea");
    let later_capture = pending_idempotency_key("operator-a", "capture", "Buy tea");
    assert_ne!(first, later_capture);
}

#[test]
fn one_durable_projection_becomes_a_versioned_snapshot() {
    let generated = Utc.with_ymd_and_hms(2026, 8, 11, 12, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 11, 12, 1, 0).unwrap();
    let projection = json!({
        "schema_version": "arda.harness.personal-ops.v1",
        "projection": {
            "generated_at": generated.to_rfc3339(),
            "event_count": 2,
            "inbox": [{
                "capture_id": "capture-1",
                "operator_id": "operator-a",
                "content": "Inbox note",
                "audio_reference": null,
                "occurred_at": generated.to_rfc3339()
            }],
            "today": [],
            "waiting": [],
            "scheduled": [],
            "completed": []
        }
    });
    let snapshot = project_snapshot(projection, now).expect("project snapshot");
    assert_eq!(snapshot.schema_version, PERSONAL_OPS_PROJECTION_SCHEMA);
    assert_eq!(snapshot.state, PersonalOpsLoadState::Healthy);
    assert!(snapshot.source_revision.starts_with("personal-ops-"));
    assert_eq!(snapshot.inbox["inbox"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot.resume["resume"]["inbox_count"], 1);
}

#[test]
fn source_revision_tracks_durable_content_not_projection_read_time() {
    let now = Utc.with_ymd_and_hms(2026, 8, 11, 12, 1, 0).unwrap();
    let projection = |generated_at: &str| {
        json!({
            "projection": {
                "generated_at": generated_at,
                "event_count": 1,
                "inbox": [],
                "today": [],
                "waiting": []
            }
        })
    };
    let first = project_snapshot(projection("2026-08-11T12:00:00Z"), now).unwrap();
    let reread = project_snapshot(projection("2026-08-11T12:00:30Z"), now).unwrap();
    assert_eq!(first.source_revision, reread.source_revision);
}

#[test]
fn stale_projection_reports_explicit_recovery() {
    let generated = Utc.with_ymd_and_hms(2026, 8, 11, 11, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 11, 12, 0, 0).unwrap();
    let snapshot = project_snapshot(
        json!({
            "projection": {
                "generated_at": generated.to_rfc3339(),
                "event_count": 0,
                "inbox": [],
                "today": [],
                "waiting": []
            }
        }),
        now,
    )
    .expect("stale snapshot");
    assert_eq!(snapshot.state, PersonalOpsLoadState::Stale);
    assert!(snapshot.recovery_action.is_some());
}

#[test]
fn delete_requires_exact_rust_checked_confirmation() {
    let accepted = serde_json::from_value::<DeletePersonalDataIntent>(json!({
        "confirmation": "delete-personal-data"
    }))
    .expect("typed confirmation");
    assert_eq!(accepted.confirmation, "delete-personal-data");
    let fabricated = serde_json::from_value::<DeletePersonalDataIntent>(json!({
        "confirmation": "delete-personal-data",
        "operatorId": "browser-operator"
    }));
    assert!(fabricated.is_err());
}
