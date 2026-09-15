use super::*;
pub(super) fn crash(point: &str) {
    if std::env::var("ARDA_TEST_REVOKE_PAUSE").ok().as_deref() == Some(point) {
        let root = PathBuf::from(std::env::var_os("ARDA_TEST_REVOKE_BARRIER").unwrap());
        fs::write(root.join("ready"), point).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while !root.join("continue").exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "reconciliation test barrier timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    if std::env::var("ARDA_TEST_REVOKE_CRASH").ok().as_deref() == Some(point) {
        // No destructors: exercise SQLite crash recovery, not transaction Drop.
        unsafe { libc::_exit(86) }
    }
}
fn receipt() -> Receipt {
    Receipt {
        version: 1,
        disposition: "terminal_revocation".into(),
        proof: "managed_stop".into(),
        worker_cleanup_ack: false,
        artifacts_retained: true,
        cleanup_verified: false,
        request_id: "request".into(),
        operator: "fixture".into(),
        reason: "crash proof".into(),
        owner: "owner".into(),
        run: "run".into(),
        record_digest: "record".into(),
        evidence_digest: "evidence".into(),
        prior_state: "lost".into(),
        allocation_prior_state: Some("allocated".into()),
    }
}
fn initialize(db: &Connection) {
    db.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
        CREATE TABLE snapshots(run TEXT PRIMARY KEY,state TEXT,authority TEXT);
        CREATE TABLE runtime_allocations(run TEXT PRIMARY KEY,state TEXT);
        INSERT INTO snapshots VALUES('run','lost','unchanged-authority');
        INSERT INTO runtime_allocations VALUES('run','allocated');",
    )
    .unwrap();
}
fn assert_state(db: &Connection, committed: bool) {
    let state: (String, String) = db
        .query_row(
            "SELECT state,authority FROM snapshots WHERE run='run'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        state.0,
        if committed {
            "reconciled_revoked"
        } else {
            "lost"
        }
    );
    assert_eq!(state.1, "unchanged-authority");
    let allocation: String = db
        .query_row(
            "SELECT state FROM runtime_allocations WHERE run='run'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        allocation,
        if committed {
            "reconciled_revoked"
        } else {
            "allocated"
        }
    );
    let saved = saved_receipt(db, "request").unwrap();
    assert_eq!(saved.is_some(), committed);
    if let Some(saved) = saved {
        assert!(saved == receipt());
    }
}
#[test]
fn abrupt_exit_keeps_receipt_tombstone_and_allocation_atomic() {
    if let Some(path) = std::env::var_os("ARDA_TEST_REVOKE_DB") {
        let mut db = Connection::open(path).unwrap();
        db.execute_batch("PRAGMA synchronous=FULL;").unwrap();
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        commit_revocation(tx, receipt(), true, || Ok(())).unwrap();
        panic!("failpoint not reached");
    }
    for point in [
        "receipt",
        "snapshot",
        "allocation",
        "precommit",
        "committed",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("owner.sqlite3");
        let db = Connection::open(&path).unwrap();
        initialize(&db);
        drop(db);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "keeper_reconcile::tests::abrupt_exit_keeps_receipt_tombstone_and_allocation_atomic", "--nocapture"])
            .env("ARDA_TEST_REVOKE_DB", &path).env("ARDA_TEST_REVOKE_CRASH", point).status().unwrap();
        assert_eq!(result.code(), Some(86));
        let mut db = Connection::open(&path).unwrap();
        assert_state(&db, point == "committed");
        if point != "committed" {
            let tx = db
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            commit_revocation(tx, receipt(), true, || Ok(())).unwrap();
        }
        assert_state(&db, true);
    }
}
#[test]
fn failed_final_stop_recheck_rolls_back_all_mutations() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize(&db);
    let tx = db.transaction().unwrap();
    assert!(commit_revocation(tx, receipt(), true, || bail!(
        "fixture restart interleaving"
    ))
    .is_err());
    assert_state(&db, false);
}
