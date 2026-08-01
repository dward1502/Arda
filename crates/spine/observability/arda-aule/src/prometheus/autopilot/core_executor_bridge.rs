#![cfg(feature = "full-cli")]
//! Adapter from CEO autopilot plans to the canonical Arda task queue.
//!
//! Queue append happens before this adapter is called. The adapter therefore
//! acknowledges handoff as pending; it never fabricates execution completion.

use super::decomposer::PlannedTask;
use super::taxonomy::is_apollo_dispatchable;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, Serialize)]
pub enum Dispatch {
    Skipped {
        reason: String,
    },
    Submitted {
        task_id: String,
        status: ExecutionStatus,
        joules: f64,
        transport: &'static str,
    },
}

/// The current executor adapter. Operational plans remain on Arda's canonical
/// task queue for the active loop/executor authority to claim.
pub struct CoreExecutorClient;

impl CoreExecutorClient {
    pub fn auto(_retired_socket_path: PathBuf) -> Self {
        Self
    }

    pub fn in_process() -> Self {
        Self
    }

    pub fn transport_label(&self) -> &'static str {
        "arda_core_queue"
    }

    pub fn refresh_transport(&mut self, _retired_socket_path: PathBuf) -> bool {
        false
    }
}

pub async fn dispatch(client: &CoreExecutorClient, task_id: &str, plan: &PlannedTask) -> Dispatch {
    dispatch_with_conditions(client, task_id, plan, &[]).await
}

pub async fn dispatch_with_conditions(
    _client: &CoreExecutorClient,
    task_id: &str,
    plan: &PlannedTask,
    _oracle_conditions: &[String],
) -> Dispatch {
    if !is_apollo_dispatchable(&plan.task_type) {
        return Dispatch::Skipped {
            reason: format!("non-operational task_type: {}", plan.task_type),
        };
    }

    Dispatch::Submitted {
        task_id: task_id.to_string(),
        status: ExecutionStatus::Pending,
        joules: 0.0,
        transport: "arda_core_queue",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::autopilot::decomposer::Priority;

    fn operational_task() -> PlannedTask {
        PlannedTask {
            key: "k".into(),
            title: "observe runtime".into(),
            task_type: "monitor".into(),
            depends_on: vec![],
            priority: Priority::High,
            joule_cost: 5.0,
            eta_seconds: 30,
            assigned_agent: Some("ceo".into()),
        }
    }

    #[tokio::test]
    async fn operational_plan_remains_pending_for_canonical_core_queue() {
        let client = CoreExecutorClient::in_process();
        let dispatch = dispatch(&client, "tsk_core", &operational_task()).await;
        assert!(matches!(
            dispatch,
            Dispatch::Submitted {
                status: ExecutionStatus::Pending,
                transport: "arda_core_queue",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn non_operational_plan_remains_queued_for_another_authority() {
        let client = CoreExecutorClient::in_process();
        let mut task = operational_task();
        task.task_type = "research".into();
        assert!(matches!(
            dispatch(&client, "tsk_research", &task).await,
            Dispatch::Skipped { .. }
        ));
    }
}
