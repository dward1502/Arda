use super::*;
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn backpressured_request_cancels_without_dispatch_delimiter() {
    let temp = tempfile::tempdir().unwrap();
    let endpoint = temp.path().join("worker.sock");
    let listener = tokio::net::UnixListener::bind(&endpoint).unwrap();
    let binding = RetainedExecution {
        snapshot: crate::objectives::RetainedSnapshot {
            endpoint: endpoint.to_str().unwrap().into(),
            capability: "fixture".into(),
            manifest_digest: "fixture".into(),
        },
        lease: wire::Lease {
            run_id: "fixture".into(),
            generation: 1,
            owner: "fixture".into(),
            expires_ms: i64::MAX,
        },
    };
    let cancellation = AdapterCancellation::default();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
    let peer = async {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut first = [0];
        socket.read_exact(&mut first).await.unwrap();
        started_tx.send(()).unwrap();
        // Do not drain until the cancelled client has returned. This forces a
        // request larger than the Unix send buffer to remain incomplete.
        finished_rx.await.unwrap();
        let mut bytes = first.to_vec();
        socket.read_to_end(&mut bytes).await.unwrap();
        assert!(!bytes.contains(&b'\n'), "cancelled request was dispatched");
        assert!(
            bytes.len() < 1_900_000,
            "fixture did not force backpressure"
        );
    };
    let cancel = async {
        started_rx.await.unwrap();
        cancellation.cancel();
    };
    let client = async {
        let result = execute(
            &binding,
            vec!["x".repeat(1_900_000)],
            BTreeMap::new(),
            Duration::from_secs(30),
            &cancellation,
            100,
            65536,
        )
        .await;
        assert!(matches!(result, Err(HermesAdapterError::Cancelled)));
        finished_tx.send(()).unwrap();
    };
    tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(peer, cancel, client);
    })
    .await
    .expect("cancellation waited for execution timeout");
}
