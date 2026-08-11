use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunGraphError, WorkerRouteClass};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerLimits {
    pub schema_version: String,
    pub max_total_workers: usize,
    pub max_local_workers: usize,
    pub max_hosted_workers: usize,
    pub max_run_cost_usd: f64,
    pub max_run_joules: f64,
    pub max_cycle_cost_usd: f64,
    pub max_cycle_joules: f64,
    pub max_daily_cost_usd: f64,
}

impl WorkerLimits {
    pub fn from_toml_str(raw: &str) -> Result<Self, WorkerLimitsError> {
        let limits: Self = toml::from_str(raw).map_err(WorkerLimitsError::Parse)?;
        if limits.schema_version != "arda.worker-limits.v1"
            || limits.max_total_workers == 0
            || limits.max_hosted_workers > limits.max_total_workers
            || limits.max_local_workers > limits.max_total_workers
            || [
                limits.max_run_cost_usd,
                limits.max_run_joules,
                limits.max_cycle_cost_usd,
                limits.max_cycle_joules,
                limits.max_daily_cost_usd,
            ]
            .into_iter()
            .any(|limit| !limit.is_finite() || limit < 0.0)
        {
            return Err(WorkerLimitsError::Invalid);
        }
        Ok(limits)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkerLimitsError {
    #[error("worker limit configuration is invalid")]
    Invalid,
    #[error("worker limit configuration could not be parsed: {0}")]
    Parse(toml::de::Error),
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            schema_version: "arda.worker-limits.v1".into(),
            max_total_workers: 4,
            max_local_workers: 1,
            max_hosted_workers: 3,
            max_run_cost_usd: 5.0,
            max_run_joules: 50_000.0,
            max_cycle_cost_usd: 10.0,
            max_cycle_joules: 100_000.0,
            max_daily_cost_usd: 25.0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkerUsage {
    pub active_total_workers: usize,
    pub active_local_workers: usize,
    pub active_hosted_workers: usize,
    pub spent_cost_usd: f64,
    pub spent_joules: f64,
    pub cycle_spent_cost_usd: f64,
    pub cycle_spent_joules: f64,
    pub daily_spent_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerAvailability {
    pub local_worker_available: bool,
    pub local_thermal_ok: bool,
    pub degraded_routes: BTreeSet<String>,
}

impl Default for WorkerAvailability {
    fn default() -> Self {
        Self {
            local_worker_available: true,
            local_thermal_ok: true,
            degraded_routes: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerProgressState {
    Selected,
    Queued,
    Running,
    Blocked,
    Retrying,
    Degraded,
    Succeeded,
    Failed,
    Cancelled,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerBlockReason {
    Dependency,
    Deadline,
    TotalCapacity,
    LocalCapacity,
    HostedCapacity,
    LocalUnavailable,
    LocalThermalPressure,
    RouteDegraded,
    CostBudget,
    EnergyBudget,
    CycleCostBudget,
    CycleEnergyBudget,
    DailyCostBudget,
    MissingWorkerContract,
    HumanDecisionRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerBlock {
    pub node_id: NodeId,
    pub reason: WorkerBlockReason,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulingDecision {
    pub selected: Vec<NodeId>,
    pub queued: Vec<WorkerBlock>,
    pub blocked: Vec<WorkerBlock>,
}

pub fn schedule_ready_workers(
    graph: &RunGraph,
    limits: &WorkerLimits,
    usage: &WorkerUsage,
    availability: &WorkerAvailability,
    now_unix_ms: u128,
) -> SchedulingDecision {
    let mut decision = SchedulingDecision::default();
    let mut total = usage.active_total_workers;
    let mut local = usage.active_local_workers;
    let mut hosted = usage.active_hosted_workers;
    let mut reserved_cost = 0.0;
    let mut reserved_joules = 0.0;
    let mut candidates = graph.nodes.iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));

    for node in candidates {
        if !matches!(
            node.state,
            NodeState::Pending | NodeState::Ready | NodeState::Blocked | NodeState::Failed
        ) {
            continue;
        }
        let Some(worker) = &node.worker else {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::MissingWorkerContract,
            });
            continue;
        };
        if worker.route_class == WorkerRouteClass::Human {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::HumanDecisionRequired,
            });
            continue;
        }
        if !dependencies_succeeded(graph, &node.id) {
            decision.queued.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::Dependency,
            });
            continue;
        }
        if worker.deadline_unix_ms <= now_unix_ms {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::Deadline,
            });
            continue;
        }
        if availability.degraded_routes.contains(&worker.route_id) {
            decision.queued.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::RouteDegraded,
            });
            continue;
        }
        let remaining_attempts = u64::from(node.retry.max_attempts)
            .saturating_sub(node.checkpoint.sequence)
            .max(1) as f64;
        let node_cost_reservation = node.budget.max_cost_usd * remaining_attempts;
        let node_joule_reservation = node.budget.max_joules * remaining_attempts;
        if usage.spent_cost_usd + reserved_cost + node_cost_reservation > limits.max_run_cost_usd {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::CostBudget,
            });
            continue;
        }
        if usage.spent_joules + reserved_joules + node_joule_reservation > limits.max_run_joules {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::EnergyBudget,
            });
            continue;
        }
        if usage.cycle_spent_cost_usd + reserved_cost + node_cost_reservation
            > limits.max_cycle_cost_usd
        {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::CycleCostBudget,
            });
            continue;
        }
        if usage.cycle_spent_joules + reserved_joules + node_joule_reservation
            > limits.max_cycle_joules
        {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::CycleEnergyBudget,
            });
            continue;
        }
        if usage.daily_spent_cost_usd + reserved_cost + node_cost_reservation
            > limits.max_daily_cost_usd
        {
            decision.blocked.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::DailyCostBudget,
            });
            continue;
        }
        if total >= limits.max_total_workers {
            decision.queued.push(WorkerBlock {
                node_id: node.id.clone(),
                reason: WorkerBlockReason::TotalCapacity,
            });
            continue;
        }
        match worker.route_class {
            WorkerRouteClass::Local if !availability.local_worker_available => {
                decision.queued.push(WorkerBlock {
                    node_id: node.id.clone(),
                    reason: WorkerBlockReason::LocalUnavailable,
                });
                continue;
            }
            WorkerRouteClass::Local if !availability.local_thermal_ok => {
                decision.queued.push(WorkerBlock {
                    node_id: node.id.clone(),
                    reason: WorkerBlockReason::LocalThermalPressure,
                });
                continue;
            }
            WorkerRouteClass::Local if local >= limits.max_local_workers => {
                decision.queued.push(WorkerBlock {
                    node_id: node.id.clone(),
                    reason: WorkerBlockReason::LocalCapacity,
                });
                continue;
            }
            WorkerRouteClass::Hosted if hosted >= limits.max_hosted_workers => {
                decision.queued.push(WorkerBlock {
                    node_id: node.id.clone(),
                    reason: WorkerBlockReason::HostedCapacity,
                });
                continue;
            }
            WorkerRouteClass::Human => unreachable!("human workers were handled above"),
            WorkerRouteClass::Local => local += 1,
            WorkerRouteClass::Hosted => hosted += 1,
            WorkerRouteClass::Deterministic => {}
        }
        total += 1;
        reserved_cost += node_cost_reservation;
        reserved_joules += node_joule_reservation;
        decision.selected.push(node.id.clone());
    }
    decision
}

