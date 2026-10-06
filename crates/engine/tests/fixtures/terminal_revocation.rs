use super::*;
use arda_engine::objectives::{
    keeper_client::KeeperFailure,
    terminal_revocation::{TerminalRevocationProof, TerminalRevocationReceipt},
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Revoked {
    queries: AtomicUsize,
    proof: Mutex<TerminalRevocationProof>,
    generic_failure: bool,
}
impl SnapshotAdmission for Revoked {
    fn prepare(&self, _: &str, _: &Path, _: &str) -> Result<RetainedSnapshot> {
        panic!("must never prepare")
    }
    fn commit(&self, _: &RetainedSnapshot, _: &str, _: i64, _: &str, _: i64) -> Result<()> {
        panic!("must never commit")
    }
    fn release(&self, _: &RetainedSnapshot, _: &str) -> Result<()> {
        if self.generic_failure {
            bail!("unrelated failure");
        }
        Err(KeeperFailure::MissingRevokedAuthority.into())
    }
    fn terminal_revocation(
        &self,
        run: &str,
        workspace: &str,
        identity: &str,
    ) -> Result<TerminalRevocationProof> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        let proof = self.proof.lock().unwrap().clone();
        assert!(!run.is_empty() && !workspace.is_empty() && !identity.is_empty());
        Ok(proof)
    }
}
fn retired_fixture() -> (tempfile::TempDir, ObjectiveStore, Arc<Revoked>, PathBuf) {
    let (temp, store, keeper) = fixture();
    store.claim_runnable("worker", 10, 100, 1).unwrap();
    store
        .apply_control("snapshot", ControlAction::Cancel, "cancel", "operator", 11)
        .unwrap();
    let db = rusqlite::Connection::open(&keeper.database).unwrap();
    let (run,identity):(String,String)=db.query_row("SELECT s.run_id,i.identity_json FROM retained_workspace_snapshots s JOIN lease_workspace_identities i USING(leaf_id)",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    let tuple: serde_json::Value = serde_json::from_str(&identity).unwrap();
    let revoked = Arc::new(Revoked {
        queries: AtomicUsize::new(0),
        generic_failure: false,
        proof: Mutex::new(TerminalRevocationProof {
            run: run.clone(),
            workspace: tuple[1].as_str().unwrap().into(),
            identity,
            receipt: TerminalRevocationReceipt {
                version: 1,
                disposition: "terminal_revocation".into(),
                proof: "managed_cgroup_teardown_or_reboot".into(),
                worker_cleanup_ack: false,
                artifacts_retained: true,
                cleanup_verified: false,
                request_id: "test-only".into(),
                operator: "operator".into(),
                reason: "test-only fixture".into(),
                owner: "fixture-owner".into(),
                run,
                record_digest: "a".repeat(64),
                evidence_digest: "b".repeat(64),
                prior_state: "lost".into(),
                allocation_prior_state: None,
            },
        }),
    });
    (
        temp,
        store.with_snapshot_admission(revoked.clone()),
        revoked,
        keeper.database.clone(),
    )
}
fn count(db: &rusqlite::Connection, table: &str) -> i64 {
    db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
#[test]
fn terminal_revocation_persists_proof_and_marker_across_restart_without_filesystem() {
    let (temp, store, keeper, path) = retired_fixture();
    // Original input may be an alias, while the physical root no longer exists.
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE leaves SET workspace_root='./old-symlink'", [])
        .unwrap();
    std::fs::remove_dir(temp.path().join("workspace")).unwrap();
    store.reconcile_snapshot_commits().unwrap();
    assert_eq!(count(&db, "retained_snapshot_releases"), 1);
    assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 1);
    let proof: String = db
        .query_row(
            "SELECT proof_json FROM retained_snapshot_terminal_revocations",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<TerminalRevocationProof>(&proof).unwrap(),
        *keeper.proof.lock().unwrap()
    );
    assert!(db
        .execute(
            "UPDATE retained_snapshot_terminal_revocations SET proof_json='{}'",
            []
        )
        .is_err());
    assert!(db
        .execute("DELETE FROM retained_snapshot_terminal_revocations", [])
        .is_err());
    drop(store);
    let store = ObjectiveStore::open(&path)
        .unwrap()
        .with_snapshot_admission(keeper.clone());
    store.reconcile_snapshot_commits().unwrap();
    assert!(store
        .claim_runnable("worker", 1000, 100, 1)
        .unwrap()
        .is_empty());
    assert_eq!(keeper.queries.load(Ordering::SeqCst), 1);
    assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 1);
}
#[test]
fn paused_or_mismatched_lineage_cannot_query_terminal_retirement() {
    for sql in [
        "UPDATE objectives SET state='paused'",
        "UPDATE retained_workspace_snapshots SET run_id='wrong'",
        "DELETE FROM lease_workspace_identities",
        r#"UPDATE lease_workspace_identities SET identity_json='[99,"/bad","/bad",[1,2],"x"]'"#,
    ] {
        let (_temp, store, keeper, path) = retired_fixture();
        let db = rusqlite::Connection::open(path).unwrap();
        db.execute(sql, []).unwrap();
        assert!(store.reconcile_snapshot_commits().is_err(), "{sql}");
        assert_eq!(keeper.queries.load(Ordering::SeqCst), 0, "{sql}");
        assert_eq!(count(&db, "retained_snapshot_releases"), 0);
    }
}
#[test]
fn proof_and_marker_rollback_together() {
    let (_temp, store, keeper, path) = retired_fixture();
    let db = rusqlite::Connection::open(path).unwrap();
    db.execute_batch("CREATE TRIGGER inject_marker_failure BEFORE INSERT ON retained_snapshot_releases BEGIN SELECT RAISE(ABORT,'injected marker failure'); END;").unwrap();
    assert!(store.reconcile_snapshot_commits().is_err());
    assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 0);
    assert_eq!(count(&db, "retained_snapshot_releases"), 0);
    db.execute_batch("DROP TRIGGER inject_marker_failure")
        .unwrap();
    store.reconcile_snapshot_commits().unwrap();
    assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 1);
    assert_eq!(count(&db, "retained_snapshot_releases"), 1);
    assert_eq!(keeper.queries.load(Ordering::SeqCst), 2);
}

