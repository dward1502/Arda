use arda_outpost_protocol::{
    DegradedReason, RuntimePresenceProjection, SceneState, RUNTIME_PRESENCE_SCHEMA_VERSION,
};
use chrono::{Duration, TimeZone, Utc};

fn fixture() -> RuntimePresenceProjection {
    serde_json::from_str(include_str!(
        "../../../spec/runtime-presence/v1/example.json"
    ))
    .expect("runtime presence example must deserialize")
}

#[test]
fn example_round_trip_preserves_the_v1_contract() {
    let projection = fixture();
    assert_eq!(projection.schema_version, RUNTIME_PRESENCE_SCHEMA_VERSION);
    assert_eq!(projection.nodes.len(), 3);
    assert_eq!(projection.edges.len(), 1);

    let encoded = serde_json::to_value(&projection).expect("encode runtime presence");
    let decoded: RuntimePresenceProjection =
        serde_json::from_value(encoded).expect("decode runtime presence");
    assert_eq!(decoded, projection);
}

#[test]
fn fresh_receipted_projection_is_active() {
    let projection = fixture();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 30, 19, 0, 10)
        .single()
        .expect("valid timestamp");

    let disposition = projection.scene_disposition_at(now);
    assert_eq!(disposition.state, SceneState::Active);
    assert_eq!(disposition.degraded_reason, None);
}

#[test]
fn expired_projection_fails_closed_to_idle_degraded() {
    let projection = fixture();
    let disposition =
        projection.scene_disposition_at(projection.valid_until + Duration::seconds(1));

    assert_eq!(disposition.state, SceneState::IdleDegraded);
    assert_eq!(disposition.degraded_reason, Some(DegradedReason::Expired));
}

#[test]
fn unreceipted_projection_fails_closed_to_idle_degraded() {
    let mut projection = fixture();
    projection.source_receipt_refs.clear();

    let disposition = projection.scene_disposition_at(projection.generated_at);
    assert_eq!(disposition.state, SceneState::IdleDegraded);
    assert_eq!(
        disposition.degraded_reason,
        Some(DegradedReason::Unverifiable)
    );
}

#[test]
fn out_of_range_pressure_fails_closed_to_idle_degraded() {
    let mut projection = fixture();
    projection.nodes[1]
        .resource_pressure
        .as_mut()
        .expect("fixture pressure")
        .provider = 1.01;

    let disposition = projection.scene_disposition_at(projection.generated_at);
    assert_eq!(disposition.state, SceneState::IdleDegraded);
    assert_eq!(
        disposition.degraded_reason,
        Some(DegradedReason::InvalidSignal)
    );
}

#[test]
fn fixture_contains_no_private_content_payload_fields() {
    fn visit(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    assert!(
                        !matches!(
                            key.as_str(),
                            "prompt"
                                | "message"
                                | "messages"
                                | "payload"
                                | "secret"
                                | "health_detail"
                        ),
                        "private content field must not appear in the contract fixture: {key}"
                    );
                    visit(child);
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(visit),
            _ => {}
        }
    }

    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/runtime-presence/v1/example.json"
    ))
    .expect("parse fixture");
    visit(&fixture);
}

#[test]
fn private_payload_fields_are_rejected_by_the_rust_contract() {
    let mut value = serde_json::to_value(fixture()).expect("encode fixture");
    value["prompt"] = serde_json::json!("private task content");

    assert!(serde_json::from_value::<RuntimePresenceProjection>(value).is_err());
}
