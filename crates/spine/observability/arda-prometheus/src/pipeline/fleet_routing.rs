use crate::orders::OrderStatus;
use crate::pipeline::Pipeline;
use Arda_core::error::Result;
use Arda_core::message::Message;
use Arda_core::task::{Task, TaskStatus};

impl Pipeline {
    pub(super) async fn try_fleet_or_external_route(
        &self,
        task: &mut Task,
    ) -> Result<Option<Task>> {
        let fleet_decision = if let Some(ref fleet) = self.fleet_manager {
            let decision = fleet.evaluate_task(task);
            self.ledger.append(&Message::event(
                "ceo",
                "fleet_routing_decision",
                serde_json::to_value(&decision).unwrap_or_default(),
            ))?;
            Some(decision)
        } else {
            None
        };

        if let Some(ref decision) = fleet_decision {
            if let Some(node_id) = decision.decision.node_id() {
                if decision.decision.is_accepted() {
                    if let Some(ref dispatcher) = self.edge_dispatcher {
                        tracing::info!(node = %node_id, "attempting fleet dispatch");
                        let task_json = serde_json::to_string(task).unwrap_or_default();
                        let dispatch_result = dispatcher
                            .dispatch_task(&decision.decision, &task.id.to_string(), &task_json)
                            .await;

                        match dispatch_result {
                            Ok(result) if result.is_success() => {
                                self.append_thought(
                                    "audit",
                                    "fleet_dispatch",
                                    &format!("Dispatched to {}", node_id),
                                );
                                return Ok(Some(task.clone()));
                            }
                            Ok(result) => {
                                tracing::warn!(node = %node_id, error = %result.stderr, "fleet dispatch failed");
                                self.append_thought(
                                    "concern",
                                    "fleet_dispatch_failed",
                                    &result.stderr,
                                );
                            }
                            Err(err) => {
                                tracing::warn!(node = %node_id, error = %err, "fleet dispatch error");
                                self.append_thought(
                                    "concern",
                                    "fleet_dispatch_error",
                                    &err.to_string(),
                                );
                            }
                        }
                    }
                }

                if !decision.routing_context.local_node_available {
                    let provider = &self.provider_tracker;
                    if let Some(provider_config) = provider.get_available_provider() {
                        self.append_thought(
                            "audit",
                            "external_provider_fallback",
                            &format!(
                                "Routing task {} to external provider {} ({})",
                                task.id, provider_config.id, provider_config.name
                            ),
                        );
                        self.ledger.append(&Message::event(
                            "ceo",
                            "external_provider_routing",
                            serde_json::json!({
                                "task_id": task.id,
                                "provider_id": provider_config.id,
                                "provider_name": provider_config.name,
                                "base_url": provider_config.base_url,
                                "reason": "fleet_capacity_exhausted"
                            }),
                        ))?;

                        let model = provider_config
                            .models
                            .iter()
                            .find(|m| m.is_default)
                            .map(|m| m.id.clone())
                            .unwrap_or_else(|| "opencode/deepseek-v4-flash-free".to_string());

                        let messages = vec![
                            serde_json::json!({
                                "role": "system",
                                "content": "You are Arda, an autonomous CEO agent system."
                            }),
                            serde_json::json!({
                                "role": "user",
                                "content": task.description
                            }),
                        ];

                        match provider
                            .call_api(&provider_config.id, &model, messages)
                            .await
                        {
                            Ok(response) => {
                                self.append_thought(
                                    "audit",
                                    "external_api_success",
                                    &format!(
                                        "External API call success for task {}: {} tokens used",
                                        task.id, response.usage.total_tokens
                                    ),
                                );

                                let response_content = response
                                    .choices
                                    .first()
                                    .map(|c| c.message.content.clone())
                                    .unwrap_or_default();

                                task.result = Some(serde_json::json!({
                                    "external_provider": provider_config.id,
                                    "model": model,
                                    "usage": response.usage,
                                    "response": response_content
                                }));

                                let _ = provider.write_snapshot();

                                return Ok(Some(task.clone()));
                            }
                            Err(err) => {
                                self.append_thought(
                                    "concern",
                                    "external_api_failed",
                                    &format!(
                                        "External API call failed for task {}: {}",
                                        task.id, err
                                    ),
                                );

                                let _ = provider.write_snapshot();
                            }
                        }
                    } else {
                        let _ = provider.write_snapshot();

                        task.transition(TaskStatus::Pending);
                        self.append_order(
                            task.id,
                            &task.task_type,
                            OrderStatus::Escalated,
                            None,
                            "fleet backpressure: no capacity, no external provider",
                        );
                        self.append_thought(
                            "concern",
                            "fleet_backpressure",
                            &format!(
                                "Task {} queued - fleet exhausted, no external provider available",
                                task.id
                            ),
                        );
                        self.emit_memory_event(
                            "fleet_backpressure",
                            &format!(
                                "Task {} held - fleet capacity exhausted, no external fallback",
                                task.id
                            ),
                            Some(0.1),
                            vec!["backpressure".to_string(), "no_fallback".to_string()],
                        );
                        return Ok(Some(task.clone()));
                    }
                }
            }
        }

        Ok(None)
    }
}
