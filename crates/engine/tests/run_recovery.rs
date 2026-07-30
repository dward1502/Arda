use arda_core::run_graph::{NodeId, NodeState, RunId};
use arda_engine::runs::{AppendOutcome, RunEventDraft, RunEventKind, RunStore, RunStoreError};

fn draft(key: &str, state: NodeState) -> RunEventDraft {
    RunEventDraft {
        node_id: NodeId::new("execute").unwrap(),
        idempotency_key: key.to_string(),
        kind: RunEventKind::NodeTransition { state },
        receipt_digest: Some("sha256:receipt".to_string()),
    }
}

#[test]
fn restart_after_mutation_receipt_does_not_repeat_idempotency_key() {
    let temp = tempfile::tempdir().unwrap();
    let run_id = RunId::new("restartable").unwrap();
    let store = RunStore::open(temp.path(), run_id.clone()).unwrap();
    assert_eq!(
        store
            .append(draft("mutation-1", NodeState::Succeeded))
            .unwrap(),
        AppendOutcome::Appended { sequence: 1 }
    );

    drop(store); // crash/restart before any checkpoint or result projection

    let reopened = RunStore::open(temp.path(), run_id).unwrap();
    assert_eq!(
        reopened
            .append(draft("mutation-1", NodeState::Succeeded))
            .unwrap(),
        AppendOutcome::AlreadyApplied { sequence: 1 }
    );
    assert_eq!(reopened.recover().unwrap().events.len(), 1);
}

#[test]
fn corrupt_or_truncated_journal_tail_fails_visibly() {
    let temp = tempfile::tempdir().unwrap();
    let run_id = RunId::new("corrupt").unwrap();
    let store = RunStore::open(temp.path(), run_id.clone()).unwrap();
    store
        .append(draft("mutation-1", NodeState::Succeeded))
        .unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(store.events_path())
        .unwrap()
        .write_all(b"{\"schema_version\":")
        .unwrap();

    let reopened = RunStore::open(temp.path(), run_id).unwrap();
    assert!(matches!(
        reopened.recover(),
        Err(RunStoreError::CorruptJournal { line: 2, .. })
    ));
}

#[test]
fn journal_requires_contiguous_sequences() {
    let temp = tempfile::tempdir().unwrap();
    let run_id = RunId::new("sequence-gap").unwrap();
    let store = RunStore::open(temp.path(), run_id.clone()).unwrap();
    store
        .append(draft("mutation-1", NodeState::Succeeded))
        .unwrap();
    let raw = std::fs::read_to_string(store.events_path()).unwrap();
    let broken = raw.replacen("\"sequence\":1", "\"sequence\":2", 1);
    std::fs::write(store.events_path(), broken).unwrap();

    assert!(matches!(
        RunStore::open(temp.path(), run_id).unwrap().recover(),
        Err(RunStoreError::SequenceGap {
            expected: 1,
            actual: 2
        })
    ));
}

use std::io::Write;
