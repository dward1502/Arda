use arda_core::external_capability::{ExternalCapability, ExternalCapabilityError};

const FIXTURE_ROOT: &str = "../../../../spec/external-capability/v1/fixtures";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURE_ROOT)
            .join(name),
    )
    .expect("external-capability fixture")
}

#[test]
fn hermes_workbench_contract_is_strict_deterministic_and_authority_bounded() {
    let contract = ExternalCapability::from_json_str(&fixture("valid-hermes-workbench.json"))
        .expect("valid external capability");
    assert_eq!(contract.identity.adapter_id, "hermes-workbench");
    assert!(!contract.authority.task_authority);
    assert!(!contract.authority.memory_authority);
    assert!(!contract.authority.governance_authority);
    assert_eq!(contract.digest().unwrap(), contract.digest().unwrap());

    let mut value: serde_json::Value =
        serde_json::from_str(&fixture("valid-hermes-workbench.json")).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(matches!(
        ExternalCapability::from_json_str(&value.to_string()),
        Err(ExternalCapabilityError::InvalidJson(_))
    ));
}

#[test]
fn external_contract_cannot_duplicate_arda_authority_or_inline_secrets() {
    assert!(matches!(
        ExternalCapability::from_json_str(&fixture("invalid-duplicate-authority.json")),
        Err(ExternalCapabilityError::DuplicateAuthority)
    ));

    let mut value: serde_json::Value =
        serde_json::from_str(&fixture("valid-hermes-workbench.json")).unwrap();
    value["requirements"]["secrets"][0]["reference"] = serde_json::json!("API_KEY=private");
    assert!(matches!(
        ExternalCapability::from_json_str(&value.to_string()),
        Err(ExternalCapabilityError::InlineSecret)
    ));
}
