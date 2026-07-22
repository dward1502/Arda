// sigil: REPAIR
//! Love Equation scoring shared across governance surfaces.

use arda_core::Task;
use serde::{Deserialize, Serialize};

use crate::versions::{legacy_love_equation_policy_version, LOVE_EQUATION_POLICY_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoveEquationScore {
    #[serde(default = "legacy_love_equation_policy_version")]
    pub policy_version: String,
    pub score: f64,
    pub impact: f64,
    pub reach: f64,
    pub energy: f64,
    pub time: f64,
    #[serde(default = "default_love_equation_semantic")]
    pub semantic: String,
    #[serde(default = "default_love_equation_source")]
    pub source: String,
}

fn default_love_equation_semantic() -> String {
    "task_value_proxy".to_string()
}

fn default_love_equation_source() -> String {
    "impact_reach_energy_time_proxy_not_canonical_love_dynamics".to_string()
}

pub fn love_equation_score(task: &Task) -> LoveEquationScore {
    let impact = match task.status {
        arda_core::TaskStatus::Complete => 0.95,
        arda_core::TaskStatus::Running => 0.7,
        arda_core::TaskStatus::Pending => 0.55,
        arda_core::TaskStatus::Retry { .. } => 0.45,
        arda_core::TaskStatus::Failed { .. } => 0.2,
    };

    let reach = match task.task_type.as_str() {
        "communicate" | "boardroom" | "delivery" => 0.85,
        "deploy" | "governance" | "monitor" => 0.75,
        "query" | "dispatch" | "route" => 0.65,
        _ => 0.6,
    };

    let energy = if task.joule_cost_actual > 0.0 {
        task.joule_cost_actual.max(1.0)
    } else if task.joule_cost_estimated > 0.0 {
        task.joule_cost_estimated.max(1.0)
    } else {
        1.0
    };

    let time = task
        .execution_duration_secs()
        .max(task.planning_duration_secs())
        .max(1.0);
    let score = ((impact * reach) / (energy * time)).clamp(0.0, 1.0);

    let score = LoveEquationScore {
        policy_version: LOVE_EQUATION_POLICY_VERSION.to_string(),
        score,
        impact,
        reach,
        energy,
        time,
        semantic: default_love_equation_semantic(),
        source: default_love_equation_source(),
    };
    crate::global_governance_metrics().observe_love_proxy(&score);
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{Task, TaskStatus};

    #[test]
    fn love_equation_prefers_completed_low_cost_work() {
        let mut task = Task::new("send executive update", "communicate");
        task.status = TaskStatus::Complete;
        task.planning_started_at = Some(task.created_at);
        task.execution_started_at = Some(task.created_at + chrono::TimeDelta::seconds(1));
        task.updated_at = task.created_at + chrono::TimeDelta::seconds(3);
        task.joule_cost_estimated = 1.0;
        task.joule_cost_actual = 1.0;

        let score = love_equation_score(&task);
        assert!(score.score > 0.1);
        assert!(score.impact > 0.9);
        assert_eq!(score.semantic, "task_value_proxy");
        assert!(score.source.contains("not_canonical_love_dynamics"));
    }

    #[test]
    fn love_equation_score_metadata_is_backward_compatible() {
        let score: LoveEquationScore = serde_json::from_str(
            r#"{"score":0.5,"impact":0.6,"reach":0.7,"energy":1.0,"time":2.0}"#,
        )
        .expect("legacy score json should deserialize with metadata defaults");

        assert_eq!(score.semantic, "task_value_proxy");
        assert_eq!(
            score.source,
            "impact_reach_energy_time_proxy_not_canonical_love_dynamics"
        );
    }

    #[test]
    fn love_equation_proxy_does_not_saturate_by_extra_scaling() {
        let mut task = Task::new("send executive update", "communicate");
        task.status = TaskStatus::Complete;
        task.joule_cost_actual = 1.0;

        let score = love_equation_score(&task);
        let expected_proxy =
            (score.impact * score.reach / (score.energy * score.time)).clamp(0.0, 1.0);

        assert!(score.score < 1.0);
        assert!((score.score - expected_proxy).abs() < 1e-9);
    }
}
