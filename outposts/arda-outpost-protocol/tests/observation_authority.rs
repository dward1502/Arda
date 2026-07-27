use arda_outpost_protocol::{
    AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation, SCHEMA_VERSION,
};

#[test]
fn observation_json_round_trip_preserves_authoritative_fields() {
    let observation = OutpostObservation::new(
        "node-pi5-warden",
        ObservationScope::Crates,
        ObservationClassification::DerivedEstimate,
        AuthorityClass::Advisory,
        serde_json::json!({"crate": "manwe", "role": "inference_gateway"}),
    )
    .with_freshness(42)
    .with_confidence(0.87)
    .with_provenance("file://crates/spine/runtime/manwe/README.md")
    .local_only();

    let encoded = serde_json::to_string(&observation).expect("encode");
    let decoded: OutpostObservation = serde_json::from_str(&encoded).expect("decode");

    assert_eq!(decoded.source, "node-pi5-warden");
    assert_eq!(decoded.scope, ObservationScope::Crates);
    assert_eq!(
        decoded.classification,
        ObservationClassification::DerivedEstimate
    );
    assert_eq!(decoded.authority, AuthorityClass::Advisory);
    assert_eq!(decoded.schema_version, SCHEMA_VERSION);
    assert!(decoded.is_advisory());
    assert!(decoded.local_only);
    assert_eq!(decoded.freshness_seconds, 42);
    assert_eq!(decoded.confidence, 0.87);
    assert_eq!(
        decoded.provenance,
        Some("file://crates/spine/runtime/manwe/README.md".into())
    );
}

#[test]
fn presentation_authority_is_not_execution_prohibited() {
    let observation = OutpostObservation::new(
        "node-pi5-citadel-avatar",
        ObservationScope::Apps,
        ObservationClassification::Default,
        AuthorityClass::Presentation,
        serde_json::json!({"app": "arda-launcher"}),
    );
    assert_eq!(observation.authority, AuthorityClass::Presentation);
    assert_ne!(observation.authority, AuthorityClass::ExecutionProhibited);
}
