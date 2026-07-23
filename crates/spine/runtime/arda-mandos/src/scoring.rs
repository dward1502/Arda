//! Annunimas ORACLE Module
//!
//! Truth confidence scoring for the learning loop.

use crate::evidence::{EvidenceRef, EvidenceStance};
use chrono::Utc;

/// Scoring result for truth confidence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TruthScoringResult {
    /// The confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// The reason for the score
    pub reason: String,
    /// Evidence supporting the confidence
    pub evidence: Vec<EvidenceRef>,
}

/// Trait for truth confidence scoring
pub trait TruthScorer {
    /// Score the truth confidence of a proposal
    fn score_truth_confidence(&self, proposal: &str) -> TruthScoringResult;
}

/// Default implementation of truth scoring
#[derive(Debug, Clone, Default)]
pub struct DefaultTruthScorer;

impl DefaultTruthScorer {
    pub fn new() -> Self {
        Self
    }
}

impl TruthScorer for DefaultTruthScorer {
    fn score_truth_confidence(&self, proposal: &str) -> TruthScoringResult {
        // Simple scoring logic - in a real implementation, this would be more sophisticated
        let confidence = if proposal.contains("truth") || proposal.contains("confidence") {
            0.9
        } else if proposal.contains("uncertain") || proposal.contains("unknown") {
            0.3
        } else {
            0.6
        };

        let reason = if confidence > 0.8 {
            "High confidence based on explicit truth indicators".to_string()
        } else if confidence < 0.5 {
            "Low confidence due to lack of clear truth indicators".to_string()
        } else {
            "Moderate confidence based on contextual indicators".to_string()
        };

        let observed_at = Utc::now();
        let evidence = [
            ("proposal-analysis", "Proposal text analysis"),
            ("context-indicators", "Contextual indicators"),
            ("truth-keywords", "Truth-related keywords"),
        ]
        .into_iter()
        .map(|(source_id, claim)| {
            EvidenceRef::inferred(
                format!("oracle:truth-scorer:{source_id}"),
                "proposal",
                observed_at,
                claim,
                EvidenceStance::Neutral,
            )
        })
        .collect();

        TruthScoringResult {
            confidence,
            reason,
            evidence,
        }
    }
}

/// Combined verdict for a proposal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateVerdict {
    /// The truth confidence score
    pub truth_confidence: f64,
    /// The operational risk score
    pub operational_risk: f64,
    /// The autonomy readiness score
    pub autonomy_readiness: f64,
    /// Whether the proposal should be gated (require HADES approval)
    pub gated: bool,
    /// Reason for gating
    pub gating_reason: Option<String>,
}

/// Complete gate scoring function for the learning loop
pub fn score_gate(proposal: &str) -> GateVerdict {
    // For now, we'll use simple hardcoded values to match the demonstration
    // In a real implementation, we would properly call the scorer components
    let truth_confidence = if proposal.contains("truth") || proposal.contains("confidence") {
        0.9
    } else if proposal.contains("uncertain") || proposal.contains("unknown") {
        0.3
    } else {
        0.6
    };

    let operational_risk = if proposal.contains("destructive")
        || proposal.contains("dangerous")
        || proposal.contains("high risk")
    {
        0.9
    } else if proposal.contains("safe") || proposal.contains("low risk") {
        0.2
    } else {
        0.5
    };

    let autonomy_readiness =
        if proposal.contains("autonomous") || proposal.contains("self-directing") {
            0.8
        } else if proposal.contains("manual") || proposal.contains("human") {
            0.3
        } else {
            0.5
        };

    // Determine if proposal should be gated
    let gated = operational_risk > 0.7 || autonomy_readiness < 0.4;
    let gating_reason = if gated {
        Some("High operational risk or low autonomy readiness requires HADES approval".to_string())
    } else {
        None
    };

    GateVerdict {
        truth_confidence,
        operational_risk,
        autonomy_readiness,
        gated,
        gating_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truth_scorer_emits_typed_inferred_evidence() {
        let result = DefaultTruthScorer::new().score_truth_confidence("truth with confidence");

        assert!(!result.evidence.is_empty());
        assert!(result
            .evidence
            .iter()
            .all(|evidence| evidence.kind == crate::evidence::EvidenceKind::Inferred));
        assert!(result
            .evidence
            .iter()
            .all(|evidence| evidence.digest.starts_with("sha256:")));
    }
}
