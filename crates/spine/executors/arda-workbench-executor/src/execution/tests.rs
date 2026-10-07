use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const DEFAULT_PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub(super) fn bind_test_run(item: &ExplicitWorkbenchWorkItem, mut value: Value) -> Value {
    if value.get("run").is_some() {
        let nested = value["run"].take();
        value["run"] = bind_test_run(item, nested);
    }
    if value.get("graph").is_some() {
        value["graph"]["run_id"] = json!(item.run_id);
        value["graph"]["objective_id"] = json!(item.leaf_id);
        value["graph"]["provenance"] = json!({"project_contract_digest": item.project_contract_digest, "parent_receipts": [item.approval_envelope["approval"]["approval_id"], item.objective_plan_receipt]});
        if let Some(nodes) = value["graph"]["nodes"].as_array_mut() {
            for node in nodes {
                if node.get("kind").is_none() {
                    node["kind"] = node["id"].clone();
                }
            }
        }
    }
    value
}

async fn scripted_explicit_harness(
    item: &ExplicitWorkbenchWorkItem,
    responses: Vec<Option<(u16, String)>>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    scripted_harness(
        responses
            .into_iter()
            .map(|entry| {
                entry.map(|(status, body)| {
                    let body = if status == 200 {
                        bind_test_run(item, serde_json::from_str(&body).unwrap()).to_string()
                    } else {
                        body
                    };
                    (status, body)
                })
            })
            .collect(),
    )
    .await
}

pub(super) async fn scripted_harness(
    responses: Vec<Option<(u16, String)>>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let Ok(Ok((mut stream, _))) =
                tokio::time::timeout(std::time::Duration::from_secs(1), listener.accept()).await
            else {
                break;
            };
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = stream.read(&mut buffer).await.unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..headers_end + 4]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= headers_end + 4 + content_length {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request).into_owned();
            requests.push(request.clone());
            let Some((status, body)) = response else {
                break;
            };
            let body = if body.contains("__REQUEST_RECEIPT_DIGEST__") {
                let digest = request
                    .split_once("\r\n\r\n")
                    .and_then(|(_, body)| serde_json::from_str::<Value>(body).ok())
                    .and_then(|body| body["receipt_digest"].as_str().map(str::to_owned))
                    .expect("scripted response requested the submitted receipt digest");
                body.replace("__REQUEST_RECEIPT_DIGEST__", &digest)
            } else {
                body
            };
            let reason = if status == 200 { "OK" } else { "Not Found" };
            stream
                    .write_all(
                        format!(
                            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
        }
        requests
    });
    (format!("http://{address}"), server)
}

#[tokio::test]
async fn explicit_work_item_recovers_receipts_without_reading_or_writing_queue() {
    let dir = tempfile::tempdir().unwrap();
    let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
    let item = ExplicitWorkbenchWorkItem {
        objective_id: "objective-1".into(),
        read_only: false,
        leaf_id: "leaf-1".into(),
        run_id: "objective-1-leaf-1-attempt-1".into(),
        objective: "Inspect exact project authority".into(),
        execution_prompt: "Inspect only the bound project.".into(),
        verification_prompt: "Verify the inspection receipt.".into(),
        review_prompt: "Independently review the evidence.".into(),
        project_id: DEFAULT_PROJECT_ID.into(),
        project_contract_digest: digest('f'),
        workspace_root: dir.path().to_path_buf(),
        approval_envelope: json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-1",
                "approval_id": "operator-approval-1",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": "objective-1-leaf-1"
        }),
        objective_plan_receipt: digest('1'),
        dependency_receipts: Vec::new(),
        context_assembly: None,
    };
    let close_receipt = canonical_explicit_close_receipt(&item, &digest('d')).unwrap();
    persist_explicit_close_receipt(dir.path(), &item, &close_receipt).unwrap();
    for corrupt in [
        json!({}),
        json!({"graph": {"nodes": []}}),
        json!({"graph": {"nodes": [{"id": "execute"}]}}),
    ] {
        let (url, server) = scripted_harness(vec![Some((200, corrupt.to_string()))]).await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), url).unwrap();
        assert!(
            adapter
                .reconcile(&item, &runtime_admission::FixtureAdmission)
                .await
                .is_err(),
            "malformed successful GET must remain retryable"
        );
        assert_eq!(server.await.unwrap().len(), 1);
    }
    let (harness_url, server) = scripted_explicit_harness(&item, vec![Some((
            200,
            json!({
                "graph": {"nodes": [
                    {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
                    {"id": "execute", "state": "succeeded", "output_digest": digest('b')},
                    {"id": "verify", "state": "succeeded", "output_digest": digest('c')},
                    {"id": "review", "state": "succeeded", "output_digest": digest('d')},
                    {"id": "close", "state": "succeeded", "output_digest": close_receipt.receipt_digest}
                ]}
            })
            .to_string(),
        ))])
        .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();

    let outcome = adapter
        .execute(&item, &runtime_admission::FixtureAdmission)
        .await
        .unwrap();
    let requests = server.await.unwrap();

    assert_eq!(outcome.status, "succeeded");
    assert_eq!(outcome.receipts.len(), 4);
    assert_eq!(
        outcome.root_receipt_digest.as_deref(),
        Some(close_receipt.receipt_digest.as_str())
    );
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /v1/runs/objective-1-leaf-1-attempt-1 "));
    assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!dir
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
}

