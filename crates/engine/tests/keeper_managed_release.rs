#![cfg(target_os = "linux")]
#[path = "fixtures/installed_policy.rs"]
#[allow(dead_code)] // Shared fixture exports are only partially used here.
mod installed_policy;
#[path = "../src/bin/keeper_managed/mod.rs"]
#[allow(dead_code)] // Exercise production code without its other binary callers.
mod keeper_managed;
#[path = "../src/bin/keeper_reconcile/mod.rs"]
#[allow(dead_code)]
mod keeper_reconcile;
#[path = "../src/bin/keeper_storage/mod.rs"]
#[allow(dead_code)]
mod keeper_storage;
use arda_engine::objectives::{
    keeper_client::KeeperClient, ControlAction, NewLeaf, NewObjective, ObjectiveStore,
    ProjectAuthority, SnapshotAdmission,
};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::Path,
    process::Command,
    sync::Arc,
    time::Duration,
};

fn command(args: &[&str]) -> String {
    let output = Command::new(args[0]).args(&args[1..]).output().unwrap();
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
struct Managed {
    unit: String,
    stop_timeout: bool,
}
impl Drop for Managed {
    fn drop(&mut self) {
        for action in ["stop", "unmask", "reset-failed"] {
            let _ = Command::new("systemctl")
                .args(["--user", action, "--runtime", &self.unit])
                .output();
        }
    }
}
impl Managed {
    fn start(&self, durable: &Path, runtime: &Path, policy: Option<&Path>) {
        let out = self.start_output(durable, runtime, policy);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fn start_output(
        &self,
        durable: &Path,
        runtime: &Path,
        policy: Option<&Path>,
    ) -> std::process::Output {
        let mut cmd = Command::new("systemd-run");
        cmd.args([
            "--user",
            &format!("--unit={}", self.unit),
            "--service-type=notify",
            "-p",
            "NotifyAccess=main",
            "-p",
            "ProtectControlGroups=yes",
            "-p",
            "Delegate=no",
            "-p",
            "KillMode=control-group",
            "-p",
            "RuntimeDirectoryPreserve=yes",
            "-p",
            "TimeoutStartSec=30",
            "-p",
            "TimeoutStopSec=1",
            &format!("--setenv=ARDA_KEEPER_SYSTEMD_UNIT={}", self.unit),
        ]);
        if self.stop_timeout {
            // A nonterminating initial stop signal forces actual manager escalation.
            cmd.args(["-p", "KillSignal=SIGCONT"]);
        }
        cmd.arg(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
            .arg(durable)
            .arg(runtime)
            .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"));
        if let Some(policy) = policy {
            cmd.arg("--runtime-policy").arg(policy);
        }
        cmd.output().unwrap()
    }
}
fn cli(
    action: &str,
    durable: &Path,
    runtime: &Path,
    owner: &str,
    run: &str,
    extra: &[&str],
) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .args(["reconcile", action, "--durable"])
        .arg(durable)
        .arg("--runtime")
        .arg(runtime)
        .args(["--owner", owner, "--run", run])
        .args(extra)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn marker_count(db: &Path) -> i64 {
    rusqlite::Connection::open(db)
        .unwrap()
        .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| {
            r.get(0)
        })
        .unwrap()
}

#[test]
#[ignore = "requires user systemd, namespaces and installed runtime grant sources; no provider calls"]
fn real_authority_revocation_release_ack_loss_and_engine_restart() {
    managed_release(None);
}

#[test]
#[ignore = "isolated real systemd and production reconciliation handler crash matrix"]
fn managed_reconciliation_cli_crash_matrix() {
    for point in [
        "receipt",
        "snapshot",
        "allocation",
        "precommit",
        "committed",
    ] {
        managed_release(Some(point));
    }
}

#[test]
#[ignore = "subprocess entry only"]
fn managed_reconcile_crash_child() {
    let Ok(serialized) = std::env::var("ARDA_TEST_RECONCILE_ARGS") else {
        return;
    };
    let arguments: Vec<String> = serde_json::from_str(&serialized).unwrap();
    keeper_reconcile::main(arguments.into_iter().map(Into::into).collect()).unwrap();
    panic!("expected reconciliation crash checkpoint");
}

#[test]
#[ignore = "real managed restart attempt after final teardown check"]
fn managed_restart_between_final_check_and_commit() {
    managed_release(Some("restart"));
}

#[test]
#[ignore = "real managed stop timeout and cgroup kill escalation"]
fn managed_stop_timeout_preserves_reconciliation_safety() {
    managed_release(Some("stop-timeout"));
}

#[path = "fixtures/managed_escape.rs"]
mod managed_escape;

#[test]
#[ignore = "real configured worker descendant escape attempts and managed teardown"]
fn managed_descendant_escape_is_denied_and_stop_reaps_detached_child() {
    managed_release(Some("anti-escape"));
}

fn managed_release(crash_point: Option<&str>) {
    let temp = tempfile::tempdir_in("/var/tmp").unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let durable = temp.path().join("owner");
    let workspace = temp.path().join("workspace");
    fs::create_dir(&durable).unwrap();
    fs::create_dir(&workspace).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    command(&[
        env!("CARGO_BIN_EXE_arda-snapshot-keeper"),
        "--initialize",
        durable.to_str().unwrap(),
    ]);
    let policy_root = temp.path().join("policy");
    fs::create_dir(&policy_root).unwrap();
    let policy = installed_policy::write(&policy_root);
    if crash_point == Some("anti-escape") {
        managed_escape::configure(&policy, &policy_root);
    }
    let managed = Managed {
        stop_timeout: crash_point == Some("stop-timeout"),
        unit: format!(
            "arda-reconcile-authority-{}.service",
            uuid::Uuid::new_v4().simple()
        ),
    };
    managed.start(&durable, runtime.path(), Some(&policy));
    let socket = runtime.path().join("keeper.sock");
    let db = temp.path().join("engine.sqlite3");
    let client = || Arc::new(KeeperClient::new(socket.clone()));
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client());
    let now = chrono::Utc::now().timestamp_millis();
    store
        .create_authenticated_objective(
            NewObjective {
                id: "reconcile".into(),
                source_id: "fixture".into(),
                idempotency_key: "fixture".into(),
                operator_id: "operator".into(),
                text: "no-provider reconciliation proof".into(),
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
                    workspace_root: workspace.to_str().unwrap().into(),
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
            "reconcile",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            now,
        )
        .unwrap();
    let claim = store
        .claim_runnable("fixture", now, 120000, 1)
        .unwrap()
        .remove(0);
    let run = claim.execution_run_id.unwrap();
    let binding = store.retained_execution(&run, now).unwrap().unwrap();
    let attack = (crash_point == Some("anti-escape"))
        .then(|| managed_escape::start(&binding, &workspace, &managed.unit));
    let snapshot_bytes = serde_json::to_vec(&binding.snapshot).unwrap();
    let identity_before: String = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row(
            "SELECT identity_json FROM lease_workspace_identities",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&identity_before).unwrap()[0],
        3
    );
    fs::rename(&workspace, temp.path().join("original-workspace")).unwrap();
    fs::create_dir(&workspace).unwrap();
    store
        .apply_control(
            "reconcile",
            ControlAction::Cancel,
            "cancel",
            "operator",
            now + 1,
        )
        .unwrap();
    drop(store);
    // Socket loss is not teardown: the real confined owner and worker remain.
    let group = command(&[
        "systemctl",
        "--user",
        "show",
        &managed.unit,
        "--property=ControlGroup",
        "--value",
    ]);
    let group_path = Path::new("/sys/fs/cgroup").join(group.trim().trim_start_matches('/'));
    assert!(
        fs::read_to_string(group_path.join("cgroup.procs"))
            .unwrap()
            .lines()
            .count()
            > 1
    );
    fs::remove_file(&socket).unwrap();
    let denied_proof = temp.path().join("denied-proof.json");
    let live_owner = fs::read_to_string(durable.join("owner.identity")).unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .args(["reconcile", "stop-proof", "--durable"])
        .arg(&durable)
        .arg("--runtime")
        .arg(runtime.path())
        .args(["--owner", &live_owner, "--run", &run, "--output"])
        .arg(&denied_proof)
        .arg("--confirm-managed-stop")
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(!denied_proof.exists());
    assert!(fs::read_to_string(group_path.join("cgroup.events"))
        .unwrap()
        .lines()
        .any(|line| line == "populated 1"));
    if crash_point == Some("stop-timeout") {
        let pids = fs::read_to_string(group_path.join("cgroup.procs")).unwrap();
        command(&[
            "systemctl",
            "--user",
            "kill",
            "--kill-whom=all",
            "--signal=SIGSTOP",
            &managed.unit,
        ]);
        command(&["systemctl", "--user", "stop", "--no-block", &managed.unit]);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let state = command(&[
                "systemctl",
                "--user",
                "show",
                &managed.unit,
                "--property=ActiveState",
                "--value",
            ]);
            if state.trim() == "deactivating" {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "stop did not begin");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            keeper_storage::offline::open(&durable, runtime.path(), &live_owner, true).is_err()
        );
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &managed.unit])
            .output()
            .unwrap();
        let result = command(&[
            "systemctl",
            "--user",
            "show",
            &managed.unit,
            "--property=Result",
            "--value",
        ]);
        assert_eq!(result.trim(), "timeout");
        for pid in pids.lines() {
            assert!(
                !Path::new("/proc").join(pid).exists(),
                "managed process survived stop"
            );
        }
        command(&["systemctl", "--user", "reset-failed", &managed.unit]);
    } else {
        command(&["systemctl", "--user", "stop", &managed.unit]);
    }
    command(&["systemctl", "--user", "mask", "--runtime", &managed.unit]);
    if let Some(attack) = attack {
        attack.after_stop();
    }
    let owner = fs::read_to_string(durable.join("owner.identity")).unwrap();
    let inspected = cli(
        "inspect",
        &durable,
        runtime.path(),
        &owner,
        &run,
        &["--json"],
    );
    assert!(inspected["blockers"].is_null());
    let evidence = temp.path().join("proof.json");
    cli(
        "stop-proof",
        &durable,
        runtime.path(),
        &owner,
        &run,
        &[
            "--output",
            evidence.to_str().unwrap(),
            "--confirm-managed-stop",
        ],
    );
    let request = uuid::Uuid::new_v4().to_string();
    if let Some(point) =
        crash_point.filter(|point| !matches!(*point, "stop-timeout" | "anti-escape"))
    {
        let arguments = vec![
            "revoke",
            "--durable",
            durable.to_str().unwrap(),
            "--runtime",
            runtime.path().to_str().unwrap(),
            "--owner",
            &owner,
            "--run",
            &run,
            "--expect-record-digest",
            inspected["record_digest"].as_str().unwrap(),
            "--managed-stop-evidence",
            evidence.to_str().unwrap(),
            "--request-id",
            &request,
            "--operator",
            "fixture",
            "--reason",
            "isolated terminal loss",
            "--confirm-terminal-revocation",
        ];
        let database = durable.join("owner.sqlite3");
        let before = rusqlite::Connection::open(&database).unwrap();
        let prior: (String, String) = before
            .query_row(
                "SELECT state,authority FROM snapshots WHERE run=?1",
                [&run],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let allocation: String = before
            .query_row(
                "SELECT state FROM runtime_allocations WHERE run=?1",
                [&run],
                |r| r.get(0),
            )
            .unwrap();
        drop(before);
        let mut child_command = Command::new(std::env::current_exe().unwrap());
        child_command
            .args([
                "--exact",
                "managed_reconcile_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env(
                "ARDA_TEST_RECONCILE_ARGS",
                serde_json::to_string(&arguments).unwrap(),
            )
            .env(
                "ARDA_TEST_REVOKE_CRASH",
                if point == "restart" {
                    "committed"
                } else {
                    point
                },
            );
        if point == "restart" {
            child_command
                .env("ARDA_TEST_REVOKE_PAUSE", "precommit")
                .env("ARDA_TEST_REVOKE_BARRIER", temp.path());
        }
        let child = child_command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        if point == "restart" {
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            while !temp.path().join("ready").exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "crash child barrier not reached"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            command(&["systemctl", "--user", "unmask", "--runtime", &managed.unit]);
            let restarted = managed.start_output(&durable, runtime.path(), None);
            assert!(
                !restarted.status.success(),
                "keeper restarted while offline owner lock held"
            );
            let pid = command(&[
                "systemctl",
                "--user",
                "show",
                &managed.unit,
                "--property=MainPID",
                "--value",
            ]);
            assert_eq!(pid.trim(), "0");
            let journal = command(&[
                "journalctl",
                "--user",
                "-u",
                &managed.unit,
                "--no-pager",
                "-o",
                "cat",
            ]);
            assert!(
                journal.contains("snapshot keeper already owned"),
                "restart did not reach owner exclusion"
            );
            fs::write(temp.path().join("continue"), "continue").unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Reopen through real offline locks and the anchored VFS after abrupt exit.
        let offline =
            keeper_storage::offline::open(&durable, runtime.path(), &owner, true).unwrap();
        let after: (String, String) = offline
            .db
            .query_row(
                "SELECT state,authority FROM snapshots WHERE run=?1",
                [&run],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(prior.1, after.1);
        let allocated: String = offline
            .db
            .query_row(
                "SELECT state FROM runtime_allocations WHERE run=?1",
                [&run],
                |r| r.get(0),
            )
            .unwrap();
        if matches!(point, "committed" | "restart") {
            assert_eq!(after.0, "reconciled_revoked");
            assert_eq!(allocated, "reconciled_revoked");
            let count: i64 = offline
                .db
                .query_row(
                    "SELECT count(*) FROM snapshot_reconciliations WHERE request_id=?1",
                    [&request],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1);
        } else {
            assert_eq!(after.0, prior.0);
            assert_eq!(allocated, allocation);
            let present: bool = offline.db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='snapshot_reconciliations')", [], |r| r.get(0)).unwrap();
            assert!(!present);
        }
    }
    let receipt = cli(
        "revoke",
        &durable,
        runtime.path(),
        &owner,
        &run,
        &[
            "--expect-record-digest",
            inspected["record_digest"].as_str().unwrap(),
            "--managed-stop-evidence",
            evidence.to_str().unwrap(),
            "--request-id",
            &request,
            "--operator",
            "fixture",
            "--reason",
            "isolated terminal loss",
            "--confirm-terminal-revocation",
        ],
    );
    assert_eq!(receipt["worker_cleanup_ack"], false);
    let owner_db = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    let allocation: String = owner_db
        .query_row(
            "SELECT state FROM runtime_allocations WHERE run=?1",
            [&run],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(allocation, "reconciled_revoked");
    drop(owner_db);
    assert_eq!(marker_count(&db), 0);
    command(&["systemctl", "--user", "unmask", "--runtime", &managed.unit]);
    if crash_point == Some("restart") {
        command(&["systemctl", "--user", "reset-failed", &managed.unit]);
    }
    managed.start(&durable, runtime.path(), None);
    let proxy = runtime.path().join("release-proxy.sock");
    let listener = UnixListener::bind(&proxy).unwrap();
    let actual = socket.clone();
    let forward = std::thread::spawn(move || {
        let (mut incoming, _) = listener.accept().unwrap();
        incoming
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let mut request = String::new();
        BufReader::new(&mut incoming)
            .read_line(&mut request)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(value["op"], "release");
        let mut upstream = UnixStream::connect(actual).unwrap();
        upstream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        upstream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        BufReader::new(upstream).read_line(&mut response).unwrap();
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["ok"], true);
        // Drop the real successful keeper response, not a mock acknowledgement.
    });
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(Arc::new(KeeperClient::new(proxy)));
    assert!(store.reconcile_snapshot_commits().is_err());
    forward.join().unwrap();
    drop(store);
    assert_eq!(marker_count(&db), 0);
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client());
    store.reconcile_snapshot_commits().unwrap();
    store.reconcile_snapshot_commits().unwrap();
    assert_eq!(marker_count(&db), 1);
    let identity_after: String = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row(
            "SELECT identity_json FROM lease_workspace_identities",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(identity_before, identity_after);
    let saved: String = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row(
            "SELECT capability_json FROM retained_workspace_snapshots WHERE run_id=?1",
            [&run],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&saved).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&snapshot_bytes).unwrap()
    );
    assert!(client().prepare(&run, &workspace, "invalid").is_err());
    assert!(client()
        .commit(&binding.snapshot, &run, 1, "fixture", now + 120000)
        .is_err());
    assert!(policy_root.join("sessions").exists());
    // No adapter or provider API is invoked anywhere in this test.
}
