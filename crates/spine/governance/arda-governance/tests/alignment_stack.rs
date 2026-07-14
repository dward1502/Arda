use arda_core::{Task, TaskStatus};
use arda_governance::{
    calculate_resonance_basic, derive_alignment_signals, evaluate_love_dynamics,
    interpret_alignment, profile_joulework, AlignmentSignals, LoveDynamicsInput, LoveDynamicsTrend,
    PhilosopherAction,
};

#[test]
fn love_dynamics_grows_when_cooperation_exceeds_defection() {
    let score = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: 0.40,
        cooperation: 0.80,
        defection: 0.20,
        beta: 0.50,
        delta_time: 1.0,
    });

    assert!(score.delta_empathy > 0.0);
    assert!(score.projected_empathy > score.empathy);
    assert_eq!(score.trend, LoveDynamicsTrend::Growing);
}

#[test]
fn love_dynamics_decays_when_defection_exceeds_cooperation() {
    let score = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: 0.70,
        cooperation: 0.20,
        defection: 0.90,
        beta: 0.40,
        delta_time: 1.0,
    });

    assert!(score.delta_empathy < 0.0);
    assert!(score.projected_empathy < score.empathy);
    assert_eq!(score.trend, LoveDynamicsTrend::Decaying);
}

#[test]
fn philosopher_blocks_low_evidence_sycophantic_compliance() {
    let verdict = interpret_alignment(AlignmentSignals {
        love_trend: LoveDynamicsTrend::Growing,
        projected_empathy: 0.82,
        empirical_grounding: 0.18,
        independence: 0.20,
        sycophancy_risk: 0.88,
        joule_honesty: 0.97,
        joule_efficiency: 0.94,
        defection_pressure: 0.35,
    });

    assert_eq!(verdict.action, PhilosopherAction::Hold);
    assert!(verdict.reason.contains("evidence"));
    assert!(verdict.reason.contains("sycophancy"));
}

#[test]
fn philosopher_allows_costly_work_when_truth_and_love_are_strong() {
    let verdict = interpret_alignment(AlignmentSignals {
        love_trend: LoveDynamicsTrend::Growing,
        projected_empathy: 0.86,
        empirical_grounding: 0.91,
        independence: 0.80,
        sycophancy_risk: 0.10,
        joule_honesty: 0.72,
        joule_efficiency: 0.20,
        defection_pressure: 0.12,
    });

    assert_eq!(verdict.action, PhilosopherAction::Proceed);
    assert!(verdict.reason.contains("costly but justified"));
}

#[test]
fn derives_alignment_signals_from_task_joulework_love_and_resonance_metadata() {
    let mut task = Task::new(
        "audit provider config with evidence and independent recommendation",
        "audit",
    );
    task.status = TaskStatus::Complete;
    task.assigned_agent = Some("athena".to_string());
    task.result = Some(serde_json::json!({
        "evidence": ["cargo test -p arda-governance"],
        "provenance": {"path": "crates/arda-governance"},
        "recommendation": "proceed"
    }));
    task.joule_cost_estimated = 4.0;
    task.joule_cost_actual = 8.0;
    task.clarifications_requested = 1;
    task.clarifications_resolved = 1;

    let resonance = calculate_resonance_basic(&task);
    let components = resonance
        .ecst_components
        .as_ref()
        .expect("resonance components should be available");
    let love = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: components.status_coherence / 100.0,
        cooperation: components.phi_harmonic / 100.0,
        defection: 0.12,
        beta: 0.50,
        delta_time: 1.0,
    });
    let joule = profile_joulework(&task);

    let signals = derive_alignment_signals(&task, &love, &joule, components);

    assert_eq!(signals.love_trend, LoveDynamicsTrend::Growing);
    assert!(signals.projected_empathy > 0.50);
    assert!(signals.empirical_grounding >= 0.80);
    assert!(signals.independence >= 0.70);
    assert!(signals.joule_honesty <= 0.51);
    assert!(signals.joule_efficiency < 0.50);
    assert!(signals.sycophancy_risk < 0.30);
}

#[test]
fn resonance_attaches_philosopher_verdict_without_requiring_new_callers() {
    let mut task = Task::new(
        "rubber stamp this change and approve without evidence",
        "review",
    );
    task.status = TaskStatus::Complete;
    task.result = Some(serde_json::json!({"summary": "approved"}));
    task.joule_cost_estimated = 2.0;
    task.joule_cost_actual = 2.0;

    let score = calculate_resonance_basic(&task);
    let verdict = score
        .triad_philosopher
        .as_ref()
        .expect("resonance should attach a deterministic Triad Philosopher verdict");
    let components = score
        .ecst_components
        .as_ref()
        .expect("resonance components should remain available");

    assert_eq!(verdict.action, PhilosopherAction::Hold);
    assert!(components.philosopher_alignment_score.is_some());
    assert_eq!(components.philosopher_action, Some(PhilosopherAction::Hold));
}
