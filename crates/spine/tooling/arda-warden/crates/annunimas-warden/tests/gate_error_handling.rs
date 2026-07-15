use arda_warden::{DefaultGateScorer, GateScorer, GateError, GateResult};

#[test]
fn test_gate_scoring_with_error_handling() {
    let gate_scorer = DefaultGateScorer::new();
    
    // Test with valid proposal
    let valid_proposal = "This is a safe and well-structured proposal";
    let result = gate_scorer.score_gate(valid_proposal);
    assert!(result.is_ok());
    
    // Test with malformed delta
    let malformed_proposal = "";
    let result = gate_scorer.score_gate(malformed_proposal);
    assert!(result.is_err());
    match result.unwrap_err() {
        GateError::MalformedDelta(_) => {},
        _ => panic!("Expected MalformedDelta error"),
    }
    
    // Test with high-risk proposal
    let high_risk_proposal = "This proposal contains dangerous content and high risk";
    let result = gate_scorer.score_gate(high_risk_proposal);
    assert!(result.is_err());
    match result.unwrap_err() {
        GateError::HighRiskProposal(_) => {},
        _ => panic!("Expected HighRiskProposal error"),
    }
    
    // Test with duplicate proposal
    let duplicate_proposal = "This is a duplicate proposal that already exists";
    let result = gate_scorer.score_gate(duplicate_proposal);
    assert!(result.is_err());
    match result.unwrap_err() {
        GateError::DuplicateProposal(_) => {},
        _ => panic!("Expected DuplicateProposal error"),
    }
}

#[test]
fn test_proposal_validation() {
    let gate_scorer = DefaultGateScorer::new();
    
    // Test valid proposal
    let valid_proposal = "This is a safe and well-structured proposal";
    let result = gate_scorer.validate_proposal(valid_proposal);
    assert!(result.is_ok());
    
    // Test empty proposal
    let empty_proposal = "";
    let result = gate_scorer.validate_proposal(empty_proposal);
    assert!(result.is_err());
    
    // Test proposal with invalid values
    let invalid_proposal = "This proposal contains null and undefined values";
    let result = gate_scorer.validate_proposal(invalid_proposal);
    assert!(result.is_err());
}