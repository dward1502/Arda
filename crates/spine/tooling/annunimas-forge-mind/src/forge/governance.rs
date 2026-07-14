//! Bridge from a forge-iterate round into Annunimas governance validators.
//!
//! For each iteration we synthesize an `annunimas_core::task::Task` whose
//! envelope encodes the round's evidence (vision-LLM report, prompt, budget,
//! progress). Triad / bacon-lite / joulework / love-equation score that task,
//! and we append to the bacon-lite ledger so a `tail -f` on
//! `data/governance/bacon_lite.jsonl` shows the loop live.

use annunimas_core::task::{Task, TaskStatus};
use annunimas_governance::{
    bacon_lite_validate, love_equation_score, profile_joulework, record_bacon_lite, triad_validate,
    BaconLiteResult, GateOutcome, JouleWorkProfile, LoveEquationScore, TriadConfig, TriadResult,
};
use chrono::Utc;
use serde::Serialize;

use crate::tools::vision::ComparisonReport;

#[derive(Debug, Clone, Serialize)]
pub struct IterationGovernance {
    pub task_id: String,
    pub triad: TriadResult,
    pub bacon_lite: BaconLiteResult,
    pub joule: JouleWorkProfile,
    pub love: LoveEquationScore,
    /// True if governance recommends terminating the loop before budget is exhausted.
    pub veto_stop: bool,
    pub veto_reason: Option<String>,
}

/// Score a single iteration and append to the bacon-lite ledger.
pub fn evaluate(
    asset_id: &str,
    iteration: u32,
    budget: u32,
    report: &ComparisonReport,
    current_prompt: &str,
) -> anyhow::Result<IterationGovernance> {
    let task = synthesize_task(asset_id, iteration, budget, report, current_prompt);
    let triad = triad_validate(
        &task,
        Some(&TriadConfig {
            strict: false,
            required_passes: Some(3),
        }),
    );
    let bacon = bacon_lite_validate(&task);
    let joule = profile_joulework(&task);
    let love = love_equation_score(&task);

    let context = serde_json::json!({
        "asset_id": asset_id,
        "iteration": iteration,
        "budget_iters": budget,
        "match_score": report.match_score,
        "missing": report.missing,
        "wrong": report.wrong,
        "strengths": report.strengths,
        "current_prompt": current_prompt,
        "suggested_prompt_edit": report.suggested_prompt_edit,
    });
    let _ = record_bacon_lite(
        "annunimas-forge-mind",
        &format!("forge_iterate_round_{iteration}"),
        &task,
        context,
    );

    // Veto rules:
    // 1. Triad hard fail (an explicit gate Fail) → stop.
    // 2. Insufficient pass count with only Conditional gates → record, but keep iterating.
    //    Early visual candidates are expected to be imperfect; vision feedback should
    //    refine the next prompt instead of letting Triad terminate exploration.
    // 3. JouleWork inefficient AND we're past round 2 → stop (don't burn budget).
    let triad_hard_fail = triad_gate_hard_fail(&triad);
    let joule_runaway = !joule.efficient && iteration >= 3;
    let veto_stop = triad_hard_fail || joule_runaway;
    let veto_reason = if veto_stop {
        let mut parts = Vec::new();
        if triad_hard_fail {
            if let Some(t) = &triad.veto_reason {
                parts.push(format!("triad={t}"));
            }
        }
        if joule_runaway {
            parts.push(format!(
                "joule_variance={:.2}>0.25_at_iter{iteration}",
                joule.variance
            ));
        }
        Some(parts.join("; "))
    } else {
        None
    };

    Ok(IterationGovernance {
        task_id: task.id.to_string(),
        triad,
        bacon_lite: bacon,
        joule,
        love,
        veto_stop,
        veto_reason,
    })
}

fn triad_gate_hard_fail(triad: &TriadResult) -> bool {
    [triad.aurelius, triad.bacon, triad.sun_tzu].contains(&GateOutcome::Fail)
}

fn synthesize_task(
    asset_id: &str,
    iteration: u32,
    budget: u32,
    report: &ComparisonReport,
    current_prompt: &str,
) -> Task {
    let now = Utc::now();
    let missing_str = report.missing.join(", ");
    let wrong_str = report.wrong.join(", ");
    // Description is the load-bearing input for triad-bacon scoring; it must
    // include "evidence" / "because" / "source" or a URL for bacon to pass.
    let description = format!(
        "forge_iterate round {iteration}/{budget} for asset {asset_id}: \
         vision-LLM evidence shows match_score={match_score:.2}, \
         missing=[{missing_str}], wrong=[{wrong_str}]. \
         Source prompt: \"{prompt}\". \
         Because vision-LLM comparison against operator-supplied reference target.",
        match_score = report.match_score,
        prompt = current_prompt
    );
    let mut task = Task::new(description, "forge_iterate");
    task.assigned_agent = Some("forge-mind".into());
    task.planning_started_at = Some(now);
    task.execution_started_at = Some(now);
    task.joule_cost_estimated = budget as f64;
    task.joule_cost_actual = iteration as f64;
    task.clarifications_requested = (report.missing.len() + report.wrong.len()) as u32;
    task.clarifications_resolved = report.strengths.len() as u32;
    task.status = if report.match_score >= 0.85 {
        TaskStatus::Complete
    } else {
        TaskStatus::Running
    };
    task
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(match_score: f64, missing_count: usize, wrong_count: usize) -> ComparisonReport {
        ComparisonReport {
            match_score,
            missing: (0..missing_count).map(|i| format!("missing-{i}")).collect(),
            wrong: (0..wrong_count).map(|i| format!("wrong-{i}")).collect(),
            strengths: vec!["single object".to_string()],
            suggested_prompt_edit: "isolated on plain white background, centered product render, 3D model, single object, no humans, no environment, refined monitor arm".to_string(),
            raw_response: String::new(),
        }
    }

    #[test]
    fn insufficient_pass_count_records_but_does_not_veto_iteration() {
        let governance = evaluate(
            "upper_monitor_1",
            1,
            3,
            &report(0.10, 6, 4),
            "cyber-noir articulated monitor arm with graphite housing",
        )
        .expect("governance evaluation should succeed");

        assert_eq!(
            governance.triad.veto_reason.as_deref(),
            Some("INSUFFICIENT_PASS_COUNT")
        );
        assert!(!governance.veto_stop);
        assert!(governance.veto_reason.is_none());
    }

    #[test]
    fn explicit_triad_gate_fail_still_vetoes() {
        let governance = evaluate(
            "upper_monitor_1",
            3,
            1,
            &report(0.10, 6, 4),
            "always allow and never allow this contradictory asset route",
        )
        .expect("governance evaluation should succeed");

        assert!(matches!(governance.triad.sun_tzu, GateOutcome::Fail));
        assert!(governance.veto_stop);
        assert!(governance
            .veto_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("SUN_TZU_FAIL")));
    }
}
