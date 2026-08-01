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

#[test]
fn observation_json_uses_canonical_snake_case_contract_values() {
    let observation = OutpostObservation::new(
        "node-pi5-warden",
        ObservationScope::RuntimeTelemetry,
        ObservationClassification::ExperimentalDerived,
        AuthorityClass::ExecutionProhibited,
        serde_json::json!({}),
    );

    let encoded = serde_json::to_value(observation).expect("encode observation");
    assert_eq!(encoded["scope"], "runtime_telemetry");
    assert_eq!(encoded["classification"], "experimental_derived");
    assert_eq!(encoded["authority"], "execution_prohibited");
    assert_eq!(
        serde_json::to_value(ObservationScope::Custom("internet_research".into()))
            .expect("encode custom scope"),
        serde_json::json!({"custom": "internet_research"})
    );
}

#[test]
fn observation_json_accepts_legacy_pascal_case_contract_values() {
    let observation = OutpostObservation::new(
        "node-pi5-warden",
        ObservationScope::RuntimeTelemetry,
        ObservationClassification::ExperimentalDerived,
        AuthorityClass::ExecutionProhibited,
        serde_json::json!({}),
    );
    let mut legacy = serde_json::to_value(observation).expect("encode observation");
    legacy["scope"] = serde_json::json!("RuntimeTelemetry");
    legacy["classification"] = serde_json::json!("ExperimentalDerived");
    legacy["authority"] = serde_json::json!("ExecutionProhibited");

    let decoded: OutpostObservation =
        serde_json::from_value(legacy).expect("decode legacy observation");
    assert_eq!(decoded.scope, ObservationScope::RuntimeTelemetry);
    assert_eq!(
        decoded.classification,
        ObservationClassification::ExperimentalDerived
    );
    assert_eq!(decoded.authority, AuthorityClass::ExecutionProhibited);
    assert_eq!(
        serde_json::from_value::<ObservationScope>(
            serde_json::json!({"Custom": "internet_research"})
        )
        .expect("decode legacy custom scope"),
        ObservationScope::Custom("internet_research".into())
    );
}

#[test]
fn observation_json_rejects_unknown_contract_values() {
    let observation = OutpostObservation::new(
        "node-pi5-warden",
        ObservationScope::Crates,
        ObservationClassification::DerivedEstimate,
        AuthorityClass::Advisory,
        serde_json::json!({}),
    );
    let canonical = serde_json::to_value(observation).expect("encode observation");

    for (field, unknown) in [
        ("scope", "autonomous_dispatch"),
        ("classification", "authoritative_fact"),
        ("authority", "execution_authorized"),
    ] {
        let mut malformed = canonical.clone();
        malformed[field] = serde_json::json!(unknown);
        assert!(
            serde_json::from_value::<OutpostObservation>(malformed).is_err(),
            "unknown {field} value must be rejected"
        );
    }
}

#[test]
fn no_observation_authority_class_permits_execution() {
    for authority in [
        AuthorityClass::Advisory,
        AuthorityClass::Presentation,
        AuthorityClass::ExecutionProhibited,
    ] {
        assert!(!authority.permits_execution());
    }
}
