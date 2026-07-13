use annunimas_signal_grid::contract::contract;
use annunimas_signal_grid::pipeline::{CommentSignal, CrisisTier, RouteDecision, SignalGridPolicy};
use annunimas_signal_grid::service::{
    crisis_tier, plan_route, route_signal, status, validate_governance_baseline,
};

#[test]
fn sovereign_baseline_contract_is_present() {
    let base = contract();
    assert!(base.governance.triad_required);
    assert!(base.governance.bacon_lite_required);
    assert!(base.governance.joulework_required);
    assert!(base.governance.love_equation_required);
    assert!(base.continuity.task_ledger_linked);
    assert!(base.continuity.memory_checkpoint_expected);
    assert_eq!(
        base.state_export_path,
        "core/state/annunimas-signal-grid.json"
    );
    assert!(validate_governance_baseline(base.governance).ready);
}

#[test]
fn service_status_reports_governance_ready() {
    let report = status();
    assert!(report.governance_ready);
}

#[test]
fn negative_sentiment_pauses_and_alerts() {
    assert_eq!(
        route_signal(CommentSignal::NegativeSentiment),
        RouteDecision::PauseAndAlertHuman
    );
    assert_eq!(crisis_tier(true, false), CrisisTier::Pause);
}

#[test]
fn legal_threat_route_plan_requires_full_stop_review() {
    let plan = plan_route(
        CommentSignal::ProductFeedback,
        SignalGridPolicy {
            negative_spike: false,
            legal_threat: true,
            export_state: true,
        },
    );

    assert_eq!(plan.decision, RouteDecision::RouteToProductOrOperations);
    assert_eq!(plan.crisis_tier, CrisisTier::FullStop);
    assert!(plan.human_review_required);
    assert_eq!(plan.governance_validators.len(), 5);
}
