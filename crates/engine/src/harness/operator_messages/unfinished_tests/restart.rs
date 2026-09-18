use super::*;
use std::{path::Path, sync::Arc, time::Duration};

const TARGET: &str = "harness::operator_messages::unfinished_tests::harness_owned_recovery_survives_waiter_loss_and_records_exact_start_error";
const REOPEN: &str = "harness::operator_messages::unfinished_tests::restart::reopen_child";

fn bounded(command: &mut std::process::Command) -> std::process::Output {
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "recovery subprocess timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn abrupt_loss_reopens_original_ingress_without_redispatch() {
    for phase in ["before", "between", "after"] {
        let handoff = tempfile::tempdir().unwrap();
        let mode = format!("waiter-loss-crash-{phase}-outcome");
        let output = bounded(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TARGET, "--nocapture"])
                .env("ARDA_UNFINISHED_RECOVERY_CHILD", &mode)
                .env("ARDA_RECOVERY_CRASH_HANDOFF", handoff.path())
                .env("ARDA_HERMES_LOCAL_CAPABILITY", "fixture-token"),
        );
        assert_eq!(
            output.status.code(),
            Some(77),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let root = std::fs::read_to_string(handoff.path().join("root")).unwrap();
        let output = bounded(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", REOPEN, "--ignored", "--nocapture"])
                .env("ARDA_UNFINISHED_RECOVERY_CHILD", "waiter-loss-reopen")
                .env("ARDA_RECOVERY_REOPEN_ROOT", &root)
                .env("ARDA_RECOVERY_REOPEN_PHASE", phase)
                .env("ARDA_HERMES_LOCAL_CAPABILITY", "fixture-token"),
        );
        // Root was created solely by this test's child; exit bypassed TempDir Drop.

        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            output.status.success(),
            "reopen {phase}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[ignore = "fresh-process helper for abrupt_loss_reopens_original_ingress_without_redispatch"]
fn reopen_child() {
    let root = std::env::var("ARDA_RECOVERY_REOPEN_ROOT").unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(reopen(Path::new(&root)));
}

async fn reopen(root: &Path) {
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("restart-transport.json")).unwrap())
            .unwrap();
    let manifest: Manifest = serde_json::from_value(saved["manifest"].clone()).unwrap();
    let snapshot: RetainedSnapshot = serde_json::from_value(saved["snapshot"].clone()).unwrap();
    // Rebind isolated mock transport at the same captured endpoint, not a new grant.
    for socket in ["worker.sock", "keeper.sock"] {
        std::fs::remove_file(root.join(socket)).unwrap();
    }
    let transport = transport::start(root, &snapshot, manifest);
    std::env::set_var("ARDA_SNAPSHOT_KEEPER_SOCKET", root.join("keeper.sock"));
    std::env::set_var(
        "ARDA_RETAINED_HERMES_CONFIG_OVERRIDE",
        root.join("config/adapters/hermes-workbench.toml"),
    );
    let run = crate::runs::RunStore::open(
        root,
        arda_core::run_graph::RunId::new("recovery-evidence-fixture").unwrap(),
    )
    .unwrap();
    let events_before = std::fs::read(run.events_path()).unwrap();
    let execute_path =
        root.join("data/runs/recovery-evidence-fixture/execution-receipts/execute.json");
    let execute_before = std::fs::read(&execute_path).unwrap();
    let recovered = run.recover().unwrap();
    let start = recovered
        .events
        .iter()
        .rev()
        .find(|event| event.idempotency_key.contains(":provider-running:"))
        .unwrap();
    let node = NodeId::new("verify").unwrap();
    let outcome_before = run
        .read_recovery_outcome_evidence(&node, &start.idempotency_key)
        .unwrap();
    let phase = std::env::var("ARDA_RECOVERY_REOPEN_PHASE").unwrap();
    let after = phase != "before";
    let between = phase == "between";
    assert_eq!(outcome_before.is_some(), after);
    if between {
        assert_eq!(
            recovered.events.last().unwrap().idempotency_key,
            start.idempotency_key
        );
    }
    let state = HarnessState {
        harness_addr: "127.0.0.1:0".into(),
        child_pids: Arc::new(tokio::sync::RwLock::new(vec![])),
        service_names: Arc::new(vec![]),
        service_statuses: Arc::new(tokio::sync::RwLock::new(vec![])),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: super::super::super::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: super::super::super::DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: super::super::super::presence::HarnessPresenceState::default(),
        workbench_root: root.to_path_buf(),
        operator_id: "operator:fixture".into(),
    };
    let message = std::fs::read(root.join("restart-message.json")).unwrap();
    let jobs = super::super::super::RecoveryJobs::default();
    let mut settled_events = None;
    for _ in 0..2 {
        let mut headers = HeaderMap::new();
        headers.insert("x-arda-local-capability", "fixture-token".parse().unwrap());
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            ingest_local_operator_message(
                State(state.clone()),
                ConnectInfo("127.0.0.1:1234".parse().unwrap()),
                Some(axum::Extension(super::super::super::RuntimePrerequisites {
                    recovery_jobs: jobs.clone(),
                    keeper_socket: Some(root.join("keeper.sock")),
                })),
                headers,
                Json(serde_json::from_slice(&message).unwrap()),
            ),
        )
        .await
        .expect("reopen attempted execution instead of resolving original start");
        assert!(
            result.is_err(),
            "failed/missing outcome must not become success"
        );
        use axum::response::IntoResponse;
        let response = result.err().unwrap().into_response();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            error["message"],
            if after {
                "provider failed; exact-start failure evidence retained"
            } else {
                "node `verify` exhausted provider attempts"
            }
        );
        let _ = jobs.settle().await;
        let events = std::fs::read(run.events_path()).unwrap();
        if let Some(previous) = &settled_events {
            assert_eq!(&events, previous);
        }
        settled_events = Some(events);
    }
    assert_eq!(transport.chats.load(Ordering::SeqCst), 0);
    assert_eq!(transport.exports.load(Ordering::SeqCst), 0);
    assert_eq!(transport.releases.load(Ordering::SeqCst), 0);
    if between {
        let events = run.recover().unwrap().events;
        assert_eq!(events.len(), recovered.events.len() + 1);
        assert!(std::fs::read(run.events_path())
            .unwrap()
            .starts_with(&events_before));
        let error = events.last().unwrap();
        assert_eq!(
            error.idempotency_key,
            start
                .idempotency_key
                .replace(":provider-running:", ":provider-error:")
        );
        assert!(matches!(
            error.kind,
            RunEventKind::NodeTransition {
                state: arda_core::run_graph::NodeState::Failed
            }
        ));
    } else {
        assert_eq!(std::fs::read(run.events_path()).unwrap(), events_before);
    }
    assert_eq!(std::fs::read(execute_path).unwrap(), execute_before);
    assert_eq!(
        run.read_recovery_outcome_evidence(&node, &start.idempotency_key)
            .unwrap(),
        outcome_before
    );
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM recovery_publications", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    transport.stop().await;
}
