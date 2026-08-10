//! Versioned contract for bounded external, built-in, and sidecar capabilities.

use crate::capability_composition::{CompositionAuthorityClass, DataClass};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExternalCapabilityError {
    #[error("invalid external capability JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported external capability schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("invalid SHA-256 digest in {0}")]
    InvalidDigest(&'static str),
    #[error("external capability requires at least one capability ID")]
    MissingCapability,
    #[error("external capability requires at least one declared protocol")]
    MissingProtocol,
    #[error("external capability resource limits must be non-zero")]
    InvalidResourceLimits,
    #[error("external capability retry policy is invalid")]
    InvalidRetryPolicy,
    #[error("external capability health policy is invalid")]
    InvalidHealthPolicy,
    #[error("external capability receipts must require correlation, source digest, and observation time")]
    IncompleteProvenance,
    #[error("external capability cannot own task, memory, or governance authority")]
    DuplicateAuthority,
    #[error("external capability authority ceiling exceeds execute-with-approval")]
    ExcessiveAuthority,
    #[error("secret requirements must use secret references and cannot contain values")]
    InlineSecret,
    #[error("external network access requires declared egress destinations")]
    MissingEgressDestination,
    #[error("compensation is required for declared external writes")]
    MissingCompensation,
    #[error("rollback and removal commands must be bounded operation IDs, not shell text")]
    UnsafeOperationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCapability {
    pub schema_version: String,
    pub identity: ExternalCapabilityIdentity,
    pub capabilities: BTreeMap<String, ExternalCapabilityMaturity>,
    pub data_classes: BTreeSet<DataClass>,
    pub requirements: ExternalRequirements,
    pub interface: ExternalInterface,
    pub resource_limits: ExternalResourceLimits,
    pub health: ExternalHealthContract,
    pub lifecycle: ExternalLifecycleContract,
    pub provenance: ExternalReceiptProvenance,
    pub compatibility: ExternalCompatibility,
    pub authority: ExternalAuthorityBoundary,
}

impl ExternalCapability {
    pub const SCHEMA_VERSION: &'static str = "arda.external-capability.v1";

    pub fn from_json_str(raw: &str) -> Result<Self, ExternalCapabilityError> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|error| ExternalCapabilityError::InvalidJson(error.to_string()))?;
        if let Some(version) = value.get("schema_version").and_then(|value| value.as_str()) {
            if version != Self::SCHEMA_VERSION {
                return Err(ExternalCapabilityError::UnsupportedSchemaVersion(
                    version.to_owned(),
                ));
            }
        }
        let contract: Self = serde_json::from_value(value)
            .map_err(|error| ExternalCapabilityError::InvalidJson(error.to_string()))?;
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), ExternalCapabilityError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(ExternalCapabilityError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        for (field, value) in [
            ("identity.adapter_id", self.identity.adapter_id.as_str()),
            ("identity.candidate_id", self.identity.candidate_id.as_str()),
            ("identity.version", self.identity.version.as_str()),
            ("identity.source", self.identity.source.as_str()),
            ("identity.license", self.identity.license.as_str()),
            ("identity.sbom.format", self.identity.sbom.format.as_str()),
            (
                "identity.sbom.reference",
                self.identity.sbom.reference.as_str(),
            ),
            (
                "interface.input_schema",
                self.interface.input_schema.as_str(),
            ),
            (
                "interface.output_schema",
                self.interface.output_schema.as_str(),
            ),
            (
                "health.probe_operation",
                self.health.probe_operation.as_str(),
            ),
            (
                "provenance.receipt_schema",
                self.provenance.receipt_schema.as_str(),
            ),
            (
                "compatibility.minimum_arda_version",
                self.compatibility.minimum_arda_version.as_str(),
            ),
        ] {
            require_text(field, value)?;
        }
        require_digest("identity.source_digest", &self.identity.source_digest)?;
        require_digest("identity.sbom.digest", &self.identity.sbom.digest)?;
        if self.capabilities.is_empty() {
            return Err(ExternalCapabilityError::MissingCapability);
        }
        if self.capabilities.keys().any(|id| id.trim().is_empty()) {
            return Err(ExternalCapabilityError::EmptyField("capabilities.id"));
        }
        if self.interface.protocols.is_empty() {
            return Err(ExternalCapabilityError::MissingProtocol);
        }
        if self.resource_limits.timeout_ms == 0
            || self.resource_limits.max_input_bytes == 0
            || self.resource_limits.max_output_bytes == 0
            || self.resource_limits.max_memory_bytes == 0
            || self.resource_limits.max_processes == 0
        {
            return Err(ExternalCapabilityError::InvalidResourceLimits);
        }
        if self.lifecycle.retry.max_attempts == 0
            || self.lifecycle.retry.initial_delay_ms > self.lifecycle.retry.max_delay_ms
        {
            return Err(ExternalCapabilityError::InvalidRetryPolicy);
        }
        if self.health.freshness_secs == 0
            || self.health.degraded_after_failures == 0
            || self.health.degraded_after_failures > self.health.unavailable_after_failures
        {
            return Err(ExternalCapabilityError::InvalidHealthPolicy);
        }
        if !self.provenance.correlation_required
            || !self.provenance.source_digest_required
            || !self.provenance.observed_at_required
        {
            return Err(ExternalCapabilityError::IncompleteProvenance);
        }
        if self.authority.task_authority
            || self.authority.memory_authority
            || self.authority.governance_authority
        {
            return Err(ExternalCapabilityError::DuplicateAuthority);
        }
        if !CompositionAuthorityClass::ExecuteWithApproval.permits(self.authority.authority_ceiling)
        {
            return Err(ExternalCapabilityError::ExcessiveAuthority);
        }
        if self.requirements.secrets.iter().any(|secret| {
            secret.purpose.trim().is_empty()
                || secret.reference.trim().is_empty()
                || !secret.reference.starts_with("secret://")
                || secret.reference.contains('=')
        }) {
            return Err(ExternalCapabilityError::InlineSecret);
        }
        if self.requirements.network == ExternalNetworkRequirement::ExternalApproved
            && (self.requirements.egress_destinations.is_empty()
                || self
                    .requirements
                    .egress_destinations
                    .iter()
                    .any(|destination| destination.trim().is_empty()))
        {
            return Err(ExternalCapabilityError::MissingEgressDestination);
        }
        if self.requirements.filesystem_write
            && self.lifecycle.compensation == ExternalCompensationPolicy::None
        {
            return Err(ExternalCapabilityError::MissingCompensation);
        }
        for operation in [
            &self.compatibility.backup_operation,
            &self.compatibility.removal_operation,
            &self.compatibility.rollback_operation,
        ] {
            if operation.trim().is_empty()
                || operation.chars().any(|character| character.is_whitespace())
                || operation.contains([';', '|', '&'])
            {
                return Err(ExternalCapabilityError::UnsafeOperationId);
            }
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, ExternalCapabilityError> {
        self.validate()?;
        serde_json::to_string(self)
            .map_err(|error| ExternalCapabilityError::InvalidJson(error.to_string()))
    }

    pub fn digest(&self) -> Result<String, ExternalCapabilityError> {
        let digest = Sha256::digest(self.canonical_json()?.as_bytes());
        Ok(format!("sha256:{digest:x}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCapabilityIdentity {
    pub adapter_id: String,
    pub candidate_id: String,
    pub version: String,
    pub source: String,
    pub license: String,
    pub source_digest: String,
    pub sbom: SbomReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SbomReference {
    pub format: String,
    pub digest: String,
    pub reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCapabilityMaturity {
    Experimental,
    Preview,
    Stable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRequirements {
    pub filesystem_read: bool,
    pub filesystem_write: bool,
    pub process_spawn: bool,
    pub network: ExternalNetworkRequirement,
    #[serde(default)]
    pub egress_destinations: BTreeSet<String>,
    #[serde(default)]
    pub devices: BTreeSet<String>,
    #[serde(default)]
    pub secrets: BTreeSet<SecretRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalNetworkRequirement {
    Denied,
    LocalOnly,
    ExternalApproved,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretRequirement {
    pub purpose: String,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalInterface {
    pub input_schema: String,
    pub output_schema: String,
    pub protocols: BTreeSet<ExternalProtocol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalProtocol {
    ProjectAdapter,
    Mcp,
    Otlp,
    CalDav,
    Jsonl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalResourceLimits {
    pub timeout_ms: u64,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_memory_bytes: u64,
    pub max_processes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalHealthContract {
    pub probe_operation: String,
    pub freshness_secs: u64,
    pub degraded_after_failures: u32,
    pub unavailable_after_failures: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalLifecycleContract {
    pub retry: ExternalRetryPolicy,
    pub cancellation: ExternalCancellationPolicy,
    pub idempotency: ExternalIdempotencyPolicy,
    pub compensation: ExternalCompensationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCancellationPolicy {
    Immediate,
    Graceful,
    CheckpointThenStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalIdempotencyPolicy {
    Required,
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCompensationPolicy {
    None,
    BestEffort,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalReceiptProvenance {
    pub receipt_schema: String,
    pub correlation_required: bool,
    pub source_digest_required: bool,
    pub observed_at_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCompatibility {
    pub minimum_arda_version: String,
    pub tested_at: DateTime<Utc>,
    pub backup_operation: String,
    pub removal_operation: String,
    pub rollback_operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalAuthorityBoundary {
    pub authority_ceiling: CompositionAuthorityClass,
    pub task_authority: bool,
    pub memory_authority: bool,
    pub governance_authority: bool,
}

fn require_text(field: &'static str, value: &str) -> Result<(), ExternalCapabilityError> {
    if value.trim().is_empty() {
        Err(ExternalCapabilityError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn require_digest(field: &'static str, value: &str) -> Result<(), ExternalCapabilityError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(ExternalCapabilityError::InvalidDigest(field));
    };
    if hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ExternalCapabilityError::InvalidDigest(field))
    }
}
