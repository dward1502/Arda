//! Canonical, read-only operator projection shared by phone, CLI/API, HUD,
//! voice, and optional outposts.
//!
//! The projection carries canonical identifiers and authority meaning. A
//! transport may reformat it, but no projection consumer may transition state.

use crate::run_graph::{NodeKind, NodeState, WorkerRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionAuthority {
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionFreshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyHealth {
    NotConfigured,
    Unavailable,
    Degraded,
    Stale,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Pending,
    Active,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Blocked,
    AwaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Pending,
    Delivered,
    Deferred,
    Acknowledged,
    Dismissed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementSource {
    Observed,
    Estimated,
    DefaultFallback,
    SyntheticRestoration,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcknowledgementStatus {
    NotRequired,
    Pending,
    Acknowledged,
    Deferred,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveBudgetProjection {
    pub max_joules: f64,
    pub max_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveProjection {
    pub objective_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub status: ObjectiveStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_continuation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_wake_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<ObjectiveBudgetProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
}

impl ObjectiveProjection {
    /// Creates the stable required objective identity while defaulting additive
    /// read-only control fields for Rust consumers migrating from the v1 shape.
    #[must_use]
    pub fn new(
        objective_id: impl Into<String>,
        project_id: Option<String>,
        title: impl Into<String>,
        status: ObjectiveStatus,
    ) -> Self {
        Self {
            objective_id: objective_id.into(),
            project_id,
            title: title.into(),
            status,
            current_task_id: None,
            current_run_id: None,
            current_node_id: None,
            evidence: Vec::new(),
            next_continuation: None,
            next_wake_at: None,
            provider_route: None,
            budget: None,
            blocker: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeProjection {
    pub node_id: String,
    pub kind: NodeKind,
    pub state: NodeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerProjection {
    pub node_id: String,
    pub role: WorkerRole,
    pub worker_id: String,
    pub route_id: String,
    pub state: NodeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunProjection {
    pub run_id: String,
    pub objective_id: String,
    pub status: RunStatus,
    pub nodes: Vec<NodeProjection>,
    pub workers: Vec<WorkerProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityProjection {
    pub capability_id: String,
    pub version: String,
    pub health: DependencyHealth,
    pub selected: bool,
    pub optional: bool,
    pub selection_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingApprovalProjection {
    pub approval_id: String,
    pub run_id: String,
    pub node_id: Option<String>,
    pub scope: String,
    pub action_digest: String,
    pub expires_at: DateTime<Utc>,
    pub status: ApprovalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilProjection {
    pub council_id: String,
    pub run_id: String,
    pub state: String,
    pub synthesis: String,
    pub material_tensions: Vec<String>,
    pub non_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReminderProjection {
    pub reminder_id: String,
    pub item_id: String,
    pub status: ReminderStatus,
    pub next_due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalOperationsProjection {
    pub captures: usize,
    pub resumable_items: usize,
    pub reminders: Vec<ReminderProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JouleWorkProjection {
    pub budget_joules: f64,
    pub consumed_joules: f64,
    pub remaining_joules: f64,
    pub source: MeasurementSource,
    pub source_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProjection {
    pub evidence_id: String,
    pub kind: String,
    pub uri: String,
    pub observed_at: DateTime<Utc>,
    pub freshness: ProjectionFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommunicationProjection {
    pub communication_id: String,
    pub transport: String,
    pub delivery: DeliveryStatus,
    pub acknowledgement: AcknowledgementStatus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyProjection {
    pub dependency_id: String,
    pub health: DependencyHealth,
    pub freshness: ProjectionFreshness,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorProjection {
    pub schema_version: String,
    pub projection_id: String,
    pub generated_at: DateTime<Utc>,
    pub authority: ProjectionAuthority,
    pub freshness: ProjectionFreshness,
    pub objectives: Vec<ObjectiveProjection>,
    pub runs: Vec<RunProjection>,
    pub capabilities: Vec<CapabilityProjection>,
    pub pending_approvals: Vec<PendingApprovalProjection>,
    pub councils: Vec<CouncilProjection>,
    pub personal_operations: PersonalOperationsProjection,
    pub joulework: JouleWorkProjection,
    pub evidence: Vec<EvidenceProjection>,
    pub communications: Vec<CommunicationProjection>,
    pub dependencies: Vec<DependencyProjection>,
}

impl OperatorProjection {
    pub const SCHEMA_VERSION: &'static str = "arda.operator-projection.v1";

    pub fn from_json_str(raw: &str) -> Result<Self, OperatorProjectionError> {
        let projection: Self = serde_json::from_str(raw)
            .map_err(|error| OperatorProjectionError::InvalidJson(error.to_string()))?;
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> Result<(), OperatorProjectionError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(OperatorProjectionError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        require_text("projection_id", &self.projection_id)?;

        unique_ids(
            "objective",
            self.objectives
                .iter()
                .map(|item| item.objective_id.as_str()),
        )?;
        unique_ids("run", self.runs.iter().map(|item| item.run_id.as_str()))?;
        unique_ids(
            "capability",
            self.capabilities
                .iter()
                .map(|item| item.capability_id.as_str()),
        )?;
        unique_ids(
            "approval",
            self.pending_approvals
                .iter()
                .map(|item| item.approval_id.as_str()),
        )?;
        unique_ids(
            "council",
            self.councils.iter().map(|item| item.council_id.as_str()),
        )?;
        unique_ids(
            "evidence",
            self.evidence.iter().map(|item| item.evidence_id.as_str()),
        )?;
        unique_ids(
            "communication",
            self.communications
                .iter()
                .map(|item| item.communication_id.as_str()),
        )?;
        unique_ids(
            "dependency",
            self.dependencies
                .iter()
                .map(|item| item.dependency_id.as_str()),
        )?;

        let objective_ids = self
            .objectives
            .iter()
            .map(|item| item.objective_id.as_str())
            .collect::<HashSet<_>>();
        let run_ids = self
            .runs
            .iter()
            .map(|item| item.run_id.as_str())
            .collect::<HashSet<_>>();

        for objective in &self.objectives {
            require_text("objective_id", &objective.objective_id)?;
            require_text("objective.title", &objective.title)?;
            if let Some(task_id) = &objective.current_task_id {
                require_text("objective.current_task_id", task_id)?;
            }
            if let Some(run_id) = &objective.current_run_id {
                if !self
                    .runs
                    .iter()
                    .any(|run| run.run_id == *run_id && run.objective_id == objective.objective_id)
                {
                    return Err(OperatorProjectionError::MissingReference {
                        lane: format!("objective:{}", objective.objective_id),
                        field: "current_run_id".to_string(),
                        id: run_id.clone(),
                    });
                }
            }
            if let Some(node_id) = &objective.current_node_id {
                require_text("objective.current_node_id", node_id)?;
                let current_run = objective
                    .current_run_id
                    .as_deref()
                    .and_then(|run_id| self.runs.iter().find(|run| run.run_id == run_id));
                if !current_run
                    .is_some_and(|run| run.nodes.iter().any(|node| node.node_id == *node_id))
                {
                    return Err(OperatorProjectionError::MissingReference {
                        lane: format!("objective:{}", objective.objective_id),
                        field: "current_node_id".to_string(),
                        id: node_id.clone(),
                    });
                }
            }
            if let Some(route) = &objective.provider_route {
                require_text("objective.provider_route", route)?;
            }
            if let Some(blocker) = &objective.blocker {
                require_text("objective.blocker", blocker)?;
            }
            if let Some(budget) = &objective.budget {
                validate_non_negative("objective.budget.max_joules", budget.max_joules)?;
                validate_non_negative("objective.budget.max_cost_usd", budget.max_cost_usd)?;
            }
        }
        for run in &self.runs {
            if !objective_ids.contains(run.objective_id.as_str()) {
                return Err(OperatorProjectionError::MissingReference {
                    lane: format!("run:{}", run.run_id),
                    field: "objective_id".to_string(),
                    id: run.objective_id.clone(),
                });
            }
            unique_ids(
                &format!("run:{}:node", run.run_id),
                run.nodes.iter().map(|node| node.node_id.as_str()),
            )?;
            let node_ids = run
                .nodes
                .iter()
                .map(|node| node.node_id.as_str())
                .collect::<HashSet<_>>();
            for worker in &run.workers {
                if !node_ids.contains(worker.node_id.as_str()) {
                    return Err(OperatorProjectionError::MissingReference {
                        lane: format!("run:{}:worker", run.run_id),
                        field: "node_id".to_string(),
                        id: worker.node_id.clone(),
                    });
                }
                require_text("worker_id", &worker.worker_id)?;
                require_text("route_id", &worker.route_id)?;
            }
        }
        for capability in &self.capabilities {
            require_text("capability_id", &capability.capability_id)?;
            require_text("capability.version", &capability.version)?;
            if capability.selected && capability.selection_reasons.is_empty() {
                return Err(OperatorProjectionError::MissingSelectionReason(
                    capability.capability_id.clone(),
                ));
            }
        }
        for approval in &self.pending_approvals {
            if !run_ids.contains(approval.run_id.as_str()) {
                return Err(OperatorProjectionError::MissingReference {
                    lane: format!("approval:{}", approval.approval_id),
                    field: "run_id".to_string(),
                    id: approval.run_id.clone(),
                });
            }
            require_text("approval.scope", &approval.scope)?;
            require_text("approval.action_digest", &approval.action_digest)?;
        }
        for council in &self.councils {
            if !run_ids.contains(council.run_id.as_str()) {
                return Err(OperatorProjectionError::MissingReference {
                    lane: format!("council:{}", council.council_id),
                    field: "run_id".to_string(),
                    id: council.run_id.clone(),
                });
            }
            if !council.non_approval {
                return Err(OperatorProjectionError::CouncilClaimsApproval(
                    council.council_id.clone(),
                ));
            }
        }

        validate_non_negative("budget_joules", self.joulework.budget_joules)?;
        validate_non_negative("consumed_joules", self.joulework.consumed_joules)?;
        validate_non_negative("remaining_joules", self.joulework.remaining_joules)?;
        if !(0.0..=1.0).contains(&self.joulework.source_confidence) {
            return Err(OperatorProjectionError::InvalidConfidence {
                lane: "joulework".to_string(),
                value: self.joulework.source_confidence.to_string(),
            });
        }
        let expected_remaining =
            (self.joulework.budget_joules - self.joulework.consumed_joules).max(0.0);
        if (expected_remaining - self.joulework.remaining_joules).abs() > f64::EPSILON {
            return Err(OperatorProjectionError::InvalidBudgetBalance);
        }

        for dependency in &self.dependencies {
            if matches!(dependency.health, DependencyHealth::Stale)
                && dependency.freshness == ProjectionFreshness::Fresh
            {
                return Err(OperatorProjectionError::InconsistentFreshness {
                    lane: format!("dependency:{}", dependency.dependency_id),
                    health: dependency.health,
                    freshness: dependency.freshness,
                });
            }
        }
        Ok(())
    }
}

fn require_text(field: &str, value: &str) -> Result<(), OperatorProjectionError> {
    if value.trim().is_empty() {
        return Err(OperatorProjectionError::EmptyField(field.to_string()));
    }
    Ok(())
}

fn unique_ids<'a>(
    lane: &str,
    ids: impl Iterator<Item = &'a str>,
) -> Result<(), OperatorProjectionError> {
    let mut seen = HashSet::new();
    for id in ids {
        require_text(&format!("{lane}.id"), id)?;
        if !seen.insert(id) {
            return Err(OperatorProjectionError::DuplicateIdentifier {
                lane: lane.to_string(),
                id: id.to_string(),
            });
        }
    }
    Ok(())
}

fn validate_non_negative(field: &str, value: f64) -> Result<(), OperatorProjectionError> {
    if !value.is_finite() || value < 0.0 {
        return Err(OperatorProjectionError::InvalidNumber {
            field: field.to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum OperatorProjectionError {
    #[error("invalid operator projection JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported operator projection schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("operator projection field is empty: {0}")]
    EmptyField(String),
    #[error("duplicate {lane} identifier: {id}")]
    DuplicateIdentifier { lane: String, id: String },
    #[error("{lane} references missing {field}: {id}")]
    MissingReference {
        lane: String,
        field: String,
        id: String,
    },
    #[error("selected capability lacks a selection reason: {0}")]
    MissingSelectionReason(String),
    #[error("council projection claims approval authority: {0}")]
    CouncilClaimsApproval(String),
    #[error("invalid projection confidence for {lane}: {value}")]
    InvalidConfidence { lane: String, value: String },
    #[error("invalid numeric field {field}: {value}")]
    InvalidNumber { field: String, value: String },
    #[error("JouleWork remaining budget does not match budget minus consumption")]
    InvalidBudgetBalance,
    #[error("inconsistent freshness for {lane}: health={health:?}, freshness={freshness:?}")]
    InconsistentFreshness {
        lane: String,
        health: DependencyHealth,
        freshness: ProjectionFreshness,
    },
}
