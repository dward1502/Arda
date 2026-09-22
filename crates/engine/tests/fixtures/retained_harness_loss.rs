use super::*;
use arda_engine::objectives::{
    keeper_client::KeeperClient, ControlAction, NewLeaf, NewObjective, ObjectiveStore,
    ProjectAuthority, RetainedExecution,
};
use std::{
    os::unix::fs::PermissionsExt,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Keeper(Child);
impl Drop for Keeper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn admission(root: &TempDir) -> (Keeper, TempDir, ObjectiveStore, String, RetainedExecution) {
    let durable = root.path().parent().unwrap().join("owner");
    fs::create_dir(&durable).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .args(["--initialize"])
        .arg(&durable)
        .status()
        .unwrap()
        .success());
    let mut keeper = Keeper(
        Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
            .arg(&durable)
            .arg(runtime.path())
            .arg(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
            .stdin(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(runtime.path().join("keeper.sock")).is_err() {
        assert!(keeper.0.try_wait().unwrap().is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3"))
        .unwrap()
        .with_snapshot_admission(Arc::new(KeeperClient::new(
            runtime.path().join("keeper.sock"),
        )));
    let now = chrono::Utc::now().timestamp_millis();
    store
        .create_authenticated_objective(
            NewObjective {
                id: "loss".into(),
                source_id: "fixture".into(),
                idempotency_key: "fixture".into(),
                operator_id: "operator".into(),
                text: "loss fixture".into(),
                priority: 0,
                projects: vec![ProjectAuthority {
                    project_id: PROJECT_ID.into(),
                    contract_digest: "sha256:fixture".into(),
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
    store
        .apply_control(
            "loss",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            now,
        )
        .unwrap();
    let run = store
        .claim_runnable("owner", now, 60_000, 1)
        .unwrap()
        .remove(0)
        .execution_run_id
        .unwrap();
    let binding = store.retained_execution(&run, now).unwrap().unwrap();
    (keeper, runtime, store, run, binding)
}
#[tokio::test]
#[ignore = "requires Linux retained workers, user/mount namespaces and bubblewrap"]
async fn active_retained_harness_loss_matrix() {
    for mode in ["disconnect", "keeper-loss", "shutdown"] {
        retained_loss(mode).await;
    }
}
async fn retained_loss(mode: &str) {
    use arda_engine::supervisor::Shutdown;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    let parent = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = tempfile::tempdir_in(parent.path()).unwrap();
    let (mut keeper, _runtime, objectives, run_id, binding) = admission(&root);
    let executable = root.path().join("slow-provider");
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let socket_path = root.path().join("provider.sock");
    let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
    fs::write(
        &executable,
        format!(
            "#!/usr/bin/python3\nimport socket,time\ns=socket.socket(socket.AF_UNIX)\ns.connect('{}')\ntime.sleep(10)\n",
            socket_path.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    write_file_only_hermes_config(&root);
    let config_path = root.path().join("config/adapters/hermes-workbench.toml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace("/bin/true", executable.to_str().unwrap())
        .replace("max_timeout_ms = 1000", "max_timeout_ms = 30000");
    fs::write(config_path, config).unwrap();
    let shutdown = Shutdown::new();
    let (bound, mut handle) = arda_engine::harness::serve_with_shutdown(
        Some("127.0.0.1:0".parse().unwrap()),
        harness_state(&root),
        shutdown.clone(),
    )
    .await
    .unwrap();
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    let mut work = graph(&run_id, "inspect", "inspect");
    work["nodes"][0]["state"] = json!("ready");
    work["nodes"][0]["timeout_ms"] = json!(30000);
    work["nodes"][0]["parent_receipts"] = json!(["receipt:approval"]);
    work["nodes"]
        .as_array_mut()
        .unwrap()
        .push(graph(&run_id, "approval", "approval")["nodes"][0].clone());
    work["nodes"][1]["idempotency_key"] = json!("node-shutdown-approval");
    work["edges"] = json!([{"id": "approval-inspect", "from": "approval", "to": "inspect", "parent_receipt": "receipt:approval"}]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID, "graph": work, "envelope": envelope("plan-shutdown")
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("http://{bound}/v1/runs/{run_id}/approve"))
        .json(&json!({"node_id": "approval", "envelope": envelope("approve-shutdown")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let request_run = run_id.clone();
    let request_lease = binding.lease.clone();
    let request = tokio::spawn(async move {
        client
            .post(format!(
                "http://{bound}/v1/runs/{request_run}/nodes/inspect/execute-provider"
            ))
            .json(
                &json!({"envelope": envelope("execute-shutdown"), "objective": "inspect fixture", "expected_retained_lease": request_lease}),
            )
            .send()
            .await
            .unwrap()
    });
    let started = tokio::time::timeout(Duration::from_secs(2), listener.accept()).await;
    // Kernel peer credentials are expressed in the receiving host namespace;
    // never interpret the worker's namespace-local getpid()/$$ as a host PID.
    let provider = started
        .as_ref()
        .ok()
        .and_then(|result| result.as_ref().ok())
        .map(|(stream, _)| {
            let pid = stream.peer_cred().unwrap().pid().unwrap();
            let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
            assert!(fd >= 0, "cannot open owned provider pidfd");
            unsafe { OwnedFd::from_raw_fd(fd as i32) }
        });
    let is_alive = |fd: &OwnedFd| {
        let mut poll = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut poll, 1, 0) };
        assert!(result >= 0);
        result == 0
    };
    assert!(
        provider.as_ref().is_some_and(&is_alive),
        "provider not live before shutdown"
    );
    if mode == "disconnect" {
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        // Observe cancellation/provider exit from disconnect alone, before shutdown.
        let deadline = Instant::now() + Duration::from_secs(3);
        while is_alive(provider.as_ref().unwrap()) {
            assert!(
                Instant::now() < deadline,
                "provider did not exit after disconnect"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!handle.is_finished());
        assert!(reqwest::get(format!("http://{bound}/v1/runs/{run_id}"))
            .await
            .unwrap()
            .status()
            .is_success());
    } else if mode == "keeper-loss" {
        keeper.0.kill().unwrap();
        keeper.0.wait().unwrap();
        let response = tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .unwrap()
            .unwrap();
        assert!(!response.status().is_success());
        assert!(
            !is_alive(provider.as_ref().unwrap()),
            "provider survived keeper death"
        );
        assert!(!handle.is_finished(), "keeper loss stopped Harness");
    } else {
        shutdown.trigger();
        let response = tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    }
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), &mut handle)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !is_alive(provider.as_ref().unwrap()),
        "provider survived cleanup"
    );
    // Transport/process loss must not erase durable recovery authority or
    // manufacture terminal cleanup acknowledgement before explicit cancellation.
    let retained = objectives
        .retained_execution(&run_id, chrono::Utc::now().timestamp_millis())
        .unwrap()
        .unwrap();
    assert!(retained.snapshot == binding.snapshot);
    assert!(retained.lease == binding.lease);
    let db = rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    if mode != "keeper-loss" {
        objectives
            .apply_control(
                "loss",
                ControlAction::Cancel,
                "cancel",
                "operator",
                chrono::Utc::now().timestamp_millis(),
            )
            .unwrap();
        objectives.reconcile_snapshot_commits().unwrap();
        let db =
            rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    let store = arda_engine::runs::RunStore::open(
        root.path(),
        arda_core::run_graph::RunId::new(&run_id).unwrap(),
    )
    .unwrap();
    let recovered = store.recover().unwrap();
    assert_eq!(
        recovered.checkpoint.unwrap().nodes[0].state,
        if mode == "keeper-loss" {
            arda_core::run_graph::NodeState::Failed
        } else {
            arda_core::run_graph::NodeState::Running
        },
        "{mode}"
    );
    assert!(!root
        .path()
        .join(format!(
            "data/runs/{run_id}/execution-receipts/inspect.json"
        ))
        .exists());
    println!("retained Harness {mode}: provider exited; authority preserved; no success receipt");
}
