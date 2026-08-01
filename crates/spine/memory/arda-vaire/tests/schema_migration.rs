use arda_vaire::{
    InformantEvent, MnemosyneService, CONTINUITY_SCHEMA_VERSION, EPISODIC_SCHEMA_VERSION,
    LEGACY_EPISODIC_SCHEMA_VERSION,
};
use chrono::Utc;
use std::fs;

fn month_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("episodic")
        .join(Utc::now().format("%Y-%m").to_string())
}

#[test]
fn new_episodes_and_continuity_status_publish_explicit_schema_versions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = MnemosyneService::new(dir.path()).expect("service");
    let encoded = service
        .encode(InformantEvent {
            informant_id: "schema-test".to_owned(),
            crate_name: "arda-vaire".to_owned(),
            event_type: "continuity_checkpoint".to_owned(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "Versioned continuity checkpoint with governance evidence".to_owned(),
            confidence_hint: Some(0.9),
            tags: vec!["checkpoint".to_owned(), "continuity".to_owned()],
        })
        .expect("encode")
        .expect("durable memory");

    assert_eq!(encoded.schema_version, EPISODIC_SCHEMA_VERSION);
    assert_eq!(encoded.migrated_from_schema, None);

    let path = month_dir(dir.path()).join(format!("{}.jsonl", encoded.memory_id));
    let lines = fs::read_to_string(path).expect("episode");
    let values = lines
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json"))
        .collect::<Vec<_>>();
    assert_eq!(values[0]["schema_version"], EPISODIC_SCHEMA_VERSION);
    assert_eq!(values[1]["schema_version"], EPISODIC_SCHEMA_VERSION);

    let status = service.status().expect("status");
    assert_eq!(status["schema_version"], CONTINUITY_SCHEMA_VERSION);
    assert_eq!(
        status["status"]["schema_version"],
        CONTINUITY_SCHEMA_VERSION
    );
}

#[test]
fn legacy_episodes_migrate_on_read_and_unknown_schemas_are_disclosed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = MnemosyneService::new(dir.path()).expect("service");
    let month = month_dir(dir.path());
    fs::create_dir_all(&month).expect("month");
    let now = Utc::now().to_rfc3339();

    fs::write(
        month.join("mem_legacy.jsonl"),
        format!(
            "{{\"sigil\":\"MNEME_ACTIVE\",\"memory_id\":\"mem_legacy\",\"version\":\"0.1.0\"}}\n{{\"type\":\"episodic\",\"source_crate\":\"legacy\",\"event_type\":\"checkpoint\",\"memory_scope\":\"system_continuity\",\"significance\":0.8,\"confidence\":0.7,\"trust\":0.6,\"content\":\"legacy continuity survives migration\",\"tags\":[\"continuity\"],\"ts_utc\":{now:?}}}\n"
        ),
    )
    .expect("legacy episode");
    fs::write(
        month.join("mem_future.jsonl"),
        format!(
            "{{\"schema_version\":\"arda.mnemosyne.episodic.v99\",\"sigil\":\"MNEME_ACTIVE\"}}\n{{\"schema_version\":\"arda.mnemosyne.episodic.v99\",\"content\":\"unsupported\",\"ts_utc\":{now:?}}}\n"
        ),
    )
    .expect("future episode");

    let recalled = service.recall_recent(24, None).expect("recall");
    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].memory_id, "mem_legacy");
    assert_eq!(recalled[0].schema_version, EPISODIC_SCHEMA_VERSION);
    assert_eq!(
        recalled[0].migrated_from_schema.as_deref(),
        Some(LEGACY_EPISODIC_SCHEMA_VERSION)
    );

    let stats = service.stats().expect("stats");
    assert_eq!(stats.legacy_episodic_records, 1);
    assert_eq!(stats.unsupported_episodic_records, 1);
}
