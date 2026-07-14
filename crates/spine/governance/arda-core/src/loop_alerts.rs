//! Warden alerts for the autonomy loop (Phase 2 step 10).
//!
//! Catches the loop misbehaving and ledgers it. The PRD calls out
//! joule-budget breach, repeated failure, runaway loops, schema
//! drift, and market collapse. v0.2 covers market collapse,
//! budget block, and repeated-failure (≥2 consecutive Failures for
//! the same task across reflections); the rest land alongside
//! their natural producers.
//!
//! Alerts are appended to `<state>/alerts/alerts_<UTC-date>.jsonl`
//! so an operator (or `arda-cli warden status`) can read them
//! without standing up the Warden agent.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::contract::Reflection;
use crate::loop_engine::DispatchPass;
use crate::state::StateRoot;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WardenAlertKind {
    MarketCollapse,
    BudgetBlocked,
    RepeatedFailure,
    RunawayLoop,
    BadReflection,
    SchemaDrift,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenAlert {
    pub kind: WardenAlertKind,
    pub subject: String,
    pub message: String,
    pub observed_at: DateTime<Utc>,
}

impl WardenAlert {
    pub fn new(
        kind: WardenAlertKind,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            message: message.into(),
            observed_at: Utc::now(),
        }
    }
}

/// Build the alert file path for today (UTC).
pub fn alerts_path(state: &StateRoot) -> PathBuf {
    let dir = state.root().join("alerts");
    let day = Utc::now().format("%Y-%m-%d");
    dir.join(format!("alerts_{day}.jsonl"))
}

/// Append alerts to today's alert file. Creates the directory if
/// needed. Returns the number written.
pub fn append_alerts(state: &StateRoot, alerts: &[WardenAlert]) -> std::io::Result<usize> {
    if alerts.is_empty() {
        return Ok(0);
    }
    let path = alerts_path(state);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    for a in alerts {
        let line = serde_json::to_string(a).map_err(std::io::Error::other)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
    }
    Ok(alerts.len())
}

/// Read the last `n` alerts from today's alert file. Returns empty
/// vec when there's no file.
pub fn read_recent(state: &StateRoot, n: usize) -> std::io::Result<Vec<WardenAlert>> {
    let path = alerts_path(state);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    let mut all: Vec<WardenAlert> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if all.len() > n {
        let drop = all.len() - n;
        all.drain(..drop);
    }
    Ok(all)
}

/// Analyze a finished dispatch pass + the current reflection list
/// and return any alerts that should fire.
pub fn analyze_tick(pass: &DispatchPass, reflections: &[Reflection]) -> Vec<WardenAlert> {
    let mut out = Vec::new();

    for tid in &pass.market_collapses {
        out.push(WardenAlert::new(
            WardenAlertKind::MarketCollapse,
            tid,
            "no agent willing to bid for this task — investigate registry".to_string(),
        ));
    }
    for entry in &pass.budget_blocked {
        // entry shape: "<task_id>:goal=<gid>:spent=<x>/budget=<y>"
        let subject = entry.split(':').next().unwrap_or(entry).to_string();
        out.push(WardenAlert::new(
            WardenAlertKind::BudgetBlocked,
            subject,
            format!("daily joule budget exhausted: {entry}"),
        ));
    }

    // Repeated failure: ≥2 consecutive Failure reflections for the
    // same task. We sort by completed_at and look at the last few
    // per task; if the latest two are both Failure, raise.
    use crate::contract::ReflectionOutcome;
    use std::collections::HashMap;
    let mut by_task: HashMap<&str, Vec<&Reflection>> = HashMap::new();
    for r in reflections {
        by_task.entry(r.task_id.as_str()).or_default().push(r);
    }
    for (task_id, mut refs) in by_task {
        refs.sort_by_key(|r| r.completed_at);
        if refs.len() >= 2 {
            let n = refs.len();
            let last = &refs[n - 1];
            let prev = &refs[n - 2];
            if last.outcome == ReflectionOutcome::Failure
                && prev.outcome == ReflectionOutcome::Failure
            {
                out.push(WardenAlert::new(
                    WardenAlertKind::RepeatedFailure,
                    task_id,
                    "2 consecutive failure reflections on the same task".to_string(),
                ));
            }
        }
    }

    out
}

