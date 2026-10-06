use super::*;
use arda_engine::objectives::{snapshot_protocol::Request, RetainedSnapshot};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct DropFirstCommit {
    client: KeeperClient,
    socket: std::path::PathBuf,
    wire_loss: bool,
    drop_commit: AtomicBool,
    prepares: AtomicUsize,
    commits: std::sync::Mutex<Vec<serde_json::Value>>,
}
impl SnapshotAdmission for DropFirstCommit {
    fn prepare(&self, run: &str, root: &Path, identity: &str) -> anyhow::Result<RetainedSnapshot> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        self.client.prepare(run, root, identity)
    }
    fn commit(
        &self,
        snapshot: &RetainedSnapshot,
        run: &str,
        generation: i64,
        owner: &str,
        expires: i64,
    ) -> anyhow::Result<()> {
        self.commits.lock().unwrap().push(serde_json::json!({
            "snapshot": snapshot, "run": run, "generation": generation,
            "owner": owner, "expires": expires
        }));
        if self.drop_commit.swap(false, Ordering::SeqCst) {
            if self.wire_loss {
                use arda_engine::objectives::keeper_client::KeeperRequest;
                use std::io::{BufRead, BufReader, Write};
                use std::os::unix::net::{UnixListener, UnixStream};
                let proxy = self.socket.with_file_name("lost-commit.sock");
                let listener = UnixListener::bind(&proxy)?;
                let target = self.socket.clone();
                let forward = std::thread::spawn(move || {
                    let (mut downstream, _) = listener.accept().unwrap();
                    downstream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut payload = String::new();
                    BufReader::new(&mut downstream)
                        .read_line(&mut payload)
                        .unwrap();
                    let mut upstream = UnixStream::connect(target).unwrap();
                    upstream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    upstream.write_all(payload.as_bytes()).unwrap();
                    let mut response = String::new();
                    BufReader::new(upstream).read_line(&mut response).unwrap();
                    let response: serde_json::Value = serde_json::from_str(&response).unwrap();
                    assert_eq!(response["ok"], true);
                    // Close downstream without forwarding the successful server ACK.
                });
                let lost = arda_engine::objectives::keeper_client::exchange::<_, serde_json::Value>(
                    &proxy,
                    &KeeperRequest::Commit {
                        snapshot: snapshot.clone(),
                        lease: arda_engine::objectives::snapshot_protocol::Lease {
                            run_id: run.into(),
                            generation,
                            owner: owner.into(),
                            expires_ms: expires,
                        },
                    },
                    Duration::from_secs(5),
                );
                forward.join().unwrap();
                assert!(lost.is_err(), "missing socket reply must fail transport");
                anyhow::bail!("fixture loses Commit ACK after transport delivery");
            }
            anyhow::bail!("fixture drops Commit before transport delivery");
        }
        self.client
            .commit(snapshot, run, generation, owner, expires)
    }
    fn release(&self, snapshot: &RetainedSnapshot, run: &str) -> anyhow::Result<()> {
        self.client.release(snapshot, run)
    }
}

#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn expired_undelivered_commit_recovers_same_snapshot_without_execution() {
    commit_loss(false);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn wire_commit_ack_loss_recovers_same_snapshot_without_execution() {
    commit_loss(true);
}
fn commit_loss(wire_loss: bool) {
    let parent = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = parent.path().join("workspace");
    let durable = parent.path().join("owner");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&durable).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .arg("--initialize")
        .arg(&durable)
        .status()
        .unwrap()
        .success());
    let _keeper = start_with_policy(&durable, runtime.path(), None);
    let client = Arc::new(DropFirstCommit {
        client: KeeperClient::new(runtime.path().join("keeper.sock")),
        socket: runtime.path().join("keeper.sock"),
        wire_loss,
        drop_commit: AtomicBool::new(true),
        prepares: AtomicUsize::new(0),
        commits: std::sync::Mutex::new(Vec::new()),
    });
    let db = parent.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client.clone());
    let old_now = chrono::Utc::now().timestamp_millis() - 10_000;
    store
        .create_authenticated_objective(
            NewObjective {
                id: "undelivered".into(),
                source_id: "fixture".into(),
                idempotency_key: "fixture".into(),
                operator_id: "operator".into(),
                text: "never execute expired intent".into(),
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
                    authority: "read_only".into(),
                    dependencies: vec![],
                    execution: None,
                }],
            },
            old_now,
        )
        .unwrap();
    store
        .apply_control(
            "undelivered",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            old_now,
        )
        .unwrap();
    assert!(store
        .claim_runnable("first", old_now, 1, 1)
        .unwrap_err()
        .to_string()
        .contains(if wire_loss {
            "after transport delivery"
        } else {
            "before transport delivery"
        }));
    let connection = rusqlite::Connection::open(&db).unwrap();
    let (run, encoded): (String, String) = connection
        .query_row(
            "SELECT run_id,capability_json FROM retained_workspace_snapshots",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let snapshot: RetainedSnapshot = serde_json::from_str(&encoded).unwrap();
    let old_lease = arda_engine::objectives::snapshot_protocol::Lease {
        run_id: run.clone(),
        generation: 1,
        owner: "first".into(),
        expires_ms: old_now + 1,
    };
    drop(store);
    let intent: (i64, String, i64, i64) = connection.query_row(
        "SELECT i.generation,i.lease_owner,i.lease_expires_ms,s.committed_generation FROM retained_snapshot_lease_intents i JOIN retained_workspace_snapshots s ON s.leaf_id=i.leaf_id",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
    assert_eq!(intent, (1, "first".into(), old_now + 1, 0));
    let expected = serde_json::json!({"snapshot": snapshot, "run": run, "generation": 1, "owner": "first", "expires": old_now + 1});
    assert_eq!(*client.commits.lock().unwrap(), vec![expected.clone()]);
    let reopened = ObjectiveStore::open_existing(&db)
        .unwrap()
        .with_snapshot_admission(client.clone());
    reopened
        .reconcile_snapshot_commits()
        .expect("expired undelivered intent must reconcile as non-executable fencing");
    assert_eq!(
        *client.commits.lock().unwrap(),
        vec![expected.clone(), expected]
    );
    assert_eq!(client.prepares.load(Ordering::SeqCst), 1);
    let denied: serde_json::Value = arda_engine::objectives::keeper_client::exchange(
        Path::new(&snapshot.endpoint),
        &Request::Execute {
            capability: snapshot.capability.clone(),
            lease: old_lease.clone(),
            argv: vec![
                "/usr/bin/touch".into(),
                root.join("must-not-exist").to_str().unwrap().into(),
            ],
            environment: BTreeMap::new(),
            timeout_ms: 1000,
            max_output_bytes: 1024,
        },
        Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(denied["ok"], false);
    assert!(!root.join("must-not-exist").exists());
    let now = chrono::Utc::now().timestamp_millis();
    let claim = reopened
        .claim_runnable("second", now, 60_000, 1)
        .unwrap()
        .remove(0);
    assert_eq!(claim.execution_run_id.as_deref(), Some(run.as_str()));
    assert_eq!(claim.attempt, 2);
    let binding = reopened.retained_execution(&run, now).unwrap().unwrap();
    assert!(binding.snapshot == snapshot);
    assert_eq!(client.prepares.load(Ordering::SeqCst), 1);
    assert!(client
        .client
        .commit(&snapshot, &run, 1, "first", old_now + 1)
        .is_err());
    reopened
        .apply_control(
            "undelivered",
            ControlAction::Cancel,
            "cancel",
            "operator",
            now,
        )
        .unwrap();
    reopened.reconcile_snapshot_commits().unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
