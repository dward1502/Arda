use arda_core::{Task, TaskStatus};
use arda_governance::resonance::phi_harmonic_components;
use arda_governance::{
    assess_governance_evidence, calculate_resonance_without_governance, evaluate_governance_chain,
    GateOutcome, GovernanceChainConfig, GovernanceEvidenceGrade, GovernanceScoringSource,
};

fn bacon_outcome(task: &Task) -> (GateOutcome, f64) {
    let result = evaluate_governance_chain(task, &GovernanceChainConfig::default_triad());
    let bacon = result
        .lenses
        .iter()
        .find(|lens| lens.lens_id == "bacon")
        .expect("Bacon lens");
    (bacon.outcome, bacon.score)
}

#[test]
fn structured_source_evidence_clears_bacon_without_description_keywords() {
    let mut task = Task::new("process the record", "custom");
    task.result = Some(serde_json::json!({
        "governance_evidence": {
            "schema_version": "arda.governance.evidence.v1",
            "evidence_anchors": [{
                "kind": "source",
                "uri": "https://example.com/receipt/42",
                "claim": "the observed result is reproducible"
            }],
            "action_intent": "validate the observed result",
            "justified_urgency": null,
            "cooperation": 0.8,
            "defection": 0.1,
            "disconfirming_evidence": ["independent replay may fail"],
            "risk_boundary": "do not publish when replay fails",
            "fallback_path": "return to operator review"
        }
    }));

    let (outcome, score) = bacon_outcome(&task);
    assert_eq!(outcome, GateOutcome::Pass);
    assert!(score >= 0.75);
}

#[test]
fn keyword_stuffing_without_evidence_is_explicitly_penalized() {
    let task = Task::new(
        "verify source evidence https://example.com because 2026 receipt proof data",
        "query",
    );

    let (outcome, score) = bacon_outcome(&task);
    assert_ne!(outcome, GateOutcome::Pass);
    assert!(score < 0.50);
}

#[test]
fn missing_phi_inputs_are_zero_weight_not_neutral_fifties() {
    let mut task = Task::new("degraded resonance", "governance");
    task.status = TaskStatus::Complete;

    let phi = phi_harmonic_components(&task);
    assert_eq!(phi.composite_score, 0.0);
    assert_eq!(phi.time_score, 0.0);
    assert_eq!(phi.resource_score, 0.0);
    assert_eq!(phi.question_score, 0.0);

    let score = calculate_resonance_without_governance(&task, None, None);
    let components = score.ecst_components.expect("resonance components");
    assert_eq!(components.phi_harmonic, 0.0);
    assert_eq!(components.phi_time_score, None);
    assert_eq!(components.phi_resource_score, None);
    assert_eq!(components.phi_question_score, None);
}

fn structured_result(action_intent: &str) -> serde_json::Value {
    serde_json::json!({
        "governance_evidence": {
            "schema_version": "arda.governance.evidence.v1",
            "evidence_anchors": [{
                "kind": "source",
                "uri": "urn:receipt:verified",
                "claim": "verified observation"
            }],
            "action_intent": action_intent,
            "cooperation": 0.8,
            "defection": 0.1,
            "disconfirming_evidence": ["independent replay could disagree"],
            "risk_boundary": "hold on replay mismatch",
            "fallback_path": "operator review"
        }
    })
}

#[test]
fn structured_evidence_grade_and_source_are_serialized() {
    let mut task = Task::new("procesar el registro", "custom");
    task.result = Some(structured_result("validar el resultado observado"));

    let result = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    assert_eq!(
        result.evidence.grade,
        GovernanceEvidenceGrade::StructuredValidated
    );
    assert_eq!(
        result.evidence.scoring_source,
        GovernanceScoringSource::StructuredEvidence
    );
    assert!(result.evidence.missing_fields.is_empty());
    assert_eq!(bacon_outcome(&task).0, GateOutcome::Pass);
}

