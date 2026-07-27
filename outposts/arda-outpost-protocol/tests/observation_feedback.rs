use arda_outpost_protocol::{AgentFeedback, AuthorityClass, OutpostObservation, SCHEMA_VERSION};

#[test]
fn scout_feedback_round_trip_preserves_fields_schema_and_confidence() {
    let feedback = AgentFeedback::new(
        "arda-outpost-scout",
        "crates".to_string(),
        arda_outpost_protocol::ObservationClassification::DerivedEstimate,
        AuthorityClass::Advisory,
        0.8,
        SCHEMA_VERSION.to_string(),
        serde_json::json!({"path":"crates/foo","status":"active"}),
    );

    let observation: OutpostObservation = feedback.try_into().expect("schema matches");

    assert_eq!(observation.source, "arda-outpost-scout");
    assert_eq!(
        observation.payload.get("status").and_then(|v| v.as_str()),
        Some("active")
    );
    assert_eq!(
        observation.provenance.as_deref(),
        Some("arda-outpost-scout://survey")
    );
}

#[test]
fn schema_mismatch_rejects_with_conversion_error() {
    let feedback = AgentFeedback::new(
        "arda-outpost-scout",
        "crates".to_string(),
        arda_outpost_protocol::ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        0.65,
        "0.0.0".to_string(),
        serde_json::json!({"path":"crates/foo"}),
    );

    let err = OutpostObservation::try_from(feedback).unwrap_err();
    assert!(format!("{}", err).contains("schema_version mismatch"));
}
