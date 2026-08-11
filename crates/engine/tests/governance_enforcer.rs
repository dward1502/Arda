use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunId};
use arda_engine::runs::{
    read_governance_receipts, AdvisoryDecision, ArdaEngineGovernanceEnforcer,
    CanonicalGovernanceVerdict, GovernanceAdvisories, GovernanceEnforcementError,
    GovernanceEvaluationRequest, ResourceDemand, ResourceLedgerError, ResourceMeasurementSource,
    ResourceUsageDraft, RunEventDraft, RunEventKind, RunStore, RuntimeGovernorBudgetPolicy,
};
use serde_json::json;
use std::path::{Path, PathBuf};

fn spec_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec")
        .join(path)
}

fn budget_policy_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/runtime/runtime_governor_budget.toml")
}

fn graph(run_id: &str) -> RunGraph {
    let raw =
        std::fs::read_to_string(spec_path("run-graph/v1/fixtures/valid-run-graph.json")).unwrap();
    let mut graph = RunGraph::from_json_str(&raw).unwrap();
    graph.run_id = RunId::new(run_id).unwrap();
    graph
}

fn decision(passed: bool, name: &str) -> AdvisoryDecision {
    AdvisoryDecision {
        receipt_digest: format!("sha256:{name}"),
        passed,
    }
}

fn advisories() -> GovernanceAdvisories {
    GovernanceAdvisories {
        love: decision(true, "love"),
        triad: decision(true, "triad"),
        bacon_lite: decision(true, "bacon"),
        readiness: decision(true, "readiness"),
        realm_policy: decision(true, "realm"),
        joulework: decision(true, "joulework-observed"),
    }
}

fn demand() -> ResourceDemand {
    ResourceDemand {
        provider_id: Some("litellm_gateway".to_string()),
        local_joulework: 10.0,
        hosted_cost_usd: 0.25,
        hosted_requests: 1,
        reasoning_pressure: 0.1,
    }
}

fn request(key: &str) -> GovernanceEvaluationRequest {
    GovernanceEvaluationRequest {
        decision_key: key.to_string(),
        node_id: NodeId::new("execute").unwrap(),
        requested_transition: NodeState::Ready,
        action: json!({"operation": key, "target": "fixture"}),
        advisories: advisories(),
        approval_required: false,
        approval_granted: false,
        demand: demand(),
    }
}

fn enforcer() -> ArdaEngineGovernanceEnforcer {
    ArdaEngineGovernanceEnforcer::new(
        RuntimeGovernorBudgetPolicy::from_toml_file(budget_policy_path()).unwrap(),
    )
}

#[test]
fn canonical_owner_applies_allowed_blocked_approval_and_budget_stop_transitions() {
    let temp = tempfile::tempdir().unwrap();
    let enforcer = enforcer();

    let mut allowed_graph = graph("p15-allowed");
    let allowed_store = RunStore::open(temp.path(), allowed_graph.run_id.clone()).unwrap();
    let allowed = enforcer
        .enforce_transition(&allowed_store, &mut allowed_graph, &request("allowed"))
        .unwrap();
    assert_eq!(
        allowed.decision.receipt.verdict,
        CanonicalGovernanceVerdict::Allowed
    );
    assert_eq!(allowed_graph.nodes[1].state, NodeState::Ready);
    assert!(allowed.resource_reservation.is_some());

    let mut blocked_graph = graph("p15-blocked");
    let blocked_store = RunStore::open(temp.path(), blocked_graph.run_id.clone()).unwrap();
    let mut blocked_request = request("blocked");
    blocked_request.advisories.love.passed = false;
    let blocked = enforcer
        .enforce_transition(&blocked_store, &mut blocked_graph, &blocked_request)
        .unwrap();
    assert_eq!(
        blocked.decision.receipt.verdict,
        CanonicalGovernanceVerdict::Blocked
    );
    assert_eq!(blocked_graph.nodes[1].state, NodeState::Blocked);

    let mut approval_graph = graph("p15-approval");
    let approval_store = RunStore::open(temp.path(), approval_graph.run_id.clone()).unwrap();
    let mut approval_request = request("approval");
    approval_request.approval_required = true;
    let approval = enforcer
        .enforce_transition(&approval_store, &mut approval_graph, &approval_request)
        .unwrap();
    assert_eq!(
        approval.decision.receipt.verdict,
        CanonicalGovernanceVerdict::ApprovalRequired
    );
    assert!(approval.decision.receipt.approval_required);
    assert_eq!(approval_graph.nodes[1].state, NodeState::Blocked);

    let mut exhausted_graph = graph("p15-exhausted");
    let exhausted_store = RunStore::open(temp.path(), exhausted_graph.run_id.clone()).unwrap();
    let mut exhausted_request = request("exhausted");
    exhausted_request.approval_granted = true;
    exhausted_request.demand.hosted_cost_usd = 401.0;
    let exhausted = enforcer
        .enforce_transition(&exhausted_store, &mut exhausted_graph, &exhausted_request)
        .unwrap();
    assert_eq!(
        exhausted.decision.receipt.verdict,
        CanonicalGovernanceVerdict::BudgetExhausted
    );
    assert_eq!(exhausted_graph.nodes[1].state, NodeState::Blocked);
    assert!(exhausted.resource_reservation.is_none());
}

