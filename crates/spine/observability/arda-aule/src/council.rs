// sigil: ANKH
#[cfg(feature = "full-cli")]
use arda_core::council_run::{CouncilAuthority, CouncilRun, CouncilState};
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

/// Evidence-backed operator projection for a retained structured council run.
/// Missing worker opinions remain explicitly unavailable rather than being
/// interpreted as agreement.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg(feature = "full-cli")]
pub struct StructuredCouncilProjection {
    pub council_id: String,
    pub run_id: String,
    pub state: CouncilState,
    pub participant_count: usize,
    pub agreement_count: usize,
    pub material_tension_count: usize,
    pub unresolved_tension_count: usize,
    pub synthesis: String,
    pub requested_decision: Option<String>,
    pub evidence_available: bool,
    pub non_approval: bool,
}

#[cfg(feature = "full-cli")]
impl StructuredCouncilProjection {
    pub fn from_run(council: &CouncilRun) -> Self {
        let unresolved_tension_count = council
            .material_tensions
            .iter()
            .filter(|tension| !tension.resolved)
            .count();
        let evidence_available = !council.evidence_boundary.is_empty()
            && council
                .participants
                .iter()
                .all(|participant| !participant.evidence_refs.is_empty());
        Self {
            council_id: council.council_id.clone(),
            run_id: council.run_id.clone(),
            state: council.state,
            participant_count: council.participants.len(),
            agreement_count: council.agreements.len(),
            material_tension_count: council.material_tensions.len(),
            unresolved_tension_count,
            synthesis: council.synthesis.clone(),
            requested_decision: matches!(
                council.authority,
                CouncilAuthority::HumanDecisionRequired
            )
            .then(|| council.escalation_recommendation.clone()),
            evidence_available,
            non_approval: council.non_approval,
        }
    }

    pub fn unavailable(council_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            council_id: council_id.into(),
            run_id: run_id.into(),
            state: CouncilState::CollectingOpinions,
            participant_count: 0,
            agreement_count: 0,
            material_tension_count: 0,
            unresolved_tension_count: 0,
            synthesis: "council evidence unavailable".to_string(),
            requested_decision: None,
            evidence_available: false,
            non_approval: true,
        }
    }
}
