//! Phase 1 deterministic Planner.
//!
//! Reads active `Goal`s, emits one `Plan` per goal per UTC day with a
//! canned step sequence keyed off `goal.id`. No LLM in v0.1 — per
//! `docs/plans/PHASE_1_PLAN.md` step 3 and the PRD's "boring + reliable
//! beats clever + flaky" guidance for Phase 1.
//!
//! Idempotency: plan id is `plan_<goal_id>_<YYYYMMDD>`. If a plan with
//! that id already exists on disk, the planner skips it. So running
//! `loop tick` more than once per day per goal is a no-op.

use std::path::Path;

use arda_core::contract::{Goal, GoalStatus, Plan, PlanStep};
use arda_core::error::Result;
use arda_core::state::{self, StateRoot};
use arda_core::task::Task;
use chrono::Utc;
use serde_json::json;

/// Result of a single planner pass.
#[derive(Debug, Default)]
pub struct PlanPass {
    pub goals_considered: usize,
    pub plans_written: Vec<String>,
    pub plans_skipped_existing: Vec<String>,
    pub goals_without_recipe: Vec<String>,
    pub goals_inactive: Vec<String>,
    pub tasks_emitted: usize,
}

/// Run one planner pass against `state`. Tasks emitted from plan
/// steps are appended to `queue_path` (typically
/// `<repo>/core/projects/tasks/queue.jsonl` per FILE_LAYOUT §4.2).
/// Pass `None` to suppress task emission (used in tests that only
/// care about plan generation).
pub fn run(state: &StateRoot, queue_path: Option<&Path>) -> Result<PlanPass> {
    let mut pass = PlanPass::default();
    let goals = state::list_goals(state)?;
    pass.goals_considered = goals.len();

    let today = Utc::now().format("%Y%m%d").to_string();
    let existing_plan_ids: std::collections::HashSet<String> = state::list_plans(state)?
        .into_iter()
        .map(|p| p.id)
        .collect();

    for goal in goals {
        if goal.status != GoalStatus::Active {
            pass.goals_inactive.push(goal.id);
            continue;
        }
        let plan_id = format!("plan_{}_{}", goal.id, today);
        if existing_plan_ids.contains(&plan_id) {
            pass.plans_skipped_existing.push(plan_id);
            continue;
        }
        let Some((summary, steps)) = recipe_for(&goal) else {
            pass.goals_without_recipe.push(goal.id);
            continue;
        };
        let plan = Plan::new(plan_id.clone(), goal.id.clone(), summary, steps.clone());
        state::write_plan(state, &plan)?;
        pass.plans_written.push(plan_id.clone());

        if let Some(qp) = queue_path {
            for (idx, step) in steps.iter().enumerate() {
                let description = format!("{}#{}: {}", plan_id, idx, step.intent);
                let task =
                    Task::new(description, step.intent.clone()).with_plan_lineage(&plan_id, idx);
                state::append_task(qp, &task)?;
                pass.tasks_emitted += 1;
            }
        }
    }

    Ok(pass)
}

/// Canned step recipe per known goal id. Returns None for goals the
/// planner doesn't know how to decompose (those are surfaced in the
/// pass summary so the operator can either author a recipe or
/// abandon the goal).
fn recipe_for(goal: &Goal) -> Option<(String, Vec<PlanStep>)> {
    let summary;
    let steps: Vec<PlanStep>;
    match goal.id.as_str() {
        "goal_provider_mesh_health" => {
            summary = "Probe each provider tier; retire any failing two consecutive checks.".into();
            steps = vec![
                step("probe_provider", json!({"tier": "free"})),
                step("probe_provider", json!({"tier": "paid"})),
                step("retire_failing", json!({"consecutive_failures": 2})),
            ];
        }
        "goal_daily_joulework_report" => {
            summary = "Roll up today's joule spend and emit a single summary LedgerEntry.".into();
            steps = vec![
                step("collect_joule_samples", json!({"since": "day_start_utc"})),
                step("summarize_by_agent", json!({})),
                step("summarize_by_provider_tier", json!({})),
                step("emit_ledger_summary", json!({"kind": "joulework_daily"})),
            ];
        }
        "goal_knowledge_index_freshness" => {
            summary = "Reindex any knowledge source whose mtime moved since last pass.".into();
            steps = vec![
                step("scan_knowledge_sources", json!({})),
                step("diff_against_last_index", json!({})),
                step("reindex_changed", json!({})),
            ];
        }
        "goal_ledger_compaction" => {
            summary = "Archive ledger segments older than 14 days; preserve audit lineage.".into();
            steps = vec![
                step("list_ledger_segments", json!({})),
                step(
                    "archive_older_than",
                    json!({"days": 14, "destination": "core/state/ledger/_archive/"}),
                ),
            ];
        }
        "goal_council_presence" => {
            summary = "Probe each Council seat; escalate after two consecutive failures.".into();
            steps = vec![
                step("probe_seat", json!({"seat": "aurelius"})),
                step("probe_seat", json!({"seat": "bacon"})),
                step("probe_seat", json!({"seat": "sun_tzu"})),
                step(
                    "escalate_if_repeat_failure",
                    json!({"consecutive_failures": 2}),
                ),
            ];
        }
        _ => return None,
    }
    Some((summary, steps))
}

