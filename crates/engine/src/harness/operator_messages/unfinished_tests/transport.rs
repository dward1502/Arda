use super::*;
use crate::objectives::{
    keeper_client::KeeperRequest,
    runtime_operation::RuntimeOperation,
    snapshot_protocol::{Lease, Request},
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};

pub(super) struct Transport {
    pub chats: Arc<AtomicUsize>,
    pub commits: Arc<AtomicUsize>,
    pub releases: Arc<AtomicUsize>,
    pub exports: Arc<AtomicUsize>,
    pub drop_started: Arc<tokio::sync::Notify>,
    pub drop_closed: Arc<tokio::sync::Notify>,
    pub worker_go: Arc<tokio::sync::Notify>,

    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Transport {
    pub async fn stop(self) {
        for task in self.tasks {
            task.abort();
            if let Err(error) = task.await {
                assert!(error.is_cancelled(), "{error}");
            }
        }
    }
}

pub(super) fn start(
    root: &std::path::Path,
    snapshot: &RetainedSnapshot,
    manifest: Manifest,
) -> Transport {
    if std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD")
        .is_ok_and(|mode| mode.starts_with("waiter-loss-crash-"))
    {
        std::fs::write(
            root.join("restart-transport.json"),
            serde_json::to_vec(&json!({"manifest": manifest, "snapshot": snapshot})).unwrap(),
        )
        .unwrap();
    }
    let worker = UnixListener::bind(root.join("worker.sock")).unwrap();
    let keeper = UnixListener::bind(root.join("keeper.sock")).unwrap();
    let lease: Arc<Mutex<Option<Lease>>> = Arc::new(Mutex::new(None));
    let keeper_lease = lease.clone();
    let commits = Arc::new(AtomicUsize::new(0));
    let releases = Arc::new(AtomicUsize::new(0));
    let commit_count = commits.clone();
    let release_count = releases.clone();
    let snapshot = snapshot.clone();
    let expected = snapshot.clone();

    let keeper_task = tokio::spawn(async move {
        loop {
            let (stream, _) = keeper.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            match serde_json::from_str::<KeeperRequest>(&line).unwrap() {
                KeeperRequest::Commit { snapshot, lease } => {
                    commit_count.fetch_add(1, Ordering::SeqCst);
                    assert!(snapshot == expected);
                    assert_eq!(lease.run_id, "recovery-evidence-fixture");
                    if std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD").as_deref()
                        == Ok("waiter-loss-lease-expiry")
                    {
                        assert!([2, 3].contains(&lease.generation));
                    } else {
                        assert_eq!(lease.generation, 2);
                    }
                    *keeper_lease.lock().unwrap() = Some(lease);
                }
                KeeperRequest::Release { snapshot, run } => {
                    release_count.fetch_add(1, Ordering::SeqCst);
                    *keeper_lease.lock().unwrap() = None;
                    assert!(snapshot == expected);
                    assert_eq!(run, "recovery-evidence-fixture");
                }
                KeeperRequest::Prepare { .. } => panic!("recovery recaptured authority"),
                KeeperRequest::QueryTerminalRevocation { .. } => {
                    panic!("live recovery queried terminal revocation")
                }
            }
            stream
                .get_mut()
                .write_all(b"{\"ok\":true,\"snapshot\":null}\n")
                .await
                .unwrap();
        }
    });

    let chats = Arc::new(AtomicUsize::new(0));
    let count = chats.clone();
    let exports = Arc::new(AtomicUsize::new(0));
    let export_count = exports.clone();
    let root = root.to_owned();

    let drop_mode =
        std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD").as_deref() == Ok("drop-characterization");
    let waiter_loss_mode = std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD")
        .is_ok_and(|mode| mode.starts_with("waiter-loss-"));

    let drop_started = Arc::new(tokio::sync::Notify::new());
    let drop_closed = Arc::new(tokio::sync::Notify::new());
    let started = drop_started.clone();
    let closed = drop_closed.clone();
    let worker_go = Arc::new(tokio::sync::Notify::new());
    let worker_go_for_struct = worker_go.clone();

    let worker_task = tokio::spawn(async move {
        if drop_mode || waiter_loss_mode {
            loop {
                let (stream, _) = worker.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                stream.read_line(&mut line).await.unwrap();
                let response = match serde_json::from_str::<Request>(&line).unwrap() {
                    Request::Inspect => json!({
                        "ok": true,
                        "manifest": manifest,
                        "manifest_digest": snapshot.manifest_digest,
                    }),
                    Request::Runtime {
                        capability,
                        lease: actual,
                        operation,
                        ..
                    } => {
                        assert_eq!(capability, snapshot.capability);
                        assert!(lease.lock().unwrap().as_ref() == Some(&actual));
                        let args = match operation {
                            RuntimeOperation::Chat {
                                query,
                                toolsets,
                                workspace_writable,
                                ..
                            } => {
                                assert!(!workspace_writable);
                                assert_eq!(toolsets, vec!["file"]);
                                let node: serde_json::Value = serde_json::Deserializer::from_str(
                                    query
                                        .split("Canonical node context follows:\n")
                                        .nth(1)
                                        .unwrap(),
                                )
                                .into_iter()
                                .next()
                                .unwrap()
                                .unwrap();
                                let index = count.fetch_add(1, Ordering::SeqCst);
                                assert_eq!(
                                    node["node"]["kind"],
                                    if index == 0 { "verify" } else { "review" }
                                );
                                assert!(index < 2);

                                if drop_mode {
                                    assert_eq!(index, 0);
                                    started.notify_one();
                                    let mut extra = String::new();
                                    let eof = tokio::time::timeout(
                                        std::time::Duration::from_secs(10),
                                        stream.read_line(&mut extra),
                                    )
                                    .await
                                    .unwrap()
                                    .unwrap();
                                    assert_eq!(
                                        eof, 0,
                                        "dropped ingress must close retained transport"
                                    );
                                    closed.notify_one();
                                    continue;
                                }

                                if waiter_loss_mode {
                                    assert_eq!(index, 0);
                                    started.notify_one();
                                    worker_go.notified().await;
                                    if !std::env::var("ARDA_UNFINISHED_RECOVERY_CHILD")
                                        .unwrap()
                                        .ends_with("-success")
                                    {
                                        let output = tokio::process::Command::new("/bin/sh")
                                            .arg("-c")
                                            .arg("exit 71")
                                            .output()
                                            .await
                                            .expect("synthetic error child");
                                        assert_eq!(output.status.code(), Some(71));
                                        let response = json!({
                                            "ok": true, "timed_out": false,
                                            "cancelled": false, "output_limit": false,
                                            "stdout": "", "stderr": "", "code": output.status.code(),
                                            "error": null,
                                        });
                                        stream
                                            .get_mut()
                                            .write_all(
                                                (serde_json::to_string(&response).unwrap() + "\n")
                                                    .as_bytes(),
                                            )
                                            .await
                                            .unwrap();
                                        let mut extra = String::new();
                                        let eof = tokio::time::timeout(
                                            std::time::Duration::from_secs(10),
                                            stream.read_line(&mut extra),
                                        )
                                        .await
                                        .unwrap()
                                        .unwrap();
                                        assert_eq!(
                                            eof, 0,
                                            "waiter-loss must close retained transport"
                                        );
                                        closed.notify_one();
                                        continue;
                                    }
                                }

                                vec!["-q".to_owned(), query]
                            }
                            RuntimeOperation::Export { session_id } => {
                                export_count.fetch_add(1, Ordering::SeqCst);
                                vec![
                                    "sessions".into(),
                                    "export".into(),
                                    "-".into(),
                                    "--session-id".into(),
                                    session_id,
                                ]
                            }
                            _ => panic!("unexpected operation"),
                        };
                        let output = tokio::process::Command::new(root.join("fake-hermes-context"))
                            .current_dir(&root)
                            .args(args)
                            .output()
                            .await
                            .unwrap();
                        assert!(
                            output.status.success(),
                            "{}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        json!({
                            "ok": true,
                            "timed_out": false,
                            "cancelled": false,
                            "output_limit": false,
                            "stdout": String::from_utf8(output.stdout).unwrap(),
                            "stderr": "",
                            "code": 0,
                            "error": null,
                        })
                    }
                    _ => panic!("unexpected worker request"),
                };
                stream
                    .get_mut()
                    .write_all((serde_json::to_string(&response).unwrap() + "\n").as_bytes())
                    .await
                    .unwrap();
            }
        } else {
            loop {
                let (stream, _) = worker.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut line = String::new();
                stream.read_line(&mut line).await.unwrap();
                let response = match serde_json::from_str::<Request>(&line).unwrap() {
                    Request::Inspect => json!({
                        "ok": true,
                        "manifest": manifest,
                        "manifest_digest": snapshot.manifest_digest,
                    }),
                    Request::Runtime {
                        capability,
                        lease: actual,
                        operation,
                        ..
                    } => {
                        assert_eq!(capability, snapshot.capability);
                        assert!(lease.lock().unwrap().as_ref() == Some(&actual));
                        let args = match operation {
                            RuntimeOperation::Chat {
                                query,
                                toolsets,
                                workspace_writable,
                                ..
                            } => {
                                assert!(!workspace_writable);
                                assert_eq!(toolsets, vec!["file"]);
                                let node: serde_json::Value = serde_json::Deserializer::from_str(
                                    query
                                        .split("Canonical node context follows:\n")
                                        .nth(1)
                                        .unwrap(),
                                )
                                .into_iter()
                                .next()
                                .unwrap()
                                .unwrap();
                                let index = count.fetch_add(1, Ordering::SeqCst);
                                assert_eq!(
                                    node["node"]["kind"],
                                    if index == 0 { "verify" } else { "review" }
                                );
                                assert!(index < 2);
                                vec!["-q".to_owned(), query]
                            }
                            RuntimeOperation::Export { session_id } => {
                                export_count.fetch_add(1, Ordering::SeqCst);
                                vec![
                                    "sessions".into(),
                                    "export".into(),
                                    "-".into(),
                                    "--session-id".into(),
                                    session_id,
                                ]
                            }
                            _ => panic!("unexpected operation"),
                        };
                        let output = tokio::process::Command::new(root.join("fake-hermes-context"))
                            .current_dir(&root)
                            .args(args)
                            .output()
                            .await
                            .unwrap();
                        assert!(
                            output.status.success(),
                            "{}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        json!({
                            "ok": true,
                            "timed_out": false,
                            "cancelled": false,
                            "output_limit": false,
                            "stdout": String::from_utf8(output.stdout).unwrap(),
                            "stderr": "",
                            "code": 0,
                            "error": null,
                        })
                    }
                    _ => panic!("unexpected worker request"),
                };
                stream
                    .get_mut()
                    .write_all((serde_json::to_string(&response).unwrap() + "\n").as_bytes())
                    .await
                    .unwrap();
            }
        }
    });

    Transport {
        chats,
        commits,
        releases,
        exports,
        drop_started,
        drop_closed,
        worker_go: worker_go_for_struct,

        tasks: vec![keeper_task, worker_task],
    }
}