#[tokio::test]
async fn explicit_work_item_reconciles_scheduler_conflict_from_terminal_run() {
    let dir = tempfile::tempdir().unwrap();
    let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
    let initial = json!({
        "graph": {"nodes": [
            {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
            {"id": "execute", "state": "pending", "output_digest": null},
            {"id": "verify", "state": "pending", "output_digest": null},
            {"id": "review", "state": "pending", "output_digest": null},
            {"id": "close", "state": "pending", "output_digest": null}
        ]}
    });
    let item = ExplicitWorkbenchWorkItem {
        objective_id: "objective-conflict".into(),
        read_only: false,
        leaf_id: "leaf-conflict".into(),
        run_id: "objective-conflict-leaf-conflict-attempt-1".into(),
        objective: "Inspect exact project authority".into(),
        execution_prompt: "Inspect only the bound project.".into(),
        verification_prompt: "Verify the inspection receipt.".into(),
        review_prompt: "Independently review the evidence.".into(),
        project_id: DEFAULT_PROJECT_ID.into(),
        project_contract_digest: digest('f'),
        workspace_root: dir.path().to_path_buf(),
        approval_envelope: json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-conflict",
                "approval_id": "operator-approval-conflict",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": "objective-conflict-leaf-conflict"
        }),
        objective_plan_receipt: digest('1'),
        dependency_receipts: Vec::new(),
        context_assembly: None,
    };
    let close_receipt = canonical_explicit_close_receipt(&item, &digest('d')).unwrap();
    persist_explicit_close_receipt(dir.path(), &item, &close_receipt).unwrap();
    let terminal = json!({
        "graph": {"nodes": [
            {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
            {"id": "execute", "state": "succeeded", "output_digest": digest('b')},
            {"id": "verify", "state": "succeeded", "output_digest": digest('c')},
            {"id": "review", "state": "succeeded", "output_digest": digest('d')},
            {"id": "close", "state": "succeeded", "output_digest": close_receipt.receipt_digest}
        ]}
    });
    let (harness_url, server) = scripted_explicit_harness(&item, vec![
            Some((200, initial.to_string())),
            Some((
                409,
                json!({"code": "scheduler_not_admitted", "message": "not selected by deterministic scheduler"})
                    .to_string(),
            )),
            Some((200, terminal.to_string())),
        ])
        .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();

    let outcome = adapter
        .execute(&item, &runtime_admission::FixtureAdmission)
        .await
        .unwrap();
    let requests = server.await.unwrap();

    assert_eq!(outcome.status, "succeeded");
    assert_eq!(
        outcome.root_receipt_digest.as_deref(),
        Some(close_receipt.receipt_digest.as_str())
    );
    assert_eq!(requests.len(), 3);
    assert!(requests[1].contains("/nodes/execute/execute-provider "));
    assert!(requests[2].starts_with("GET /v1/runs/objective-conflict-leaf-conflict-attempt-1 "));

    let mut forged = close_receipt.clone();
    forged.recorded_at_unix_ms = forged.recorded_at_unix_ms.saturating_add(1);
    forged.receipt_digest = forged.computed_digest().unwrap();
    persist_explicit_close_receipt(dir.path(), &item, &forged).unwrap();
    let mut forged_terminal = terminal;
    forged_terminal["graph"]["nodes"][4]["output_digest"] =
        Value::String(forged.receipt_digest.clone());
    let error = validate_explicit_close_receipt(dir.path(), &item, &forged_terminal).unwrap_err();
    assert!(error.to_string().contains("recomputed canonical payload"));
}

