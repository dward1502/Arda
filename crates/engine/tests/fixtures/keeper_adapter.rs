use super::*;
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
fn start(durable: &Path, runtime: &Path) -> Keeper {
    let mut process = Keeper(
        Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
            .arg(durable)
            .arg(runtime)
            .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
            .stdin(Stdio::null())
            .spawn()
            .unwrap(),
    );
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
    let parent = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = parent.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let durable = parent.path().join("owner");
    fs::create_dir(&durable).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    let scratch = tempfile::Builder::new()
        .prefix("arda-keeper-")
        .tempdir_in("/dev/shm")
        .unwrap();
    fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let executable = write_fake_hermes(&root);
    let config = write_config(parent.path());
    let raw = fs::read_to_string(&config).unwrap().replace(
        "executable = \"hermes\"",
        &format!("executable = {:?}", executable.display().to_string()),
    );
    fs::write(&config, raw).unwrap();
    assert!(Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .arg("--initialize")
        .arg(&durable)
        .status()
        .unwrap()
        .success());
    let mut keeper = start(&durable, scratch.path());
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
                }],
                leaves: vec![NewLeaf {
                    id: "leaf".into(),
                    project_id: Some("fixture".into()),
                    workspace_root: root.to_str().unwrap().into(),
                    authority: "read_only".into(),
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
        .claim_runnable("fixture", now, 60000, 1)
        .unwrap()
        .remove(0);
    let run = claim.execution_run_id.unwrap();
    let binding = store.retained_execution(&run, now).unwrap().unwrap();
    let snapshot = binding.snapshot.clone();
    drop(store);
    let original = parent.path().join("original");
    fs::rename(&root, &original).unwrap();
    fs::create_dir(&root).unwrap();
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
            &host_environment(&root, "success"),
            binding,
        )
        .unwrap();
        let mut work = task(3000);
        work.run_id = RunId::new(run.clone()).unwrap();
        assert_eq!(
            adapter
                .execute(&work, AdapterCancellation::default())
                .await
                .unwrap()
                .status,
            HermesReceiptStatus::Succeeded
        );
        assert!(original.join("capture.json").is_file());
        assert!(original.join("transcript.json").is_file());
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
    let _restarted = start(&durable, scratch.path());
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
