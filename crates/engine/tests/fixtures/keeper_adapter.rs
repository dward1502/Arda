use super::*;
#[path = "keeper_active_rebind.rs"]
mod active_rebind;
#[path = "installed_policy.rs"]
mod installed_policy;
#[path = "keeper_prepare_rollback.rs"]
mod prepare_rollback;
#[path = "keeper_undelivered_commit.rs"]
mod undelivered_commit;
use arda_engine::objectives::{
    keeper_client::KeeperClient, ControlAction, NewLeaf, NewObjective, ObjectiveStore,
    ProjectAuthority, SnapshotAdmission,
};
use std::{
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

struct Keeper(Child);
impl Drop for Keeper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_with_policy(durable: &Path, runtime: &Path, policy: Option<&Path>) -> Keeper {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"));
    command
        .arg(durable)
        .arg(runtime)
        .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
        .stdin(Stdio::null());
    if let Some(policy) = policy {
        command.arg("--runtime-policy").arg(policy);
    }
    let mut process = Keeper(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(
            process.0.try_wait().unwrap().is_none(),
            "keeper failed to start"
        );
        if std::os::unix::net::UnixStream::connect(runtime.join("keeper.sock")).is_ok() {
            return process;
        }
        assert!(Instant::now() < deadline, "keeper startup timeout");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[tokio::test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
async fn keeper_store_admission_chat_export_reopen_and_durable_release() {
    keeper_adapter_case(false).await;
}
#[tokio::test]
#[ignore = "requires installed Hermes, namespaces, and the approved live local Manwe route"]
async fn installed_keeper_production_adapter_reopen_and_receipt_validation() {
    keeper_adapter_case(true).await;
}
async fn keeper_adapter_case(installed: bool) {
    let parent = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = parent.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let check_source = b"from pathlib import Path\nassert Path('retained.txt').read_bytes() == b'retained-adapter\\n'\n";
    if installed {
        fs::write(root.join("check.py"), check_source).unwrap();
    }
    let durable = parent.path().join("owner");
    fs::create_dir(&durable).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    let scratch = tempfile::Builder::new()
        .prefix("arda-keeper-")
        .tempdir_in("/dev/shm")
        .unwrap();
    fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let policy = installed.then(|| installed_policy::write(parent.path()));
    let config = if installed {
        installed_policy::adapter_config(parent.path())
    } else {
        let executable = write_fake_hermes(&root);
        let config = write_config(parent.path());
        let raw = fs::read_to_string(&config).unwrap().replace(
            "executable = \"hermes\"",
            &format!("executable = {:?}", executable.display().to_string()),
        );
        fs::write(&config, raw).unwrap();
        config
    };
    assert!(Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .arg("--initialize")
        .arg(&durable)
        .status()
        .unwrap()
        .success());
    let mut keeper = start_with_policy(&durable, scratch.path(), policy.as_deref());
    let db = parent.path().join("objectives.sqlite3");
    let client = || Arc::new(KeeperClient::new(scratch.path().join("keeper.sock")));
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client());
    let now = chrono::Utc::now().timestamp_millis();
    store
        .create_authenticated_objective(
            NewObjective {
                id: "retained-fixture".into(),
                source_id: "fixture".into(),
                idempotency_key: "fixture".into(),
                operator_id: "operator".into(),
                text: "real keeper".into(),
                priority: 0,
                projects: vec![ProjectAuthority {
                    project_id: "fixture".into(),
                    contract_digest: "sha256:fixture".into(),
                    authority: "operator_test".into(),
                    checks: vec!["build".into(), "lint".into()],
                }],
                leaves: vec![NewLeaf {
                    id: "leaf".into(),
                    project_id: Some("fixture".into()),
                    workspace_root: root.to_str().unwrap().into(),
                    authority: if installed {
                        "execute_with_approval"
                    } else {
                        "read_only"
                    }
                    .into(),
                    dependencies: vec![],
                    execution: None,
                }],
            },
            now,
        )
        .unwrap();
    store
        .apply_control(
            "retained-fixture",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            now,
        )
        .unwrap();
    let claim = store
        .claim_runnable("fixture", now, if installed { 180000 } else { 60000 }, 1)
        .unwrap()
        .remove(0);
    let run = claim.execution_run_id.unwrap();
    let binding = store.retained_execution(&run, now).unwrap().unwrap();
    let snapshot = binding.snapshot.clone();
    drop(store);
    let original = parent.path().join("original");
    fs::rename(&root, &original).unwrap();
    fs::create_dir(&root).unwrap();
    let mut saved_receipt = None;
    for _ in 0..2 {
        let reopened = ObjectiveStore::open(&db)
            .unwrap()
            .with_snapshot_admission(client());
        let binding = reopened
            .retained_execution(&run, chrono::Utc::now().timestamp_millis())
            .unwrap()
            .unwrap();
        assert!(binding.snapshot == snapshot);
        let adapter = HermesAdapter::load_retained(
            &config,
            &root,
            &if installed {
                BTreeMap::new()
            } else {
                host_environment(&root, "success")
            },
            binding,
        )
        .unwrap();
        let mut work = task(if installed { 120000 } else { 3000 });
        work.run_id = RunId::new(run.clone()).unwrap();
        if installed {
            work.check_commands.insert(
                "python-smoke".into(),
                format!("/usr/bin/python3 {}", root.join("check.py").display()),
            );
            work.instructions = "Create only retained.txt by running printf 'retained-adapter\\n' > retained.txt in the terminal. Run sha256sum retained.txt for its real artifact digest. Run the exact python-smoke value in check_commands in a separate terminal call with project_root as workdir (the approved check.py already exists; do not modify it). Do not use inline Python or heredocs: headless dangerous-command approvals remain denied. Return the required JSON result with empty tool_evidence and test_evidence; Arda derives them from the transcript. Include retained.txt and its real sha256 digest in artifacts.".into();
        }
        if let Some(receipt) = saved_receipt.as_ref() {
            adapter
                .validate_stored_receipt_authority(&work, receipt)
                .unwrap();
        } else {
            let execution = adapter.execute(&work, AdapterCancellation::default()).await;
            if installed && execution.is_err() {
                if let Err(arda_engine::adapters::HermesAdapterError::InvalidResult(message)) =
                    &execution
                {
                    println!(
                        "Missing exact declared check evidence: {}",
                        message
                            == "check python-smoke has no actual terminal result in Hermes export"
                    );
                }
                let journal = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
                let source: String = journal
                    .query_row(
                        "SELECT source FROM runtime_allocations WHERE run=?1",
                        [&run],
                        |r| r.get(0),
                    )
                    .unwrap();
                let state =
                    rusqlite::Connection::open(Path::new(&source).join("state.db")).unwrap();
                let rows = state
                    .prepare("SELECT model,billing_provider,model_config FROM sessions")
                    .unwrap()
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, Option<String>>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, Option<String>>(2)?,
                        ))
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                for (model, billing, config) in rows {
                    let parsed: serde_json::Value =
                        serde_json::from_str(config.as_deref().unwrap_or("null")).unwrap();
                    println!("Session provenance metadata: model={model:?}, billing={billing:?}, recorded_provider={:?}",parsed.get("provider").and_then(|v|v.as_str()));
                }
                let mut statement=state.prepare("SELECT role,tool_calls,content FROM messages WHERE tool_calls IS NOT NULL OR role='tool' ORDER BY id").unwrap();
                let diagnostics = statement
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, Option<String>>(2)?,
                        ))
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                for (role, calls, content) in diagnostics {
                    println!(
                        "Synthetic-task tool diagnostic: role={role}, calls={}, result={}",
                        calls.unwrap_or_default(),
                        if role == "tool" {
                            content
                                .unwrap_or_default()
                                .chars()
                                .take(800)
                                .collect::<String>()
                        } else {
                            String::new()
                        }
                    );
                }
                let evidence = parent.keep();
                panic!(
                    "installed provider execution failed; private fixture evidence retained at {}",
                    evidence.display()
                );
            }
            let receipt = execution.unwrap();
            assert_eq!(
                receipt.status,
                HermesReceiptStatus::Succeeded,
                "{}",
                receipt.summary
            );
            if installed {
                assert!(!receipt.tool_evidence.is_empty());
                assert!(!receipt.test_evidence.is_empty());
                assert_eq!(
                    fs::read(original.join("retained.txt")).unwrap(),
                    b"retained-adapter\n"
                );
                assert_eq!(fs::read(original.join("check.py")).unwrap(), check_source);
                assert_eq!(receipt.artifacts.len(), 1);
                println!("Genuine production adapter emitted receipt with tool/test evidence and verified retained artifact");
                saved_receipt = Some(receipt);
            }
        }
        if !installed {
            assert!(original.join("capture.json").is_file());
            assert!(original.join("transcript.json").is_file());
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    }
    let reopened = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client());
    reopened
        .apply_control(
            "retained-fixture",
            ControlAction::Cancel,
            "cancel",
            "operator",
            chrono::Utc::now().timestamp_millis(),
        )
        .unwrap();
    assert!(reopened
        .claim_runnable("fixture", chrono::Utc::now().timestamp_millis(), 60000, 1)
        .unwrap()
        .is_empty());
    assert!(!Path::new(&snapshot.endpoint).exists());
    // A second, still-held admission must never be reconstructed on restart.
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::metadata(&original).unwrap();
    let identity = serde_json::to_string(&(
        2u32,
        &original,
        &original,
        Some((metadata.dev(), metadata.ino())),
        format!(
            "{:x}",
            Sha256::digest(fs::read("/proc/self/mountinfo").unwrap())
        ),
    ))
    .unwrap();
    let lost = client().prepare("lost", &original, &identity).unwrap();
    client()
        .commit(
            &lost,
            "lost",
            1,
            "fixture",
            chrono::Utc::now().timestamp_millis() + 60000,
        )
        .unwrap();
    keeper.0.kill().unwrap();
    keeper.0.wait().unwrap();
    // Release ACKs survive owner restart without consulting current paths.
    fs::remove_file(scratch.path().join("keeper.sock")).unwrap();
    let _restarted = start_with_policy(&durable, scratch.path(), policy.as_deref());
    client().release(&snapshot, &run).unwrap();
    assert!(client().prepare("lost", &original, &identity).is_err());
    assert!(client()
        .commit(
            &lost,
            "lost",
            1,
            "fixture",
            chrono::Utc::now().timestamp_millis() + 60000
        )
        .is_err());
    assert!(client().release(&lost, "lost").is_err());
    let journal = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    let lost_state: String = journal
        .query_row("SELECT state FROM snapshots WHERE run='lost'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(lost_state, "lost");
    let state: String = journal
        .query_row("SELECT state FROM snapshots WHERE run=?1", [&run], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(state, "released");
}
