//! Connected authenticated saved-completion replay; all authority is synthetic.
use super::*;
#[path = "completion_fence_tests.rs"]
mod completion_fence_tests;
use crate::objectives::{ControlAction, NewObjective, ObjectiveStore, RetainedSnapshot};
use crate::runs::{RecoveryGrant, RunEventDraft, RunEventKind, RECOVERY_WINDOW_MS};
use arda_core::run_graph::{NodeId, NodeState};
use completion_fence_tests::check_pending_completion_fences;
use rusqlite::params;
use std::io::{BufRead, BufReader, Write};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authenticated_recovery_completion_replay_releases_only_exact_leaf() {
    // Isolate the capability environment from parallel unit tests.
    const TEST: &str = "harness::operator_messages::recovery_tests::authenticated_recovery_completion_replay_releases_only_exact_leaf";
    if std::env::var_os("ARDA_RECOVERY_INGRESS_TEST_CHILD").is_none() {
        for mode in [
            "success",
            "pending-control",
            "pending-journal",
            "applied-journal",
            "pending-resume",
            "applied-resume",
            "pending-resume-completed",
            "applied-resume-completed",
            "pending-resume-failed",
            "applied-resume-failed",
            "pending-node",
        ] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env("ARDA_RECOVERY_INGRESS_TEST_CHILD", "1")
                .env("ARDA_RECOVERY_CLEANUP_MODE", mode)
                .env("ARDA_HERMES_LOCAL_CAPABILITY", "synthetic-recovery-test")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{mode}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    let (temp, run, mut graph, mut bindings, execute) =
        crate::runs::recovery_evidence::tests::fixture();
    let root = temp.path();
    let now = Utc::now().timestamp_millis();
    let message = json!({
        "operator":{"operator_id":"operator:fixture","authenticated":true,
            "authentication_method":"local_session","authenticated_at":Utc::now().to_rfc3339()},
        "adapter_id":"hermes-cli",
        "event":{"text":format!("arda recover-retained {} {} {} one-verify-start 30m reuse-expired-context",
            bindings.objective_id,bindings.leaf_id,bindings.run_id.as_str()),
            "message_type":"text","user_id":"operator:fixture","user_name":"fixture",
            "source":{"platform":"cli","chat_id":"fixture-session","chat_type":"private",
                "thread_id":null,"message_id":"fixture-event"},
            "message_id":"fixture-event","media_urls":[],"media_types":[],
            "timestamp":Utc::now().to_rfc3339(),"prompt_response":null}
    });
    let mut incoming: GatewayOperatorMessage = serde_json::from_value(message.clone()).unwrap();
    let event = gateway_event_id(&incoming, "fixture-event");
    incoming.event.message_id = Some(event.clone());
    incoming.event.source.message_id = Some(event.clone());
    let payload = format!("{:x}", Sha256::digest(serde_json::to_vec(&json!({
        "operator_id":incoming.operator.operator_id,"adapter_id":incoming.adapter_id,"event":incoming.event
    })).unwrap()));
    let snapshot = RetainedSnapshot {
        endpoint: "fixture://no-provider".into(),
        capability: "synthetic-test".into(),
        manifest_digest: format!("sha256:{}", "a".repeat(64)),
    };
    bindings.retained_authority_digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&snapshot).unwrap())
    );
    let grant = RecoveryGrant {
        authenticated_event_id: event.clone(),
        authenticated_payload_digest: format!("sha256:{payload}"),
        bindings,
        activated_at_unix_ms: now as u64 - RECOVERY_WINDOW_MS - 60_000,
        expires_at_unix_ms: now as u64 - 60_000,
    };
    let b = &grant.bindings;
    let path = root.join("data/arda/objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let objective: NewObjective = serde_json::from_value(json!({
        "id":b.objective_id,"source_id":"fixture","idempotency_key":"fixture",
        "operator_id":"operator:fixture","text":"fixture","priority":1,
        "projects":[{"project_id":"project-a","contract_digest":b.project_contract_digest}],
        "leaves":[{"id":b.leaf_id,"project_id":"project-a","workspace_root":root,
            "authority":"read_only","dependencies":[],"execution":null},
            {"id":"sibling","project_id":"project-a","workspace_root":root,
            "authority":"read_only","dependencies":[],"execution":null}]
    }))
    .unwrap();
    store.create_authenticated_objective(objective, 1).unwrap();
    store
        .apply_control(
            &b.objective_id,
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator:fixture",
            2,
        )
        .unwrap();
    store
        .apply_control(
            &b.objective_id,
            ControlAction::Pause,
            "pause",
            "operator:fixture",
            3,
        )
        .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE leaves SET execution_run_id=?1,stage='execute',attempt=1 WHERE id=?2",
        params![b.run_id.as_str(), b.leaf_id],
    )
    .unwrap();
    store.bind_gateway_event(&event, &payload).unwrap();
    // Seed historical synthetic authority directly: a NEW admission after expiry
    // would correctly fail. This fixture starts at already-durable completion.
    db.execute("INSERT INTO recovery_admissions (authenticated_event_id,operator_id,objective_id,leaf_id,run_id,grant_json) VALUES (?1,'operator:fixture',?2,?3,?4,?5)",
        params![event,b.objective_id,b.leaf_id,b.run_id.as_str(),serde_json::to_string(&grant).unwrap()]).unwrap();
    run.activate_recovery(grant.clone()).unwrap();
    db.execute(
        "INSERT INTO retained_workspace_snapshots VALUES (?1,?2,?3,1)",
        params![
            b.leaf_id,
            b.run_id.as_str(),
            serde_json::to_string(&snapshot).unwrap()
        ],
    )
    .unwrap();
    db.execute("INSERT INTO retained_snapshot_lease_intents(leaf_id,generation,lease_owner,lease_expires_ms,recovery_event_id) VALUES (?1,1,'fixture-owner',?2,?3)",
        params![b.leaf_id,now-1,event]).unwrap();
    let mut parent: Option<String> = None;
    for stage in ["execute", "verify", "review", "close"] {
        let mut receipt = execute.clone();
        if stage != "execute" {
            receipt.node_id = stage.into();
            receipt.idempotency_key = format!("{stage}-fixture");
            receipt.parent_receipts = vec![parent.clone().unwrap()];
            receipt.receipt_digest = receipt.computed_digest().unwrap();
            run.write_execution_receipt(
                &NodeId::new(stage).unwrap(),
                &serde_json::to_value(&receipt).unwrap(),
            )
            .unwrap();
            run.append(RunEventDraft {
                node_id: NodeId::new(stage).unwrap(),
                idempotency_key: format!("{stage}-fixture-success"),
                kind: RunEventKind::NodeTransition {
                    state: NodeState::Succeeded,
                },
                receipt_digest: Some(receipt.receipt_digest.clone()),
            })
            .unwrap();
        }
        let node = graph
            .nodes
            .iter_mut()
            .find(|n| n.id.as_str() == stage)
            .unwrap();
        node.state = NodeState::Succeeded;
        node.output_digest = Some(receipt.receipt_digest.clone());
        node.parent_receipts = receipt.parent_receipts.clone();
        db.execute("INSERT INTO stage_receipts(leaf_id,stage,contract,digest,predecessor_digest,run_path,provider,model,started_at_ms,completed_at_ms,verdict,recorded_at_ms)
            VALUES (?1,?2,'arda.hermes_execution_receipt.v4',?3,?4,?5,'fixture','fixture',1,2,'fixture',3)",
            params![b.leaf_id,stage,receipt.receipt_digest,parent,format!("data/runs/{}/execution-receipts/{stage}.json",b.run_id.as_str())]).unwrap();
        parent = Some(receipt.receipt_digest);
    }
    run.write_checkpoint(&graph).unwrap();
    db.execute("UPDATE leaves SET stage='complete',current_receipt_digest=?1,lease_owner=NULL,lease_expires_ms=NULL WHERE id=?2",params![parent,b.leaf_id]).unwrap();
    check_pending_completion_fences(root, &store, &db, &grant);
    let mode = std::env::var("ARDA_RECOVERY_CLEANUP_MODE").unwrap();
    if mode.starts_with("pending") {
        // Restore the fixture's pre-ACK crash boundary; no real effect was made
        // by the sentinel callback used by the preceding store-fence matrix.
        db.execute("UPDATE recovery_publications SET applied_at_ms=NULL WHERE publication_key='completion'", []).unwrap();
    }
    if mode == "pending-control" {
        store
            .apply_control(
                &b.objective_id,
                ControlAction::Cancel,
                "later-cancel",
                "operator:fixture",
                now,
            )
            .unwrap();
    }
    if mode.contains("resume") {
        store
            .apply_control(
                &b.objective_id,
                ControlAction::Resume,
                "later-resume",
                "operator:fixture",
                now,
            )
            .unwrap();
    }
    if mode.ends_with("journal") {
        run.append(RunEventDraft {
            node_id: b.close_node_id.clone(),
            idempotency_key: "fixture-cancel-after-completion".into(),
            kind: RunEventKind::Cancelled {
                reason: "fixture cancellation".into(),
            },
            receipt_digest: None,
        })
        .unwrap();
    }
    if mode == "pending-node" {
        run.append(RunEventDraft {
            node_id: b.close_node_id.clone(),
            idempotency_key: "fixture-node-cancel".into(),
            kind: RunEventKind::NodeTransition {
                state: arda_core::run_graph::NodeState::Cancelled,
            },
            receipt_digest: None,
        })
        .unwrap();
    }
    // Count requests separately from idempotent remote effects: refusal,
    // successful release with lost ACK, then acknowledged retry.
    let socket = root.join("keeper.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let effects = Arc::new(AtomicUsize::new(0));
    let release_effects = effects.clone();
    let mut released = false;
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let count = calls.clone();
    let stopped = stop.clone();
    let expected = b.run_id.as_str().to_owned();
    let keeper = std::thread::spawn(move || {
        while !stopped.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                        .unwrap();
                    let mut line = String::new();
                    BufReader::new(stream.try_clone().unwrap())
                        .read_line(&mut line)
                        .unwrap();
                    let request: crate::objectives::keeper_client::KeeperRequest =
                        serde_json::from_str(&line).unwrap();
                    match request {
                        crate::objectives::keeper_client::KeeperRequest::Release {
                            run,
                            snapshot: actual,
                        } => {
                            assert_eq!(run, expected);
                            assert!(actual == snapshot);
                        }
                        _ => panic!("replay attempted a new admission"),
                    }
                    let prior = count.fetch_add(1, Ordering::SeqCst);
                    if prior > 0 && !released {
                        released = true;
                        release_effects.fetch_add(1, Ordering::SeqCst);
                    }
                    if prior == 1 {
                        // Drop the connection after release, before acknowledgment.
                        continue;
                    }
                    writeln!(stream, "{{\"ok\":{},\"snapshot\":null}}", prior > 0).unwrap();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(2))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    let state = HarnessState {
        research_store_policy: super::super::ResearchStorePolicy::Isolated,
        harness_addr: "127.0.0.1:0".into(),
        child_pids: Arc::new(tokio::sync::RwLock::new(vec![])),
        service_names: Arc::new(vec![]),
        service_statuses: Arc::new(tokio::sync::RwLock::new(vec![])),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: super::super::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: super::super::DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: super::super::presence::HarnessPresenceState::default(),
        workbench_root: root.to_path_buf(),
        operator_id: "operator:fixture".into(),
    };
    let before = serde_json::to_value(run.recover().unwrap().events).unwrap();
    let recovery_jobs = super::super::RecoveryJobs::default();
    let invoke = |value: Value, authorized: bool| {
        let state = state.clone();
        let socket = socket.clone();
        let recovery_jobs = recovery_jobs.clone();
        async move {
            let mut headers = HeaderMap::new();
            if authorized {
                headers.insert(
                    "x-arda-local-capability",
                    "synthetic-recovery-test".parse().unwrap(),
                );
            }
            ingest_local_operator_message(
                State(state),
                ConnectInfo("127.0.0.1:1234".parse().unwrap()),
                Some(axum::Extension(super::super::RuntimePrerequisites {
                    recovery_jobs,
                    keeper_socket: Some(socket),
                })),
                headers,
                Json(serde_json::from_value(value).unwrap()),
            )
            .await
        }
    };
    assert!(invoke(message.clone(), false).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // Synthetic fixture corruption must not cause suppression or keeper I/O.
    let original = std::fs::read(run.events_path()).unwrap();
    let mut events: Vec<crate::runs::RunEvent> = String::from_utf8(original.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    events
        .iter_mut()
        .find(|e| e.sequence == b.failed_event_sequence)
        .unwrap()
        .idempotency_key
        .push_str("-changed");
    let changed = events
        .iter()
        .map(|e| serde_json::to_string(e).unwrap() + "\n")
        .collect::<String>();
    std::fs::write(run.events_path(), changed).unwrap();
    assert!(invoke(message.clone(), true).await.is_err());
    std::fs::write(run.events_path(), original).unwrap();
    let checkpoint = std::fs::read(run.checkpoint_path()).unwrap();
    std::fs::remove_file(run.checkpoint_path()).unwrap();
    assert!(invoke(message.clone(), true).await.is_err());
    std::fs::write(run.checkpoint_path(), checkpoint).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM recovery_completion_suppressions",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(
        invoke(message.clone(), true).await.is_err(),
        "keeper refusal must not claim success"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let suppressions: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM recovery_completion_suppressions",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(suppressions, i64::from(mode.starts_with("pending")));
    // Fixture-only root terminalization after the first release attempt:
    // retry must survive this state change without broadening leaf authority.
    if mode.ends_with("completed") || mode.ends_with("failed") {
        let terminal = if mode.ends_with("completed") {
            "completed"
        } else {
            "failed"
        };
        db.execute(
            "UPDATE objectives SET state=?1 WHERE id=?2",
            params![terminal, b.objective_id],
        )
        .unwrap();
    }
    if mode == "pending-journal" || mode == "pending-node" {
        store
            .reconcile_recovery_publications(root, "operator:fixture", &event, |_, _| {
                panic!("generic selector replayed suppression")
            })
            .unwrap();
    }
    if mode.starts_with("pending") {
        assert!(store
            .reconcile_recovery_completion(root, "operator:fixture", &event, |_| panic!(
                "suppressed effects replayed"
            ))
            .is_err());
        assert!(db.execute("UPDATE recovery_publications SET applied_at_ms=1 WHERE publication_key='completion'", []).is_err());
    }
    assert!(
        invoke(message.clone(), true).await.is_err(),
        "lost release ACK must remain pending"
    );
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM retained_snapshot_releases WHERE leaf_id=?1",
            [&b.leaf_id],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let response = invoke(message.clone(), true).await.unwrap().0;
    assert_eq!(
        response
            .summary
            .contains("after cancellation or supersession"),
        mode != "success"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(invoke(message.clone(), true).await.is_ok());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "completed replay released twice"
    );
    let mut changed = message.clone();
    changed["event"]["user_name"] = json!("changed-valid-payload");
    assert!(invoke(changed, true).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        effects.load(Ordering::SeqCst),
        1,
        "ACK loss repeated the remote effect"
    );
    stop.store(true, Ordering::SeqCst);
    keeper.join().unwrap();
    assert_eq!(
        serde_json::to_value(run.recover().unwrap().events).unwrap(),
        before,
        "replay added provider or journal activity"
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM retained_snapshot_releases WHERE leaf_id=?1",
            [&b.leaf_id],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT state FROM objectives WHERE id=?1",
            [&b.objective_id],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        if mode == "pending-control" {
            "cancelled"
        } else if mode.ends_with("completed") {
            "completed"
        } else if mode.ends_with("failed") {
            "failed"
        } else if mode.contains("resume") {
            "approved"
        } else {
            "paused"
        }
    );
    assert_eq!(
        db.query_row("SELECT attempt FROM leaves WHERE id='sibling'", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}
