#![cfg(not(feature = "full-cli"))]
use arda_aule::council::{CouncilSeat, QueryMode};

#[test]
fn council_root_module_resolves_without_cross_crate_dependencies() {
    assert_eq!(CouncilSeat::Strategist as u8, CouncilSeat::Strategist as u8);
    assert_eq!(QueryMode::SingleSeat as u8, QueryMode::SingleSeat as u8);
}
