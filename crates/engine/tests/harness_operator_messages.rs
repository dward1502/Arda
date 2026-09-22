use arda_engine::objectives::{ObjectiveState, ObjectiveStore};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Notify;

#[path = "fixtures/gateway.rs"]
mod gateway;
use gateway::*;
#[path = "fixtures/admitted_evidence.rs"]
mod admitted_evidence;
#[path = "fixtures/gateway_authority.rs"]
mod gateway_authority;

const PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

#[path = "fixtures/context_provider.rs"]
mod context_provider;
mod research_delivery;

async fn start_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    start_research_harness(root, None).await
}

async fn start_research_harness(
    root: &TempDir,
    warden_scout_url: Option<String>,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    start_research_harness_at(root.path(), warden_scout_url).await
}

async fn start_capability_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    std::env::set_var("ARDA_HERMES_GATEWAY_CAPABILITY", GATEWAY_CAPABILITY);
    start_harness(root).await
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

fn created_objective_id(response: &Value) -> &str {
    response["evidence_refs"]
        .as_array()
        .expect("evidence refs")
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|reference| reference.strip_prefix("arda://objectives/"))
        .find(|objective_id| !objective_id.contains('/'))
        .expect("resident objective reference")
}

#[tokio::test]
async fn gateway_cannot_initialize_lost_authority() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let directory = root.path().join("data/arda");
    if directory.exists() {
        fs::rename(&directory, root.path().join("saved-arda")).unwrap();
    }
    let response = gateway_client()
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("lost-authority", "arda objectives"))
        .send()
        .await
        .unwrap();
    assert!(!response.status().is_success());
    assert!(
        !directory.exists(),
        "ingress must not initialize missing authority"
    );
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn recovery_snapshot_deletion_requires_private_owner_and_closed_evidence() {
    // Unknown transports must not be promoted to operator-private authority.
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    for kind in ["thread", "unknown", "", "DM", "operator_private", "group"] {
        for command in [
            "arda context",
            "arda objective-from-brief missing 550e8400-e29b-41d4-a716-446655440000 Inspect",
            "arda recover-retained objective leaf run one-verify-start 30m reuse-expired-context",
        ] {
            let mut message = gateway_message(&format!("private-{kind}-{command}"), command);
            message["event"]["source"]["chat_type"] = json!(kind);
            let response = gateway_client()
                .post(format!("http://{bound}/v1/operator/messages"))
                .json(&message)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 403, "{kind}: {command}");
        }
    }
    shutdown.notify_waiters();
    handle.await.unwrap();
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let endpoint = format!("http://{bound}/v1/operator/messages");
    let path = root.path().join("data/arda/objectives.sqlite3");
    let _store = ObjectiveStore::open(&path).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    // Isolated persisted-state fixture; no provider or live objective is used.
    db.execute_batch("INSERT INTO objectives
        (id, source_id, ingress_key, payload_digest, operator_id, text, priority, revision,
         approved_revision, state, terminal_receipt_digest, created_at_ms, updated_at_ms) VALUES
        ('objective-delete', 'source-delete', 'ingress-delete', 'payload', 'discord-user-1',
         'test', 0, 1, 1, 'completed', 'terminal-digest', 0, 0);
        INSERT INTO leaves (id, objective_id, workspace_root, authority, stage, attempt,
                            execution_run_id, context_bound, current_receipt_digest, updated_at_ms)
        VALUES ('leaf-delete', 'objective-delete', '/fixture', 'test', 'complete', 1,
                'run-delete', 1, 'close-digest', 0);
        INSERT INTO stage_receipts (leaf_id, stage, contract, digest, run_path, provider, model,
                                   started_at_ms, completed_at_ms, verdict, recorded_at_ms)
        VALUES ('leaf-delete', 'close', 'fixture', 'close-digest', 'data/runs/run-delete/close.json',
                'fixture', 'fixture', 0, 1, 'passed', 1);
        INSERT INTO resident_context_bindings VALUES
            ('run-delete', 'request-digest', 'private snapshot', NULL),
            ('other-run', 'other-digest', 'other snapshot', NULL);").unwrap();
    let command = "arda delete-recovery-context objective-delete run-delete";
    let body = gateway_message("delete-snapshot", command);
    assert_eq!(
        reqwest::Client::new()
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let mut public = body.clone();
    public["event"]["source"]["chat_type"] = json!("group");
    assert_eq!(
        gateway_client()
            .post(&endpoint)
            .json(&public)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let mut foreign = body.clone();
    foreign["operator"]["operator_id"] = json!("other-operator");
    foreign["event"]["user_id"] = json!("other-operator");
    assert_eq!(
        gateway_client()
            .post(&endpoint)
            .json(&foreign)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    for sql in [
        "UPDATE objectives SET state = 'running'",
        "UPDATE objectives SET state = 'cancelled'; UPDATE leaves SET stage = 'cancelled'",
        "UPDATE objectives SET state = 'failed'; UPDATE leaves SET stage = 'verify'",
        "UPDATE objectives SET state = 'completed'; UPDATE leaves SET stage = 'complete', current_receipt_digest = 'not-the-close-receipt'",
        "UPDATE leaves SET current_receipt_digest = 'close-digest'; INSERT INTO leaves (id, objective_id, workspace_root, authority, stage, updated_at_ms) VALUES ('unfinished-sibling', 'objective-delete', '/sibling', 'test', 'verify', 0)",
        "DELETE FROM leaves WHERE id = 'unfinished-sibling'; UPDATE leaves SET execution_run_id = 'different-run'",
        "UPDATE leaves SET execution_run_id = 'run-delete', context_bound = NULL",
        "UPDATE leaves SET context_bound = 1, lease_expires_ms = 9223372036854775807",
        "UPDATE leaves SET lease_expires_ms = NULL; UPDATE objectives SET operator_id = 'other-operator'",
    ] {
        db.execute_batch(sql).unwrap();
        assert_eq!(gateway_client().post(&endpoint).json(&body).send().await.unwrap().status(), 409, "{sql}");
        let snapshot: String = db.query_row("SELECT assembly_json FROM resident_context_bindings WHERE run_id = 'run-delete'", [], |row| row.get(0)).unwrap();
        assert_eq!(snapshot, "private snapshot");
    }
    db.execute_batch("UPDATE objectives SET operator_id = 'discord-user-1'")
        .unwrap();
    let accepted = gateway_client()
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status(), 200, "{}", accepted.text().await.unwrap());
    let read = || {
        db.query_row(
        "SELECT assembly_json, deleted_by_operator_ms, request_digest FROM resident_context_bindings WHERE run_id = 'run-delete'",
        [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, String>(2)?)),
    ).unwrap()
    };
    let deleted = read();
    assert_eq!(deleted.0, "");
    assert!(deleted.1.is_some());
    assert_eq!(deleted.2, "request-digest");
    // ObjectiveStore replay is idempotent; Oromë rejects a duplicate transport
    // event rather than appending a second operator-session record.
    let replay = gateway_client()
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 409);
    assert!(replay
        .text()
        .await
        .unwrap()
        .contains("duplicate transport event"));
    assert_eq!(read(), deleted);
    let changed = gateway_message(
        "delete-snapshot",
        "arda delete-recovery-context objective-delete other-run",
    );
    assert_eq!(
        gateway_client()
            .post(&endpoint)
            .json(&changed)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    assert_eq!(
        db.query_row(
            "SELECT assembly_json FROM resident_context_bindings WHERE run_id = 'other-run'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "other snapshot"
    );
    assert_eq!(
        db.query_row("SELECT digest FROM stage_receipts", [], |row| row
            .get::<_, String>(0))
            .unwrap(),
        "close-digest"
    );
    drop(db);
    ObjectiveStore::open(&path).unwrap();
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn gateway_capability_is_required_before_operator_ingestion() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_capability_harness(&root).await;
    let endpoint = format!("http://{bound}/v1/operator/messages");
    let body = gateway_message("discord-capability-rejection", "arda status");

    let missing = reqwest::Client::new()
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .expect("missing capability response");
    assert_eq!(missing.status(), 403);

    let wrong = reqwest::Client::new()
        .post(&endpoint)
        .header("x-arda-gateway-capability", "wrong-capability")
        .json(&body)
        .send()
        .await
        .expect("wrong capability response");
    assert_eq!(wrong.status(), 403);
    assert!(!root
        .path()
        .join("core/state/orome/operator-session/operator_sessions.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn local_cli_intake_requires_distinct_authentication_and_preserves_provenance() {
    std::env::set_var("ARDA_HERMES_LOCAL_CAPABILITY", "test-local-capability");
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_capability_harness(&root).await;
    let endpoint = format!("http://{bound}/v1/operator/local-messages");
    let mut body = gateway_message("cli:session:message:operation", "arda status");
    body["operator"]["authentication_method"] = json!("local_session");
    body["adapter_id"] = json!("hermes-cli");
    body["event"]["source"]["platform"] = json!("cli");
    body["event"]["source"]["chat_id"] = json!("session");
    body["event"]["source"]["chat_type"] = json!("private");
    let client = reqwest::Client::new();
    let missing = client.post(&endpoint).json(&body).send().await.unwrap();
    assert_eq!(missing.status(), 403);
    let gateway_only = gateway_client()
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(gateway_only.status(), 403);
    let accepted = client
        .post(&endpoint)
        .header("x-arda-local-capability", "test-local-capability")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status(), 200);
    let wrong_route = gateway_client()
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_route.status(), 403);
    body["event"]["source"]["platform"] = json!("discord");
    let forged = client
        .post(&endpoint)
        .header("x-arda-local-capability", "test-local-capability")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(forged.status(), 403);
    shutdown.notify_waiters();
    handle.await.unwrap();
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
    let client = gateway_client();
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
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-context-objective")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    let objective: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-context-objective",
            &format!("arda objective {PROJECT_ID} Review Arda against the operator vision"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective body");

    let response: Value = client
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
    assert_eq!(
        response["evidence_refs"][1],
        format!("arda://objectives/{}", created_objective_id(&objective))
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objectives_reads_the_resident_objective_store() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-objectives-list")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objectives-create",
            &format!("arda objective {PROJECT_ID} Resume deferred repair"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective body");

    let response: Value = client
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
    assert!(summary.contains("authority=resident_objective_store"));
    assert!(summary.contains("[pending_approval]"));
    assert!(summary.contains("text=Resume deferred repair"));
    assert_eq!(
        response["evidence_refs"][2],
        format!("arda://objectives/{}", created_objective_id(&created))
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_controls_mutate_only_resident_objective_store() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-resident-controls")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");
    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-controls-objective",
            &format!("arda objective {PROJECT_ID} Original operator objective"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective response");
    let objective_id = created_objective_id(&created).to_owned();
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let task_id = store.list_leaves(&objective_id).expect("objective leaves")[0]
        .id
        .clone();

    let before_replay = store.objective(&objective_id).unwrap().unwrap();
    let event_path = root
        .path()
        .join("core/state/orome/operator-session/operator_sessions.jsonl");
    let events_before = fs::read(&event_path).unwrap();
    let replay = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-controls-objective",
            &format!("arda cancel-task {task_id} {objective_id} reused creation event"),
        ))
        .send()
        .await
        .expect("cross-command replay");
    assert_eq!(replay.status(), reqwest::StatusCode::CONFLICT);
    let after_replay = store.objective(&objective_id).unwrap().unwrap();
    assert_eq!(after_replay.state, before_replay.state);
    assert_eq!(after_replay.updated_at_ms, before_replay.updated_at_ms);
    assert_eq!(fs::read(&event_path).unwrap(), events_before);

    for (message_id, command, expected) in [
        (
            "discord-pause-task",
            format!("arda pause-task {task_id} {objective_id} operator requested pause"),
            format!("Paused resident objective {objective_id}: operator requested pause"),
        ),
        (
            "discord-resume-task",
            format!("arda resume-task {task_id} {objective_id} operator requested resume"),
            format!("Resumed resident objective {objective_id}: operator requested resume"),
        ),
        (
            "discord-reprioritize-task",
            format!("arda reprioritize {task_id} {objective_id} critical urgent operator priority"),
            format!("Reprioritized {task_id} to 100: urgent operator priority"),
        ),
        (
            "discord-revise-objective",
            format!(
                "arda revise-objective {task_id} {objective_id} Revised operator objective --reason operator corrected scope"
            ),
            "persisted execution plan".to_owned(),
        ),
        (
            "discord-approve-objective",
            format!("arda approve-objective {task_id} {objective_id} operator accepts revision"),
            format!("Approved resident objective {objective_id}: operator accepts revision"),
        ),
    ] {
        let response = client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, &command))
            .send()
            .await
            .expect("control");
        let status = response.status();
        let body = response.text().await.expect("control body");
        if command.starts_with("arda revise-objective ") {
            assert_eq!(status, reqwest::StatusCode::CONFLICT);
            assert!(body.contains(&expected));
            continue;
        }
        assert!(status.is_success(), "{command}: {status} {body}");
        let response: Value = serde_json::from_str(&body).expect("control JSON");
        assert_eq!(response["summary"], expected);
    }

    let approved = store
        .objective(&objective_id)
        .expect("read controlled objective")
        .expect("controlled objective");
    assert_eq!(approved.state, ObjectiveState::Approved);
    assert_eq!(approved.text, "Original operator objective");
    assert_eq!(approved.priority, 100);
    assert_eq!(approved.revision, 1);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
    let operator_rows = fs::read_to_string(
        root.path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl"),
    )
    .unwrap();
    let operator_event_count = operator_rows.lines().count();
    let wrong_lineage_command =
        format!("arda cancel-task {task_id} wrong-objective must not cancel");
    let wrong_lineage = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-task-wrong-objective",
            &wrong_lineage_command,
        ))
        .send()
        .await
        .expect("wrong-lineage cancellation");
    assert_eq!(wrong_lineage.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join("core/state/orome/operator-session/operator_sessions.jsonl"),
        )
        .unwrap()
        .lines()
        .count(),
        operator_event_count,
        "rejected canonical preflight must not append an operator session event"
    );
    let cancelled: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-cancel-task",
            &format!(
                "arda cancel-task {task_id} {objective_id} operator no longer wants this objective"
            ),
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
        format!(
            "Cancelled resident objective {objective_id}: operator no longer wants this objective"
        )
    );
    assert_eq!(
        store
            .objective(&objective_id)
            .expect("read cancelled objective")
            .expect("cancelled objective")
            .state,
        ObjectiveState::Cancelled
    );
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
async fn gateway_cross_family_replay_and_invalid_transport_cannot_create_objectives() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract, "envelope": mutation_envelope("attach-replay")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let url = format!("http://{bound}/v1/operator/messages");
    let store =
        ObjectiveStore::open_existing(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let status = gateway_message("cross-family", "arda context");
    client
        .post(&url)
        .json(&status)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let ledger = root
        .path()
        .join("core/state/orome/operator-session/operator_sessions.jsonl");
    let before = fs::read(&ledger).unwrap();
    // Pre-cutover ledger entries have no shared SQLite payload binding.
    rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3"))
        .unwrap()
        .execute("DELETE FROM gateway_event_bindings", [])
        .unwrap();
    let replay = gateway_message(
        "cross-family",
        &format!("arda objective {PROJECT_ID} must not be created"),
    );
    assert_eq!(
        client
            .post(&url)
            .json(&replay)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    assert!(store.list_objectives().unwrap().is_empty());
    assert_eq!(before, fs::read(&ledger).unwrap());
    let mut invalid = gateway_message(
        "invalid-transport",
        &format!("arda objective {PROJECT_ID} invalid adapter"),
    );
    invalid["adapter_id"] = json!("");
    assert!(!client
        .post(&url)
        .json(&invalid)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert!(store.list_objectives().unwrap().is_empty());
    assert_eq!(before, fs::read(&ledger).unwrap());

    // Fail the personal-event append after the resident transaction commits.
    // No transport event was consumed, but its payload binding must survive.
    fs::write(
        root.path().join("data/personal"),
        b"fixture blocks directory",
    )
    .unwrap();
    let original = gateway_message(
        "interrupted-event",
        &format!("arda objective {PROJECT_ID} retain exactly once"),
    );
    assert!(!client
        .post(&url)
        .json(&original)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert_eq!(store.list_objectives().unwrap().len(), 1);
    assert_eq!(before, fs::read(&ledger).unwrap());
    shutdown.notify_waiters();
    handle.await.unwrap();

    let (bound, shutdown, handle) = start_harness(&root).await;
    let url = format!("http://{bound}/v1/operator/messages");
    let mut changed = original.clone();
    changed["event"]["text"] = json!("arda context");
    assert_eq!(
        client
            .post(&url)
            .json(&changed)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CONFLICT
    );
    fs::remove_file(root.path().join("data/personal")).unwrap();
    client
        .post(&url)
        .json(&original)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(store.list_objectives().unwrap().len(), 1);

    let first = gateway_message("race-event", "arda context");
    let second = gateway_message(
        "race-event",
        &format!("arda objective {PROJECT_ID} at most one winner"),
    );
    let responses = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            client.post(&url).json(&first).send(),
            client.post(&url).json(&second).send()
        )
    })
    .await
    .expect("ledger locking must not deadlock the async executor");
    let first = responses.0.unwrap().status();
    let second = responses.1.unwrap().status();
    assert_ne!(first.is_success(), second.is_success());
    assert_eq!(
        if first.is_success() { second } else { first },
        reqwest::StatusCode::CONFLICT
    );
    assert_eq!(
        store.list_objectives().unwrap().len(),
        if second.is_success() { 2 } else { 1 }
    );
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn gateway_shared_objective_intake_is_pending_and_does_not_publish_personal_context() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let mut contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    contract["identity"]["name"] = json!("Routing review");
    contract["permissions"]["authority"] = json!("read_only");
    contract["permissions"]["filesystem"]["write"] = json!(false);
    contract["commands"] = json!([]);
    contract["checks"] = json!([]);
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract, "envelope": mutation_envelope("shared-attach")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let projects_before = fs::read(root.path().join("data/workbench/projects.json")).unwrap();
    let endpoint = format!("http://{bound}/v1/operator/messages");
    for kind in ["group", "thread", "channel", "guild"] {
        let project = if kind == "group" {
            "\"Routing review\""
        } else {
            PROJECT_ID
        };
        let mut message = gateway_message(
            &format!("shared-objective-{kind}"),
            &format!("arda objective {project} Inspect repository-only routing"),
        );
        message["event"]["source"]["chat_type"] = json!(kind);
        let response = client.post(&endpoint).json(&message).send().await.unwrap();
        assert_eq!(
            response.status(),
            200,
            "{kind}: {}",
            response.text().await.unwrap()
        );
        let body: Value = response.json().await.unwrap();
        assert!(body["summary"]
            .as_str()
            .unwrap()
            .contains("requires review"));
        assert!(!body.to_string().contains("personal/captures"));
        assert!(!body.to_string().contains(PROJECT_ID));
        let replay = client.post(&endpoint).json(&message).send().await.unwrap();
        assert_eq!(replay.status(), 409);
    }
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let objectives = store.list_objectives().unwrap();
    assert_eq!(objectives.len(), 4);
    for objective in objectives {
        assert_eq!(objective.state, ObjectiveState::PendingApproval);
        assert_eq!(objective.project_ids, vec![PROJECT_ID]);
        assert!(store.list_leaves(&objective.id).unwrap().iter()
            .all(|leaf| leaf.authority == "read_only"));
    }
    assert_eq!(
        projects_before,
        fs::read(root.path().join("data/workbench/projects.json")).unwrap()
    );
    assert!(!root.path().join("data/runs").exists());
    // Shared intake must not create a personal capture or disclose its references.
    assert!(!root.path().join("data/personal").exists());
    for kind in ["unknown", "", "operator_private"] {
        let mut message = gateway_message(
            &format!("unknown-objective-{kind}"),
            &format!("arda objective {PROJECT_ID} Inspect"),
        );
        message["event"]["source"]["chat_type"] = json!(kind);
        assert_eq!(
            client
                .post(&endpoint)
                .json(&message)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    for (index, (text, expected)) in [
        ("arda capture private note", 403),
        ("arda context", 403),
        ("arda status", 403),
        (
            "arda objective-from-brief private-brief \"Routing review\" inspect",
            403,
        ),
        ("arda objective \"Missing project\" inspect", 404),
        ("arda objective \"routing review\" inspect", 404),
        ("arda objective \"Routing review\"inspect", 400),
        ("arda objective \"Routing review", 400),
    ]
    .iter()
    .enumerate()
    {
        let mut message = gateway_message(&format!("rejected-shared-{index}"), text);
        message["event"]["source"]["chat_type"] = json!("group");
        let response = client.post(&endpoint).json(&message).send().await.unwrap();
        assert_eq!(
            response.status().as_u16(),
            *expected,
            "{text}: {}",
            response.text().await.unwrap()
        );
    }
    let mut unauthenticated = gateway_message(
        "unauthenticated-shared",
        "arda objective \"Routing review\" inspect",
    );
    unauthenticated["event"]["source"]["chat_type"] = json!("group");
    unauthenticated["operator"]["authenticated"] = json!(false);
    assert_eq!(
        client
            .post(&endpoint)
            .json(&unauthenticated)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let mut duplicate_name = contract.clone();
    duplicate_name["identity"]["project_id"] = json!("550e8400-e29b-41d4-a716-446655440001");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(
            &json!({"contract": duplicate_name, "envelope": mutation_envelope("ambiguous-attach")}),
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let mut ambiguous = gateway_message(
        "ambiguous-project-name",
        "arda objective \"Routing review\" inspect",
    );
    ambiguous["event"]["source"]["chat_type"] = json!("group");
    let response = client
        .post(&endpoint)
        .json(&ambiguous)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 409);
    assert!(response.text().await.unwrap().contains("ambiguous"));
    assert_eq!(store.list_objectives().unwrap().len(), 4);
    assert!(!root.path().join("data/personal").exists());
    let selected = store.list_objectives().unwrap().remove(0);
    let private = gateway_message(
        "private-intake-peer",
        &format!("arda objective {PROJECT_ID} Private objective text"),
    );
    client
        .post(&endpoint)
        .json(&private)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let private_id = store
        .list_objectives()
        .unwrap()
        .into_iter()
        .find(|objective| objective.text == "Private objective text")
        .unwrap()
        .id;
    let task_id = format!("{}-project-1", selected.id);
    let mut status = gateway_message("shared-status", "arda objectives");
    status["event"]["source"]["chat_type"] = json!("group");
    let response = client.post(&endpoint).json(&status).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert!(body["summary"]
        .as_str()
        .unwrap()
        .contains("pending_approval"));
    assert!(body["summary"].as_str().unwrap().contains(&selected.id));
    assert!(!body.to_string().contains("Inspect repository-only routing"));
    assert!(!body.to_string().contains(PROJECT_ID));
    assert!(!body.to_string().contains(&private_id));
    assert!(body["summary"]
        .as_str()
        .unwrap()
        .contains("arda pause-task"));
    let mut private_control = gateway_message(
        "shared-private-pause",
        &format!("arda pause-task {private_id}-project-1 {private_id} hold"),
    );
    private_control["event"]["source"]["chat_type"] = json!("group");
    assert_eq!(
        client
            .post(&endpoint)
            .json(&private_control)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    for (index, (field, value)) in [
        ("chat_id", "another-room"),
        ("thread_id", "another-thread"),
        ("platform", "telegram"),
    ]
    .iter()
    .enumerate()
    {
        let mut other = gateway_message(&format!("other-status-{index}"), "arda objectives");
        other["event"]["source"]["chat_type"] = json!("group");
        other["event"]["source"][*field] = json!(value);
        let response = client.post(&endpoint).json(&other).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert!(!response.text().await.unwrap().contains(&selected.id));
        let mut control = gateway_message(
            &format!("other-pause-{index}"),
            &format!("arda pause-task {task_id} {} hold", selected.id),
        );
        control["event"]["source"]["chat_type"] = json!("group");
        control["event"]["source"][*field] = json!(value);
        assert_eq!(
            client
                .post(&endpoint)
                .json(&control)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    for (verb, expected_state) in [
        ("pause-task", ObjectiveState::Paused),
        ("resume-task", ObjectiveState::PendingApproval),
        ("cancel-task", ObjectiveState::Cancelled),
    ] {
        let mut control = gateway_message(
            &format!("same-room-{verb}"),
            &format!("arda {verb} {task_id} {} operator request", selected.id),
        );
        control["event"]["source"]["chat_type"] = json!("group");
        let response = client.post(&endpoint).json(&control).send().await.unwrap();
        assert_eq!(
            response.status(),
            200,
            "{verb}: {}",
            response.text().await.unwrap()
        );
        assert_eq!(
            store.objective(&selected.id).unwrap().unwrap().state,
            expected_state
        );
        assert_eq!(
            client
                .post(&endpoint)
                .json(&control)
                .send()
                .await
                .unwrap()
                .status(),
            409
        );
    }
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn gateway_python_bridge_roundtrip_uses_real_http_and_storage() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let mut contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    contract["identity"]["name"] = json!("Routing review");
    gateway_client()
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract, "envelope": mutation_envelope("roundtrip-attach")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        tokio::process::Command::new("python3")
            .arg(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../adapters/hermes-operator-bridge/test_harness_roundtrip.py"),
            )
            .arg(format!("http://{bound}"))
            .env("HERMES_HOME", root.path().join("hermes"))
            .env("ARDA_OPERATOR_ID", "discord-user-1")
            .env("ARDA_HERMES_GATEWAY_CAPABILITY", GATEWAY_CAPABILITY)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["controls"], 3);
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let objectives = store.list_objectives().unwrap();
    assert_eq!(objectives.len(), 1);
    assert_eq!(objectives[0].state, ObjectiveState::Cancelled);
    assert_eq!(objectives[0].text, "Roundtrip inspection");
    assert!(!root.path().join("data/runs").exists());
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn gateway_multi_project_objective_preserves_all_attached_project_authorities() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let first: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    let mut second = first.clone();
    let second_id = "550e8400-e29b-41d4-a716-446655440001";
    second["identity"]["project_id"] = Value::String(second_id.into());
    second["identity"]["name"] = Value::String("second-real-project".into());
    for (contract, key) in [
        (first, "attach-multi-objective-first"),
        (second, "attach-multi-objective-second"),
    ] {
        client
            .post(format!("http://{bound}/v1/projects/attach"))
            .json(&json!({
                "contract": contract,
                "envelope": mutation_envelope(key)
            }))
            .send()
            .await
            .expect("attach")
            .error_for_status()
            .expect("attach status");
    }

    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-multi-project-objective",
            &format!(
                "arda objective {PROJECT_ID},{second_id} inspect both real projects and join the evidence"
            ),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("multi-project objective status")
        .json()
        .await
        .expect("objective response");
    let objective_id = created_objective_id(&created);
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let objective = store
        .objective(objective_id)
        .expect("read objective")
        .expect("created objective");
    assert_eq!(objective.project_ids, vec![PROJECT_ID, second_id]);
    let leaves = store.list_leaves(objective_id).expect("objective leaves");
    assert_eq!(leaves.len(), 3, "two leaves plus dependent join");
    assert!(leaves.iter().any(|leaf| leaf.id.ends_with("-join")));
    for leaf in leaves.iter().filter(|leaf| !leaf.id.ends_with("-join")) {
        let project_id = leaf.project_id.as_deref().expect("project-bound leaf");
        let execution = leaf.execution.as_ref().expect("leaf execution spec");
        assert_eq!(
            execution.objective,
            format!("Inspect exact project {project_id} for project-local evidence."),
        );
        assert!(execution
            .execution_prompt
            .contains(&format!("Inspect only exact project {project_id}.")));
        assert!(execution
            .execution_prompt
            .contains("Do not compare sibling projects or require sibling files"));
        assert!(execution
            .execution_prompt
            .contains("Cross-project comparison is reserved for the dependent synthesis leaf"));
    }
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_objective_approval_schedules_and_can_cancel_before_first_claim() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture");
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({
            "contract": contract,
            "envelope": mutation_envelope("attach-objective-control")
        }))
        .send()
        .await
        .expect("attach")
        .error_for_status()
        .expect("attach status");

    let created: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objective-control",
            &format!("arda objective {PROJECT_ID} create a disposable acceptance artifact"),
        ))
        .send()
        .await
        .expect("objective")
        .error_for_status()
        .expect("objective status")
        .json()
        .await
        .expect("objective response");
    let objective_id = created_objective_id(&created).to_owned();
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let task_id = store.list_leaves(&objective_id).expect("objective leaves")[0]
        .id
        .clone();

    for (message_id, command) in [
        (
            "discord-objective-control-revise",
            format!(
                "arda revise-objective {task_id} {objective_id} create only a disposable acceptance artifact --reason bound the acceptance scope"
            ),
        ),
        (
            "discord-objective-control-approve",
            format!(
                "arda approve-objective {task_id} {objective_id} approve bounded disposable acceptance"
            ),
        ),
    ] {
        let response = client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&gateway_message(message_id, &command))
            .send()
            .await
            .expect("objective control");
        if command.starts_with("arda revise-objective ") {
            assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
            assert!(response.text().await.unwrap().contains("persisted execution plan"));
        } else {
            response.error_for_status().expect("objective control status");
        }
    }

    let approved = store
        .objective(&objective_id)
        .expect("read approved objective")
        .expect("approved objective");
    assert_eq!(approved.state, ObjectiveState::Approved);
    assert_eq!(approved.revision, 1);
    assert_eq!(approved.text, "create a disposable acceptance artifact");

    let cancelled: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "discord-objective-control-cancel",
            &format!("arda cancel-task {task_id} {objective_id} acceptance scenario cleanup"),
        ))
        .send()
        .await
        .expect("cancel unclaimed task")
        .error_for_status()
        .expect("cancel unclaimed task status")
        .json()
        .await
        .unwrap();
    assert_eq!(
        cancelled["summary"],
        format!("Cancelled resident objective {objective_id}: acceptance scenario cleanup")
    );
    let terminal = store
        .objective(&objective_id)
        .expect("read cancelled objective")
        .expect("cancelled objective");
    assert_eq!(terminal.state, ObjectiveState::Cancelled);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn gateway_research_command_persists_question_without_creating_commitment() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();
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
    assert_eq!(duplicate.status(), 200);
    assert_eq!(duplicate.json::<Value>().await.unwrap(), response);
    let mut changed = body.clone();
    changed["event"]["text"] = "arda research a different question".into();
    assert!(!client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&changed)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn authenticated_gateway_approval_cancel_and_resume_use_canonical_runs() {
    let root = TempDir::new().expect("root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = gateway_client();

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
    let client = gateway_client();
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
    let objective_id = created_objective_id(&objective);
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .expect("resident objective store");
    let resident = store
        .objective(objective_id)
        .expect("read resident objective")
        .expect("resident objective");
    assert_eq!(resident.text, "finish the operator bridge");
    assert_eq!(resident.state, ObjectiveState::PendingApproval);
    assert_eq!(resident.project_ids, vec![PROJECT_ID.to_owned()]);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
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
        .is_some_and(|summary| summary.contains("finish the operator bridge")));

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
    let client = gateway_client();

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
    let client = gateway_client();
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
    let client = gateway_client();
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
    let client = gateway_client();
    let mut body = gateway_message("discord-denied-1", "arda capture denied");
    body["operator"]["authenticated"] = json!(false);

    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .expect("denied");
    assert_eq!(denied.status(), 403);

    let mut forged = gateway_message("discord-forged-1", "arda objectives");
    forged["operator"]["operator_id"] = json!("forged-operator");
    forged["event"]["user_id"] = json!("forged-operator");
    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&forged)
        .send()
        .await
        .expect("forged identity");
    assert_eq!(denied.status(), 403);

    let mut stale = gateway_message("discord-stale-auth", "arda objectives");
    stale["operator"]["authenticated_at"] = json!("1970-01-01T00:00:00Z");
    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&stale)
        .send()
        .await
        .expect("stale authentication");
    assert_eq!(denied.status(), 403);

    let mut malformed = gateway_message("discord-malformed-auth", "arda objectives");
    malformed["operator"]["authenticated_at"] = json!("not-a-timestamp");
    let denied = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&malformed)
        .send()
        .await
        .expect("malformed authentication");
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
    let client = gateway_client();

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
