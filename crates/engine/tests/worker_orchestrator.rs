use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, EdgeId, EvidencePolicy, NodeId, NodeKind,
    NodeState, ObjectiveId, Provenance, RetryPolicy, RunEdge, RunGraph, RunId, RunNode,
    WorkerExecutionSpec, WorkerRole, WorkerRouteClass,
};
use arda_engine::runs::{
    mark_selected_workers_ready, project_worker_progress, recover_orphaned_workers,
    schedule_ready_workers, WorkerAvailability, WorkerBlockReason, WorkerLimits,
    WorkerProgressState, WorkerUsage,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
#[path = "fixtures/recovery_window.rs"]
mod recovery_window;

#[test]
fn recovery_scheduling_is_exact_bounded_and_preserves_captured_deadline() {
    use arda_engine::runs::schedule_recovery_worker;
    let mut graph = parallel_graph();
    graph.nodes[0].state = NodeState::Succeeded;
    graph.nodes[1].state = NodeState::Succeeded;
    graph.nodes[0].output_digest = Some("sha256:planner".into());
    graph.nodes[1].output_digest = Some("sha256:critic".into());
    graph.nodes[2].worker.as_mut().unwrap().deadline_unix_ms = 1;
    let review = graph.nodes[2].id.clone();
    let grant =
        recovery_window::grant(&graph.run_id, &NodeId::new("verify").unwrap(), &review, 100);
    graph.provenance.project_contract_digest = grant.bindings.project_contract_digest.clone();
    let captured = serde_json::to_vec(&graph).unwrap();
    let limits = WorkerLimits::default();
    let usage = WorkerUsage::default();
    let availability = WorkerAvailability::default();
    let ordinary = schedule_ready_workers(&graph, &limits, &usage, &availability, 100);
    assert!(ordinary
        .blocked
        .iter()
        .any(|b| b.node_id == review && b.reason == WorkerBlockReason::Deadline));
    let scoped =
        schedule_recovery_worker(&graph, &limits, &usage, &availability, 100, &grant, &review)
            .unwrap();
    assert_eq!(scoped.selected, vec![review.clone()]);
    for now in [99, grant.expires_at_unix_ms.into()] {
        assert!(schedule_recovery_worker(
            &graph,
            &limits,
            &usage,
            &availability,
            now,
            &grant,
            &review
        )
        .is_err());
    }
    assert!(schedule_recovery_worker(
        &graph,
        &limits,
        &usage,
        &availability,
        100,
        &grant,
        &graph.nodes[0].id
    )
    .is_err());
    let degraded = WorkerAvailability {
        degraded_routes: [graph.nodes[2].worker.as_ref().unwrap().route_id.clone()].into(),
        ..WorkerAvailability::default()
    };
    let blocked =
        schedule_recovery_worker(&graph, &limits, &usage, &degraded, 100, &grant, &review).unwrap();
    assert!(blocked.selected.is_empty());
    assert_eq!(blocked.queued[0].reason, WorkerBlockReason::RouteDegraded);
    assert_eq!(serde_json::to_vec(&graph).unwrap(), captured);
    graph.nodes[0].state = NodeState::Pending;
    let dependency =
        schedule_recovery_worker(&graph, &limits, &usage, &availability, 100, &grant, &review)
            .unwrap();
    assert!(dependency.selected.is_empty());
    assert_eq!(dependency.queued[0].reason, WorkerBlockReason::Dependency);
}

fn worker_node(
    id: &str,
    kind: NodeKind,
    authority: AuthorityClass,
    role: WorkerRole,
    route_class: WorkerRouteClass,
    dependencies: &[&str],
) -> RunNode {
    RunNode {
        id: NodeId::new(id).unwrap(),
        kind,
        state: NodeState::Pending,
        authority,
        budget: Budget {
            max_joules: 100.0,
            max_cost_usd: 0.25,
        },
        retry: RetryPolicy { max_attempts: 2 },
        timeout_ms: 5_000,
        idempotency_key: format!("p3:{id}"),
        input_digest: Some(format!("sha256:{}", "d".repeat(64))),
        output_digest: None,
        parent_receipts: Vec::new(),
        checkpoint: CheckpointMetadata::default(),
        worker: Some(WorkerExecutionSpec {
            role,
            worker_id: format!("hermes:{id}"),
            route_id: format!(
                "{}:{id}",
                match route_class {
                    WorkerRouteClass::Local => "local",
                    WorkerRouteClass::Hosted => "hosted",
                    WorkerRouteClass::Deterministic => "deterministic",
                    WorkerRouteClass::Human => "human",
                }
            ),
            route_class,
            prompt_digest: format!("sha256:{}", "e".repeat(64)),
            allowed_toolsets: BTreeSet::from(["terminal".into()]),
            dependencies: dependencies
                .iter()
                .map(|id| NodeId::new(*id).unwrap())
                .collect(),
            deadline_unix_ms: 1_800_000_000_000,
            output_contract: "arda.worker-result.v1".into(),
            evidence_policy: match role {
                WorkerRole::IndependentVerifier => EvidencePolicy::ProjectNativeChecks,
                _ => EvidencePolicy::WorkerReport,
            },
        }),
    }
}

fn parallel_graph() -> RunGraph {
    let planner = worker_node(
        "planner",
        NodeKind::Plan,
        AuthorityClass::ReadOnly,
        WorkerRole::PlannerProposer,
        WorkerRouteClass::Hosted,
        &[],
    );
    let critic = worker_node(
        "critic",
        NodeKind::Inspect,
        AuthorityClass::ReadOnly,
        WorkerRole::SecurityPrivacyCritic,
        WorkerRouteClass::Local,
        &[],
    );
    let join = worker_node(
        "join",
        NodeKind::Review,
        AuthorityClass::Verify,
        WorkerRole::Adjudicator,
        WorkerRouteClass::Hosted,
        &["planner", "critic"],
    );
    let graph = RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new("p3-orchestration").unwrap(),
        objective_id: ObjectiveId::new("p3-objective").unwrap(),
        nodes: vec![planner, critic, join],
        edges: vec![
            RunEdge {
                id: EdgeId::new("planner-to-join").unwrap(),
                from: NodeId::new("planner").unwrap(),
                to: NodeId::new("join").unwrap(),
                parent_receipt: Some("sha256:planner".into()),
            },
            RunEdge {
                id: EdgeId::new("critic-to-join").unwrap(),
                from: NodeId::new("critic").unwrap(),
                to: NodeId::new("join").unwrap(),
                parent_receipt: Some("sha256:critic".into()),
            },
        ],
        provenance: Provenance {
            project_contract_digest: "sha256:project".into(),
            created_by: "test:p3".into(),
            parent_receipts: Vec::new(),
        },
        approval_receipts: Vec::new(),
    };
    graph.validate().unwrap();
    graph
}

