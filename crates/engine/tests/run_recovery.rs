use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunGraphError, RunId};
use arda_engine::runs::{AppendOutcome, RunEventDraft, RunEventKind, RunStore, RunStoreError};
use std::path::{Path, PathBuf};

fn spec_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec")
        .join(path)
}

fn fixed_graph(path: impl AsRef<Path>) -> RunGraph {
    let raw = std::fs::read_to_string(spec_path(path)).unwrap();
    RunGraph::from_json_str(&raw).unwrap()
}

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
fn disk_pressure_write_failure_does_not_publish_partial_result() {
    let temp = tempfile::tempdir().unwrap();
    let run_id = RunId::new("disk-pressure").unwrap();
    let store = RunStore::open(temp.path(), run_id).unwrap();
    let blocked_temporary = store
        .result_path()
        .with_extension(format!("tmp.{}", std::process::id()));
    std::fs::create_dir(&blocked_temporary).unwrap();

    let error = store
        .write_result(&serde_json::json!({"terminal_state": "succeeded"}))
        .expect_err("unavailable temporary allocation must fail closed");

    assert!(matches!(error, RunStoreError::Io { .. }));
    assert_eq!(store.read_result().unwrap(), None);
    assert!(!store.result_path().exists());
}

#[test]
fn corrupt_or_truncated_journal_tail_fails_visibly() {
    let temp = tempfile::tempdir().unwrap();
    let base = fixed_graph("run-graph/v1/fixtures/valid-run-graph.json");
    let original = serde_json::to_vec(&base).unwrap();
    let run_id = base.run_id.clone();
    let store = RunStore::open(temp.path(), run_id.clone()).unwrap();
    store.write_checkpoint(&base).unwrap();
    let checkpoint_before = std::fs::read(store.checkpoint_path()).unwrap();
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
    assert_eq!(serde_json::to_vec(&base).unwrap(), original);
    assert_eq!(
        std::fs::read(reopened.checkpoint_path()).unwrap(),
        checkpoint_before
    );
}

#[test]
fn malformed_newline_terminated_journal_fails_without_advancing_projection() {
    let temp = tempfile::tempdir().unwrap();
    let base = fixed_graph("run-graph/v1/fixtures/valid-run-graph.json");
    let original = serde_json::to_vec(&base).unwrap();
    let store = RunStore::open(temp.path(), base.run_id.clone()).unwrap();
    store.write_checkpoint(&base).unwrap();
    let checkpoint_before = std::fs::read(store.checkpoint_path()).unwrap();
    std::fs::write(store.events_path(), b"{\"schema_version\":}\n").unwrap();

    assert!(matches!(
        store.recover(),
        Err(RunStoreError::CorruptJournal { line: 1, .. })
    ));
    assert_eq!(serde_json::to_vec(&base).unwrap(), original);
    assert_eq!(
        std::fs::read(store.checkpoint_path()).unwrap(),
        checkpoint_before
    );
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

#[test]
fn fixed_journal_replay_is_deterministic_and_matches_expected_projection() {
    let temp = tempfile::tempdir().unwrap();
    let base = fixed_graph("run-graph/v1/fixtures/valid-run-graph.json");
    let expected = fixed_graph("run-event/v1/fixtures/expected-projection.json");
    let store = RunStore::open(temp.path(), base.run_id.clone()).unwrap();
    std::fs::copy(
        spec_path("run-event/v1/fixtures/replay-events.jsonl"),
        store.events_path(),
    )
    .unwrap();

    let recovered = store.recover().unwrap();
    let first = recovered.replay(&base).unwrap();
    let second = recovered.replay(&base).unwrap();

    assert_eq!(first, expected);
    assert_eq!(second, expected);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn semantically_out_of_sequence_event_fails_without_advancing_projection() {
    let temp = tempfile::tempdir().unwrap();
    let base = fixed_graph("run-graph/v1/fixtures/valid-run-graph.json");
    let original = serde_json::to_vec(&base).unwrap();
    let store = RunStore::open(temp.path(), base.run_id.clone()).unwrap();
    let events =
        std::fs::read_to_string(spec_path("run-event/v1/fixtures/replay-events.jsonl")).unwrap();
    let mut lines = events.lines().take(2);
    let ready = lines.next().unwrap();
    let invalid = lines
        .next()
        .unwrap()
        .replace("\"state\":\"running\"", "\"state\":\"succeeded\"");
    std::fs::write(store.events_path(), format!("{ready}\n{invalid}\n")).unwrap();

    let error = store.recover().unwrap().replay(&base).unwrap_err();
    assert!(matches!(
        error,
        RunStoreError::Graph(RunGraphError::InvalidTransition { .. })
    ));
    assert_eq!(serde_json::to_vec(&base).unwrap(), original);
}

use std::io::Write;
