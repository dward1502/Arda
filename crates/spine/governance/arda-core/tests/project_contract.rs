use arda_core::project_contract::{
    AuthorityMode, ProjectContract, ProjectContractError, ProjectContractVersion,
};

fn parse_fixture(path: &str) -> ProjectContract {
    let raw = std::fs::read_to_string(path).expect("read fixture");
    ProjectContract::from_json_str(&raw).expect("fixture parses")
}

#[test]
fn parses_v1_project_identity_and_workspace_boundary() {
    let contract = parse_fixture("examples/rust-project.json");

    assert_eq!(contract.schema_version, ProjectContractVersion::V1);
    assert_eq!(contract.identity.name, "arda-rust-example");
    assert_eq!(
        contract.identity.project_id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(contract.workspace.root.as_str(), ".");
    assert_eq!(
        contract
            .workspace
            .canonical_path("crates/app/src/lib.rs")
            .unwrap()
            .as_str(),
        "crates/app/src/lib.rs"
    );
}

#[test]
fn rejects_unsupported_major_versions_with_typed_error() {
    let raw = r#"{
        "schema_version": "arda.project-contract.v2",
        "identity": {"project_id": "550e8400-e29b-41d4-a716-446655440000", "name": "future", "kind": "rust"},
        "workspace": {"root": "."},
        "runtime": {"adapter": "cargo"},
        "commands": [],
        "checks": [],
        "artifacts": [],
        "permissions": {},
        "rollback": {"strategy": "git_revert"},
        "memory": {"scope": "project"},
        "provenance": {"declared_by": "test", "declared_at": "2026-07-30T00:00:00Z"}
    }"#;

    let err = ProjectContract::from_json_str(raw).expect_err("v2 rejected");
    assert!(matches!(
        err,
        ProjectContractError::UnsupportedMajorVersion { major: 2 }
    ));
}

#[test]
fn canonical_paths_reject_parent_absolute_and_empty_traversal() {
    let contract = parse_fixture("examples/rust-project.json");

    assert!(matches!(
        contract.workspace.canonical_path("../outside"),
        Err(ProjectContractError::PathEscapesWorkspace { .. })
    ));
    assert!(matches!(
        contract.workspace.canonical_path("/tmp/outside"),
        Err(ProjectContractError::AbsolutePathDenied { .. })
    ));
    assert!(matches!(
        contract.workspace.canonical_path(""),
        Err(ProjectContractError::InvalidRelativePath { .. })
    ));
}

#[test]
fn declares_commands_checks_and_artifacts_with_canonical_paths() {
    let contract = parse_fixture("examples/rust-project.json");

    let test_command = contract.command("test").expect("test command");
    assert_eq!(test_command.program, "cargo");
    assert_eq!(test_command.args, ["test", "-p", "arda-core"]);
    assert_eq!(test_command.working_dir.as_str(), ".");

    let fmt_check = contract.check("fmt").expect("fmt check");
    assert_eq!(fmt_check.command, "fmt");

    assert_eq!(
        contract.artifacts[0].path.as_str(),
        "target/debug/libarda_core.rlib"
    );
}

#[test]
fn secrets_are_environment_names_only_and_values_are_rejected() {
    let contract = parse_fixture("examples/python-project.json");
    assert_eq!(contract.permissions.secrets.env_names, ["PYPI_TOKEN"]);

    let raw_with_value = std::fs::read_to_string("examples/python-project.json")
        .expect("fixture")
        .replace(
            "\"PYPI_TOKEN\"",
            "{\"name\": \"PYPI_TOKEN\", \"value\": \"secret\"}",
        );
    let err = ProjectContract::from_json_str(&raw_with_value).expect_err("secret value rejected");
    assert!(matches!(
        err,
        ProjectContractError::SecretValueDenied { .. }
    ));
}

#[test]
fn authority_defaults_fail_closed_when_permissions_are_omitted() {
    let raw = r#"{
        "schema_version": "arda.project-contract.v1",
        "identity": {"project_id": "550e8400-e29b-41d4-a716-446655440001", "name": "minimal", "kind": "python"},
        "workspace": {"root": "."},
        "runtime": {"adapter": "python"},
        "commands": [],
        "checks": [],
        "artifacts": [],
        "rollback": {"strategy": "git_revert"},
        "memory": {"scope": "project"},
        "provenance": {"declared_by": "test", "declared_at": "2026-07-30T00:00:00Z"}
    }"#;

    let contract = ProjectContract::from_json_str(raw).expect("minimal contract parses");
    assert_eq!(contract.permissions.authority, AuthorityMode::DenyByDefault);
    assert!(!contract.permissions.network.allow);
    assert!(!contract.permissions.filesystem.write);
    assert!(contract.permissions.secrets.env_names.is_empty());
}

#[test]
fn rust_and_python_fixtures_deserialize_to_same_core_contract_model() {
    let rust = parse_fixture("examples/rust-project.json");
    let python = parse_fixture("examples/python-project.json");

    assert_eq!(rust.schema_version, python.schema_version);
    assert_eq!(rust.workspace.root.as_str(), python.workspace.root.as_str());
    assert_ne!(rust.runtime.adapter, python.runtime.adapter);
    assert_eq!(rust.rollback.strategy, python.rollback.strategy);
    assert_eq!(rust.memory.scope, python.memory.scope);
}

#[test]
fn additive_migration_policy_rejects_unknown_required_fields_but_allows_extension_fields() {
    let contract = parse_fixture("examples/rust-project.json");
    assert!(contract.provenance.extensions.contains_key("stage"));

    let raw = std::fs::read_to_string("examples/rust-project.json")
        .expect("fixture")
        .replace("\"schema_version\": \"arda.project-contract.v1\"", "\"schema_version\": \"arda.project-contract.v1\", \"required\": [\"unknown_new_field\"]");
    let err = ProjectContract::from_json_str(&raw).expect_err("unknown required rejected");
    assert!(
        matches!(err, ProjectContractError::UnsupportedRequiredField { field } if field == "unknown_new_field")
    );
}
