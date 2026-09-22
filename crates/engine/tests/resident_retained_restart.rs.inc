// Host-qualified vertical: real subprocess, Resident/Workbench/Harness and installed Hermes.
#[path = "fixtures/installed_policy.rs"]
mod resident_installed_policy;

struct ResidentTestKeeper(std::process::Child);
// Fixture-only: the real keeper intentionally survives daemon restarts, but it
// must not survive forced termination of the test process that owns it.
fn fixture_parent_death(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    let parent = unsafe { libc::getpid() };
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
            }
            Ok(())
        });
    }
}

#[test]
fn fixture_keeper_owner_child() {
    let Some(path) = std::env::var_os("ARDA_FIXTURE_KEEPER_PID") else {
        return;
    };
    let mut command = std::process::Command::new("/usr/bin/sleep");
    command.arg("60");
    fixture_parent_death(&mut command);
    let mut child = command.spawn().unwrap();
    let path = std::path::PathBuf::from(path);
    let pending = path.with_extension("pending");
    fs::write(&pending, child.id().to_string()).unwrap();
    fs::rename(pending, path).unwrap();
    child.wait().unwrap();
}

#[tokio::test]
async fn fixture_keeper_dies_when_owner_is_killed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("keeper.pid");
    let mut owner = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fixture_keeper_owner_child", "--nocapture"])
        .env("ARDA_FIXTURE_KEEPER_PID", &path)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !path.exists() {
            assert!(owner.try_wait().unwrap().is_none());
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let pid = fs::read_to_string(path).unwrap();
    owner.kill().await.unwrap();
    owner.wait().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match fs::read_to_string(format!("/proc/{pid}/stat")) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Ok(stat) if stat.split_once(") ").unwrap().1.starts_with('Z') => break,
                Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
                Err(error) => panic!("{error}"),
            }
        }
    })
    .await
    .unwrap();
}
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
    println!("installed-retained-without-live-path-completed");
}

#[tokio::test]
#[ignore = "requires installed Hermes, local Manwe, user/mount namespaces"]
async fn resident_retained_installed_recovery_uses_distinct_config() {
    retained_installed_config_process(false).await;
}

#[tokio::test]
#[ignore = "gateway synthetic evidence through installed Hermes retained recovery"]
async fn gateway_admitted_evidence_survives_installed_retained_recovery() {
    retained_installed_config_process(true).await;
}

