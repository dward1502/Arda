#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Objective decomposition — breaks high-level objectives into ordered tasks
//! using a registered template library, with resource estimates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub statement: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub key: String,
    pub title: String,
    pub task_type: String,
    pub depends_on: Vec<String>,
    pub priority: Priority,
    pub joule_cost: f64,
    pub eta_seconds: u64,
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

pub struct ObjectiveTemplate {
    pub matcher: fn(&Objective) -> bool,
    pub builder: fn(&Objective) -> Vec<PlannedTask>,
    pub label: &'static str,
}

pub struct ObjectiveDecomposer {
    templates: Vec<ObjectiveTemplate>,
    base_costs: BTreeMap<String, f64>,
    default_cost: f64,
}

impl Default for ObjectiveDecomposer {
    fn default() -> Self {
        Self {
            templates: vec![
                ObjectiveTemplate {
                    label: "reliability",
                    matcher: matches_reliability,
                    builder: build_reliability,
                },
                ObjectiveTemplate {
                    label: "research",
                    matcher: matches_research,
                    builder: build_research,
                },
                ObjectiveTemplate {
                    label: "deploy",
                    matcher: matches_deploy,
                    builder: build_deploy,
                },
            ],
            base_costs: BTreeMap::new(),
            default_cost: 10.0,
        }
    }
}

impl ObjectiveDecomposer {
    pub fn with_base_costs(mut self, costs: BTreeMap<String, f64>) -> Self {
        self.base_costs = costs;
        self
    }
    pub fn default_cost(mut self, c: f64) -> Self {
        self.default_cost = c;
        self
    }

    pub fn register(&mut self, t: ObjectiveTemplate) {
        self.templates.push(t);
    }

    pub fn decompose(&self, obj: &Objective) -> Vec<PlannedTask> {
        let mut tasks = self
            .templates
            .iter()
            .find(|t| (t.matcher)(obj))
            .map(|t| (t.builder)(obj))
            .unwrap_or_else(|| build_generic(obj));
        for t in &mut tasks {
            if t.joule_cost == 0.0 {
                let canonical = super::taxonomy::canonical(&t.task_type);
                t.joule_cost = self
                    .base_costs
                    .get(canonical)
                    .or_else(|| self.base_costs.get(&t.task_type))
                    .copied()
                    .unwrap_or(self.default_cost);
            }
        }
        tasks
    }
}

fn matches_reliability(o: &Objective) -> bool {
    let s = o.statement.to_lowercase();
    s.contains("reliability") || s.contains("uptime") || s.contains("availab")
}
fn matches_research(o: &Objective) -> bool {
    let s = o.statement.to_lowercase();
    s.contains("research") || s.contains("investigate") || s.contains("audit")
}
fn matches_deploy(o: &Objective) -> bool {
    let s = o.statement.to_lowercase();
    s.contains("deploy") || s.contains("release") || s.contains("rollout")
}

fn build_reliability(o: &Objective) -> Vec<PlannedTask> {
    vec![
        task(
            "inspect",
            &format!("Inspect current health for: {}", o.statement),
            "monitor",
            &[],
            Priority::High,
            "warden",
            30,
        ),
        task(
            "hypothesize",
            "Form hypotheses for failure modes",
            "analysis",
            &["inspect"],
            Priority::High,
            "athena",
            60,
        ),
        task(
            "remediate",
            "Apply remediation steps",
            "ops",
            &["hypothesize"],
            Priority::Critical,
            "ceo",
            120,
        ),
        task(
            "verify",
            "Verify post-remediation metrics",
            "monitor",
            &["remediate"],
            Priority::High,
            "warden",
            30,
        ),
    ]
}
fn build_research(o: &Objective) -> Vec<PlannedTask> {
    vec![
        task(
            "scope",
            &format!("Scope research: {}", o.statement),
            "analysis",
            &[],
            Priority::Medium,
            "athena",
            30,
        ),
        task(
            "gather",
            "Gather sources and evidence",
            "research",
            &["scope"],
            Priority::Medium,
            "athena",
            180,
        ),
        task(
            "synthesize",
            "Synthesize findings into report",
            "synthesis",
            &["gather"],
            Priority::High,
            "prometheus",
            120,
        ),
    ]
}
fn build_deploy(o: &Objective) -> Vec<PlannedTask> {
    vec![
        task(
            "preflight",
            "Preflight checks and budget validation",
            "policy",
            &[],
            Priority::High,
            "ceo",
            30,
        ),
        task(
            "build",
            "Build release artifacts",
            "build",
            &["preflight"],
            Priority::High,
            "ceo",
            600,
        ),
        task(
            "rollout",
            &format!("Rollout: {}", o.statement),
            "ops",
            &["build"],
            Priority::Critical,
            "ceo",
            300,
        ),
        task(
            "monitor",
            "Post-rollout monitoring window",
            "monitor",
            &["rollout"],
            Priority::High,
            "warden",
            600,
        ),
    ]
}
fn build_generic(o: &Objective) -> Vec<PlannedTask> {
    vec![
        task(
            "plan",
            &format!("Plan: {}", o.statement),
            "analysis",
            &[],
            Priority::Medium,
            "prometheus",
            60,
        ),
        task(
            "execute",
            &format!("Execute: {}", o.statement),
            "ops",
            &["plan"],
            Priority::High,
            "ceo",
            180,
        ),
        task(
            "verify",
            "Verify outcome",
            "monitor",
            &["execute"],
            Priority::Medium,
            "warden",
            60,
        ),
    ]
}

fn task(
    key: &str,
    title: &str,
    task_type: &str,
    deps: &[&str],
    pri: Priority,
    agent: &str,
    eta: u64,
) -> PlannedTask {
    PlannedTask {
        key: key.into(),
        title: title.into(),
        task_type: task_type.into(),
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
        priority: pri,
        joule_cost: 0.0,
        eta_seconds: eta,
        assigned_agent: Some(agent.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn obj(s: &str) -> Objective {
        Objective {
            id: "o".into(),
            statement: s.into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        }
    }
    #[test]
    fn picks_reliability_template() {
        let d = ObjectiveDecomposer::default();
        let t = d.decompose(&obj("Increase system reliability"));
        assert_eq!(t.len(), 4);
        assert!(t.iter().all(|p| p.joule_cost > 0.0));
        assert!(t.iter().any(|p| p.priority >= Priority::High));
    }
    #[test]
    fn falls_back_to_generic() {
        let d = ObjectiveDecomposer::default();
        let t = d.decompose(&obj("Refactor module foo"));
        assert_eq!(t.len(), 3);
    }
}
