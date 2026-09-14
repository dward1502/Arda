use arda_core::organism::{OrganismManifest, OrganismManifestError};

const VALID_MANIFEST: &str = r#"
schema_version = "arda.organism-manifest.v1"
organism_id = "arda:mythos:primary"
display_name = "Arda"
mission = "Compose bounded nodes into one governed digital organism."
operator_id = "operator:mythos"
privacy_domains = ["personal", "business", "system"]
accepted_transports = ["in_process_rust", "arda_harness_http", "hermes_plugin_hook", "linux_foundation_a2a", "mcp", "manwe_openai_api", "systemd_or_engine_adapter", "outpost_protocol"]
enabled_transports = ["in_process_rust", "arda_harness_http", "hermes_plugin_hook", "manwe_openai_api", "systemd_or_engine_adapter", "outpost_protocol"]

[authorities]
objective = "arda-core"
run = "arda-engine"
node = "arda-engine+arda-outpost-protocol"
session = "hermes-agent"
agent = "hermes-agent+a2a-agent-card"
semantic_envelope = "arda-orome"
a2a_wire = "hermes-a2a"
model_route = "manwe"
memory = "arda-vaire"
evidence = "arda-varda"
governance = "arda-governance"
projection = "arda-aule"

[contract_versions]
organism_manifest = "arda.organism-manifest.v1"
organism_context = "arda.organism-context.v1"
organism_outcome = "arda.organism-outcome.v1"
"#;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn repository_manifest_is_valid() {
    let manifest = OrganismManifest::load_from_root(workspace_root()).expect("repository manifest");
    assert_eq!(manifest.schema_version, OrganismManifest::SCHEMA_VERSION);
}

#[test]
fn valid_manifest_is_canonical_and_digest_stable() {
    let manifest = OrganismManifest::from_toml_str(VALID_MANIFEST).expect("valid manifest");
    assert_eq!(manifest.organism_id, "arda:mythos:primary");
    assert!(manifest.enabled_transports.len() < manifest.accepted_transports.len());
    assert_eq!(manifest.digest().unwrap(), manifest.digest().unwrap());
    assert!(manifest.digest().unwrap().starts_with("sha256:"));
}

#[test]
fn manifest_rejects_unknown_fields_and_owner_drift() {
    let unknown = VALID_MANIFEST.replace("\n[authorities]", "\nunexpected = true\n\n[authorities]");
    assert!(matches!(
        OrganismManifest::from_toml_str(&unknown),
        Err(OrganismManifestError::InvalidToml(_))
    ));

    let drifted =
        VALID_MANIFEST.replace("objective = \"arda-core\"", "objective = \"hermes-agent\"");
    assert!(matches!(
        OrganismManifest::from_toml_str(&drifted),
        Err(OrganismManifestError::AuthorityMismatch {
            concern: "objective",
            ..
        })
    ));
}

#[test]
fn enabled_transport_must_be_accepted() {
    let invalid = VALID_MANIFEST.replace(
        "enabled_transports = [\"in_process_rust\",",
        "enabled_transports = [\"unknown_transport\", \"in_process_rust\",",
    );
    assert!(OrganismManifest::from_toml_str(&invalid).is_err());
}
