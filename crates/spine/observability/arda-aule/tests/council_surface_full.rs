#![cfg(feature = "full-cli")]
use arda_aule::council::{CouncilBrief, CouncilQuery, CouncilSeat, QueryMode};
use arda_aule::render_governance_prometheus;
use arda_governance::{GovernanceCounterSnapshot, GovernanceMetricsSnapshot};
use std::collections::BTreeMap;

#[test]
fn council_root_module_resolves_under_full_cli() {
    assert_eq!(CouncilSeat::Strategist as u8, CouncilSeat::Strategist as u8);
    assert_eq!(QueryMode::SingleSeat as u8, QueryMode::SingleSeat as u8);
}

#[test]
fn full_council_brief_remains_available_under_full_cli() {
    let query = CouncilQuery {
        mode: QueryMode::FullCouncil,
        seats: vec![CouncilSeat::Operator],
        prompt: "Review a material governance decision".to_string(),
    };

    let brief = CouncilBrief::from_query(&query);

    assert_eq!(brief.participating_seats.len(), 7);
    assert!(brief.participating_seats.contains(&CouncilSeat::Attorney));
    assert!(brief.escalation_required);
}

#[test]
fn governance_metrics_render_under_full_cli() {
    let snapshot = GovernanceMetricsSnapshot {
        collection_mode: "aule_full_cli_test".to_string(),
        owns_http_server: false,
        bacon_lite_writer: None,
        counters: vec![GovernanceCounterSnapshot {
            name: "arda_governance_cli_contract_total".to_string(),
            labels: BTreeMap::from([("result".to_string(), "pass".to_string())]),
            value: 1,
        }],
        histograms: Vec::new(),
    };
    let rendered = render_governance_prometheus(&snapshot);
    assert!(rendered.contains("arda_governance_cli_contract_total"));
    assert!(rendered.contains("result=\"pass\""));
}
