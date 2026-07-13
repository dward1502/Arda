use annunimas_oracle::{DefaultTruthScorer, TruthScorer};
use annunimas_warden::{DefaultGateScorer, GateScorer};

#[test]
fn test_truth_scoring() {
    let scorer = DefaultTruthScorer::new();

    let proposal1 = "This proposal is based on confirmed facts and high confidence data";
    let result1 = scorer.score_truth_confidence(proposal1);

    assert_eq!(result1.confidence, 0.9);
    assert!(result1.reason.contains("High confidence"));

    let proposal2 = "This proposal is uncertain and based on limited information";
    let result2 = scorer.score_truth_confidence(proposal2);

    assert_eq!(result2.confidence, 0.3);
    assert!(result2.reason.contains("Low confidence"));

    let proposal3 = "This is a general proposal with no specific indicators";
    let result3 = scorer.score_truth_confidence(proposal3);

    assert_eq!(result3.confidence, 0.6);
    assert!(result3.reason.contains("Moderate confidence"));
}

#[test]
fn test_gate_scoring() {
    let scorer = DefaultGateScorer::new();

    // Test a safe proposal
    let proposal1 = "This is a safe proposal with low risk and bounded evidence";
    let verdict1 = scorer.score_gate(proposal1);

    assert_eq!(verdict1.truth_confidence, 0.6); // Moderate confidence
    assert_eq!(verdict1.operational_risk, 0.2); // Low risk
    assert_eq!(verdict1.autonomy_readiness, 0.5); // Moderate autonomy
    assert_eq!(verdict1.gated, false); // Should not be gated

    // Test a high-risk proposal
    let proposal2 = "This is a destructive proposal with high operational risk";
    let verdict2 = scorer.score_gate(proposal2);

    assert_eq!(verdict2.operational_risk, 0.9); // High risk
    assert_eq!(verdict2.gated, true); // Should be gated
    assert!(verdict2.gating_reason.is_some());

    // Test a low-readiness proposal
    let proposal3 = "This is an autonomous proposal that requires HADES approval";
    let verdict3 = scorer.score_gate(proposal3);

    assert_eq!(verdict3.autonomy_readiness, 0.8); // High autonomy
                                                  // Note: This might not be gated since it's high autonomy readiness
}
