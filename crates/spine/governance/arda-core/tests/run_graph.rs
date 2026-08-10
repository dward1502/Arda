use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, EdgeId, EvidencePolicy, NodeId, NodeKind,
    NodeState, ObjectiveId, Provenance, RetryPolicy, RunEdge, RunGraph, RunGraphError, RunId,
    RunNode, WorkerExecutionSpec, WorkerRole, WorkerRouteClass,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn spec_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec/run-graph/v1/fixtures")
        .join(name)
}

fn node(id: &str, kind: NodeKind, authority: AuthorityClass, idempotency_key: &str) -> RunNode {
    RunNode {
        id: NodeId::new(id).unwrap(),
        kind,
        state: NodeState::Pending,
        authority,
        budget: Budget {
            max_joules: 100.0,
            max_cost_usd: 1.0,
        },
        retry: RetryPolicy { max_attempts: 1 },
        timeout_ms: 5_000,
        idempotency_key: idempotency_key.to_string(),
        input_digest: Some("sha256:input".to_string()),
        output_digest: None,
        parent_receipts: vec!["receipt:parent".to_string()],
        checkpoint: CheckpointMetadata::default(),
        worker: None,
    }
}

fn graph(nodes: Vec<RunNode>, edges: Vec<RunEdge>) -> RunGraph {
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.to_string(),
        run_id: RunId::new("run-1").unwrap(),
        objective_id: ObjectiveId::new("objective-1").unwrap(),
        nodes,
        edges,
        provenance: Provenance {
            project_contract_digest: "sha256:project".to_string(),
            created_by: "operator:test".to_string(),
            parent_receipts: vec!["receipt:root".to_string()],
        },
    }
}

#[test]
fn rejects_cycles_in_initial_executable_dag() {
    let nodes = vec![
        node(
            "inspect",
            NodeKind::Inspect,
            AuthorityClass::ReadOnly,
            "inspect-1",
        ),
        node("plan", NodeKind::Plan, AuthorityClass::ReadOnly, "plan-1"),
    ];
    let edges = vec![
        RunEdge::new("a", "inspect", "plan").unwrap(),
        RunEdge::new("b", "plan", "inspect").unwrap(),
    ];

    assert!(matches!(
        graph(nodes, edges).validate(),
        Err(RunGraphError::Cycle)
    ));
}

#[test]
fn rejects_missing_approval_parent_and_duplicate_idempotency_keys() {
    let execute = node(
        "execute",
        NodeKind::Execute,
        AuthorityClass::ExecuteWithApproval,
        "mutation-1",
    );
    assert!(matches!(
        graph(vec![execute], vec![]).validate(),
        Err(RunGraphError::MissingApprovalParent { .. })
    ));

    let nodes = vec![
        node(
            "inspect",
            NodeKind::Inspect,
            AuthorityClass::ReadOnly,
            "same",
        ),
        node("verify", NodeKind::Verify, AuthorityClass::Verify, "same"),
    ];
    assert!(matches!(
        graph(nodes, vec![]).validate(),
        Err(RunGraphError::DuplicateIdempotencyKey(_))
    ));
}

#[test]
fn validates_approval_lineage_and_round_trips_authority_and_provenance() {
    let nodes = vec![
        node(
            "approval",
            NodeKind::Approval,
            AuthorityClass::HumanApproval,
            "approval-1",
        ),
        node(
            "execute",
            NodeKind::Execute,
            AuthorityClass::ExecuteWithApproval,
            "mutation-1",
        ),
    ];
    let edges = vec![RunEdge {
        id: EdgeId::new("approval-to-execute").unwrap(),
        from: NodeId::new("approval").unwrap(),
        to: NodeId::new("execute").unwrap(),
        parent_receipt: Some("receipt:approval".to_string()),
    }];
    let graph = graph(nodes, edges);
    graph.validate().unwrap();

    let encoded = serde_json::to_string(&graph).unwrap();
    let decoded: RunGraph = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, graph);
    assert_eq!(decoded.provenance.project_contract_digest, "sha256:project");
    assert_eq!(
        decoded.nodes[1].authority,
        AuthorityClass::ExecuteWithApproval
    );
}

#[test]
fn enforces_node_state_transition_table() {
    assert!(NodeState::Pending.can_transition_to(NodeState::Ready));
    assert!(NodeState::Ready.can_transition_to(NodeState::Running));
    assert!(NodeState::Running.can_transition_to(NodeState::Succeeded));
    assert!(!NodeState::Succeeded.can_transition_to(NodeState::Running));
    assert!(!NodeState::Pending.can_transition_to(NodeState::Succeeded));
}