#[test]
fn restart_between_verdict_and_transition_replays_once_without_budget_drift() {
    let temp = tempfile::tempdir().unwrap();
    let enforcer = enforcer();
    let mut graph = graph("p15-restart");
    let request = request("restart-boundary");
    let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    let recorded = enforcer.evaluate_and_record(&store, &request).unwrap();
    assert_eq!(read_governance_receipts(&store).unwrap().len(), 1);
    assert!(store.recover().unwrap().events.is_empty());
    drop(store);

    let reopened = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    let replayed = enforcer.evaluate_and_record(&reopened, &request).unwrap();
    assert_eq!(recorded.receipt_digest, replayed.receipt_digest);
    let stale_graph = graph.clone();
    enforcer
        .apply_recorded_transition(&reopened, &mut graph, replayed, &request.demand)
        .unwrap();
    assert_eq!(graph.nodes[1].state, NodeState::Ready);
    assert_eq!(reopened.recover().unwrap().events.len(), 2);
    assert_eq!(reopened.read_resource_ledger().unwrap().len(), 1);

    drop(reopened);
    let reopened = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    // Simulate a crash after journal append but before checkpoint replacement by
    // retrying from the stale pre-transition graph projection.
    let mut replay_graph = stale_graph;
    let outcome = enforcer
        .enforce_transition(&reopened, &mut replay_graph, &request)
        .unwrap();
    assert!(matches!(
        outcome.transition,
        arda_engine::runs::TransitionOutcome::AlreadyApplied { .. }
    ));
    assert_eq!(reopened.recover().unwrap().events.len(), 2);
    assert_eq!(reopened.read_resource_ledger().unwrap().len(), 1);
}

#[test]
fn duplicate_receipt_conflicts_and_fallback_measurement_cannot_authorize() {
    let temp = tempfile::tempdir().unwrap();
    let enforcer = enforcer();
    let conflict_graph = graph("p15-conflict");
    let store = RunStore::open(temp.path(), conflict_graph.run_id.clone()).unwrap();
    let original = request("same-key");
    enforcer.evaluate_and_record(&store, &original).unwrap();
    let mut conflicting = original.clone();
    conflicting.action = json!({"operation": "different"});
    assert!(matches!(
        enforcer.evaluate_and_record(&store, &conflicting),
        Err(GovernanceEnforcementError::DuplicateReceiptConflict { .. })
    ));

    let fallback_graph = graph("p15-fallback");
    let fallback_store = RunStore::open(temp.path(), fallback_graph.run_id.clone()).unwrap();
    let mut fallback = request("fallback-only");
    fallback.advisories.joulework = decision(false, "joulework-default-fallback");
    let receipt = enforcer
        .evaluate_and_record(&fallback_store, &fallback)
        .unwrap();
    assert_eq!(receipt.receipt.verdict, CanonicalGovernanceVerdict::Blocked);
}

