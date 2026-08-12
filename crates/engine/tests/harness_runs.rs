use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

const PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

async fn start_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator-0".to_string(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().expect("loopback address")),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (bound, shutdown, handle)
}

fn envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-runs-1",
            "approval_id": "approval-runs-1",
            "ledger_writes": ["test-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-07-31T00:00:00Z"
        },
        "idempotency_key": key
    })
}

fn receipt_digest(node_id: &str) -> String {
    let marker = match node_id {
        "execute" => 'e',
        "verify" => 'f',
        "review" => 'a',
        "close" => 'c',
        "different-close" => 'd',
        _ => panic!("unexpected receipt fixture {node_id}"),
    };
    format!("sha256:{}", marker.to_string().repeat(64))
}

fn contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture")
}

fn graph(run_id: &str, node_id: &str, kind: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": kind,
            "state": "pending",
            "authority": if kind == "approval" { "human_approval" } else { "read_only" },
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 1000,
            "idempotency_key": format!("node-{run_id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": [],
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
        }],
        "edges": [],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "integration-test",
            "parent_receipts": []
        }
    })
}

fn completion_graph(run_id: &str) -> Value {
    let node = |id: &str, kind: &str, authority: &str, receipts: Vec<&str>| {
        json!({
            "id": id,
            "kind": kind,
            "state": "pending",
            "authority": authority,
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 1000,
            "idempotency_key": format!("node-{run_id}-{id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": receipts,
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
        })
    };
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [
            node("plan", "plan", "read_only", vec![]),
            node("approval", "approval", "human_approval", vec!["receipt:plan"]),
            node("execute", "execute", "execute_with_approval", vec!["receipt:approval"]),
            node("verify", "verify", "verify", vec!["receipt:execute"]),
            node("review", "review", "verify", vec!["receipt:verify"]),
            node("close", "close", "read_only", vec!["receipt:review"])
        ],
        "edges": [
            {"id": "plan-approval", "from": "plan", "to": "approval", "parent_receipt": "receipt:plan"},
            {"id": "approval-execute", "from": "approval", "to": "execute", "parent_receipt": "receipt:approval"},
            {"id": "execute-verify", "from": "execute", "to": "verify", "parent_receipt": "receipt:execute"},
            {"id": "verify-review", "from": "verify", "to": "review", "parent_receipt": "receipt:verify"},
            {"id": "review-close", "from": "review", "to": "close", "parent_receipt": "receipt:review"}
        ],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "integration-test",
            "parent_receipts": []
        }
    })
}

async fn attach(client: &reqwest::Client, bound: std::net::SocketAddr) {
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": envelope("attach-for-runs")}))
        .send()
        .await
        .expect("attach request")
        .error_for_status()
        .expect("attach status");
}

async fn plan(
    client: &reqwest::Client,
    bound: std::net::SocketAddr,
    run_id: &str,
    node_id: &str,
    kind: &str,
) -> reqwest::Response {
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph(run_id, node_id, kind),
            "envelope": envelope(&format!("plan-{run_id}"))
        }))
        .send()
        .await
        .expect("plan request")
}

