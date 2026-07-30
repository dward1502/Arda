use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, EdgeId, NodeId, NodeKind, NodeState, ObjectiveId,
    Provenance, RetryPolicy, RunEdge, RunGraph, RunGraphError, RunId, RunNode,
};

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
