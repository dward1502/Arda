use crate::error::{GateError, GateResult};
use crate::scoring::{AutonomyReadinessResult, OperationalRiskResult, TruthScoringResult};
use crate::scoring::{AutonomyReadinessScorer, DefaultAutonomyReadinessScorer, DefaultGateScorer, DefaultOperationalRiskScorer, OperationalRiskScorer, TruthScoringResult, TruthScorer};
use annunimas_oracle::{DefaultTruthScorer, TruthScorer};
use serde::{Deserialize, Serialize};

/// Combined verdict for a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Trait for operational risk scoring
pub trait OperationalRiskScorer {
    /// Score the operational risk of a proposal
    fn score_operational_risk(&self, proposal: &str) -> OperationalRiskResult;
}

/// Trait for autonomy readiness scoring
pub trait AutonomyReadinessScorer {
    /// Score the autonomy readiness of a proposal
    fn score_autonomy_readiness(&self, proposal: &str) -> AutonomyReadinessResult;
}

/// Trait for complete gate scoring
pub trait GateScorer {
    /// Score all aspects of a proposal for the gate
    fn score_gate(&self, proposal: &str) -> GateResult<GateVerdict>;
    
    /// Validate the proposal before scoring for potential errors
    fn validate_proposal(&self, proposal: &str) -> GateResult<()>;
}

/// Default implementation of operational risk scoring
#[derive(Debug, Clone)]
pub struct DefaultOperationalRiskScorer;

impl DefaultOperationalRiskScorer {
    pub fn new() -> Self {
        Self
    }
}

impl OperationalRiskScorer for DefaultOperationalRiskScorer {
    fn score_operational_risk(&self, proposal: &str) -> OperationalRiskResult {
        // Simple scoring logic - in a real implementation, this would be more sophisticated
        let risk = if proposal.contains("destructive")
            || proposal.contains("dangerous")
            || proposal.contains("high risk")
        {
            0.9
        } else if proposal.contains("safe") || proposal.contains("low risk") {
            0.2
        } else {
            0.5
        };

        let reason = if risk > 0.8 {
            "High operational risk due to destructive or dangerous content".to_string()
        } else if risk < 0.5 {
            "Low operational risk - safe or low-risk content".to_string()
        } else {
            "Moderate operational risk based on content indicators".to_string()
        };

        let evidence = vec![
            "Content analysis".to_string(),
            "Risk indicators".to_string(),
            "Contextual assessment".to_string(),
        ];

        OperationalRiskResult {
            risk,
            reason,
            evidence,
        }
    }
}

/// Default implementation of autonomy readiness scoring
#[derive(Debug, Clone)]
pub struct DefaultAutonomyReadinessScorer;

impl DefaultAutonomyReadinessScorer {
    pub fn new() -> Self {
        Self
    }
}

impl AutonomyReadinessScorer for DefaultAutonomyReadinessScorer {
    fn score_autonomy_readiness(&self, proposal: &str) -> AutonomyReadinessResult {
        // Simple scoring logic - in a real implementation, this would be more sophisticated
        let readiness = if proposal.contains("autonomous") || proposal.contains("self-directing") {
            0.8
        } else if proposal.contains("manual") || proposal.contains("human") {
            0.3
        } else {
            0.5
        };

        let reason = if readiness > 0.7 {
            "High autonomy readiness based on self-directing indicators".to_string()
        } else if readiness < 0.5 {
            "Low autonomy readiness - requires human intervention".to_string()
        } else {
            "Moderate autonomy readiness".to_string()
        };

        let evidence = vec![
            "Autonomy indicators".to_string(),
            "Self-directing signals".to_string(),
            "Contextual assessment".to_string(),
        ];

        AutonomyReadinessResult {
            readiness,
            reason,
            evidence,
        }
    }
}

/// Default implementation of gate scoring
#[derive(Debug, Clone)]
pub struct DefaultGateScorer;

impl DefaultGateScorer {
    pub fn new() -> Self {
        Self
    }
}

impl GateScorer for DefaultGateScorer {
    fn score_gate(&self, proposal: &str) -> GateResult<GateVerdict> {
        // Validate the proposal first to catch errors
        self.validate_proposal(proposal)?;
        
        let operational_scorer = DefaultOperationalRiskScorer::new();
        let autonomy_scorer = DefaultAutonomyReadinessScorer::new();
        let truth_scorer = DefaultTruthScorer::new();

        let operational_result = operational_scorer.score_operational_risk(proposal);
        let autonomy_result = autonomy_scorer.score_autonomy_readiness(proposal);
        let truth_result = truth_scorer.score_truth_confidence(proposal);

        // Determine if proposal should be gated
        let gated = operational_result.risk > 0.7 || autonomy_result.readiness < 0.4;
        let gating_reason = if gated {
            Some(
                "High operational risk or low autonomy readiness requires HADES approval"
                    .to_string(),
            )
        } else {
            None
        };

        Ok(GateVerdict {
            truth_confidence: truth_result.confidence,
            operational_risk: operational_result.risk,
            autonomy_readiness: autonomy_result.readiness,
            gated,
            gating_reason,
        })
    }
    
    fn validate_proposal(&self, proposal: &str) -> GateResult<()> {
        // Check for malformed deltas (empty or malformed input)
        if proposal.is_empty() {
            return Err(GateError::MalformedDelta("Proposal cannot be empty".to_string()));
        }
        
        // Check for common malformed indicators
        if proposal.contains("undefined") || proposal.contains("null") || proposal.contains("NaN") {
            return Err(GateError::MalformedDelta("Proposal contains invalid values".to_string()));
        }
        
        // Check for expired evidence indicators (simulated)
        if proposal.contains("expired") || proposal.contains("outdated") {
            return Err(GateError::ExpiredEvidence("Proposal references expired evidence".to_string()));
        }
        
        // Check for missing surfaces (simulated)
        if proposal.contains("missing") && proposal.contains("surface") {
            return Err(GateError::MissingSurface("Proposal references missing surface".to_string()));
        }
        
        // Check for duplicate proposals (simulated)
        if proposal.contains("duplicate") || proposal.contains("already exists") {
            return Err(GateError::DuplicateProposal("Proposal is a duplicate".to_string()));
        }
        
        // Check for high-risk content (simulated)
        if proposal.contains("high risk") || proposal.contains("dangerous") {
            return Err(GateError::HighRiskProposal("Proposal contains high-risk content".to_string()));
        }
        
        // Check for low truth confidence (simulated)
        if proposal.contains("uncertain") || proposal.contains("unknown") {
            return Err(GateError::LowTruthConfidence("Proposal has low truth confidence".to_string()));
        }
        
        // Check for high operational risk (simulated)
        if proposal.contains("destructive") {
            return Err(GateError::HighOperationalRisk("Proposal has destructive content".to_string()));
        }
        
        // Check for insufficient autonomy readiness (simulated)
        if proposal.contains("manual") || proposal.contains("human") {
            return Err(GateError::InsufficientAutonomyReadiness("Proposal has insufficient autonomy readiness".to_string()));
        }
        
        Ok(())
    }
}