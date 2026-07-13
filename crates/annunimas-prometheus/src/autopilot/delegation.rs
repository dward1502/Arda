// sigil: REPAIR
//! Delegation manager + agent capability registry.

use super::learning::LearningState;
use crate::autopilot::decomposer::PlannedTask;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct AgentCapabilities {
    pub agent_id: String,
    pub task_types: Vec<String>,
    pub max_concurrent: usize,
    pub current_load: usize,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, AgentCapabilities>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, a: AgentCapabilities) {
        self.agents.insert(a.agent_id.clone(), a);
    }
    pub fn agents(&self) -> impl Iterator<Item = &AgentCapabilities> {
        self.agents.values()
    }

    pub fn pick(&self, task: &PlannedTask) -> Option<String> {
        // Prefer assigned, capacity-respecting, high-success agent.
        if let Some(preferred) = task.assigned_agent.as_deref() {
            if let Some(a) = self.agents.get(preferred) {
                if a.current_load < a.max_concurrent
                    && a.task_types.iter().any(|t| t == &task.task_type)
                {
                    return Some(preferred.to_string());
                }
            }
        }
        let mut best: Option<(&AgentCapabilities, f64)> = None;
        for a in self.agents.values() {
            if a.current_load >= a.max_concurrent {
                continue;
            }
            if !a.task_types.iter().any(|t| t == &task.task_type) {
                continue;
            }
            let head = (a.max_concurrent - a.current_load) as f64;
            let score = a.success_rate * 0.7 + (head / a.max_concurrent as f64) * 0.3;
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((a, score));
            }
        }
        best.map(|(a, _)| a.agent_id.clone())
    }

    pub fn record_assigned(&mut self, agent_id: &str) {
        if let Some(a) = self.agents.get_mut(agent_id) {
            a.current_load = a.current_load.saturating_add(1);
        }
    }
    pub fn record_completed(&mut self, agent_id: &str, success: bool) {
        if let Some(a) = self.agents.get_mut(agent_id) {
            a.current_load = a.current_load.saturating_sub(1);
            // EWMA over success rate, alpha=0.2.
            let obs = if success { 1.0 } else { 0.0 };
            a.success_rate = 0.8 * a.success_rate + 0.2 * obs;
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Delegation {
    pub task_key: String,
    pub assigned_agent: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DelegationReport {
    pub delegations: Vec<Delegation>,
    pub unassigned: Vec<String>,
}

pub fn delegate_plan(
    reg: &mut AgentRegistry,
    learning: &LearningState,
    plan: &[PlannedTask],
) -> DelegationReport {
    let mut report = DelegationReport::default();
    for t in plan {
        let fallback = reg.pick(t).or_else(|| {
            if t.task_type.starts_with("delegate::") {
                learning.best_agent(&t.task_type)
            } else {
                None
            }
        });
        match fallback {
            Some(a) => {
                reg.record_assigned(&a);
                report.delegations.push(Delegation {
                    task_key: t.key.clone(),
                    assigned_agent: a,
                });
            }
            None => report.unassigned.push(t.key.clone()),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autopilot::decomposer::{PlannedTask, Priority};

    fn task(k: &str, t: &str, agent: Option<&str>) -> PlannedTask {
        PlannedTask {
            key: k.into(),
            title: k.into(),
            task_type: t.into(),
            depends_on: vec![],
            priority: Priority::Medium,
            joule_cost: 1.0,
            eta_seconds: 1,
            assigned_agent: agent.map(|s| s.into()),
        }
    }
    fn agent(id: &str, types: &[&str], cap: usize) -> AgentCapabilities {
        AgentCapabilities {
            agent_id: id.into(),
            task_types: types.iter().map(|s| s.to_string()).collect(),
            max_concurrent: cap,
            current_load: 0,
            success_rate: 1.0,
        }
    }
    #[test]
    fn delegates_to_capable_agent() {
        let mut r = AgentRegistry::new();
        r.register(agent("warden", &["monitor"], 4));
        r.register(agent("athena", &["analysis"], 4));
        let plan = vec![task("t1", "monitor", None), task("t2", "analysis", None)];
        let rep = delegate_plan(&mut r, &LearningState::default(), &plan);
        assert_eq!(rep.delegations.len(), 2);
        assert!(rep.unassigned.is_empty());
    }
    #[test]
    fn reports_unassigned_when_no_capability() {
        let mut r = AgentRegistry::new();
        r.register(agent("warden", &["monitor"], 4));
        let plan = vec![task("t1", "exotic", None)];
        let rep = delegate_plan(&mut r, &LearningState::default(), &plan);
        assert_eq!(rep.delegations.len(), 0);
        assert_eq!(rep.unassigned, vec!["t1".to_string()]);
    }
    #[test]
    fn fallback_learns_best_agent_when_registry_is_ambiguous() {
        let mut state = LearningState::default();
        for _ in 0..5 {
            state.observe("oracle", "delegate::planning", true, 1.0, 1.0);
        }
        let mut r = AgentRegistry::new();
        r.register(agent("athena", &["analysis"], 4));
        let plan = vec![task("t1", "delegate::planning", None)];
        let rep = delegate_plan(&mut r, &state, &plan);
        assert_eq!(rep.delegations.len(), 1);
        assert_eq!(rep.delegations[0].assigned_agent, "oracle");
    }
    #[test]
    fn fallback_is_noop_when_evidence_insufficient() {
        let state = LearningState::default();
        let mut r = AgentRegistry::new();
        let plan = vec![task("t1", "delegate::planning", None)];
        let rep = delegate_plan(&mut r, &state, &plan);
        assert_eq!(rep.delegations.len(), 0);
        assert_eq!(rep.unassigned, vec!["t1".to_string()]);
    }
}
