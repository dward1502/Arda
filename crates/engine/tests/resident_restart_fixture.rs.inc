// Included by context_bootstrap_restart: reuse its isolated harness/provider fixture.
#[tokio::test]
async fn resident_receipt_crash_child() {
    use arda_engine::objectives::*;
    use sha2::{Digest, Sha256};
    let Some(root) = std::env::var_os("ARDA_RESIDENT_CRASH_TEST_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    install_fake_hermes(&root);
    let (bound, _, _) = start_at(&root).await;
    let client = reqwest::Client::new();
    let project: arda_core::project_contract::ProjectContract =
        serde_json::from_value(contract()).unwrap();
    let digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&project).unwrap())
    );
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract":project,"envelope":envelope("resident-attach")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let store = ObjectiveStore::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let mut resident_approval = envelope("resident-execution");
    resident_approval["approval"]["ledger_writes"] =
        json!(["data/arda/objectives.sqlite3", "data/runs"]);
    store
        .create_authenticated_objective(
            NewObjective {
                id: "resident-crash".into(),
                source_id: "test:resident-crash".into(),
                idempotency_key: "resident-create".into(),
                operator_id: "operator-0".into(),
                text: OBJECTIVE.into(),
                priority: 0,
                projects: vec![ProjectAuthority {
                    project_id: PROJECT_ID.into(),
                    contract_digest: digest,
                }],
                leaves: vec![NewLeaf {
                    id: "resident-leaf".into(),
                    project_id: Some(PROJECT_ID.into()),
                    workspace_root: root.to_string_lossy().into_owned(),
                    authority: "execute_with_approval".into(),
                    dependencies: vec![],
                    execution: Some(LeafExecutionSpec {
                        objective: OBJECTIVE.into(),
                        execution_prompt: OBJECTIVE.into(),
                        verification_prompt: OBJECTIVE.into(),
                        review_prompt: OBJECTIVE.into(),
                        approval_envelope: resident_approval,
                        objective_plan_receipt: format!("sha256:{}", "a".repeat(64)),
                    }),
                }],
            },
            100,
        )
        .unwrap();
    store
        .apply_control(
            "resident-crash",
            ControlAction::Approve { revision: 1 },
            "resident-approve",
            "operator-0",
            110,
        )
        .unwrap();
    let mut claim = store
        .claim_runnable("crashed-worker", 120, 100, 1)
        .unwrap()
        .remove(0);
    if std::env::var("ARDA_RESIDENT_CRASH_POINT").as_deref() == Ok("retry-limit") {
        for now in [220, 320, 420, 520] {
            claim = store
                .claim_runnable("crashed-worker", now, 100, 1)
                .unwrap()
                .remove(0);
        }
        assert_eq!(claim.attempt, MAX_OBJECTIVE_ATTEMPTS);
    }
    fs::write(
        root.join("initial-claim.json"),
        serde_json::to_vec(&claim).unwrap(),
    )
    .unwrap();
    let bound = match std::env::var("ARDA_RESIDENT_CRASH_POINT")
        .ok()
        .and_then(|point| point.strip_prefix("after-").map(str::to_owned))
    {
        Some(stage) => resident_stage_crash_proxy(bound, stage).await,
        None => bound,
    };
    let executor = WorkbenchLeafExecution::with_adapter(&root,
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(&root, format!("http://{bound}")).unwrap());
    if std::env::var("ARDA_RESIDENT_CRASH_POINT").as_deref() == Ok("before-context") {
        std::process::exit(74);
    }
    let result = executor.execute(claim).await.unwrap();
    fs::write(
        root.join("completed-before-projection.json"),
        serde_json::to_vec(&result.receipts).unwrap(),
    )
    .unwrap();
    // Exact failpoint: canonical provider/memory receipts exist, but no
    // ObjectiveRuntime result persistence has run. Exit without destructors.
    std::process::exit(73);
}

