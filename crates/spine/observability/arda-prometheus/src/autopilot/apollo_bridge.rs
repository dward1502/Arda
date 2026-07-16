// sigil: REPAIR
//! Apollo execution bridge — dispatches operational PlannedTasks through
//! `ApolloExecutor` (in-process) or an `ApolloDaemon` (IPC) and returns the
//! resulting status. For tasks that belong to a non-operational realm,
//! returns `Skipped` so the queue.jsonl path remains the delegation channel.

use super::decomposer::{PlannedTask, Priority};
use super::taxonomy::is_apollo_dispatchable;
use arda_aule::transport::ipc::send_command;
use arda_aule::{ApolloExecutor, ExecutionPriority, ExecutionRequest, ExecutionStatus};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

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

/// Dispatch transport for Apollo. The autopilot prefers the IPC daemon when
/// the socket exists so dispatches survive autopilot restarts; otherwise it
/// falls back to an embedded executor.
pub enum ApolloClient {
    InProcess(ApolloExecutor),
    Daemon { socket_path: PathBuf },
}

impl ApolloClient {
    /// Choose Daemon if the socket exists at construct time; otherwise InProcess.
    pub fn auto(socket_path: PathBuf) -> Self {
        if socket_path.exists() {
            Self::Daemon { socket_path }
        } else {
            Self::InProcess(ApolloExecutor::new())
        }
    }

    pub fn in_process() -> Self {
        Self::InProcess(ApolloExecutor::new())
    }

    pub fn daemon(socket_path: PathBuf) -> Self {
        Self::Daemon { socket_path }
    }

    pub fn transport_label(&self) -> &'static str {
        match self {
            Self::InProcess(_) => "in_process",
            Self::Daemon { .. } => "daemon",
        }
    }

    /// Re-evaluate the daemon socket and switch transports when runtime state changes.
    ///
    /// Returns true when the selected transport changed.
    pub fn refresh_transport(&mut self, socket_path: PathBuf) -> bool {
        match self {
            Self::InProcess(_) if socket_path.exists() => {
                *self = Self::Daemon { socket_path };
                true
            }
            Self::Daemon {
                socket_path: current,
            } if !current.exists() => {
                *self = Self::InProcess(ApolloExecutor::new());
                true
            }
            _ => false,
        }
    }
}

fn map_priority(p: Priority) -> ExecutionPriority {
    match p {
        Priority::Low => ExecutionPriority::Low,
        Priority::Medium => ExecutionPriority::Normal,
        Priority::High => ExecutionPriority::High,
        Priority::Critical => ExecutionPriority::Critical,
    }
}

fn priority_str(p: ExecutionPriority) -> &'static str {
    match p {
        ExecutionPriority::Low => "low",
        ExecutionPriority::Normal => "normal",
        ExecutionPriority::High => "high",
        ExecutionPriority::Critical => "critical",
    }
}

pub async fn dispatch(client: &ApolloClient, task_id: &str, plan: &PlannedTask) -> Dispatch {
    dispatch_with_conditions(client, task_id, plan, &[]).await
}

