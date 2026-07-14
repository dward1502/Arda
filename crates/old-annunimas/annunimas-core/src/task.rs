// sigil: REPAIR
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum JouleWorkMeasurementSource {
    OperatorEstimate,
    DefaultFallback,
    RuntimeTimer,
    ProcessResourceSample,
    ProviderUsageReport,
    ExternalPowerMeter,
}

impl Default for JouleWorkMeasurementSource {
    fn default() -> Self {
        Self::DefaultFallback
    }
}

impl JouleWorkMeasurementSource {
    pub fn is_observed(self) -> bool {
        matches!(
            self,
            Self::RuntimeTimer
                | Self::ProcessResourceSample
                | Self::ProviderUsageReport
                | Self::ExternalPowerMeter
        )
    }

    pub fn is_autonomy_truth(self) -> bool {
        !matches!(self, Self::DefaultFallback)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Complete,
    Failed { reason: String },
    Retry { attempt: u32, max_attempts: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assigned_agent: Option<String>,
    pub result: Option<serde_json::Value>,

    // Phi harmonic inputs
    pub planning_started_at: Option<DateTime<Utc>>,
    pub execution_started_at: Option<DateTime<Utc>>,
    pub joule_cost_estimated: f64,
    pub joule_cost_actual: f64,
    #[serde(default)]
    pub joulework_measurement_source: JouleWorkMeasurementSource,
    #[serde(default)]
    pub joulework_measurement_confidence: f64,
    pub clarifications_requested: u32,
    pub clarifications_resolved: u32,

    // Phase 1 contract: link a Task back to the Plan that produced it
    // so the Reflector can score it. Optional and serde-skipped when
    // None so existing callers and on-disk records stay unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_step_index: Option<usize>,
}

impl Task {
    pub fn new(description: impl Into<String>, task_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            task_type: task_type.into(),
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            assigned_agent: None,
            result: None,
            // Phi harmonic fields
            planning_started_at: None,
            execution_started_at: None,
            joule_cost_estimated: 0.0,
            joule_cost_actual: 0.0,
            joulework_measurement_source: JouleWorkMeasurementSource::DefaultFallback,
            joulework_measurement_confidence: 0.0,
            clarifications_requested: 0,
            clarifications_resolved: 0,
            plan_id: None,
            plan_step_index: None,
        }
    }

    pub fn transition(&mut self, new_status: TaskStatus) {
        self.status = new_status;
        self.updated_at = Utc::now();
    }

    /// Attach a planner-issued lineage so the Reflector can later
    /// score this Task against the Plan step that produced it.
    pub fn with_plan_lineage(mut self, plan_id: impl Into<String>, step_index: usize) -> Self {
        self.plan_id = Some(plan_id.into());
        self.plan_step_index = Some(step_index);
        self
    }

    pub fn assign(&mut self, agent_name: impl Into<String>) {
        self.assigned_agent = Some(agent_name.into());
        self.planning_started_at = Some(Utc::now());
        self.transition(TaskStatus::Running);
    }

    pub fn start_execution(&mut self) {
        self.execution_started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, result: serde_json::Value) {
        self.result = Some(result);
        self.transition(TaskStatus::Complete);
    }

    pub fn fail(&mut self, reason: impl Into<String>) {
        self.transition(TaskStatus::Failed {
            reason: reason.into(),
        });
    }

    // Phi harmonic helper methods
    pub fn planning_duration_secs(&self) -> f64 {
        match (self.planning_started_at, self.execution_started_at) {
            (Some(start), Some(end)) => (end - start).num_milliseconds() as f64 / 1000.0,
            (Some(start), None) => (Utc::now() - start).num_milliseconds() as f64 / 1000.0,
            _ => 0.0,
        }
    }

    pub fn execution_duration_secs(&self) -> f64 {
        match (self.execution_started_at, self.updated_at) {
            (Some(start), end) => (end - start).num_milliseconds() as f64 / 1000.0,
            _ => 0.0,
        }
    }

    pub fn calculate_resonance(&self) -> f64 {
        let delta = self.updated_at - self.created_at;
        let base = delta.num_seconds() as f64 / 60.0;
        let factor = match self.status {
            TaskStatus::Complete => 0.9,
            TaskStatus::Running => 0.5,
            _ => 0.2,
        };
        (base * factor).min(1.0) * 100.0
    }
}
