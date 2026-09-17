//! Adversarial fixture histories, not permitted production journal writes.
use super::*;

fn activated() -> (
    tempfile::TempDir,
    RunStore,
    RunGraph,
    RecoveryBindings,
    HermesExecutionReceipt,
) {
    let result = fixture();
    result
        .1
        .activate_recovery(RecoveryGrant {
            authenticated_event_id: "fixture-replay-regression".into(),
            authenticated_payload_digest: format!("sha256:{}", "b".repeat(64)),
            bindings: result.3.clone(),
            activated_at_unix_ms: 1_000,
            expires_at_unix_ms: 1_000 + RECOVERY_WINDOW_MS,
        })
        .unwrap();
    result
}

fn append_fixture_event(events: &mut Vec<RunEvent>, node: &str, kind: RunEventKind) {
    let mut event = events[0].clone();
    event.sequence = events.last().unwrap().sequence + 1;
    event.node_id = NodeId::new(node).unwrap();
    event.idempotency_key = format!("adversarial-fixture-{}", event.sequence);
    event.receipt_digest = None;
    event.kind = kind;
    events.push(event);
}

fn save_fixture_events(store: &RunStore, events: &[RunEvent]) {
    let mut bytes = Vec::new();
    for event in events {
        bytes.extend(serde_json::to_vec(event).unwrap());
        bytes.push(b'\n');
    }
    std::fs::write(store.events_path(), bytes).unwrap();
}

#[test]
fn standalone_run_cancellation_is_rejected_from_canonical_files() {
    let (_temp, store, graph, bindings, receipt) = fixture();
    let mut events = store.recover().unwrap().events;
    append_fixture_event(
        &mut events,
        "verify",
        RunEventKind::Cancelled {
            reason: "fixture cancellation".into(),
        },
    );
    assert!(validate_history(&graph, &events, &receipt, &bindings).is_err());
    save_fixture_events(&store, &events);
    assert!(store.validate_recovery_run_evidence(&bindings).is_err());
}

#[test]
fn success_followed_by_running_is_rejected_even_within_start_ceiling() {
    for node in ["verify", "review"] {
        let (_temp, store, graph, bindings, receipt) = activated();
        let mut events = store.recover().unwrap().events;
        if node == "review" {
            append_fixture_event(
                &mut events,
                node,
                RunEventKind::NodeTransition {
                    state: NodeState::Running,
                },
            );
        }
        for state in [NodeState::Succeeded, NodeState::Ready, NodeState::Running] {
            append_fixture_event(&mut events, node, RunEventKind::NodeTransition { state });
        }
        assert!(
            validate_history(&graph, &events, &receipt, &bindings).is_err(),
            "accepted success replay for {node}"
        );
        save_fixture_events(&store, &events);
        assert!(store.validate_recovery_run_evidence(&bindings).is_err());
    }
}

#[test]
fn successful_forward_progress_without_restart_remains_valid() {
    let (_temp, store, graph, bindings, receipt) = activated();
    let mut events = store.recover().unwrap().events;
    for node in ["verify", "review"] {
        for state in [NodeState::Running, NodeState::Succeeded] {
            append_fixture_event(&mut events, node, RunEventKind::NodeTransition { state });
        }
    }
    append_fixture_event(
        &mut events,
        "close",
        RunEventKind::NodeTransition {
            state: NodeState::Succeeded,
        },
    );
    validate_history(&graph, &events, &receipt, &bindings).unwrap();
    save_fixture_events(&store, &events);
    store.validate_recovery_run_evidence(&bindings).unwrap();
}
