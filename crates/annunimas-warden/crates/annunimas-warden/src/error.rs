// sigil: REPAIR
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GateError {
    #[error("Malformed delta: {0}")]
    MalformedDelta(String),
    
    #[error("Expired evidence: {0}")]
    ExpiredEvidence(String),
    
    #[error("Missing surface: {0}")]
    MissingSurface(String),
    
    #[error("Duplicate proposal detected: {0}")]
    DuplicateProposal(String),
    
    #[error("High-risk proposal requires HADES approval: {0}")]
    HighRiskProposal(String),
    
    #[error("Gate scoring failed: {0}")]
    GateScoringFailed(String),
    
    #[error("Truth confidence too low: {0}")]
    LowTruthConfidence(String),
    
    #[error("Operational risk too high: {0}")]
    HighOperationalRisk(String),
    
    #[error("Autonomy readiness insufficient: {0}")]
    InsufficientAutonomyReadiness(String),
}

pub type GateResult<T> = std::result::Result<T, GateError>;