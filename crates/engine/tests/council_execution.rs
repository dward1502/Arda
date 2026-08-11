use arda_core::council_run::{
    CouncilAuthority, CouncilParticipant, CouncilRoleKind, CouncilRun, CouncilState,
    MaterialTension,
};
use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, EvidencePolicy, NodeId, NodeKind, NodeState,
    ObjectiveId, Provenance, RetryPolicy, RunEdge, RunGraph, RunId, RunNode, WorkerExecutionSpec,
    WorkerRole, WorkerRouteClass,
};
use arda_engine::council::{
    council_is_warranted, CouncilFallbackPolicy, CouncilOperatorProjection, CouncilWorkerReceipt,
    CouncilWorkerReceiptStatus, CouncilWorkerRequest, ManweCouncilClient, ManweCouncilConfig,
};
use arda_engine::runs::RunStore;
use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
struct MockState {
    calls: Arc<Mutex<Vec<Value>>>,
}

async fn health() -> Json<Value> {
    Json(json!({"ok": true, "providers_healthy": 3}))
}

async fn capabilities() -> Json<Value> {
    Json(json!({"adaptive_routing": true, "providers_healthy": 3}))
}

async fn provider_capabilities() -> Json<Value> {
    Json(json!({
        "capabilities": {"providers": [{
            "provider_id": "llama.cpp",
            "access_tier": "local",
            "enabled": true,
            "models": [{
                "model_id": "local-critic",
                "healthy": true,
                "capabilities": {"basic_chat": {"state": "passed"}}
            }]
        }]}
    }))
}

async fn completion(State(state): State<MockState>, Json(body): Json<Value>) -> impl IntoResponse {
    state.calls.lock().unwrap().push(body.clone());
    let worker = body["agent_id"].as_str().unwrap();
    let local = worker.contains("critic");
    let role = if worker.contains("security") {
        "Security review rejects the overlapping credential window."
    } else if worker.contains("implementation") {
        "Implementation review requires an atomic rollback checkpoint."
    } else if worker.contains("proposer") {
        "Proceed with a short overlapping credential migration."
    } else {
        "Revise the plan to remove credential overlap and retain rollback."
    };
    let content = json!({
        "summary": role,
        "confidence": 0.82,
        "uncertainty": "Fixture is limited to the cited migration evidence.",
        "evidence_refs": [format!("artifact:{worker}")]
    })
    .to_string();
    let mut response = (
        StatusCode::OK,
        Json(json!({
            "choices": [{"message": {"content": content}}],
            "usage": {"prompt_tokens": 64, "completion_tokens": 32}
        })),
    )
        .into_response();
    let headers = response.headers_mut();
    headers.insert(
        "x-manwe-route-id",
        HeaderValue::from_str(&format!("route:{worker}")).unwrap(),
    );
    headers.insert(
        "x-manwe-provider-id",
        HeaderValue::from_static(if local { "llama.cpp" } else { "hosted-fixture" }),
    );
    headers.insert(
        "x-manwe-model-id",
        HeaderValue::from_static(if local {
            "local-critic"
        } else {
            "hosted-reasoner"
        }),
    );
    headers.insert(
        "x-manwe-route-class",
        HeaderValue::from_static(if local { "local" } else { "hosted" }),
    );
    response
}