pub async fn dispatch_with_conditions(
    client: &ApolloClient,
    task_id: &str,
    plan: &PlannedTask,
    oracle_conditions: &[String],
) -> Dispatch {
    if !is_apollo_dispatchable(&plan.task_type) {
        return Dispatch::Skipped {
            reason: format!("non-apollo task_type: {}", plan.task_type),
        };
    }
    let agent = plan.assigned_agent.clone().unwrap_or_else(|| "ceo".into());
    let payload = json!({
        "title": plan.title, "task_type": plan.task_type,
        "depends_on": plan.depends_on, "eta_seconds": plan.eta_seconds,
        "joule_cost": plan.joule_cost,
        "oracle_conditions": oracle_conditions,
    });
    let priority = map_priority(plan.priority);
    let timeout_secs = plan.eta_seconds.max(30) * 2;

    match client {
        ApolloClient::InProcess(exec) => {
            let req = ExecutionRequest {
                task_id: task_id.into(),
                agent_id: agent,
                payload,
                priority,
                timeout_secs,
            };
            let id = exec.submit(req).await;
            match exec.execute(&id).await {
                Some(r) => Dispatch::Submitted {
                    task_id: r.task_id,
                    status: r.status,
                    joules: r.joule_work,
                    transport: "in_process",
                },
                None => Dispatch::Skipped {
                    reason: "executor returned no result".into(),
                },
            }
        }
        ApolloClient::Daemon { socket_path } => {
            let submit_payload = json!({
                "task_id": task_id,
                "agent_id": agent,
                "payload": payload,
                "priority": priority_str(priority),
                "timeout_secs": timeout_secs,
            });
            let returned_id =
                match send_command(socket_path.clone(), "submit", submit_payload).await {
                    Ok(v) => v
                        .get("task_id")
                        .and_then(|x| x.as_str())
                        .unwrap_or(task_id)
                        .to_string(),
                    Err(e) => {
                        return Dispatch::Skipped {
                            reason: format!("daemon submit failed: {e}"),
                        }
                    }
                };
            let exec_resp = match send_command(
                socket_path.clone(),
                "execute",
                json!({"task_id": returned_id}),
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    return Dispatch::Skipped {
                        reason: format!("daemon execute failed: {e}"),
                    }
                }
            };
            let result = exec_resp.get("result").cloned().unwrap_or(Value::Null);
            if result.is_null() {
                return Dispatch::Skipped {
                    reason: "daemon returned no result".into(),
                };
            }
            let status: ExecutionStatus = match serde_json::from_value(
                result.get("status").cloned().unwrap_or(Value::Null),
            ) {
                Ok(s) => s,
                Err(e) => {
                    return Dispatch::Skipped {
                        reason: format!("invalid status: {e}"),
                    }
                }
            };
            let joules = result
                .get("joule_work")
                .and_then(|j| j.as_f64())
                .unwrap_or(0.0);
            let rid = result
                .get("task_id")
                .and_then(|j| j.as_str())
                .unwrap_or(&returned_id)
                .to_string();
            Dispatch::Submitted {
                task_id: rid,
                status,
                joules,
                transport: "daemon",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::decomposer::PlannedTask;
    use super::*;

    fn ops_task() -> PlannedTask {
        PlannedTask {
            key: "k".into(),
            title: "ship".into(),
            task_type: "ops".into(),
            depends_on: vec![],
            priority: Priority::High,
            joule_cost: 5.0,
            eta_seconds: 30,
            assigned_agent: Some("ceo".into()),
        }
    }

    #[tokio::test]
    async fn dispatches_apollo_task_in_process() {
        let client = ApolloClient::in_process();
        let d = dispatch(&client, "tsk_abc", &ops_task()).await;
        match d {
            Dispatch::Submitted {
                status, transport, ..
            } => {
                assert_eq!(status, ExecutionStatus::Completed);
                assert_eq!(transport, "in_process");
            }
            _ => panic!("expected Submitted"),
        }
    }

    #[tokio::test]
    async fn skips_non_apollo_task() {
        let client = ApolloClient::in_process();
        let pt = PlannedTask {
            key: "k".into(),
            title: "x".into(),
            task_type: "research".into(),
            depends_on: vec![],
            priority: Priority::Medium,
            joule_cost: 1.0,
            eta_seconds: 10,
            assigned_agent: Some("athena".into()),
        };
        let d = dispatch(&client, "tsk_xyz", &pt).await;
        assert!(matches!(d, Dispatch::Skipped { .. }));
    }

    #[tokio::test]
    async fn auto_falls_back_to_in_process_when_socket_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.sock");
        let client = ApolloClient::auto(path);
        assert_eq!(client.transport_label(), "in_process");
    }

    #[tokio::test]
    async fn refresh_promotes_to_daemon_when_socket_appears() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("apollo.sock");
        let mut client = ApolloClient::auto(path.clone());
        assert_eq!(client.transport_label(), "in_process");
        std::fs::write(&path, "").unwrap();
        assert!(client.refresh_transport(path));
        assert_eq!(client.transport_label(), "daemon");
    }

    #[tokio::test]
    async fn refresh_falls_back_when_daemon_socket_disappears() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("apollo.sock");
        std::fs::write(&path, "").unwrap();
        let mut client = ApolloClient::auto(path.clone());
        assert_eq!(client.transport_label(), "daemon");
        std::fs::remove_file(&path).unwrap();
        assert!(client.refresh_transport(path));
        assert_eq!(client.transport_label(), "in_process");
    }

    #[tokio::test]
    async fn dispatches_apollo_task_via_daemon() {
        use arda_aule::transport::ipc::run_ipc_server;
        use arda_aule::ApolloService;
        use tokio::time::{sleep, Duration};

        let dir = tempfile::tempdir().unwrap();
        let service = ApolloService::from_home(dir.path()).expect("service");
        let socket_path = dir.path().join("apollo.sock");
        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        // Skip the test if the runner cannot bind a unix socket here (sandbox).
        if !socket_path.exists() {
            server.abort();
            return;
        }

        let client = ApolloClient::daemon(socket_path);
        let d = dispatch(&client, "tsk_daemon", &ops_task()).await;
        server.abort();

        match d {
            Dispatch::Submitted {
                status, transport, ..
            } => {
                assert_eq!(status, ExecutionStatus::Completed);
                assert_eq!(transport, "daemon");
            }
            Dispatch::Skipped { reason } => {
                // Sandbox may forbid unix socket connections — accept that as a skip.
                assert!(
                    reason.contains("Operation not permitted")
                        || reason.contains("Permission denied")
                        || reason.contains("connect")
                        || reason.contains("No such file"),
                    "unexpected daemon dispatch failure: {reason}",
                );
            }
        }
    }
}
