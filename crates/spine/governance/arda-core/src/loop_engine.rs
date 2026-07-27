//! Phase 1 autonomy-loop substrate: Dispatcher + Reflector.
//!
//! Per docs/plans/PHASE_1_PLAN.md: lives in arda-core for v0.1
//! so the CLI can drive a complete tick without growing the crate
//! dependency graph. The PRD lists supervisor + oracle as the
//! eventual canonical homes; that move is bookkeeping once the loop
//! is proven.
//!
//! Today the dispatcher does NOT execute anything. It picks an agent,
//! ledgers a Decision, and *simulates* completion so the Reflector
//! has something to score. Real execution wires in alongside the
//! Phase 2 joule market — the contract slot is what matters now.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::contract::{
    Decision, DecisionClass, PhilosopherVerdict, Reflection, ReflectionOutcome, TriadOutcome,
    TriadVerdict,
};
use crate::error::Result;
use crate::governance_gates::{
    AffordabilityPolicy, AllowAllAffordability, GovernanceGates, GovernancePolicyMode,
};
use crate::ledger::Ledger;
use crate::loop_observability::{DecisionLatencyKind, LatencyProbe, LoopObservabilityConfig};
use crate::state::{self, StateRoot};
use crate::task::{Task, TaskStatus};

// ---------------------------------------------------------------
// Routing table — intent -> agent. Hard-coded for v0.1 per PRD §Phase
// 1 risk row "boring + reliable". Phase 2 replaces this with the
// joule market.
// ---------------------------------------------------------------
fn agent_for_intent(intent: &str) -> Option<&'static str> {
    Some(match intent {
        "probe_provider" | "retire_failing" => "charon",
        "collect_joule_samples"
        | "summarize_by_agent"
        | "summarize_by_provider_tier"
        | "emit_ledger_summary" => "plutus",
        "scan_knowledge_sources" | "diff_against_last_index" | "reindex_changed" => "athena",
        "list_ledger_segments" | "archive_older_than" => "supervisor",
        "probe_seat" | "escalate_if_repeat_failure" => "oracle",
        _ => return None,
    })
}

#[derive(Debug, Default)]
pub struct DispatchPass {
    pub tasks_seen: usize,
    pub dispatched: Vec<String>,          // task ids
    pub no_route: Vec<String>,            // (task id) intents we don't know how to route
    pub already_terminal: Vec<String>,    // already-Complete/Failed contract tasks; nothing to do
    pub triad_unconsulted: Vec<String>,   // recorded as audit-flagged unconsulted decisions
    pub budget_blocked: Vec<String>, // task ids skipped because goal joule budget for today is exhausted
    pub bids_recorded: usize,        // total bids ledgered across all dispatched tasks
    pub market_collapses: Vec<String>, // task ids no agent was willing to bid on
    pub councils_held: usize,        // governance gate: extra Council deliberations ledgered
    pub council_joules_charged: f64, // total joule cost charged to goals for council look
    pub triad_passes: usize,         // live triad: Pass verdicts
    pub triad_conditionals: usize,   // live triad: Conditional verdicts
    pub triad_vetoes: Vec<String>,   // live triad: Fail outcomes recorded as veto evidence
    pub triad_blocked: Vec<String>,  // live triad: Fail outcomes blocked by policy
    pub action_gate_blocked: Vec<String>, // action-class gates blocked dispatch before execution
    pub aipkg_preflight_blocked: Vec<String>, // task ids blocked by failing AIPKG preflight
    pub aipkg_preflight_passed: usize, // task ids that passed AIPKG preflight
    pub capped_at: Option<usize>,    // dispatch loop bailed out at this count (rate cap)
    pub halted: bool,                // halt file present; dispatcher refused to act
}

// ---------------------------------------------------------------
// Joule estimator hook. Phase 2 step 4: every Decision carries a
// real joule estimate. The trait lives here (rather than in plutus)
// so loop_engine doesn't depend on plutus — plutus implements it.
// ---------------------------------------------------------------

/// Produces a joule estimate for a Task at dispatch time. Stamped
/// onto both the Task (`joule_cost_estimated`) and the Decision
/// (`joule_estimate`).
pub trait JouleEstimator: Send + Sync {
    fn estimate_for_task(&self, task: &Task) -> f64;
}

/// Default no-op estimator. Used by back-compat callers (and tests
/// that don't care about joules). Returns 0.0 — same behavior as
/// Phase 1.
pub struct ZeroJouleEstimator;

impl JouleEstimator for ZeroJouleEstimator {
    fn estimate_for_task(&self, _task: &Task) -> f64 {
        0.0
    }
}

// ---------------------------------------------------------------
// Triad consultation hook. Phase 2 step 6: dispatcher consults the
// real Triad and ledgers the verdict. The default policy preserves
// record-and-proceed behavior per Phase 1 closeout §6 Q1, but
// governance-gate config can now enable blocking on Triad Fail.
// ---------------------------------------------------------------

/// Produces a TriadOutcome for a Task. Same dependency-direction
/// trick as JouleEstimator: trait in core, real impl in governance.
pub trait TriadConsultant: Send + Sync {
    fn consult(&self, task: &Task) -> TriadOutcome;
}

// ---------------------------------------------------------------
// Internal joule market — bid layer (step 8). Agents post bids
// (agent_id + joule_cost + confidence). Orchestrator selects by
// confidence / joule_cost (effectiveness per joule). Bids + the
// selection are ledgered as DecisionClass::Bid so the market is
// auditable.
// ---------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentBid {
    pub agent_id: &'static str,
    pub joule_cost: f64,
    pub confidence: f64,
}

impl AgentBid {
    /// Selection score: effectiveness per joule. Larger is better.
    /// Joule_cost == 0 collapses to confidence (avoid div-by-zero).
    pub fn score(&self) -> f64 {
        if self.joule_cost <= f64::EPSILON {
            self.confidence
        } else {
            self.confidence / self.joule_cost
        }
    }
}

/// Produces bids for a Task. Default impl is `StaticBidBoard`
/// (single bidder per intent from the Phase 1 routing table).
pub trait BidBoard: Send + Sync {
    fn bids_for(&self, task: &Task) -> Vec<AgentBid>;
}

/// Default bid board: one bid per intent matching the Phase 1
/// hard-coded routing table, with the joule cost coming from the
/// caller-supplied estimator. Exists so the market machinery
/// runs end-to-end before per-agent capability registries land.
pub struct StaticBidBoard;

