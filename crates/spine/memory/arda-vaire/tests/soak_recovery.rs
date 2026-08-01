use arda_vaire::{InformantEvent, MnemosyneService};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;

#[test]
fn append_consolidation_soak_recovers_across_malformed_records_and_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let memory_root = dir.path().join("data/mnemosyne");
    let service = MnemosyneService::new(&memory_root).expect("service");
    let iterations = 128usize;
    let mut encoded = 0usize;
    let mut receipt_count = 0usize;

    for index in 0..iterations {
        let result = service.encode(soak_event(index)).expect("encode");
        encoded += usize::from(result.is_some());

        if (index + 1) % 16 == 0 {
            let report = service.consolidate(24).expect("consolidate cycle");
            assert_eq!(report.episodic_scanned, encoded);
            receipt_count += report.promotion_receipts_written;
        }
    }

    let month = memory_root
        .join("episodic")
        .join(Utc::now().format("%Y-%m").to_string());
    fs::write(
        month.join("mem_malformed.jsonl"),
        "{\"schema_version\":\"arda.mnemosyne.episodic.v1\"}\n{broken\n",
    )
    .expect("malformed episode");
    let mut noise = OpenOptions::new()
        .append(true)
        .open(memory_root.join("noise.jsonl"))
        .expect("noise ledger");
    writeln!(noise, "{{broken").expect("malformed noise");
    let mut archive = OpenOptions::new()
        .create(true)
        .append(true)
        .open(memory_root.join("archive/consolidation.jsonl"))
        .expect("archive ledger");
    writeln!(archive, "[broken").expect("malformed archive");
    drop(service);

    let recovered = MnemosyneService::new(&memory_root).expect("recovered service");
    let recent = recovered.recall_recent(24, None).expect("recovered recall");
    assert_eq!(recent.len(), encoded);
    assert!(recent
        .iter()
        .any(|entry| entry.content.contains("checkpoint 127")));

    let final_report = recovered.consolidate(24).expect("post-restart consolidate");
    let stats = recovered.stats().expect("recovered stats");
    assert_eq!(stats.malformed_episodic_records, 1);
    assert_eq!(stats.malformed_noise_records, 1);
    assert!(stats.malformed_archive_records >= 1);
    assert_eq!(
        stats.memory_counts.core + stats.memory_counts.active,
        encoded
    );
    assert!(receipt_count > 0);
    assert!(final_report.promotion_receipts_written > 0);
    assert_eq!(stats.chain_integrity, "head_present");
}

#[test]
#[ignore = "operator-scale soak; run explicitly before Mnemosyne release closeout"]
fn operator_scale_append_consolidation_soak() {
    let dir = tempfile::tempdir().expect("tempdir");
    let memory_root = dir.path().join("data/mnemosyne");
    let service = MnemosyneService::new(&memory_root).expect("service");
    let mut encoded = 0usize;

    for index in 0..512 {
        encoded += usize::from(
            service
                .encode(soak_event(index))
                .expect("operator-scale encode")
                .is_some(),
        );
        if (index + 1) % 64 == 0 {
            let report = service.consolidate(24).expect("operator-scale consolidate");
            assert_eq!(report.episodic_scanned, encoded);
        }
    }

    drop(service);
    let recovered = MnemosyneService::new(&memory_root).expect("recovered service");
    assert_eq!(
        recovered
            .recall_recent(24, None)
            .expect("operator-scale recall")
            .len(),
        encoded
    );
}

fn soak_event(index: usize) -> InformantEvent {
    InformantEvent {
        informant_id: "soak-harness".to_owned(),
        crate_name: if index.is_multiple_of(2) {
            "arda-vaire".to_owned()
        } else {
            "arda-varda".to_owned()
        },
        event_type: "task_completed".to_owned(),
        ts_utc: Utc::now().to_rfc3339(),
        content: format!("Continuity checkpoint {index} completed with governed receipt evidence"),
        confidence_hint: Some(0.91),
        tags: vec!["continuity".to_owned(), "checkpoint".to_owned()],
    }
}
