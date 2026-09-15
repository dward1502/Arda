use super::*;
use arda_engine::objectives::RetainedSnapshot;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

struct LostPrepare {
    client: KeeperClient,
    drop_response: AtomicBool,
    authority: Mutex<Option<RetainedSnapshot>>,
}
impl SnapshotAdmission for LostPrepare {
    fn prepare(&self, run: &str, root: &Path, identity: &str) -> anyhow::Result<RetainedSnapshot> {
        let snapshot = self.client.prepare(run, root, identity)?;
        let mut saved = self.authority.lock().unwrap();
        if let Some(prior) = saved.as_ref() {
            assert!(*prior == snapshot);
        }
        *saved = Some(snapshot.clone());
        if self.drop_response.swap(false, Ordering::SeqCst) {
            anyhow::bail!(
                "fixture drops successful Prepare response before Engine records authority"
            );
        }
        Ok(snapshot)
    }
    fn commit(
        &self,
        snapshot: &RetainedSnapshot,
        run: &str,
        generation: i64,
        owner: &str,
        expires: i64,
    ) -> anyhow::Result<()> {
        self.client
            .commit(snapshot, run, generation, owner, expires)
    }
    fn release(&self, snapshot: &RetainedSnapshot, run: &str) -> anyhow::Result<()> {
        self.client.release(snapshot, run)
    }
}

#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn successful_prepare_response_loss_preserves_one_nonexecuting_orphan() {
    prepare_rollback(false);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn successful_prepare_engine_transaction_rollback_preserves_authority() {
    prepare_rollback(true);
}
fn prepare_rollback(abort_transaction: bool) {
    let parent = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = parent.path().join("workspace");
    let durable = parent.path().join("owner");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&durable).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    assert!(Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .arg("--initialize")
        .arg(&durable)
        .status()
        .unwrap()
        .success());
    let _keeper = start_with_policy(&durable, runtime.path(), None);
    let client = Arc::new(LostPrepare {
        client: KeeperClient::new(runtime.path().join("keeper.sock")),
        drop_response: AtomicBool::new(!abort_transaction),
        authority: Mutex::new(None),
    });
    let db = parent.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client.clone());
    let now = chrono::Utc::now().timestamp_millis();
    store
        .create_authenticated_objective(
            NewObjective {
                id: "prepare-rollback".into(),
                source_id: "fixture".into(),
                idempotency_key: "fixture".into(),
                operator_id: "operator".into(),
                text: "retained preparation rollback".into(),
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
            "prepare-rollback",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            now,
        )
        .unwrap();
    let engine = rusqlite::Connection::open(&db).unwrap();
    if abort_transaction {
        engine.execute_batch("CREATE TRIGGER fixture_abort BEFORE INSERT ON retained_workspace_snapshots BEGIN SELECT RAISE(ABORT, 'fixture abort after Prepare'); END;").unwrap();
    }
    let failure = store.claim_runnable("first", now, 60_000, 1).unwrap_err();
    assert!(format!("{failure:#}").contains(if abort_transaction {
        "fixture abort after Prepare"
    } else {
        "fixture drops successful Prepare"
    }));
    for table in [
        "retained_workspace_snapshots",
        "lease_workspace_identities",
        "retained_snapshot_lease_intents",
    ] {
        assert_eq!(
            engine
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        engine
            .query_row("SELECT attempt FROM leaves WHERE id='leaf'", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let owner = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    let (run, encoded): (String, String) = owner
        .query_row(
            "SELECT run,authority FROM snapshots WHERE state='prepared'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let snapshot: RetainedSnapshot = serde_json::from_str(&encoded).unwrap();
    let inspect = || -> serde_json::Value {
        arda_engine::objectives::keeper_client::exchange(
            Path::new(&snapshot.endpoint),
            &arda_engine::objectives::snapshot_protocol::Request::Inspect,
            Duration::from_secs(3),
        )
        .unwrap()
    };
    let inspection = inspect();
    assert_eq!(inspection["ok"], true);
    assert_eq!(inspection.get("committed"), Some(&serde_json::Value::Null));
    let endpoint_metadata = fs::metadata(&snapshot.endpoint).unwrap();
    // Remove the injection before testing identity refusal, so it cannot mask
    // an accidentally accepted replacement workspace.
    if abort_transaction {
        engine.execute_batch("DROP TRIGGER fixture_abort").unwrap();
    }
    let original = parent.path().join("original");
    fs::rename(&root, &original).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(store
        .claim_runnable("changed-path", now, 60_000, 1)
        .unwrap_err()
        .to_string()
        .contains("snapshot keeper refused"));
    let inspection = inspect();
    assert_eq!(inspection["ok"], true);
    assert_eq!(inspection.get("committed"), Some(&serde_json::Value::Null));
    fs::remove_dir(&root).unwrap();
    fs::rename(&original, &root).unwrap();
    drop(store);
    let reopened = ObjectiveStore::open_existing(&db)
        .unwrap()
        .with_snapshot_admission(client);
    let claim = reopened
        .claim_runnable("retry", chrono::Utc::now().timestamp_millis(), 60_000, 1)
        .unwrap()
        .remove(0);
    assert_eq!(claim.execution_run_id.as_deref(), Some(run.as_str()));
    let binding = reopened
        .retained_execution(&run, chrono::Utc::now().timestamp_millis())
        .unwrap()
        .unwrap();
    assert!(binding.snapshot == snapshot);
    use std::os::unix::fs::MetadataExt;
    let after = fs::metadata(&snapshot.endpoint).unwrap();
    assert_eq!(
        (endpoint_metadata.dev(), endpoint_metadata.ino()),
        (after.dev(), after.ino())
    );
    assert_eq!(
        owner
            .query_row("SELECT count(*) FROM snapshots", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    reopened
        .apply_control(
            "prepare-rollback",
            ControlAction::Cancel,
            "cancel",
            "operator",
            chrono::Utc::now().timestamp_millis(),
        )
        .unwrap();
    reopened.reconcile_snapshot_commits().unwrap();
    assert_eq!(
        engine
            .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
