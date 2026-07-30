use arda_core::project_contract::{AuthorityMode, ProjectContract, ProjectContractVersion};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPermissionSummary {
    pub authority: String,
    pub network_allowed: bool,
    pub filesystem_write: bool,
    pub secret_env_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContractValidation {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub kind: String,
    pub workspace_root: String,
    pub runtime_adapter: String,
    pub command_ids: Vec<String>,
    pub check_ids: Vec<String>,
    pub permissions: WorkbenchPermissionSummary,
}

pub fn validate_project_contract_path(path: &Path) -> Result<ProjectContractValidation, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read project contract {}: {error}",
            path.display()
        )
    })?;
    let contract = ProjectContract::from_json_str(&raw)
        .map_err(|error| format!("project contract validation failed: {error}"))?;

    let schema_version = match contract.schema_version {
        ProjectContractVersion::V1 => "arda.project-contract.v1",
    };
    let authority = match contract.permissions.authority {
        AuthorityMode::DenyByDefault => "deny_by_default",
        AuthorityMode::ReadOnly => "read_only",
        AuthorityMode::ApprovalRequired => "approval_required",
    };

    Ok(ProjectContractValidation {
        schema_version: schema_version.to_owned(),
        project_id: contract.identity.project_id.to_string(),
        name: contract.identity.name,
        kind: contract.identity.kind,
        workspace_root: contract.workspace.root.as_str().to_owned(),
        runtime_adapter: contract.runtime.adapter,
        command_ids: contract
            .commands
            .into_iter()
            .map(|command| command.id)
            .collect(),
        check_ids: contract.checks.into_iter().map(|check| check.id).collect(),
        permissions: WorkbenchPermissionSummary {
            authority: authority.to_owned(),
            network_allowed: contract.permissions.network.allow,
            filesystem_write: contract.permissions.filesystem.write,
            secret_env_names: contract.permissions.secrets.env_names,
        },
    })
}

#[tauri::command]
pub fn validate_project_contract(path: String) -> Result<ProjectContractValidation, String> {
    validate_project_contract_path(Path::new(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn rust_contract_fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../spec/project-contract/v1/examples/rust-project.json")
    }

    #[test]
    fn validates_contract_and_projects_effective_attachment_summary() {
        let summary = validate_project_contract_path(&rust_contract_fixture())
            .expect("valid project contract should produce an attachment summary");

        assert_eq!(summary.schema_version, "arda.project-contract.v1");
        assert_eq!(summary.name, "arda-rust-example");
        assert_eq!(summary.kind, "rust");
        assert_eq!(summary.workspace_root, ".");
        assert_eq!(summary.runtime_adapter, "cargo");
        assert_eq!(summary.command_ids, vec!["test"]);
        assert_eq!(summary.check_ids, vec!["test"]);
        assert_eq!(summary.permissions.authority, "approval_required");
        assert!(!summary.permissions.network_allowed);
        assert!(summary.permissions.filesystem_write);
        assert!(summary.permissions.secret_env_names.is_empty());
    }

    #[test]
    fn rejects_missing_contract_path() {
        let error = validate_project_contract_path(Path::new("/definitely/missing/project.json"))
            .expect_err("missing contract must fail closed");

        assert!(error.contains("read project contract"));
    }
}
