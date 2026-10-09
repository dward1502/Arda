use super::*;

#[tokio::test]
async fn retained_transport_bound_rejects_before_advice_history_or_start() {
    use arda_engine::objectives::{
        ControlAction, NewLeaf, NewObjective, ObjectiveStore, ProjectAuthority, RetainedSnapshot,
        SnapshotAdmission,
    };
    use std::{path::Path, sync::Arc};
    struct Keeper;
    impl SnapshotAdmission for Keeper {
        fn prepare(&self, _: &str, _: &Path, _: &str) -> anyhow::Result<RetainedSnapshot> {
            Ok(RetainedSnapshot {
                endpoint: "/nonexistent/bounded-fixture.sock".into(),
                capability: "fixture".into(),
                manifest_digest: "a".repeat(64),
            })
        }
        fn commit(
            &self,
            _: &RetainedSnapshot,
            _: &str,
            _: i64,
            _: &str,
            _: i64,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn release(&self, _: &RetainedSnapshot, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }
    let root = TempDir::new().unwrap();
    write_file_only_hermes_config(&root);
    let config = root.path().join("config/adapters/hermes-workbench.toml");
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("max_prompt_bytes = 32768", "max_prompt_bytes = 131072"),
    )
    .unwrap();
    let (bound, shutdown, server) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    let objectives = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .unwrap()
        .with_snapshot_admission(Arc::new(Keeper));
    let now = chrono::Utc::now().timestamp_millis();
    objectives
        .create_authenticated_objective(
            NewObjective {
                id: "bounded".into(),
                source_id: "bounded".into(),
                idempotency_key: "bounded".into(),
                operator_id: "operator".into(),
                text: "Inspect source".into(),
                priority: 1,
                projects: vec![ProjectAuthority {
                    project_id: PROJECT_ID.into(),
                    contract_digest: "sha256:fixture".into(),
                    authority: "operator_test".into(),
                    checks: vec!["test".into()],
                }],
                leaves: vec![NewLeaf {
                    id: "leaf".into(),
                    project_id: Some(PROJECT_ID.into()),
                    workspace_root: root.path().to_str().unwrap().into(),
                    authority: "read_only".into(),
                    dependencies: vec![],
                    execution: None,
                }],
            },
            now,
        )
        .unwrap();
    objectives
        .apply_control(
            "bounded",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            now,
        )
        .unwrap();
    let claim = objectives
        .claim_runnable("worker", now, 300_000, 1)
        .unwrap()
        .remove(0);
    let run_id = claim.execution_run_id.unwrap();
    let mut graph = provider_review_graph(&run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({"project_id":PROJECT_ID,"graph":graph,"envelope":envelope("plan-bounded")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let events = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&events).unwrap();
    let lease = objectives
        .retained_execution(&run_id, now)
        .unwrap()
        .unwrap()
        .lease;
    let response = client.post(format!("http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"))
        .json(&json!({"envelope":envelope("execute-bounded"),"objective":"x".repeat(70_000),"expected_retained_lease":lease}))
        .send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(
        !root
            .path()
            .join(format!("data/runs/{run_id}/reasoning/review.json"))
            .exists(),
        "transport-invalid request pinned advice: {status} {body}"
    );
    assert!(!root
        .path()
        .join("data/oracle/verdict_history.jsonl")
        .exists());
    assert_eq!(
        before,
        fs::read(events).unwrap(),
        "rejection consumed a start"
    );
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert!(body.contains("bounded preflight"), "{body}");
    shutdown.notify_waiters();
    server.await.unwrap();
}

#[tokio::test]
async fn cached_advice_and_outage_survive_harness_restart_without_reevaluation() {
    for unavailable in [false, true] {
        let root = TempDir::new().unwrap();
        write_file_only_hermes_config(&root);
        if unavailable {
            fs::create_dir_all(root.path().join("data")).unwrap();
            fs::write(root.path().join("data/oracle"), "unavailable fixture").unwrap();
        }
        let (bound, shutdown, server) = start_harness(&root).await;
        let client = reqwest::Client::new();
        attach(&client, bound).await;
        let mut graph = provider_review_graph("run-advice-restart");
        graph["nodes"][0]["state"] = json!("ready");
        graph["nodes"][0]["retry"]["max_attempts"] = json!(2);
        graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
        client.post(format!("http://{bound}/v1/runs/plan"))
            .json(&json!({"project_id":PROJECT_ID,"graph":graph,"envelope":envelope("plan-advice-restart")}))
            .send().await.unwrap().error_for_status().unwrap();
        let body = json!({"envelope":envelope("execute-advice-restart"),"objective":"Inspect authorized source evidence"});
        let response = client
            .post(format!(
                "http://{bound}/v1/runs/run-advice-restart/nodes/review/execute-provider"
            ))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(response
            .text()
            .await
            .unwrap()
            .contains("omitted its session header"));
        let path = root
            .path()
            .join("data/runs/run-advice-restart/reasoning/review.json");
        let advice = fs::read(&path).unwrap();
        let parsed: Value = serde_json::from_slice(&advice).unwrap();
        assert_eq!(
            parsed["outcome"],
            if unavailable {
                "unavailable"
            } else {
                "conditional"
            }
        );
        let history = root.path().join("data/oracle/verdict_history.jsonl");
        let before = fs::read(&history).ok();
        shutdown.notify_waiters();
        server.await.unwrap();
        if unavailable {
            fs::remove_file(root.path().join("data/oracle")).unwrap();
        }
        let (bound, shutdown, server) = start_harness(&root).await;
        let response = client
            .post(format!(
                "http://{bound}/v1/runs/run-advice-restart/nodes/review/execute-provider"
            ))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(response
            .text()
            .await
            .unwrap()
            .contains("omitted its session header"));
        assert_eq!(advice, fs::read(path).unwrap());
        assert_eq!(before, fs::read(history).ok());
        let events = fs::read_to_string(
            root.path()
                .join("data/runs/run-advice-restart/events.jsonl"),
        )
        .unwrap();
        let starts = events
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|event| event["kind"]["state"] == "running")
            .count();
        assert_eq!(
            starts, 2,
            "restart must reach a second authorized fixture dispatch"
        );
        shutdown.notify_waiters();
        server.await.unwrap();
    }
}

#[tokio::test]
async fn rejected_preflight_does_not_pin_reasoning_or_write_oracle_history() {
    let root = TempDir::new().unwrap();
    write_file_only_hermes_config(&root);
    let (bound, shutdown, server) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    let mut graph = provider_review_graph("run-advice-preflight");
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client.post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({"project_id": PROJECT_ID, "graph": graph, "envelope": envelope("plan-advice-preflight")}))
        .send().await.unwrap().error_for_status().unwrap();
    let url = format!("http://{bound}/v1/runs/run-advice-preflight/nodes/review/execute-provider");
    let response = client.post(&url).json(&json!({"envelope":envelope("execute-advice-preflight"),"objective":"x".repeat(100_000)})).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert!(response.text().await.unwrap().contains("bounded preflight"));
    assert!(
        !root
            .path()
            .join("data/runs/run-advice-preflight/reasoning/review.json")
            .exists(),
        "rejected request pinned advice"
    );
    assert!(
        !root
            .path()
            .join("data/oracle/verdict_history.jsonl")
            .exists(),
        "rejected request evaluated Mandos"
    );
    let response = client.post(&url).json(&json!({"envelope":envelope("execute-advice-corrected"),"objective":"Inspect authorized sources"})).send().await.unwrap();
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("omitted its session header"),
        "corrected request must reach fixture worker"
    );
    shutdown.notify_waiters();
    server.await.unwrap();
}

