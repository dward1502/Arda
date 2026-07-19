#![cfg(feature = "full-cli")]
// sigil: REPAIR
use crate::registry::AgentRosterSnapshot;
use arda_core::task::Task;
use arda_council::council::{CouncilQuery, CouncilSeat, QueryMode};
use arda_council::service::build_brief;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilGateConfig {
    pub enabled: bool,
    pub complexity_threshold: f64,
    pub timeout_ms: u64,
}

impl Default for CouncilGateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            complexity_threshold: 0.80,
            timeout_ms: 1500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilOutcome {
    pub triggered: bool,
    pub responders_expected: usize,
    pub responders_available: usize,
    pub timed_out: bool,
    pub adjusted_confidence: f64,
    pub query_mode: String,
    pub participating_seats: Vec<String>,
    pub escalation_required: bool,
    pub reason: String,
}

pub fn run_council_gate(
    task: &Task,
    base_confidence: f64,
    roster: Option<&AgentRosterSnapshot>,
    config: &CouncilGateConfig,
) -> CouncilOutcome {
    if !config.enabled || base_confidence < config.complexity_threshold {
        return CouncilOutcome {
            triggered: false,
            responders_expected: 0,
            responders_available: 0,
            timed_out: false,
            adjusted_confidence: base_confidence,
            query_mode: "single_seat".to_string(),
            participating_seats: Vec::new(),
            escalation_required: false,
            reason: "council not required".to_string(),
        };
    }

    let query = derive_council_query(task);
    let brief = build_brief(&query);

    let responders_expected = 4usize; // ATHENA, HADES, CHARON, MNEMOSYNE (scaffold)
    let responders_available = roster
        .map(|r| r.online_agents.min(responders_expected))
        .unwrap_or(1);
    let completion_ratio = responders_available as f64 / responders_expected as f64;
    let timed_out = completion_ratio < 0.5;

    let mut adjusted = base_confidence;
    if completion_ratio >= 0.75 {
        adjusted += 0.03;
    } else if completion_ratio >= 0.5 {
        adjusted -= 0.02;
    } else {
        adjusted -= 0.07;
    }

    let lower = task.description.to_ascii_lowercase();
    if lower.contains("delete") || lower.contains("security") || lower.contains("external") {
        adjusted -= 0.05;
    }
    if brief.escalation_required {
        adjusted -= 0.03;
    }

    CouncilOutcome {
        triggered: true,
        responders_expected,
        responders_available,
        timed_out,
        adjusted_confidence: adjusted.clamp(0.0, 1.0),
        query_mode: council_mode_name(query.mode).to_string(),
        participating_seats: brief
            .participating_seats
            .iter()
            .map(|seat| council_seat_name(*seat).to_string())
            .collect(),
        escalation_required: brief.escalation_required,
        reason: format!(
            "council responders {}/{} (ratio {:.2}) mode={} escalation_required={}",
            responders_available,
            responders_expected,
            completion_ratio,
            council_mode_name(query.mode),
            brief.escalation_required
        ),
    }
}

fn derive_council_query(task: &Task) -> CouncilQuery {
    let lower = task.description.to_ascii_lowercase();
    let mut seats = vec![CouncilSeat::Strategist, CouncilSeat::Operator];
    if contains_any(&lower, &["legal", "contract", "liability", "terms"]) {
        seats.push(CouncilSeat::Attorney);
        seats.push(CouncilSeat::ContractSpecialist);
    }
    if contains_any(
        &lower,
        &[
            "finance", "budget", "revenue", "pricing", "invoice", "tax", "margin",
        ],
    ) {
        seats.push(CouncilSeat::Economist);
        seats.push(CouncilSeat::Cfo);
    }
    if lower.contains("tax") {
        seats.push(CouncilSeat::TaxStrategist);
    }
    seats.sort_by_key(|seat| *seat as u8);
    seats.dedup();

    let mode = if seats.len() >= 5 {
        QueryMode::FullCouncil
    } else if lower.contains("scenario") || lower.contains("what if") || lower.contains("stress") {
        QueryMode::ScenarioStressTest
    } else if lower.contains("review") || lower.contains("document") {
        QueryMode::DocumentReview
    } else if seats.len() >= 2 {
        QueryMode::DualSeat
    } else {
        QueryMode::SingleSeat
    };

    CouncilQuery {
        mode,
        seats,
        prompt: task.description.clone(),
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn council_mode_name(mode: QueryMode) -> &'static str {
    match mode {
        QueryMode::SingleSeat => "single_seat",
        QueryMode::DualSeat => "dual_seat",
        QueryMode::FullCouncil => "full_council",
        QueryMode::DevilsAdvocate => "devils_advocate",
        QueryMode::ScenarioStressTest => "scenario_stress_test",
        QueryMode::DocumentReview => "document_review",
    }
}

fn council_seat_name(seat: CouncilSeat) -> &'static str {
    match seat {
        CouncilSeat::Economist => "economist",
        CouncilSeat::Attorney => "attorney",
        CouncilSeat::Cfo => "cfo",
        CouncilSeat::TaxStrategist => "tax_strategist",
        CouncilSeat::ContractSpecialist => "contract_specialist",
        CouncilSeat::Strategist => "strategist",
        CouncilSeat::Operator => "operator",
    }
}

#[cfg(test)]
mod tests {
    use super::{run_council_gate, CouncilGateConfig};
    use crate::registry::{AgentRosterSnapshot, AgentStatus};
    use arda_core::task::Task;

    #[test]
    fn council_reduces_when_low_availability() {
        let task = Task::new("external delete", "decision");
        let roster = AgentRosterSnapshot {
            total_agents: 1,
            online_agents: 1,
            silent_agents: 0,
            agents: vec![AgentStatus {
                id: "athena".to_string(),
                name: "Athena".to_string(),
                status: "ONLINE".to_string(),
                last_heartbeat: None,
            }],
        };
        let out = run_council_gate(&task, 0.9, Some(&roster), &CouncilGateConfig::default());
        assert!(out.triggered);
        assert!(out.adjusted_confidence < 0.9);
        assert!(out.timed_out);
        assert_eq!(out.responders_available, 1);
        assert_eq!(out.responders_expected, 4);
        assert!(out
            .participating_seats
            .iter()
            .any(|seat| seat == "strategist"));
    }

    #[test]
    fn finance_and_legal_tasks_expand_council_seats() {
        let task = Task::new("review legal contract and tax pricing exposure", "decision");
        let out = run_council_gate(&task, 0.9, None, &CouncilGateConfig::default());
        assert_eq!(out.query_mode, "full_council");
        assert!(out.escalation_required);
        assert!(out
            .participating_seats
            .iter()
            .any(|seat| seat == "attorney"));
        assert!(out
            .participating_seats
            .iter()
            .any(|seat| seat == "tax_strategist"));
    }
}
