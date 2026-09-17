use super::*;

#[test]
fn atomic_outcome_replay_preserves_framing_and_survives_partial_temporary() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("context_outcome_receipts.jsonl");
    let mut receipt: ContextOutcomeReceipt = serde_json::from_value(serde_json::json!({
        "schema_version": CONTEXT_OUTCOME_RECEIPT_SCHEMA_VERSION,
        "receipt_id":"one", "receipt_digest":"", "context_use_receipt_id":"use",
        "capsule_id":"capsule", "objective_id":"objective", "run_id":null,
        "consumer_id":"fixture", "disposition":"deferred", "selected_memory_refs":[],
        "influenced_memory_refs":[], "evidence_refs":[], "rationale":"fixture", "recorded_at_unix_ms":1
    })).unwrap();
    receipt.receipt_digest = receipt_digest(&receipt).unwrap();
    persist_receipt(root.path(), &receipt).unwrap();
    // An interrupted new replacement cannot corrupt the existing ledger.
    std::fs::write(
        root.path().join(".context_outcome_receipts.jsonl.tmp"),
        b"{\"partial",
    )
    .unwrap();
    persist_receipt(root.path(), &receipt).unwrap();
    assert_eq!(
        read_outcome_receipts(&File::open(&path).unwrap()).unwrap(),
        vec![receipt.clone()]
    );
    // Legacy complete-but-unframed values are repaired before another append.
    std::fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    persist_receipt(root.path(), &receipt).unwrap();
    assert!(std::fs::read(&path).unwrap().ends_with(b"\n"));
    receipt.receipt_id = "two".into();
    receipt.receipt_digest = receipt_digest(&receipt).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| persist_receipt(root.path(), &receipt).unwrap());
        }
    });
    assert_eq!(
        read_outcome_receipts(&File::open(&path).unwrap())
            .unwrap()
            .len(),
        2
    );
    let before = std::fs::read(&path).unwrap();
    receipt.rationale = "conflict".into();
    receipt.receipt_digest = receipt_digest(&receipt).unwrap();
    assert!(persist_receipt(root.path(), &receipt).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // Pre-existing corruption remains fail-closed rather than being discarded.
    std::fs::write(&path, b"{\"torn").unwrap();
    assert!(persist_receipt(root.path(), &receipt).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{\"torn");
}
