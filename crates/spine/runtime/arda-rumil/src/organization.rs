use std::collections::BTreeMap;
use std::path::{Component, Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "crypto")]
use crate::contracts::LegacyHadesImport;
use crate::contracts::{
    OrganizationCandidate, OrganizationPlan, OrganizationPlanStatus, OrganizationRisk,
};
use crate::error::{Result, RumilError};
use crate::inventory::InventoryReport;

/// Project-neutral organization rules. A rule is inert unless the selected
/// profile explicitly enables it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationRule {
    MissingRootDocument,
    StaleGeneratedArtifact,
    MisplacedOutput,
    DuplicatePath,
    DocumentationDrift,
}

/// Declarative organization settings shared by Rust and non-Rust projects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationProfile {
    pub profile_id: String,
    #[serde(default)]
    pub enabled_rules: Vec<OrganizationRule>,
    #[serde(default)]
    pub required_root_documents: Vec<String>,
    #[serde(default)]
    pub allowed_output_roots: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationIssueKind {
    StaleGeneratedArtifact,
    MisplacedOutput,
    DocumentationDrift,
}

impl OrganizationIssueKind {
    fn rule(self) -> OrganizationRule {
        match self {
            Self::StaleGeneratedArtifact => OrganizationRule::StaleGeneratedArtifact,
            Self::MisplacedOutput => OrganizationRule::MisplacedOutput,
            Self::DocumentationDrift => OrganizationRule::DocumentationDrift,
        }
    }

    fn candidate_type(self) -> &'static str {
        match self {
            Self::StaleGeneratedArtifact => "stale_generated_artifact",
            Self::MisplacedOutput => "misplaced_output",
            Self::DocumentationDrift => "documentation_drift",
        }
    }

    fn risk(self) -> OrganizationRisk {
        match self {
            Self::StaleGeneratedArtifact => OrganizationRisk::Low,
            Self::MisplacedOutput | Self::DocumentationDrift => OrganizationRisk::Medium,
        }
    }
}

/// Tool-backed organization evidence supplied to the planner. Rúmil does not
/// infer stale timestamps or documentation semantics when no provider supplied
/// that evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationObservation {
    pub kind: OrganizationIssueKind,
    pub path: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub affected_paths: Vec<String>,
    pub detail: String,
}

/// Receipt emitted for every planning run, including zero-candidate runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationDryRunReceipt {
    pub receipt_id: Uuid,
    pub plan_id: Uuid,
    pub audit_id: Uuid,
    pub profile_id: String,
    pub candidate_count: u64,
    pub generated_at_utc: DateTime<Utc>,
    pub dry_run: bool,
    pub filesystem_mutated: bool,
    pub authority: String,
}

/// Describes the future governance boundary without exposing an implementation
/// or direct mutation API from this crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationHandoffBoundary {
    pub governance_contract: String,
    pub implemented: bool,
    pub direct_mutation_available: bool,
    pub operator_approval_required: bool,
    pub exact_path_and_digest_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationPlanBundle {
    pub plan: OrganizationPlan,
    pub receipt: OrganizationDryRunReceipt,
    pub mutation_handoff: MutationHandoffBoundary,
}

