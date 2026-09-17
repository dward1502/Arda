use super::*;
use crate::objectives::runtime_operation::RuntimeOperation;
use std::sync::atomic::{AtomicBool, Ordering};

struct Gate {
    send: bool,
    fail_after: bool,
    cancel_before: Option<AdapterCancellation>,
    called: AtomicBool,
}
impl RecoveryDispatchGate for Gate {
    fn dispatch(
        &self,
        _: &RetainedExecution,
        _: &RuntimeOperation,
        delimiter: Box<dyn FnOnce() -> Result<(), HermesAdapterError> + '_>,
    ) -> Result<(), HermesAdapterError> {
        assert!(!self.called.swap(true, Ordering::SeqCst));
        if let Some(cancel) = &self.cancel_before {
            cancel.cancel();
        }
        if self.send {
            delimiter()?;
        }
        if !self.send || self.fail_after {
            return Err(HermesAdapterError::InvalidTask(
                "fixture fence denied".into(),
            ));
        }
        Ok(())
    }
}

async fn exercise(send: bool, fail_after: bool, acknowledge: bool, cancel_before: bool) {
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
    let cancel = AdapterCancellation::default();
    let gate = Gate {
        send,
        fail_after,
        cancel_before: cancel_before.then(|| cancel.clone()),
        called: AtomicBool::new(false),
    };
    let ack_sent = AtomicBool::new(false);
    let peer = async {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        let mut request = Vec::new();
        socket.read_until(b'\n', &mut request).await.unwrap();
        assert_eq!(request.last() == Some(&b'\n'), send && !cancel_before);
        if !send || cancel_before {
            return;
        }
        if fail_after {
            let mut extra = Vec::new();
            socket.read_to_end(&mut extra).await.unwrap();
            assert!(extra.is_empty());
            if !acknowledge {
                return;
            }
            tokio::task::yield_now().await;
        }
        let response = serde_json::json!({"ok": true, "stdout":"", "stderr":"", "code":0});
        socket
            .get_mut()
            .write_all(format!("{response}\n").as_bytes())
            .await
            .unwrap();
        ack_sent.store(true, Ordering::SeqCst);
    };
    let client = async {
        let operation = RuntimeOperation::Export {
            session_id: "fixture".into(),
        };
        let result = execute_inner(
            &binding,
            vec!["fixture".into()],
            BTreeMap::new(),
            &cancel,
            ExecutionLimits {
                duration: Duration::from_secs(2),
                grace_ms: 100,
                limit: 65536,
            },
            None,
            Some((&gate, &operation)),
        )
        .await;
        assert!(gate.called.load(Ordering::SeqCst), "launch bypassed fence");
        if cancel_before {
            assert!(matches!(result, Err(HermesAdapterError::Cancelled)));
        } else if send && fail_after && !acknowledge {
            assert!(matches!(result, Err(HermesAdapterError::ReapTimeout)));
        } else if !send || fail_after {
            assert!(matches!(result, Err(HermesAdapterError::InvalidTask(_))));
            if send {
                assert!(ack_sent.load(Ordering::SeqCst));
            }
        } else {
            assert!(result.is_ok());
        }
    };
    tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(peer, client);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn gate_rejection_never_sends_delimiter() {
    exercise(false, false, true, false).await;
}
#[tokio::test]
async fn gate_success_dispatches_once() {
    exercise(true, false, true, false).await;
}
#[tokio::test]
async fn gate_postsend_failure_waits_for_cleanup() {
    exercise(true, true, true, false).await;
}
#[tokio::test]
async fn gate_postsend_failure_without_ack_requires_reconciliation() {
    exercise(true, true, false, false).await;
}
#[tokio::test]
async fn gate_cancellation_during_validation_never_sends() {
    exercise(true, false, true, true).await;
}