impl BidBoard for StaticBidBoard {
    fn bids_for(&self, task: &Task) -> Vec<AgentBid> {
        match agent_for_intent(&task.task_type) {
            Some(agent) => vec![AgentBid {
                agent_id: agent,
                joule_cost: task.joule_cost_estimated.max(0.001),
                confidence: 1.0,
            }],
            None => Vec::new(),
        }
    }
}

/// Phase 1 stub — verdicts are honest about not having actually
/// consulted. Kept for tests and back-compat.
pub struct UnconsultedTriad;

impl TriadConsultant for UnconsultedTriad {
    fn consult(&self, _task: &Task) -> TriadOutcome {
        let unconsulted = PhilosopherVerdict {
            verdict: TriadVerdict::Pass,
            reason: Some("v0.1 stub: triad not consulted".into()),
        };
        TriadOutcome {
            verdict: TriadVerdict::Pass,
            aurelius: unconsulted.clone(),
            bacon: unconsulted.clone(),
            sun_tzu: unconsulted,
        }
    }
}

/// Hard cap on dispatches per single tick. Per PRD §Phase 1 risk row:
/// "runaway loops, unbounded task generation". Default leaves headroom
/// for the seed goals (16 tasks per day) but bounds a misbehaving
/// planner.
pub const DEFAULT_DISPATCH_CAP_PER_TICK: usize = 64;

/// Halt file the dispatcher checks each tick. Per PRD §Phase 2 kill
/// switch deliverable, surfaced one phase early because a halt
/// mechanism is the cheapest insurance against runaways.
pub const HALT_FILE_NAME: &str = "HALT";

#[derive(Debug, Default)]
pub struct ReflectPass {
    pub tasks_seen: usize,
    pub reflections_written: Vec<String>, // task ids
    pub already_reflected: Vec<String>,
    pub no_plan_link: Vec<String>, // task missing plan_id; cannot score
}

// ---------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------

/// Run one dispatcher pass. Reads contract Tasks from `queue_path`,
/// routes Pending ones, ledgers a Decision per dispatch, and writes
/// the simulated terminal task state back to the queue (as a fresh
/// jsonl line — last-write-wins on task id when re-read).
pub fn dispatch(state: &StateRoot, queue_path: &Path) -> Result<DispatchPass> {
    dispatch_with_cap(state, queue_path, DEFAULT_DISPATCH_CAP_PER_TICK)
}

pub fn dispatch_with_cap(
    state: &StateRoot,
    queue_path: &Path,
    cap_per_tick: usize,
) -> Result<DispatchPass> {
    dispatch_with_cap_and_estimator(state, queue_path, cap_per_tick, &ZeroJouleEstimator)
}

/// Phase 2 hook: dispatch with a real joule estimator. The estimate
/// is stamped onto `Task::joule_cost_estimated` and the ledgered
/// `Decision::joule_estimate` so the Reflector sees an estimate-vs-
/// actual delta in `Reflection::joule_variance()`.
pub fn dispatch_with_cap_and_estimator(
    state: &StateRoot,
    queue_path: &Path,
    cap_per_tick: usize,
    estimator: &dyn JouleEstimator,
) -> Result<DispatchPass> {
    dispatch_full(
        state,
        queue_path,
        cap_per_tick,
        estimator,
        &UnconsultedTriad,
        &StaticBidBoard,
        &GovernanceGates::permissive(),
    )
}

/// Phase 2 step 6+8+9: dispatch with a joule estimator, a triad
/// consultant, a bid board, and governance gates. Triad verdicts
/// are recorded; Fail remains record-and-proceed unless the Dispatch
/// governance policy sets `block_on_triad_fail`. Bids are ledgered as
/// DecisionClass::Bid. Governance gates ledger an
/// extra DecisionClass::Governance "council deliberation" line
/// when a class's policy requires it, charging the deliberation's
/// joule cost to the task's goal budget.
pub fn dispatch_full(
    state: &StateRoot,
    queue_path: &Path,
    cap_per_tick: usize,
    estimator: &dyn JouleEstimator,
    triad: &dyn TriadConsultant,
    bid_board: &dyn BidBoard,
    gates: &GovernanceGates,
) -> Result<DispatchPass> {
    dispatch_full_with_affordability(
        state,
        queue_path,
        cap_per_tick,
        estimator,
        triad,
        bid_board,
        gates,
        &AllowAllAffordability,
    )
}

