use std::path::{Component, Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contracts::{Finding, FindingConfidenceClass, FindingSeverity, FindingStatus};
use crate::error::{Result, RumilError};

/// Provider-neutral finding input. `evidence_identity` is the stable identity
/// of the check/advisory, not a run-specific command or output digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingDraft {
    pub category: String,
    pub severity: FindingSeverity,
    pub status: FindingStatus,
    pub confidence_class: FindingConfidenceClass,
    pub path_or_scope: String,
    pub evidence_identity: String,
    pub summary: String,
    pub recommendation: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub provider_id: Option<String>,
    pub source_command_id: Option<Uuid>,
    pub review_required: bool,
}

/// Explicit operator disposition. Feedback is evidence and never silently
/// mutates a classifier or provider policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingDisposition {
    Accepted,
    FalsePositive,
    RiskAccepted,
    Deferred,
    NeedsEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingFeedback {
    pub finding_id: Uuid,
    pub disposition: FindingDisposition,
    pub rationale: String,
    pub reviewed_by: String,
    pub reviewed_at_utc: DateTime<Utc>,
    /// Always false. Learning requires a separate versioned policy.
    pub mutates_classifier: bool,
}

impl FindingFeedback {
    pub fn new(
        finding_id: Uuid,
        disposition: FindingDisposition,
        rationale: impl Into<String>,
        reviewed_by: impl Into<String>,
        reviewed_at_utc: DateTime<Utc>,
    ) -> Result<Self> {
        let rationale = rationale.into();
        let reviewed_by = reviewed_by.into();
        if rationale.trim().is_empty() || reviewed_by.trim().is_empty() {
            return Err(RumilError::InvalidRequest(
                "finding feedback requires rationale and reviewer".to_string(),
            ));
        }
        Ok(Self {
            finding_id,
            disposition,
            rationale,
            reviewed_by,
            reviewed_at_utc,
            mutates_classifier: false,
        })
    }
}

/// Normalize a provider/tool/heuristic result into the canonical finding
/// contract. The stable UUID excludes run-specific audit and command IDs.
pub fn normalize_finding(
    project_id: Uuid,
    audit_id: Uuid,
    mut draft: FindingDraft,
) -> Result<Finding> {
    require_nonempty("category", &draft.category)?;
    require_nonempty("evidence_identity", &draft.evidence_identity)?;
    require_nonempty("summary", &draft.summary)?;
    let path_or_scope = normalize_relative_scope(&draft.path_or_scope)?;
    draft.evidence_refs.sort();
    draft.evidence_refs.dedup();

    let stable_identity = serde_json::to_vec(&(
        draft.category.trim(),
        path_or_scope.as_str(),
        draft.evidence_identity.trim(),
    ))?;
    let finding_id = Uuid::new_v5(&project_id, &stable_identity);

    Ok(Finding {
        finding_id,
        audit_id,
        category: draft.category,
        severity: draft.severity,
        status: draft.status,
        confidence_class: draft.confidence_class,
        path_or_scope,
        summary: draft.summary,
        recommendation: draft.recommendation,
        evidence_refs: draft.evidence_refs,
        provider_id: draft.provider_id,
        source_command_id: draft.source_command_id,
        prior_finding_id: None,
        review_required: draft.review_required,
        mutation_allowed: false,
    })
}

fn require_nonempty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(RumilError::InvalidRequest(format!(
            "finding {field} must not be empty"
        )));
    }
    Ok(())
}

fn normalize_relative_scope(value: &str) -> Result<String> {
    require_nonempty("path_or_scope", value)?;
    let normalized = value.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RumilError::PathRejected(
            "finding scope must be project-relative".to_string(),
        ));
    }
    Ok(normalized)
}
