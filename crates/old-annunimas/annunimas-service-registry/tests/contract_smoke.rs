use annunimas_service_registry::contract::contract;
use annunimas_service_registry::registry::{ServiceRecord, ServiceRegistry};
use annunimas_service_registry::service::status;

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
        "core/state/annunimas-service-registry.json"
    );
}

#[test]
fn service_status_reports_governance_ready() {
    let report = status();
    assert!(report.governance_ready);
}

#[test]
fn startup_order_sorts_dependencies() {
    let registry = ServiceRegistry {
        services: vec![
            ServiceRecord {
                service_id: "registry".into(),
                dependencies: vec![],
                startup_order: 1,
            },
            ServiceRecord {
                service_id: "daemon".into(),
                dependencies: vec!["registry".into()],
                startup_order: 2,
            },
        ],
    };
    let order = registry.startup_order().expect("order");
    assert_eq!(order, vec!["registry".to_string(), "daemon".to_string()]);
}
