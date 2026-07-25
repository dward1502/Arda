//! Independent empirical-grounding and falsifiability assessment.
//!
//! Empirical Distrust consumes the versioned governance-evidence contract and
//! remains advisory. It does not treat confidence or keywords as proof.

use arda_core::Task;
use serde::{Deserialize, Serialize};

use crate::{assess_governance_evidence, GovernanceEvidenceGrade, GovernanceScoringSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmpiricalDistrustVerdict {
    Grounded,
    ReviewRequired,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmpiricalDistrustAssessment {
    pub empirical_grounding: f64,
    pub verdict: EmpiricalDistrustVerdict,
    pub evidence_grade: GovernanceEvidenceGrade,
    pub scoring_source: GovernanceScoringSource,
    #[serde(default)]
    pub concerns: Vec<String>,
}

pub fn assess_empirical_distrust(task: &Task) -> EmpiricalDistrustAssessment {
    let context = assess_governance_evidence(task);
    let empirical_grounding = match context.assessment.grade {
        GovernanceEvidenceGrade::StructuredValidated => 1.0,
        GovernanceEvidenceGrade::StructuredPartial => 0.80,
        GovernanceEvidenceGrade::HeuristicOnly => 0.35,
        GovernanceEvidenceGrade::NoEvidence => 0.20,
    };
    let verdict = if empirical_grounding >= 0.75 {
        EmpiricalDistrustVerdict::Grounded
    } else if empirical_grounding >= 0.35 {
        EmpiricalDistrustVerdict::ReviewRequired
    } else {
        EmpiricalDistrustVerdict::Unsupported
    };
    let mut concerns = context.assessment.missing_fields.clone();
    concerns.extend(context.assessment.validation_errors.clone());
    if verdict == EmpiricalDistrustVerdict::Unsupported && concerns.is_empty() {
        concerns.push("no receipted evidence or falsification path is available".to_string());
    }

    EmpiricalDistrustAssessment {
        empirical_grounding,
        verdict,
        evidence_grade: context.assessment.grade,
        scoring_source: context.assessment.scoring_source,
        concerns,
    }
}
