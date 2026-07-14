use crate::pipeline::Pipeline;
use annunimas_core::error::Result;
use annunimas_core::message::Message;
use annunimas_core::task::Task;
use annunimas_governance::record_bacon_lite;

impl Pipeline {
    pub(super) async fn route_and_execute_locally(
        &self,
        task: &mut Task,
        confidence: f64,
    ) -> Result<()> {
        match self.router.route(task) {
            Ok(agent) => {
                let agent_name = agent.name().to_string();
                task.assign(&agent_name);
                let task_id = task.id;
                self.ledger
                    .append(&Message::task_assignment(task_id, &agent_name))?;
                self.append_order(
                    task_id,
                    &task.task_type,
                    crate::orders::OrderStatus::Assigned,
                    Some(&agent_name),
                    "task delegated",
                );
                self.append_thought(
                    "audit",
                    "task_delegation",
                    &format!(
                        "Delegated task {} [{}] to {} with confidence {:.2}.",
                        task.id, task.task_type, agent_name, confidence
                    ),
                );
                self.emit_memory_event(
                    "task_delegated",
                    &format!(
                        "Delegated task {} [{}] to {} because routing confidence was {:.2}",
                        task.id, task.task_type, agent_name, confidence
                    ),
                    Some(confidence),
                    vec![
                        "delegation".to_string(),
                        "checkpoint".to_string(),
                        "decision".to_string(),
                        task.task_type.clone(),
                        agent_name.clone(),
                    ],
                );

                match agent.execute(task).await {
                    Ok(()) => {
                        let completion_result = match task.result.clone() {
                            Some(value) if !value.is_null() => value,
                            _ => {
                                let value = serde_json::json!({"status": "completed"});
                                task.result = Some(value.clone());
                                value
                            }
                        };
                        self.ledger.append(&Message::task_complete(
                            task_id,
                            &agent_name,
                            completion_result,
                        ))?;
                        self.append_order(
                            task_id,
                            &task.task_type,
                            crate::orders::OrderStatus::Complete,
                            Some(&agent_name),
                            "task completed",
                        );
                        self.append_thought(
                            "reflection",
                            "task_complete",
                            &format!("Task {} completed by {}.", task_id, agent_name),
                        );
                        self.emit_memory_event(
                            "task_completed",
                            &format!(
                                "Task {} completed by {} because delegated execution finished successfully",
                                task_id, agent_name
                            ),
                            Some(0.9),
                            vec![
                                "completion".to_string(),
                                "checkpoint".to_string(),
                                agent_name.clone(),
                            ],
                        );
                        if let Err(err) = record_bacon_lite(
                            "prometheus",
                            "task_completed",
                            task,
                            serde_json::json!({
                                "agent": agent_name,
                                "confidence": confidence,
                            }),
                        ) {
                            tracing::debug!(
                                error = %err,
                                "PROMETHEUS bacon-lite completion record failed"
                            );
                        }
                    }
                    Err(e) => {
                        let reason = e.to_string();
                        task.fail(&reason);
                        self.ledger
                            .append(&Message::task_failed(task_id, &agent_name, &reason))?;
                        self.append_order(
                            task_id,
                            &task.task_type,
                            crate::orders::OrderStatus::Failed,
                            Some(&agent_name),
                            &reason,
                        );
                        self.append_thought(
                            "concern",
                            "task_failed",
                            &format!("Task {} failed under {}: {}", task_id, agent_name, reason),
                        );
                        self.emit_memory_event(
                            "task_failed",
                            &format!(
                                "Task {} failed under {} because {}",
                                task_id, agent_name, reason
                            ),
                            Some(0.4),
                            vec![
                                "failure".to_string(),
                                "checkpoint".to_string(),
                                agent_name.clone(),
                            ],
                        );
                        if let Err(err) = record_bacon_lite(
                            "prometheus",
                            "task_failed",
                            task,
                            serde_json::json!({
                                "agent": agent_name,
                                "reason": reason,
                            }),
                        ) {
                            tracing::debug!(
                                error = %err,
                                "PROMETHEUS bacon-lite failure record failed"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                task.fail(format!("No route: {}", e));
                self.append_order(
                    task.id,
                    &task.task_type,
                    crate::orders::OrderStatus::Failed,
                    None,
                    "no route available",
                );
                self.append_thought(
                    "question",
                    "routing",
                    &format!(
                        "No route available for task {} [{}].",
                        task.id, task.task_type
                    ),
                );
                self.emit_memory_event(
                    "routing_failure",
                    &format!(
                        "No route for task {} [{}] because no eligible agent accepted the workload",
                        task.id, task.task_type
                    ),
                    Some(0.35),
                    vec![
                        "routing".to_string(),
                        "failure".to_string(),
                        "checkpoint".to_string(),
                    ],
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use annunimas_core::agent::Agent;
    use annunimas_core::ledger::Ledger;
    use annunimas_core::router::Router;
    use annunimas_core::task::{Task, TaskStatus};
    use async_trait::async_trait;
    use tempfile::tempdir;

    struct FakeAgent;

    #[async_trait]
    impl Agent for FakeAgent {
        fn name(&self) -> &str {
            "athena"
        }

        fn capabilities(&self) -> &[&str] {
            &["research"]
        }

        async fn execute(&self, task: &mut Task) -> Result<()> {
            task.complete(serde_json::json!({"handled_by": "athena"}));
            Ok(())
        }
    }

    fn test_pipeline() -> Pipeline {
        let dir = tempdir().expect("tempdir");
        let path = dir.keep();
        let ledger = Ledger::new(&path).expect("ledger");
        let router = Router::new();
        Pipeline::new(router, ledger, 100)
    }

    fn routed_pipeline() -> Pipeline {
        let dir = tempdir().expect("tempdir");
        let path = dir.keep();
        let ledger = Ledger::new(&path).expect("ledger");
        let mut router = Router::new();
        router.register(Box::new(FakeAgent));
        Pipeline::new(router, ledger, 100)
    }

    #[tokio::test]
    async fn local_execution_marks_task_failed_when_no_route_exists() {
        let pipeline = test_pipeline();
        let mut task = Task::new("unhandled task", "no_such_capability");

        pipeline
            .route_and_execute_locally(&mut task, 0.82)
            .await
            .expect("execution");

        assert!(matches!(task.status, TaskStatus::Failed { .. }));
        assert!(task.assigned_agent.is_none());
    }

    #[tokio::test]
    async fn local_execution_delegates_to_routable_agent_and_completes_task() {
        let pipeline = routed_pipeline();
        let mut task = Task::new("research task", "research");

        pipeline
            .route_and_execute_locally(&mut task, 0.91)
            .await
            .expect("execution");

        assert!(matches!(task.status, TaskStatus::Complete));
        assert_eq!(task.assigned_agent.as_deref(), Some("athena"));
        assert_eq!(
            task.result,
            Some(serde_json::json!({"handled_by": "athena"}))
        );
    }
}
