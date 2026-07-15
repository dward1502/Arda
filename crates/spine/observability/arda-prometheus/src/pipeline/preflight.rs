use crate::council::run_council_gate;
use crate::orders::OrderStatus;
use crate::pipeline::Pipeline;
use crate::service::PrometheusService;
use arda_core::error::Result;
use arda_core::message::Message;
use arda_core::task::{Task, TaskStatus};
use arda_governance::record_bacon_lite;

impl Pipeline {
    pub(super) fn record_task_intake(&self, task: &Task) -> Result<()> {
        self.append_order(
            task.id,
            &task.task_type,
            OrderStatus::Open,
            None,
            "task received",
        );
        self.append_thought(
            "audit",
            "task_received",
            &format!(
                "Received task {} [{}] with heartbeat mode {:?}",
                task.id, task.task_type, self.heartbeat.mode
            ),
        );
        self.emit_memory_event(
            "task_received",
            &format!(
                "PROMETHEUS received task {} [{}] because {}",
                task.id, task.task_type, task.description
            ),
            None,
            vec![
                "prometheus".to_string(),
                task.task_type.clone(),
                "checkpoint".to_string(),
            ],
        );

        self.ledger.append(&Message::event(
            "ceo",
            "task_received",
            serde_json::to_value(task)?,
        ))?;
        if let Err(err) = record_bacon_lite(
            "prometheus",
            "task_received",
            task,
            serde_json::json!({
                "heartbeat_mode": self.heartbeat.mode.to_string(),
                "confidence_threshold": self.confidence_threshold,
            }),
        ) {
            tracing::debug!(error = %err, "PROMETHEUS bacon-lite task_received record failed");
        }

        if let Some(profile) = &self.autonomy {
            self.ledger.append(&Message::event(
                "ceo",
                "core_profile_loaded",
                serde_json::json!({
                    "core_root": profile.source_root.display().to_string(),
                    "heartbeat_ms": profile.heartbeat_ms,
                    "triad_bypass": profile.triad_bypass,
                    "world_status": profile.world_status,
                    "world_resonance": profile.world_resonance,
                }),
            ))?;
        }
        self.ledger.append(&Message::event(
            "ceo",
            "heartbeat_selected",
            serde_json::json!({
                "mode": self.heartbeat.mode.to_string(),
                "interval_ms": self.heartbeat.interval_ms,
                "reason": self.heartbeat.reason,
            }),
        ))?;
        if let Some(roster) = &self.roster {
            self.ledger.append(&Message::event(
                "ceo",
                "order_of_battle_loaded",
                serde_json::json!({
                    "total_agents": roster.total_agents,
                    "online_agents": roster.online_agents,
                    "silent_agents": roster.silent_agents,
                }),
            ))?;
        }

        Ok(())
    }