pub fn mark_selected_workers_ready(
    graph: &mut RunGraph,
    selected: &[NodeId],
) -> Result<(), RunGraphError> {
    for node_id in selected {
        let state = graph
            .nodes
            .iter()
            .find(|node| &node.id == node_id)
            .map(|node| node.state)
            .ok_or_else(|| RunGraphError::MissingNode(node_id.clone()))?;
        match state {
            NodeState::Ready => {}
            NodeState::Pending | NodeState::Blocked | NodeState::Failed => {
                graph.transition_node(node_id, NodeState::Ready)?;
            }
            _ => {
                return Err(RunGraphError::InvalidTransition {
                    node: node_id.clone(),
                    from: state,
                    to: NodeState::Ready,
                });
            }
        }
    }
    Ok(())
}

pub fn recover_orphaned_workers(
    graph: &mut RunGraph,
    persisted_receipts: &BTreeMap<String, String>,
) -> Result<Vec<NodeId>, RunGraphError> {
    let running = graph
        .nodes
        .iter()
        .filter(|node| node.state == NodeState::Running)
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    let mut retrying = Vec::new();
    for node_id in running {
        if let Some(receipt) = persisted_receipts.get(node_id.as_str()) {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.id == node_id)
                .expect("running node remains present");
            node.output_digest = Some(receipt.clone());
            graph.transition_node(&node_id, NodeState::Succeeded)?;
            continue;
        }
        let can_retry = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .is_some_and(|node| node.checkpoint.sequence < u64::from(node.retry.max_attempts));
        graph.transition_node(&node_id, NodeState::Failed)?;
        if can_retry {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.id == node_id)
                .expect("failed node remains present");
            node.checkpoint.sequence += 1;
            node.checkpoint.recovery_token = Some(format!(
                "{}:retry:{}",
                node.idempotency_key, node.checkpoint.sequence
            ));
            graph.transition_node(&node_id, NodeState::Ready)?;
            retrying.push(node_id);
        }
    }
    Ok(retrying)
}

pub fn project_worker_progress(graph: &RunGraph) -> BTreeMap<String, WorkerProgressState> {
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            node.worker.as_ref()?;
            let state = match node.state {
                NodeState::Pending => WorkerProgressState::Queued,
                NodeState::Ready => WorkerProgressState::Selected,
                NodeState::Running => WorkerProgressState::Running,
                NodeState::Blocked => WorkerProgressState::Blocked,
                NodeState::Succeeded => WorkerProgressState::Succeeded,
                NodeState::Failed => WorkerProgressState::Failed,
                NodeState::Cancelled => WorkerProgressState::Cancelled,
                NodeState::Superseded => WorkerProgressState::Superseded,
            };
            Some((node.id.as_str().to_string(), state))
        })
        .collect()
}

fn dependencies_succeeded(graph: &RunGraph, node_id: &NodeId) -> bool {
    graph
        .edges
        .iter()
        .filter(|edge| &edge.to == node_id)
        .all(|edge| {
            graph
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .is_some_and(|parent| {
                    parent.state == NodeState::Succeeded
                        && edge.parent_receipt.is_some()
                        && parent.output_digest == edge.parent_receipt
                })
        })
}