#[test]
fn negated_safe_action_does_not_trigger_naive_contradiction() {
    let mut task = Task::new("安全な検証", "custom");
    task.result = Some(structured_result(
        "do not deploy until the independent result is verified",
    ));

    let result = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    let aurelius = result
        .lenses
        .iter()
        .find(|lens| lens.lens_id == "aurelius")
        .expect("Aurelius lens");
    assert_eq!(aurelius.outcome, GateOutcome::Pass);
}

#[test]
fn contradictory_structured_intent_is_not_graded_as_aurelius_pass() {
    let mut task = Task::new("localized description", "custom");
    task.result = Some(structured_result("always deploy and never deploy"));

    let result = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    let aurelius = result
        .lenses
        .iter()
        .find(|lens| lens.lens_id == "aurelius")
        .expect("Aurelius lens");
    assert_ne!(aurelius.outcome, GateOutcome::Pass);
}

#[test]
fn malformed_structured_payload_falls_back_without_non_finite_scores() {
    let mut task = Task::new("source evidence because https://example.com 2026", "query");
    task.result = Some(serde_json::json!({
        "governance_evidence": {
            "schema_version": "arda.governance.evidence.v1",
            "evidence_anchors": "not-an-array",
            "action_intent": 42,
            "cooperation": "high"
        }
    }));
    task.joule_cost_estimated = f64::NAN;
    task.joule_cost_actual = f64::INFINITY;

    let assessment = assess_governance_evidence(&task).assessment;
    assert_eq!(
        assessment.scoring_source,
        GovernanceScoringSource::MalformedStructuredFallback
    );
    let first = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    let second = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    for (left, right) in first.lenses.iter().zip(&second.lenses) {
        assert!(left.score.is_finite());
        assert!((0.0..=1.0).contains(&left.score));
        assert_eq!(left.score, right.score);
    }
}

#[test]
fn legacy_result_fields_map_to_partial_structured_evidence() {
    let mut task = Task::new("process record", "custom");
    task.result = Some(serde_json::json!({
        "evidence": ["urn:test:receipt"],
        "recommendation": "review result",
        "risk_boundary": "do not publish",
        "fallback_path": "operator review"
    }));

    let context = assess_governance_evidence(&task);
    assert_eq!(
        context.assessment.scoring_source,
        GovernanceScoringSource::LegacyResultMapping
    );
    assert_eq!(
        context.assessment.grade,
        GovernanceEvidenceGrade::StructuredPartial
    );
    assert_eq!(
        context
            .evidence
            .expect("mapped evidence")
            .evidence_anchors
            .len(),
        1
    );
}

#[test]
fn available_phi_signal_is_renormalized_instead_of_diluted_by_missing_dimensions() {
    let mut task = Task::new("measure one real signal", "governance");
    task.joule_cost_estimated = 1.618033988749895;
    task.joule_cost_actual = 1.0;

    let phi = phi_harmonic_components(&task);
    assert_eq!(phi.resource_score, 100.0);
    assert_eq!(phi.composite_score, 100.0);
    assert_eq!(phi.available_weight, 0.45);
    assert_eq!(phi.missing_inputs, vec!["timing", "clarifications"]);
}

#[test]
fn adding_validated_evidence_is_monotonic_for_each_default_lens() {
    let baseline = Task::new("process the record safely", "custom");
    let mut evidenced = baseline.clone();
    evidenced.result = Some(structured_result("process the record safely"));

    let baseline_result =
        evaluate_governance_chain(&baseline, &GovernanceChainConfig::default_triad());
    let evidenced_result =
        evaluate_governance_chain(&evidenced, &GovernanceChainConfig::default_triad());
    for (before, after) in baseline_result.lenses.iter().zip(&evidenced_result.lenses) {
        assert_eq!(before.lens_id, after.lens_id);
        assert!(
            after.score >= before.score,
            "validated evidence lowered {} from {} to {}",
            before.lens_id,
            before.score,
            after.score
        );
    }
}
