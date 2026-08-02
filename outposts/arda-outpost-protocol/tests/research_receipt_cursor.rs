use arda_outpost_protocol::ResearchReceiptLedger;
use tempfile::tempdir;

#[test]
fn ledger_cursor_advances_monotonically_and_survives_restart() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("warden-receipts.jsonl");
    let ledger = ResearchReceiptLedger::open(&path).unwrap();
    assert_eq!(ledger.read_cursor("observations").unwrap().sequence, 0);
    ledger
        .advance_cursor("observations", 3, "observation-3")
        .unwrap();
    assert!(ledger
        .advance_cursor("observations", 2, "observation-2")
        .is_err());
    let reopened = ResearchReceiptLedger::open(&path).unwrap();
    let cursor = reopened.read_cursor("observations").unwrap();
    assert_eq!(cursor.sequence, 3);
    assert_eq!(cursor.last_id.as_deref(), Some("observation-3"));
}
