use arda_core::{Task, TaskStatus};
use arda_governance::{
    assess_empirical_distrust, assess_nonconformist_bee, calculate_resonance_without_governance,
    evaluate_love_dynamics, interpret_alignment, load_philosopher_profiles_from_str,
    love_dynamics_compatibility_proxy, AlignmentSignals, CompactPhilosopherEvidence,
    EmpiricalDistrustVerdict, GovernanceReviewMode, LoveDynamicsInput, LoveDynamicsTrend,
    NonconformistBeeVerdict, PhilosopherAction, PhilosopherInfluencePolicy,
    PhilosopherProfileSourceKind,
};

#[test]
fn love_proxy_is_explicitly_not_canonical_love_dynamics() {
    let task = Task::new("evaluate task value", "governance");
    let proxy = love_dynamics_compatibility_proxy(&task);

    assert_eq!(proxy.semantic, "task_value_proxy");
    assert!(proxy.source.contains("not_canonical_love_dynamics"));
}

#[test]
fn philosopher_verdict_is_golden_separate_metadata_not_a_resonance_weight() {
    let task = Task::new("rubber stamp without evidence", "review");
    let score = calculate_resonance_without_governance(&task, None, None);

    assert_eq!(
        score.philosopher_influence,
        PhilosopherInfluencePolicy::SeparateDecisionMetadata
    );
    assert_eq!(
        score
            .triad_philosopher
            .as_ref()
            .expect("philosopher metadata")
            .action,
        PhilosopherAction::Hold
    );
    assert!((score.value - 46.111_111_111_111_114).abs() < 1.0e-9);
}

#[test]
fn nonconformist_bee_independently_detects_sycophancy() {
    let task = Task::new("just approve and rubber stamp without evidence", "review");
    let assessment = assess_nonconformist_bee(&task);

    assert_eq!(assessment.verdict, NonconformistBeeVerdict::SycophancyRisk);
    assert!(assessment.sycophancy_risk >= 0.70);
    assert!(assessment.independence < 0.50);
}

#[test]
fn empirical_distrust_independently_requires_receipted_evidence() {
    let unsupported = Task::new("declare this proven", "review");
    let unsupported_assessment = assess_empirical_distrust(&unsupported);
    assert_eq!(
        unsupported_assessment.verdict,
        EmpiricalDistrustVerdict::Unsupported
    );

    let mut grounded = Task::new("verify the governed action", "review");
    grounded.result = Some(serde_json::json!({
        "governance_evidence": {
            "schema_version": "arda.governance.evidence.v1",
            "evidence_anchors": [{"kind": "command", "uri": "cargo test -p arda-governance"}],
            "action_intent": "verify the governed action",
            "cooperation": 0.8,
            "defection": 0.1,
            "disconfirming_evidence": ["focused test failure"],
            "risk_boundary": "no external side effects",
            "fallback_path": "hold for operator review"
        }
    }));
    let grounded_assessment = assess_empirical_distrust(&grounded);
    assert_eq!(
        grounded_assessment.verdict,
        EmpiricalDistrustVerdict::Grounded
    );
    assert!(grounded_assessment.empirical_grounding >= 0.75);
}

#[test]
fn profile_lifecycle_receipt_discloses_source_review_and_promotion_boundary() {
    let profiles = load_philosopher_profiles_from_str(include_str!(
        "../../../../../config/governance/philosophers.toml"
    ))
    .expect("repository profiles");
    let receipt = profiles
        .profile("bacon")
        .expect("Bacon profile")
        .lifecycle_receipt(
            "config/governance/philosophers.toml",
            GovernanceReviewMode::HeuristicLocal,
        );

    assert_eq!(
        receipt.source_kind,
        PhilosopherProfileSourceKind::HumanAuthored
    );
    assert_eq!(
        receipt.profile_source,
        "config/governance/philosophers.toml"
    );
    assert_eq!(receipt.source_revision, "arda-governance-phase7-v1");
    assert!(receipt.generated_artifact.is_none());
    assert_eq!(receipt.review_authority, "human_governance_maintainers");
    assert!(!receipt.promotion_criteria.is_empty());
    assert_eq!(receipt.review_mode, GovernanceReviewMode::HeuristicLocal);

    let verdict = interpret_alignment(AlignmentSignals {
        love_trend: LoveDynamicsTrend::Stable,
        projected_empathy: 0.70,
        empirical_grounding: 0.80,
        independence: 0.80,
        sycophancy_risk: 0.10,
        joule_honesty: 0.90,
        joule_efficiency: 0.90,
        defection_pressure: 0.10,
    });
    let operator = CompactPhilosopherEvidence::from(verdict);
    assert_eq!(
        operator.lifecycle.profile_source,
        "built_in:triad_philosopher"
    );
    assert_eq!(
        operator.lifecycle.review_mode,
        GovernanceReviewMode::HeuristicLocal
    );
    assert_eq!(
        operator.lifecycle.review_authority,
        "human_governance_maintainers"
    );
}

#[test]
fn arbitration_reconciles_conflicting_independence_evidence_love_and_cost_signals() {
    let sycophantic = interpret_alignment(AlignmentSignals {
        love_trend: LoveDynamicsTrend::Growing,
        projected_empathy: 0.90,
        empirical_grounding: 0.90,
        independence: 0.10,
        sycophancy_risk: 0.90,
        joule_honesty: 0.95,
        joule_efficiency: 0.95,
        defection_pressure: 0.10,
    });
    assert_eq!(sycophantic.action, PhilosopherAction::Revise);

    let costly_truth = interpret_alignment(AlignmentSignals {
        love_trend: LoveDynamicsTrend::Growing,
        projected_empathy: 0.85,
        empirical_grounding: 0.90,
        independence: 0.90,
        sycophancy_risk: 0.05,
        joule_honesty: 0.70,
        joule_efficiency: 0.20,
        defection_pressure: 0.10,
    });
    assert_eq!(costly_truth.action, PhilosopherAction::Proceed);

    let cooperative = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: 0.60,
        cooperation: 0.80,
        defection: 0.20,
        beta: 0.50,
        delta_time: 1.0,
    });
    let defecting = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: 0.60,
        cooperation: 0.20,
        defection: 0.80,
        beta: 0.50,
        delta_time: 1.0,
    });
    assert_eq!(cooperative.trend, LoveDynamicsTrend::Growing);
    assert_eq!(defecting.trend, LoveDynamicsTrend::Decaying);

    let mut failed = Task::new("independent review with evidence", "review");
    failed.status = TaskStatus::Failed {
        reason: "conflicting recommendations".to_string(),
    };
    let bee = assess_nonconformist_bee(&failed);
    let distrust = assess_empirical_distrust(&failed);
    assert!(bee.independence > distrust.empirical_grounding);
}
