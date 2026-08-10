use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, EvidencePolicy, NodeId, NodeKind, NodeState,
    ObjectiveId, Provenance, RetryPolicy, RunEdge, RunGraph, RunId, RunNode, WorkerExecutionSpec,
    WorkerRole, WorkerRouteClass,
};
use arda_core::{
    CouncilAuthority, CouncilParticipant, CouncilRoleKind, CouncilRun, CouncilRunError,
    CouncilState, MaterialTension,
};
use std::collections::BTreeSet;

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
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
            max_cost_usd: 1.0,
        },
        retry: RetryPolicy { max_attempts: 1 },
        timeout_ms: 5_000,
        idempotency_key: format!("council-{id}"),
        input_digest: Some(digest('a')),
        output_digest: None,
        parent_receipts: Vec::new(),
        checkpoint: CheckpointMetadata::default(),
        worker: Some(WorkerExecutionSpec {
            role,
            worker_id: format!("worker:{id}"),
            route_id: format!("route:{id}"),
            route_class,
            prompt_digest: digest('b'),
            allowed_toolsets: BTreeSet::new(),
            dependencies: dependencies
                .iter()
                .map(|dependency| NodeId::new(*dependency).unwrap())
                .collect(),
            deadline_unix_ms: 1_800_000_000_000,
            output_contract: "arda.council-opinion.v1".to_string(),
            evidence_policy: EvidencePolicy::WorkerReport,
        }),
    }
}

fn graph() -> RunGraph {
    let opinion_nodes = vec![
        worker_node(
            "proposer",
            NodeKind::Plan,
            AuthorityClass::ReadOnly,
            WorkerRole::PlannerProposer,
            WorkerRouteClass::Hosted,
            &[],
        ),
        worker_node(
            "security",
            NodeKind::Inspect,
            AuthorityClass::ReadOnly,
            WorkerRole::SecurityPrivacyCritic,
            WorkerRouteClass::Local,
            &[],
        ),
        worker_node(
            "implementation",
            NodeKind::Inspect,
            AuthorityClass::ReadOnly,
            WorkerRole::ImplementationRiskCritic,
            WorkerRouteClass::Local,
            &[],
        ),
    ];
    let adjudicator = worker_node(
        "adjudicator",
        NodeKind::Review,
        AuthorityClass::Verify,
        WorkerRole::Adjudicator,
        WorkerRouteClass::Hosted,
        &["proposer", "security", "implementation"],
    );
    let mut nodes = opinion_nodes;
    nodes.push(adjudicator);
    let edges = ["proposer", "security", "implementation"]
        .into_iter()
        .map(|from| RunEdge::new(format!("{from}-adjudicator"), from, "adjudicator").unwrap())
        .collect();
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.to_string(),
        run_id: RunId::new("council-run-1").unwrap(),
        objective_id: ObjectiveId::new("objective-1").unwrap(),
        nodes,
        edges,
        provenance: Provenance {
            project_contract_digest: digest('c'),
            created_by: "operator:test".to_string(),
            parent_receipts: Vec::new(),
        },
    }
}

fn participant(
    role: CouncilRoleKind,
    node_id: &str,
    route_class: WorkerRouteClass,
    opinion_character: char,
) -> CouncilParticipant {
    CouncilParticipant {
        role,
        node_id: node_id.to_string(),
        worker_id: format!("worker:{node_id}"),
        route_id: format!("route:{node_id}"),
        route_class,
        provider_id: if route_class == WorkerRouteClass::Local {
            "manwe:llama.cpp"
        } else {
            "hosted:hermes"
        }
        .to_string(),
        model_id: format!("model:{node_id}"),
        opinion_digest: digest(opinion_character),
        confidence: 0.75,
        uncertainty: "bounded fixture uncertainty".to_string(),
        evidence_refs: vec![format!("receipt:{node_id}")],
    }
}

fn council_fixture() -> CouncilRun {
    CouncilRun {
        schema_version: CouncilRun::SCHEMA_VERSION.to_string(),
        council_id: "council-1".to_string(),
        canonical_task_ref: "task:choose-release-plan".to_string(),
        run_id: "council-run-1".to_string(),
        question: "Should the proposed release plan proceed?".to_string(),
        evidence_boundary: vec!["artifact:release-plan".to_string()],
        participants: vec![
            participant(
                CouncilRoleKind::Proposer,
                "proposer",
                WorkerRouteClass::Hosted,
                '1',
            ),
            participant(
                CouncilRoleKind::SecurityCritic,
                "security",
                WorkerRouteClass::Local,
                '2',
            ),
            participant(
                CouncilRoleKind::ImplementationCritic,
                "implementation",
                WorkerRouteClass::Local,
                '3',
            ),
            participant(
                CouncilRoleKind::Adjudicator,
                "adjudicator",
                WorkerRouteClass::Hosted,
                '4',
            ),
        ],
        agreements: vec!["The migration needs a rollback checkpoint.".to_string()],
        material_tensions: vec![MaterialTension {
            tension_id: "tension:credential-window".to_string(),
            participant_roles: vec![CouncilRoleKind::Proposer, CouncilRoleKind::SecurityCritic],
            summary: "The proposer accepts a credential overlap the security critic rejects."
                .to_string(),
            evidence_refs: vec![
                "receipt:proposer".to_string(),
                "receipt:security".to_string(),
            ],
            resolved: false,
        }],
        synthesis: "Revise the credential migration before release.".to_string(),
        escalation_recommendation: "Return the plan for revision.".to_string(),
        authority: CouncilAuthority::HumanDecisionRequired,
        non_approval: true,
        operator_disposition: None,
        state: CouncilState::RevisionRequired,
    }
}

#[test]
fn validates_independent_council_provenance_and_stable_restart_digest() {
    let graph = graph();
    let council = council_fixture();
    council.validate(&graph).unwrap();

    let persisted = serde_json::to_string_pretty(&council).unwrap();
    let restored: CouncilRun = serde_json::from_str(&persisted).unwrap();
    restored.validate(&graph).unwrap();
    assert_eq!(
        restored.stable_digest().unwrap(),
        council.stable_digest().unwrap()
    );
    assert!(restored.non_approval);
    assert!(!restored.material_tensions[0].resolved);
}

#[test]
fn rejects_fabricated_or_non_independent_opinions() {
    let graph = graph();
    let mut council = council_fixture();
    council.participants[2].worker_id = council.participants[1].worker_id.clone();
    assert_eq!(
        council.validate(&graph),
        Err(CouncilRunError::DuplicateWorkerIdentity)
    );

    let mut council = council_fixture();
    council.participants[1].provider_id.clear();
    assert_eq!(
        council.validate(&graph),
        Err(CouncilRunError::EmptyField("provider_id"))
    );
}

#[test]
fn rejects_approval_claims_and_unreceipted_operator_dispositions() {
    let graph = graph();
    let mut council = council_fixture();
    council.non_approval = false;
    assert_eq!(
        council.validate(&graph),
        Err(CouncilRunError::ApprovalClaim)
    );

    let mut council = council_fixture();
    council.state = CouncilState::Concluded;
    assert_eq!(
        council.validate(&graph),
        Err(CouncilRunError::MissingOperatorDisposition)
    );
}