/// Convert chaos injection markers into canonical Warden alerts.
/// This lets `scripts/chaos/*` assert that each Phase 2 chaos
/// scenario is caught and ledgered without mutating production state.
pub fn analyze_chaos_log(state: &StateRoot) -> std::io::Result<Vec<WardenAlert>> {
    let path = state.root().join("chaos_log.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    let mut alerts = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            alerts.push(WardenAlert::new(
                WardenAlertKind::SchemaDrift,
                "chaos_log",
                "malformed chaos marker line detected",
            ));
            continue;
        };
        let marker_type = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        match marker_type {
            "joule_overrun" | "chaos_injection" => {
                let scenario = value
                    .get("scenario")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(marker_type);
                if scenario == "joule_overrun" {
                    alerts.push(WardenAlert::new(
                        WardenAlertKind::BudgetBlocked,
                        "chaos_joule_overrun",
                        "chaos joule overrun marker contained as budget pressure",
                    ));
                }
            }
            "planner_loop" => alerts.push(WardenAlert::new(
                WardenAlertKind::RunawayLoop,
                "chaos_planner_loop",
                "chaos planner loop marker contained as runaway-loop risk",
            )),
            "bad_reflection" => alerts.push(WardenAlert::new(
                WardenAlertKind::BadReflection,
                "chaos_bad_reflection",
                "chaos bad reflection marker contained for review",
            )),
            "schema_drift" => alerts.push(WardenAlert::new(
                WardenAlertKind::SchemaDrift,
                "chaos_schema_drift",
                "chaos schema drift marker contained as schema alert",
            )),
            "market_collapse" => alerts.push(WardenAlert::new(
                WardenAlertKind::MarketCollapse,
                "chaos_market_collapse",
                "chaos market collapse marker contained as market alert",
            )),
            _ => alerts.push(WardenAlert::new(
                WardenAlertKind::SchemaDrift,
                "chaos_unknown",
                format!("unknown chaos marker type: {marker_type}"),
            )),
        }
    }
    Ok(alerts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_state() -> (tempfile::TempDir, StateRoot) {
        let dir = tempfile::tempdir().unwrap();
        let st = StateRoot::new(dir.path().to_path_buf());
        (dir, st)
    }

    #[test]
    fn analyze_market_collapse_and_budget_blocked() {
        let mut pass = DispatchPass::default();
        pass.market_collapses.push("task_a".into());
        pass.budget_blocked
            .push("task_b:goal=g1:spent=10.00/budget=8.00".into());
        let alerts = analyze_tick(&pass, &[]);
        assert_eq!(alerts.len(), 2);
        let kinds: Vec<_> = alerts.iter().map(|a| a.kind).collect();
        assert!(kinds.contains(&WardenAlertKind::MarketCollapse));
        assert!(kinds.contains(&WardenAlertKind::BudgetBlocked));
    }

    #[test]
    fn append_and_read_round_trips() {
        let (_d, st) = tmp_state();
        let alerts = vec![
            WardenAlert::new(WardenAlertKind::MarketCollapse, "t1", "msg1"),
            WardenAlert::new(WardenAlertKind::BudgetBlocked, "t2", "msg2"),
        ];
        let n = append_alerts(&st, &alerts).unwrap();
        assert_eq!(n, 2);
        let read = read_recent(&st, 10).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].subject, "t1");
        assert_eq!(read[1].subject, "t2");
    }

    #[test]
    fn repeated_failure_detected() {
        use crate::contract::{Reflection, ReflectionOutcome};
        let mut r1 = Reflection::new("ref_1", "task_x", "plan_1", ReflectionOutcome::Failure, 0.0);
        let mut r2 = Reflection::new("ref_2", "task_x", "plan_1", ReflectionOutcome::Failure, 0.0);
        r2.completed_at = r1.completed_at + chrono::Duration::seconds(1);
        // Avoid unused_mut on r1.
        r1.score = 0.0;
        let alerts = analyze_tick(&DispatchPass::default(), &[r1, r2]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, WardenAlertKind::RepeatedFailure);
        assert_eq!(alerts[0].subject, "task_x");
    }

    #[test]
    fn chaos_log_markers_become_canonical_alerts() {
        let (_d, st) = tmp_state();
        std::fs::create_dir_all(st.root()).unwrap();
        std::fs::write(
            st.root().join("chaos_log.jsonl"),
            [
                r#"{"type":"market_collapse"}"#,
                r#"{"type":"planner_loop"}"#,
                r#"{"type":"bad_reflection"}"#,
                r#"{"type":"schema_drift"}"#,
                r#"{"type":"chaos_injection","scenario":"joule_overrun"}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let alerts = analyze_chaos_log(&st).unwrap();
        let kinds = alerts.iter().map(|alert| alert.kind).collect::<Vec<_>>();
        assert!(kinds.contains(&WardenAlertKind::MarketCollapse));
        assert!(kinds.contains(&WardenAlertKind::RunawayLoop));
        assert!(kinds.contains(&WardenAlertKind::BadReflection));
        assert!(kinds.contains(&WardenAlertKind::SchemaDrift));
        assert!(kinds.contains(&WardenAlertKind::BudgetBlocked));
    }
}
