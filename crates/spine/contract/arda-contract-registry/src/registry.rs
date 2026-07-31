use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const DEFAULT_REGISTRY_PATH: &str = "core/state/contract_registry.json";

#[derive(Debug, thiserror::Error)]
pub enum RegistryLoadError {
    #[error("failed to read contract registry at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse contract registry at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractVersionError {
    #[error("contract schema version is not declared by the registry: {0}")]
    Undeclared(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackDefinition {
    pub track_id: String,
    pub title: String,
    pub owner: String,
    pub status: String,
    pub source_modules: Vec<String>,
    #[serde(default)]
    pub evidence_class_current: String,
    #[serde(default)]
    pub evidence_class_target: String,
    pub schema_versions: Vec<String>,
    pub receipt_stores: Vec<String>,
    pub cli_verbs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractRegistry {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub authority: String,
    pub tracks: Vec<TrackDefinition>,
}

impl ContractRegistry {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RegistryLoadError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|source| RegistryLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| RegistryLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn load_from_root(root: impl AsRef<Path>) -> Result<Self, RegistryLoadError> {
        Self::load(root.as_ref().join(DEFAULT_REGISTRY_PATH))
    }

    pub fn track_ids(&self) -> Vec<&str> {
        self.tracks.iter().map(|t| t.track_id.as_str()).collect()
    }

    pub fn require_schema_version(&self, version: &str) -> Result<(), ContractVersionError> {
        self.tracks
            .iter()
            .flat_map(|track| track.schema_versions.iter())
            .any(|declared| declared == version)
            .then_some(())
            .ok_or_else(|| ContractVersionError::Undeclared(version.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::{ContractRegistry, RegistryLoadError};

    const FIXTURE: &str = r#"{
        "schema_version": "arda.contract-registry.v1",
        "generated_at_utc": "2026-07-28T00:00:00Z",
        "authority": "fixture",
        "tracks": []
    }"#;

    #[test]
    fn loads_an_explicit_fixture_without_workspace_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("registry.json");
        std::fs::write(&path, FIXTURE).expect("write fixture");

        let registry = ContractRegistry::load(&path).expect("load fixture");
        assert_eq!(registry.schema_version, "arda.contract-registry.v1");
        assert_eq!(registry.authority, "fixture");
    }

    #[test]
    fn missing_registry_reports_the_explicit_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("missing.json");

        let error = ContractRegistry::load(&path).expect_err("missing fixture");
        assert!(matches!(error, RegistryLoadError::Read { .. }));
        assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
    }

    #[test]
    fn malformed_registry_is_distinct_from_a_missing_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("registry.json");
        std::fs::write(&path, "{").expect("write malformed fixture");

        let error = ContractRegistry::load(&path).expect_err("malformed fixture");
        assert!(matches!(error, RegistryLoadError::Parse { .. }));
    }

    #[test]
    fn declared_schema_versions_are_accepted_and_undeclared_versions_fail_closed() {
        let registry: ContractRegistry = serde_json::from_str(
            r#"{
                "schema_version": "arda.contract-registry.v1",
                "generated_at_utc": "2026-07-30T00:00:00Z",
                "authority": "fixture",
                "tracks": [{
                    "track_id": "workbench",
                    "title": "Workbench",
                    "owner": "ARDA",
                    "status": "active",
                    "source_modules": ["project_contract.rs"],
                    "schema_versions": ["arda.project-contract.v1", "arda.run-graph.v1"],
                    "receipt_stores": [],
                    "cli_verbs": []
                }]
            }"#,
        )
        .expect("fixture registry");

        assert!(registry
            .require_schema_version("arda.project-contract.v1")
            .is_ok());
        assert!(registry.require_schema_version("arda.run-graph.v1").is_ok());
        let error = registry
            .require_schema_version("arda.project-contract.v2")
            .expect_err("undeclared contract version must fail closed");
        assert!(error.to_string().contains("arda.project-contract.v2"));
    }
}
