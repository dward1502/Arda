use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const ORGANISM_MANIFEST_PATH: &str = "config/organism.toml";

#[derive(Debug, thiserror::Error)]
pub enum OrganismManifestError {
    #[error("failed to read organism manifest at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid organism manifest TOML: {0}")]
    InvalidToml(String),
    #[error("unsupported organism manifest schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("organism manifest field `{0}` cannot be empty")]
    EmptyField(&'static str),
    #[error("organism manifest field `{field}` has an invalid identifier: {value}")]
    InvalidIdentifier { field: &'static str, value: String },
    #[error("organism manifest field `{field}` contains duplicate value `{value}`")]
    DuplicateValue { field: &'static str, value: String },
    #[error("authority `{concern}` must be `{expected}`, found `{actual}`")]
    AuthorityMismatch {
        concern: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("enabled transport `{0:?}` is not accepted by the organism manifest")]
    EnabledTransportNotAccepted(TransportFamily),
    #[error("required contract version `{key}` must be `{expected}`")]
    MissingContractVersion {
        key: &'static str,
        expected: &'static str,
    },
    #[error("failed to serialize canonical organism manifest: {0}")]
    Serialize(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganismDomain {
    Personal,
    Business,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportFamily {
    InProcessRust,
    ArdaHarnessHttp,
    HermesPluginHook,
    LinuxFoundationA2a,
    Mcp,
    ManweOpenaiApi,
    SystemdOrEngineAdapter,
    OutpostProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganismAuthorities {
    pub objective: String,
    pub run: String,
    pub node: String,
    pub session: String,
    pub agent: String,
    pub semantic_envelope: String,
    pub a2a_wire: String,
    pub model_route: String,
    pub memory: String,
    pub evidence: String,
    pub governance: String,
    pub projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganismManifest {
    pub schema_version: String,
    pub organism_id: String,
    pub display_name: String,
    pub mission: String,
    pub operator_id: String,
    pub privacy_domains: Vec<OrganismDomain>,
    pub accepted_transports: Vec<TransportFamily>,
    pub enabled_transports: Vec<TransportFamily>,
    pub authorities: OrganismAuthorities,
    pub contract_versions: BTreeMap<String, String>,
}

impl OrganismManifest {
    pub const SCHEMA_VERSION: &'static str = "arda.organism-manifest.v1";

    pub fn from_toml_str(raw: &str) -> Result<Self, OrganismManifestError> {
        let manifest: Self = toml::from_str(raw)
            .map_err(|error| OrganismManifestError::InvalidToml(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load_from_root(root: impl AsRef<Path>) -> Result<Self, OrganismManifestError> {
        let path = root.as_ref().join(ORGANISM_MANIFEST_PATH);
        let raw = std::fs::read_to_string(&path)
            .map_err(|source| OrganismManifestError::Read { path, source })?;
        Self::from_toml_str(&raw)
    }

    pub fn validate(&self) -> Result<(), OrganismManifestError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(OrganismManifestError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        validate_identifier("organism_id", &self.organism_id, 128)?;
        validate_identifier("operator_id", &self.operator_id, 128)?;
        require_bounded_text("display_name", &self.display_name, 128)?;
        require_bounded_text("mission", &self.mission, 1024)?;
        reject_duplicates("privacy_domains", &self.privacy_domains)?;
        reject_duplicates("accepted_transports", &self.accepted_transports)?;
        reject_duplicates("enabled_transports", &self.enabled_transports)?;
        if self.privacy_domains.is_empty() {
            return Err(OrganismManifestError::EmptyField("privacy_domains"));
        }
        if self.accepted_transports.is_empty() {
            return Err(OrganismManifestError::EmptyField("accepted_transports"));
        }
        let accepted = self
            .accepted_transports
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for transport in &self.enabled_transports {
            if !accepted.contains(transport) {
                return Err(OrganismManifestError::EnabledTransportNotAccepted(
                    *transport,
                ));
            }
        }
        validate_authorities(&self.authorities)?;
        for (key, expected) in [
            ("organism_manifest", Self::SCHEMA_VERSION),
            ("organism_context", "arda.organism-context.v1"),
            ("organism_outcome", "arda.organism-outcome.v1"),
        ] {
            if self.contract_versions.get(key).map(String::as_str) != Some(expected) {
                return Err(OrganismManifestError::MissingContractVersion { key, expected });
            }
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, OrganismManifestError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.privacy_domains.sort();
        canonical.accepted_transports.sort();
        canonical.enabled_transports.sort();
        serde_json::to_string(&canonical)
            .map_err(|error| OrganismManifestError::Serialize(error.to_string()))
    }

    pub fn digest(&self) -> Result<String, OrganismManifestError> {
        let canonical = self.canonical_json()?;
        Ok(format!("sha256:{:x}", Sha256::digest(canonical.as_bytes())))
    }
}

fn validate_identifier(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), OrganismManifestError> {
    if value.is_empty()
        || value.len() > max
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_:./+".contains(character))
    {
        return Err(OrganismManifestError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn require_bounded_text(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), OrganismManifestError> {
    if value.trim().is_empty() {
        return Err(OrganismManifestError::EmptyField(field));
    }
    if value.len() > max || value.as_bytes().contains(&0) {
        return Err(OrganismManifestError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn reject_duplicates<T>(field: &'static str, values: &[T]) -> Result<(), OrganismManifestError>
where
    T: Ord + Copy + std::fmt::Debug,
{
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(*value) {
            return Err(OrganismManifestError::DuplicateValue {
                field,
                value: format!("{value:?}"),
            });
        }
    }
    Ok(())
}

fn validate_authorities(authorities: &OrganismAuthorities) -> Result<(), OrganismManifestError> {
    for (concern, actual, expected) in [
        ("objective", authorities.objective.as_str(), "arda-core"),
        ("run", authorities.run.as_str(), "arda-engine"),
        (
            "node",
            authorities.node.as_str(),
            "arda-engine+arda-outpost-protocol",
        ),
        ("session", authorities.session.as_str(), "hermes-agent"),
        (
            "agent",
            authorities.agent.as_str(),
            "hermes-agent+a2a-agent-card",
        ),
        (
            "semantic_envelope",
            authorities.semantic_envelope.as_str(),
            "arda-orome",
        ),
        ("a2a_wire", authorities.a2a_wire.as_str(), "hermes-a2a"),
        ("model_route", authorities.model_route.as_str(), "manwe"),
        ("memory", authorities.memory.as_str(), "arda-vaire"),
        ("evidence", authorities.evidence.as_str(), "arda-varda"),
        (
            "governance",
            authorities.governance.as_str(),
            "arda-governance",
        ),
        ("projection", authorities.projection.as_str(), "arda-aule"),
    ] {
        if actual != expected {
            return Err(OrganismManifestError::AuthorityMismatch {
                concern,
                expected,
                actual: actual.to_string(),
            });
        }
    }
    Ok(())
}
