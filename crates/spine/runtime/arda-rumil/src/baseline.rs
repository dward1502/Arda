use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contracts::{AuditReport, Comparison, Finding, FindingSeverity, MemoryObservation};
#[cfg(feature = "crypto")]
use crate::contracts::{FindingConfidenceClass, FindingStatus, LegacyHadesImport};
#[cfg(feature = "crypto")]
use crate::error::{Result, RumilError};

/// Explicitly selected prior/current packet data used for replay-safe
/// comparison. Rúmil never guesses a baseline from filesystem recency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditBaseline {
    pub audit_id: Uuid,
    pub project_id: Uuid,
    pub source_revision: Option<String>,
    #[serde(default)]
    pub findings: Vec<Finding>,
}

impl AuditBaseline {
    pub fn empty(audit_id: Uuid, project_id: Uuid, source_revision: Option<String>) -> Self {
        Self {
            audit_id,
            project_id,
            source_revision,
            findings: Vec::new(),
        }
    }
}

/// Explicit historical baseline produced from retained HADES findings. The
/// provenance remains attached and the findings never become native evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg(feature = "crypto")]
pub struct LegacyHadesBaselineImport {
    pub provenance: LegacyHadesImport,
    pub baseline: AuditBaseline,
}

#[cfg(feature = "crypto")]
#[derive(Debug, Deserialize)]
struct LegacyHadesFindingsDocument {
    #[serde(default)]
    findings: Vec<LegacyHadesFinding>,
}

#[cfg(feature = "crypto")]
#[derive(Debug, Deserialize)]
struct LegacyHadesFinding {
    category: String,
    severity: String,
    #[serde(alias = "path_or_scope")]
    path: String,
    summary: String,
}

/// Migrate retained HADES findings into a comparison-only baseline while
/// preserving explicit historical provenance.
#[cfg(feature = "crypto")]
pub fn import_legacy_hades_findings(
    legacy_path: &str,
    raw_report: &[u8],
    legacy_generated_at_utc: chrono::DateTime<chrono::Utc>,
    mapped_project_id: Uuid,
    mapped_rumil_audit_id: Uuid,
    mapping_quality: &str,
) -> Result<LegacyHadesBaselineImport> {
    let path = std::path::Path::new(legacy_path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        || raw_report.is_empty()
        || mapping_quality.trim().is_empty()
    {
        return Err(RumilError::InvalidRequest(
            "legacy HADES import requires a relative path, bytes, and mapping quality".to_string(),
        ));
    }
    let document: LegacyHadesFindingsDocument = serde_json::from_slice(raw_report)?;
    let legacy_sha256 = crate::hash::sha256_bytes(raw_report);
    let mut findings = Vec::with_capacity(document.findings.len());
    for legacy in document.findings {
        if legacy.category.trim().is_empty()
            || legacy.path.trim().is_empty()
            || legacy.summary.trim().is_empty()
        {
            return Err(RumilError::InvalidRequest(
                "legacy HADES findings require category, path, and summary".to_string(),
            ));
        }
        let severity = match legacy.severity.as_str() {
            "info" => FindingSeverity::Info,
            "low" => FindingSeverity::Low,
            "medium" => FindingSeverity::Medium,
            "high" => FindingSeverity::High,
            "critical" => FindingSeverity::Critical,
            other => {
                return Err(RumilError::InvalidRequest(format!(
                    "unsupported legacy HADES severity `{other}`"
                )))
            }
        };
        let identity = serde_json::to_vec(&(
            mapped_rumil_audit_id,
            legacy.category.as_str(),
            legacy.path.as_str(),
            legacy.summary.as_str(),
        ))?;
        findings.push(Finding {
            finding_id: Uuid::new_v5(&mapped_project_id, &identity),
            audit_id: mapped_rumil_audit_id,
            category: legacy.category,
            severity,
            status: FindingStatus::Persistent,
            confidence_class: FindingConfidenceClass::Historical,
            path_or_scope: legacy.path,
            summary: legacy.summary,
            recommendation: None,
            evidence_refs: vec![format!("legacy-hades:{legacy_sha256}")],
            provider_id: None,
            source_command_id: None,
            prior_finding_id: None,
            review_required: true,
            mutation_allowed: false,
        });
    }
    let identity = serde_json::to_vec(&(
        legacy_path,
        legacy_sha256.as_str(),
        mapped_project_id,
        mapped_rumil_audit_id,
    ))?;
    Ok(LegacyHadesBaselineImport {
        provenance: LegacyHadesImport {
            import_id: Uuid::new_v5(&mapped_project_id, &identity),
            legacy_source: "hades".to_string(),
            legacy_contract: "hades.audit-findings.legacy".to_string(),
            legacy_path: legacy_path.to_string(),
            legacy_sha256,
            legacy_generated_at_utc,
            mapped_project_id,
            mapped_rumil_audit_id,
            mapping_quality: mapping_quality.to_string(),
            unmapped_fields: Vec::new(),
            historical_only: true,
            native_rumil_evidence: false,
        },
        baseline: AuditBaseline {
            audit_id: mapped_rumil_audit_id,
            project_id: mapped_project_id,
            source_revision: None,
            findings,
        },
    })
}

/// Build the bounded continuity projection eligible for a Vairë bridge. Raw
/// source excerpts and finding summaries are deliberately not copied.
pub fn build_memory_observation(
    report: &AuditReport,
    findings: &[Finding],
    comparison: Option<&Comparison>,
) -> MemoryObservation {
    let mut finding_counts = BTreeMap::new();
    for finding in findings {
        let key = severity_name(finding.severity).to_string();
        *finding_counts.entry(key).or_insert(0) += 1;
    }
    let mut receipt_refs = report.command_receipts.clone();
    receipt_refs.sort();
    receipt_refs.dedup();
    let finding_total: u64 = finding_counts.values().sum();

    MemoryObservation {
        observation_id: Uuid::new_v5(&report.project_id, report.audit_id.as_bytes()),
        source_audit_id: report.audit_id,
        project_id: report.project_id,
        source_revision: report.source_revision.clone(),
        summary: format!(
            "Rúmil {} audit: {finding_total} normalized findings across {} severity classes",
            completeness_name(report.completeness),
            finding_counts.len()
        ),
        completeness: completeness_name(report.completeness).to_string(),
        finding_counts,
        comparison_digest: comparison.map(|value| value.comparison_id.to_string()),
        receipt_refs,
        eligible_tags: vec!["rumil".to_string(), "project_audit".to_string()],
        retention_class: "bounded_receipt_metadata".to_string(),
        provenance: "arda.rumil.audit-report.v1".to_string(),
    }
}

fn severity_name(severity: FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Info => "info",
        FindingSeverity::Low => "low",
        FindingSeverity::Medium => "medium",
        FindingSeverity::High => "high",
        FindingSeverity::Critical => "critical",
    }
}

fn completeness_name(completeness: crate::contracts::AuditReportCompleteness) -> &'static str {
    use crate::contracts::AuditReportCompleteness;
    match completeness {
        AuditReportCompleteness::Complete => "complete",
        AuditReportCompleteness::Partial => "partial",
        AuditReportCompleteness::StructureOnly => "structure_only",
        AuditReportCompleteness::Failed => "failed",
        AuditReportCompleteness::NotRequested => "not_requested",
    }
}