/// Dispatch with an explicit runtime affordability provider. This is the
/// governance integration point implemented by `EconomicsEngine`; compatibility
/// entrypoints retain allow-all behavior until a provider is supplied.
// This compatibility boundary keeps each governance collaborator explicit. A
// parameter object would obscure the public integration contract and break
// existing consumers without reducing the underlying dependencies.
#[allow(clippy::too_many_arguments)]
pub fn dispatch_full_with_affordability(
    state: &StateRoot,
    queue_path: &Path,
    cap_per_tick: usize,
    estimator: &dyn JouleEstimator,
    triad: &dyn TriadConsultant,
    bid_board: &dyn BidBoard,
    gates: &GovernanceGates,
    affordability: &dyn AffordabilityPolicy,
) -> Result<DispatchPass> {
    let mut pass = DispatchPass::default();

    // Halt file short-circuits everything. Refuse to dispatch but
    // leave reflector + planner free to run; halt is about stopping
    // *new* work, not preventing accounting on prior work.
    let halt_path = state.root().join(HALT_FILE_NAME);
    if halt_path.exists() {
        pass.halted = true;
        return Ok(pass);
    }

    let tasks = state::read_contract_tasks(queue_path)?;
    pass.tasks_seen = tasks.len();

    // Last-write-wins: collapse to most recent record per id.
    let mut latest: HashMap<String, Task> = HashMap::new();
    for t in tasks {
        latest.insert(t.id.to_string(), t);
    }

    let ledger = ledger_for(state)?;

    let loop_observability_config = LoopObservabilityConfig::from_env();
    let mut tick_probe = LatencyProbe::new(loop_observability_config.max_latency_samples);
    tick_probe.with_kind(DecisionLatencyKind::LoopTick);

    let now_ts = Utc::now().format("%Y%m%dT%H%M%S").to_string();

    // ----- Phase 2 step 7: per-goal joule budget bookkeeping. -----
    // Build today's plan_id -> goal_id map and the goal -> daily
    // budget map. Then accumulate today's already-stamped joule
    // estimates from terminal tasks tied to today's plans.
    let today_suffix = format!("_{}", Utc::now().format("%Y%m%d"));
    let mut plan_to_goal: HashMap<String, String> = HashMap::new();
    if let Ok(plans) = state::list_plans(state) {
        for p in plans {
            if p.id.ends_with(&today_suffix) {
                plan_to_goal.insert(p.id.clone(), p.goal_id.clone());
            }
        }
    }
    let mut goal_budget: HashMap<String, f64> = HashMap::new();
    if let Ok(goals) = state::list_goals(state) {
        for g in goals {
            if let Some(b) = g.joule_budget_per_day {
                goal_budget.insert(g.id.clone(), b);
            }
        }
    }
    let mut goal_spent: HashMap<String, f64> = HashMap::new();
    for t in latest.values() {
        let Some(pid) = t.plan_id.as_deref() else {
            continue;
        };
        let Some(gid) = plan_to_goal.get(pid) else {
            continue;
        };
        if matches!(
            t.status,
            TaskStatus::Complete | TaskStatus::Failed { .. } | TaskStatus::Running
        ) {
            *goal_spent.entry(gid.clone()).or_insert(0.0) += t.joule_cost_estimated;
        }
    }
    let mut dispatch_seq: u64 = 0;

    for (id, mut task) in latest.into_iter() {
        match task.status {
            TaskStatus::Pending => {}
            _ => {
                pass.already_terminal.push(id);
                continue;
            }
        }

        if pass.dispatched.len() >= cap_per_tick {
            pass.capped_at = Some(cap_per_tick);
            break;
        }

        // AIPKG preflight gate: if a task carries a manifest, validate
        // it before allocating joule/bids/triad resources. Invalid or
        // failing manifests are recorded and skipped.
        if let Some(manifest) = task.aipkg_manifest.as_ref() {
            match manifest.validate() {
                Ok(_) => pass.aipkg_preflight_passed += 1,
                Err(err) => {
                    pass.aipkg_preflight_blocked.push(format!(
                        "{id}:{}",
                        err
                    ));
                    continue;
                }
            }
        }

        let intent = task.task_type.clone();

        // Joule estimate first — needed by both the budget gate
        // and (via task.joule_cost_estimated) the StaticBidBoard.
        let joule_estimate = estimator.estimate_for_task(&task);
        task.joule_cost_estimated = joule_estimate;

        let affordability_decision = gates.evaluate_affordability(affordability, joule_estimate);
        if !affordability_decision.allowed {
            let mut budget_decision = Decision::new(
                format!("budget_{now_ts}_{dispatch_seq:04}"),
                DecisionClass::Budget,
                task.id.to_string(),
                affordability_decision.policy,
                format!(
                    "runtime affordability denied cost {joule_estimate:.2}: {}",
                    affordability_decision.reason
                ),
                triad.consult(&task),
            );
            budget_decision.joule_estimate = joule_estimate;
            budget_decision.extensions.insert(
                "affordability".to_owned(),
                serde_json::to_value(&affordability_decision)?,
            );
            ledger.append(&budget_decision)?;
            dispatch_seq += 1;
            pass.budget_blocked.push(format!(
                "{id}:policy={}:cost={joule_estimate:.2}:reason={}",
                affordability_decision.policy, affordability_decision.reason
            ));
            continue;
        }

        // Per-goal joule budget enforcement (step 7). If today's
        // spend on this task's goal would exceed the daily budget,
        // skip the task — it stays Pending and is reconsidered at
        // the next UTC-day rollover (today's plan_id stops matching).
        if let Some(pid) = task.plan_id.as_deref() {
            if let Some(gid) = plan_to_goal.get(pid) {
                if let Some(budget) = goal_budget.get(gid).copied() {
                    let spent = goal_spent.get(gid).copied().unwrap_or(0.0);
                    if spent + joule_estimate > budget {
                        pass.budget_blocked.push(format!(
                            "{id}:goal={gid}:spent={spent:.2}/budget={budget:.2}"
                        ));
                        continue;
                    }
                }
            }
        }

        // Joule market (step 8). Ask the bid board; if no agent
        // bids, this is a market collapse and the task waits.
        let bids = bid_board.bids_for(&task);
        if bids.is_empty() {
            pass.no_route.push(format!("{id}:{intent}"));
            pass.market_collapses.push(id.clone());
            continue;
        }
        let mut best_idx = 0;
        let mut best_score = bids[0].score();
        for (i, b) in bids.iter().enumerate().skip(1) {
            let s = b.score();
            if s > best_score {
                best_score = s;
                best_idx = i;
            }
        }
        let agent = bids[best_idx].agent_id;

        // Triad consultation (record-and-proceed). Consult once,
        // share the outcome between the Bid and Dispatch lines.
        let triad_outcome = triad.consult(&task);

        // Ledger a Bid decision listing all bidders + the winner.
        let bid_dec_id = format!("bid_{now_ts}_{:04}", dispatch_seq);
        let mut bid_dec = Decision::new(
            bid_dec_id.clone(),
            DecisionClass::Bid,
            task.id.to_string(),
            agent,
            format!(
                "joule market: chose {} (score={:.3} from {} bidder{})",
                agent,
                best_score,
                bids.len(),
                if bids.len() == 1 { "" } else { "s" }
            ),
            triad_outcome.clone(),
        );
        bid_dec.options_considered = bids
            .iter()
            .map(|b| format!("{}@j={:.3},c={:.2}", b.agent_id, b.joule_cost, b.confidence))
            .collect();
        bid_dec.joule_estimate = bids[best_idx].joule_cost;
        ledger.append(&bid_dec)?;
        pass.bids_recorded += bids.len();

        // Council gate (step 9). If policy requires it for the Bid
        // class, ledger a Governance deliberation line with the
        // configured joule cost and charge it to the goal budget.
        let bid_policy = gates.policy_for(DecisionClass::Bid);
        if bid_policy.require_council {
            let council_id = format!("gov_{now_ts}_{:04}b", dispatch_seq);
            let mut gov_dec = Decision::new(
                council_id,
                DecisionClass::Governance,
                bid_dec_id.clone(),
                "council",
                format!(
                    "council deliberation gate for bid {bid_dec_id} (cost {:.2} J)",
                    bid_policy.council_joule_cost
                ),
                triad_outcome.clone(),
            );
            gov_dec.joule_estimate = bid_policy.council_joule_cost;
            ledger.append(&gov_dec)?;
            pass.councils_held += 1;
            pass.council_joules_charged += bid_policy.council_joule_cost;
            if let Some(pid) = task.plan_id.as_deref() {
                if let Some(gid) = plan_to_goal.get(pid) {
                    *goal_spent.entry(gid.clone()).or_insert(0.0) += bid_policy.council_joule_cost;
                }
            }
        }

        let dec_id = format!("dec_{now_ts}_{:04}", dispatch_seq);
        dispatch_seq += 1;
        let verdict = triad_outcome.verdict;
        let stubbed = matches!(
            triad_outcome.aurelius.reason.as_deref(),
            Some(s) if s.starts_with("v0.1 stub")
        );
        let mut dec = Decision::new(
            dec_id.clone(),
            DecisionClass::Dispatch,
            task.id.to_string(),
            agent,
            format!("joule market route: intent '{intent}' -> {agent} (bid {bid_dec_id})"),
            triad_outcome.clone(),
        );
        dec.joule_estimate = joule_estimate;

        // Compact policy metadata makes the dispatch decision auditable
        // without changing default permissive behavior.
        let dispatch_policy = gates.policy_for(DecisionClass::Dispatch);
        let action_gate = action_gate_for_task(&task, gates);
        dec.extensions.insert(
            "governance_policy".to_string(),
            json!({
                "policy_mode": dispatch_policy.policy_mode(),
                "blocks_on_triad_fail": dispatch_policy.blocks_on_triad_fail(),
                "require_council": dispatch_policy.require_council,
                "council_joule_cost": dispatch_policy.council_joule_cost,
            }),
        );
        dec.extensions.insert(
            "affordability".to_owned(),
            serde_json::to_value(&affordability_decision)?,
        );
        dec.extensions
            .insert("action_gate".to_string(), action_gate.to_json());
        ledger.append(&dec)?;
        append_action_gate_receipt(state, &dec_id, &task, &action_gate)?;

        // Council gate for the Dispatch line itself.
        if dispatch_policy.require_council {
            let council_id = format!("gov_{now_ts}_{:04}d", dispatch_seq);
            let mut gov_dec = Decision::new(
                council_id,
                DecisionClass::Governance,
                dec_id.clone(),
                "council",
                format!(
                    "council deliberation gate for dispatch {dec_id} (cost {:.2} J)",
                    dispatch_policy.council_joule_cost
                ),
                triad_outcome.clone(),
            );
            gov_dec.joule_estimate = dispatch_policy.council_joule_cost;
            ledger.append(&gov_dec)?;
            pass.councils_held += 1;
            pass.council_joules_charged += dispatch_policy.council_joule_cost;
            if let Some(pid) = task.plan_id.as_deref() {
                if let Some(gid) = plan_to_goal.get(pid) {
                    *goal_spent.entry(gid.clone()).or_insert(0.0) +=
                        dispatch_policy.council_joule_cost;
                }
            }
        }
        let block_on_triad_fail =
            matches!(verdict, TriadVerdict::Fail) && dispatch_policy.blocks_on_triad_fail();

        if stubbed {
            pass.triad_unconsulted.push(dec_id.clone());
        } else {
            match verdict {
                TriadVerdict::Pass => pass.triad_passes += 1,
                TriadVerdict::Conditional => pass.triad_conditionals += 1,
                TriadVerdict::Fail => pass.triad_vetoes.push(dec_id.clone()),
            }
        }

        if block_on_triad_fail {
            pass.triad_blocked.push(dec_id);
            continue;
        }
        if !action_gate.allowed {
            pass.action_gate_blocked
                .push(format!("{}:{}", dec_id, action_gate.reason));
            continue;
        }

        // Simulate execution: assign + complete with stub result.
        task.assign(agent);
        task.start_execution();
        task.complete(json!({
            "v0_1_stub": true,
            "intent": intent,
            "agent": agent,
        }));
        // Append the new terminal state.
        state::append_task(queue_path, &task)?;
        if let Some(pid) = task.plan_id.as_deref() {
            if let Some(gid) = plan_to_goal.get(pid) {
                *goal_spent.entry(gid.clone()).or_insert(0.0) += joule_estimate;
            }
        }
        pass.dispatched.push(id);
    }

    if loop_observability_config.economy_snapshot_enabled {
        let _ = tick_probe
            .with_kind(DecisionLatencyKind::EconomySnapshot)
            .sample();
    }

    Ok(pass)
}