async fn mock_manwe() -> (String, MockState) {
    let state = MockState::default();
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/v1/capabilities", get(capabilities))
        .route("/providers/capabilities", get(provider_capabilities))
        .route("/v1/chat/completions", post(completion))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{address}"), state)
}

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn role_spec(role: CouncilRoleKind) -> (WorkerRole, NodeKind, AuthorityClass, WorkerRouteClass) {
    match role {
        CouncilRoleKind::Proposer => (
            WorkerRole::PlannerProposer,
            NodeKind::Plan,
            AuthorityClass::ReadOnly,
            WorkerRouteClass::Hosted,
        ),
        CouncilRoleKind::SecurityCritic => (
            WorkerRole::SecurityPrivacyCritic,
            NodeKind::Inspect,
            AuthorityClass::ReadOnly,
            WorkerRouteClass::Local,
        ),
        CouncilRoleKind::ImplementationCritic => (
            WorkerRole::ImplementationRiskCritic,
            NodeKind::Inspect,
            AuthorityClass::ReadOnly,
            WorkerRouteClass::Local,
        ),
        CouncilRoleKind::Adjudicator => (
            WorkerRole::Adjudicator,
            NodeKind::Review,
            AuthorityClass::Verify,
            WorkerRouteClass::Hosted,
        ),
    }
}

fn node(role: CouncilRoleKind, dependencies: &[&str]) -> RunNode {
    let id = match role {
        CouncilRoleKind::Proposer => "proposer",
        CouncilRoleKind::SecurityCritic => "security-critic",
        CouncilRoleKind::ImplementationCritic => "implementation-critic",
        CouncilRoleKind::Adjudicator => "adjudicator",
    };
    let (worker_role, kind, authority, route_class) = role_spec(role);
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
        idempotency_key: format!("council:{id}"),
        input_digest: Some(digest('a')),
        output_digest: None,
        parent_receipts: Vec::new(),
        checkpoint: CheckpointMetadata::default(),
        worker: Some(WorkerExecutionSpec {
            role: worker_role,
            worker_id: format!("worker:{id}"),
            route_id: format!("route:worker:{id}"),
            route_class,
            prompt_digest: digest('b'),
            allowed_toolsets: BTreeSet::new(),
            dependencies: dependencies
                .iter()
                .map(|dependency| NodeId::new(*dependency).unwrap())
                .collect(),
            deadline_unix_ms: 1_900_000_000_000,
            output_contract: "arda.council-opinion.v1".into(),
            evidence_policy: EvidencePolicy::WorkerReport,
        }),
    }
}

fn graph() -> RunGraph {
    let dependencies = ["proposer", "security-critic", "implementation-critic"];
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new("council-acceptance").unwrap(),
        objective_id: ObjectiveId::new("objective-safe-migration").unwrap(),
        nodes: vec![
            node(CouncilRoleKind::Proposer, &[]),
            node(CouncilRoleKind::SecurityCritic, &[]),
            node(CouncilRoleKind::ImplementationCritic, &[]),
            node(CouncilRoleKind::Adjudicator, &dependencies),
        ],
        edges: dependencies
            .into_iter()
            .map(|parent| RunEdge::new(format!("{parent}-join"), parent, "adjudicator").unwrap())
            .collect(),
        provenance: Provenance {
            project_contract_digest: digest('c'),
            created_by: "operator:fixture".into(),
            parent_receipts: Vec::new(),
        },
    }
}

fn request(role: CouncilRoleKind) -> CouncilWorkerRequest {
    let node_id = match role {
        CouncilRoleKind::Proposer => "proposer",
        CouncilRoleKind::SecurityCritic => "security-critic",
        CouncilRoleKind::ImplementationCritic => "implementation-critic",
        CouncilRoleKind::Adjudicator => "adjudicator",
    };
    CouncilWorkerRequest {
        run_id: "council-acceptance".into(),
        node_id: node_id.into(),
        worker_id: format!("worker:{node_id}"),
        role,
        question: "Should the credential migration proceed?".into(),
        evidence_boundary: vec![format!("artifact:{node_id}")],
        fallback_policy: if matches!(
            role,
            CouncilRoleKind::SecurityCritic | CouncilRoleKind::ImplementationCritic
        ) {
            CouncilFallbackPolicy::LocalOnly
        } else {
            CouncilFallbackPolicy::AllowHosted
        },
    }
}

