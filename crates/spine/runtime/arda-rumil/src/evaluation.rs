use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contracts::{AuditReport, AuditReportCompleteness, Finding, FindingConfidenceClass};
use crate::error::{Result, RumilError};

/// Bounded evidence classes projected to reasoning and evaluation consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RumilEvidenceClass {
    ToolBacked,
    Heuristic,
    Historical,
    Partial,
    Unavailable,
}

/// Receipt-only projection for consumers that must not read the project tree or
/// embed unbounded audit source content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RumilEvidenceReference {
    pub audit_id: Uuid,
    pub project_id: Uuid,
    pub packet_reference: String,
    pub packet_sha256: String,
    pub generated_at_utc: chrono::DateTime<chrono::Utc>,
    pub completeness: AuditReportCompleteness,
    pub classes: Vec<RumilEvidenceClass>,
    pub stale_baseline: bool,
    #[serde(default)]
    pub rejected_providers: Vec<String>,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    pub authority: String,
    pub execution_authorized: bool,
}

/// Convert a report into bounded receipt metadata. Finding summaries, file
/// records, excerpts, package graphs, and command output are deliberately not
/// copied into the projection.
pub fn project_evidence_reference(
    report: &AuditReport,
    findings: &[Finding],
    packet_reference: &str,
    packet_sha256: &str,
    stale_baseline: bool,
) -> Result<RumilEvidenceReference> {
    validate_packet_reference(packet_reference)?;
    if packet_sha256.trim().is_empty() {
        return Err(RumilError::InvalidRequest(
            "Rúmil evidence requires a packet digest".to_string(),
        ));
    }
    if report.authority != "advisory_read_only" {
        return Err(RumilError::InvalidRequest(
            "Rúmil evidence projection requires advisory_read_only authority".to_string(),
        ));
    }

    let mut classes = Vec::new();
    let has_class = |class| {
        findings
            .iter()
            .any(|finding| finding.confidence_class == class)
    };
    if has_class(FindingConfidenceClass::ToolBacked)
        || has_class(FindingConfidenceClass::SourceBacked)
        || !report.command_receipts.is_empty()
    {
        classes.push(RumilEvidenceClass::ToolBacked);
    }
    if has_class(FindingConfidenceClass::Heuristic) {
        classes.push(RumilEvidenceClass::Heuristic);
    }
    if has_class(FindingConfidenceClass::Historical) {
        classes.push(RumilEvidenceClass::Historical);
    }
    if !report.completeness.is_complete() {
        classes.push(RumilEvidenceClass::Partial);
    }
    if has_class(FindingConfidenceClass::Unavailable)
        || !report.capabilities_unavailable.is_empty()
        || matches!(report.completeness, AuditReportCompleteness::Failed)
    {
        classes.push(RumilEvidenceClass::Unavailable);
    }

    let mut rejected_providers = report
        .capabilities_unavailable
        .iter()
        .filter(|outcome| outcome.status == "rejected" || outcome.status == "denied")
        .filter_map(|outcome| outcome.provider_id.clone())
        .collect::<Vec<_>>();
    rejected_providers.sort();
    rejected_providers.dedup();

    let mut missing_evidence = report
        .capabilities_unavailable
        .iter()
        .map(|outcome| outcome.capability.clone())
        .collect::<Vec<_>>();
    missing_evidence.sort();
    missing_evidence.dedup();

    Ok(RumilEvidenceReference {
        audit_id: report.audit_id,
        project_id: report.project_id,
        packet_reference: packet_reference.to_string(),
        packet_sha256: packet_sha256.to_string(),
        generated_at_utc: report.generated_at_utc,
        completeness: report.completeness,
        classes,
        stale_baseline,
        rejected_providers,
        missing_evidence,
        authority: report.authority.clone(),
        execution_authorized: false,
    })
}

fn validate_packet_reference(reference: &str) -> Result<()> {
    let path = Path::new(reference);
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
            "Rúmil packet references must be project-relative".to_string(),
        ));
    }
    Ok(())
}
