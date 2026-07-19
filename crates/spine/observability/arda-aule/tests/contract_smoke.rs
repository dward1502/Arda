use arda_aule::contract::contract;
use arda_aule::service::status;

#[test]
fn sovereign_baseline_contract_is_present() {
    let base = contract();
    assert!(base.governance.triad_required);
    assert!(base.governance.bacon_lite_required);
    assert!(base.governance.joulework_required);
    assert!(base.governance.love_equation_required);
    assert!(base.continuity.task_ledger_linked);
    assert!(base.continuity.memory_checkpoint_expected);
    assert_eq!(base.state_export_path, "core/state/arda-aule.json");
}

#[test]
fn service_status_reports_observability_ready() {
    let report = status();
    assert!(report.governance_ready);
}