fn participant(receipt: &CouncilWorkerReceipt) -> CouncilParticipant {
    let route_class = match receipt.route_class.as_deref() {
        Some("local") => WorkerRouteClass::Local,
        Some("hosted") => WorkerRouteClass::Hosted,
        other => panic!("unexpected route class {other:?}"),
    };
    let opinion = receipt.opinion.as_ref().unwrap();
    CouncilParticipant {
        role: receipt.role,
        node_id: receipt.node_id.clone(),
        worker_id: receipt.worker_id.clone(),
        route_id: receipt.route_id.clone().unwrap(),
        route_class,
        provider_id: receipt.provider_id.clone().unwrap(),
        model_id: receipt.model_id.clone().unwrap(),
        opinion_digest: receipt.opinion_digest.clone().unwrap(),
        confidence: opinion.confidence,
        uncertainty: opinion.uncertainty.clone(),
        evidence_refs: opinion.evidence_refs.clone(),
    }
}

#[tokio::test]
async fn independent_manwe_deliberation_persists_across_restart_without_approval() {
    let (base_url, state) = mock_manwe().await;
    let client = ManweCouncilClient::new(ManweCouncilConfig {
        base_url,
        timeout: Duration::from_secs(2),
        ..ManweCouncilConfig::default()
    })
    .unwrap();
    let mut receipts = BTreeMap::new();
    for role in [
        CouncilRoleKind::Proposer,
        CouncilRoleKind::SecurityCritic,
        CouncilRoleKind::ImplementationCritic,
        CouncilRoleKind::Adjudicator,
    ] {
        let receipt = client.execute(&request(role)).await;
        assert!(matches!(
            receipt.status,
            CouncilWorkerReceiptStatus::Succeeded | CouncilWorkerReceiptStatus::Degraded
        ));
        assert!(receipt.non_approval);
        receipts.insert(format!("{role:?}"), receipt);
    }
    for role in ["SecurityCritic", "ImplementationCritic"] {
        let receipt = &receipts[role];
        assert_eq!(receipt.route_class.as_deref(), Some("local"));
        assert!(!receipt.fallback_used);
        assert_eq!(receipt.provider_id.as_deref(), Some("llama.cpp"));
    }

    let graph = graph();
    let council = CouncilRun {
        schema_version: CouncilRun::SCHEMA_VERSION.into(),
        council_id: "council:safe-migration".into(),
        canonical_task_ref: "task:safe-migration".into(),
        run_id: graph.run_id.as_str().into(),
        question: "Should the credential migration proceed?".into(),
        evidence_boundary: vec!["artifact:migration-plan".into()],
        participants: [
            "Proposer",
            "SecurityCritic",
            "ImplementationCritic",
            "Adjudicator",
        ]
        .into_iter()
        .map(|role| participant(&receipts[role]))
        .collect(),
        agreements: vec!["A rollback checkpoint is required.".into()],
        material_tensions: vec![MaterialTension {
            tension_id: "tension:credential-overlap".into(),
            participant_roles: vec![CouncilRoleKind::Proposer, CouncilRoleKind::SecurityCritic],
            summary: "The proposer permits credential overlap; the security critic rejects it."
                .into(),
            evidence_refs: vec![
                receipts["Proposer"].evidence_ref().unwrap(),
                receipts["SecurityCritic"].evidence_ref().unwrap(),
            ],
            resolved: false,
        }],
        synthesis: "Revise the migration to eliminate credential overlap and retain rollback."
            .into(),
        escalation_recommendation: "Return the plan for revision before operator approval.".into(),
        authority: CouncilAuthority::HumanDecisionRequired,
        non_approval: true,
        operator_disposition: None,
        state: CouncilState::RevisionRequired,
    };
    council.validate(&graph).unwrap();

    let root = tempfile::tempdir().unwrap();
    let store = RunStore::open(root.path(), graph.run_id.clone()).unwrap();
    store.write_checkpoint(&graph).unwrap();
    let digest = store.write_council_run(&council, &graph).unwrap();
    drop(store);

    let reopened = RunStore::open(root.path(), graph.run_id.clone()).unwrap();
    let restored_graph = reopened.recover().unwrap().checkpoint.unwrap();
    let restored = reopened.read_council_run().unwrap().unwrap();
    restored.validate(&restored_graph).unwrap();
    assert_eq!(restored.stable_digest().unwrap(), digest);
    assert!(restored.non_approval);

    let projection = CouncilOperatorProjection::from_run(&restored);
    let phone_message = projection.concise_message();
    assert!(phone_message.contains("Material tension:"));
    assert!(phone_message.contains("Decision requested:"));
    assert!(phone_message.contains("operator approval has not been granted"));
    assert!(projection.evidence_available);

    let calls = state.calls.lock().unwrap();
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|call| call["tools"] == json!([])));
    assert!(calls.iter().all(|call| {
        call["routing"]["execution_lane"] == "council_read_only"
            && call["routing"]["tool_use_required"] == false
    }));
}

