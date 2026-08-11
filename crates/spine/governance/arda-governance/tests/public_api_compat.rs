use arda_core::governance_gates::{
    ActionReversibility, CoercionRisk, ConsentAuthority, HumanFacingActionReview,
    HumanImpactReviewInput,
};
use arda_core::Task;
use arda_governance::{
    bacon_lite_validate, calculate_resonance_without_governance,
    default_governance_readiness_report, evaluate_governance_chain, evaluate_human_impact_review,
    evaluate_love_dynamics, interpret_alignment, load_philosopher_profiles_from_str,
    love_dynamics_compatibility_proxy, profile_joulework, triad_validate, AlignmentSignals,
    BaconLiteEvent, BaconLiteResult, GameTheory, GameTheorySelectionResult, GovernanceChainConfig,
    GovernanceChainResult, GovernanceEvidence, GovernanceEvidenceAssessment,
    GovernanceReadinessReport, JouleWorkProfile, LoveDynamicsInput, LoveDynamicsScore,
    LoveDynamicsTrend, LoveEquationScore, PhilosopherProfileStatusProjection, ResonanceScore,
    TriadPhilosopherVerdict, TriadResult,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

fn assert_contract<T>(contracts: &Value, name: &str, value: &T)
where
    T: Serialize + DeserializeOwned,
{
    let encoded = serde_json::to_value(value).expect("public type should serialize");
    let object = encoded
        .as_object()
        .expect("public result should be an object");
    let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
    let mut expected = contracts[name]
        .as_array()
        .expect("fixture entry should be an array")
        .iter()
        .map(|field| field.as_str().expect("fixture field should be a string"))
        .collect::<Vec<_>>();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "serialized field contract changed for {name}"
    );
    let _: T = serde_json::from_value(encoded).expect("serialized public type should round-trip");
}