#[test]
fn late_observed_provider_usage_supersedes_default_without_losing_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let enforcer = enforcer();
    let mut graph = graph("p15-late-usage");
    let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    let request = request("late-usage");
    enforcer
        .enforce_transition(&store, &mut graph, &request)
        .unwrap();
    store
        .append(RunEventDraft {
            node_id: NodeId::new("execute").unwrap(),
            idempotency_key: "late-usage:result".to_string(),
            kind: RunEventKind::ResultProjected,
            receipt_digest: Some("sha256:result".to_string()),
        })
        .unwrap();
    store
        .append_resource_usage(ResourceUsageDraft {
            idempotency_key: "late-usage:observed".to_string(),
            source: ResourceMeasurementSource::Observed,
            provider_id: Some("litellm_gateway".to_string()),
            local_joulework: 8.0,
            hosted_cost_usd: 0.2,
            hosted_requests: 1,
            supersedes: Some("late-usage:budget-reservation".to_string()),
        })
        .unwrap();

    let entries = store.read_resource_ledger().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0].source,
        ResourceMeasurementSource::DefaultFallback
    );
    assert_eq!(entries[1].source, ResourceMeasurementSource::Observed);
    assert!(entries[1].recorded_after_run_completion);
    let rollup = store.resource_rollup_since(0, None).unwrap();
    assert_eq!(rollup.default_entries, 0);
    assert_eq!(rollup.observed_entries, 1);
    assert_eq!(rollup.hosted_cost_usd, 0.2);
}

#[test]
fn corrupted_resource_tail_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let graph = graph("p15-corrupt-ledger");
    let store = RunStore::open(temp.path(), graph.run_id).unwrap();
    if let Some(parent) = store.resource_ledger_path().parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(store.resource_ledger_path(), b"{\"schema_version\":").unwrap();
    assert!(matches!(
        store.read_resource_ledger(),
        Err(ResourceLedgerError::CorruptTail)
    ));
    assert!(matches!(
        enforcer().evaluate_route_budget(&store, &demand()),
        Err(GovernanceEnforcementError::Resource(
            ResourceLedgerError::CorruptTail
        ))
    ));
}

#[test]
fn route_selection_enforces_local_provider_and_pressure_caps() {
    let temp = tempfile::tempdir().unwrap();
    let graph = graph("p15-route-caps");
    let store = RunStore::open(temp.path(), graph.run_id).unwrap();
    let enforcer = enforcer();

    let mut local = demand();
    local.local_joulework = 2_501.0;
    assert_eq!(
        enforcer
            .evaluate_route_budget(&store, &local)
            .unwrap()
            .disposition,
        arda_engine::runs::BudgetDisposition::Exhausted
    );

    let mut pressure = demand();
    pressure.reasoning_pressure = 0.9;
    assert_eq!(
        enforcer
            .evaluate_route_budget(&store, &pressure)
            .unwrap()
            .disposition,
        arda_engine::runs::BudgetDisposition::Exhausted
    );

    let mut provider = demand();
    provider.hosted_requests = 250_001;
    assert_eq!(
        enforcer
            .evaluate_route_budget(&store, &provider)
            .unwrap()
            .disposition,
        arda_engine::runs::BudgetDisposition::Exhausted
    );
}

#[test]
fn named_consumers_cannot_lower_or_misrepresent_canonical_decision() {
    let temp = tempfile::tempdir().unwrap();
    let graph = graph("p15-presenters");
    let store = RunStore::open(temp.path(), graph.run_id).unwrap();
    let mut blocked = request("presenters");
    blocked.advisories.realm_policy.passed = false;
    let receipt = enforcer()
        .evaluate_and_record(&store, &blocked)
        .unwrap()
        .receipt;
    for presenter in ["orome", "hermes", "manwe", "hud", "external_adapter"] {
        assert!(matches!(
            receipt.validate_presentation(presenter, CanonicalGovernanceVerdict::Allowed, false),
            Err(GovernanceEnforcementError::ConflictingPresentation { .. })
        ));
        receipt
            .validate_presentation(presenter, CanonicalGovernanceVerdict::Blocked, false)
            .unwrap();
    }
}