#[tokio::test]
async fn local_only_failure_is_unavailable_and_never_fabricates_an_opinion() {
    let app = Router::new()
        .route(
            "/healthz",
            get(|| async {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({"ok": false, "providers_healthy": 0})),
                )
            }),
        )
        .route("/v1/capabilities", get(capabilities));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = ManweCouncilClient::new(ManweCouncilConfig {
        base_url: format!("http://{address}"),
        timeout: Duration::from_secs(2),
        ..ManweCouncilConfig::default()
    })
    .unwrap();

    let receipt = client
        .execute(&request(CouncilRoleKind::SecurityCritic))
        .await;
    assert_eq!(receipt.status, CouncilWorkerReceiptStatus::Unavailable);
    assert!(receipt.opinion.is_none());
    assert!(receipt.opinion_digest.is_none());
    assert!(receipt.unavailable_reason.is_some());
    assert!(receipt.non_approval);
}

#[test]
fn council_is_skipped_when_deterministic_checks_or_one_worker_suffice() {
    assert!(!council_is_warranted(false, 1, false));
    assert!(!council_is_warranted(true, 3, true));
    assert!(council_is_warranted(false, 2, false));
    assert!(council_is_warranted(true, 1, false));
}

#[tokio::test]
#[ignore = "requires the live Manwë gateway and an eligible local model"]
async fn live_manwe_executes_two_independent_local_critics_with_route_provenance() {
    let base_url =
        std::env::var("ARDA_MANWE_URL").unwrap_or_else(|_| "http://127.0.0.1:7171".into());
    let client = ManweCouncilClient::new(ManweCouncilConfig {
        base_url,
        timeout: Duration::from_secs(180),
        preferred_local_provider: Some("edge_beelink_light".into()),
        preferred_local_model: Some("Ternary-Bonsai-8B-Q2_0".into()),
        ..ManweCouncilConfig::default()
    })
    .unwrap();
    let security_receipt = client
        .execute(&request(CouncilRoleKind::SecurityCritic))
        .await;
    let implementation_receipt = client
        .execute(&request(CouncilRoleKind::ImplementationCritic))
        .await;
    println!(
        "{}",
        serde_json::to_string_pretty(&vec![&security_receipt, &implementation_receipt]).unwrap()
    );
    for receipt in [&security_receipt, &implementation_receipt] {
        assert_eq!(receipt.status, CouncilWorkerReceiptStatus::Succeeded);
        assert!(receipt.provider_id.is_some());
        assert!(receipt.model_id.is_some());
        assert!(receipt.route_id.is_some());
        assert!(receipt.opinion.is_some());
        assert!(!receipt.fallback_used);
        assert!(receipt.non_approval);
    }
    assert_ne!(security_receipt.worker_id, implementation_receipt.worker_id);
    assert_ne!(security_receipt.node_id, implementation_receipt.node_id);
    assert_ne!(security_receipt.route_id, implementation_receipt.route_id);
}
