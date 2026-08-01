#![cfg(not(feature = "full-cli"))]
use arda_aule::council::{CouncilBrief, CouncilQuery, CouncilSeat, QueryMode};

#[test]
fn council_root_module_resolves_without_cross_crate_dependencies() {
    assert_eq!(CouncilSeat::Strategist as u8, CouncilSeat::Strategist as u8);
    assert_eq!(QueryMode::SingleSeat as u8, QueryMode::SingleSeat as u8);
}

#[test]
fn full_council_brief_expands_seats_and_requires_escalation() {
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
