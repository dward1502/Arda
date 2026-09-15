// Host-qualified vertical: real subprocess, Resident/Workbench/Harness and installed Hermes.
#[path = "fixtures/installed_policy.rs"]
mod resident_installed_policy;

struct ResidentTestKeeper(std::process::Child);
impl Drop for ResidentTestKeeper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
#[ignore = "requires installed Hermes, local Manwe, user/mount namespaces"]
async fn resident_recovers_retained_installed_execution_after_process_exit() {
    retained_installed_restart_case(true, false).await;
}

#[tokio::test]
#[ignore = "requires installed Hermes, local Manwe, user/mount namespaces"]
async fn resident_recovers_retained_installed_execution_without_live_path() {
    retained_installed_restart_case(false, false).await;
}

#[tokio::test]
#[ignore = "requires installed Hermes, local Manwe, user/mount namespaces"]
async fn resident_recovers_retained_running_verify_with_missing_and_replaced_workspace() {
    retained_installed_restart_case(false, true).await;
    retained_installed_restart_case(true, true).await;
}

async fn retained_installed_restart_case(replacement: bool, interrupt_verify: bool) {
    use arda_engine::objectives::keeper_client::KeeperClient;
    use arda_engine::objectives::*;
    assert!(
        std::env::var_os("ARDA_HERMES_ADAPTER_CONFIG").is_none(),
        "unset ARDA_HERMES_ADAPTER_CONFIG for this isolated fixture"
    );
    let parent = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = parent.path().join("arda");
    let workspace = root.join("project");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join("context-bootstrap-input.txt"),
        "governed-next-action\n",
    )
    .unwrap();
    fs::write(workspace.join("verify-context-bootstrap.py"), "from pathlib import Path\nassert Path('context-bootstrap-input.txt').read_text() == 'governed-next-action\\n'\nprint('context-bootstrap-check: ok')\n").unwrap();
    let policy_root = parent.path().join("runtime-policy");
    fs::create_dir(&policy_root).unwrap();
    let policy = resident_installed_policy::write(&policy_root);
    let adapter = resident_installed_policy::adapter_config(&policy_root);
    fs::create_dir_all(root.join("config/adapters")).unwrap();
    fs::copy(adapter, root.join("config/adapters/hermes-workbench.toml")).unwrap();
    let durable = parent.path().join("keeper");
    let runtime_parent = tempfile::tempdir_in("/dev/shm").unwrap();
    let runtime = runtime_parent.path().join("owner");
    fs::create_dir(&durable).unwrap();
    fs::create_dir(&runtime).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
    let keeper_bin = env!("CARGO_BIN_EXE_arda-snapshot-keeper");
    assert!(std::process::Command::new(keeper_bin)
        .arg("--initialize")
        .arg(&durable)
        .status()
        .unwrap()
        .success());
    let socket = runtime.join("keeper.sock");
    let mut keeper = ResidentTestKeeper(
        std::process::Command::new(keeper_bin)
            .arg(&durable)
            .arg(&runtime)
            .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
            .arg("--runtime-policy")
            .arg(&policy)
            .spawn()
            .unwrap(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if std::os::unix::net::UnixStream::connect(&socket).is_ok() {
                break;
            }
            assert!(
                keeper.0.try_wait().unwrap().is_none(),
                "keeper exited before readiness"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "resident_receipt_crash_child", "--nocapture"])
            .env("ARDA_RESIDENT_CRASH_TEST_ROOT", &root)
            .env(
                "ARDA_RESIDENT_CRASH_POINT",
                if interrupt_verify {
                    "after-running-verify"
                } else {
                    "after-execute"
                },
            )
            .env("ARDA_RETAINED_KEEPER_SOCKET", &socket)
            .env("ARDA_RETAINED_TEST_WORKSPACE", &workspace)
            .env_remove("ARDA_HERMES_ADAPTER_CONFIG")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    if output.status.code() != Some(75) {
        let owner_db = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
        let mut allocations = owner_db
            .prepare("SELECT source FROM runtime_allocations WHERE state='allocated'")
            .unwrap();
        for source in allocations
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .flatten()
        {
            if let Ok(session_db) =
                rusqlite::Connection::open(std::path::Path::new(&source).join("state.db"))
            {
                if let Ok(reply) = session_db.query_row("SELECT content FROM messages WHERE role='assistant' AND content IS NOT NULL ORDER BY id DESC LIMIT 1", [], |r| r.get::<_,String>(0)) {
                    println!("Isolated fixture final reply: {}", reply.chars().take(1600).collect::<String>());
                }
            }
        }
    }
    assert_eq!(
        output.status.code(),
        Some(75),
        "child did not reach execute receipt barrier: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let initial: ClaimedLeaf =
        serde_json::from_slice(&fs::read(root.join("initial-claim.json")).unwrap()).unwrap();
    let run_id = initial.execution_run_id.as_deref().unwrap();
    let run_root = root.join("data/runs").join(run_id);
    let execute_path = run_root.join("execution-receipts/execute.json");
    let execute = fs::read(&execute_path).unwrap();
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let original_context: String = db
        .query_row(
            "SELECT assembly_json FROM resident_context_bindings WHERE run_id=?1",
            [run_id],
            |r| r.get(0),
        )
        .unwrap();
    let original_snapshot: String = db
        .query_row(
            "SELECT capability_json FROM retained_workspace_snapshots WHERE run_id=?1",
            [run_id],
            |r| r.get(0),
        )
        .unwrap();
    let keeper_db = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    let sessions: String = keeper_db
        .query_row(
            "SELECT source FROM runtime_allocations WHERE run=?1",
            [run_id],
            |r| r.get(0),
        )
        .unwrap();
    let session_count = || -> i64 {
        rusqlite::Connection::open(std::path::Path::new(&sessions).join("state.db"))
            .unwrap()
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .unwrap()
    };
    if interrupt_verify {
        let graph: arda_core::run_graph::RunGraph =
            serde_json::from_slice(&fs::read(run_root.join("checkpoint.json")).unwrap()).unwrap();
        assert!(matches!(
            graph
                .nodes
                .iter()
                .find(|node| node.id.as_str() == "verify")
                .unwrap()
                .state,
            arda_core::run_graph::NodeState::Running
        ));
        assert!((1..=2).contains(&session_count()));
    } else {
        assert_eq!(
            session_count(),
            1,
            "only execute may have invoked Hermes before the crash"
        );
    }
    fs::rename(&workspace, root.join("original-project")).unwrap();
    if replacement {
        fs::create_dir(&workspace).unwrap();
    }
    let uses_path = root.join("data/vaire/context_use_receipts.jsonl");
    let original_uses = fs::read(&uses_path).unwrap();
    // Use the actual wall clock: synthetic fixture timestamps are not valid keeper leases.
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        while chrono::Utc::now().timestamp_millis() <= initial.lease_expires_ms {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let (bound, shutdown, harness) = start_at(&root).await;
    let store = ObjectiveStore::open(root.join("data/arda/objectives.sqlite3"))
        .unwrap()
        .with_snapshot_admission(Arc::new(KeeperClient::new(socket.clone())));
    let executor = WorkbenchLeafExecution::with_adapter(&root,
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(&root, format!("http://{bound}")).unwrap());
    let mut resident =
        ObjectiveRuntime::new(store, executor.clone(), "retained-recovery", 1, 120_000);
    let recovered = resident
        .run_round(chrono::Utc::now().timestamp_millis())
        .await;
    let recovered = match recovered {
        Ok(recovered) => recovered,
        Err(error) => {
            // Preserve isolated evidence, not live state, for provider failures.
            let evidence = parent.keep();
            panic!(
                "retained recovery failed; isolated evidence at {}: {error:#}",
                evidence.display()
            );
        }
    };
    assert_eq!(recovered.len(), 1);
    assert_eq!(
        resident
            .store()
            .objective("resident-crash")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
    let leaf = resident.store().leaf("resident-leaf").unwrap().unwrap();
    assert_eq!(leaf.attempt, 2);
    let committed_generation: i64 = db
        .query_row(
            "SELECT committed_generation FROM retained_workspace_snapshots WHERE run_id=?1",
            [run_id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(committed_generation > initial.attempt);
    let recovered_run: String = db
        .query_row(
            "SELECT execution_run_id FROM leaves WHERE id='resident-leaf'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recovered_run, run_id);
    assert_eq!(
        db.query_row(
            "SELECT assembly_json FROM resident_context_bindings WHERE run_id=?1",
            [run_id],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        original_context
    );
    assert_eq!(
        db.query_row(
            "SELECT capability_json FROM retained_workspace_snapshots WHERE run_id=?1",
            [run_id],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        original_snapshot
    );
    assert_eq!(fs::read(&execute_path).unwrap(), execute);
    if interrupt_verify {
        let events: Vec<serde_json::Value> = fs::read_to_string(run_root.join("events.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let starts = |node: &str| {
            events
                .iter()
                .filter(|event| {
                    event["node_id"] == node
                        && event["kind"]["type"] == "node_transition"
                        && event["kind"]["state"] == "running"
                })
                .count()
        };
        assert_eq!(starts("execute"), 1, "completed execution must not replay");
        assert_eq!(
            starts("verify"),
            2,
            "orphaned verify gets one bounded retry"
        );
        assert!(
            (3..=4).contains(&session_count()),
            "only an interrupted verify plus verify/review may add sessions"
        );
    } else {
        assert_eq!(
            session_count(),
            3,
            "only verify and review may dispatch after recovery"
        );
    }
    let review_reads: i64 =
        rusqlite::Connection::open(std::path::Path::new(&sessions).join("state.db"))
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE session_id = \
         (SELECT id FROM sessions ORDER BY rowid DESC LIMIT 1) \
         AND role = 'tool' AND tool_name = 'read_file'",
                [],
                |row| row.get(0),
            )
            .unwrap();
    assert!(review_reads > 0, "review must actually read retained files");
    eprintln!("retained review observed {review_reads} read_file results");
    if replacement {
        assert!(
            fs::read_dir(&workspace).unwrap().next().is_none(),
            "replacement workspace must stay untouched"
        );
    } else {
        assert!(
            !workspace.exists(),
            "recovery must not recreate the live project path"
        );
    }
    let events = fs::read(run_root.join("events.jsonl")).unwrap();
    let close = fs::read(run_root.join("execution-receipts/close.json")).unwrap();
    let uses = fs::read(&uses_path).unwrap();
    assert!(uses.starts_with(&original_uses));
    let outcomes_path = root.join("data/vaire/context_outcome_receipts.jsonl");
    let outcomes = fs::read(&outcomes_path).unwrap();
    assert_eq!(
        String::from_utf8(outcomes.clone()).unwrap().lines().count(),
        1
    );
    // Exercise receipt reconciliation explicitly after terminal capability release.
    let sessions_before_reconciliation = session_count();
    assert!(executor.reconcile(initial.clone()).await.unwrap().is_some());
    assert!(resident
        .run_round(chrono::Utc::now().timestamp_millis())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(fs::read(run_root.join("events.jsonl")).unwrap(), events);
    assert_eq!(
        fs::read(run_root.join("execution-receipts/close.json")).unwrap(),
        close
    );
    assert_eq!(fs::read(&uses_path).unwrap(), uses);
    assert_eq!(fs::read(&outcomes_path).unwrap(), outcomes);
    assert_eq!(session_count(), sessions_before_reconciliation);
    shutdown.notify_waiters();
    harness.await.unwrap();
}
