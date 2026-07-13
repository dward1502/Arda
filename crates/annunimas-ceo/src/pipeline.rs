// sigil: REPAIR
//! Annunimas CEO Pipeline
//! 
//! Simplified orchestration - routes tasks to agents

use crate::core_link::CoreAutonomyProfile;
use annunimas_core::error::Result;
use annunimas_core::ledger::Ledger;
use annunimas_core::message::Message;
use annunimas_core::task::{Task, TaskStatus};
use annunimas_core::router::Router;

pub struct Pipeline {
    router: Router,
    ledger: Ledger,
    joule_budget: u64,
    autonomy: Option<CoreAutonomyProfile>,
    confidence_threshold: f64,
}

impl Pipeline {
    pub fn new(router: Router, ledger: Ledger, joule_budget: u64) -> Self {
        Self {
            router,
            ledger,
            joule_budget,
            autonomy: None,
            confidence_threshold: 0.0,
        }
    }

    pub fn with_core_link(
        router: Router,
        ledger: Ledger,
        joule_budget: u64,
        core_root: impl AsRef<std::path::Path>,
    ) -> Self {
        Self {
            router,
            ledger,
            joule_budget,
            autonomy: CoreAutonomyProfile::load(core_root),
            confidence_threshold: 0.75,
        }
    }

    pub async fn submit(&self, mut task: Task) -> Result<Task> {
        self.ledger.append(&Message::event(
            "ceo",
            "task_received",
            serde_json::to_value(&task)?,
        ))?;

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

        // Check JouleWork budget using core/realm boot cost table when available.
        let estimated_cost = self.estimate_joule_cost(&task.task_type);
        if estimated_cost > self.joule_budget {
            task.fail("JouleWork budget exceeded");
            return Ok(task);
        }

        let confidence = self.score_confidence(&task);
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
            return Ok(task);
        }

        // Route task to agent
        match self.router.route(&task) {
            Ok(agent) => {
                let agent_name = agent.name().to_string();
                task.assign(&agent_name);
                let task_id = task.id;
                self.ledger.append(&Message::task_assignment(task_id, &agent_name))?;

                match agent.execute(&mut task).await {
                    Ok(()) => {
                        let completion_result = Self::normalized_completion_result(&mut task);
                        self.ledger.append(&Message::task_complete(
                            task_id,
                            &agent_name,
                            completion_result,
                        ))?;
                    }
                    Err(e) => {
                        let reason = e.to_string();
                        task.fail(&reason);
                        self.ledger.append(&Message::task_failed(task_id, &agent_name, &reason))?;
                    }
                }
            }
            Err(e) => {
                task.fail(&format!("No route: {}", e));
            }
        }

        Ok(task)
    }

    fn estimate_joule_cost(&self, task_type: &str) -> u64 {
        let default_cost = 10u64;
        if let Some(profile) = &self.autonomy {
            if let Some(cost) = profile.base_cost_for(task_type) {
                return cost.ceil() as u64;
            }
        }
        default_cost
    }

    fn score_confidence(&self, task: &Task) -> f64 {
        // Deterministic bootstrap scoring until council/memory integration lands.
        let mut score: f64 = 0.45;

        if self.router.route(task).is_ok() {
            score += 0.25;
        }

        if !task.description.trim().is_empty() {
            score += 0.10;
        }

        if let Some(profile) = &self.autonomy {
            if let Some(status) = &profile.world_status {
                if status.eq_ignore_ascii_case("ONLINE") || status.eq_ignore_ascii_case("READY") {
                    score += 0.10;
                }
            }
            if let Some(res) = profile.world_resonance {
                if res >= 50.0 {
                    score += 0.10;
                } else if res > 0.0 {
                    score += 0.05;
                }
            }
        }

        score.clamp(0.0, 1.0)
    }

    fn normalized_completion_result(task: &mut Task) -> serde_json::Value {
        match task.result.clone() {
            Some(value) if !value.is_null() => value,
            _ => {
                let value = serde_json::json!({"status": "completed"});
                task.result = Some(value.clone());
                value
            }
        }
    }
}
