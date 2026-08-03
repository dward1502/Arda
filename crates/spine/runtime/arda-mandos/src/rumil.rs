use arda_rumil::{RumilEvidenceClass, RumilEvidenceReference};
use serde::{Deserialize, Serialize};

use crate::reasoning::OracleQueryError;

/// Mandos reasoning projection over a bounded Rúmil receipt. This carries no
/// project filesystem path and no source excerpt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MandosRumilEvidence {
    pub audit_id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub packet_reference: String,
    pub packet_sha256: String,
    pub classes: Vec<RumilEvidenceClass>,
    pub degraded: bool,
    #[serde(default)]
    pub degraded_reasons: Vec<String>,
    pub accepted_for_reasoning: bool,
    pub authority: String,
    pub execution_authorized: bool,
}

pub fn classify_rumil_evidence(
    reference: &RumilEvidenceReference,
) -> Result<MandosRumilEvidence, OracleQueryError> {
    if reference.authority != "advisory_read_only" || reference.execution_authorized {
        return Err(OracleQueryError::ReasoningContext {
            message: "Rúmil evidence must remain advisory and non-executable".to_string(),
        });
    }

    let mut degraded_reasons = Vec::new();
    if reference.classes.contains(&RumilEvidenceClass::Partial) {
        degraded_reasons.push("partial_coverage".to_string());
    }
    if reference.classes.contains(&RumilEvidenceClass::Unavailable) {
        degraded_reasons.push("missing_evidence".to_string());
    }
    if reference.stale_baseline {
        degraded_reasons.push("stale_baseline".to_string());
    }
    if !reference.rejected_providers.is_empty() {
        degraded_reasons.push("rejected_provider".to_string());
    }
    degraded_reasons.sort();
    degraded_reasons.dedup();
    let accepted_for_reasoning =
        reference.classes.contains(&RumilEvidenceClass::ToolBacked) && degraded_reasons.is_empty();

    Ok(MandosRumilEvidence {
        audit_id: reference.audit_id,
        project_id: reference.project_id,
        packet_reference: reference.packet_reference.clone(),
        packet_sha256: reference.packet_sha256.clone(),
        classes: reference.classes.clone(),
        degraded: !degraded_reasons.is_empty(),
        degraded_reasons,
        accepted_for_reasoning,
        authority: "advisory_reasoning_evidence".to_string(),
        execution_authorized: false,
    })
}