#[tokio::test]
async fn run_routes_plan_approve_read_and_expose_canonical_events() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let planned = plan(&client, bound, "run-approve", "approval-node", "approval").await;
    assert_eq!(planned.status(), 201);
    let retry = plan(&client, bound, "run-approve", "approval-node", "approval").await;
    assert_eq!(retry.status(), 200);

    let approved: Value = client
        .post(format!("http://{bound}/v1/runs/run-approve/approve"))
        .json(&json!({
            "node_id": "approval-node",
            "envelope": envelope("approve-run-approve")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status")
        .json()
        .await
        .expect("approve body");
    assert_eq!(approved["graph"]["nodes"][0]["state"], "succeeded");

    let run: Value = client
        .get(format!("http://{bound}/v1/runs/run-approve"))
        .send()
        .await
        .expect("get run")
        .error_for_status()
        .expect("get run status")
        .json()
        .await
        .expect("get run body");
    assert_eq!(run["graph"]["run_id"], "run-approve");

    let events: Value = client
        .get(format!("http://{bound}/v1/runs/run-approve/events"))
        .send()
        .await
        .expect("get events")
        .error_for_status()
        .expect("events status")
        .json()
        .await
        .expect("events body");
    assert!(events["events"]
        .as_array()
        .is_some_and(|events| events.len() >= 4));
    assert!(root
        .path()
        .join("data/runs/run-approve/events.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn run_event_stream_is_sse_and_emits_canonical_events() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    plan(&client, bound, "run-stream", "plan-node", "plan")
        .await
        .error_for_status()
        .expect("plan status");

    let projection_started = std::time::Instant::now();
    let mut response = client
        .get(format!("http://{bound}/v1/runs/run-stream/events/stream"))
        .send()
        .await
        .expect("stream request")
        .error_for_status()
        .expect("stream status");
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut body = Vec::new();
        while !body.windows(2).any(|window| window == b"\n\n") {
            body.extend_from_slice(
                &response
                    .chunk()
                    .await
                    .expect("stream chunk")
                    .expect("open stream"),
            );
        }
        body
    })
    .await
    .expect("stream event timeout");
    assert!(
        projection_started.elapsed() < std::time::Duration::from_secs(1),
        "event projection exceeded the 1s U3 budget"
    );
    let text = String::from_utf8(frame).expect("utf-8 SSE frame");
    assert!(text.contains("event: run_event"));
    assert!(text.contains("\"schema_version\":\"arda.run-event.v1\""));
    assert!(text.contains("\"run_id\":\"run-stream\""));

    drop(response);
    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn cancel_is_idempotent_and_mutations_require_typed_envelopes() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    assert_eq!(
        plan(&client, bound, "run-cancel", "approval-node", "approval")
            .await
            .status(),
        201
    );

    let missing_envelope = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&json!({"reason": "operator requested"}))
        .send()
        .await
        .expect("missing-envelope request");
    assert_eq!(missing_envelope.status(), 422);

    let cancel_body = json!({
        "reason": "operator requested",
        "envelope": envelope("cancel-run-cancel")
    });
    let cancelled: Value = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel request")
        .error_for_status()
        .expect("cancel status")
        .json()
        .await
        .expect("cancel body");
    assert_eq!(cancelled["graph"]["nodes"][0]["state"], "cancelled");

    let retry = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel retry");
    assert_eq!(retry.status(), 200);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn operator_rejection_is_durable_and_cannot_authorize_execution() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    assert_eq!(
        plan(&client, bound, "run-rejected", "approval-node", "approval")
            .await
            .status(),
        201
    );

    let rejected: Value = client
        .post(format!("http://{bound}/v1/runs/run-rejected/cancel"))
        .json(&json!({
            "reason": "approval rejected; revise objective before replanning",
            "envelope": envelope("reject-run-rejected")
        }))
        .send()
        .await
        .expect("rejection request")
        .error_for_status()
        .expect("rejection status")
        .json()
        .await
        .expect("rejection body");
    assert!(rejected["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|node| node["state"] == "cancelled"));
    assert!(rejected["events"].as_array().unwrap().iter().any(|event| {
        event["kind"]["type"] == "cancelled"
            && event["kind"]["reason"] == "approval rejected; revise objective before replanning"
    }));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn operator_receipts_complete_execute_verify_review_and_close_in_order() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-complete"),
            "envelope": envelope("plan-run-complete")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/run-complete"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let plan_node = planned["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "plan"))
        .expect("plan node");
    assert_eq!(plan_node["state"], "succeeded");
    assert_eq!(plan_node["output_digest"], "receipt:plan");
    assert!(plan_node["checkpoint"]["sequence"].as_u64().unwrap_or(0) > 0);
    assert!(plan_node["checkpoint"]["recovery_token"].is_string());
    assert!(plan_node["checkpoint"]["checkpoint_digest"].is_string());
    client
        .post(format!("http://{bound}/v1/runs/run-complete/approve"))
        .json(&json!({
            "node_id": "approval",
            "envelope": envelope("approve-run-complete")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status");

    let malformed_digest = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": envelope("reject-malformed-digest"),
            "receipt_digest": "sha256:not-a-digest"
        }))
        .send()
        .await
        .expect("malformed digest request");
    assert_eq!(malformed_digest.status(), 400);

    let traversal_evidence = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": envelope("reject-traversal-evidence"),
            "receipt_digest": receipt_digest("execute"),
            "evidence": {
                "changes": [{
                    "path": "../outside",
                    "status": "modified",
                    "additions": 1,
                    "deletions": 0
                }]
            }
        }))
        .send()
        .await
        .expect("traversal evidence request");
    assert_eq!(traversal_evidence.status(), 400);

    for node_id in ["execute", "verify", "review", "close"] {
        let digest = receipt_digest(node_id);
        let response: Value = client
            .post(format!(
                "http://{bound}/v1/runs/run-complete/nodes/{node_id}/complete"
            ))
            .json(&json!({
                "envelope": envelope(&format!("complete-{node_id}")),
                "receipt_digest": digest,
                "evidence": match node_id {
                    "execute" => json!({
                        "changes": [{
                            "path": "src/lib.rs",
                            "status": "modified",
                            "additions": 2,
                            "deletions": 2,
                            "diff": "-hello\n+hello, Arda"
                        }],
                        "provider_receipt": {
                            "provider": "nous",
                            "model": "fixture-model",
                            "adapter": "hermes-workbench",
                            "receipt_digest": receipt_digest("execute"),
                            "summary": "Bounded fixture mutation completed."
                        }
                    }),
                    "verify" => json!({
                        "tests": [{
                            "name": "cargo test --quiet",
                            "status": "passed",
                            "duration_ms": 12,
                            "details": "exit 0"
                        }]
                    }),
                    _ => Value::Null,
                }
            }))
            .send()
            .await
            .expect("complete request")
            .error_for_status()
            .expect("complete status")
            .json()
            .await
            .expect("complete body");
        let node = response["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["id"] == node_id))
            .expect("completed node");
        assert_eq!(node["state"], "succeeded");
        assert_eq!(node["output_digest"], receipt_digest(node_id));
        assert!(node["checkpoint"]["sequence"].as_u64().unwrap_or(0) > 0);
        assert!(node["checkpoint"]["recovery_token"].is_string());
        assert!(node["checkpoint"]["checkpoint_digest"].is_string());
    }

    let recovered: Value = client
        .get(format!("http://{bound}/v1/runs/run-complete"))
        .send()
        .await
        .expect("get completed run")
        .error_for_status()
        .expect("get completed run status")
        .json()
        .await
        .expect("get completed run body");
    assert!(recovered["graph"]["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().all(|node| node["state"] == "succeeded")));
    let recovered_nodes = recovered["graph"]["nodes"].as_array().expect("nodes");
    for edge in recovered["graph"]["edges"].as_array().expect("edges") {
        let parent = recovered_nodes
            .iter()
            .find(|node| node["id"] == edge["from"])
            .expect("edge parent");
        let child = recovered_nodes
            .iter()
            .find(|node| node["id"] == edge["to"])
            .expect("edge child");
        assert_eq!(edge["parent_receipt"], parent["output_digest"]);
        assert!(child["parent_receipts"]
            .as_array()
            .is_some_and(|receipts| receipts.contains(&edge["parent_receipt"])));
    }
    assert_eq!(recovered["review"]["changes"][0]["path"], "src/lib.rs");
    assert_eq!(recovered["review"]["tests"][0]["status"], "passed");
    assert_eq!(recovered["review"]["provider_receipt"]["provider"], "nous");

    let retry = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/close/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-close"),
            "receipt_digest": receipt_digest("close")
        }))
        .send()
        .await
        .expect("complete retry");
    assert_eq!(retry.status(), 200);

    let conflicting_retry = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/close/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-close"),
            "receipt_digest": receipt_digest("different-close")
        }))
        .send()
        .await
        .expect("conflicting complete retry");
    assert_eq!(conflicting_retry.status(), 409);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn completed_workbench_mutation_survives_process_restart_and_replays_once() {
    let root = TempDir::new().expect("temp root");
    let client = reqwest::Client::new();
    let (bound, shutdown, handle) = start_harness(&root).await;
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-restart-recovery"),
            "envelope": envelope("plan-run-restart-recovery")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    client
        .post(format!(
            "http://{bound}/v1/runs/run-restart-recovery/approve"
        ))
        .json(&json!({
            "node_id": "approval",
            "envelope": envelope("approve-run-restart-recovery")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status");

    let completion_body = json!({
        "envelope": envelope("complete-run-restart-recovery-execute"),
        "receipt_digest": receipt_digest("execute"),
        "evidence": {
            "changes": [{
                "path": "src/lib.rs",
                "status": "modified",
                "additions": 1,
                "deletions": 1
            }],
            "provider_receipt": {
                "provider": "nous",
                "model": "fixture-model",
                "adapter": "hermes-workbench",
                "receipt_digest": receipt_digest("execute"),
                "summary": "Restart acceptance mutation completed."
            }
        }
    });
    let completed: Value = client
        .post(format!(
            "http://{bound}/v1/runs/run-restart-recovery/nodes/execute/complete"
        ))
        .json(&completion_body)
        .send()
        .await
        .expect("complete request")
        .error_for_status()
        .expect("complete status")
        .json()
        .await
        .expect("complete body");
    let completed_node = completed["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
        .expect("completed execute node");
    assert_eq!(completed_node["state"], "succeeded");
    assert_eq!(completed_node["output_digest"], receipt_digest("execute"));
    let recovery_token = completed_node["checkpoint"]["recovery_token"]
        .as_str()
        .expect("recovery token")
        .to_string();
    let checkpoint_digest = completed_node["checkpoint"]["checkpoint_digest"]
        .as_str()
        .expect("checkpoint digest")
        .to_string();
    let events_path = root
        .path()
        .join("data/runs/run-restart-recovery/events.jsonl");
    let checkpoint_path = root
        .path()
        .join("data/runs/run-restart-recovery/checkpoint.json");
    let result_path = root
        .path()
        .join("data/runs/run-restart-recovery/result.json");
    assert!(events_path.exists(), "durable event receipt must exist");
    assert!(checkpoint_path.exists(), "durable checkpoint must exist");
    assert!(result_path.exists(), "durable review projection must exist");
    let events_before_restart = std::fs::read(&events_path).expect("read durable events");
    let checkpoint_before_restart =
        std::fs::read(&checkpoint_path).expect("read durable checkpoint");
    let result_before_restart = std::fs::read(&result_path).expect("read durable result");

    shutdown.notify_waiters();
    handle.await.expect("first harness shutdown");

    let (restarted_bound, restarted_shutdown, restarted_handle) = start_harness(&root).await;
    let recovered: Value = client
        .get(format!(
            "http://{restarted_bound}/v1/runs/run-restart-recovery"
        ))
        .send()
        .await
        .expect("recover run request")
        .error_for_status()
        .expect("recover run status")
        .json()
        .await
        .expect("recover run body");
    let recovered_node = recovered["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
        .expect("recovered execute node");
    assert_eq!(recovered_node["state"], "succeeded");
    assert_eq!(recovered_node["output_digest"], receipt_digest("execute"));
    assert_eq!(
        recovered_node["checkpoint"]["recovery_token"],
        recovery_token
    );
    assert_eq!(
        recovered_node["checkpoint"]["checkpoint_digest"],
        checkpoint_digest
    );
    assert_eq!(
        recovered["review"]["provider_receipt"]["receipt_digest"],
        receipt_digest("execute")
    );

    let replay: Value = client
        .post(format!(
            "http://{restarted_bound}/v1/runs/run-restart-recovery/nodes/execute/complete"
        ))
        .json(&completion_body)
        .send()
        .await
        .expect("completion replay")
        .error_for_status()
        .expect("completion replay status")
        .json()
        .await
        .expect("completion replay body");
    assert_eq!(
        replay["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
            .expect("replayed execute node")["checkpoint"]["recovery_token"],
        recovery_token
    );
    assert_eq!(
        std::fs::read(&events_path).expect("read replayed events"),
        events_before_restart,
        "idempotent replay must not append another mutation receipt"
    );
    assert_eq!(
        std::fs::read(&checkpoint_path).expect("read replayed checkpoint"),
        checkpoint_before_restart,
        "authoritative checkpoint must remain stable after replay"
    );
    assert_eq!(
        std::fs::read(&result_path).expect("read replayed result"),
        result_before_restart,
        "durable review projection must remain stable after replay"
    );

    restarted_shutdown.notify_waiters();
    restarted_handle.await.expect("restarted harness shutdown");
}

#[tokio::test]
async fn failed_verification_is_durable_and_blocks_review() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-failed-verification"),
            "envelope": envelope("plan-failed-verification")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/approve"
        ))
        .json(&json!({"node_id": "approval", "envelope": envelope("approve-failed-verification")}))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status");

    client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-failed-verification-execute"),
            "receipt_digest": receipt_digest("execute")
        }))
        .send()
        .await
        .expect("execute completion")
        .error_for_status()
        .expect("execute status");

    let failed: Value = client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/nodes/verify/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-failed-verification-verify"),
            "receipt_digest": receipt_digest("verify"),
            "evidence": {"tests": [{
                "name": "cargo test --quiet",
                "status": "failed",
                "duration_ms": 12,
                "details": "fixture assertion failed"
            }]}
        }))
        .send()
        .await
        .expect("verify completion")
        .error_for_status()
        .expect("verify status")
        .json()
        .await
        .expect("failed run response");
    assert_eq!(
        failed["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == "verify")
            .unwrap()["state"],
        "failed"
    );
    assert_eq!(failed["review"]["tests"][0]["status"], "failed");

    let blocked = client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/nodes/review/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-blocked-review"),
            "receipt_digest": receipt_digest("review")
        }))
        .send()
        .await
        .expect("blocked review request");
    assert_eq!(blocked.status(), 409);

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}