#[derive(Debug, Clone)]
struct ActionGate {
    action_class: String,
    policy_mode: GovernancePolicyMode,
    allowed: bool,
    reason: String,
    required_authority: String,
}

impl ActionGate {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "contract": "arda.action_classification.v1",
            "action_class": self.action_class,
            "policy_mode": self.policy_mode,
            "allowed": self.allowed,
            "reason": self.reason,
            "required_authority": self.required_authority
        })
    }
}

fn action_gate_for_task(task: &Task, gates: &GovernanceGates) -> ActionGate {
    let action_class = normalize_action_class(&task.task_type);
    let policy = gates.policy_for_action_class(&action_class);
    let policy_mode = policy.policy_mode();
    let (allowed, reason, required_authority) = match policy_mode {
        GovernancePolicyMode::ObserveOnly | GovernancePolicyMode::RecordAndProceed => (
            true,
            "record_and_proceed_or_observe_only".to_string(),
            "single_gate".to_string(),
        ),
        GovernancePolicyMode::BlockOnFail => (
            true,
            "triad_policy_may_block_on_fail".to_string(),
            "triad_2_of_3".to_string(),
        ),
        GovernancePolicyMode::EscalateToHuman => (
            false,
            "human_required_action_class".to_string(),
            "human".to_string(),
        ),
        GovernancePolicyMode::RequireIndependentReceipts => (
            false,
            "independent_receipts_required".to_string(),
            "independent_receipts".to_string(),
        ),
    };
    let unknown_blocked = action_class == "unknown_action";
    ActionGate {
        action_class,
        policy_mode,
        allowed: allowed && !unknown_blocked,
        reason: if unknown_blocked {
            "unknown_action_class".to_string()
        } else {
            reason
        },
        required_authority: if unknown_blocked {
            "review".to_string()
        } else {
            required_authority
        },
    }
}

