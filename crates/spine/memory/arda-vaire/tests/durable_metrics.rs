use arda_vaire::{InformantEvent, MnemosyneService};
use chrono::Utc;
use std::fs;

#[test]
fn observability_is_durable_and_runtime_snapshots_are_atomic_exports() {
    let dir = tempfile::tempdir().expect("tempdir");
    let metrics_root = dir.path().join("core/metrics/by_crate/mnemosyne");
    let service = MnemosyneService::new(dir.path().join("data/mnemosyne"))
        .expect("service")
        .with_metrics_root(metrics_root.clone());

    for index in 0..2 {
        service
            .encode(InformantEvent {
                informant_id: "metrics-test".to_owned(),
                crate_name: "arda-vaire".to_owned(),
                event_type: "task_completed".to_owned(),
                ts_utc: Utc::now().to_rfc3339(),
                content: format!(
                    "Governed memory promotion checkpoint {index} with durable evidence"
                ),
                confidence_hint: Some(0.9),
                tags: vec!["governance".to_owned(), "checkpoint".to_owned()],
            })
            .expect("encode");
    }

    let recalled = service
        .recall_relevant("governed checkpoint", 24, None, None, 2)
        .expect("recall");
    assert_eq!(recalled.len(), 2);
    let report = service.consolidate(24).expect("consolidate");
    assert!(report.promotion_receipts_written >= 1);
    service
        .export_runtime_snapshots()
        .expect("runtime snapshots");

    let observability: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(metrics_root.join("observability.json")).expect("observability"),
    )
    .expect("observability json");
    assert_eq!(
        observability["schema_version"],
        "arda.mnemosyne.observability.v1"
    );
    assert!(observability["metrics"]["recall_requests_total"]
        .as_u64()
        .is_some_and(|value| value >= 2));
    assert!(observability["metrics"]["promotion_receipts_total"]
        .as_u64()
        .is_some_and(|value| value >= 1));

    let stats: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(metrics_root.join("stats.json")).expect("stats"))
            .expect("stats json");
    let status: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(metrics_root.join("status.json")).expect("status"),
    )
    .expect("status json");
    assert_eq!(stats["schema_version"], "arda.mnemosyne.continuity.v1");
    assert_eq!(status["schema_version"], "arda.mnemosyne.continuity.v1");
    assert!(!metrics_root.join(".observability.json.tmp").exists());
    assert!(!metrics_root.join(".stats.json.tmp").exists());
    assert!(!metrics_root.join(".status.json.tmp").exists());
}

#[test]
fn canonical_memory_root_infers_the_aule_metrics_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = MnemosyneService::new(dir.path().join("data/mnemosyne")).expect("service");

    service
        .recall_relevant("continuity", 4, None, None, 1)
        .expect("recall");

    assert!(dir
        .path()
        .join("core/metrics/by_crate/mnemosyne/observability.json")
        .is_file());
}