#[tokio::test]
async fn non_scheduler_conflict_is_not_reconciled() {
    let (harness_url, server) = scripted_harness(vec![Some((
        409,
        json!({"code": "conflict", "message": "different canonical payload"}).to_string(),
    ))])
    .await;
    let response = reqwest::get(harness_url).await.unwrap();
    let error = require_scheduler_admission_conflict(response, "execute")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("different canonical payload"));
    server.await.unwrap();
}

#[tokio::test]
async fn conflicted_close_is_not_treated_as_scheduler_owned_completion() {
    let dir = tempfile::tempdir().unwrap();
    let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
    let run = |close_state: &str, close_digest: Option<String>| {
        json!({
            "graph": {"run_id": "objective-close-conflict-leaf-close-conflict-attempt-1", "objective_id": "leaf-close-conflict", "provenance": {"project_contract_digest": digest('f'), "parent_receipts": ["operator-approval-close-conflict", digest('1')]}, "nodes": [
                {"id": "approval", "kind": "approval", "state": "succeeded", "output_digest": digest('a')},
                {"id": "execute", "kind": "execute", "state": "succeeded", "output_digest": digest('b')},
                {"id": "verify", "kind": "verify", "state": "succeeded", "output_digest": digest('c')},
                {"id": "review", "kind": "review", "state": "succeeded", "output_digest": digest('d')},
                {"id": "close", "kind": "close", "state": close_state, "output_digest": close_digest}
            ]}
        })
    };
    let initial = run("pending", None);
    let (harness_url, server) = scripted_harness(vec![
        Some((200, initial.to_string())),
        Some((409, json!({"code": "conflict"}).to_string())),
    ])
    .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();
    let item = ExplicitWorkbenchWorkItem {
        objective_id: "objective-close-conflict".into(),
        read_only: false,
        leaf_id: "leaf-close-conflict".into(),
        run_id: "objective-close-conflict-leaf-close-conflict-attempt-1".into(),
        objective: "Inspect exact project authority".into(),
        execution_prompt: "Inspect only the bound project.".into(),
        verification_prompt: "Verify the inspection receipt.".into(),
        review_prompt: "Independently review the evidence.".into(),
        project_id: DEFAULT_PROJECT_ID.into(),
        project_contract_digest: digest('f'),
        workspace_root: dir.path().to_path_buf(),
        approval_envelope: json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-close-conflict",
                "approval_id": "operator-approval-close-conflict",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": "objective-close-conflict-leaf-close-conflict"
        }),
        objective_plan_receipt: digest('1'),
        dependency_receipts: Vec::new(),
        context_assembly: None,
    };
    let error = adapter
        .execute(&item, &runtime_admission::FixtureAdmission)
        .await
        .unwrap_err();
    let requests = server.await.unwrap();

    assert!(error.to_string().contains("returned 409"));
    assert_eq!(requests.len(), 2);
    let staged_receipt = dir
        .path()
        .join("data/runs")
        .join(&item.run_id)
        .join("execution-receipts/close.json");
    assert!(staged_receipt.exists());
    let staged: CanonicalHermesExecutionReceipt =
        serde_json::from_slice(&std::fs::read(staged_receipt).unwrap()).unwrap();
    assert!(staged.has_valid_digest().unwrap());
}