fn preserved_state(db: &rusqlite::Connection) -> Vec<(String, Vec<String>)> {
    let names: Vec<String> = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('retained_snapshot_releases','retained_snapshot_terminal_revocations') ORDER BY name").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
    names
        .into_iter()
        .map(|name| {
            let mut statement = db
                .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
                .unwrap();
            let columns = statement.column_count();
            let mut rows = statement
                .query_map([], |r| {
                    let values = (0..columns)
                        .map(|i| r.get::<_, rusqlite::types::Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    Ok(format!("{values:?}"))
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            rows.sort();
            (name, rows)
        })
        .collect()
}
#[test]
fn generic_failure_never_queries_or_changes_history() {
    let (_temp, store, keeper, path) = retired_fixture();
    let refused = Arc::new(Revoked {
        queries: AtomicUsize::new(0),
        proof: Mutex::new(keeper.proof.lock().unwrap().clone()),
        generic_failure: true,
    });
    let store = store.with_snapshot_admission(refused.clone());
    let db = rusqlite::Connection::open(path).unwrap();
    let before = preserved_state(&db);
    assert!(store
        .reconcile_snapshot_commits_for_leaf("leaf")
        .unwrap_err()
        .to_string()
        .contains("unrelated failure"));
    assert_eq!(refused.queries.load(Ordering::SeqCst), 0);
    assert_eq!(count(&db, "retained_snapshot_releases"), 0);
    assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 0);
    assert_eq!(preserved_state(&db), before);
}
#[test]
fn incorrect_proof_bindings_and_flags_leave_no_marker_or_history_changes() {
    for field in [
        "run",
        "workspace",
        "identity",
        "version",
        "disposition",
        "receipt_run",
        "worker_cleanup_ack",
        "cleanup_verified",
        "artifacts_retained",
        "record_digest",
        "evidence_digest",
    ] {
        let (_temp, store, keeper, path) = retired_fixture();
        {
            let mut p = keeper.proof.lock().unwrap();
            match field {
                "run" => p.run.push_str("wrong"),
                "workspace" => p.workspace.push_str("wrong"),
                "identity" => p.identity.push_str("wrong"),
                "version" => p.receipt.version = 2,
                "disposition" => p.receipt.disposition = "released".into(),
                "receipt_run" => p.receipt.run.push_str("wrong"),
                "worker_cleanup_ack" => p.receipt.worker_cleanup_ack = true,
                "cleanup_verified" => p.receipt.cleanup_verified = true,
                "artifacts_retained" => p.receipt.artifacts_retained = false,
                "record_digest" => p.receipt.record_digest = "wrong".into(),
                "evidence_digest" => p.receipt.evidence_digest = "wrong".into(),
                _ => unreachable!(),
            }
        }
        let db = rusqlite::Connection::open(path).unwrap();
        let before = preserved_state(&db);
        let error = store
            .reconcile_snapshot_commits_for_leaf("leaf")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid bound terminal revocation proof"),
            "{field}: {error}"
        );
        assert_eq!(count(&db, "retained_snapshot_releases"), 0);
        assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 0);
        assert_eq!(preserved_state(&db), before);
    }
}
#[test]
fn preexisting_proof_requires_exact_match_and_preserves_all_other_state() {
    for conflict in [false, true] {
        let (_temp, store, keeper, path) = retired_fixture();
        let db = rusqlite::Connection::open(path).unwrap();
        let mut proof = keeper.proof.lock().unwrap().clone();
        if conflict {
            proof.receipt.reason.push_str(" changed");
        }
        db.execute("INSERT INTO retained_snapshot_terminal_revocations VALUES('leaf','snapshot',?1,?2,?3,'terminal_revocation',?4)",rusqlite::params![proof.run,proof.workspace,proof.identity,serde_json::to_string(&proof).unwrap()]).unwrap();
        let before = preserved_state(&db);
        let result = store.reconcile_snapshot_commits_for_leaf("leaf");
        if conflict {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("conflicts with retained evidence"));
        } else {
            result.unwrap();
        }
        assert_eq!(
            count(&db, "retained_snapshot_releases"),
            if conflict { 0 } else { 1 }
        );
        assert_eq!(count(&db, "retained_snapshot_terminal_revocations"), 1);
        assert_eq!(preserved_state(&db), before);
    }
}
