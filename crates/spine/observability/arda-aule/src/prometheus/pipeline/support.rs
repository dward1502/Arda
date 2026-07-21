#![cfg(feature = "full-cli")]
use crate::orders::OrderStatus;
use crate::pipeline::Pipeline;
use annunimas_core::task::Task;
use annunimas_mnemosyne::InformantEvent;

impl Pipeline {
    pub(super) fn estimate_joule_cost(&self, task_type: &str) -> u64 {
        let default_cost = 10u64;
        if let Some(profile) = &self.autonomy {
            if let Some(cost) = profile.base_cost_for(task_type) {
                return cost.ceil() as u64;
            }
        }
        default_cost
    }

    pub(super) fn score_confidence(&self, task: &Task) -> f64 {
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

    pub(super) fn append_thought(&self, thought_type: &str, trigger: &str, content: &str) {
        if let Some(ledger) = &self.thought_ledger {
            if let Err(err) = ledger.append(thought_type, trigger, content) {
                tracing::debug!(error = %err, "failed to append PROMETHEUS thought");
            }
        }
    }

    pub(super) fn append_order(
        &self,
        task_id: uuid::Uuid,
        task_type: &str,
        status: OrderStatus,
        agent: Option<&str>,
        note: &str,
    ) {
        if let Some(store) = &self.order_store {
            if let Err(err) = store.append_order(task_id, task_type, status, agent, note) {
                tracing::debug!(error = %err, "failed to append PROMETHEUS order");
            }
        }
    }

    pub(super) fn append_escalation(&self, task_id: uuid::Uuid, reason: &str, confidence: f64) {
        if let Some(store) = &self.order_store {
            if let Err(err) = store.append_escalation(task_id, reason, confidence) {
                tracing::debug!(error = %err, "failed to append PROMETHEUS escalation");
            }
        }
    }

    pub(super) fn emit_memory_event(
        &self,
        event_type: &str,
        content: &str,
        confidence_hint: Option<f64>,
        tags: Vec<String>,
    ) {
        if let Some(m) = &self.mnemosyne {
            let event = InformantEvent {
                informant_id: "prometheus_mneme".to_string(),
                crate_name: "prometheus".to_string(),
                event_type: event_type.to_string(),
                ts_utc: chrono::Utc::now().to_rfc3339(),
                content: content.to_string(),
                confidence_hint,
                tags,
            };
            if let Err(err) = m.encode(event) {
                tracing::debug!(error = %err, "failed to emit MNEMOSYNE informant event");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_link::CoreAutonomyProfile;
    use annunimas_core::ledger::Ledger;
    use annunimas_core::router::Router;
    use annunimas_core::task::Task;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn base_pipeline() -> Pipeline {
        let dir = tempdir().expect("tempdir");
        let path = dir.keep();
        let ledger = Ledger::new(&path).expect("ledger");
        let router = Router::new();
        Pipeline::new(router, ledger, 1_000)
    }

    fn pipeline_with_profile(world_status: &str, resonance: f64) -> Pipeline {
        let mut p = base_pipeline();
        p.autonomy = Some(CoreAutonomyProfile {
            heartbeat_ms: 500,
            triad_bypass: false,
            base_costs: HashMap::new(),
            world_status: Some(world_status.to_string()),
            world_resonance: Some(resonance),
            source_root: PathBuf::from("core"),
        });
        p
    }

    proptest! {
        #[test]
        fn score_confidence_is_always_in_unit_interval(
            description in ".*",
            task_type in "[a-z_]{3,12}",
        ) {
            let pipeline = base_pipeline();
            let task = Task::new(&description, &task_type);
            let score = pipeline.score_confidence(&task);
            prop_assert!((0.0..=1.0).contains(&score),
                "score {score} out of [0, 1]");
        }

        #[test]
        fn online_world_yields_higher_score_than_degraded(
            description in "[a-z ]{5,30}",
            task_type in "[a-z_]{3,10}",
        ) {
            let online = pipeline_with_profile("ONLINE", 75.0);
            let degraded = pipeline_with_profile("DEGRADED", 20.0);
            let task_online = Task::new(&description, &task_type);
            let task_degraded = Task::new(&description, &task_type);
            let score_online = online.score_confidence(&task_online);
            let score_degraded = degraded.score_confidence(&task_degraded);
            prop_assert!(score_online >= score_degraded,
                "online score {score_online} should be >= degraded score {score_degraded}");
        }

        #[test]
        fn non_empty_description_never_lowers_score(
            task_type in "[a-z_]{3,10}",
            description in "[a-z ]{1,30}",
        ) {
            let pipeline = base_pipeline();
            let with_desc = pipeline.score_confidence(&Task::new(&description, &task_type));
            let without_desc = pipeline.score_confidence(&Task::new("", &task_type));
            prop_assert!(with_desc >= without_desc,
                "non-empty description should not lower score");
        }

        #[test]
        fn joule_cost_falls_back_to_10_when_no_profile(task_type in "[a-z_]{3,12}") {
            let pipeline = base_pipeline();
            prop_assert_eq!(pipeline.estimate_joule_cost(&task_type), 10);
        }
    }
}
