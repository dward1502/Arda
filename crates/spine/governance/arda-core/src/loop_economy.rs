//! Phase 2 loop economy snapshot.
//!
//! Reads the Decision ledger and emits the HUD/operator-facing
//! summary required by the PRD: live joules/minute, joules by agent,
//! and the current bid spread in the internal joule market.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::contract::{Decision, DecisionClass};
use crate::state::StateRoot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopEconomySnapshot {
    pub generated_at_utc: chrono::DateTime<Utc>,
    pub ledger_path: PathBuf,
    pub decisions_today: usize,
    pub total_joules_today: f64,
    pub joules_per_minute_last_60s: f64,
    pub joules_by_agent: BTreeMap<String, f64>,
    pub bid_count_today: usize,
    pub latest_bid_spread: Option<BidSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BidSpread {
    pub decided_at_utc: chrono::DateTime<Utc>,
    pub low_joules: f64,
    pub high_joules: f64,
    pub spread_joules: f64,
    pub bidders: usize,
}

pub fn snapshot_path(state: &StateRoot) -> PathBuf {
    state.root().join("loop_economy.json")
}

pub fn today_ledger_path(state: &StateRoot) -> PathBuf {
    let today = Utc::now().format("%Y-%m-%d");
    state
        .root()
        .join("ledger")
        .join(format!("ledger_{today}.jsonl"))
}

pub fn build_snapshot(state: &StateRoot) -> std::io::Result<LoopEconomySnapshot> {
    let ledger_path = today_ledger_path(state);
    let generated_at_utc = Utc::now();
    let cutoff = generated_at_utc - Duration::seconds(60);
    let mut decisions = Vec::new();

    if ledger_path.exists() {
        let raw = std::fs::read_to_string(&ledger_path)?;
        decisions = raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<Decision>(line).ok())
            .collect();
    }

    let mut total_joules_today = 0.0;
    let mut joules_last_60s = 0.0;
    let mut joules_by_agent = BTreeMap::new();
    let mut bid_count_today = 0usize;
    let mut latest_bid_spread = None;

    for decision in &decisions {
        total_joules_today += decision.joule_estimate;
        if decision.decided_at >= cutoff {
            joules_last_60s += decision.joule_estimate;
        }
        if matches!(decision.decision_class, DecisionClass::Dispatch) {
            *joules_by_agent
                .entry(decision.chosen.clone())
                .or_insert(0.0) += decision.joule_estimate;
        }
        if matches!(decision.decision_class, DecisionClass::Bid) {
            bid_count_today += 1;
            if let Some(spread) = bid_spread_from_decision(decision) {
                latest_bid_spread = Some(spread);
            }
        }
    }

    Ok(LoopEconomySnapshot {
        generated_at_utc,
        ledger_path,
        decisions_today: decisions.len(),
        total_joules_today,
        joules_per_minute_last_60s: joules_last_60s,
        joules_by_agent,
        bid_count_today,
        latest_bid_spread,
    })
}

pub fn write_snapshot(state: &StateRoot) -> std::io::Result<LoopEconomySnapshot> {
    let snapshot = build_snapshot(state)?;
    let path = snapshot_path(state);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(&snapshot).map_err(std::io::Error::other)? + "\n";
    std::fs::write(path, payload)?;
    Ok(snapshot)
}

fn bid_spread_from_decision(decision: &Decision) -> Option<BidSpread> {
    let joules = decision
        .options_considered
        .iter()
        .filter_map(|option| parse_bid_joules(option))
        .collect::<Vec<_>>();
    if joules.is_empty() {
        return None;
    }
    let low = joules.iter().copied().fold(f64::INFINITY, f64::min);
    let high = joules.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some(BidSpread {
        decided_at_utc: decision.decided_at,
        low_joules: low,
        high_joules: high,
        spread_joules: high - low,
        bidders: joules.len(),
    })
}

fn parse_bid_joules(option: &str) -> Option<f64> {
    let after = option.split("@j=").nth(1)?;
    let value = after.split(',').next().unwrap_or(after);
    value.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{PhilosopherVerdict, TriadOutcome, TriadVerdict};
    use crate::ledger::Ledger;

    fn triad() -> TriadOutcome {
        let verdict = PhilosopherVerdict {
            verdict: TriadVerdict::Pass,
            reason: None,
        };
        TriadOutcome {
            verdict: TriadVerdict::Pass,
            aurelius: verdict.clone(),
            bacon: verdict.clone(),
            sun_tzu: verdict,
        }
    }

    #[test]
    fn snapshot_counts_joules_agents_and_bid_spread() {
        let dir = tempfile::tempdir().unwrap();
        let state = StateRoot::new(dir.path().join("core/state"));
        let ledger = Ledger::new(state.root().join("ledger")).unwrap();

        let mut bid = Decision::new("bid_1", DecisionClass::Bid, "task_1", "cheap", "r", triad());
        bid.options_considered = vec![
            "expensive@j=9.000,c=0.90".to_string(),
            "cheap@j=1.000,c=0.60".to_string(),
        ];
        bid.joule_estimate = 1.0;
        ledger.append(&bid).unwrap();

        let mut dispatch = Decision::new(
            "dec_1",
            DecisionClass::Dispatch,
            "task_1",
            "cheap",
            "r",
            triad(),
        );
        dispatch.joule_estimate = 2.5;
        ledger.append(&dispatch).unwrap();

        let snapshot = write_snapshot(&state).unwrap();
        assert_eq!(snapshot.decisions_today, 2);
        assert!((snapshot.total_joules_today - 3.5).abs() < 1e-9);
        assert_eq!(snapshot.joules_by_agent.get("cheap").copied(), Some(2.5));
        assert_eq!(snapshot.bid_count_today, 1);
        assert_eq!(
            snapshot.latest_bid_spread.as_ref().map(|s| s.spread_joules),
            Some(8.0)
        );
        assert!(snapshot_path(&state).exists());
    }
}
