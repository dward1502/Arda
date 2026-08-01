use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::path::{Component, Path};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectContractVersion {
    #[serde(rename = "arda.project-contract.v1")]
    V1,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectContractError {
    #[error("invalid project contract JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported project contract major version {major}")]
    UnsupportedMajorVersion { major: u64 },
    #[error("unsupported required field `{field}`")]
    UnsupportedRequiredField { field: String },
    #[error("path escapes workspace: {path}")]
    PathEscapesWorkspace { path: String },
    #[error("absolute paths are denied: {path}")]
    AbsolutePathDenied { path: String },
    #[error("invalid relative path: {path}")]
    InvalidRelativePath { path: String },
    #[error("secret values are denied; declare environment names only")]
    SecretValueDenied { field: String },
    #[error("duplicate identifier `{id}` in {collection}")]
    DuplicateIdentifier {
        collection: &'static str,
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SafeRelativePath(String);

impl SafeRelativePath {
    pub fn new(path: impl Into<String>) -> Result<Self, ProjectContractError> {
        let path = path.into();
        if path.is_empty() {
            return Err(ProjectContractError::InvalidRelativePath { path });
        }
        let parsed = Path::new(&path);
        if parsed.is_absolute() {
            return Err(ProjectContractError::AbsolutePathDenied { path });
        }
        if parsed
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(ProjectContractError::PathEscapesWorkspace { path });
        }
        if parsed
            .components()
            .any(|component| matches!(component, Component::RootDir | Component::Prefix(_)))
        {
            return Err(ProjectContractError::AbsolutePathDenied { path });
        }
        Ok(Self(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SafeRelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectIdentity {
    pub project_id: Uuid,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBoundary {
    pub root: SafeRelativePath,
}

impl WorkspaceBoundary {
    pub fn canonical_path(&self, path: &str) -> Result<SafeRelativePath, ProjectContractError> {
        let child = SafeRelativePath::new(path)?;
        if self.root.as_str() == "." {
            Ok(child)
        } else {
            SafeRelativePath::new(format!(
                "{}/{}",
                self.root.as_str().trim_end_matches('/'),
                child.as_str()
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeclaration {
    pub adapter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDeclaration {
    pub id: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub working_dir: SafeRelativePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckDeclaration {
    pub id: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDeclaration {
    pub id: String,
    pub path: SafeRelativePath,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityMode {
    #[default]
    DenyByDefault,
    ReadOnly,
    ApprovalRequired,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPermission {
    #[serde(default)]
    pub allow: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemPermission {
    #[serde(default)]
    pub write: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretPermission {
    #[serde(default)]
    pub env_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    #[serde(default)]
    pub authority: AuthorityMode,
    #[serde(default)]
    pub network: NetworkPermission,
    #[serde(default)]
    pub filesystem: FilesystemPermission,
    #[serde(default)]
    pub secrets: SecretPermission,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackDeclaration {
    pub strategy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryDeclaration {
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractProvenance {
    pub declared_by: String,
    pub declared_at: String,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectContract {
    pub schema_version: ProjectContractVersion,
    pub identity: ProjectIdentity,
    pub workspace: WorkspaceBoundary,
    pub runtime: RuntimeDeclaration,
    #[serde(default)]
    pub commands: Vec<CommandDeclaration>,
    #[serde(default)]
    pub checks: Vec<CheckDeclaration>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactDeclaration>,
    #[serde(default)]
    pub permissions: Permissions,
    pub rollback: RollbackDeclaration,
    pub memory: MemoryDeclaration,
    pub provenance: ContractProvenance,
}

impl ProjectContract {
    pub fn from_json_str(raw: &str) -> Result<Self, ProjectContractError> {
        let value: Value = serde_json::from_str(raw)?;
        validate_version(&value)?;
        validate_required_extensions(&value)?;
        validate_secret_shape(&value)?;
        let contract: Self = serde_json::from_value(value)?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), ProjectContractError> {
        reject_duplicate_ids(
            "commands",
            self.commands.iter().map(|item| item.id.as_str()),
        )?;
        reject_duplicate_ids("checks", self.checks.iter().map(|item| item.id.as_str()))?;
        reject_duplicate_ids(
            "artifacts",
            self.artifacts.iter().map(|item| item.id.as_str()),
        )?;
        for command in &self.commands {
            self.workspace
                .canonical_path(command.working_dir.as_str())?;
        }
        for artifact in &self.artifacts {
            self.workspace.canonical_path(artifact.path.as_str())?;
        }
        Ok(())
    }

    pub fn command(&self, id: &str) -> Option<&CommandDeclaration> {
        self.commands.iter().find(|command| command.id == id)
    }

    pub fn check(&self, id: &str) -> Option<&CheckDeclaration> {
        self.checks.iter().find(|check| check.id == id)
    }
}

fn validate_version(value: &Value) -> Result<(), ProjectContractError> {
    let Some(version) = value.get("schema_version").and_then(Value::as_str) else {
        return Ok(());
    };
    if version == "arda.project-contract.v1" {
        return Ok(());
    }
    if let Some(major) = version
        .strip_prefix("arda.project-contract.v")
        .and_then(|value| value.parse().ok())
    {
        return Err(ProjectContractError::UnsupportedMajorVersion { major });
    }
    Ok(())
}

fn validate_required_extensions(value: &Value) -> Result<(), ProjectContractError> {
    const SUPPORTED: &[&str] = &[
        "schema_version",
        "identity",
        "workspace",
        "runtime",
        "commands",
        "checks",
        "artifacts",
        "permissions",
        "rollback",
        "memory",
        "provenance",
    ];
    if let Some(required) = value.get("required").and_then(Value::as_array) {
        for field in required.iter().filter_map(Value::as_str) {
            if !SUPPORTED.contains(&field) {
                return Err(ProjectContractError::UnsupportedRequiredField {
                    field: field.to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn validate_secret_shape(value: &Value) -> Result<(), ProjectContractError> {
    if let Some(names) = value
        .pointer("/permissions/secrets/env_names")
        .and_then(Value::as_array)
    {
        if names.iter().any(|name| !name.is_string()) {
            return Err(ProjectContractError::SecretValueDenied {
                field: "permissions.secrets.env_names".to_owned(),
            });
        }
    }
    Ok(())
}

fn reject_duplicate_ids<'a>(
    collection: &'static str,
    ids: impl Iterator<Item = &'a str>,
) -> Result<(), ProjectContractError> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(ProjectContractError::DuplicateIdentifier {
                collection,
                id: id.to_owned(),
            });
        }
    }
    Ok(())
}

impl fmt::Display for SafeRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