#[test]
fn public_result_shapes_match_the_v1_compatibility_fixture() {
    fn assert_public_wire_type<T: Serialize + DeserializeOwned>() {}
    assert_public_wire_type::<BaconLiteEvent>();
    assert_public_wire_type::<BaconLiteResult>();
    assert_public_wire_type::<GameTheorySelectionResult>();
    assert_public_wire_type::<GovernanceChainResult>();
    assert_public_wire_type::<GovernanceEvidence>();
    assert_public_wire_type::<GovernanceEvidenceAssessment>();
    assert_public_wire_type::<GovernanceReadinessReport>();
    assert_public_wire_type::<HumanFacingActionReview>();
    assert_public_wire_type::<JouleWorkProfile>();
    assert_public_wire_type::<LoveDynamicsScore>();
    assert_public_wire_type::<LoveEquationScore>();
    assert_public_wire_type::<PhilosopherProfileStatusProjection>();
    assert_public_wire_type::<ResonanceScore>();
    assert_public_wire_type::<TriadPhilosopherVerdict>();
    assert_public_wire_type::<TriadResult>();

    let contracts: Value = serde_json::from_str(include_str!("fixtures/public_api_v1.json"))
        .expect("public API fixture should be valid JSON");
    let task = Task::new(
        "verify governance source evidence with a documented fallback",
        "governance",
    );

    let triad = triad_validate(&task, None);
    assert_contract(&contracts, "TriadResult", &triad);

    let bacon = bacon_lite_validate(&task);
    assert_contract(&contracts, "BaconLiteResult", &bacon);
    let event = BaconLiteEvent {
        policy_version: bacon.policy_version.clone(),
        scorer_version: bacon.triad.policy_version.clone(),
        review_mode: bacon.triad.review_mode,
        source_maturity: bacon.triad.profile_maturity.clone(),
        evidence_source: Some(bacon.triad.evidence.scoring_source),
        ts_utc: "2026-01-01T00:00:00Z".to_string(),
        crate_name: "fixture".to_string(),
        action: "verify".to_string(),
        task_id: task.id.to_string(),
        task_type: task.task_type.clone(),
        description: task.description.clone(),
        passed: bacon.passed,
        confidence: bacon.confidence,
        rationale: bacon.rationale.clone(),
        triad_passed: bacon.triad.passed,
        typed_veto: bacon.triad.veto.clone(),
        confidence_band: Default::default(),
        philosopher_evidence: None,
        aurelius_outcome: Some(bacon.triad.aurelius),
        bacon_outcome: Some(bacon.triad.bacon),
        sun_tzu_outcome: Some(bacon.triad.sun_tzu),
        aurelius_score: bacon.triad.aurelius_score,
        bacon_score: bacon.triad.bacon_score,
        sun_tzu_score: bacon.triad.sun_tzu_score,
        context: json!({"fixture": true}),
    };
    assert_contract(&contracts, "BaconLiteEvent", &event);

    let chain = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    assert_contract(&contracts, "GovernanceChainResult", &chain);
    assert_contract(
        &contracts,
        "ResonanceScore",
        &calculate_resonance_without_governance(&task, None, None),
    );
    assert_contract(&contracts, "JouleWorkProfile", &profile_joulework(&task));
    assert_contract(
        &contracts,
        "LoveEquationScore",
        &love_dynamics_compatibility_proxy(&task),
    );
    assert_contract(
        &contracts,
        "HumanFacingActionReview",
        &evaluate_human_impact_review(HumanImpactReviewInput {
            affected_parties: vec!["operator".to_string()],
            reversibility: ActionReversibility::Reversible,
            interruption_reason: Some("scheduled reminder".to_string()),
            consent_authority: ConsentAuthority::OperatorAuthored,
            uncertainty: 0.1,
            coercion_risk: CoercionRisk::Low,
        }),
    );

    let love = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: 0.5,
        cooperation: 0.8,
        defection: 0.2,
        beta: 0.5,
        delta_time: 1.0,
    });
    assert_contract(&contracts, "LoveDynamicsScore", &love);
    let philosopher = interpret_alignment(AlignmentSignals {
        love_trend: LoveDynamicsTrend::Growing,
        projected_empathy: 0.7,
        empirical_grounding: 0.8,
        independence: 0.8,
        sycophancy_risk: 0.1,
        joule_honesty: 0.9,
        joule_efficiency: 0.8,
        defection_pressure: 0.1,
    });
    assert_contract(&contracts, "TriadPhilosopherVerdict", &philosopher);

    let selection = GameTheory::new().select_agent_with_policy("governance");
    assert_contract(&contracts, "GameTheorySelectionResult", &selection);
    assert_contract(
        &contracts,
        "GovernanceReadinessReport",
        &default_governance_readiness_report(),
    );

    let profiles = load_philosopher_profiles_from_str(include_str!(
        "../../../../../config/governance/philosophers.toml"
    ))
    .expect("repository profile fixture should parse");
    let projection = profiles.status_projection("config/governance/philosophers.toml");
    assert_contract(
        &contracts,
        "PhilosopherProfileStatusProjection",
        &projection,
    );
}

#[test]
fn stable_enum_encodings_remain_unchanged() {
    assert_eq!(json!(arda_governance::GateOutcome::Pass), json!("Pass"));
    assert_eq!(json!(LoveDynamicsTrend::Growing), json!("Growing"));
    assert_eq!(
        json!(arda_governance::GovernanceReviewMode::HeuristicLocal),
        json!("heuristic_local")
    );
    assert_eq!(
        json!(arda_governance::GovernanceReadinessLevel::RuntimeReceipted),
        json!("runtime_receipted")
    );
}

#[test]
fn pre_evidence_triad_records_deserialize_with_a_safe_default_assessment() {
    let old_record = json!({
        "chain_id": "default_triad",
        "chain_version": "heuristic_local_v1",
        "profile_source": "config/governance/philosophers.toml",
        "review_mode": "heuristic_local",
        "profile_maturity": "draft_human_authored",
        "aurelius": "Pass",
        "bacon": "Conditional",
        "sun_tzu": "Pass",
        "aurelius_score": 0.8,
        "bacon_score": 0.4,
        "sun_tzu_score": 0.8,
        "passed": true,
        "veto_reason": null
    });

    let decoded: TriadResult = serde_json::from_value(old_record).expect("legacy Triad result");
    assert_eq!(
        decoded.evidence.grade,
        arda_governance::GovernanceEvidenceGrade::NoEvidence
    );
}
