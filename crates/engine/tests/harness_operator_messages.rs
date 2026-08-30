use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
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
        operator_id: "discord-user-1".to_string(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().expect("loopback")),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (bound, shutdown, handle)
}

fn mutation_envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-operator-test",
            "approval_id": "approval-operator-test",
            "ledger_writes": ["test-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": Utc::now().to_rfc3339()
        },
        "idempotency_key": key
    })
}

fn gateway_message(message_id: &str, text: &str) -> Value {
    let timestamp = Utc::now().to_rfc3339();
    json!({
        "operator": {
            "operator_id": "discord-user-1",
            "authenticated": true,
            "authentication_method": "gateway_identity",
            "authenticated_at": timestamp
        },
        "adapter_id": "hermes-discord-default",
        "event": {
            "text": text,
            "message_type": "text",
            "user_id": "discord-user-1",
            "user_name": "operator",
            "source": {
                "platform": "discord",
                "chat_id": "discord-dm-1",
                "chat_type": "dm",
                "thread_id": null,
                "message_id": message_id
            },
            "message_id": message_id,
            "media_urls": [],
            "media_types": [],
            "timestamp": timestamp,
            "prompt_response": null
        }
    })
}

fn approval_graph(run_id: &str, node_id: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": "approval",
            "state": "pending",
            "authority": "human_approval",
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
            "created_by": "operator-test",
            "parent_receipts": []
        }
    })
}

fn execution_graph(run_id: &str, node_id: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": "execute",
            "state": "pending",
            "authority": "read_only",
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
            "created_by": "operator-test",
            "parent_receipts": []
        }
    })
}

async fn attach_and_plan(client: &reqwest::Client, bound: std::net::SocketAddr, run_id: &str) {
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract, "envelope": mutation_envelope("attach-operator-test")}))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": approval_graph(run_id, "approval"),
            "envelope": mutation_envelope(&format!("plan-{run_id}"))
        }))
        .send()
        .await
        .expect("plan")
        .error_for_status()
        .expect("plan status");
}

