use super::*;
#[path = "keeper_active_release.rs"]
mod active_release;
use arda_engine::objectives::{
    keeper_client::exchange,
    snapshot_protocol::{Lease, Request},
    RetainedSnapshot,
};
use std::{
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::{UnixListener, UnixStream},
    },
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};

struct Witness {
    pid: libc::pid_t,
    fd: OwnedFd,
}
impl Witness {
    fn exited(&self) -> bool {
        let mut event = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert!(unsafe { libc::poll(&mut event, 1, 0) } >= 0);
        event.revents & libc::POLLIN != 0
    }
}
struct RebindClient {
    client: KeeperClient,
    socket: std::path::PathBuf,
    lose_release: AtomicBool,
    witness: Mutex<Option<Witness>>,
    supervisors: Mutex<Vec<Witness>>,
    premature: Mutex<Option<UnixStream>>,
    lose_ack: bool,
    injected: AtomicBool,
    prepares: AtomicUsize,
}
impl SnapshotAdmission for RebindClient {
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
        let first_rebind = generation == 2 && !self.injected.swap(true, Ordering::SeqCst);
        if first_rebind {
            for invalid in 0..3 {
                let request = Request::Commit {
                    capability: if invalid == 0 {
                        "invalid".into()
                    } else {
                        snapshot.capability.clone()
                    },
                    manifest_digest: if invalid == 1 {
                        "invalid".into()
                    } else {
                        snapshot.manifest_digest.clone()
                    },
                    lease: Lease {
                        run_id: if invalid == 2 {
                            "wrong-run".into()
                        } else {
                            run.into()
                        },
                        generation,
                        owner: owner.into(),
                        expires_ms: expires,
                    },
                };
                let denied: serde_json::Value = exchange(
                    Path::new(&snapshot.endpoint),
                    &request,
                    Duration::from_secs(2),
                )?;
                assert_eq!(denied["ok"], false);
                assert!(
                    !self.witness.lock().unwrap().as_ref().unwrap().exited(),
                    "invalid Commit cancelled work"
                );
            }
            // Queue an N+1 dispatch before Commit while N is still running.
            // It must be rejected, not execute as an early generation handoff.
            let request = execute(
                snapshot,
                Lease {
                    run_id: run.into(),
                    generation,
                    owner: owner.into(),
                    expires_ms: expires,
                },
                vec!["/usr/bin/touch".into(), "before-ack".into()],
                1000,
            );
            let mut stream = UnixStream::connect(&snapshot.endpoint)?;
            stream.set_write_timeout(Some(Duration::from_secs(1)))?;
            stream.set_read_timeout(Some(Duration::from_secs(12)))?;
            use std::io::Write;
            let mut payload = serde_json::to_vec(&request)?;
            payload.push(b'\n');
            stream.write_all(&payload)?;
            *self.premature.lock().unwrap() = Some(stream);
            assert!(
                !self.witness.lock().unwrap().as_ref().unwrap().exited(),
                "old execution must be live at rebind delivery"
            );
        }
        let result = self
            .client
            .commit(snapshot, run, generation, owner, expires);
        if generation == 2 {
            let witness = self.witness.lock().unwrap();
            let witness = witness.as_ref().unwrap();
            if result.is_ok() {
                assert!(witness.exited(), "ACK preceded prior execution exit");
                assert!(
                    !Path::new(&format!("/proc/{}", witness.pid)).exists(),
                    "ACK preceded reap"
                );
                for supervisor in self.supervisors.lock().unwrap().iter() {
                    assert!(supervisor.exited(), "ACK preceded supervisor exit");
                    assert!(
                        !Path::new(&format!("/proc/{}", supervisor.pid)).exists(),
                        "ACK preceded supervisor reap"
                    );
                }
            }
        }
        result?;
        if first_rebind && self.lose_ack {
            anyhow::bail!("fixture drops successful rebind ACK");
        }
        Ok(())
    }
    fn release(&self, snapshot: &RetainedSnapshot, run: &str) -> anyhow::Result<()> {
        if self.lose_release.swap(false, Ordering::SeqCst) {
            return active_release::lose_ack(&self.socket, snapshot, run);
        }
        self.client.release(snapshot, run)
    }
}
fn execute(
    snapshot: &RetainedSnapshot,
    lease: Lease,
    argv: Vec<String>,
    timeout_ms: u64,
) -> Request {
    Request::Execute {
        capability: snapshot.capability.clone(),
        lease,
        argv,
        environment: BTreeMap::new(),
        timeout_ms,
        max_output_bytes: 1024,
    }
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_rebind_ack_loss_waits_for_prior_reap() {
    active_rebind(true, false, false, 0);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_rebind_cancels_before_timeout() {
    active_rebind(false, false, false, 0);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_rebind_owner_restart_preserves_uncertainty() {
    active_rebind(true, true, false, 0);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_rebind_incomplete_control_cannot_delay_timeout() {
    active_rebind(false, false, true, 0);
}
fn active_rebind(lose_ack: bool, restart: bool, incomplete: bool, terminal: u8) {
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
    let mut keeper = start_with_policy(&durable, runtime.path(), None);
    let client = Arc::new(RebindClient {
        client: KeeperClient::new(runtime.path().join("keeper.sock")),
        socket: runtime.path().join("keeper.sock"),
        lose_release: AtomicBool::new(terminal == 2),
        witness: Mutex::new(None),
        supervisors: Mutex::new(Vec::new()),
        premature: Mutex::new(None),
        lose_ack,
        injected: AtomicBool::new(false),
        prepares: AtomicUsize::new(0),
    });
    let db = parent.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&db)
        .unwrap()
        .with_snapshot_admission(client.clone());
    let now = chrono::Utc::now().timestamp_millis();
    store
        .create_authenticated_objective(
            NewObjective {
                id: "rebind".into(),
                source_id: "fixture".into(),
                idempotency_key: "fixture".into(),
                operator_id: "operator".into(),
                text: "active execution rebind".into(),
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
            now,
        )
        .unwrap();
    store
        .apply_control(
            "rebind",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            now,
        )
        .unwrap();
    let claim = store
        .claim_runnable("first", now, 60_000, 1)
        .unwrap()
        .remove(0);
    let run = claim.execution_run_id.unwrap();
    let binding = store.retained_execution(&run, now).unwrap().unwrap();
    let snapshot = binding.snapshot.clone();
    let listener = UnixListener::bind(root.join("witness.sock")).unwrap();
    listener.set_nonblocking(true).unwrap();
    let request = execute(&snapshot, binding.lease.clone(), vec!["/usr/bin/python3".into(), "-c".into(), "import os,socket,time;child=os.fork();s=socket.socket(socket.AF_UNIX);s.connect('witness.sock') if child==0 else None;time.sleep(60)".into()], if incomplete { 700 } else { 30_000 });
    let endpoint = snapshot.endpoint.clone();
    let execution = std::thread::spawn(move || {
        exchange::<_, serde_json::Value>(Path::new(&endpoint), &request, Duration::from_secs(12))
            .unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(4);
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "execution never reached witness");
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("{error}"),
        }
    };
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of_val(&credentials) as libc::socklen_t;
    assert_eq!(
        unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut size,
            )
        },
        0
    );
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, credentials.pid, 0) } as i32;
    assert!(fd >= 0);
    *client.witness.lock().unwrap() = Some(Witness {
        pid: credentials.pid,
        fd: unsafe { OwnedFd::from_raw_fd(fd) },
    });
    let mut pid = credentials.pid;
    let mut found_worker = false;
    let mut found_launcher = false;
    for _ in 0..12 {
        let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
        pid = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let exe = fs::read_link(format!("/proc/{pid}/exe")).unwrap();
        if exe.file_name().unwrap() == "arda-snapshot-worker" {
            let args = fs::read(format!("/proc/{pid}/cmdline")).unwrap();
            if args
                .split(|byte| *byte == 0)
                .any(|arg| arg == b"--launch-bwrap")
            {
                found_launcher = true;
            } else {
                found_worker = true;
                break;
            }
        }
        assert!(pid > 1, "failed to locate worker ancestor");
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) as i32 };
        assert!(fd >= 0);
        client.supervisors.lock().unwrap().push(Witness {
            pid,
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        });
    }
    assert!(found_worker);
    assert!(
        found_launcher,
        "launcher missing from witnessed supervisor chain"
    );
    assert!(
        client.supervisors.lock().unwrap().len() >= 2,
        "missing provider/supervisor ancestors"
    );
    if terminal != 0 {
        active_release::finish(terminal, &store, &client, &snapshot, &run, &db, &durable);
        let result = execution.join().unwrap();
        assert_eq!(result["cancelled"], true);
        assert_eq!(result["timed_out"], false);
        return;
    }
    if incomplete {
        use std::io::Write;
        let mut partial = UnixStream::connect(&snapshot.endpoint).unwrap();
        partial.write_all(b"{").unwrap();
        let began = Instant::now();
        let result = execution.join().unwrap();
        assert!(
            began.elapsed() < Duration::from_millis(1500),
            "incomplete control stalled execution supervision"
        );
        assert_eq!(result["timed_out"], true);
        assert_eq!(result["cancelled"], false);
        assert!(client.witness.lock().unwrap().as_ref().unwrap().exited());
        for ancestor in client.supervisors.lock().unwrap().iter() {
            assert!(ancestor.exited());
            assert!(!Path::new(&format!("/proc/{}", ancestor.pid)).exists());
        }
        let retained = store.retained_execution(&run, now).unwrap().unwrap();
        assert!(retained.snapshot == snapshot && retained.lease == binding.lease);
        store
            .apply_control(
                "rebind",
                ControlAction::Cancel,
                "cancel",
                "operator",
                now + 1000,
            )
            .unwrap();
        store.reconcile_snapshot_commits().unwrap();
        return;
    }
    // Advance only Engine's injected clock: the worker's actual wall/monotonic
    // lease remains live, so this cannot pass merely by expiring prior work.
    let recovered_now = now + 60_001;
    let began = Instant::now();
    let result = store.claim_runnable("second", recovered_now, 60_000, 1);
    if lose_ack {
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("successful rebind ACK"));
    } else {
        assert_eq!(result.unwrap().len(), 1);
    }
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "rebind waited for timeout"
    );
    let engine = rusqlite::Connection::open(&db).unwrap();
    assert_eq!(
        engine
            .query_row(
                "SELECT committed_generation FROM retained_workspace_snapshots WHERE run_id=?1",
                [&run],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        if lose_ack { 1 } else { 2 }
    );
    drop(store);
    let reopened = ObjectiveStore::open_existing(&db)
        .unwrap()
        .with_snapshot_admission(client.clone());
    if restart {
        keeper.0.kill().unwrap();
        keeper.0.wait().unwrap();
        drop(keeper);
        let _restarted = start_with_policy(&durable, runtime.path(), None);
        assert!(reopened.reconcile_snapshot_commits().is_err());
        assert!(reopened.retained_execution(&run, recovered_now).is_err());
        let intent: (i64, String, i64) = engine.query_row(
            "SELECT generation,lease_owner,lease_expires_ms FROM retained_snapshot_lease_intents WHERE leaf_id='leaf' AND generation=2", [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        assert_eq!(intent, (2, "second".into(), recovered_now + 60_000));
        assert_eq!(
            engine
                .query_row(
                    "SELECT committed_generation FROM retained_workspace_snapshots WHERE run_id=?1",
                    [&run],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            engine
                .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let owner = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
        let (state, authority): (String, String) = owner
            .query_row(
                "SELECT state,authority FROM snapshots WHERE run=?1",
                [&run],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(state, "lost");
        assert!(serde_json::from_str::<RetainedSnapshot>(&authority).unwrap() == snapshot);
        assert_eq!(client.prepares.load(Ordering::SeqCst), 1);
        let result = execution.join().unwrap();
        assert_eq!(result["cancelled"], true);
        return;
    }
    reopened.reconcile_snapshot_commits().unwrap();
    let result = execution.join().unwrap();
    let premature = client.premature.lock().unwrap().take().unwrap();
    use std::io::BufRead;
    let mut reply = String::new();
    std::io::BufReader::new(premature)
        .read_line(&mut reply)
        .unwrap();
    let refused: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(refused["ok"], false);
    assert_eq!(refused["error"], "snapshot execution lease is fenced");
    assert!(!root.join("before-ack").exists());
    assert_eq!(result["ok"], true);
    assert_eq!(result["timed_out"], false);
    assert_eq!(result["cancelled"], true);
    let recovered = reopened
        .retained_execution(&run, recovered_now)
        .unwrap()
        .unwrap();
    assert!(recovered.snapshot == snapshot);
    assert_eq!(recovered.lease.generation, 2);
    assert_eq!(client.prepares.load(Ordering::SeqCst), 1);
    let stale = execute(
        &snapshot,
        binding.lease,
        vec!["/usr/bin/touch".into(), "stale-write".into()],
        1000,
    );
    let denied: serde_json::Value = exchange(
        Path::new(&snapshot.endpoint),
        &stale,
        Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(denied["ok"], false);
    assert!(denied["error"]
        .as_str()
        .unwrap()
        .contains("lease is fenced"));
    assert!(!root.join("stale-write").exists());
    let next = execute(
        &snapshot,
        recovered.lease,
        vec!["/usr/bin/touch".into(), "new-write".into()],
        1000,
    );
    let allowed: serde_json::Value =
        exchange(Path::new(&snapshot.endpoint), &next, Duration::from_secs(3)).unwrap();
    assert_eq!(allowed["ok"], true);
    assert_eq!(allowed["code"], 0);
    assert!(root.join("new-write").exists());
    reopened
        .apply_control(
            "rebind",
            ControlAction::Cancel,
            "cancel",
            "operator",
            recovered_now,
        )
        .unwrap();
    reopened.reconcile_snapshot_commits().unwrap();
    assert!(!Path::new(&snapshot.endpoint).exists());
    let owner = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    assert_eq!(
        owner
            .query_row("SELECT state FROM snapshots WHERE run=?1", [&run], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "released"
    );
}
