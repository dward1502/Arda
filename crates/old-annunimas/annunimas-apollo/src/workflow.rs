// sigil: REPAIR
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub status: WorkflowStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub action: String,
    pub payload: serde_json::Value,
    pub dependencies: Vec<String>,
    pub retry_count: u32,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Paused,
}

impl Workflow {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            steps: Vec::new(),
            status: WorkflowStatus::Pending,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn validate(&self) -> Result<(), WorkflowError> {
        let step_ids: HashSet<&str> = self.steps.iter().map(|s| s.id.as_str()).collect();

        for step in &self.steps {
            for dep in &step.dependencies {
                if !step_ids.contains(dep.as_str()) {
                    return Err(WorkflowError::InvalidDependency(
                        step.id.clone(),
                        dep.clone(),
                    ));
                }
            }
        }

        if self.steps.is_empty() {
            return Err(WorkflowError::EmptyWorkflow);
        }

        Ok(())
    }

    pub fn ready_steps(&self, completed: &HashSet<String>) -> Vec<&WorkflowStep> {
        self.steps
            .iter()
            .filter(|s| !completed.contains(&s.id))
            .filter(|s| s.dependencies.iter().all(|d| completed.contains(d)))
            .collect()
    }

    pub fn is_complete(&self, completed: &HashSet<String>) -> bool {
        self.steps.iter().all(|s| completed.contains(&s.id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowError {
    EmptyWorkflow,
    InvalidDependency(String, String),
    StepFailed(String),
    Timeout,
}

pub struct WorkflowEngine {
    workflows: HashMap<String, Workflow>,
    step_results: HashMap<String, serde_json::Value>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            step_results: HashMap::new(),
        }
    }

    pub fn register(&mut self, workflow: Workflow) -> Result<(), WorkflowError> {
        workflow.validate()?;
        self.workflows.insert(workflow.id.clone(), workflow);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Workflow> {
        self.workflows.get(id)
    }

    pub fn execute(
        &mut self,
        workflow_id: &str,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        let workflow = self
            .workflows
            .get_mut(workflow_id)
            .ok_or_else(|| WorkflowError::StepFailed("Workflow not found".to_string()))?;

        workflow.status = WorkflowStatus::Running;

        let mut completed: HashSet<String> = HashSet::new();
        let mut results: HashMap<String, serde_json::Value> = HashMap::new();

        let max_iterations = workflow.steps.len() * 2;
        let mut iterations = 0;

        while !workflow.is_complete(&completed) && iterations < max_iterations {
            iterations += 1;

            let ready = workflow.ready_steps(&completed);

            if ready.is_empty() {
                if !completed.is_empty() && !workflow.is_complete(&completed) {
                    return Err(WorkflowError::StepFailed(
                        "Deadlock - no ready steps".to_string(),
                    ));
                }
                break;
            }

            for step in ready {
                let result = execute_step_impl(step);
                results.insert(step.id.clone(), result.clone());
                self.step_results.insert(step.id.clone(), result);
                completed.insert(step.id.clone());
            }
        }

        if workflow.is_complete(&completed) {
            workflow.status = WorkflowStatus::Completed;
            Ok(results)
        } else {
            workflow.status = WorkflowStatus::Failed;
            Err(WorkflowError::StepFailed("Workflow incomplete".to_string()))
        }
    }
}

fn execute_step_impl(step: &WorkflowStep) -> serde_json::Value {
    serde_json::json!({
        "step_id": step.id,
        "step_name": step.name,
        "action": step.action,
        "executed": true,
    })
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Workflow, WorkflowEngine, WorkflowError, WorkflowStatus, WorkflowStep};
    use serde_json::json;
    use std::collections::HashSet;

    fn step(id: &str, deps: &[&str]) -> WorkflowStep {
        WorkflowStep {
            id: id.to_string(),
            name: format!("step-{id}"),
            action: "run".to_string(),
            payload: json!({"id": id}),
            dependencies: deps.iter().map(|dep| dep.to_string()).collect(),
            retry_count: 0,
            max_retries: 1,
        }
    }

    #[test]
    fn validate_rejects_empty_and_missing_dependencies() {
        let empty = Workflow::new("empty");
        assert!(matches!(
            empty.validate(),
            Err(WorkflowError::EmptyWorkflow)
        ));

        let invalid = Workflow::new("invalid").add_step(step("a", &["missing"]));
        assert!(matches!(
            invalid.validate(),
            Err(WorkflowError::InvalidDependency(step_id, dep))
                if step_id == "a" && dep == "missing"
        ));
    }

    #[test]
    fn ready_steps_follow_dependency_completion() {
        let workflow = Workflow::new("deps")
            .add_step(step("a", &[]))
            .add_step(step("b", &["a"]))
            .add_step(step("c", &["a", "b"]));

        let none_complete = HashSet::new();
        let ready = workflow.ready_steps(&none_complete);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "a");

        let completed = HashSet::from(["a".to_string()]);
        let ready = workflow.ready_steps(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "b");
    }

    #[test]
    fn engine_executes_registered_workflow_and_marks_complete() {
        let workflow = Workflow::new("happy-path")
            .add_step(step("a", &[]))
            .add_step(step("b", &["a"]));
        let workflow_id = workflow.id.clone();

        let mut engine = WorkflowEngine::new();
        engine.register(workflow).expect("register");

        let results = engine.execute(&workflow_id).expect("execute");
        assert_eq!(results.len(), 2);
        assert_eq!(results["a"]["executed"], true);
        assert_eq!(results["b"]["step_name"], "step-b");
        assert_eq!(
            engine.get(&workflow_id).expect("workflow").status,
            WorkflowStatus::Completed
        );
    }
}
