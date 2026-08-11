//! Canonical adapter catalog derived from governed external-capability contracts.

use arda_core::{
    external_capability::{ExternalCapability, ExternalCapabilityMaturity},
    service_registry::{
        CapabilityDeclaration, CapabilityExecutionAdapter, CapabilityHealth, CapabilityMaturity,
        CapabilityProvenance, CapabilityRegistry, CapabilityRegistryError, CapabilityRemovalStatus,
        CapabilityRuntimeState,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    BuiltIn,
    External,
    Sidecar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterCatalogRecord {
    pub kind: AdapterKind,
    pub contract: ExternalCapability,
    pub contract_digest: String,
    pub installed: bool,
    pub health: CapabilityHealth,
    pub eligible: bool,
    pub selected: bool,
    pub removal_status: CapabilityRemovalStatus,
    consecutive_failures: u32,
    observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct AdapterCatalog {
    records: BTreeMap<String, AdapterCatalogRecord>,
}

impl AdapterCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        kind: AdapterKind,
        contract: ExternalCapability,
        installed: bool,
        health: CapabilityHealth,
        eligible: bool,
    ) -> Result<(), AdapterCatalogError> {
        contract.validate()?;
        let adapter_id = contract.identity.adapter_id.clone();
        if self.records.contains_key(&adapter_id) {
            return Err(AdapterCatalogError::DuplicateAdapter(adapter_id));
        }
        validate_state(&contract, installed, health, eligible, false)?;
        let contract_digest = contract.digest()?;
        self.records.insert(
            adapter_id,
            AdapterCatalogRecord {
                kind,
                contract,
                contract_digest,
                installed,
                health,
                eligible,
                selected: false,
                removal_status: CapabilityRemovalStatus::Active,
                consecutive_failures: 0,
                observed_at: None,
            },
        );
        Ok(())
    }

    pub fn load_contract(
        &mut self,
        root: &Path,
        kind: AdapterKind,
        contract_path: &str,
        installed: bool,
        health: CapabilityHealth,
        eligible: bool,
    ) -> Result<(), AdapterCatalogError> {
        let path = root.join(contract_path);
        let raw = std::fs::read_to_string(&path)
            .map_err(|source| AdapterCatalogError::ReadContract { path, source })?;
        let contract = ExternalCapability::from_json_str(&raw)?;
        self.register(kind, contract, installed, health, eligible)
    }

    pub fn observe_health(
        &mut self,
        adapter_id: &str,
        healthy: bool,
        observed_at: DateTime<Utc>,
    ) -> Result<CapabilityHealth, AdapterCatalogError> {
        let record = self
            .records
            .get_mut(adapter_id)
            .ok_or_else(|| AdapterCatalogError::NotFound(adapter_id.to_string()))?;
        if record.removal_status == CapabilityRemovalStatus::Removed || !record.installed {
            record.health = CapabilityHealth::NotConfigured;
            record.eligible = false;
            record.selected = false;
            return Ok(record.health);
        }
        record.observed_at = Some(observed_at);
        if healthy {
            record.consecutive_failures = 0;
            record.health = CapabilityHealth::Ready;
        } else {
            record.consecutive_failures = record.consecutive_failures.saturating_add(1);
            record.health = if record.consecutive_failures
                >= record.contract.health.unavailable_after_failures
            {
                CapabilityHealth::Unavailable
            } else if record.consecutive_failures >= record.contract.health.degraded_after_failures
            {
                CapabilityHealth::Degraded
            } else {
                CapabilityHealth::Ready
            };
            if record.health != CapabilityHealth::Ready {
                record.eligible = false;
                record.selected = false;
            }
        }
        Ok(record.health)
    }

    pub fn refresh_staleness(&mut self, now: DateTime<Utc>) {
        for record in self.records.values_mut() {
            let Some(observed_at) = record.observed_at else {
                continue;
            };
            if record.health == CapabilityHealth::Ready
                && now.signed_duration_since(observed_at).num_seconds()
                    > record.contract.health.freshness_secs as i64
            {
                record.health = CapabilityHealth::Stale;
                record.eligible = false;
                record.selected = false;
            }
        }
    }

    pub fn set_eligible(
        &mut self,
        adapter_id: &str,
        eligible: bool,
    ) -> Result<(), AdapterCatalogError> {
        let record = self
            .records
            .get_mut(adapter_id)
            .ok_or_else(|| AdapterCatalogError::NotFound(adapter_id.to_string()))?;
        validate_state(
            &record.contract,
            record.installed,
            record.health,
            eligible,
            record.selected,
        )?;
        record.eligible = eligible;
        Ok(())
    }

    pub fn remove(&mut self, adapter_id: &str) -> Result<(), AdapterCatalogError> {
        let record = self
            .records
            .get_mut(adapter_id)
            .ok_or_else(|| AdapterCatalogError::NotFound(adapter_id.to_string()))?;
        record.removal_status = CapabilityRemovalStatus::Removed;
        record.installed = false;
        record.health = CapabilityHealth::NotConfigured;
        record.eligible = false;
        record.selected = false;
        Ok(())
    }

    pub fn get(&self, adapter_id: &str) -> Option<&AdapterCatalogRecord> {
        self.records.get(adapter_id)
    }

    pub fn records(&self) -> impl Iterator<Item = &AdapterCatalogRecord> {
        self.records.values()
    }

    pub fn register_capabilities(
        &self,
        registry: &mut CapabilityRegistry,
    ) -> Result<(), CapabilityRegistryError> {
        for record in self.records.values() {
            for (capability_id, maturity) in &record.contract.capabilities {
                registry.register(
                    CapabilityDeclaration {
                        id: capability_id.clone(),
                        version: record.contract.identity.version.clone(),
                        owner: "arda-engine".to_string(),
                        maturity: map_maturity(*maturity),
                        data_classes: record.contract.data_classes.clone(),
                        authority_ceiling: record.contract.authority.authority_ceiling,
                        execution_adapter: CapabilityExecutionAdapter::ExternalAdapter {
                            adapter_id: record.contract.identity.adapter_id.clone(),
                        },
                        removal_status: record.removal_status,
                        provenance: CapabilityProvenance::ExternalAdapter {
                            adapter_id: record.contract.identity.adapter_id.clone(),
                            adapter_version: record.contract.identity.version.clone(),
                            source_digest: record.contract.identity.source_digest.clone(),
                        },
                    },
                    CapabilityRuntimeState {
                        installed: record.installed,
                        health: record.health,
                        eligible: record.eligible,
                        selected: record.selected,
                    },
                )?;
            }
        }
        Ok(())
    }
}

fn validate_state(
    contract: &ExternalCapability,
    installed: bool,
    health: CapabilityHealth,
    eligible: bool,
    selected: bool,
) -> Result<(), AdapterCatalogError> {
    if eligible && (!installed || health != CapabilityHealth::Ready) {
        return Err(AdapterCatalogError::InvalidState {
            adapter_id: contract.identity.adapter_id.clone(),
            reason: "eligibility requires installed and ready".to_string(),
        });
    }
    if selected && !eligible {
        return Err(AdapterCatalogError::InvalidState {
            adapter_id: contract.identity.adapter_id.clone(),
            reason: "selection requires eligibility".to_string(),
        });
    }
    Ok(())
}

fn map_maturity(maturity: ExternalCapabilityMaturity) -> CapabilityMaturity {
    match maturity {
        ExternalCapabilityMaturity::Experimental => CapabilityMaturity::Experimental,
        ExternalCapabilityMaturity::Preview => CapabilityMaturity::Preview,
        ExternalCapabilityMaturity::Stable => CapabilityMaturity::Stable,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterCatalogError {
    #[error(transparent)]
    Contract(#[from] arda_core::external_capability::ExternalCapabilityError),
    #[error("failed to read adapter contract {path}: {source}")]
    ReadContract {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("duplicate adapter ID {0}")]
    DuplicateAdapter(String),
    #[error("adapter {0} not found")]
    NotFound(String),
    #[error("invalid adapter state for {adapter_id}: {reason}")]
    InvalidState { adapter_id: String, reason: String },
}