#[test]
fn canonical_rust_type_accepts_the_fixed_run_graph_fixture() {
    let raw = std::fs::read_to_string(spec_fixture("valid-run-graph.json")).unwrap();
    let graph = RunGraph::from_json_str(&raw).expect("fixed fixture must validate");

    assert_eq!(graph.schema_version, RunGraph::SCHEMA_VERSION);
    assert_eq!(graph.run_id.as_str(), "s4-c1-replay");
    assert_eq!(graph.nodes.len(), 3);
}

#[test]
fn canonical_rust_type_rejects_fixed_invalid_run_graph_fixtures() {
    for fixture in ["invalid-schema-version.json", "invalid-run-graph.json"] {
        let raw = std::fs::read_to_string(spec_fixture(fixture)).unwrap();
        assert!(
            RunGraph::from_json_str(&raw).is_err(),
            "{fixture} must fail closed"
        );
    }
}

#[test]
fn validates_persisted_worker_roles_and_exact_dependency_contracts() {
    let mut approval = node(
        "approval",
        NodeKind::Approval,
        AuthorityClass::HumanApproval,
        "approval-worker",
    );
    approval.worker = Some(WorkerExecutionSpec {
        role: WorkerRole::HumanApproval,
        worker_id: "operator:owner".into(),
        route_id: "human:owner".into(),
        route_class: WorkerRouteClass::Human,
        prompt_digest: format!("sha256:{}", "a".repeat(64)),
        allowed_toolsets: BTreeSet::new(),
        dependencies: Vec::new(),
        deadline_unix_ms: 1_800_000_000_000,
        output_contract: "arda.human-decision-receipt.v1".into(),
        evidence_policy: EvidencePolicy::HumanDecisionReceipt,
    });
    let mut execute = node(
        "execute",
        NodeKind::Execute,
        AuthorityClass::ExecuteWithApproval,
        "implementation-worker",
    );
    execute.worker = Some(WorkerExecutionSpec {
        role: WorkerRole::Implementer,
        worker_id: "hermes:implementation-1".into(),
        route_id: "hosted:implementation".into(),
        route_class: WorkerRouteClass::Hosted,
        prompt_digest: format!("sha256:{}", "b".repeat(64)),
        allowed_toolsets: BTreeSet::from(["file".into(), "terminal".into()]),
        dependencies: vec![NodeId::new("approval").unwrap()],
        deadline_unix_ms: 1_800_000_000_000,
        output_contract: "arda.hermes-job-result.v1".into(),
        evidence_policy: EvidencePolicy::WorkerReport,
    });
    let edge = RunEdge {
        id: EdgeId::new("approval-to-execute").unwrap(),
        from: NodeId::new("approval").unwrap(),
        to: NodeId::new("execute").unwrap(),
        parent_receipt: Some("receipt:approval".into()),
    };

    graph(vec![approval.clone(), execute.clone()], vec![edge.clone()])
        .validate()
        .unwrap();

    execute.worker.as_mut().unwrap().dependencies.clear();
    assert!(matches!(
        graph(vec![approval, execute], vec![edge]).validate(),
        Err(RunGraphError::WorkerDependencyMismatch(_))
    ));
}

#[test]
fn independent_verifier_requires_native_project_evidence() {
    let mut verifier = node(
        "verify",
        NodeKind::Verify,
        AuthorityClass::Verify,
        "independent-verifier",
    );
    verifier.worker = Some(WorkerExecutionSpec {
        role: WorkerRole::IndependentVerifier,
        worker_id: "hermes:verification-1".into(),
        route_id: "hosted:verification".into(),
        route_class: WorkerRouteClass::Hosted,
        prompt_digest: format!("sha256:{}", "c".repeat(64)),
        allowed_toolsets: BTreeSet::from(["terminal".into()]),
        dependencies: Vec::new(),
        deadline_unix_ms: 1_800_000_000_000,
        output_contract: "arda.verification-receipt.v1".into(),
        evidence_policy: EvidencePolicy::WorkerReport,
    });

    assert!(matches!(
        graph(vec![verifier], vec![]).validate(),
        Err(RunGraphError::WorkerRoleMismatch(_))
    ));
}