fn normalize_action_class(task_type: &str) -> String {
    let value = task_type.trim();
    match value {
        "read_only_audit"
        | "bounded_research"
        | "documentation_indexing"
        | "safe_exports"
        | "routine_status_reporting"
        | "non_destructive_benchmarking"
        | "local_refactors"
        | "routine_maintenance"
        | "provider_status_check"
        | "provider_reload"
        | "provider_reroute"
        | "service_restart"
        | "service_disable"
        | "destructive_delete"
        | "archive_or_retention"
        | "generated_artifact_cleanup"
        | "credential_rotation_or_disclosure"
        | "funds_movement"
        | "legal_commitment"
        | "customer_commitment"
        | "external_customer_commitment_without_prior_scope"
        | "fleet_reimage"
        | "governance_policy_change"
        | "autonomy_level_increase" => value.to_string(),
        "scan_knowledge_sources" | "diff_against_last_index" | "reindex_changed" => {
            "read_only_audit".to_string()
        }
        "probe_provider" => "provider_status_check".to_string(),
        "collect_joule_samples"
        | "summarize_by_agent"
        | "summarize_by_provider_tier"
        | "emit_ledger_summary" => "routine_status_reporting".to_string(),
        "list_ledger_segments" => "read_only_audit".to_string(),
        "archive_older_than" => "archive_or_retention".to_string(),
        "probe_seat" | "escalate_if_repeat_failure" => "read_only_audit".to_string(),
        _ => "unknown_action".to_string(),
    }
}

fn append_action_gate_receipt(
    state: &StateRoot,
    decision_id: &str,
    task: &Task,
    gate: &ActionGate,
) -> Result<()> {
    let Some(core_dir) = state.root().parent() else {
        return Ok(());
    };
    let Some(repo_root) = core_dir.parent() else {
        return Ok(());
    };
    let path = repo_root
        .join("data")
        .join("governance")
        .join("action_gate_receipts.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let receipt = json!({
        "schema_version": "arda.action_gate_receipt.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "decision_id": decision_id,
        "task_id": task.id.to_string(),
        "task_type": task.task_type,
        "action_gate": gate.to_json(),
        "source": "arda-core.dispatch_full",
        "mutation_performed": false
    });
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    use std::io::Write;
    writeln!(f, "{}", serde_json::to_string(&receipt)?)?;
    Ok(())
}

// ---------------------------------------------------------------
// Reflector
// ---------------------------------------------------------------

/// Run one reflector pass. Reads recent Tasks; for each Complete or
/// Failed task that carries plan_id + plan_step_index and doesn't
/// already have a Reflection on disk, emit one with trivial scoring.
pub fn reflect(state: &StateRoot, queue_path: &Path) -> Result<ReflectPass> {
    let mut pass = ReflectPass::default();
    let tasks = state::read_contract_tasks(queue_path)?;

    // Last-write-wins per id (mirrors dispatcher's view).
    let mut latest: HashMap<String, Task> = HashMap::new();
    for t in tasks {
        latest.insert(t.id.to_string(), t);
    }

    let already_reflected: std::collections::HashSet<String> = state::list_reflections(state)?
        .into_iter()
        .map(|r| r.task_id)
        .collect();

    for (id, task) in latest.into_iter() {
        let terminal = matches!(
            task.status,
            TaskStatus::Complete | TaskStatus::Failed { .. }
        );
        if !terminal {
            continue;
        }
        pass.tasks_seen += 1;

        if already_reflected.contains(&id) {
            pass.already_reflected.push(id);
            continue;
        }

        let Some(plan_id) = task.plan_id.clone() else {
            pass.no_plan_link.push(id);
            continue;
        };

        let (outcome, score) = match &task.status {
            TaskStatus::Complete => (ReflectionOutcome::Success, 1.0),
            TaskStatus::Failed { .. } => (ReflectionOutcome::Failure, 0.0),
            _ => unreachable!(),
        };

        let r_id = format!("ref_{}", id);
        let mut r = Reflection::new(r_id, &id, &plan_id, outcome, score);
        r.joule_estimated = task.joule_cost_estimated;
        r.joule_actual = task.joule_cost_actual;
        r.narrative = format!(
            "v0.1 trivial score: task ended in {:?}; plan_step_index={:?}",
            task.status, task.plan_step_index
        );
        state::write_reflection(state, &r)?;
        pass.reflections_written.push(id);
    }

    Ok(pass)
}

// ---------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------

fn ledger_for(state: &StateRoot) -> Result<Ledger> {
    // The state root is .../core/state — the canonical ledger dir is
    // .../core/state/ledger per FILE_LAYOUT §3 / config.paths.ledger_dir.
    Ledger::new(state.root().join("ledger"))
}

// re-export for the CLI tick
#[derive(Debug, Serialize, Deserialize)]
pub struct TickSummary {
    pub queue_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aipkg::{AipkgGovernance, AipkgManifest, AipkgPreflight, AipkgReceiptPolicy};
    use crate::contract::{Goal, GoalPriority, Plan, PlanStep};

    fn tmp_state() -> (tempfile::TempDir, StateRoot, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let state = StateRoot::new(dir.path().join("core/state"));
        let queue = dir.path().join("core/projects/tasks/queue.jsonl");
        (dir, state, queue)
    }

    fn seed_one_plan_with_tasks(
        state: &StateRoot,
        queue: &Path,
        intent: &str,
    ) -> (Plan, Vec<Task>) {
        let g = Goal::new("g1", "T", "I", "owner", GoalPriority::Medium);
        state::write_goal(state, &g).unwrap();
        let plan = Plan::new(
            "plan_g1_today",
            "g1",
            "summary",
            vec![PlanStep {
                intent: intent.into(),
                params: json!({}),
            }],
        );
        state::write_plan(state, &plan).unwrap();
        let task = Task::new("t1: probe", intent).with_plan_lineage(&plan.id, 0);
        state::append_task(queue, &task).unwrap();
        (plan, vec![task])
    }

    #[test]
    fn dispatch_routes_known_intent_and_ledgers_decision() {
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch(&st, &q).unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.no_route.len(), 0);
        assert_eq!(pass.triad_unconsulted.len(), 1);

