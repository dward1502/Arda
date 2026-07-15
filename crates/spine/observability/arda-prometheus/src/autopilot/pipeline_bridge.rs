// sigil: REPAIR
//! Bridge approved autopilot plans into the legacy PROMETHEUS `Pipeline`
//! so its ledger/order/memory side effects stay populated while Apollo
//! remains the execution path for operational tasks.

use super::decomposer::{Objective, PlannedTask};
use super::oracle_gate::GateDecision;
use arda_core::agent::Agent;
use arda_core::error::Result;
use arda_core::ledger::Ledger;
use arda_core::router::Router;
use arda_core::task::Task;
use async_trait::async_trait;
use std::path::Path;

const AUTOPILOT_PIPELINE_CAPABILITIES: &[&str] = &["ceo_autopilot_plan"];

pub async fn submit_plan(
    root: impl AsRef<Path>,
    objective: &Objective,
    plan: &[PlannedTask],
    gate: &GateDecision,
) -> Result<Task> {
    let root = root.as_ref();
    let mut router = Router::new();
    router.register(Box::new(AutopilotPipelineAgent));
    let ledger = Ledger::new(root.join("data/ceo/pipeline_ledger"))?;
    let joule_budget = plan
        .iter()
        .map(|task| task.joule_cost.ceil() as u64)
        .sum::<u64>()
        .max(100);
    let pipeline = crate::Pipeline::new(router, ledger, joule_budget);
    let mut task = Task::new(
        format!(
            "CEO autopilot approved objective `{}` with {} planned tasks under gate {:?}",
            objective.statement,
            plan.len(),
            gate
        ),
        "ceo_autopilot_plan",
    );
    task.joule_cost_estimated = plan.iter().map(|step| step.joule_cost).sum();
    task.result = Some(serde_json::json!({
        "objective_id": objective.id,
        "planned_tasks": plan.len(),
        "gate": gate,
    }));
    pipeline.submit(task).await
}

struct AutopilotPipelineAgent;

#[async_trait]
impl Agent for AutopilotPipelineAgent {
    fn name(&self) -> &str {
        "ceo_autopilot"
    }

    fn capabilities(&self) -> &[&str] {
        AUTOPILOT_PIPELINE_CAPABILITIES
    }

    async fn execute(&self, task: &mut Task) -> Result<()> {
        task.start_execution();
        task.complete(serde_json::json!({
            "status": "completed",
            "executed_by": "ceo_autopilot_pipeline_bridge",
        }));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::decomposer::Priority;
    use super::*;
    use arda_core::task::TaskStatus;

    #[tokio::test]
    async fn submits_approved_plan_to_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let objective = Objective {
            id: "obj".into(),
            statement: "keep pipeline ledger populated".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        let plan = vec![PlannedTask {
            key: "a".into(),
            title: "A".into(),
            task_type: "ops".into(),
            depends_on: vec![],
            priority: Priority::High,
            joule_cost: 2.0,
            eta_seconds: 30,
            assigned_agent: Some("ceo".into()),
        }];

        let task = submit_plan(
            dir.path(),
            &objective,
            &plan,
            &GateDecision::Approved { resonance: 1.0 },
        )
        .await
        .expect("pipeline submit");
        assert!(matches!(task.status, TaskStatus::Complete));
        assert!(dir.path().join("data/ceo/pipeline_ledger").exists());
    }
}
