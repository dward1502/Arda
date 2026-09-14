use arda_outpost_protocol::{
    MirromereSurfaceProjection, MirromereSurfaceValidationError, MIRROMERE_SURFACE_SCHEMA_VERSION,
};
use chrono::{Duration, TimeZone, Utc};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "idle" => include_str!("../../../spec/mirromere-surface/v1/fixtures/ambient-idle.json"),
        "degraded" => {
            include_str!("../../../spec/mirromere-surface/v1/fixtures/system-degraded.json")
        }
        "handoff" => include_str!(
            "../../../spec/mirromere-surface/v1/fixtures/continuity-handoff-ready.json"
        ),
        other => panic!("unknown fixture {other}"),
    })
    .expect("fixture must parse as json")
}

fn parse_fixture(name: &str) -> MirromereSurfaceProjection {
    serde_json::from_value(fixture(name)).expect("fixture must deserialize")
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 17, 12, 1, 0)
        .single()
        .expect("valid timestamp")
}

#[test]
fn representative_scenes_round_trip_and_validate() {
    for name in ["idle", "degraded", "handoff"] {
        let projection = parse_fixture(name);
        assert_eq!(projection.schema_version, MIRROMERE_SURFACE_SCHEMA_VERSION);
        projection.validate_at(now()).expect("fixture validates");

        let encoded = serde_json::to_value(&projection).expect("encode projection");
        let decoded: MirromereSurfaceProjection =
            serde_json::from_value(encoded).expect("decode projection");
        assert_eq!(decoded, projection);
    }
}

#[test]
fn unknown_fields_are_rejected_by_strict_serde() {
    let mut value = fixture("idle");
    value["unexpected"] = json!(true);

    assert!(serde_json::from_value::<MirromereSurfaceProjection>(value).is_err());
}

#[test]
fn expired_scene_fails_closed_to_unavailable() {
    let mut projection = parse_fixture("idle");
    projection.expires_at = now() - Duration::seconds(1);

    assert_eq!(
        projection.validate_at(now()),
        Err(MirromereSurfaceValidationError::Expired)
    );
}

#[test]
fn privacy_escalation_above_visibility_ceiling_is_rejected() {
    let mut projection = parse_fixture("idle");
    projection.privacy.privacy_class =
        arda_outpost_protocol::MirromerePrivacyClass::OperatorPrivate;
    projection.privacy.visibility_ceiling =
        arda_outpost_protocol::MirromereVisibilityCeiling::PublicAmbient;

    assert_eq!(
        projection.validate_at(now()),
        Err(MirromereSurfaceValidationError::PrivacyEscalation)
    );
}

#[test]
fn unknown_interaction_id_is_rejected_before_render() {
    let mut value = fixture("idle");
    value["allowed_interactions"] = json!(["launch_shell"]);

    assert!(serde_json::from_value::<MirromereSurfaceProjection>(value).is_err());
}

#[test]
fn arbitrary_url_html_and_shell_payloads_are_rejected() {
    let mut media = fixture("idle");
    media["slots"][0]["content"] = json!({
        "kind": "media_ref",
        "asset_id": "hero",
        "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "mime_type": "image/png",
        "url": "https://example.invalid/raw.png"
    });
    assert!(serde_json::from_value::<MirromereSurfaceProjection>(media).is_err());

    let mut html = fixture("idle");
    html["slots"][0]["content"] = json!({ "kind": "text", "text": "<script>alert(1)</script>" });
    let projection: MirromereSurfaceProjection =
        serde_json::from_value(html).expect("text shape deserializes");
    assert_eq!(
        projection.validate_at(now()),
        Err(MirromereSurfaceValidationError::UnsafeContent)
    );

    let mut shell = fixture("idle");
    shell["slots"][0]["content"] =
        json!({ "kind": "app_view", "view_id": "terminal", "command": "rm -rf /" });
    assert!(serde_json::from_value::<MirromereSurfaceProjection>(shell).is_err());

    let mut metadata = parse_fixture("idle");
    metadata.scene.purpose = "<b>trusted</b>".to_string();
    assert_eq!(
        metadata.validate_at(now()),
        Err(MirromereSurfaceValidationError::UnsafeContent)
    );
}

#[test]
fn oversized_slot_collection_is_rejected() {
    let mut projection = parse_fixture("idle");
    let slot = projection.slots[0].clone();
    while projection.slots.len() <= arda_outpost_protocol::MIRROMERE_MAX_SLOTS {
        projection.slots.push(slot.clone());
    }

    assert_eq!(
        projection.validate_at(now()),
        Err(MirromereSurfaceValidationError::TooManySlots)
    );
}

#[test]
fn missing_evidence_source_is_rejected() {
    let mut projection = parse_fixture("idle");
    projection.evidence.clear();

    assert_eq!(
        projection.validate_at(now()),
        Err(MirromereSurfaceValidationError::MissingEvidence)
    );
}
