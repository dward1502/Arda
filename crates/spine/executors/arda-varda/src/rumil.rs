use arda_rumil::{RumilEvidenceClass, RumilEvidenceReference};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RumilEvaluationDisposition {
    AcceptedAdvisory,
    ReviewRequired,
    Rejected,
}

/// Varda receipt for bounded Rúmil evidence. It is intentionally unable to
/// authorize execution or carry a proposal/approval payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RumilEvaluationReceipt {
    pub audit_id: Uuid,
    pub project_id: Uuid,
    pub packet_reference: String,
    pub packet_sha256: String,
    pub evidence_classes: Vec<RumilEvidenceClass>,
    pub disposition: RumilEvaluationDisposition,
    pub accepted_for_evaluation: bool,
    #[serde(default)]
    pub review_reasons: Vec<String>,
    pub authority: String,
    pub execution_authorized: bool,
}

pub fn evaluate_rumil_evidence(reference: &RumilEvidenceReference) -> RumilEvaluationReceipt {
    let authority_rejected =
        reference.authority != "advisory_read_only" || reference.execution_authorized;
    let mut review_reasons = Vec::new();
    if reference.classes.contains(&RumilEvidenceClass::Partial) {
        review_reasons.push("partial_coverage".to_string());
    }
    if reference.classes.contains(&RumilEvidenceClass::Unavailable)
        || !reference.missing_evidence.is_empty()
    {
        review_reasons.push("missing_evidence".to_string());
    }
    if reference.stale_baseline {
        review_reasons.push("stale_baseline".to_string());
    }
    if !reference.rejected_providers.is_empty() {
        review_reasons.push("rejected_provider".to_string());
    }
    review_reasons.sort();
    review_reasons.dedup();

    let complete_tool_backed =
        reference.classes.contains(&RumilEvidenceClass::ToolBacked) && review_reasons.is_empty();
    let disposition = if authority_rejected {
        RumilEvaluationDisposition::Rejected
    } else if complete_tool_backed {
        RumilEvaluationDisposition::AcceptedAdvisory
    } else {
        RumilEvaluationDisposition::ReviewRequired
    };

    RumilEvaluationReceipt {
        audit_id: reference.audit_id,
        project_id: reference.project_id,
        packet_reference: reference.packet_reference.clone(),
        packet_sha256: reference.packet_sha256.clone(),
        evidence_classes: reference.classes.clone(),
        disposition,
        accepted_for_evaluation: matches!(
            disposition,
            RumilEvaluationDisposition::AcceptedAdvisory
        ),
        review_reasons,
        authority: "advisory_evaluation_evidence".to_string(),
        execution_authorized: false,
    }
}
