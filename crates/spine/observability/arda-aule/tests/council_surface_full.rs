#![cfg(feature = "full-cli")]
use arda_aule::council::{CouncilSeat, QueryMode};
use arda_aule::prometheus::council::{CouncilGateConfig, CouncilOutcome, run_council_gate};

#[test]
fn council_root_module_resolves_under_full_cli() {
    assert_eq!(CouncilSeat::Strategist as u8, CouncilSeat::Strategist as u8);
    assert_eq!(QueryMode::SingleSeat as u8, QueryMode::SingleSeat as u8);
}

#[test]
fn prometheus_council_types_are_constructible_under_full_cli() {
    let config = CouncilGateConfig::default();
    let outcome = CouncilOutcome {
        triggered: false,
        responders_expected: 0,
        responders_available: 0,
        timed_out: false,
        adjusted_confidence: 0.95,
        query_mode: "single_seat".to_string(),
        participating_seats: Vec::new(),
        escalation_required: false,
        reason: "council not required".to_string(),
    };

    assert_eq!(config.timeout_ms, 1500);
    assert!(!outcome.triggered);
    assert!((outcome.adjusted_confidence - 0.95).abs() < f64::EPSILON);
}

#[test]
fn run_council_gate_triggers_on_high_complexity_under_full_cli() {
    let task = arda_core::task::Task::new("review legal contract and tax pricing exposure", "decision");
    let config = CouncilGateConfig::default();
    let outcome = run_council_gate(&task, 0.95, None, &config);
    assert!(outcome.triggered);
    assert!(outcome.participating_seats.contains(&"attorney".to_string()));
    assert!(outcome.participating_seats.contains(&"tax_strategist".to_string()));
}

#[test]
fn run_council_gate_skips_when_confidence_below_threshold_under_full_cli() {
    let task = arda_core::task::Task::new("delete everything immediately", "decision");
    let config = CouncilGateConfig::default();
    let outcome = run_council_gate(&task, 0.1, None, &config);
    assert!(!outcome.triggered);
    assert!((outcome.adjusted_confidence - 0.1).abs() < f64::EPSILON);
}

#[test]
fn run_council_gate_adjusts_confidence_for_low_availability_under_full_cli() {
    let task = arda_core::task::Task::new("external security operation", "decision");
    let config = CouncilGateConfig::default();
    let outcome = run_council_gate(&task, 0.95, None, &config);
    assert!(outcome.timed_out);
    assert!(outcome.adjusted_confidence < 0.85);
}

#[test]
fn run_council_gate_aggregates_seats_for_legal_and_tax_task_under_full_cli() {
    let task = arda_core::task::Task::new("legal contract and tax pricing exposure", "decision");
    let config = CouncilGateConfig::default();
    let outcome = run_council_gate(&task, 0.95, None, &config);
    assert!(outcome.triggered);
    assert!(
        outcome
            .participating_seats
            .contains(&"strategist".to_string())
    );
    assert!(
        outcome
            .participating_seats
            .contains(&"operator".to_string())
    );
    assert!(
        outcome
            .participating_seats
            .contains(&"attorney".to_string())
    );
    assert!(
        outcome
            .participating_seats
            .contains(&"economist".to_string())
    );
    assert!(
        outcome
            .participating_seats
            .contains(&"cfo".to_string())
    );
    assert!(
        outcome
            .participating_seats
            .contains(&"tax_strategist".to_string())
    );
    assert!(outcome.escalation_required);
}
