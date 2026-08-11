use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Result, RumilError};
use crate::inventory::InventoryReport;
use crate::policy::{AuditPolicy, BudgetPolicy, ExclusionKind, ExclusionRule, RootIdentity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Arda,
    Rust,
    Node,
    Python,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTarget {
    Host,
    PiCollector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileInventory {
    pub root_relative: String,
    pub max_depth: usize,
    pub max_files: u64,
    pub max_total_bytes: u64,
    pub max_excerpt_bytes: u64,
    pub scan_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub redactions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRetention {
    pub packet_class: String,
    pub max_age_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileOrganization {
    pub review_required: bool,
    pub no_delete: bool,
    pub no_move: bool,
    pub no_rewrite: bool,
    pub mutation_authorized: bool,
}

/// Declarative project-kind profile. Provider IDs are registry hooks; profiles
/// never accept arbitrary commands or arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectProfile {
    pub schema_version: String,
    pub profile_id: String,
    pub project_kind: ProjectKind,
    pub execution_role: ExecutionTarget,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    pub inventory: ProfileInventory,
    pub retention: ProfileRetention,
    pub organization: ProfileOrganization,
}

impl ProjectProfile {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != "arda.rumil.project-profile.v1"
            || self.profile_id.trim().is_empty()
            || !safe_relative_root(&self.inventory.root_relative)
            || self.inventory.max_depth == 0
            || self.inventory.max_files == 0
            || self.inventory.max_total_bytes == 0
            || self.inventory.scan_timeout_seconds == 0
            || self.retention.packet_class.trim().is_empty()
        {
            return Err(RumilError::InvalidRequest(
                "Rúmil profile is incomplete or has an unsupported schema".to_string(),
            ));
        }
        if self.organization.mutation_authorized
            || !self.organization.review_required
            || !self.organization.no_delete
            || !self.organization.no_move
            || !self.organization.no_rewrite
        {
            return Err(RumilError::InvalidRequest(
                "Rúmil organization profiles must remain review-only and non-mutating".to_string(),
            ));
        }
        if self.execution_role == ExecutionTarget::PiCollector && !self.providers.is_empty() {
            return Err(RumilError::InvalidRequest(
                "Pi collectors may inventory only; provider execution belongs on the host"
                    .to_string(),
            ));
        }
        if self
            .providers
            .iter()
            .any(|provider| !registered_provider_id(provider))
        {
            return Err(RumilError::InvalidRequest(
                "profile names an unregistered provider hook".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn builtin_profile(profile_id: &str) -> Result<ProjectProfile> {
    let raw = match profile_id {
        "arda-v1" => include_str!("../profiles/arda-v1.toml"),
        "generic-rust-v1" => include_str!("../profiles/generic-rust-v1.toml"),
        "generic-node-v1" => include_str!("../profiles/generic-node-v1.toml"),
        "generic-python-v1" => include_str!("../profiles/generic-python-v1.toml"),
        "generic-mixed-v1" => include_str!("../profiles/generic-mixed-v1.toml"),
        _ => {
            return Err(RumilError::InvalidRequest(format!(
                "unknown Rúmil profile `{profile_id}`"
            )))
        }
    };
    let profile: ProjectProfile = toml::from_str(raw)?;
    profile.validate()?;
    Ok(profile)
}

pub fn validate_execution_target(profile: &ProjectProfile, target: ExecutionTarget) -> Result<()> {
    profile.validate()?;
    if profile.execution_role != target {
        return Err(RumilError::InvalidRequest(
            "requested execution target does not match the profile role".to_string(),
        ));
    }
    Ok(())
}

pub fn audit_with_profile(root: &Path, profile: &ProjectProfile) -> Result<InventoryReport> {
    validate_execution_target(profile, profile.execution_role)?;
    let canonical_root = root.canonicalize()?;
    let project_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, profile.profile_id.as_bytes());
    let policy = AuditPolicy {
        profile_id: profile.profile_id.clone(),
        root_identity: RootIdentity {
            project_id,
            name: canonical_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("project")
                .to_string(),
            kind: format!("{:?}", profile.project_kind).to_lowercase(),
            remote_url: None,
        },
        root_relative: profile.inventory.root_relative.clone(),
        exclusion_rules: profile
            .inventory
            .exclusions
            .iter()
            .map(|pattern| ExclusionRule {
                pattern: pattern.clone(),
                kind: ExclusionKind::Glob,
            })
            .collect(),
        budget: BudgetPolicy {
            max_depth: profile.inventory.max_depth,
            max_files: profile.inventory.max_files,
            max_total_bytes: profile.inventory.max_total_bytes,
            max_excerpt_bytes: profile.inventory.max_excerpt_bytes,
            scan_timeout_seconds: profile.inventory.scan_timeout_seconds,
            command_timeout_seconds: profile.inventory.command_timeout_seconds,
        },
        provider_allowlist: profile.providers.clone(),
        redaction_policy: profile.inventory.redactions.clone(),
    };
    crate::inventory::inventory_repo(&canonical_root, &policy)
}

fn safe_relative_root(root: &str) -> bool {
    let path = Path::new(root);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
}

fn registered_provider_id(provider: &str) -> bool {
    matches!(
        provider,
        "rumil.cargo_check.v1"
            | "rumil.cargo_audit.v1"
            | "rumil.cargo_deny.v1"
            | "rumil.cargo_modules.v1"
    )
}
