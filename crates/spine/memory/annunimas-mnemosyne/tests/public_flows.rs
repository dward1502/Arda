use annunimas_mnemosyne::{InformantEvent, MnemosyneService};
use chrono::Utc;

#[test]
fn public_encode_recall_and_identity_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = MnemosyneService::new(dir.path()).expect("service");

    let encoded = service
        .encode(InformantEvent {
            informant_id: "prometheus_mneme".to_owned(),
            crate_name: "prometheus".to_owned(),
            event_type: "decision_completed".to_owned(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "Boardroom routing decision finalized for ARDA continuity".to_owned(),
            confidence_hint: Some(0.93),
            tags: vec!["boardroom".to_owned(), "routing".to_owned()],
        })
        .expect("encode")
        .expect("memory");

    assert_eq!(encoded.memory_scope, "boardroom_council");

    let relevant = service
        .recall_relevant("routing continuity", 24, Some("prometheus"), None, 5)
        .expect("recall relevant");
    assert_eq!(relevant.len(), 1);
    assert_eq!(relevant[0].source_crate, "prometheus");

    let identity = service.identity_state().expect("identity");
    assert!(!identity.recent_events.is_empty());
    assert!(identity.core_memory_count + identity.active_memory_count >= 1);
}

#[test]
fn public_sync_consolidate_and_status_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = MnemosyneService::new(dir.path()).expect("service");
    let vault = dir.path().join("human").join(".obsidian");
    std::fs::create_dir_all(vault.join("notes")).expect("mkdir");
    std::fs::write(
        vault.join("notes").join("mission.md"),
        "# Mission\nKeep human continuity active",
    )
    .expect("write");

    for idx in 0..3 {
        let _ = service
            .encode(InformantEvent {
                informant_id: "hermes_mneme".to_owned(),
                crate_name: "hermes".to_owned(),
                event_type: "task_completed".to_owned(),
                ts_utc: Utc::now().to_rfc3339(),
                content: format!("Human continuity delivery checkpoint {idx}"),
                confidence_hint: Some(0.82),
                tags: vec!["human".to_owned(), "continuity".to_owned()],
            })
            .expect("encode");
    }

    let sync = service.sync_obsidian(&vault, 20).expect("sync");
    assert!(sync.notes_indexed >= 1);

    let report = service.consolidate(24).expect("consolidate");
    assert!(report.semantic_patterns_written >= 1 || report.procedural_patterns_written >= 1);

    let status = service.status().expect("status");
    assert_eq!(status["ok"], true);

    let stats = service.stats().expect("stats");
    assert!(stats.last_consolidation_utc.is_some());
    assert_eq!(stats.chain_integrity, "head_present");
}