        // Ledger file exists with at least one entry.
        let today = Utc::now().format("%Y-%m-%d");
        let ledger_file = st
            .root()
            .join("ledger")
            .join(format!("ledger_{today}.jsonl"));
        let content = std::fs::read_to_string(&ledger_file).unwrap();
        assert!(content.contains("dispatch"));
        assert!(content.contains("joule market route"));
    }

    #[test]
    fn dispatch_marks_unknown_intent_as_no_route() {
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "made_up_intent");
        let pass = dispatch(&st, &q).unwrap();
        assert_eq!(pass.dispatched.len(), 0);
        assert_eq!(pass.no_route.len(), 1);
    }

    #[test]
    fn dispatch_honors_halt_file() {
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        std::fs::create_dir_all(st.root()).unwrap();
        std::fs::write(st.root().join(HALT_FILE_NAME), b"halt").unwrap();
        let pass = dispatch(&st, &q).unwrap();
        assert!(pass.halted);
        assert_eq!(pass.dispatched.len(), 0);
    }

    #[test]
    fn dispatch_caps_per_tick() {
        let (_d, st, q) = tmp_state();
        // Seed 3 tasks; cap at 2.
        let g = Goal::new("g1", "T", "I", "owner", GoalPriority::Medium);
        state::write_goal(&st, &g).unwrap();
        let plan = Plan::new(
            "plan_g1_today",
            "g1",
            "summary",
            (0..3)
                .map(|_| PlanStep {
                    intent: "probe_provider".into(),
                    params: json!({}),
                })
                .collect(),
        );
        state::write_plan(&st, &plan).unwrap();
        for i in 0..3 {
            let t = Task::new(format!("t{i}"), "probe_provider").with_plan_lineage(&plan.id, i);
            state::append_task(&q, &t).unwrap();
        }
        let pass = dispatch_with_cap(&st, &q, 2).unwrap();
        assert_eq!(pass.dispatched.len(), 2);
        assert_eq!(pass.capped_at, Some(2));
    }

    #[test]
    fn dispatch_cap_zero_dispatches_nothing() {
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_with_cap(&st, &q, 0).unwrap();
        assert_eq!(pass.dispatched.len(), 0);
        assert_eq!(pass.capped_at, Some(0));
    }

    #[test]
    fn dispatch_picks_highest_market_score_and_ledgers_bid() {
        struct TwoBidders;
        impl BidBoard for TwoBidders {
            fn bids_for(&self, _task: &Task) -> Vec<AgentBid> {
                vec![
                    // High joule, high confidence: 0.9/9.0 = 0.10
                    AgentBid {
                        agent_id: "expensive",
                        joule_cost: 9.0,
                        confidence: 0.9,
                    },
                    // Low joule, decent confidence: 0.6/1.0 = 0.60 — wins.
                    AgentBid {
                        agent_id: "cheap",
                        joule_cost: 1.0,
                        confidence: 0.6,
                    },
                ]
            }
        }
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &TwoBidders,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.bids_recorded, 2);

        let today = Utc::now().format("%Y-%m-%d");
        let ledger_file = st
            .root()
            .join("ledger")
            .join(format!("ledger_{today}.jsonl"));
        let content = std::fs::read_to_string(&ledger_file).unwrap();
        assert!(content.contains("\"decision_class\":\"bid\""));
        assert!(content.contains("\"chosen\":\"cheap\""));
        // Both bidders appear in options_considered.
        assert!(content.contains("expensive@j=9"));
        assert!(content.contains("cheap@j=1"));
    }

    #[test]
    fn dispatch_ledgers_council_when_gate_required() {
        let raw = r#"
default:
  require_council: false
  council_joule_cost: 0.0
classes:
  dispatch:
    require_council: true
    council_joule_cost: 1.5
"#;
        let gates = GovernanceGates::load_from_str(raw).unwrap();
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &StaticBidBoard,
            &gates,
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.councils_held, 1);
        assert!((pass.council_joules_charged - 1.5).abs() < 1e-9);

        let today = Utc::now().format("%Y-%m-%d");
        let ledger_file = st
            .root()
            .join("ledger")
            .join(format!("ledger_{today}.jsonl"));
        let content = std::fs::read_to_string(&ledger_file).unwrap();
        assert!(content.contains("\"decision_class\":\"governance\""));
        assert!(content.contains("council deliberation gate"));
    }

    #[test]
    fn dispatch_records_action_gate_receipt_for_safe_action() {
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.action_gate_blocked.len(), 0);

        let receipt_file = st
            .root()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("data/governance/action_gate_receipts.jsonl");
        let content = std::fs::read_to_string(receipt_file).unwrap();
        assert!(content.contains("\"schema_version\":\"arda.action_gate_receipt.v1\""));
        assert!(content.contains("\"action_class\":\"provider_status_check\""));
        assert!(content.contains("\"allowed\":true"));
    }

    #[test]
    fn dispatch_blocks_human_required_action_class_before_execution() {
        struct AnyBidder;
        impl BidBoard for AnyBidder {
            fn bids_for(&self, _task: &Task) -> Vec<AgentBid> {
                vec![AgentBid {
                    agent_id: "warden",
                    joule_cost: 1.0,
                    confidence: 1.0,
                }]
            }
        }
        let raw = r#"
default:
  policy_mode: record_and_proceed
action_classes:
  destructive_delete:
    policy_mode: escalate_to_human
"#;
        let gates = GovernanceGates::load_from_str(raw).unwrap();
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "destructive_delete");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &AnyBidder,
            &gates,
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 0);
        assert_eq!(pass.action_gate_blocked.len(), 1);

        let tasks = state::read_contract_tasks(&q).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(matches!(tasks[0].status, TaskStatus::Pending));

        let receipt_file = st
            .root()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("data/governance/action_gate_receipts.jsonl");
        let content = std::fs::read_to_string(receipt_file).unwrap();
        assert!(content.contains("\"action_class\":\"destructive_delete\""));
        assert!(content.contains("\"allowed\":false"));
        assert!(content.contains("human_required_action_class"));
    }

    #[test]
    fn dispatch_records_market_collapse_when_no_bidders() {
        struct NoBidders;
        impl BidBoard for NoBidders {
            fn bids_for(&self, _task: &Task) -> Vec<AgentBid> {
                Vec::new()
            }
        }
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &NoBidders,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 0);
        assert_eq!(pass.market_collapses.len(), 1);
        assert_eq!(pass.no_route.len(), 1);
    }

    #[test]
    fn dispatch_skips_terminal_tasks_and_does_not_double_count_budget() {
        let (_d, st, q) = tmp_state();
        let g = Goal::new("g1", "T", "I", "owner", GoalPriority::Medium);
        state::write_goal(&st, &g).unwrap();
        let today_suffix = format!("_{}", Utc::now().format("%Y%m%d"));
        let plan_id = format!("plan_g1{today_suffix}");
        let plan = Plan::new(
            &plan_id,
            "g1",
            "summary",
            vec![
                PlanStep {
                    intent: "probe_provider".into(),
                    params: json!({}),
                },
                PlanStep {
                    intent: "probe_provider".into(),
                    params: json!({}),
                },
            ],
        );
        state::write_plan(&st, &plan).unwrap();

        let mut t0 = Task::new("t0", "probe_provider").with_plan_lineage(&plan_id, 0);
        t0.status = TaskStatus::Complete;
        t0.joule_cost_estimated = 4.0;
        let t1 = Task::new("t1", "probe_provider").with_plan_lineage(&plan_id, 1);
        state::append_task(&q, &t0).unwrap();
        state::append_task(&q, &t1).unwrap();

        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.budget_blocked.len(), 0);
        assert_eq!(pass.already_terminal, vec![t0.id.to_string()]);
    }

    #[test]
    fn dispatch_cap_limits_dispatched_task_count() {
        let (_d, st, q) = tmp_state();
        let g = Goal::new("g1", "T", "I", "owner", GoalPriority::Medium);
        state::write_goal(&st, &g).unwrap();
        let today_suffix = format!("_{}", Utc::now().format("%Y%m%d"));
        let plan_id = format!("plan_g1{today_suffix}");
        let plan = Plan::new(
            &plan_id,
            "g1",
            "summary",
            (0..3)
                .map(|_| PlanStep {
                    intent: "probe_provider".into(),
                    params: json!({}),
                })
                .collect(),
        );
        state::write_plan(&st, &plan).unwrap();
        for i in 0..3 {
            let t = Task::new(format!("t{i}"), "probe_provider").with_plan_lineage(&plan_id, i);
            state::append_task(&q, &t).unwrap();
        }

        let pass = dispatch_full(
            &st,
            &q,
            1,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.capped_at, Some(1));
        assert_eq!(pass.budget_blocked.len(), 0);
        assert_eq!(pass.no_route.len(), 0);
    }

    #[test]
    fn dispatch_blocks_budget_when_estimator_reports_high_joule_cost() {
        struct OneHundredJoule;
        impl JouleEstimator for OneHundredJoule {
            fn estimate_for_task(&self, _task: &Task) -> f64 {
                100.0
            }
        }

        let (_d, st, q) = tmp_state();
        let g = Goal::new("g1", "T", "I", "owner", GoalPriority::Medium);
        state::write_goal(&st, &g).unwrap();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_with_cap_and_estimator(&st, &q, 64, &OneHundredJoule).unwrap();
        assert_eq!(pass.dispatched.len(), 1);
        // Budget gate does not reject a single unknown task for a high estimator.
        assert_eq!(pass.budget_blocked.len(), 0);
    }

    #[test]
    fn dispatch_blocks_when_goal_budget_exhausted() {
        struct FixedThree;
        impl JouleEstimator for FixedThree {
            fn estimate_for_task(&self, _task: &Task) -> f64 {
                3.0
            }
        }
        let (_d, st, q) = tmp_state();
        // Goal with a 5-joule daily budget.
        let mut g = Goal::new("g1", "T", "I", "owner", GoalPriority::Medium);
        g.joule_budget_per_day = Some(5.0);
        state::write_goal(&st, &g).unwrap();

        // Plan with today's-suffix id so the budget gate sees it.
        let today_suffix = format!("_{}", Utc::now().format("%Y%m%d"));
        let plan_id = format!("plan_g1{today_suffix}");
        let plan = Plan::new(
            &plan_id,
            "g1",
            "summary",
            (0..2)
                .map(|_| PlanStep {
                    intent: "probe_provider".into(),
                    params: json!({}),
                })
                .collect(),
        );
        state::write_plan(&st, &plan).unwrap();
        for i in 0..2 {
            let t = Task::new(format!("t{i}"), "probe_provider").with_plan_lineage(&plan_id, i);
            state::append_task(&q, &t).unwrap();
        }

        let pass = dispatch_full(
            &st,
            &q,
            64,
            &FixedThree,
            &UnconsultedTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        // First task fits (spent 0 + 3 <= 5). Second task would push
        // to 6 > 5 and gets budget-blocked.
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.budget_blocked.len(), 1);
    }

    #[test]
    fn dispatch_uses_runtime_affordability_policy() {
        struct FixedThree;
        impl JouleEstimator for FixedThree {
            fn estimate_for_task(&self, _task: &Task) -> f64 {
                3.0
            }
        }
        struct TwoJouleRuntimeBudget;
        impl AffordabilityPolicy for TwoJouleRuntimeBudget {
            fn policy_name(&self) -> &'static str {
                "test_runtime_budget"
            }

            fn can_afford(&self, estimated_cost: f64) -> bool {
                estimated_cost <= 2.0
            }
        }

        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full_with_affordability(
            &st,
            &q,
            64,
            &FixedThree,
            &UnconsultedTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
            &TwoJouleRuntimeBudget,
        )
        .unwrap();

        assert!(pass.dispatched.is_empty());
        assert_eq!(pass.budget_blocked.len(), 1);
        assert!(pass.budget_blocked[0].contains("policy=test_runtime_budget"));
        assert!(pass.budget_blocked[0].contains("reason=budget_exceeded"));

        let today = Utc::now().format("%Y-%m-%d");
        let ledger_path = st
            .root()
            .join("ledger")
            .join(format!("ledger_{today}.jsonl"));
        let decisions = std::fs::read_to_string(ledger_path).unwrap();
        let budget_decision = decisions
            .lines()
            .map(|line| serde_json::from_str::<Decision>(line).unwrap())
            .find(|decision| decision.decision_class == DecisionClass::Budget)
            .expect("budget denial decision");
        assert_eq!(
            budget_decision.extensions["affordability"]["allowed"],
            false
        );
    }

    #[test]
    fn dispatch_records_triad_veto_without_blocking() {
        struct VetoTriad;
        impl TriadConsultant for VetoTriad {
            fn consult(&self, _task: &Task) -> TriadOutcome {
                let v = PhilosopherVerdict {
                    verdict: TriadVerdict::Fail,
                    reason: Some("test veto".into()),
                };
                TriadOutcome {
                    verdict: TriadVerdict::Fail,
                    aurelius: v.clone(),
                    bacon: v.clone(),
                    sun_tzu: v,
                }
            }
        }
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &VetoTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
        )
        .unwrap();
        // Record-and-proceed: vetoed but still dispatched.
        assert_eq!(pass.dispatched.len(), 1);
        assert_eq!(pass.triad_vetoes.len(), 1);
        assert_eq!(pass.triad_unconsulted.len(), 0);
        assert_eq!(pass.triad_passes, 0);
    }

    #[test]
    fn dispatch_blocks_triad_veto_when_policy_requires_it() {
        struct VetoTriad;
        impl TriadConsultant for VetoTriad {
            fn consult(&self, _task: &Task) -> TriadOutcome {
                let v = PhilosopherVerdict {
                    verdict: TriadVerdict::Fail,
                    reason: Some("test veto".into()),
                };
                TriadOutcome {
                    verdict: TriadVerdict::Fail,
                    aurelius: v.clone(),
                    bacon: v.clone(),
                    sun_tzu: v,
                }
            }
        }
        let raw = r#"
default:
  require_council: false
  council_joule_cost: 0.0
classes:
  dispatch:
    policy_mode: block_on_fail
"#;
        let gates = GovernanceGates::load_from_str(raw).unwrap();
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &VetoTriad,
            &StaticBidBoard,
            &gates,
        )
        .unwrap();

        assert_eq!(pass.dispatched.len(), 0);
        assert_eq!(pass.triad_vetoes.len(), 1);
        assert_eq!(pass.triad_blocked.len(), 1);

        let tasks = state::read_contract_tasks(&q).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(matches!(tasks[0].status, TaskStatus::Pending));

        let today = Utc::now().format("%Y-%m-%d");
        let ledger_file = st
            .root()
            .join("ledger")
            .join(format!("ledger_{today}.jsonl"));
        let ledger_content = std::fs::read_to_string(&ledger_file).unwrap();
        let decisions: Vec<Decision> = ledger_content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<Decision>(line).unwrap())
            .collect();
        let dispatch_decision = decisions
            .iter()
            .find(|decision| decision.decision_class == DecisionClass::Dispatch)
            .unwrap_or_else(|| panic!("dispatch decision should be ledgered before blocking"));
        assert_eq!(
            dispatch_decision
                .extensions
                .get("governance_policy")
                .and_then(|value| value.get("policy_mode"))
                .and_then(|value| value.as_str()),
            Some("block_on_fail")
        );
        assert_eq!(
            dispatch_decision
                .extensions
                .get("governance_policy")
                .and_then(|value| value.get("blocks_on_triad_fail"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn dispatch_stamps_joule_estimate_from_estimator() {
        struct FixedEstimator;
        impl JouleEstimator for FixedEstimator {
            fn estimate_for_task(&self, _task: &Task) -> f64 {
                3.5
            }
        }
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let pass = dispatch_with_cap_and_estimator(&st, &q, 64, &FixedEstimator).unwrap();
        assert_eq!(pass.dispatched.len(), 1);

        // Decision in the ledger carries 3.5.
        let today = Utc::now().format("%Y-%m-%d");
        let ledger_file = st
            .root()
            .join("ledger")
            .join(format!("ledger_{today}.jsonl"));
        let content = std::fs::read_to_string(&ledger_file).unwrap();
        assert!(
            content.contains("\"joule_estimate\":3.5"),
            "expected joule_estimate=3.5 in ledger; got: {content}"
        );

        // Task on disk also carries 3.5.
        let tasks = state::read_contract_tasks(&q).unwrap();
        let last = tasks.last().expect("at least one task");
        assert!((last.joule_cost_estimated - 3.5).abs() < 1e-9);
    }

    #[test]
    fn dispatch_accepts_tasks_with_valid_aipkg_manifest() {
        struct AnyBidder;
        impl BidBoard for AnyBidder {
            fn bids_for(&self, _task: &Task) -> Vec<AgentBid> {
                vec![AgentBid {
                    agent_id: "warden",
                    joule_cost: 1.0,
                    confidence: 1.0,
                }]
            }
        }

        let manifest = AipkgManifest {
            manifest_version: "0.1".into(),
            package_id: "org.arda.valid".into(),
            version: "0.1.0".into(),
            package_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            runtime_profile: "local-sovereign".into(),
            preflight: AipkgPreflight {
                zero_work_required: true,
                compatibility_required: true,
                quote_required: true,
            },
            governance: AipkgGovernance {
                triad_required: true,
                bacon_lite_required: true,
                joulework_budget_required: true,
                love_eq_guard_required: true,
                soterion_trace_required: true,
            },
            receipts: AipkgReceiptPolicy {
                preflight_required: true,
                execution_required: true,
                validation_required: true,
                settlement_optional: true,
                signatures_required: true,
            },
        };

        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let mut tasks = state::read_contract_tasks(&q).unwrap();
        let mut task = tasks.pop().expect("seeded task");
        task.aipkg_manifest = Some(manifest);
        state::append_task(&q, &task).unwrap();

        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &AnyBidder,
            &GovernanceGates::permissive(),
        )
        .unwrap();

        assert_eq!(pass.aipkg_preflight_passed, 1);
        assert_eq!(pass.aipkg_preflight_blocked.len(), 0);
        assert_eq!(pass.dispatched.len(), 1);
    }

    #[test]
    fn dispatch_blocks_tasks_with_invalid_aipkg_manifest() {
        let manifest = AipkgManifest {
            manifest_version: "not-0.1".into(),
            package_id: "no-namespace".into(),
            version: "0.1.0".into(),
            package_digest: "sha256:".into(),
            runtime_profile: "unknown".into(),
            preflight: AipkgPreflight {
                zero_work_required: false,
                compatibility_required: true,
                quote_required: true,
            },
            governance: AipkgGovernance {
                triad_required: true,
                bacon_lite_required: true,
                joulework_budget_required: true,
                love_eq_guard_required: true,
                soterion_trace_required: true,
            },
            receipts: AipkgReceiptPolicy {
                preflight_required: true,
                execution_required: true,
                validation_required: true,
                settlement_optional: false,
                signatures_required: true,
            },
        };

        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        let mut tasks = state::read_contract_tasks(&q).unwrap();
        let mut task = tasks.pop().expect("seeded task");
        task.aipkg_manifest = Some(manifest);
        state::append_task(&q, &task).unwrap();

        let pass = dispatch_full(
            &st,
            &q,
            64,
            &ZeroJouleEstimator,
            &UnconsultedTriad,
            &StaticBidBoard,
            &GovernanceGates::permissive(),
        )
        .unwrap();

        assert_eq!(pass.aipkg_preflight_passed, 0);
        assert_eq!(pass.dispatched.len(), 0);
        assert_eq!(pass.aipkg_preflight_blocked.len(), 1);
        let blocked = &pass.aipkg_preflight_blocked[0];
        assert!(blocked.contains("manifest_version must be 0.1"));
    }

    #[test]
    fn reflect_emits_one_per_completed_task() {
        let (_d, st, q) = tmp_state();
        seed_one_plan_with_tasks(&st, &q, "probe_provider");
        dispatch(&st, &q).unwrap();
        let pass = reflect(&st, &q).unwrap();
        assert_eq!(pass.reflections_written.len(), 1);
        assert_eq!(pass.no_plan_link.len(), 0);

        // Re-running is idempotent.
        let again = reflect(&st, &q).unwrap();
        assert_eq!(again.reflections_written.len(), 0);
        assert_eq!(again.already_reflected.len(), 1);
    }
}
