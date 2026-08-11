use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::baseline::AuditBaseline;
use crate::contracts::{Comparison, Finding, FindingStatus, RevisionRelation};
use crate::error::{Result, RumilError};

/// Compare two selected baselines for the same canonical project identity.
/// Input order cannot affect the result or comparison ID.
pub fn compare_baselines(
    current: &AuditBaseline,
    prior: &AuditBaseline,
    revision_relation: RevisionRelation,
) -> Result<Comparison> {
    if current.project_id != prior.project_id {
        return Err(RumilError::InvalidRequest(
            "cannot compare baselines from different project identities".to_string(),
        ));
    }
    let current_by_id = index_findings(&current.findings, "current")?;
    let prior_by_id = index_findings(&prior.findings, "prior")?;
    let mut new_findings = Vec::new();
    let mut persistent_findings = Vec::new();
    let mut changed_findings = Vec::new();
    let mut resolved_findings = Vec::new();
    let mut stale_findings = Vec::new();
    let mut unverifiable_findings = Vec::new();

    for (id, finding) in &current_by_id {
        match finding.status {
            FindingStatus::Stale => stale_findings.push(*id),
            FindingStatus::Unverifiable => unverifiable_findings.push(*id),
            _ => match prior_by_id.get(id) {
                None => new_findings.push(*id),
                Some(prior_finding)
                    if material_fingerprint(finding)? == material_fingerprint(prior_finding)? =>
                {
                    persistent_findings.push(*id);
                }
                Some(_) => changed_findings.push(*id),
            },
        }
    }
    let current_ids: BTreeSet<_> = current_by_id.keys().copied().collect();
    for id in prior_by_id.keys() {
        if !current_ids.contains(id) {
            resolved_findings.push(*id);
        }
    }

    let comparison_identity = serde_json::to_vec(&(
        current.audit_id,
        prior.audit_id,
        &revision_relation,
        &new_findings,
        &persistent_findings,
        &changed_findings,
        &resolved_findings,
        &stale_findings,
        &unverifiable_findings,
    ))?;

    Ok(Comparison {
        comparison_id: Uuid::new_v5(&current.project_id, &comparison_identity),
        current_audit_id: current.audit_id,
        prior_audit_id: prior.audit_id,
        identity_match: true,
        revision_relation,
        new_findings,
        persistent_findings,
        changed_findings,
        resolved_findings,
        stale_findings,
        unverifiable_findings,
        baseline_warnings: Vec::new(),
    })
}

fn index_findings<'a>(findings: &'a [Finding], label: &str) -> Result<BTreeMap<Uuid, &'a Finding>> {
    let mut by_id = BTreeMap::new();
    for finding in findings {
        if by_id.insert(finding.finding_id, finding).is_some() {
            return Err(RumilError::InvalidRequest(format!(
                "duplicate finding ID in {label} baseline: {}",
                finding.finding_id
            )));
        }
    }
    Ok(by_id)
}

fn material_fingerprint(finding: &Finding) -> Result<Vec<u8>> {
    let mut evidence_refs = finding.evidence_refs.clone();
    evidence_refs.sort();
    evidence_refs.dedup();
    serde_json::to_vec(&(
        &finding.category,
        &finding.severity,
        &finding.confidence_class,
        &finding.path_or_scope,
        &finding.summary,
        &finding.recommendation,
        evidence_refs,
        &finding.provider_id,
        finding.review_required,
        finding.mutation_allowed,
    ))
    .map_err(Into::into)
}
