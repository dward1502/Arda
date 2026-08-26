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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveContextSource {
    pub kind: String,
    pub reference: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

impl ObjectiveContextSource {
    pub fn new(kind: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            reference: reference.into(),
            digest: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectivePlan {
    pub objective_id: String,
    pub tasks: Vec<PlannedTask>,
    pub context_sources: Vec<ObjectiveContextSource>,
    pub acceptance_criteria: Vec<String>,
    pub approval_required: bool,
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

    /// Build a general, evidence-grounded objective plan. The plan separates
    /// context recovery, inspection, synthesis, outcome production, and
    /// acceptance verification so arbitrary operator objectives do not collapse
    /// into one opaque execution prompt.
    pub fn decompose_grounded(
        &self,
        obj: &Objective,
        context_sources: Vec<ObjectiveContextSource>,
    ) -> ObjectivePlan {
        let mut tasks =
            vec![
            task(
                "recover-context",
                "Recover authoritative project, plan, evidence, receipt, and repository context",
                "context_recovery",
                &[],
                Priority::High,
                "vaire",
                30,
            ),
            task(
                "inspect-authorities",
                &format!("Inspect live behavior and authorities for: {}", obj.statement),
                "analysis",
                &["recover-context"],
                Priority::High,
                "prometheus",
                120,
            ),
            task(
                "synthesize-findings",
                "Synthesize evidence into prioritized findings and smallest authoritative repairs",
                "synthesis",
                &["inspect-authorities"],
                Priority::High,
                "prometheus",
                120,
            ),
            task(
                "produce-outcome",
                &format!("Produce the concrete operator-visible outcome for: {}", obj.statement),
                "ops",
                &["synthesize-findings"],
                Priority::Critical,
                "ceo",
                180,
            ),
            task(
                "verify-acceptance",
                "Verify objective acceptance criteria and evidence",
                "monitor",
                &["produce-outcome"],
                Priority::High,
                "warden",
                60,
            ),
        ];
        for planned in &mut tasks {
            let canonical = super::taxonomy::canonical(&planned.task_type);
            planned.joule_cost = self
                .base_costs
                .get(canonical)
                .or_else(|| self.base_costs.get(&planned.task_type))
                .copied()
                .unwrap_or(self.default_cost);
        }
        ObjectivePlan {
            objective_id: obj.id.clone(),
            tasks,
            context_sources,
            acceptance_criteria: obj.success_criteria.clone(),
            approval_required: true,
        }
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

    #[test]
    fn grounded_decomposition_preserves_context_and_acceptance() {
        let mut objective = obj("Review the system against the operator vision");
        objective.success_criteria =
            vec!["Produce a concrete prioritized repair backlog with source evidence".into()];
        let context = vec![
            ObjectiveContextSource::new("project_contract", "data/workbench/projects.json"),
            ObjectiveContextSource::new(
                "active_plan",
                "docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md",
            ),
            ObjectiveContextSource::new("repository_state", "git status --short"),
        ];

        let plan = ObjectiveDecomposer::default().decompose_grounded(&objective, context);

        assert_eq!(plan.objective_id, "o");
        assert_eq!(plan.context_sources.len(), 3);
        assert_eq!(plan.acceptance_criteria, objective.success_criteria);
        assert!(plan.approval_required);
        assert_eq!(
            plan.tasks.last().unwrap().title,
            "Verify objective acceptance criteria and evidence"
        );
        assert!(plan
            .tasks
            .last()
            .unwrap()
            .depends_on
            .contains(&"produce-outcome".to_string()));
    }

    #[test]
    fn grounded_decomposition_requires_evidence_sources() {
        let mut objective = obj("Review the system");
        objective.success_criteria = vec!["Produce an evidence-backed result".into()];

        let plan = ObjectiveDecomposer::default().decompose_grounded(&objective, Vec::new());

        assert!(plan.context_sources.is_empty());
        assert!(plan.tasks.iter().any(|task| task.key == "recover-context"));
    }
}
