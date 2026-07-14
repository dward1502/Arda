// sigil: ANKH
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CouncilSeat {
    Economist,
    Attorney,
    Cfo,
    TaxStrategist,
    ContractSpecialist,
    Strategist,
    Operator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryMode {
    SingleSeat,
    DualSeat,
    FullCouncil,
    DevilsAdvocate,
    ScenarioStressTest,
    DocumentReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CouncilQuery {
    pub mode: QueryMode,
    pub seats: Vec<CouncilSeat>,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CouncilBrief {
    pub participating_seats: Vec<CouncilSeat>,
    pub escalation_required: bool,
    pub required_outputs: Vec<&'static str>,
}

impl CouncilBrief {
    pub fn from_query(query: &CouncilQuery) -> Self {
        let participating_seats = if matches!(query.mode, QueryMode::FullCouncil) {
            vec![
                CouncilSeat::Economist,
                CouncilSeat::Attorney,
                CouncilSeat::Cfo,
                CouncilSeat::TaxStrategist,
                CouncilSeat::ContractSpecialist,
                CouncilSeat::Strategist,
                CouncilSeat::Operator,
            ]
        } else {
            query.seats.clone()
        };
        let escalation_required = participating_seats.iter().any(|seat| {
            matches!(
                seat,
                CouncilSeat::Attorney | CouncilSeat::Cfo | CouncilSeat::TaxStrategist
            )
        });
        Self {
            participating_seats,
            escalation_required,
            required_outputs: vec![
                "seat_opinions",
                "points_of_agreement",
                "points_of_tension",
                "synthesis_recommendation",
                "licensed_professional_escalation_flag",
            ],
        }
    }
}
