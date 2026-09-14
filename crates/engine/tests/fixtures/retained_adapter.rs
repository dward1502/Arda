use super::*;
use arda_engine::objectives::{snapshot_protocol::Lease, RetainedExecution, RetainedSnapshot};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Worker(std::process::Child);
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
