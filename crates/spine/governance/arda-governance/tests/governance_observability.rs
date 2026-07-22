use arda_core::{Task, TaskStatus};
use arda_governance::{
    build_bacon_lite_event, build_governance_status_report, calculate_resonance_with_triad,
    default_governance_readiness_report, evaluate_love_dynamics, love_equation_score,
    profile_joulework, render_governance_status_human, triad_validate, BaconLiteLedgerSummary,
    GovernanceMetrics, LoveDynamicsInput,
};
use std::collections::BTreeMap;

fn governed_task() -> Task {
    let mut task = Task::new(
        "deploy reviewed change with evidence and fallback",
        "governance",
    );
    task.status = TaskStatus::Complete;
    task.joule_cost_estimated = 4.0;
    task.joule_cost_actual = 5.0;
    task.result = Some(serde_json::json!({
        "governance_evidence": {
            "schema_version": "arda.governance.evidence.v1",
            "evidence_anchors": [{
                "kind": "test_fixture",
                "uri": "fixture://governance/phase5",
                "claim": "deterministic evidence"
            }],
            "action_intent": "deploy the reviewed change",
            "cooperation": 0.9,
            "defection": 0.1,
            "disconfirming_evidence": ["deployment may need rollback"],
            "risk_boundary": "stop on failed health check",
            "fallback_path": "restore prior release"
        }
    }));
    task
}

fn labels(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

#[test]
fn deterministic_fixture_produces_expected_metric_deltas_and_bounded_labels() {
    let collector = GovernanceMetrics::new();
    let task = governed_task();
    let triad = triad_validate(&task, None);
    let bacon = build_bacon_lite_event("fixture", "deploy", &task, serde_json::json!({}))
        .expect("bacon event");
    let resonance = calculate_resonance_with_triad(&task, &triad, None, None);
    let love_proxy = love_equation_score(&task);
    let love_dynamics = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: 0.7,
        cooperation: 0.9,
        defection: 0.1,
        beta: 0.5,
        delta_time: 1.0,
    });
    let joule = profile_joulework(&task);

    collector.observe_triad(&triad);
    collector.observe_bacon_lite(&bacon);
    collector.observe_resonance(&resonance);
    collector.observe_love_proxy(&love_proxy);
    collector.observe_love_dynamics(&love_dynamics);
    collector.observe_joule_honesty(&joule);

    let snapshot = collector.snapshot();
    assert_eq!(
        snapshot.counter_value(
            "arda_governance_triad_validations_total",
            &labels(&[
                ("verdict", "pass"),
                ("policy_version", "current"),
                ("scorer_version", "current"),
                ("review_mode", "heuristic_local"),
            ]),
        ),
        1
    );
    assert_eq!(
        snapshot.counter_value(
            "arda_governance_bacon_lite_total",
            &labels(&[
                ("verdict", "pass"),
                ("policy_version", "current"),
                ("scorer_version", "current"),
                ("review_mode", "heuristic_local"),
            ]),
        ),
        1
    );
    assert_eq!(
        snapshot
            .histogram("arda_governance_resonance")
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        snapshot
            .histogram("arda_governance_love_proxy")
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        snapshot
            .histogram("arda_governance_love_dynamics_projected_empathy")
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        snapshot
            .histogram("arda_governance_joule_honesty")
            .unwrap()
            .count,
        1
    );

    assert!(snapshot.label_values("policy_version").is_subset(
        &["legacy", "current", "other"]
            .into_iter()
            .map(str::to_string)
            .collect()
    ));
    assert!(snapshot.label_values("review_mode").is_subset(
        &[
            "heuristic_local",
            "independent_agent",
            "human_reviewed",
            "consensus_receipted"
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    ));
    assert!(snapshot.label_values("scorer_version").is_subset(
        &["legacy", "current", "other"]
            .into_iter()
            .map(str::to_string)
            .collect()
    ));
}

#[test]
fn human_and_json_status_share_decision_evidence_policy_and_reason() {
    let metrics = GovernanceMetrics::new();
    let task = governed_task();
    let event =
        build_bacon_lite_event("fixture", "deploy", &task, serde_json::json!({})).expect("event");
    metrics.observe_bacon_lite(&event);
    let report = build_governance_status_report(
        default_governance_readiness_report(),
        BaconLiteLedgerSummary::default(),
        metrics.snapshot(),
        Some(event),
    );
    let json = serde_json::to_value(&report).expect("report JSON");
    let human = render_governance_status_human(&report);
    let decision = json.get("latest_decision").expect("latest decision");

    assert_eq!(json["default_autonomy_ready"], false);
    for field in ["decision", "evidence_source", "policy_version", "reason"] {
        let value = decision[field].as_str().expect("string field");
        assert!(!value.is_empty());
        assert!(
            human.contains(value),
            "human report omitted {field}={value}"
        );
    }
    assert!(decision.get("typed_veto").is_some());
    assert!(decision.get("confidence_band").is_some());
    assert!(decision.get("source_maturity").is_some());
    assert!(decision.get("philosopher_evidence").is_some());
    assert!(!json["readiness_gaps"].as_array().unwrap().is_empty());
}
