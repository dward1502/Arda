// sigil: REPAIR
use annunimas_core::task::Task;
use annunimas_governance::game_theory::GameTheory;
use annunimas_governance::triad::triad_validate;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForeignState {
    Unknown,
    Quarantined,
    Probation,
    Trusted,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignAgent {
    pub name: String,
    pub container_id: String,
    pub state: ForeignState,
    pub entry_time: chrono::DateTime<chrono::Utc>,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub resonance_history: Vec<f64>,
    pub triad_fails: u32,
    pub probation_end: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct ForeignProtocol {
    agents: HashMap<String, ForeignAgent>,
    _game_theory: GameTheory,
}

impl Default for ForeignProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl ForeignProtocol {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            _game_theory: GameTheory::new(),
        }
    }

    /// Register a new foreign agent
    pub fn register_new(&mut self, name: String, container_id: String) {
        let agent = ForeignAgent {
            name: name.clone(),
            container_id,
            state: ForeignState::Unknown,
            entry_time: Utc::now(),
            last_check: Utc::now(),
            resonance_history: vec![],
            triad_fails: 0,
            probation_end: None,
        };

        // Immediate quarantine on registration
        let mock_task = Task::new(format!("Vet new agent {}", name), "admission".to_string());
        let result = triad_validate(&mock_task, None);

        if result.passed {
            self.transition_to_probation(&mock_task, agent);
        } else {
            self.transition_to_quarantine(&mock_task, agent);
        }
    }

    fn transition_to_quarantine(&mut self, task: &Task, mut agent: ForeignAgent) {
        agent.state = ForeignState::Quarantined;
        agent.last_check = Utc::now();
        self.agents.insert(agent.name.clone(), agent);
        tracing::warn!("Quarantined foreign agent: {}", task.description);
    }

    fn transition_to_probation(&mut self, task: &Task, mut agent: ForeignAgent) {
        agent.state = ForeignState::Probation;
        agent.probation_end = Some(Utc::now() + chrono::Duration::hours(24));
        agent.last_check = Utc::now();
        self.agents.insert(agent.name.clone(), agent);
        tracing::info!("Probation started for foreign agent: {}", task.description);
    }

    /// Check probation status
    pub fn check_probation(&mut self) {
        let now = Utc::now();
        let mut to_remove = Vec::new();

        for (name, agent) in self.agents.iter_mut() {
            if agent.state != ForeignState::Probation {
                continue;
            }

            if let Some(end) = agent.probation_end {
                if now > end {
                    let avg_resonance = agent.resonance_history.iter().sum::<f64>()
                        / agent.resonance_history.len().max(1) as f64;

                    if avg_resonance > 75.0 && agent.triad_fails < 2 {
                        agent.state = ForeignState::Trusted;
                        tracing::info!("Foreign agent {} promoted to Trusted", name);
                    } else {
                        agent.state = ForeignState::Revoked;
                        to_remove.push(name.clone());
                        tracing::warn!("Foreign agent {} revoked", name);
                    }
                }
            }
        }

        for name in to_remove {
            self.agents.remove(&name);
        }
    }

    /// Update agent resonance
    pub fn update_resonance(&mut self, agent_name: &str, resonance: f64) {
        if let Some(agent) = self.agents.get_mut(agent_name) {
            agent.resonance_history.push(resonance);
            agent.last_check = Utc::now();
        }
    }

    pub fn get_agent(&self, agent_name: &str) -> Option<&ForeignAgent> {
        self.agents.get(agent_name)
    }
}

#[cfg(test)]
mod tests {
    use super::{ForeignProtocol, ForeignState};
    use chrono::{Duration, Utc};

    #[test]
    fn register_new_places_agent_under_management() {
        let mut protocol = ForeignProtocol::new();
        protocol.register_new("external_oracle".to_owned(), "ctr-123".to_owned());

        let agent = protocol.get_agent("external_oracle").expect("agent");
        assert_eq!(agent.name, "external_oracle");
        assert_eq!(agent.container_id, "ctr-123");
        assert!(matches!(
            agent.state,
            ForeignState::Probation | ForeignState::Quarantined
        ));
    }

    #[test]
    fn probation_agent_with_strong_resonance_promotes_to_trusted() {
        let mut protocol = ForeignProtocol::new();
        protocol.register_new("trusted_candidate".to_owned(), "ctr-789".to_owned());

        let Some(agent) = protocol.agents.get_mut("trusted_candidate") else {
            panic!("agent missing");
        };
        agent.state = ForeignState::Probation;
        agent.probation_end = Some(Utc::now() - Duration::minutes(1));
        agent.resonance_history = vec![80.0, 82.0, 90.0];
        agent.triad_fails = 0;

        protocol.check_probation();

        let agent = protocol
            .get_agent("trusted_candidate")
            .expect("trusted agent");
        assert_eq!(agent.state, ForeignState::Trusted);
    }

    #[test]
    fn probation_agent_with_weak_resonance_is_revoked_and_removed() {
        let mut protocol = ForeignProtocol::new();
        protocol.register_new("weak_candidate".to_owned(), "ctr-456".to_owned());

        let Some(agent) = protocol.agents.get_mut("weak_candidate") else {
            panic!("agent missing");
        };
        agent.state = ForeignState::Probation;
        agent.probation_end = Some(Utc::now() - Duration::minutes(1));
        agent.resonance_history = vec![30.0, 40.0, 45.0];
        agent.triad_fails = 0;

        protocol.check_probation();

        assert!(protocol.get_agent("weak_candidate").is_none());
    }
}