#[tokio::test]
async fn authenticated_gateway_capture_is_durable_and_duplicate_safe() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    let body = gateway_message(
        "discord-capture-1",
        "arda capture buy transplant-safe groceries",
    );

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("capture")
        .error_for_status()
        .expect("capture status")
        .json()
        .await
        .expect("capture body");
    assert!(response["summary"]
        .as_str()
        .is_some_and(|text| text.starts_with("Captured")));

    let inbox: Value = client
        .get(format!("http://{bound}/v1/personal/inbox"))
        .header("x-arda-operator-id", "discord-user-1")
        .send()
        .await
        .expect("inbox")
        .error_for_status()
        .expect("inbox status")
        .json()
        .await
        .expect("inbox body");
    assert_eq!(
        inbox["inbox"][0]["content"],
        "buy transplant-safe groceries"
    );

    let duplicate = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("duplicate");
    assert_eq!(duplicate.status(), 409);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_context_returns_the_canonical_cross_domain_next_action() {
    let root = TempDir::new().expect("root");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        json!({
            "id": "operator-next",
            "title": "Review Arda against the operator vision",
            "status": "pending",
            "priority": "critical",
            "owner": "discord-user-1",
            "origin": "operator-authored-session-objective",
            "meta": {"mutation_risk": "review_required", "execution_authority": "none_until_review", "lifecycle_phase": "current"}
        })
        .to_string()
            + "\n",
    )
    .unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;

    let response: Value = reqwest::Client::new()
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-context-next", "arda context"))
        .send()
        .await
        .expect("context")
        .error_for_status()
        .expect("context status")
        .json()
        .await
        .expect("context body");

    assert!(response["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Review Arda against the operator vision")));
    assert_eq!(response["evidence_refs"][1], "arda://next-action");

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objectives_reads_the_canonical_operator_projection() {
    let root = TempDir::new().expect("root");
    fs::create_dir_all(root.path().join("data/runs")).unwrap();
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        &queue,
        format!(
            "{}\n",
            json!({
                "id": "task-deferred",
                "title": "Resume deferred repair",
                "status": "blocked",
                "priority": "high",
                "continuation_decision": "wait_until",
                "detail": "waiting for the dependency window",
                "meta": {
                    "objective_id": "objective-deferred",
                    "project_id": "project-deferred"
                }
            })
        ),
    )
    .unwrap();
    fs::write(
        root.path().join("core/projects/tasks/schedules.jsonl"),
        format!(
            "{}\n",
            json!({
                "contract": "arda.workbench.schedule_record.v1",
                "task_id": "task-deferred",
                "objective_id": "objective-deferred",
                "mode": "deferred",
                "state": "scheduled",
                "not_before_utc": "2030-01-01T00:00:00Z",
                "recorded_at_utc": "2026-08-30T00:00:00Z"
            })
        ),
    )
    .unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;

    let response: Value = reqwest::Client::new()
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-objectives-1", "arda objectives"))
        .send()
        .await
        .expect("objectives")
        .error_for_status()
        .expect("objectives status")
        .json()
        .await
        .expect("objectives body");

    let summary = response["summary"].as_str().expect("summary");
    assert!(summary.contains("Objectives: 1"));
    assert!(summary.contains("objective-deferred [blocked]"));
    assert!(summary.contains("task=task-deferred"));
    assert!(summary.contains("next=wait_until"));
    assert!(summary.contains("wake=2030-01-01T00:00:00Z"));
    assert!(summary.contains("blocker=waiting for the dependency window"));
    assert_eq!(response["evidence_refs"][1], "arda://operator-projection");

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_controls_mutate_only_canonical_queue_and_schedule_ledgers() {
    let root = TempDir::new().expect("root");
    fs::create_dir_all(root.path().join("data/runs")).unwrap();
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        &queue,
        format!(
            "{}\n",
            json!({
                "id": "task-control",
                "title": "Original operator objective",
                "owner": "prometheus",
                "priority": "medium",
                "status": "queued",
                "meta": {
                    "action_class": "approved_autopilot_plan_step",
                    "mutation_risk": "operator-approved",
                    "execution_authority": "arda_workbench",
                    "source_objective_packet_id": "objective-control",
                    "approval_packet_id": "approval-1"
                }
            })
        ),
    )
    .unwrap();
    let schedules = root.path().join("core/projects/tasks/schedules.jsonl");
    fs::write(
        &schedules,
        format!(
            "{}\n",
            json!({
                "contract": "arda.workbench.schedule_record.v1",
                "task_id": "task-control",
                "objective_id": "objective-control",
                "mode": "once",
                "state": "scheduled",
                "not_before_utc": "2030-01-01T00:00:00Z",
                "recorded_at_utc": "2026-08-30T00:00:00Z"
            })
        ),
    )
    .unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    for (message_id, command, expected) in [
        (
            "discord-pause-task",
            "arda pause-task task-control objective-control operator requested pause",
            "Paused schedule for task-control.",
        ),
        (
            "discord-resume-task",
            "arda resume-task task-control objective-control operator requested resume",
            "Resumed schedule for task-control.",
        ),
        (
            "discord-reprioritize-task",
            "arda reprioritize task-control objective-control critical urgent operator priority",
            "Reprioritized task-control to critical.",
        ),
        (
            "discord-revise-objective",
            "arda revise-objective task-control objective-control Revised operator objective --reason operator corrected scope",
            "Revised objective for task-control; fresh approval is required.",
        ),
        (
            "discord-approve-objective",
            "arda approve-objective task-control objective-control operator accepts revision",
            "Approved revised objective for task-control.",
        ),
    ] {
        let response = client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, command))
            .send()
            .await
            .expect("control");
        let status = response.status();
        let body = response.text().await.expect("control body");
        assert!(status.is_success(), "{command}: {status} {body}");
        let response: Value = serde_json::from_str(&body).expect("control JSON");
        assert_eq!(response["summary"], expected);
    }

    let schedule_rows = fs::read_to_string(&schedules).unwrap();
    assert_eq!(schedule_rows.lines().count(), 3);
    let resumed: Value = serde_json::from_str(schedule_rows.lines().last().unwrap()).unwrap();
    assert_eq!(resumed["state"], "scheduled");
    let queue_rows = fs::read_to_string(&queue).unwrap();
    assert_eq!(queue_rows.lines().count(), 4);
    let approved: Value = serde_json::from_str(queue_rows.lines().last().unwrap()).unwrap();
    assert_eq!(
        approved["contract"],
        "arda.workbench.objective_revision_approval.v1"
    );
    assert_eq!(approved["title"], "Revised operator objective");
    assert_eq!(approved["priority"], "critical");
    assert_eq!(approved["reviewed_by"], "discord-user-1");
    assert_eq!(
        approved["meta"]["approval_packet_id"],
        "gateway:discord-approve-objective"
    );
    let operator_rows = fs::read_to_string(
        root.path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl"),
    )
    .unwrap();
    assert!(operator_rows
        .lines()
        .all(|row| { serde_json::from_str::<Value>(row).unwrap()["operation"] == "control" }));

    attach_and_plan(&client, bound, "queue-task-control").await;
    let cancelled: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-task",
            "arda cancel-task task-control operator no longer wants this objective",
        ))
        .send()
        .await
        .expect("cancel task")
        .error_for_status()
        .expect("cancel task status")
        .json()
        .await
        .expect("cancel task body");
    assert_eq!(
        cancelled["summary"],
        "Cancelled canonical task task-control."
    );
    let cancelled_schedule: Value = serde_json::from_str(
        fs::read_to_string(&schedules)
            .unwrap()
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(cancelled_schedule["state"], "cancelled");
    let cancelled_task: Value =
        serde_json::from_str(fs::read_to_string(&queue).unwrap().lines().last().unwrap()).unwrap();
    assert_eq!(cancelled_task["status"], "failed");
    assert_eq!(cancelled_task["result"], "cancelled");
    let operator_rows = fs::read_to_string(
        root.path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl"),
    )
    .unwrap();
    let last_operator_row: Value =
        serde_json::from_str(operator_rows.lines().last().unwrap()).unwrap();
    assert_eq!(last_operator_row["operation"], "cancel");

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_research_command_persists_question_without_creating_commitment() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    let body = gateway_message(
        "discord-research-1",
        "arda research practical x402 earning opportunities",
    );

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("research")
        .error_for_status()
        .expect("research status")
        .json()
        .await
        .expect("research body");

    assert!(response["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Research question")));
    let registry: Value = serde_json::from_str(
        &fs::read_to_string(root.path().join("data/workbench/research/questions.json"))
            .expect("question registry"),
    )
    .expect("question registry json");
    assert_eq!(registry["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        registry["records"][0]["question"],
        "practical x402 earning opportunities"
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());

    let duplicate = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("duplicate research");
    assert_eq!(duplicate.status(), 409);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn authenticated_gateway_approval_cancel_and_resume_use_canonical_runs() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    attach_and_plan(&client, bound, "phone-approve").await;
    let approved: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-approve-1",
            "arda approve phone-approve approval",
        ))
        .send()
        .await
        .expect("approve")
        .error_for_status()
        .expect("approve status")
        .json()
        .await
        .expect("approve body");
    assert_eq!(approved["run_id"], "phone-approve");
    let run: Value = client
        .get(format!("http://{bound}/v1/runs/phone-approve"))
        .send()
        .await
        .expect("approved run")
        .error_for_status()
        .expect("approved run status")
        .json()
        .await
        .expect("approved run body");
    assert_eq!(run["graph"]["nodes"][0]["state"], "succeeded");

    attach_and_plan(&client, bound, "phone-cancel").await;
    client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-1",
            "arda cancel phone-cancel operator requested stop",
        ))
        .send()
        .await
        .expect("cancel")
        .error_for_status()
        .expect("cancel status");
    let resumed: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-resume-1",
            "arda resume phone-cancel",
        ))
        .send()
        .await
        .expect("resume")
        .error_for_status()
        .expect("resume status")
        .json()
        .await
        .expect("resume body");
    assert!(resumed["summary"]
        .as_str()
        .is_some_and(|text| text.contains("approval=cancelled")));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objective_context_status_and_result_use_canonical_state() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach_and_plan(&client, bound, "phone-status").await;

    let objective: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objective-1",
            &format!("arda objective {PROJECT_ID} finish the operator bridge"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective body");
    assert!(objective["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains(PROJECT_ID)));
    let personal_ledger = std::fs::read_to_string(root.path().join("data/personal/events.jsonl"))
        .expect("personal ledger");
    assert!(personal_ledger.contains(PROJECT_ID));
    let task_ledger = std::fs::read_to_string(root.path().join("core/projects/tasks/queue.jsonl"))
        .expect("canonical project task ledger");
    let task: Value = serde_json::from_str(task_ledger.trim()).expect("project task record");
    assert_eq!(task["project_id"], PROJECT_ID);
    assert_eq!(task["title"], "finish the operator bridge");
    assert_eq!(task["status"], "pending");
    assert_eq!(task["meta"]["execution_authority"], "none_until_review");
    assert!(objective["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Execution still requires review")));

    let context: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-context-1", "arda context"))
        .send()
        .await
        .expect("context")
        .error_for_status()
        .expect("context status")
        .json()
        .await
        .expect("context body");
    assert!(context["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Next action: objective-phone-status")));

    let status: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("discord-status-1", "arda status"))
        .send()
        .await
        .expect("status")
        .error_for_status()
        .expect("status status")
        .json()
        .await
        .expect("status body");
    assert!(status["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("awaiting approval: 1")));

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": execution_graph("phone-result", "execute"),
            "envelope": mutation_envelope("plan-phone-result")
        }))
        .send()
        .await
        .expect("plan result")
        .error_for_status()
        .expect("plan result status");
    let receipt_digest = format!("sha256:{}", "a".repeat(64));
    client
        .post(format!(
            "http://{bound}/v1/runs/phone-result/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": mutation_envelope("complete-phone-result"),
            "receipt_digest": receipt_digest,
            "evidence": {
                "changes": [{
                    "path": "artifacts/report.md",
                    "status": "added",
                    "additions": 4,
                    "deletions": 0
                }],
                "tests": [{"name": "operator-result-check", "status": "passed"}]
            }
        }))
        .send()
        .await
        .expect("complete result")
        .error_for_status()
        .expect("complete result status");
    let result: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-result-1",
            "arda result phone-result",
        ))
        .send()
        .await
        .expect("result")
        .error_for_status()
        .expect("result status")
        .json()
        .await
        .expect("result body");
    assert!(result["summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("Verified tests: 1/1")));
    assert!(result["evidence_refs"].as_array().is_some_and(|refs| {
        refs.iter().any(|reference| {
            reference
                .as_str()
                .is_some_and(|reference| reference.ends_with("files/artifacts/report.md"))
        })
    }));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_reject_and_revise_consume_scoped_decisions() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    for (run_id, message_id, command, expected_operation) in [
        (
            "phone-reject",
            "discord-reject-1",
            "arda reject phone-reject approval scope is too broad",
            "reject",
        ),
        (
            "phone-revise",
            "discord-revise-1",
            "arda revise phone-revise approval narrow the requested change",
            "revise",
        ),
    ] {
        attach_and_plan(&client, bound, run_id).await;
        client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, command))
            .send()
            .await
            .expect("decision")
            .error_for_status()
            .expect("decision status");
        let run: Value = client
            .get(format!("http://{bound}/v1/runs/{run_id}"))
            .send()
            .await
            .expect("decision run")
            .error_for_status()
            .expect("decision run status")
            .json()
            .await
            .expect("decision run body");
        assert_eq!(run["graph"]["nodes"][0]["state"], "cancelled");
        let ledger = std::fs::read_to_string(
            root.path()
                .join("core/state/orome/operator-session/operator_sessions.jsonl"),
        )
        .expect("operator ledger");
        let event: Value = serde_json::from_str(ledger.lines().last().expect("decision event"))
            .expect("decision json");
        assert_eq!(event["operation"], expected_operation);
        assert_eq!(event["approval"]["single_use_state"], "consumed");
    }

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_reminder_acknowledgement_requires_a_delivered_attempt() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    let capture: Value = client
        .post(format!("http://{bound}/v1/personal/captures"))
        .header("x-arda-operator-id", "discord-user-1")
        .header("idempotency-key", "reminder-owner-capture")
        .json(&json!({
            "operator_id": "discord-user-1",
            "text": "Reminder ownership fixture"
        }))
        .send()
        .await
        .expect("capture request")
        .error_for_status()
        .expect("capture status")
        .json()
        .await
        .expect("capture body");
    let item_id = capture["capture_id"]
        .as_str()
        .expect("canonical capture id")
        .to_string();
    let reminder_id = uuid::Uuid::new_v4().to_string();

    let out_of_order = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-ack-early",
            &format!("arda acknowledge {reminder_id}"),
        ))
        .send()
        .await
        .expect("out-of-order acknowledgement");
    assert_eq!(out_of_order.status(), 409);

    client
        .post(format!("http://{bound}/v1/personal/reminders/attempt"))
        .header("x-arda-operator-id", "discord-user-1")
        .header("idempotency-key", "reminder-attempt-1")
        .json(&json!({
            "operator_id": "discord-user-1",
            "item_id": item_id,
            "reminder_id": reminder_id,
            "state": "delivered",
            "provider_message_id": "provider-reminder-1"
        }))
        .send()
        .await
        .expect("reminder attempt")
        .error_for_status()
        .expect("reminder attempt status");
    let acknowledged: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-ack-1",
            &format!("arda acknowledge {reminder_id}"),
        ))
        .send()
        .await
        .expect("acknowledgement")
        .error_for_status()
        .expect("acknowledgement status")
        .json()
        .await
        .expect("acknowledgement body");
    assert!(acknowledged["summary"]
        .as_str()
        .is_some_and(|summary| summary.ends_with("acknowledged.")));

    let duplicate_terminal = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-defer-late",
            &format!("arda defer {reminder_id}"),
        ))
        .send()
        .await
        .expect("late defer");
    assert_eq!(duplicate_terminal.status(), 409);

    let deferred_reminder_id = uuid::Uuid::new_v4().to_string();
    client
        .post(format!("http://{bound}/v1/personal/reminders/attempt"))
        .header("x-arda-operator-id", "discord-user-1")
        .header("idempotency-key", "reminder-attempt-2")
        .json(&json!({
            "operator_id": "discord-user-1",
            "item_id": item_id,
            "reminder_id": deferred_reminder_id,
            "state": "delivered",
            "provider_message_id": "provider-reminder-2"
        }))
        .send()
        .await
        .expect("second reminder attempt")
        .error_for_status()
        .expect("second reminder attempt status");
    let deferred: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-defer-1",
            &format!("arda defer {deferred_reminder_id}"),
        ))
        .send()
        .await
        .expect("defer")
        .error_for_status()
        .expect("defer status")
        .json()
        .await
        .expect("defer body");
    assert!(deferred["summary"]
        .as_str()
        .is_some_and(|summary| summary.ends_with("deferred.")));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_private_capture_rejects_group_audience_without_mutation() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    let mut message = gateway_message("group-private-capture", "arda capture private medical note");
    message["event"]["source"]["chat_type"] = json!("group");

    let response = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .expect("send group capture");
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(
        !root.path().join("data/personal/events.jsonl").exists(),
        "wrong-audience capture must not mutate personal state"
    );
    assert!(
        !root
            .path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl")
            .exists(),
        "wrong-audience capture must be rejected before bridge ingestion"
    );

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_operator_endpoint_rejects_unauthenticated_identity() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    let mut body = gateway_message("discord-denied-1", "arda capture denied");
    body["operator"]["authenticated"] = json!(false);

    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("denied");
    assert_eq!(denied.status(), 403);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_council_query_projects_tension_and_decision_without_approval() {
    let root = TempDir::new().expect("root");
    let council_dir = root.path().join("data/runs/council-run-1");
    fs::create_dir_all(&council_dir).expect("council run directory");
    fs::write(
        council_dir.join("council-run.json"),
        include_bytes!("../../../spec/council-run/v1/fixtures/valid-independent-disagreement.json"),
    )
    .expect("council fixture");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();

    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-council-1",
            "arda council council-run-1",
        ))
        .send()
        .await
        .expect("council query")
        .error_for_status()
        .expect("council query status")
        .json()
        .await
        .expect("council query body");
    let summary = response["summary"].as_str().expect("summary");
    assert!(summary.contains("Material tension:"));
    assert!(summary.contains("Decision requested:"));
    assert!(summary.contains("operator approval has not been granted"));
    assert_eq!(
        response["evidence_refs"]
            .as_array()
            .unwrap()
            .last()
            .unwrap(),
        "arda://runs/council-run-1/council"
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}
