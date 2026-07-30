use arda_outpost_scout::{
    AuthorityClass, ObservationClassification, ObservationMemoryBridge, ObservationScope,
    OutpostObservation, ScoutRecallQuery, ScoutRecallStatus,
};
use chrono::{Duration, Utc};
use std::sync::Mutex;
use tempfile::tempdir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn encode_preserves_the_observation_and_returns_an_ingestion_receipt() {
    let _env = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");
    std::env::set_var("ARDA_PLUTUS_HOME", dir.path().join("plutus"));

    let mut observation = OutpostObservation::new(
        "fixture",
        ObservationScope::Crates,
        ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        serde_json::json!({
            "path": "crates/example",
            "message": "Scout survey observed an actionable crate status signal"
        }),
    );
    observation.confidence = 0.8;
    observation.freshness_seconds = 7;
    let bridge = ObservationMemoryBridge::at_root("outpost_scout", dir.path());
    let memory = bridge
        .encode_observation_to_memory(&observation)
        .expect("encode observation");

    assert_eq!(memory.scope, "outpost_scout");
    assert_eq!(memory.source_crate, "arda-outpost-scout");
    assert_eq!(memory.failure_reason, "encoded");
    assert!(!memory.missing_root);
    assert!(memory.memory_id.is_some());
    assert!((memory.confidence.expect("confidence") - 0.8).abs() < 1e-6);
    assert!(!memory.credentials.is_empty());
    assert_eq!(memory.suggested_event_type, "outpost_observation");
    let preserved: OutpostObservation =
        serde_json::from_str(&memory.content).expect("preserved observation");
    assert_eq!(preserved, observation);

    let raw = bridge.recall_recent_observations(24).expect("raw recall");
    assert_eq!(raw.len(), 1);
    let preserved: OutpostObservation =
        serde_json::from_str(&raw[0].content).expect("persisted observation");
    assert_eq!(preserved, observation);

    std::env::remove_var("ARDA_PLUTUS_HOME");
}

#[test]
fn scoped_recall_filters_and_reports_stale_memory() {
    let _env = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");
    std::env::set_var("ARDA_PLUTUS_HOME", dir.path().join("plutus"));

    let mut observation = OutpostObservation::new(
        "fixture",
        ObservationScope::Crates,
        ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        serde_json::json!({
            "name": "manwe",
            "path": "crates/spine/runtime/manwe",
            "message": "Scout survey observed an actionable recall signal"
        }),
    );
    observation.confidence = 0.8;
    observation.observed_at = Utc::now() - Duration::minutes(5);
    let bridge = ObservationMemoryBridge::at_root("outpost_scout", dir.path());
    let memory = bridge
        .encode_observation_to_memory(&observation)
        .expect("bridge result");
    assert_eq!(memory.failure_reason, "encoded");

    let recalled = bridge.recall_observations(&ScoutRecallQuery {
        scope: Some("crates".to_string()),
        name: Some("manwe".to_string()),
        path: Some("runtime/manwe".to_string()),
        query: Some("recall signal".to_string()),
        max_age_seconds: Some(30),
        ..ScoutRecallQuery::default()
    });
    assert_eq!(recalled.status, ScoutRecallStatus::Stale);
    assert_eq!(recalled.records.len(), 1);
    assert_eq!(recalled.records[0].observation, observation);
    assert!(recalled.records[0].stale);
    assert!(recalled.records[0].confidence > 0.0);
    assert!(recalled.records[0].trust > 0.0);

    std::env::remove_var("ARDA_PLUTUS_HOME");
}

#[test]
fn invalid_root_returns_structured_memory_fallback() {
    let dir = tempdir().expect("tempdir");
    let invalid_root = dir.path().join("not-a-directory");
    std::fs::write(&invalid_root, "fixture").expect("invalid root fixture");
    let observation = OutpostObservation::new(
        "fixture",
        ObservationScope::Crates,
        ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        serde_json::json!({"message": "Scout survey observed a status signal"}),
    );
    let memory = ObservationMemoryBridge::at_root("outpost_scout", &invalid_root)
        .encode_observation_to_memory(&observation)
        .expect("structured fallback");

    assert_eq!(memory.scope, "outpost_scout");
    assert!(memory.missing_root);
    assert!(memory.failure_reason.contains("not-a-directory"));
    assert!(memory.memory_id.is_none());
    assert!(!memory.credentials.is_empty());
}

#[test]
fn recall_degrades_when_memory_root_is_unavailable() {
    let dir = tempdir().expect("tempdir");
    let invalid_root = dir.path().join("not-a-directory");
    std::fs::write(&invalid_root, "fixture").expect("invalid root fixture");
    let report = ObservationMemoryBridge::at_root("outpost_scout", invalid_root)
        .recall_observations(&ScoutRecallQuery::default());

    assert_eq!(report.status, ScoutRecallStatus::Unavailable);
    assert!(report.records.is_empty());
    assert!(report.warning.is_some());
}

#[test]
fn configured_fallback_root_is_used_when_the_primary_root_fails() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let invalid_root = tempfile::NamedTempFile::new().expect("invalid root file");
    let fallback_root = tempdir().expect("fallback memory root");
    let plutus_root = tempdir().expect("plutus root");
    std::env::set_var("ARDA_PLUTUS_HOME", plutus_root.path());
    let observation = OutpostObservation::new(
        "fixture",
        ObservationScope::Crates,
        ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        serde_json::json!({"message": "Scout fallback root signal"}),
    );
    let bridge = ObservationMemoryBridge::at_root("outpost_scout", invalid_root.path())
        .with_fallback_root(fallback_root.path());

    let outcome = bridge
        .encode_observation_to_memory(&observation)
        .expect("fallback root encode");

    assert!(!outcome.missing_root);
    assert!(outcome.memory_id.is_some());
    assert_eq!(outcome.failure_reason, "encoded");
    std::env::remove_var("ARDA_PLUTUS_HOME");
}

#[test]
fn observation_receipts_are_append_only_and_do_not_create_queue_authority() {
    let _env = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");
    std::env::set_var("ARDA_PLUTUS_HOME", dir.path().join("plutus"));
    let bridge = ObservationMemoryBridge::at_root("outpost_scout", dir.path());

    let first = OutpostObservation::new(
        "node-pi5-warden",
        ObservationScope::Custom("internet_research".into()),
        ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        serde_json::json!({"query": "first", "results": [{"url": "https://example.com/first"}]}),
    );
    let second = OutpostObservation::new(
        "node-pi5-warden",
        ObservationScope::Custom("internet_research".into()),
        ObservationClassification::RawMeasurement,
        AuthorityClass::Advisory,
        serde_json::json!({"query": "second", "results": [{"url": "https://example.com/second"}]}),
    );

    let first_receipt = bridge
        .encode_observation_to_memory(&first)
        .expect("first append")
        .memory_id
        .expect("first receipt");
    let second_receipt = bridge
        .encode_observation_to_memory(&second)
        .expect("second append")
        .memory_id
        .expect("second receipt");
    let records = bridge
        .recall_recent_observations(24)
        .expect("receipt recall");

    assert_ne!(first_receipt, second_receipt);
    assert_eq!(records.len(), 2);
    assert!(records
        .iter()
        .any(|record| record.memory_id == first_receipt));
    assert!(records
        .iter()
        .any(|record| record.memory_id == second_receipt));
    assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!dir.path().join("data/approvals").exists());

    std::env::remove_var("ARDA_PLUTUS_HOME");
}