    pub(super) async fn apply_governance_preflight(&self, task: &mut Task) -> Result<Option<f64>> {
        let estimated_cost = self.estimate_joule_cost(&task.task_type);
        if estimated_cost > self.joule_budget {
            task.fail("JouleWork budget exceeded");
            self.append_order(
                task.id,
                &task.task_type,
                OrderStatus::Failed,
                None,
                "joule budget exceeded",
            );
            self.append_thought(
                "concern",
                "joule_budget",
                &format!(
                    "Task {} exceeded joule budget. Estimated {}, budget {}.",
                    task.id, estimated_cost, self.joule_budget
                ),
            );
            self.emit_memory_event(
                "joule_budget_exceeded",
                &format!(
                    "Task {} exceeded joule budget {} with estimate {} because execution would violate budget policy",
                    task.id, self.joule_budget, estimated_cost
                ),
                Some(0.3),
                vec!["budget".to_string(), "failure".to_string(), "checkpoint".to_string()],
            );
            return Ok(None);
        }

        let confidence_base = self.score_confidence(task);
        let council = run_council_gate(
            task,
            confidence_base,
            self.roster.as_ref(),
            &self.council_config,
        );
        let confidence = council.adjusted_confidence;
        self.ledger.append(&Message::event(
            "ceo",
            "council_gate",
            serde_json::json!({
                "task_id": task.id,
                "triggered": council.triggered,
                "timed_out": council.timed_out,
                "responders_expected": council.responders_expected,
                "responders_available": council.responders_available,
                "confidence_base": confidence_base,
                "confidence_adjusted": confidence,
                "query_mode": &council.query_mode,
                "participating_seats": &council.participating_seats,
                "escalation_required": council.escalation_required,
                "reason": &council.reason,
            }),
        ))?;
        if council.timed_out {
            self.append_thought(
                "concern",
                "council_timeout",
                &format!(
                    "Council response incomplete for task {}. Confidence adjusted to {:.2}.",
                    task.id, confidence
                ),
            );
        }
        if council.triggered {
            let context = serde_json::json!({
                "task_id": task.id,
                "task_type": task.task_type,
                "description": task.description,
                "confidence_base": confidence_base,
                "confidence_adjusted": confidence,
                "responders_expected": council.responders_expected,
                "responders_available": council.responders_available,
                "timed_out": council.timed_out,
                "query_mode": &council.query_mode,
                "participating_seats": &council.participating_seats,
                "escalation_required": council.escalation_required
            });
            match PrometheusService::from_core("core")
                .and_then(|svc| svc.council_fanout(&task.description, Vec::new(), Some(context)))
            {
                Ok(fanout) => {
                    self.ledger
                        .append(&Message::event("ceo", "council_fanout", fanout))?;
                }
                Err(err) => {
                    self.ledger.append(&Message::event(
                        "ceo",
                        "council_fanout_error",
                        serde_json::json!({"error": err.to_string(), "task_id": task.id}),
                    ))?;
                }
            }
        }
        self.ledger.append(&Message::event(
            "ceo",
            "decision_scored",
            serde_json::json!({
                "task_id": task.id,
                "task_type": task.task_type,
                "confidence": confidence,
                "threshold": self.confidence_threshold,
                "autonomous_execution": confidence >= self.confidence_threshold,
            }),
        ))?;
        if confidence < self.confidence_threshold {
            task.transition(TaskStatus::Pending);
            self.ledger.append(&Message::event(
                "ceo",
                "escalation_required",
                serde_json::json!({
                    "task_id": task.id,
                    "reason": "confidence_below_threshold",
                    "confidence": confidence,
                    "threshold": self.confidence_threshold,
                }),
            ))?;
            self.append_order(
                task.id,
                &task.task_type,
                OrderStatus::Escalated,
                None,
                "confidence below threshold",
            );
            self.append_escalation(task.id, "confidence_below_threshold", confidence);
            self.append_thought(
                "question",
                "confidence_gate",
                &format!(
                    "Task {} held for escalation: confidence {:.2} below threshold {:.2}.",
                    task.id, confidence, self.confidence_threshold
                ),
            );
            self.emit_memory_event(
                "decision_escalated",
                &format!(
                    "Task {} held for escalation at confidence {:.2}",
                    task.id, confidence
                ),
                Some(confidence),
                vec!["escalation".to_string(), task.task_type.clone()],
            );
            if let Err(err) = record_bacon_lite(
                "prometheus",
                "decision_escalated",
                task,
                serde_json::json!({
                    "confidence": confidence,
                    "threshold": self.confidence_threshold,
                    "reason": "confidence_below_threshold",
                }),
            ) {
                tracing::debug!(error = %err, "PROMETHEUS bacon-lite escalation record failed");
            }
            return Ok(None);
        }

        Ok(Some(confidence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::ledger::Ledger;
    use arda_core::router::Router;
    use arda_core::task::{Task, TaskStatus};
    use tempfile::tempdir;

    fn test_pipeline() -> Pipeline {
        let dir = tempdir().expect("tempdir");
        let path = dir.keep();
        let ledger = Ledger::new(&path).expect("ledger");
        let router = Router::new();
        let mut pipeline = Pipeline::new(router, ledger, 100);
        pipeline.confidence_threshold = 0.95;
        pipeline
    }

    #[tokio::test]
    async fn governance_preflight_holds_low_confidence_work_for_escalation() {
        let pipeline = test_pipeline();
        let mut task = Task::new("review the day summary", "briefing");

        let confidence = pipeline
            .apply_governance_preflight(&mut task)
            .await
            .expect("preflight");

        assert!(confidence.is_none());
        assert!(matches!(task.status, TaskStatus::Pending));
    }
}