fn step(intent: &str, params: serde_json::Value) -> PlanStep {
    PlanStep {
        intent: intent.into(),
        params,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::contract::GoalPriority;

    fn tmp_state() -> (tempfile::TempDir, StateRoot) {
        let dir = tempfile::tempdir().unwrap();
        let root = StateRoot::new(dir.path().to_path_buf());
        (dir, root)
    }

    #[test]
    fn produces_one_plan_per_known_active_goal() {
        let (_d, st) = tmp_state();
        for id in [
            "goal_provider_mesh_health",
            "goal_daily_joulework_report",
            "goal_knowledge_index_freshness",
            "goal_ledger_compaction",
            "goal_council_presence",
        ] {
            let g = Goal::new(id, "t", "i", "owner", GoalPriority::Medium);
            state::write_goal(&st, &g).unwrap();
        }
        let pass = run(&st, None).unwrap();
        assert_eq!(pass.plans_written.len(), 5);
        assert_eq!(pass.goals_without_recipe.len(), 0);
    }

    #[test]
    fn skips_existing_plans_same_day() {
        let (_d, st) = tmp_state();
        let g = Goal::new(
            "goal_provider_mesh_health",
            "t",
            "i",
            "prometheus",
            GoalPriority::High,
        );
        state::write_goal(&st, &g).unwrap();
        let first = run(&st, None).unwrap();
        assert_eq!(first.plans_written.len(), 1);
        let second = run(&st, None).unwrap();
        assert_eq!(second.plans_written.len(), 0);
        assert_eq!(second.plans_skipped_existing.len(), 1);
    }

    #[test]
    fn flags_unknown_goal_id() {
        let (_d, st) = tmp_state();
        let g = Goal::new("goal_made_up", "t", "i", "x", GoalPriority::Low);
        state::write_goal(&st, &g).unwrap();
        let pass = run(&st, None).unwrap();
        assert_eq!(pass.plans_written.len(), 0);
        assert_eq!(pass.goals_without_recipe, vec!["goal_made_up"]);
    }

    #[test]
    fn emits_one_task_per_plan_step_with_lineage() {
        let (_d, st) = tmp_state();
        let g = Goal::new(
            "goal_provider_mesh_health",
            "t",
            "i",
            "prometheus",
            GoalPriority::High,
        );
        state::write_goal(&st, &g).unwrap();
        let queue = st.root().join("queue.jsonl");
        let pass = run(&st, Some(&queue)).unwrap();
        // Recipe has 3 steps for provider mesh health
        assert_eq!(pass.tasks_emitted, 3);
        let tasks = arda_core::state::read_contract_tasks(&queue).unwrap();
        assert_eq!(tasks.len(), 3);
        assert!(tasks.iter().all(|t| t.plan_id.is_some()));
        assert_eq!(
            tasks
                .iter()
                .filter_map(|t| t.plan_step_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn skips_inactive_goal() {
        let (_d, st) = tmp_state();
        let mut g = Goal::new(
            "goal_provider_mesh_health",
            "t",
            "i",
            "prometheus",
            GoalPriority::High,
        );
        g.status = GoalStatus::Paused;
        state::write_goal(&st, &g).unwrap();
        let pass = run(&st, None).unwrap();
        assert_eq!(pass.plans_written.len(), 0);
        assert_eq!(pass.goals_inactive.len(), 1);
    }
}
