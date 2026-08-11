use arda_core::capability_composition::{CapabilityComposition, CapabilityCompositionError};
use arda_core::project_contract::ProjectContract;
use arda_core::run_graph::RunGraph;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec/capability-composition/v1/fixtures")
        .join(name)
}

fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).expect("composition fixture")
}

#[test]
fn accepts_all_valid_composition_fixtures() {
    for name in [
        "valid-personal-objective.json",
        "valid-software-project.json",
        "valid-council-assisted-project.json",
    ] {
        CapabilityComposition::from_json_str(&fixture_text(name))
            .unwrap_or_else(|error| panic!("{name} must validate: {error}"));
    }
}

#[test]
fn canonical_digest_is_stable_across_json_object_and_set_order() {
    let raw = fixture_text("valid-software-project.json");
    let first = CapabilityComposition::from_json_str(&raw).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["capabilities"]["required"] =
        serde_json::json!(["artifact_receipt", "verification", "hermes"]);
    let reordered = CapabilityComposition::from_json_str(&value.to_string()).unwrap();

    assert_eq!(
        first.canonical_json().unwrap(),
        reordered.canonical_json().unwrap()
    );
    assert_eq!(first.digest().unwrap(), reordered.digest().unwrap());
    assert!(first.digest().unwrap().starts_with("sha256:"));
}

#[test]
fn rejects_unknown_fields_and_schema_versions() {
    let raw = fixture_text("valid-personal-objective.json");
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["unknown_field"] = serde_json::json!(true);
    assert!(matches!(
        CapabilityComposition::from_json_str(&value.to_string()),
        Err(CapabilityCompositionError::InvalidJson(_))
    ));

    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["schema_version"] = serde_json::json!("arda.capability-composition.v2");
    assert!(matches!(
        CapabilityComposition::from_json_str(&value.to_string()),
        Err(CapabilityCompositionError::UnsupportedSchemaVersion(version))
            if version == "arda.capability-composition.v2"
    ));
}

#[test]
fn optional_capabilities_may_be_omitted() {
    let raw = fixture_text("valid-personal-objective.json");
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["capabilities"]
        .as_object_mut()
        .unwrap()
        .remove("optional");
    let composition = CapabilityComposition::from_json_str(&value.to_string()).unwrap();

    assert!(composition.capabilities.optional.is_empty());
}

#[test]
fn rejects_forbidden_capability_conflicts() {
    let raw = fixture_text("valid-personal-objective.json");
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    value["capabilities"]["forbidden"] = serde_json::json!(["phone_notification"]);

    assert!(matches!(
        CapabilityComposition::from_json_str(&value.to_string()),
        Err(CapabilityCompositionError::CapabilityConflict(capability))
            if capability == "phone_notification"
    ));
}

#[test]
fn rejects_sensitive_external_egress_fixture() {
    assert!(matches!(
        CapabilityComposition::from_json_str(&fixture_text("invalid-sensitive-egress.json")),
        Err(CapabilityCompositionError::SensitiveExternalEgress)
    ));
}

#[test]
fn rejects_worker_or_planner_authority_escalation_fixture() {
    assert!(matches!(
        CapabilityComposition::from_json_str(&fixture_text("invalid-authority-escalation.json")),
        Err(CapabilityCompositionError::AuthorityEscalation { role_id, .. })
            if role_id == "planner"
    ));
}

#[test]
fn project_and_run_graph_match_stable_composition_lineage() {
    let composition =
        CapabilityComposition::from_json_str(&fixture_text("valid-software-project.json")).unwrap();
    let project_raw = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../spec/project-contract/v1/fixtures/valid-project-contract.json"),
    )
    .unwrap();
    let project = ProjectContract::from_json_str(&project_raw).unwrap();
    let run_raw = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../spec/run-graph/v1/fixtures/valid-run-graph.json"),
    )
    .unwrap();
    let run = RunGraph::from_json_str(&run_raw).unwrap();

    assert!(project.matches_composition_lineage(&composition));
    assert!(run.matches_composition_lineage(&composition));
}