#[test]
fn schedules_independent_workers_in_parallel_then_releases_deterministic_join() {
    let mut graph = parallel_graph();
    let decision = schedule_ready_workers(
        &graph,
        &WorkerLimits::default(),
        &WorkerUsage::default(),
        &WorkerAvailability::default(),
        1_700_000_000_000,
    );
    assert_eq!(
        decision
            .selected
            .iter()
            .map(NodeId::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["critic", "planner"])
    );
    assert_eq!(decision.queued[0].reason, WorkerBlockReason::Dependency);
    mark_selected_workers_ready(&mut graph, &decision.selected).unwrap();

    for (id, receipt) in [("planner", "sha256:planner"), ("critic", "sha256:critic")] {
        let id = NodeId::new(id).unwrap();
        graph.transition_node(&id, NodeState::Running).unwrap();
        graph
            .nodes
            .iter_mut()
            .find(|node| node.id == id)
            .unwrap()
            .output_digest = Some(receipt.into());
        graph.transition_node(&id, NodeState::Succeeded).unwrap();
    }

    let join = schedule_ready_workers(
        &graph,
        &WorkerLimits::default(),
        &WorkerUsage::default(),
        &WorkerAvailability::default(),
        1_700_000_000_000,
    );
    assert_eq!(join.selected, vec![NodeId::new("join").unwrap()]);
}

#[test]
fn refuses_oversubscription_and_unavailable_local_or_degraded_routes() {
    let graph = parallel_graph();
    let limits = WorkerLimits {
        max_total_workers: 1,
        max_local_workers: 1,
        max_hosted_workers: 1,
        ..WorkerLimits::default()
    };
    let decision = schedule_ready_workers(
        &graph,
        &limits,
        &WorkerUsage::default(),
        &WorkerAvailability {
            local_worker_available: false,
            local_thermal_ok: true,
            degraded_routes: BTreeSet::new(),
        },
        1_700_000_000_000,
    );
    assert_eq!(decision.selected, vec![NodeId::new("planner").unwrap()]);
    assert!(decision
        .queued
        .iter()
        .any(|block| block.reason == WorkerBlockReason::LocalUnavailable));

    let budget_blocked = schedule_ready_workers(
        &graph,
        &WorkerLimits {
            max_run_cost_usd: 0.1,
            ..WorkerLimits::default()
        },
        &WorkerUsage::default(),
        &WorkerAvailability::default(),
        1_700_000_000_000,
    );
    assert!(budget_blocked
        .blocked
        .iter()
        .any(|block| block.reason == WorkerBlockReason::CostBudget));
}

#[test]
fn restart_reconciles_receipted_workers_and_retries_orphans_without_duplicate_success() {
    let mut graph = parallel_graph();
    for id in ["planner", "critic"] {
        let id = NodeId::new(id).unwrap();
        graph.transition_node(&id, NodeState::Ready).unwrap();
        graph.transition_node(&id, NodeState::Running).unwrap();
    }
    let receipts = BTreeMap::from([("planner".into(), "sha256:planner".into())]);
    let retrying = recover_orphaned_workers(&mut graph, &receipts).unwrap();
    assert_eq!(retrying, vec![NodeId::new("critic").unwrap()]);
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "planner")
            .unwrap()
            .state,
        NodeState::Succeeded
    );
    assert_eq!(
        project_worker_progress(&graph).get("critic"),
        Some(&WorkerProgressState::Selected)
    );
}

#[test]
fn repository_worker_limits_are_valid_and_retries_reserve_every_attempt() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/runtime/worker_orchestration.toml");
    let limits = WorkerLimits::from_toml_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(limits.max_total_workers, 4);

    let graph = parallel_graph();
    let run_limited = WorkerLimits {
        // Each ready worker reserves $0.25 × two remaining attempts.
        max_run_cost_usd: 0.49,
        ..limits.clone()
    };
    let decision = schedule_ready_workers(
        &graph,
        &run_limited,
        &WorkerUsage::default(),
        &WorkerAvailability::default(),
        1_700_000_000_000,
    );
    assert!(decision.selected.is_empty());
    assert!(decision
        .blocked
        .iter()
        .all(|blocked| blocked.reason == WorkerBlockReason::CostBudget));

    let daily_limited = schedule_ready_workers(
        &graph,
        &limits,
        &WorkerUsage {
            daily_spent_cost_usd: 24.51,
            ..WorkerUsage::default()
        },
        &WorkerAvailability::default(),
        1_700_000_000_000,
    );
    assert!(daily_limited
        .blocked
        .iter()
        .all(|blocked| blocked.reason == WorkerBlockReason::DailyCostBudget));
}