/// Produce a deterministic review-only organization plan. This function reads
/// normalized inventory/observations only and performs no filesystem mutation.
pub fn plan_organization(
    audit_id: Uuid,
    project_id: Uuid,
    profile: &OrganizationProfile,
    inventory: &InventoryReport,
    observations: &[OrganizationObservation],
    generated_at_utc: DateTime<Utc>,
) -> Result<OrganizationPlanBundle> {
    validate_profile(profile)?;
    let mut candidates = Vec::new();

    if enabled(profile, OrganizationRule::MissingRootDocument) {
        add_missing_document_candidates(project_id, profile, inventory, &mut candidates)?;
    }
    if enabled(profile, OrganizationRule::DuplicatePath) {
        add_duplicate_path_candidates(project_id, inventory, &mut candidates)?;
    }
    add_observed_candidates(project_id, profile, observations, &mut candidates)?;
    candidates.sort_by(|left, right| {
        left.candidate_type
            .cmp(&right.candidate_type)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let candidate_ids: Vec<_> = candidates
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect();
    let plan_identity = serde_json::to_vec(&(
        audit_id,
        profile.profile_id.as_str(),
        generated_at_utc,
        &candidate_ids,
    ))?;
    let plan_id = Uuid::new_v5(&project_id, &plan_identity);
    let receipt_identity = serde_json::to_vec(&(plan_id, "organization_dry_run"))?;
    let receipt_id = Uuid::new_v5(&project_id, &receipt_identity);
    let candidate_count = candidates.len() as u64;

    Ok(OrganizationPlanBundle {
        plan: OrganizationPlan {
            plan_id,
            audit_id,
            profile_id: profile.profile_id.clone(),
            scope: ".".to_string(),
            no_delete: true,
            no_move: true,
            no_rewrite: true,
            operator_review_required: true,
            mutation_authorized: false,
            generated_at_utc,
            candidates,
            status: OrganizationPlanStatus::Proposed,
        },
        receipt: OrganizationDryRunReceipt {
            receipt_id,
            plan_id,
            audit_id,
            profile_id: profile.profile_id.clone(),
            candidate_count,
            generated_at_utc,
            dry_run: true,
            filesystem_mutated: false,
            authority: "review_only".to_string(),
        },
        mutation_handoff: MutationHandoffBoundary {
            governance_contract: "external_operator_governance_receipt".to_string(),
            implemented: false,
            direct_mutation_available: false,
            operator_approval_required: true,
            exact_path_and_digest_required: true,
        },
    })
}

fn validate_profile(profile: &OrganizationProfile) -> Result<()> {
    if profile.profile_id.trim().is_empty() {
        return Err(RumilError::InvalidRequest(
            "organization profile_id must not be empty".to_string(),
        ));
    }
    for path in profile
        .required_root_documents
        .iter()
        .chain(profile.allowed_output_roots.iter())
    {
        validate_relative_path(path)?;
    }
    Ok(())
}

fn enabled(profile: &OrganizationProfile, rule: OrganizationRule) -> bool {
    profile.enabled_rules.contains(&rule)
}

fn add_missing_document_candidates(
    project_id: Uuid,
    profile: &OrganizationProfile,
    inventory: &InventoryReport,
    candidates: &mut Vec<OrganizationCandidate>,
) -> Result<()> {
    let present: std::collections::BTreeSet<_> = inventory
        .entries
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();
    for required in &profile.required_root_documents {
        if !present.contains(required.as_str()) {
            candidates.push(candidate(
                project_id,
                "missing_root_document",
                required,
                OrganizationRisk::Low,
                "review whether this project needs the missing root document",
                vec![format!("inventory:absent:{required}")],
                vec![required.clone()],
                None,
            )?);
        }
    }
    Ok(())
}

fn add_duplicate_path_candidates(
    project_id: Uuid,
    inventory: &InventoryReport,
    candidates: &mut Vec<OrganizationCandidate>,
) -> Result<()> {
    let mut folded: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in &inventory.entries {
        if entry.relative_path != "." {
            folded
                .entry(entry.relative_path.to_lowercase())
                .or_default()
                .push(entry.relative_path.clone());
        }
    }
    for paths in folded.values_mut() {
        paths.sort();
        paths.dedup();
        if paths.len() > 1 {
            candidates.push(candidate(
                project_id,
                "duplicate_path",
                &paths[0],
                OrganizationRisk::High,
                "review case-colliding paths; no move or deletion is authorized",
                vec![format!("inventory:case_collision:{}", paths.join("|"))],
                paths.clone(),
                Some("retain every path until an operator approves an exact-path handoff".into()),
            )?);
        }
    }
    Ok(())
}

fn add_observed_candidates(
    project_id: Uuid,
    profile: &OrganizationProfile,
    observations: &[OrganizationObservation],
    candidates: &mut Vec<OrganizationCandidate>,
) -> Result<()> {
    for observation in observations {
        if !enabled(profile, observation.kind.rule()) {
            continue;
        }
        validate_relative_path(&observation.path)?;
        if observation.detail.trim().is_empty() || observation.evidence_refs.is_empty() {
            return Err(RumilError::InvalidRequest(
                "organization observations require detail and evidence".to_string(),
            ));
        }
        let mut evidence_refs = observation.evidence_refs.clone();
        evidence_refs.sort();
        evidence_refs.dedup();
        let mut affected_paths = observation.affected_paths.clone();
        if affected_paths.is_empty() {
            affected_paths.push(observation.path.clone());
        }
        for path in &affected_paths {
            validate_relative_path(path)?;
        }
        affected_paths.sort();
        affected_paths.dedup();
        candidates.push(candidate(
            project_id,
            observation.kind.candidate_type(),
            &observation.path,
            observation.kind.risk(),
            &format!("review evidence before any action: {}", observation.detail),
            evidence_refs,
            affected_paths,
            None,
        )?);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn candidate(
    project_id: Uuid,
    candidate_type: &str,
    path: &str,
    risk: OrganizationRisk,
    recommended_action: &str,
    mut evidence_refs: Vec<String>,
    mut affected_paths: Vec<String>,
    rollback_note: Option<String>,
) -> Result<OrganizationCandidate> {
    validate_relative_path(path)?;
    evidence_refs.sort();
    evidence_refs.dedup();
    affected_paths.sort();
    affected_paths.dedup();
    let identity = serde_json::to_vec(&(candidate_type, path, &evidence_refs, &affected_paths))?;
    Ok(OrganizationCandidate {
        candidate_id: Uuid::new_v5(&project_id, &identity),
        path: path.to_string(),
        candidate_type: candidate_type.to_string(),
        risk,
        recommended_action: recommended_action.to_string(),
        evidence_refs,
        affected_paths,
        rollback_note,
    })
}

fn validate_relative_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RumilError::PathRejected(
            "organization paths must be non-empty and project-relative".to_string(),
        ));
    }
    Ok(())
}

/// Import legacy HADES organization output as historical provenance only. The
/// original bytes are hashed; no HADES contract is emitted from new code.
#[cfg(feature = "crypto")]
pub fn import_legacy_hades_organization_report(
    legacy_path: &str,
    raw_report: &[u8],
    legacy_generated_at_utc: DateTime<Utc>,
    mapped_project_id: Uuid,
    mapped_rumil_audit_id: Uuid,
    mapping_quality: &str,
) -> Result<LegacyHadesImport> {
    validate_relative_path(legacy_path)?;
    if raw_report.is_empty() || mapping_quality.trim().is_empty() {
        return Err(RumilError::InvalidRequest(
            "legacy import requires report bytes and mapping quality".to_string(),
        ));
    }
    let legacy_sha256 = crate::hash::sha256_bytes(raw_report);
    let identity = serde_json::to_vec(&(
        legacy_path,
        legacy_sha256.as_str(),
        mapped_project_id,
        mapped_rumil_audit_id,
    ))?;
    Ok(LegacyHadesImport {
        import_id: Uuid::new_v5(&mapped_project_id, &identity),
        legacy_source: "hades".to_string(),
        legacy_contract: "hades.organization-plan.legacy".to_string(),
        legacy_path: legacy_path.to_string(),
        legacy_sha256,
        legacy_generated_at_utc,
        mapped_project_id,
        mapped_rumil_audit_id,
        mapping_quality: mapping_quality.to_string(),
        unmapped_fields: Vec::new(),
        historical_only: true,
        native_rumil_evidence: false,
    })
}
