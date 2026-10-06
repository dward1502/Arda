use arda_engine::objectives::{
    keeper_client::{KeeperClient, KeeperFailure},
    RetainedSnapshot, SnapshotAdmission,
};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
};

fn release_response(response: &str) -> anyhow::Error {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("keeper.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let response = response.to_owned();
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(&stream).read_line(&mut request).unwrap();
        let request: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(request["op"], "release");
        writeln!(stream, "{response}").unwrap();
    });
    let result = KeeperClient::new(socket).release(
        &RetainedSnapshot {
            endpoint: "fixture".into(),
            capability: "fixture-only".into(),
            manifest_digest: "fixture".into(),
        },
        "run",
    );
    thread.join().unwrap();
    result.unwrap_err()
}
#[test]
fn only_explicit_missing_revoked_authority_routes_recovery() {
    let error =
        release_response(r#"{"ok":false,"snapshot":null,"failure":"missing_revoked_authority"}"#);
    assert_eq!(
        error.downcast_ref::<KeeperFailure>(),
        Some(&KeeperFailure::MissingRevokedAuthority)
    );
    for response in [
        r#"{"ok":false,"snapshot":null}"#,
        "not json",
        r#"{"ok":true,"snapshot":null,"failure":"missing_revoked_authority"}"#,
        r#"{"ok":false,"snapshot":{"endpoint":"x","capability":"x","manifest_digest":"x"},"failure":"missing_revoked_authority"}"#,
    ] {
        assert!(
            release_response(response)
                .downcast_ref::<KeeperFailure>()
                .is_none(),
            "{response}"
        );
    }
}
#[test]
fn abandonment_refusal_remains_typed_and_never_routes_terminal_recovery() {
    use arda_engine::objectives::keeper_client::KeeperRequest;
    use arda_engine::objectives::snapshot_protocol::Lease;
    let snapshot = RetainedSnapshot {
        endpoint: "unused".into(),
        capability: "fixture".into(),
        manifest_digest: "fixture".into(),
    };
    let requests = [
        KeeperRequest::Prepare {
            run: "abandoned".into(),
            workspace: "wrong".into(),
            identity: "wrong".into(),
        },
        KeeperRequest::Commit {
            snapshot: snapshot.clone(),
            lease: Lease {
                run_id: "abandoned".into(),
                generation: 3,
                owner: "fixture".into(),
                expires_ms: 100,
            },
        },
        KeeperRequest::Release {
            snapshot,
            run: "abandoned".into(),
        },
        KeeperRequest::QueryTerminalRevocation {
            run: "abandoned".into(),
            workspace: "wrong".into(),
            identity: "wrong".into(),
        },
    ];
    for request in requests {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("keeper.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let expected = serde_json::to_value(&request).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut input = String::new();
            BufReader::new(&stream).read_line(&mut input).unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&input).unwrap(),
                expected
            );
            writeln!(
                stream,
                "{{\"ok\":false,\"failure\":\"operator_abandoned\"}}"
            )
            .unwrap();
        });
        let client = KeeperClient::new(socket);
        let error = match request {
            KeeperRequest::Prepare {
                run,
                workspace,
                identity,
            } => client.prepare(&run, &workspace, &identity).err().unwrap(),
            KeeperRequest::Commit { snapshot, lease } => client
                .commit(
                    &snapshot,
                    &lease.run_id,
                    lease.generation,
                    &lease.owner,
                    lease.expires_ms,
                )
                .unwrap_err(),
            KeeperRequest::Release { snapshot, run } => {
                client.release(&snapshot, &run).unwrap_err()
            }
            KeeperRequest::QueryTerminalRevocation {
                run,
                workspace,
                identity,
            } => client
                .terminal_revocation(&run, &workspace, &identity)
                .err()
                .unwrap(),
        };
        server.join().unwrap();
        assert_eq!(
            error.downcast_ref::<KeeperFailure>(),
            Some(&KeeperFailure::OperatorAbandoned)
        );
        assert_ne!(
            error.downcast_ref::<KeeperFailure>(),
            Some(&KeeperFailure::MissingRevokedAuthority)
        );
    }
    for response in [
        r#"{"ok":true,"failure":"operator_abandoned"}"#,
        r#"{"ok":false,"snapshot":{"endpoint":"x","capability":"x","manifest_digest":"x"},"failure":"operator_abandoned"}"#,
    ] {
        assert!(release_response(response)
            .downcast_ref::<KeeperFailure>()
            .is_none());
    }
}

#[test]
fn receipt_query_rejects_missing_typed_payload() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("keeper.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    // Nonblocking listener keeps the test bounded even while default trait denies before transport.
    listener.set_nonblocking(true).unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::spawn(move || {
        while !flag.load(std::sync::atomic::Ordering::SeqCst) {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = String::new();
                BufReader::new(&stream).read_line(&mut request).unwrap();
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&request).unwrap()["op"],
                    "query_terminal_revocation"
                );
                writeln!(stream, "{{\"ok\":true,\"snapshot\":null}}").unwrap();
                return true;
            }
            std::thread::yield_now();
        }
        false
    });
    let result = KeeperClient::new(socket).terminal_revocation("run", "/historical", "identity");
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    let queried = thread.join().unwrap();
    assert!(result.is_err());
    assert!(
        queried,
        "production transport must execute the separate receipt query"
    );
}