#[tokio::test]
async fn resident_receipts_survive_process_exit_before_objective_projection() {
    let root = TempDir::new().unwrap();
    let output = resident_crash_process(&root, "before-projection").await;
    assert_eq!(
        output.status.code(),
        Some(73),
        "child failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    verify_resident_projection_recovery(root).await;
}

#[tokio::test]
async fn retry_limit_missing_run_fails_without_replanning_or_dispatch() {
    use arda_engine::objectives::*;
    let root = TempDir::new().unwrap();
    let output = resident_crash_process(&root, "retry-limit").await;
    assert_eq!(output.status.code(), Some(73));
    let count = fs::read(root.path().join("context-worker-count")).unwrap();
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let connection =
        rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let run_id: String = connection
        .query_row(
            "SELECT execution_run_id FROM leaves WHERE id = 'resident-leaf'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);
    let run_path = root.path().join("data/runs").join(&run_id);
    fs::rename(&run_path, root.path().join("withheld-fixture-run")).unwrap();
    let (bound, shutdown, handle) = start(&root).await;
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{bound}")).unwrap());
    let mut runtime = ObjectiveRuntime::new(store, executor, "missing-run-recovery", 1, 100);
    assert!(runtime.run_round(621).await.is_err());
    assert_eq!(
        runtime
            .store()
            .objective("resident-crash")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Failed
    );
    assert!(!run_path.exists());
    assert_eq!(
        fs::read(root.path().join("context-worker-count")).unwrap(),
        count
    );
    assert!(runtime.run_round(722).await.unwrap().is_empty());
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn retry_limit_harness_outage_does_not_discard_completed_evidence() {
    use arda_engine::objectives::*;
    let root = TempDir::new().unwrap();
    let output = resident_crash_process(&root, "retry-limit").await;
    assert_eq!(output.status.code(), Some(73));
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), "http://127.0.0.1:0").unwrap());
    let mut runtime = ObjectiveRuntime::new(store, executor, "offline-recovery", 1, 100);
    assert!(runtime.run_round(621).await.is_err());
    assert_eq!(
        runtime
            .store()
            .objective("resident-crash")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Running
    );
    let (bound, shutdown, handle) = start(&root).await;
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{bound}")).unwrap());
    let mut runtime =
        ObjectiveRuntime::new(runtime.store().clone(), executor, "online-recovery", 1, 100);
    assert_eq!(runtime.run_round(722).await.unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(root.path().join("context-worker-count"))
            .unwrap()
            .trim()
            .parse::<usize>()
            .unwrap(),
        3
    );
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn resident_reconciles_completed_run_at_retry_limit_without_dispatch() {
    use arda_engine::objectives::*;
    let root = TempDir::new().unwrap();
    let output = resident_crash_process(&root, "retry-limit").await;
    assert_eq!(
        output.status.code(),
        Some(73),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let count = fs::read(root.path().join("context-worker-count")).unwrap();
    let outcomes = fs::read(
        root.path()
            .join("data/vaire/context_outcome_receipts.jsonl"),
    )
    .unwrap();
    let uses = fs::read(root.path().join("data/vaire/context_use_receipts.jsonl")).unwrap();
    let (bound, shutdown, handle) = start(&root).await;
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{bound}")).unwrap());
    let mut runtime = ObjectiveRuntime::new(store, executor, "retry-limit-recovery", 1, 100);
    assert!(runtime.run_round(619).await.unwrap().is_empty());
    assert_eq!(runtime.run_round(621).await.unwrap().len(), 1);
    assert_eq!(
        runtime
            .store()
            .objective("resident-crash")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
    assert_eq!(
        runtime
            .store()
            .leaf("resident-leaf")
            .unwrap()
            .unwrap()
            .attempt,
        MAX_OBJECTIVE_ATTEMPTS + 1
    );
    assert!(runtime.run_round(1000).await.unwrap().is_empty());
    assert_eq!(
        fs::read(root.path().join("context-worker-count")).unwrap(),
        count
    );
    assert_eq!(
        fs::read(
            root.path()
                .join("data/vaire/context_outcome_receipts.jsonl")
        )
        .unwrap(),
        outcomes
    );
    assert_eq!(
        fs::read(root.path().join("data/vaire/context_use_receipts.jsonl")).unwrap(),
        uses
    );
    shutdown.notify_waiters();
    handle.await.unwrap();
}

// Terminate only the isolated child, after the real Harness has committed a
// stage and returned success, before Workbench receives that response. This
// introduces no production failpoint and cannot kill an installed service.
async fn resident_stage_crash_proxy(
    upstream: std::net::SocketAddr,
    stage: String,
) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound = listener.local_addr().unwrap();
    let client = reqwest::Client::new();
    let router = axum::Router::new().fallback(move |request: axum::extract::Request| {
        let stage = stage.clone();
        let client = client.clone();
        async move {
            let (parts, body) = request.into_parts();
            let path = parts.uri.to_string();
            let bytes = axum::body::to_bytes(body, 8 * 1024 * 1024).await.unwrap();
            let response = client
                .request(parts.method, format!("http://{upstream}{path}"))
                .header("content-type", "application/json")
                .body(bytes)
                .send()
                .await
                .unwrap();
            let status = response.status();
            let body = response.bytes().await.unwrap();
            if status.is_success() && path.ends_with(&format!("/nodes/{stage}/execute-provider")) {
                std::process::exit(75);
            }
            axum::http::Response::builder()
                .status(status)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body))
                .unwrap()
        }
    });
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    bound
}