#[test]
fn terminal_close_rejects_locally_valid_noncanonical_payload() {
    let dir = tempfile::tempdir().unwrap();
    let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
    let item = ExplicitWorkbenchWorkItem {
        objective_id: "objective-forged-close".into(),
        read_only: false,
        leaf_id: "leaf-forged-close".into(),
        run_id: "objective-forged-close-leaf-forged-close-attempt-1".into(),
        objective: "Inspect exact project authority".into(),
        execution_prompt: "Inspect only the bound project.".into(),
        verification_prompt: "Verify the inspection receipt.".into(),
        review_prompt: "Independently review the evidence.".into(),
        project_id: DEFAULT_PROJECT_ID.into(),
        project_contract_digest: digest('f'),
        workspace_root: dir.path().to_path_buf(),
        approval_envelope: json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-forged-close",
                "approval_id": "operator-approval-forged-close",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": "objective-forged-close-leaf-forged-close"
        }),
        objective_plan_receipt: digest('1'),
        dependency_receipts: Vec::new(),
        context_assembly: None,
    };
    let mut forged = canonical_explicit_close_receipt(&item, &digest('d')).unwrap();
    forged.summary = "forged but self-consistent close".into();
    forged.receipt_digest.clear();
    forged.receipt_digest = forged.computed_digest().unwrap();
    persist_explicit_close_receipt(dir.path(), &item, &forged).unwrap();
    let run = json!({"graph": {"nodes": [
        {"id": "review", "state": "succeeded", "output_digest": digest('d')},
        {"id": "close", "state": "succeeded", "output_digest": forged.receipt_digest}
    ]}});
    assert_eq!(
        node_output_digest(&run, "close"),
        Some(forged.receipt_digest.as_str())
    );

    let error = validate_explicit_close_receipt(dir.path(), &item, &run).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not equal the recomputed canonical payload"),
        "unexpected validation error: {error:#}"
    );
}

#[tokio::test]
async fn explicit_work_item_executes_all_stages_without_queue_continuations() {
    let dir = tempfile::tempdir().unwrap();
    let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
    let run = |completed: &[&str]| {
        let nodes = ["approval", "execute", "verify", "review", "close"]
                .into_iter()
                .map(|stage| {
                    let state = if completed.contains(&stage) { "succeeded" } else { "pending" };
                    json!({
                        "id": stage,
                        "kind": stage,
                        "state": state,
                        "output_digest": if state == "succeeded" { Some(digest(stage.chars().next().unwrap())) } else { None }
                    })
                })
                .collect::<Vec<_>>();
        json!({"graph": {"run_id": "objective-2-leaf-2-attempt-1", "objective_id": "leaf-2", "provenance": {"project_contract_digest": digest('f'), "parent_receipts": ["operator-approval-2", digest('1')]}, "nodes": nodes}})
    };
    let provider = |completed: &[&str], stage: &str| {
        json!({
            "receipt": {
                "status": "succeeded",
                "receipt_digest": digest(stage.chars().next().unwrap()),
                "summary": format!("{stage} completed")
            },
            "run": run(completed)
        })
    };
    let (harness_url, server) = scripted_harness(vec![
        Some((404, "{}".into())),
        Some((200, run(&[]).to_string())),
        Some((200, run(&["approval"]).to_string())),
        Some((
            200,
            provider(&["approval", "execute"], "execute").to_string(),
        )),
        Some((
            200,
            provider(&["approval", "execute", "verify"], "verify").to_string(),
        )),
        Some((
            200,
            provider(&["approval", "execute", "verify", "review"], "review").to_string(),
        )),
        Some((
            200,
            run(&["approval", "execute", "verify", "review", "close"])
                .to_string()
                .replace(&digest('c'), "__REQUEST_RECEIPT_DIGEST__"),
        )),
    ])
    .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();
    let item = ExplicitWorkbenchWorkItem {
        objective_id: "objective-2".into(),
        read_only: false,
        leaf_id: "leaf-2".into(),
        run_id: "objective-2-leaf-2-attempt-1".into(),
        objective: "Inspect exact project authority".into(),
        execution_prompt: "Inspect only the bound project.".into(),
        verification_prompt: "Verify the inspection receipt.".into(),
        review_prompt: "Independently review the evidence.".into(),
        project_id: DEFAULT_PROJECT_ID.into(),
        project_contract_digest: digest('f'),
        workspace_root: dir.path().to_path_buf(),
        approval_envelope: json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-2",
                "approval_id": "operator-approval-2",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": "objective-2-leaf-2"
        }),
        objective_plan_receipt: digest('1'),
        dependency_receipts: Vec::new(),
        context_assembly: None,
    };

    let outcome = adapter
        .execute(&item, &runtime_admission::FixtureAdmission)
        .await
        .unwrap();
    let requests = server.await.unwrap();

    assert_eq!(outcome.status, "succeeded");
    assert_eq!(requests.len(), 7);
    assert!(requests[1].starts_with("POST /v1/runs/plan "));
    assert!(requests[3].contains("/nodes/execute/execute-provider "));
    assert!(requests[4].contains("/nodes/verify/execute-provider "));
    assert!(requests[5].contains("/nodes/review/execute-provider "));
    assert!(requests[6].contains("/nodes/close/complete "));
    assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
}

