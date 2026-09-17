use super::*;
use arda_engine::objectives::{snapshot_protocol::Lease, RetainedExecution, RetainedSnapshot};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Worker(std::process::Child);
#[tokio::test]
async fn file_only_verifier_dispatches_readonly_retained_chat() {
    assert_file_only_dispatch(false, false).await;
}

#[tokio::test]
async fn declared_checks_cannot_make_file_only_verifier_writable() {
    assert_file_only_dispatch(true, false).await;
}

#[tokio::test]
async fn recovery_dispatch_uses_expired_context_and_worker_without_mutating_them() {
    assert_file_only_dispatch(false, true).await;
}

async fn assert_file_only_dispatch(with_checks: bool, recovery: bool) {
    use arda_engine::adapters::RecoveryDispatchGate;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct CountGate(AtomicUsize);
    impl RecoveryDispatchGate for CountGate {
        fn dispatch(
            &self,
            _: &arda_engine::objectives::RetainedExecution,
            operation: &arda_engine::objectives::runtime_operation::RuntimeOperation,
            delimiter: Box<dyn FnOnce() -> Result<(), HermesAdapterError> + '_>,
        ) -> Result<(), HermesAdapterError> {
            assert!(matches!(
                operation,
                arda_engine::objectives::runtime_operation::RuntimeOperation::Chat {
                    workspace_writable: false,
                    ..
                }
            ));
            self.0.fetch_add(1, Ordering::SeqCst);
            delimiter()
        }
    }
    let gate = Arc::new(CountGate(AtomicUsize::new(0)));
    use arda_engine::objectives::snapshot_protocol::{Manifest, RuntimeBundle};
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let temp = TempDir::new().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let grant = recovery_window::grant(
        &review_task().run_id,
        &review_task().node.id,
        &NodeId::new("recovery-review").unwrap(),
        now,
    );
    let socket = temp.path().join("worker.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let manifest = Manifest {
        version: 2,
        capability: "fixture".into(),
        root: temp.path().into(),
        device: 1,
        inode: 1,
        topology_digest: "fixture".into(),
        runtime_bundle: Some(RuntimeBundle {
            version: 1,
            policy_digest: "fixture".into(),
            grants: vec![],
        }),
        admission_digest: Some("a".repeat(64)),
    };
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&manifest).unwrap())
    );
    let binding = RetainedExecution {
        snapshot: RetainedSnapshot {
            endpoint: socket.to_str().unwrap().into(),
            capability: "fixture".into(),
            manifest_digest: digest.clone(),
        },
        lease: Lease {
            run_id: "run-hermes-contract".into(),
            generation: 1,
            owner: "fixture".into(),
            expires_ms: if recovery {
                (now + 60_000) as i64
            } else {
                i64::MAX
            },
        },
    };
    let config = write_config(temp.path());
    let raw = fs::read_to_string(&config)
        .unwrap()
        .replace("executable = \"hermes\"", "executable = \"/usr/bin/true\"");
    fs::write(&config, raw).unwrap();
    let load_adapter = || {
        HermesAdapter::load_retained(
            &config,
            temp.path(),
            &BTreeMap::new(),
            RetainedExecution {
                snapshot: binding.snapshot.clone(),
                lease: binding.lease.clone(),
            },
        )
        .unwrap()
    };
    let mut adapter = load_adapter();
    let mut node_task = review_task();
    node_task.node.kind = NodeKind::Verify;
    node_task.node.authority = AuthorityClass::Verify;
    node_task.node.worker.as_mut().unwrap().role = WorkerRole::IndependentVerifier;
    node_task.node.worker.as_mut().unwrap().evidence_policy = EvidencePolicy::ProjectNativeChecks;
    node_task.checks.clear();
    node_task.check_commands.clear();
    if with_checks {
        node_task.checks.push("test".into());
        node_task
            .check_commands
            .insert("test".into(), "cargo test".into());
    }
    node_task.node.worker.as_mut().unwrap().allowed_toolsets =
        ["file".into()].into_iter().collect();
    if recovery {
        node_task.node.worker.as_mut().unwrap().deadline_unix_ms = (now - 60_000).into();
        node_task.project_contract_digest = grant.bindings.project_contract_digest.clone();
        node_task.context_assembly = Some(context_assembly_at(
            temp.path(),
            &node_task,
            (now - 120_000).into(),
        ));
        assert!(adapter.preflight(&node_task).is_err());
        let memory = || {
            MnemosyneService::new(temp.path().join("vaire"))
                .unwrap()
                .with_contract_memory_root(temp.path().join("memory"))
        };
        for (kind, authority, id) in [
            (
                NodeKind::Verify,
                AuthorityClass::ReadOnly,
                grant.bindings.verify_node_id.clone(),
            ),
            (
                NodeKind::Review,
                AuthorityClass::Verify,
                grant.bindings.review_node_id.clone(),
            ),
            (
                NodeKind::Review,
                AuthorityClass::ExecuteWithApproval,
                grant.bindings.review_node_id.clone(),
            ),
        ] {
            let mut malformed = node_task.clone();
            malformed.node.kind = kind;
            malformed.node.authority = authority;
            malformed.node.id = id;
            let Err(error) =
                load_adapter().with_recovery_window(&malformed, grant.clone(), memory())
            else {
                panic!("malformed recovery authority accepted");
            };
            assert!(error.to_string().contains("read-only scope"));
        }
        adapter = adapter
            .with_recovery_window(&node_task, grant, memory())
            .unwrap()
            .with_recovery_dispatch_gate(gate.clone())
            .unwrap();
        adapter.preflight(&node_task).unwrap();
        let mut changed = node_task.clone();
        changed.instructions.push_str(" changed");
        assert!(adapter.preflight(&changed).is_err());
        changed = node_task.clone();
        changed.context_assembly = None;
        assert!(adapter.preflight(&changed).is_err());
    }
    let original_task = serde_json::to_vec(&node_task).unwrap();
    let peer = async {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut line = String::new();
        tokio::io::BufReader::new(&mut stream)
            .read_line(&mut line)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).unwrap()["op"],
            "inspect"
        );
        let response = serde_json::json!({"ok":true,"manifest":manifest,"manifest_digest":digest});
        stream
            .write_all(format!("{response}\n").as_bytes())
            .await
            .unwrap();
        drop(stream);
        let (mut stream, _) = listener.accept().await.unwrap();
        line.clear();
        tokio::io::BufReader::new(&mut stream)
            .read_line(&mut line)
            .await
            .unwrap();
        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["op"], "runtime");
        assert_eq!(request["operation"]["workspace_writable"], false);
        assert_eq!(
            request["operation"]["toolsets"],
            serde_json::json!(["file"])
        );
        // Deliberately stop after observing dispatch; no fabricated work receipt.
        stream.write_all(b"{\"ok\":false}\n").await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        let (result, ()) = tokio::join!(
            adapter.execute(&node_task, AdapterCancellation::default()),
            peer
        );
        assert!(result.is_err());
    })
    .await
    .unwrap();
    assert_eq!(serde_json::to_vec(&node_task).unwrap(), original_task);
    assert_eq!(gate.0.load(Ordering::SeqCst), usize::from(recovery));
    if recovery {
        use arda_core::contract::MemoryState;
        let memory = MnemosyneService::new(temp.path().join("vaire"))
            .unwrap()
            .with_contract_memory_root(temp.path().join("memory"));
        let mut consumer =
            ConsumerContext::new("hermes:fresh-context-worker", vec![MemoryDomain::System]);
        consumer.purpose = Some(node_task.objective.clone());
        let mut record = memory
            .recall_governed_memories(Some(&consumer))
            .unwrap()
            .remove(0);
        record.state = MemoryState::Revoked;
        memory
            .write_governed_memory(record, Some(&consumer))
            .unwrap();
        assert!(adapter.preflight(&node_task).is_err());
        assert!(adapter
            .execute(&node_task, AdapterCancellation::default())
            .await
            .is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(20), listener.accept())
                .await
                .is_err()
        );
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn request(path: &Path, value: serde_json::Value) -> serde_json::Value {
    let mut socket = UnixStream::connect(path).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    writeln!(socket, "{value}").unwrap();
    let mut line = String::new();
    BufReader::new(socket).read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[tokio::test]
#[ignore = "requires user/mount namespaces and bubblewrap"]
async fn retained_chat_and_export_use_original_tree_after_root_replacement() {
    let parent = TempDir::new().unwrap();
    let root = parent.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let executable = write_fake_hermes(&root);
    let script = fs::read_to_string(&executable).unwrap().replace(
        "\"artifacts\": [],",
        "\"artifacts\": [{\"path\": \"artifact.txt\", \"digest\": \"sha256:\" + __import__('hashlib').sha256(b'original').hexdigest()}],",
    );
    fs::write(&executable, script).unwrap();
    fs::write(root.join("artifact.txt"), b"original").unwrap();
    let config = write_config(parent.path());
    let raw = fs::read_to_string(&config)
        .unwrap()
        .replace(
            "executable = \"hermes\"",
            &format!("executable = {:?}", executable.display().to_string()),
        )
        .replace(
            "cancellation_grace_ms = 100",
            "cancellation_grace_ms = 5000",
        );
    fs::write(&config, raw).unwrap();
    let scratch = tempfile::Builder::new()
        .prefix("arda-adapter-")
        .tempdir_in("/dev/shm")
        .unwrap();
    fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let state = scratch.path().join("worker");
    let socket = state.join("control.sock");
    let mut worker = Worker(
        Command::new(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
            .arg(&state)
            .arg(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(
            worker.0.try_wait().unwrap().is_none(),
            "worker exited before ready"
        );
        assert!(Instant::now() < deadline, "worker startup deadline");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let inspection = request(&socket, serde_json::json!({"op":"inspect"}));
    assert_eq!(inspection["ok"], true);
    let snapshot = RetainedSnapshot {
        endpoint: socket.to_str().unwrap().into(),
        capability: inspection["manifest"]["capability"]
            .as_str()
            .unwrap()
            .into(),
        manifest_digest: inspection["manifest_digest"].as_str().unwrap().into(),
    };
    let lease = Lease {
        run_id: "run-hermes-contract".into(),
        generation: 1,
        owner: "fixture".into(),
        expires_ms: chrono::Utc::now().timestamp_millis() + 30000,
    };
    assert_eq!(
        request(
            &socket,
            serde_json::json!({"op":"commit", "capability":snapshot.capability,
        "manifest_digest":snapshot.manifest_digest, "lease":lease})
        )["ok"],
        true
    );
    let environment = host_environment(&root, "success");
    let adapter = HermesAdapter::load_retained(
        &config,
        &root,
        &environment,
        RetainedExecution {
            snapshot: snapshot.clone(),
            lease: lease.clone(),
        },
    )
    .unwrap();
    let original = parent.path().join("original");
    fs::rename(&root, &original).unwrap();
    fs::create_dir(&root).unwrap();
    let receipt = adapter
        .execute(&task(1000), AdapterCancellation::default())
        .await
        .unwrap();
    assert_eq!(receipt.status, HermesReceiptStatus::Succeeded);
    assert!(original.join("capture.json").is_file());
    assert!(original.join("transcript.json").is_file());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    let reopened = HermesAdapter::load_retained(
        &config,
        &root,
        &environment,
        RetainedExecution {
            snapshot: snapshot.clone(),
            lease,
        },
    )
    .unwrap();
    assert!(!format!("{reopened:?}").contains(&snapshot.capability));
    assert_eq!(
        reopened
            .execute(&task(1000), AdapterCancellation::default())
            .await
            .unwrap()
            .status,
        HermesReceiptStatus::Succeeded
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::write(original.join("artifact.txt"), b"tampered").unwrap();
    assert!(matches!(
        reopened
            .execute(&task(1000), AdapterCancellation::default())
            .await,
        Err(HermesAdapterError::ProcessFailed { .. })
    ));
    fs::remove_file(original.join("artifact.txt")).unwrap();
    std::os::unix::fs::symlink("/usr/bin/true", original.join("artifact.txt")).unwrap();
    assert!(matches!(
        reopened
            .execute(&task(1000), AdapterCancellation::default())
            .await,
        Err(HermesAdapterError::ProcessFailed { .. })
    ));
    assert_eq!(
        request(
            &socket,
            serde_json::json!({"op":"release", "capability":snapshot.capability})
        )["ok"],
        true
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while worker.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "worker release deadline");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!state.exists());
}