#[tokio::test]
async fn resident_recovers_intermediate_stage_process_interruptions() {
    use arda_engine::objectives::*;
    for (index, stage) in ["execute", "verify", "review"].into_iter().enumerate() {
        let root = TempDir::new().unwrap();
        let output = resident_crash_process(&root, &format!("after-{stage}")).await;
        assert_eq!(
            output.status.code(),
            Some(75),
            "{stage}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let initial: ClaimedLeaf =
            serde_json::from_slice(&fs::read(root.path().join("initial-claim.json")).unwrap())
                .unwrap();
        let run_id = initial.execution_run_id.as_deref().unwrap();
        let run_root = root.path().join("data/runs").join(run_id);
        assert_eq!(
            fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
            (index + 1).to_string()
        );
        assert!(!run_root.join("execution-receipts/close.json").exists());
        let receipts: Vec<_> = ["execute", "verify", "review"][..=index]
            .iter()
            .map(|name| {
                let path = run_root.join(format!("execution-receipts/{name}.json"));
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        let connection =
            rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
        let read_binding = || {
            connection
                .query_row(
                    "SELECT assembly_json FROM resident_context_bindings WHERE run_id = ?1",
                    [run_id],
                    |row| row.get::<_, String>(0),
                )
                .unwrap()
        };
        let original = read_binding();
        let uses_path = root.path().join("data/vaire/context_use_receipts.jsonl");
        let original_uses = fs::read(&uses_path).unwrap();
        let (bound, shutdown, handle) = start(&root).await;
        let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
        let executor = WorkbenchLeafExecution::with_adapter(root.path(),
            arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{bound}")).unwrap());
        let mut runtime =
            ObjectiveRuntime::new(store, executor, "intermediate-recovery", 1, 60_000);
        let recovered = runtime.run_round(221).await;
        assert!(recovered.is_ok(), "{stage}: {recovered:#?}");
        assert_eq!(recovered.unwrap().len(), 1);
        let leaf = runtime.store().leaf("resident-leaf").unwrap().unwrap();
        let recovered_run: String = connection
            .query_row(
                "SELECT execution_run_id FROM leaves WHERE id = 'resident-leaf'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(recovered_run, run_id);
        assert_eq!(leaf.attempt, 2);
        assert_eq!(
            runtime
                .store()
                .objective("resident-crash")
                .unwrap()
                .unwrap()
                .state,
            ObjectiveState::Completed
        );
        assert_eq!(read_binding(), original);
        for (path, bytes) in receipts {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert_eq!(
            fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
            "3"
        );
        let uses = fs::read(&uses_path).unwrap();
        assert!(uses.starts_with(&original_uses));
        let outcomes_path = root
            .path()
            .join("data/vaire/context_outcome_receipts.jsonl");
        let outcomes = fs::read(&outcomes_path).unwrap();
        assert_eq!(
            String::from_utf8(outcomes.clone()).unwrap().lines().count(),
            1
        );
        let events_path = run_root.join("events.jsonl");
        let events = fs::read(&events_path).unwrap();
        let close_path = run_root.join("execution-receipts/close.json");
        let close = fs::read(&close_path).unwrap();
        assert!(runtime.run_round(60_222).await.unwrap().is_empty());
        assert_eq!(fs::read(&events_path).unwrap(), events);
        assert_eq!(fs::read(&close_path).unwrap(), close);
        assert_eq!(fs::read(&uses_path).unwrap(), uses);
        assert_eq!(fs::read(&outcomes_path).unwrap(), outcomes);
        assert_eq!(
            fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
            "3"
        );
        shutdown.notify_waiters();
        handle.await.unwrap();
    }
}

async fn resident_crash_process(root: &TempDir, point: &str) -> std::process::Output {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "resident_receipt_crash_child", "--nocapture"])
        .env("ARDA_RESIDENT_CRASH_TEST_ROOT", root.path())
        .env("ARDA_RESIDENT_CRASH_POINT", point)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "child timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    child.wait_with_output().unwrap()
}

#[tokio::test]
async fn resident_recovers_process_exit_before_initial_context_binding() {
    use arda_engine::objectives::*;
    let root = TempDir::new().unwrap();
    let output = resident_crash_process(&root, "before-context").await;
    assert_eq!(
        output.status.code(),
        Some(74),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!root.path().join("context-worker-count").exists());
    assert!(!root
        .path()
        .join("data/vaire/context_use_receipts.jsonl")
        .exists());
    let initial: ClaimedLeaf =
        serde_json::from_slice(&fs::read(root.path().join("initial-claim.json")).unwrap()).unwrap();
    let (bound, shutdown, handle) = start(&root).await;
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{bound}")).unwrap());
    let mut runtime = ObjectiveRuntime::new(store, executor, "pre-bind-recovery", 1, 60_000);
    assert_eq!(runtime.run_round(221).await.unwrap().len(), 1);
    assert_eq!(
        runtime
            .store()
            .objective("resident-crash")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
    assert_eq!(
        runtime
            .store()
            .leaf("resident-leaf")
            .unwrap()
            .unwrap()
            .attempt,
        2
    );
    assert!(root
        .path()
        .join("data/runs")
        .join(initial.execution_run_id.unwrap())
        .join("execution-receipts/close.json")
        .exists());
    assert_eq!(
        fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
        "3"
    );
    assert!(runtime.run_round(500).await.unwrap().is_empty());
    shutdown.notify_waiters();
    handle.await.unwrap();
}

#[tokio::test]
async fn resident_unknown_context_binding_fails_closed() {
    use arda_engine::objectives::*;
    let root = TempDir::new().unwrap();
    let output = resident_crash_process(&root, "before-context").await;
    assert_eq!(output.status.code(), Some(74));
    let path = root.path().join("data/arda/objectives.sqlite3");
    // Model a migrated historical claim: absent evidence is not an unbound marker.
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("UPDATE leaves SET context_bound = NULL", [])
        .unwrap();
    let store = ObjectiveStore::open(path).unwrap();
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), "http://127.0.0.1:1").unwrap());
    let mut runtime = ObjectiveRuntime::new(store, executor, "unknown-binding", 1, 100);
    let error = runtime
        .run_round(221)
        .await
        .expect_err("unknown historical binding must remain unreconciled");
    assert!(
        error.to_string().contains("missing original context"),
        "{error:#}"
    );
    assert!(!root.path().join("context-worker-count").exists());
    assert!(!root
        .path()
        .join("data/vaire/context_use_receipts.jsonl")
        .exists());
}

async fn verify_resident_projection_recovery(root: TempDir) {
    use arda_engine::objectives::*;
    let initial: ClaimedLeaf =
        serde_json::from_slice(&fs::read(root.path().join("initial-claim.json")).unwrap()).unwrap();
    let expected: Vec<StageReceipt> = serde_json::from_slice(
        &fs::read(root.path().join("completed-before-projection.json")).unwrap(),
    )
    .unwrap();
    let worker_count = fs::read(root.path().join("context-worker-count")).unwrap();
    assert_eq!(
        String::from_utf8(worker_count.clone()).unwrap(),
        "3",
        "execute, verify and review must actually invoke the fixture provider"
    );
    let outcomes = fs::read(
        root.path()
            .join("data/vaire/context_outcome_receipts.jsonl"),
    )
    .unwrap();
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let claim = store
        .claim_runnable("recovered-worker", 221, 100, 1)
        .unwrap()
        .remove(0);
    assert_eq!(claim.execution_run_id, initial.execution_run_id);
    assert_eq!(claim.attempt, 2);
    let (bound, shutdown, handle) = start(&root).await;
    let executor = WorkbenchLeafExecution::with_adapter(root.path(),
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(root.path(), format!("http://{bound}")).unwrap());
    let use_path = root.path().join("data/vaire/context_use_receipts.jsonl");
    let use_bytes = fs::read(&use_path).unwrap();
    let connection =
        rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let run_id = claim.execution_run_id.as_deref().unwrap();
    let binding: (String, String) = connection
        .query_row(
            "SELECT request_digest, assembly_json FROM resident_context_bindings WHERE run_id = ?1",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    connection
        .execute(
            "DELETE FROM resident_context_bindings WHERE run_id = ?1",
            [run_id],
        )
        .unwrap();
    let error = executor
        .execute(claim.clone())
        .await
        .expect_err("lost bound snapshot must not be reconstructed");
    assert!(
        error.to_string().contains("missing original context"),
        "{error:#}"
    );
    assert_eq!(fs::read(&use_path).unwrap(), use_bytes);
    assert_eq!(
        fs::read(root.path().join("context-worker-count")).unwrap(),
        worker_count
    );
    connection.execute("INSERT INTO resident_context_bindings(run_id, request_digest, assembly_json) VALUES (?1, ?2, ?3)",
        rusqlite::params![run_id, binding.0, binding.1]).unwrap();
    let uses = String::from_utf8(use_bytes.clone()).unwrap();
    for stage in ["execute", "verify", "review"] {
        let retained = uses
            .lines()
            .filter(|line| {
                let value: Value = serde_json::from_str(line).unwrap();
                !value["consumer_id"]
                    .as_str()
                    .unwrap()
                    .ends_with(&format!(":{stage}"))
            })
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        assert_ne!(retained.as_bytes(), use_bytes);
        fs::write(&use_path, &retained).unwrap();
        let error = executor
            .execute(claim.clone())
            .await
            .expect_err("missing stage use receipt must block reconciliation");
        assert!(
            error.to_string().contains("durable stage context"),
            "{error:#}"
        );
        assert_eq!(fs::read(&use_path).unwrap(), retained.as_bytes());
        assert_eq!(
            fs::read(root.path().join("context-worker-count")).unwrap(),
            worker_count
        );
        assert_eq!(
            fs::read(
                root.path()
                    .join("data/vaire/context_outcome_receipts.jsonl")
            )
            .unwrap(),
            outcomes
        );
        // Restore only this isolated fixture ledger for the successful replay.
        fs::write(&use_path, &use_bytes).unwrap();
    }
    let recovered = executor.execute(claim.clone()).await.unwrap();
    assert_eq!(recovered.receipts, expected);
    assert_eq!(
        fs::read(root.path().join("context-worker-count")).unwrap(),
        worker_count
    );
    assert_eq!(
        fs::read(
            root.path()
                .join("data/vaire/context_outcome_receipts.jsonl")
        )
        .unwrap(),
        outcomes
    );
    for receipt in recovered.receipts {
        store
            .record_stage_receipt(
                &claim.leaf_id,
                "recovered-worker",
                claim.attempt,
                receipt,
                222,
            )
            .unwrap();
    }
    store
        .complete_objective_if_ready("resident-crash", &expected.last().unwrap().digest, 222)
        .unwrap();
    assert_eq!(
        store.objective("resident-crash").unwrap().unwrap().state,
        ObjectiveState::Completed
    );
    let mut runtime = ObjectiveRuntime::new(store, executor, "post-close-replay", 1, 100);
    assert!(runtime.run_round(500).await.unwrap().is_empty());
    assert_eq!(
        fs::read(root.path().join("context-worker-count")).unwrap(),
        worker_count
    );
    assert_eq!(
        fs::read(
            root.path()
                .join("data/vaire/context_outcome_receipts.jsonl")
        )
        .unwrap(),
        outcomes
    );
    shutdown.notify_waiters();
    handle.await.unwrap();
}
