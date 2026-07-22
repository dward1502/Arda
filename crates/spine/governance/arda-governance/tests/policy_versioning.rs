use arda_core::{Task, TaskStatus};
use arda_governance::{
    bacon_lite_validate, calculate_resonance_without_governance, evaluate_governance_chain,
    game_theory_score, load_governance_chain_from_str, GameTheoryConfidenceBand,
    GovernanceChainConfig, GovernanceVetoCode,
};
use serde_json::Value;

fn failed_task() -> Task {
    Task::new(
        "always deploy and never deploy without evidence or fallback",
        "custom",
    )
}

#[test]
fn every_policy_receipt_serializes_its_semantics_version() {
    let task = failed_task();
    let chain = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
    assert_eq!(chain.chain_version, "structured_evidence_v2");
    let triad = arda_governance::triad_validate(&task, None);
    let resonance = calculate_resonance_without_governance(&task, None, None);
    let bacon = bacon_lite_validate(&task);
    let selection = arda_governance::GameTheory::new().select_agent_with_policy("custom");

    let receipts = [
        serde_json::to_value(chain).expect("chain"),
        serde_json::to_value(triad).expect("triad"),
        serde_json::to_value(resonance).expect("resonance"),
        serde_json::to_value(bacon).expect("bacon"),
        serde_json::to_value(selection).expect("selection"),
    ];
    for receipt in receipts {
        let version = receipt
            .get("policy_version")
            .or_else(|| receipt.get("selection_policy_version"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(!version.is_empty(), "missing policy version in {receipt}");
    }
}

#[test]
fn failed_chain_serializes_typed_veto_and_compatibility_renderer() {
    let result = evaluate_governance_chain(&failed_task(), &GovernanceChainConfig::default_triad());
    let veto = result.veto.as_ref().expect("typed veto");
    assert_eq!(veto.code, GovernanceVetoCode::GateFailed);
    assert!(!veto.failed_gates.is_empty());
    assert_eq!(veto.required_passes, result.required_passes);
    let encoded = serde_json::to_value(result).expect("chain result");

    assert_eq!(encoded["veto"]["code"], "gate_failed");
    assert!(encoded["veto"]["failed_gates"]
        .as_array()
        .is_some_and(|gates| !gates.is_empty()));
    assert!(encoded["veto_reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("FAIL")));
}

#[test]
fn fallback_selection_serializes_no_data_confidence_band() {
    let selection = arda_governance::GameTheory::new().select_agent_with_policy("custom");
    let encoded = serde_json::to_value(selection).expect("selection");

    assert_eq!(encoded["confidence"], 0.0);
    assert_eq!(encoded["confidence_band"], "no_data");
}

#[test]
fn game_theory_score_uses_unit_interval_internally() {
    let mut task = Task::new("evaluate normalized policy", "governance");
    task.status = TaskStatus::Complete;
    task.joule_cost_estimated = 1.0;
    task.joule_cost_actual = 1.0;

    let score = game_theory_score(&task);
    assert!((0.0..=1.0).contains(&score), "score was {score}");
}

#[test]
fn unsupported_future_chain_semantics_are_rejected() {
    let raw = r#"
schema_version = "arda.governance.chains.v1"
chain_id = "future_chain"
chain_version = "structured_evidence_v999"
profile_source = "fixture"
review_mode = "heuristic_local"
profile_maturity = "fixture"
strict = false
required_passes = 1
autonomous_blocking_enabled = false

[[lenses]]
id = "bacon"
display_name = "Francis Bacon"
pass_threshold = 0.5
"#;

    let error = load_governance_chain_from_str(raw).expect_err("future version must fail closed");
    assert!(error.to_string().contains("chain_version"));
}

#[test]
fn legacy_receipts_downgrade_missing_semantic_fields_to_named_v1_defaults() {
    let resonance: arda_governance::ResonanceScore = serde_json::from_value(serde_json::json!({
        "value": 42.0,
        "ecst_components": null
    }))
    .expect("legacy resonance receipt");
    assert_eq!(resonance.policy_version, "ecst_compatibility_v1");

    let love: arda_governance::LoveEquationScore = serde_json::from_value(serde_json::json!({
        "score": 0.5,
        "impact": 0.6,
        "reach": 0.7,
        "energy": 1.0,
        "time": 2.0
    }))
    .expect("legacy Love proxy receipt");
    assert_eq!(love.policy_version, "love_proxy_v1");

    let joule: arda_governance::JouleWorkProfile = serde_json::from_value(serde_json::json!({
        "estimated": 1.0,
        "actual": 1.0,
        "variance": 0.0,
        "honesty_ratio": 1.0,
        "measurement_source": "default_fallback",
        "measurement_confidence": 0.0,
        "observed_measurement": false,
        "autonomy_truth_allowed": false,
        "efficient": true
    }))
    .expect("legacy JouleWork receipt");
    assert_eq!(joule.policy_version, "joulework_v1");

    let selection: arda_governance::GameTheorySelectionResult =
        serde_json::from_value(serde_json::json!({
            "selected_agent": null,
            "policy": {
                "kind": "Fallback",
                "label": "legacy",
                "autonomous_consensus": false
            },
            "candidate_count": 0,
            "filtered_out_count": 0,
            "fallback_reason": "no_candidates",
            "confidence": 0.0
        }))
        .expect("legacy game-theory receipt");
    assert_eq!(selection.confidence_band, GameTheoryConfidenceBand::NoData);
    assert_eq!(
        selection.selection_policy_version,
        "capability_weighted_local_v1"
    );
}