async fn retained_installed_config_process(admitted: bool) {
    let config_root = tempfile::tempdir_in("/var/tmp").unwrap();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "resident_recovers_retained_installed_execution_without_live_path",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_remove("ARDA_HERMES_ADAPTER_CONFIG")
            .env("ARDA_TEST_INSTALLED_DISTINCT_CONFIG", "1")
            .env(
                "ARDA_TEST_INSTALLED_ADMITTED",
                if admitted { "1" } else { "0" },
            )
            .env(
                "ARDA_HERMES_RETAINED_ADAPTER_CONFIG",
                config_root.path().join("retained.toml"),
            )
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("installed-retained-without-live-path-completed"));
    if admitted {
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains("installed-gateway-admission-completed"));
    }
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
    if std::env::var_os("ARDA_TEST_INSTALLED_DISTINCT_CONFIG").is_some() {
        let retained = std::env::var_os("ARDA_HERMES_RETAINED_ADAPTER_CONFIG").unwrap();
        let config = fs::read_to_string(adapter).unwrap().replace(
            "adapter_version = \"1\"",
            "adapter_version = \"installed-retained-distinct\"",
        );
        fs::write(retained, &config).unwrap();
        fs::write(
            root.join("config/adapters/hermes-workbench.toml"),
            if installed_admission::enabled() {
                &config
            } else {
                "deliberately invalid ordinary adapter config"
            },
        )
        .unwrap();
    } else {
        fs::copy(adapter, root.join("config/adapters/hermes-workbench.toml")).unwrap();
    }
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
    let mut keeper_command = std::process::Command::new(keeper_bin);
    keeper_command
        .arg(&durable)
        .arg(&runtime)
        .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
        .arg("--runtime-policy")
        .arg(&policy);
    fixture_parent_death(&mut keeper_command);
    let mut keeper = ResidentTestKeeper(keeper_command.spawn().unwrap());
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
    let retained_contents = installed_admission::enabled()
        .then(|| retained_tree_contents(&root.join("original-project")));
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
    if let Some(contents) = retained_contents {
        assert_eq!(
            retained_tree_contents(&root.join("original-project")),
            contents,
            "installed verification/review must preserve this fixture's retained contents"
        );
    }
    if std::env::var_os("ARDA_TEST_INSTALLED_DISTINCT_CONFIG").is_some() {
        for stage in ["execute", "verify", "review"] {
            let receipt: serde_json::Value = serde_json::from_slice(
                &fs::read(run_root.join(format!("execution-receipts/{stage}.json"))).unwrap(),
            )
            .unwrap();
            assert_eq!(receipt["adapter_version"], "installed-retained-distinct");
        }
    }
    assert_eq!(
        resident
            .store()
            .objective(&initial.objective_id)
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
    let leaf = resident.store().leaf(&initial.leaf_id).unwrap().unwrap();
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
            "SELECT execution_run_id FROM leaves WHERE id=?1",
            [&initial.leaf_id],
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
    let transcript =
        rusqlite::Connection::open(std::path::Path::new(&sessions).join("state.db")).unwrap();
    if installed_admission::enabled() {
        installed_admission::assert_stage_inputs(&root, &transcript, run_id);
    }
    let mut user_messages = transcript
        .prepare("SELECT session_id, content FROM messages WHERE role='user'")
        .unwrap();
    let review_sessions: Vec<String> = user_messages
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .filter_map(|(id, prompt)| {
            let (_, context) = prompt.split_once("Canonical node context follows:\n")?;
            let context = serde_json::Deserializer::from_str(context)
                .into_iter::<serde_json::Value>()
                .next()?
                .ok()?;
            (context["run_id"] == run_id && context["node"]["kind"] == "review").then_some(id)
        })
        .collect();
    assert_eq!(
        review_sessions.len(),
        1,
        "exactly one canonical review session"
    );
    let mut calls = transcript.prepare("SELECT tool_calls FROM messages WHERE session_id=?1 AND role='assistant' AND tool_calls IS NOT NULL").unwrap();
    let mut successful_paths = std::collections::BTreeSet::new();
    for calls in calls
        .query_map([&review_sessions[0]], |row| row.get::<_, String>(0))
        .unwrap()
    {
        let calls: serde_json::Value = serde_json::from_str(&calls.unwrap()).unwrap();
        for call in calls.as_array().unwrap() {
            assert!(
                matches!(
                    call["function"]["name"].as_str(),
                    Some("read_file" | "search_files")
                ),
                "read-only review requested a non-read tool: {}",
                call["function"]["name"]
            );
            if call["function"]["name"] != "read_file" {
                continue;
            }
            let args = call["function"]["arguments"].as_str().unwrap();
            let args: serde_json::Value = serde_json::from_str(args).unwrap();
            let path = std::path::PathBuf::from(args["path"].as_str().unwrap());
            let path = if path.is_absolute() {
                path
            } else {
                workspace.join(path)
            };
            let result: String = transcript.query_row("SELECT content FROM messages WHERE session_id=?1 AND role='tool' AND tool_call_id=?2 AND tool_name='read_file'", rusqlite::params![review_sessions[0], call["id"].as_str().unwrap()], |row| row.get(0)).unwrap();
            let result: serde_json::Value = serde_json::from_str(&result).unwrap();
            if result.get("error").is_none()
                && result["content"]
                    .as_str()
                    .is_some_and(|text| text.contains("governed-next-action"))
            {
                successful_paths.insert(path);
            }
        }
    }
    for file in ["context-bootstrap-input.txt", "verify-context-bootstrap.py"] {
        assert!(
            successful_paths.contains(&workspace.join(file)),
            "review must successfully read retained {file}"
        );
    }
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
    if installed_admission::enabled() {
        installed_admission::assert_completed_replay(&root, bound).await;
    }
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
    if installed_admission::enabled() {
        println!("installed-gateway-admission-completed");
    }
    shutdown.notify_waiters();
    harness.await.unwrap();
}

fn retained_tree_contents(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        dir: &std::path::Path,
        entries: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            let mut bytes = Vec::new();
            if kind.is_symlink() {
                use std::os::unix::ffi::OsStrExt;
                bytes.push(b'L');
                bytes.extend_from_slice(fs::read_link(&path).unwrap().as_os_str().as_bytes());
            } else if kind.is_dir() {
                bytes.push(b'D');
                visit(root, &path, entries);
            } else {
                assert!(kind.is_file(), "unexpected retained fixture file type");
                bytes.push(b'F');
                bytes.extend(fs::read(&path).unwrap());
            }
            entries.insert(path.strip_prefix(root).unwrap().to_owned(), bytes);
        }
    }
    let mut entries = std::collections::BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}
