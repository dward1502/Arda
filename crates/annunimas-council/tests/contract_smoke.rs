use annunimas_council::contract::contract;
use annunimas_council::council::{CouncilQuery, CouncilSeat, QueryMode};
use annunimas_council::service::{build_brief, status};

#[test]
fn sovereign_baseline_contract_is_present() {
    let base = contract();
    assert!(base.governance.triad_required);
    assert!(base.governance.bacon_lite_required);
    assert!(base.governance.joulework_required);
    assert!(base.governance.love_equation_required);
    assert!(base.continuity.task_ledger_linked);
    assert!(base.continuity.memory_checkpoint_expected);
    assert_eq!(base.state_export_path, "core/state/annunimas-council.json");
}

#[test]
fn service_status_reports_governance_ready() {
    let report = status();
    assert!(report.governance_ready);
}

#[test]
fn legal_financial_seats_trigger_escalation() {
    let query = CouncilQuery {
        mode: QueryMode::DualSeat,
        seats: vec![CouncilSeat::Attorney, CouncilSeat::Strategist],
        prompt: "Review this agreement".into(),
    };
    let brief = build_brief(&query);
    assert!(brief.escalation_required);
}
