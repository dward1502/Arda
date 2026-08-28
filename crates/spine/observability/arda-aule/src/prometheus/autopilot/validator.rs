#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Plan validator — dependency graph + resource/budget checks.

use super::decomposer::{ObjectivePlan, PlannedTask};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub topological_order: Vec<String>,
    pub estimated_total_joules: f64,
    pub agent_load: BTreeMap<String, usize>,
}

pub struct PlanValidator {
    pub joule_budget: f64,
    pub max_per_agent: usize,
}

impl Default for PlanValidator {
    fn default() -> Self {
        Self {
            joule_budget: 1_000.0,
            max_per_agent: 16,
        }
    }
}

impl PlanValidator {
    pub fn validate(&self, tasks: &[PlannedTask]) -> ValidationResult {
        let mut r = ValidationResult {
            ok: true,
            ..Default::default()
        };
        let keys: BTreeSet<&str> = tasks.iter().map(|t| t.key.as_str()).collect();
        if keys.len() != tasks.len() {
            r.errors.push("duplicate task keys".into());
            r.ok = false;
        }
        for t in tasks {
            for d in &t.depends_on {
                if !keys.contains(d.as_str()) {
                    r.errors
                        .push(format!("task '{}' depends on missing '{}'", t.key, d));
                    r.ok = false;
                }
            }
            if let Some(a) = &t.assigned_agent {
                *r.agent_load.entry(a.clone()).or_insert(0) += 1;
            }
            r.estimated_total_joules += t.joule_cost;
        }
        match toposort(tasks) {
            Ok(order) => r.topological_order = order,
            Err(cycle) => {
                r.errors
                    .push(format!("dependency cycle: {}", cycle.join(" -> ")));
                r.ok = false;
            }
        }
        if r.estimated_total_joules > self.joule_budget {
            r.errors.push(format!(
                "budget exceeded: {:.1} > {:.1}",
                r.estimated_total_joules, self.joule_budget
            ));
            r.ok = false;
        }
        for (agent, load) in &r.agent_load {
            if *load > self.max_per_agent {
                r.warnings.push(format!(
                    "agent '{}' overloaded ({} > {})",
                    agent, load, self.max_per_agent
                ));
            }
        }
        r
    }

    pub fn validate_objective_plan(&self, plan: &ObjectivePlan) -> ValidationResult {
        let mut result = self.validate(&plan.tasks);
        for task in &plan.tasks {
            let Some(contract) = plan.leaf_contracts.get(&task.key) else {
                result.errors.push(format!(
                    "task '{}' has no executable leaf contract",
                    task.key
                ));
                continue;
            };
            if contract.project_id.is_empty()
                || contract.authority_class.is_empty()
                || contract.verification_checks.is_empty()
                || contract.evidence_requirements.is_empty()
                || contract.max_joules <= 0.0
                || contract.max_cost_usd <= 0.0
                || contract.max_attempts == 0
                || contract.timeout_seconds == 0
            {
                result.errors.push(format!(
                    "task '{}' has an incomplete executable leaf contract",
                    task.key
                ));
            }
        }
        if plan.acceptance_criteria.is_empty() {
            result
                .errors
                .push("objective plan has no acceptance criteria".into());
        }
        if plan.context_sources.is_empty() {
            result
                .errors
                .push("objective plan has no grounded context sources".into());
        }
        if result.topological_order.last().map(String::as_str) != Some("verify-acceptance") {
            result
                .errors
                .push("objective plan must terminate in acceptance verification".into());
        }
        result.ok = result.errors.is_empty();
        result
    }
}

fn toposort(tasks: &[PlannedTask]) -> Result<Vec<String>, Vec<String>> {
    let mut indeg: HashMap<&str, usize> = tasks.iter().map(|t| (t.key.as_str(), 0)).collect();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for t in tasks {
        for d in &t.depends_on {
            adj.entry(d.as_str()).or_default().push(t.key.as_str());
            *indeg.entry(t.key.as_str()).or_insert(0) += 1;
        }
    }
    let mut q: VecDeque<&str> = indeg
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(k, _)| *k)
        .collect();
    let mut out = Vec::new();
    while let Some(n) = q.pop_front() {
        out.push(n.to_string());
        if let Some(nbrs) = adj.get(n) {
            for m in nbrs {
                if let Some(e) = indeg.get_mut(m) {
                    *e -= 1;
                    if *e == 0 {
                        q.push_back(*m);
                    }
                }
            }
        }
    }
    if out.len() == tasks.len() {
        Ok(out)
    } else {
        Err(indeg
            .iter()
            .filter(|(_, n)| **n > 0)
            .map(|(k, _)| k.to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::super::decomposer::{
        Objective, ObjectiveContextSource, ObjectiveDecomposer, PlannedTask, Priority,
    };
    use super::*;
    fn t(k: &str, deps: &[&str], cost: f64, agent: &str) -> PlannedTask {
        PlannedTask {
            key: k.into(),
            title: k.into(),
            task_type: "ops".into(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            priority: Priority::Medium,
            joule_cost: cost,
            eta_seconds: 10,
            assigned_agent: Some(agent.into()),
        }
    }
    #[test]
    fn detects_cycle() {
        let p = vec![t("a", &["b"], 1.0, "x"), t("b", &["a"], 1.0, "x")];
        let r = PlanValidator::default().validate(&p);
        assert!(!r.ok);
    }
    #[test]
    fn enforces_budget() {
        let p = vec![t("a", &[], 999.0, "x"), t("b", &["a"], 999.0, "x")];
        let v = PlanValidator {
            joule_budget: 100.0,
            max_per_agent: 16,
        };
        let r = v.validate(&p);
        assert!(!r.ok);
    }
    #[test]
    fn passes_clean_plan() {
        let p = vec![t("a", &[], 5.0, "x"), t("b", &["a"], 5.0, "y")];
        let r = PlanValidator::default().validate(&p);
        assert!(r.ok);
        assert_eq!(r.topological_order, vec!["a", "b"]);
    }

    #[test]
    fn grounded_plan_requires_context_acceptance_and_terminal_verification() {
        let objective = Objective {
            id: "objective-1".into(),
            statement: "Review Arda".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        let plan = ObjectiveDecomposer::default().decompose_grounded(&objective, Vec::new());

        let result = PlanValidator::default().validate_objective_plan(&plan);

        assert!(!result.ok);
        assert!(result
            .errors
            .contains(&"objective plan has no acceptance criteria".to_string()));
        assert!(result
            .errors
            .contains(&"objective plan has no grounded context sources".to_string()));
    }

    #[test]
    fn grounded_plan_with_sources_and_acceptance_is_valid() {
        let objective = Objective {
            id: "objective-1".into(),
            statement: "Review Arda".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec!["Produce a prioritized repair backlog".into()],
            tags: vec![],
        };
        let plan = ObjectiveDecomposer::default().decompose_grounded(
            &objective,
            vec![ObjectiveContextSource::new(
                "active_plan",
                "docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md",
            )],
        );

        let result = PlanValidator::default().validate_objective_plan(&plan);

        assert!(result.ok, "{:?}", result.errors);
        assert_eq!(
            result.topological_order.last().unwrap(),
            "verify-acceptance"
        );
    }
}
