use crate::project_contract::SafeRelativePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CapabilityCompositionError {
    #[error("invalid capability composition JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported capability composition schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("invalid JouleWork budget")]
    InvalidBudget,
    #[error("invalid signed request digest")]
    InvalidSignedRequestDigest,
    #[error("capability `{0}` appears in conflicting sets")]
    CapabilityConflict(String),
    #[error("sensitive data cannot be permitted external egress")]
    SensitiveExternalEgress,
    #[error("role `{role_id}` requests {requested:?} above signed ceiling {ceiling:?}")]
    AuthorityEscalation {
        role_id: String,
        requested: CompositionAuthorityClass,
        ceiling: CompositionAuthorityClass,
    },
    #[error("invalid filesystem permission path: {0}")]
    InvalidFilesystemPath(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityComposition {
    pub schema_version: String,
    pub lineage: CompositionLineage,
    pub outcome: OutcomeContract,
    pub scope: CompositionScope,
    pub sensitivity: SensitivityPolicy,
    pub capabilities: CapabilitySelection,
    pub roles: BTreeMap<String, RoleRequirement>,
    pub route_preferences: RoutePreferences,
    pub council_mode: CouncilMode,
    pub proactive_communication: ProactiveCommunicationMode,
    pub joulework_budget: JouleWorkBudget,
    pub authority: AuthorityContract,
    pub permissions: CompositionPermissions,
    pub lifecycle: LifecyclePolicy,
}

impl CapabilityComposition {
    pub const SCHEMA_VERSION: &'static str = "arda.capability-composition.v1";

    pub fn from_json_str(raw: &str) -> Result<Self, CapabilityCompositionError> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|error| CapabilityCompositionError::InvalidJson(error.to_string()))?;
        if let Some(version) = value.get("schema_version").and_then(|value| value.as_str()) {
            if version != Self::SCHEMA_VERSION {
                return Err(CapabilityCompositionError::UnsupportedSchemaVersion(
                    version.to_owned(),
                ));
            }
        }
        let composition: Self = serde_json::from_value(value)
            .map_err(|error| CapabilityCompositionError::InvalidJson(error.to_string()))?;
        composition.validate()?;
        Ok(composition)
    }

    pub fn validate(&self) -> Result<(), CapabilityCompositionError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(CapabilityCompositionError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        require_text("lineage.objective_id", &self.lineage.objective_id)?;
        require_text("lineage.run_id", &self.lineage.run_id)?;
        require_text(
            "lineage.project_contract_digest",
            &self.lineage.project_contract_digest,
        )?;
        require_text("outcome.description", &self.outcome.description)?;
        if self.outcome.acceptance_conditions.is_empty() {
            return Err(CapabilityCompositionError::EmptyField(
                "outcome.acceptance_conditions",
            ));
        }
        if self.capabilities.required.is_empty() {
            return Err(CapabilityCompositionError::EmptyField(
                "capabilities.required",
            ));
        }
        reject_capability_conflicts(&self.capabilities)?;

        if self
            .sensitivity
            .data_classes
            .iter()
            .any(DataClass::is_sensitive)
            && self
                .sensitivity
                .permitted_egress
                .iter()
                .any(EgressTarget::is_external)
        {
            return Err(CapabilityCompositionError::SensitiveExternalEgress);
        }

        if !self.joulework_budget.max_joules.is_finite()
            || !self.joulework_budget.max_cost_usd.is_finite()
            || self.joulework_budget.max_joules < 0.0
            || self.joulework_budget.max_cost_usd < 0.0
        {
            return Err(CapabilityCompositionError::InvalidBudget);
        }
        if !is_sha256_digest(&self.authority.signed_request_digest) {
            return Err(CapabilityCompositionError::InvalidSignedRequestDigest);
        }
        for (role_id, role) in &self.roles {
            require_text("roles.role_id", role_id)?;
            if role.authority.rank() > self.authority.authority_ceiling.rank() {
                return Err(CapabilityCompositionError::AuthorityEscalation {
                    role_id: role_id.clone(),
                    requested: role.authority,
                    ceiling: self.authority.authority_ceiling,
                });
            }
        }
        for path in self
            .permissions
            .filesystem
            .read_roots
            .iter()
            .chain(&self.permissions.filesystem.write_roots)
        {
            SafeRelativePath::new(path.clone())
                .map_err(|_| CapabilityCompositionError::InvalidFilesystemPath(path.clone()))?;
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, CapabilityCompositionError> {
        self.validate()?;
        serde_json::to_string(self)
            .map_err(|error| CapabilityCompositionError::InvalidJson(error.to_string()))
    }

    pub fn digest(&self) -> Result<String, CapabilityCompositionError> {
        let canonical = self.canonical_json()?;
        let digest = Sha256::digest(canonical.as_bytes());
        Ok(format!("sha256:{digest:x}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionLineage {
    pub objective_id: String,
    pub project_id: Uuid,
    pub run_id: String,
    pub project_contract_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeContract {
    pub description: String,
    pub acceptance_conditions: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionScope {
    Personal,
    Business,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityPolicy {
    pub data_classes: BTreeSet<DataClass>,
    pub permitted_egress: BTreeSet<EgressTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
    Public,
    Internal,
    Private,
    Health,
    Financial,
}

impl DataClass {
    fn is_sensitive(&self) -> bool {
        matches!(self, Self::Private | Self::Health | Self::Financial)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressTarget {
    LocalDevice,
    LocalNetwork,
    HostedProvider,
    ExternalAdapter,
    PublicNetwork,
}

impl EgressTarget {
    fn is_external(&self) -> bool {
        matches!(
            self,
            Self::HostedProvider | Self::ExternalAdapter | Self::PublicNetwork
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySelection {
    pub required: BTreeSet<String>,
    #[serde(default)]
    pub optional: BTreeSet<String>,
    #[serde(default)]
    pub forbidden: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleRequirement {
    pub kind: RoleKind,
    pub authority: CompositionAuthorityClass,
    #[serde(default)]
    pub required_capabilities: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleKind {
    Worker,
    Planner,
    Reviewer,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionAuthorityClass {
    ReadOnly,
    Plan,
    Propose,
    ExecuteWithApproval,
    ExecuteBounded,
}

impl CompositionAuthorityClass {
    fn rank(self) -> u8 {
        match self {
            Self::ReadOnly => 0,
            Self::Plan => 1,
            Self::Propose => 2,
            Self::ExecuteWithApproval => 3,
            Self::ExecuteBounded => 4,
        }
    }

    pub fn permits(self, requested: Self) -> bool {
        requested.rank() <= self.rank()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePreferences {
    pub mode: RouteMode,
    #[serde(default)]
    pub allowed_providers: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteMode {
    LocalOnly,
    PreferLocal,
    PreferHosted,
    HostedOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilMode {
    Disabled,
    Advisory,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveCommunicationMode {
    Disabled,
    ApprovalRequired,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JouleWorkBudget {
    pub max_joules: f64,
    pub max_cost_usd: f64,
    pub max_operator_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityContract {
    pub signed_request_digest: String,
    pub authority_ceiling: CompositionAuthorityClass,
    #[serde(default)]
    pub approval_scopes: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionPermissions {
    #[serde(default)]
    pub tools: BTreeSet<String>,
    pub filesystem: FilesystemPermissions,
    pub network: NetworkPermission,
    #[serde(default)]
    pub devices: BTreeSet<String>,
    #[serde(default)]
    pub payments: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemPermissions {
    #[serde(default)]
    pub read_roots: BTreeSet<String>,
    #[serde(default)]
    pub write_roots: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPermission {
    Denied,
    LocalOnly,
    ExternalApproved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecyclePolicy {
    pub restart: RestartPolicy,
    pub cancellation: CancellationPolicy,
    pub compensation: CompensationPolicy,
    pub notification: NotificationPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    ResumeFromCheckpoint,
    RestartNode,
    OperatorDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationPolicy {
    Immediate,
    CheckpointThenStop,
    OperatorApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompensationPolicy {
    None,
    BestEffort,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPolicy {
    Silent,
    OnStateChange,
    OnIntervention,
    Always,
}

fn reject_capability_conflicts(
    selection: &CapabilitySelection,
) -> Result<(), CapabilityCompositionError> {
    for capability in &selection.required {
        if selection.optional.contains(capability) || selection.forbidden.contains(capability) {
            return Err(CapabilityCompositionError::CapabilityConflict(
                capability.clone(),
            ));
        }
    }
    for capability in &selection.optional {
        if selection.forbidden.contains(capability) {
            return Err(CapabilityCompositionError::CapabilityConflict(
                capability.clone(),
            ));
        }
    }
    Ok(())
}

fn require_text(field: &'static str, value: &str) -> Result<(), CapabilityCompositionError> {
    if value.trim().is_empty() {
        Err(CapabilityCompositionError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