#[tokio::test]
async fn legacy_verify_receipt_reconstructs_original_instructions() {
    use arda_core::run_graph::{NodeKind, RunNode};
    let root = TempDir::new().unwrap();
    write_file_only_hermes_config(&root);
    let (bound, shutdown, server) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    let run_id = "run-legacy-verify";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["kind"] = json!("verify");
    graph["nodes"][0]["authority"] = json!("verify");
    graph["nodes"][0]["worker"] = Value::Null;
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:execute"]);
    let response: Value = client.post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({"project_id":PROJECT_ID,"graph":graph,"envelope":envelope("plan-legacy-verify")}))
        .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let digest = response["graph"]["provenance"]["project_contract_digest"]
        .as_str()
        .unwrap();
    let node: RunNode = serde_json::from_value(response["graph"]["nodes"][0].clone()).unwrap();
    assert_eq!(node.kind, NodeKind::Verify);
    let objective = "Verify the declared project check";
    // Exact pre-extraction instructions, independent of the production helper.
    let task = HermesNodeTask {
        run_id: arda_core::run_graph::RunId::new(run_id).unwrap(), node,
        objective: objective.into(),
        instructions: "Work only inside the attached project root. Do not commit. Execute every declared check exactly as printed before any optional exploratory command, and make no changes outside the objective. In test_evidence, reference the terminal tool call that ran the exact declared command; never reference an exploratory command. If a declared check succeeds, do not substitute ls, pwd, or inspection output for its evidence. Declared checks: test: cargo test -p arda-core".into(),
        checks: vec!["test".into()],
        check_commands: [("test".into(), "cargo test -p arda-core".into())].into(),
        project_contract_digest: digest.into(), context_assembly: None,
    };
    let mut receipt =
        stored_review_receipt(run_id, digest, vec!["receipt:execute".into()], objective);
    receipt.authority_binding_digest = task.authority_binding_digest().unwrap();
    receipt.summary = "Pre-change fixture: verification completed".into();
    receipt.receipt_digest = receipt.computed_digest().unwrap();
    write_stored_review_receipt(&root, &receipt);
    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({"envelope":envelope("replay-legacy-verify"),"objective":objective}))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    assert!(!root
        .path()
        .join("data/oracle/verdict_history.jsonl")
        .exists());
    shutdown.notify_waiters();
    server.await.unwrap();
}
