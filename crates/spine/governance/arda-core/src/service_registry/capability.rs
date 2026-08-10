//! Canonical capability declarations and live runtime registry.

use crate::capability_composition::{CompositionAuthorityClass, DataClass};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityMaturity {
    Experimental,
    Preview,
    Stable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityHealth {
    NotConfigured,
    Unavailable,
    Degraded,
    Stale,
    Ready,
}

impl CapabilityHealth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::Unavailable => "unavailable",
            Self::Degraded => "degraded",
            Self::Stale => "stale",
            Self::Ready => "ready",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRemovalStatus {
    Active,
    Deprecated,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityExecutionAdapter {
    Service { service: String },
    ExternalAdapter { adapter_id: String },
    ModelWorker { provider: String, model: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityProvenance {
    Internal {
        source: String,
        source_digest: String,
    },
    ExternalAdapter {
        adapter_id: String,
        adapter_version: String,
        source_digest: String,
    },
    ModelWorker {
        provider: String,
        model: String,
        source_digest: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDeclaration {
    pub id: String,
    pub version: String,
    pub owner: String,
    pub maturity: CapabilityMaturity,
    pub data_classes: BTreeSet<DataClass>,
    pub authority_ceiling: CompositionAuthorityClass,
    pub execution_adapter: CapabilityExecutionAdapter,
    pub removal_status: CapabilityRemovalStatus,
    pub provenance: CapabilityProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRuntimeState {
    pub installed: bool,
    pub health: CapabilityHealth,
    pub eligible: bool,
    pub selected: bool,
}

impl CapabilityRuntimeState {
    pub fn unavailable(installed: bool) -> Self {
        Self {
            installed,
            health: if installed {
                CapabilityHealth::Unavailable
            } else {
                CapabilityHealth::NotConfigured
            },
            eligible: false,
            selected: false,
        }
    }

    pub fn healthy(self) -> bool {
        self.health == CapabilityHealth::Ready
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRecord {
    pub declaration: CapabilityDeclaration,
    pub runtime: CapabilityRuntimeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityStateProjection {
    pub id: String,
    pub version: String,
    pub owner: String,
    pub status: CapabilityHealth,
    pub installed: bool,
    pub healthy: bool,
    pub eligible: bool,
    pub selected: bool,
    pub removal_status: CapabilityRemovalStatus,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CapabilityRegistryError {
    #[error(
        "capability {id}@{version} already has authority owner {existing_owner}; rejected {incoming_owner}"
    )]
    DuplicateAuthority {
        id: String,
        version: String,
        existing_owner: String,
        incoming_owner: String,
    },
    #[error("capability {id}@{version} not found")]
    NotFound { id: String, version: String },
    #[error("invalid capability declaration {id}@{version}: {reason}")]
    InvalidDeclaration {
        id: String,
        version: String,
        reason: String,
    },
    #[error("invalid capability provenance for {id}@{version}: {reason}")]
    InvalidProvenance {
        id: String,
        version: String,
        reason: String,
    },
    #[error("invalid runtime state for {id}@{version}: {reason}")]
    InvalidRuntimeState {
        id: String,
        version: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    records: BTreeMap<(String, String), CapabilityRecord>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        declaration: CapabilityDeclaration,
        runtime: CapabilityRuntimeState,
    ) -> Result<(), CapabilityRegistryError> {
        validate_declaration(&declaration)?;
        validate_runtime(&declaration, runtime)?;
        let key = (declaration.id.clone(), declaration.version.clone());
        if let Some(existing) = self.records.get(&key) {
            return Err(CapabilityRegistryError::DuplicateAuthority {
                id: declaration.id,
                version: declaration.version,
                existing_owner: existing.declaration.owner.clone(),
                incoming_owner: declaration.owner,
            });
        }
        self.records.insert(
            key,
            CapabilityRecord {
                declaration,
                runtime,
            },
        );
        Ok(())
    }

    pub fn set_runtime_state(
        &mut self,
        id: &str,
        version: &str,
        runtime: CapabilityRuntimeState,
    ) -> Result<(), CapabilityRegistryError> {
        let key = (id.to_string(), version.to_string());
        let record =
            self.records
                .get_mut(&key)
                .ok_or_else(|| CapabilityRegistryError::NotFound {
                    id: id.to_string(),
                    version: version.to_string(),
                })?;
        validate_runtime(&record.declaration, runtime)?;
        record.runtime = runtime;
        Ok(())
    }

    pub fn get(&self, id: &str, version: &str) -> Option<&CapabilityRecord> {
        self.records.get(&(id.to_string(), version.to_string()))
    }

    pub fn records(&self) -> impl Iterator<Item = &CapabilityRecord> {
        self.records.values()
    }

    pub fn projection(&self) -> Vec<CapabilityStateProjection> {
        self.records
            .values()
            .map(|record| CapabilityStateProjection {
                id: record.declaration.id.clone(),
                version: record.declaration.version.clone(),
                owner: record.declaration.owner.clone(),
                status: record.runtime.health,
                installed: record.runtime.installed,
                healthy: record.runtime.healthy(),
                eligible: record.runtime.eligible,
                selected: record.runtime.selected,
                removal_status: record.declaration.removal_status,
            })
            .collect()
    }
}

fn validate_declaration(
    declaration: &CapabilityDeclaration,
) -> Result<(), CapabilityRegistryError> {
    for (field, value) in [
        ("id", declaration.id.as_str()),
        ("version", declaration.version.as_str()),
        ("owner", declaration.owner.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(CapabilityRegistryError::InvalidDeclaration {
                id: declaration.id.clone(),
                version: declaration.version.clone(),
                reason: format!("{field} cannot be empty"),
            });
        }
    }
    validate_provenance(declaration)
}

fn validate_provenance(declaration: &CapabilityDeclaration) -> Result<(), CapabilityRegistryError> {
    let matches = match (&declaration.execution_adapter, &declaration.provenance) {
        (
            CapabilityExecutionAdapter::Service { service },
            CapabilityProvenance::Internal { source, .. },
        ) => !service.trim().is_empty() && !source.trim().is_empty(),
        (
            CapabilityExecutionAdapter::ExternalAdapter { adapter_id },
            CapabilityProvenance::ExternalAdapter {
                adapter_id: provenance_id,
                adapter_version,
                ..
            },
        ) => adapter_id == provenance_id && !adapter_version.trim().is_empty(),
        (
            CapabilityExecutionAdapter::ModelWorker { provider, model },
            CapabilityProvenance::ModelWorker {
                provider: provenance_provider,
                model: provenance_model,
                ..
            },
        ) => provider == provenance_provider && model == provenance_model,
        _ => false,
    };
    if !matches || provenance_digest(&declaration.provenance).trim().is_empty() {
        return Err(CapabilityRegistryError::InvalidProvenance {
            id: declaration.id.clone(),
            version: declaration.version.clone(),
            reason: "execution adapter and provenance must match with a source digest".to_string(),
        });
    }
    Ok(())
}

fn provenance_digest(provenance: &CapabilityProvenance) -> &str {
    match provenance {
        CapabilityProvenance::Internal { source_digest, .. }
        | CapabilityProvenance::ExternalAdapter { source_digest, .. }
        | CapabilityProvenance::ModelWorker { source_digest, .. } => source_digest,
    }
}

fn validate_runtime(
    declaration: &CapabilityDeclaration,
    runtime: CapabilityRuntimeState,
) -> Result<(), CapabilityRegistryError> {
    if runtime.eligible
        && (!runtime.installed
            || !runtime.healthy()
            || declaration.removal_status == CapabilityRemovalStatus::Removed)
    {
        return Err(CapabilityRegistryError::InvalidRuntimeState {
            id: declaration.id.clone(),
            version: declaration.version.clone(),
            reason: "eligible requires installed, ready, and not removed".to_string(),
        });
    }
    if runtime.selected && !runtime.eligible {
        return Err(CapabilityRegistryError::InvalidRuntimeState {
            id: declaration.id.clone(),
            version: declaration.version.clone(),
            reason: "selected requires eligible".to_string(),
        });
    }
    Ok(())
}