#[test]
fn critic_projection_uses_exact_execute_and_verify_receipts() {
    let root = tempfile::tempdir().expect("create run root");
    let receipt_root = root.path().join("data/runs/run-review/execution-receipts");
    std::fs::create_dir_all(&receipt_root).expect("create receipt directory");
    let make_receipt = |node_id: &str,
                        authority_binding_digest: &str,
                        provider: &str,
                        model: &str,
                        summary: &str,
                        test_evidence: Vec<CanonicalHermesTestEvidence>| {
        let mut receipt = CanonicalHermesExecutionReceipt {
            schema_version: "arda.execution-receipt.v3".into(),
            inference_routes: None,
            receipt_digest: String::new(),
            authority_binding_digest: authority_binding_digest.into(),
            run_id: "run-review".into(),
            node_id: node_id.into(),
            idempotency_key: format!("run-review:{node_id}"),
            status: json!("succeeded"),
            summary: summary.into(),
            tool_evidence: vec![],
            test_evidence,
            artifacts: vec![],
            usage: CanonicalHermesUsage {
                api_calls: 1,
                input_tokens: 1,
                output_tokens: 1,
                total_tokens: 2,
                estimated_cost_usd: 0.0,
                cost_measurement: json!("provider_reported"),
                completed: true,
                failed: false,
                provider: Some(provider.into()),
                model: Some(model.into()),
            },
            adapter: "hermes".into(),
            adapter_version: "1".into(),
            project_contract_digest: "sha256:project".into(),
            parent_receipts: vec![],
            context_capsule_id: None,
            context_capsule_digest: None,
            context_use_receipt_ref: None,
            context_handoff: None,
            recorded_at_unix_ms: 1,
        };
        receipt.receipt_digest = receipt.computed_digest().expect("digest receipt");
        receipt
    };
    let execute = make_receipt(
        "execute",
        "sha256:execute-authority",
        "provider-a",
        "model-a",
        "Project-local inspection found bounded authority evidence.",
        vec![],
    );
    let verify = make_receipt(
        "verify",
        "sha256:verify-authority",
        "provider-b",
        "model-b",
        "Verification passed.",
        vec![CanonicalHermesTestEvidence {
            check_id: "tests".into(),
            command: "cargo test".into(),
            status: "passed".into(),
            exit_code: 0,
            output_digest: "sha256:test".into(),
        }],
    );
    std::fs::write(
        receipt_root.join("execute.json"),
        serde_json::to_vec(&execute).expect("serialize execute receipt"),
    )
    .expect("write execute receipt");
    std::fs::write(
        receipt_root.join("verify.json"),
        serde_json::to_vec(&verify).expect("serialize verify receipt"),
    )
    .expect("write verify receipt");
    let run = json!({
        "graph": {"nodes": [
            {"id": "execute", "output_digest": execute.receipt_digest},
            {"id": "verify", "output_digest": verify.receipt_digest}
        ]},
        "review": {"provider_receipt": null, "tests": []}
    });

    let projection = pre_review_receipt_projection(root.path(), "run-review", &run)
        .expect("project exact receipts");
    assert_eq!(
        projection["execute_receipt"]["usage"]["provider"],
        "provider-a"
    );
    assert_eq!(projection["execute_receipt"]["usage"]["model"], "model-a");
    assert_eq!(
        projection["execute_receipt"]["summary"],
        "Project-local inspection found bounded authority evidence."
    );
    assert_eq!(
        projection["execute_receipt"]["authority_binding_digest"],
        "sha256:execute-authority"
    );
    assert_eq!(
        projection["verify_receipt"]["receipt_digest"],
        verify.receipt_digest
    );
    assert_eq!(
        projection["verify_receipt"]["test_evidence"][0]["status"],
        "passed"
    );
    assert_eq!(projection["verify_output_digest"], verify.receipt_digest);

    let mut tampered = serde_json::to_value(&execute).expect("encode receipt for tamper");
    tampered["summary"] = json!("forged summary retained the old receipt digest");
    std::fs::write(
        receipt_root.join("execute.json"),
        serde_json::to_vec(&tampered).expect("serialize tampered receipt"),
    )
    .expect("write tampered receipt");
    let error = pre_review_receipt_projection(root.path(), "run-review", &run)
        .expect_err("tampered receipt must fail canonical digest validation");
    assert!(error.to_string().contains("invalid canonical digest"));
}
