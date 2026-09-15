use anyhow::{bail, Result};
use arda_engine::objectives::{
    ControlAction, NewLeaf, NewObjective, ObjectiveStore, ProjectAuthority, RetainedSnapshot,
    SnapshotAdmission,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct KeeperState {
    prepares: usize,
    commits: Vec<(String, i64, String, i64)>,
    lose_ack: bool,
    missing: bool,
    reject_prepare: bool,
    releases: usize,
    lose_release_ack: bool,
}
struct Keeper {
    database: PathBuf,
    state: Mutex<KeeperState>,
}
impl SnapshotAdmission for Keeper {
    fn release(&self, _: &RetainedSnapshot, _: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.releases += 1;
        if std::mem::take(&mut state.lose_release_ack) {
            bail!("release acknowledgement lost");
        }
        Ok(())
    }
    fn prepare(&self, _: &str, root: &Path, identity: &str) -> Result<RetainedSnapshot> {
        assert!(root.is_dir());
        assert!(!identity.is_empty());
        let mut state = self.state.lock().unwrap();
        state.prepares += 1;
        if state.reject_prepare {
            bail!("prepare rejected");
        }
        Ok(RetainedSnapshot {
            endpoint: "/fixture/control.sock".into(),
            capability: "fixture-private-capability".into(),
            manifest_digest: "a".repeat(64),
        })
    }
    fn commit(
        &self,
        snapshot: &RetainedSnapshot,
        run: &str,
        generation: i64,
        owner: &str,
        expires: i64,
    ) -> Result<()> {
        // A separate connection must already see the exact durable intent.
        let connection = rusqlite::Connection::open(&self.database)?;
        let saved: String = connection.query_row(
            "SELECT capability_json FROM retained_workspace_snapshots WHERE run_id = ?1",
            [run],
            |row| row.get(0),
        )?;
        assert!(serde_json::from_str::<RetainedSnapshot>(&saved)? == *snapshot);
        let mut state = self.state.lock().unwrap();
        if state.missing {
            bail!("keeper lost; explicit reconciliation required");
        }
        state
            .commits
            .push((run.to_owned(), generation, owner.to_owned(), expires));
        if std::mem::take(&mut state.lose_ack) {
            bail!("commit acknowledgement lost");
        }
        Ok(())
    }
}

fn fixture() -> (tempfile::TempDir, ObjectiveStore, Arc<Keeper>) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("workspace")).unwrap();
    let database = temp.path().join("objectives.sqlite3");
    let keeper = Arc::new(Keeper {
        database: database.clone(),
        state: Mutex::new(KeeperState::default()),
    });
    let store = ObjectiveStore::open(&database)
        .unwrap()
        .with_snapshot_admission(keeper.clone());
    store
        .create_authenticated_objective(
            NewObjective {
                id: "snapshot".into(),
                source_id: "snapshot-source".into(),
                idempotency_key: "snapshot-ingress".into(),
                operator_id: "operator".into(),
                text: "Read the approved fixture".into(),
                priority: 1,
                projects: vec![ProjectAuthority {
                    project_id: "fixture".into(),
                    contract_digest: "sha256:fixture".into(),
                }],
                leaves: vec![NewLeaf {
                    id: "leaf".into(),
                    project_id: Some("fixture".into()),
                    workspace_root: temp.path().join("workspace").to_str().unwrap().into(),
                    authority: "read_only".into(),
                    dependencies: vec![],
                    execution: None,
                }],
            },
            1,
        )
        .unwrap();
    store
        .apply_control(
            "snapshot",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            2,
        )
        .unwrap();
    (temp, store, keeper)
}

#[test]
fn retained_reservation_blocks_other_eligible_work_with_spare_capacity() {
    let (temp, store, keeper) = fixture();
    let other = temp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    store
        .create_authenticated_objective(
            NewObjective {
                id: "other".into(),
                source_id: "other-source".into(),
                idempotency_key: "other-ingress".into(),
                operator_id: "operator".into(),
                text: "Other eligible work".into(),
                priority: 0,
                projects: vec![ProjectAuthority {
                    project_id: "fixture".into(),
                    contract_digest: "sha256:fixture".into(),
                }],
                leaves: vec![NewLeaf {
                    id: "other-leaf".into(),
                    project_id: Some("fixture".into()),
                    workspace_root: other.to_str().unwrap().into(),
                    authority: "read_only".into(),
                    dependencies: vec![],
                    execution: None,
                }],
            },
            3,
        )
        .unwrap();
    store
        .apply_control(
            "other",
            ControlAction::Approve { revision: 1 },
            "other-approve",
            "operator",
            4,
        )
        .unwrap();
    let first = store.claim_runnable("first", 10, 100, 4).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].objective_id, "snapshot");
    assert!(store
        .claim_runnable("second", 11, 100, 4)
        .unwrap()
        .is_empty());
    // Pausing keeps the expired capability reserved but removes it as a
    // runnable recovery candidate. The other objective still cannot start.
    store
        .apply_control("snapshot", ControlAction::Pause, "pause", "operator", 20)
        .unwrap();
    assert!(store
        .claim_runnable("second", 111, 100, 4)
        .unwrap()
        .is_empty());
    let attempt: i64 = rusqlite::Connection::open(&keeper.database)
        .unwrap()
        .query_row(
            "SELECT attempt FROM leaves WHERE id='other-leaf'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(attempt, 0);
    assert_eq!(keeper.state.lock().unwrap().prepares, 1);
}

#[test]
fn recovery_does_not_replace_missing_or_mismatched_capabilities() {
    for mutation in [
        "UPDATE retained_workspace_snapshots SET run_id='wrong-run'",
        "DELETE FROM retained_workspace_snapshots",
        "DELETE FROM lease_workspace_identities",
    ] {
        let (temp, store, keeper) = fixture();
        store.claim_runnable("first", 10, 100, 1).unwrap();
        std::fs::rename(temp.path().join("workspace"), temp.path().join("original")).unwrap();
        let db = rusqlite::Connection::open(&keeper.database).unwrap();
        // Simulate externally corrupted durable state, including broken FKs.
        db.pragma_update(None, "foreign_keys", false).unwrap();
        db.execute(mutation, []).unwrap();
        assert!(
            store.claim_runnable("second", 111, 100, 1).is_err(),
            "{mutation}"
        );
        let attempt: i64 = db
            .query_row("SELECT attempt FROM leaves", [], |r| r.get(0))
            .unwrap();
        assert_eq!(attempt, 1);
        assert_eq!(keeper.state.lock().unwrap().prepares, 1);
    }
}

fn durable(keeper: &Keeper) -> (i64, String, i64) {
    rusqlite::Connection::open(&keeper.database)
        .unwrap()
        .query_row(
            "SELECT l.attempt, s.capability_json, s.committed_generation FROM leaves l
         JOIN retained_workspace_snapshots s ON s.leaf_id = l.id",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
}

#[test]
fn snapshot_commit_ack_loss_reopens_same_intent_before_reclaim() {
    recovery_after_root_change(false);
    recovery_after_root_change(true);
}

fn recovery_after_root_change(replace: bool) {
    let (temp, store, keeper) = fixture();
    keeper.state.lock().unwrap().lose_ack = true;
    assert!(store.claim_runnable("worker", 10, 100, 1).is_err());
    let before = durable(&keeper);
    assert_eq!((before.0, before.2), (1, 0));
    drop(store);
    let store = ObjectiveStore::open(&keeper.database)
        .unwrap()
        .with_snapshot_admission(keeper.clone());
    store.reconcile_snapshot_commits().unwrap();
    let after = durable(&keeper);
    assert_eq!(before.1, after.1);
    assert_eq!((after.0, after.2), (1, 1));
    let db = rusqlite::Connection::open(&keeper.database).unwrap();
    let saved: (String, String) = db.query_row(
        "SELECT execution_run_id, identity_json FROM leaves JOIN lease_workspace_identities ON leaf_id=id",
        [], |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap();
    std::fs::rename(temp.path().join("workspace"), temp.path().join("original")).unwrap();
    if replace {
        std::fs::create_dir(temp.path().join("workspace")).unwrap();
    }
    let claim = store
        .claim_runnable("new-worker", 111, 100, 1)
        .unwrap()
        .remove(0);
    let state = keeper.state.lock().unwrap();
    assert_eq!(state.prepares, 1);
    assert_eq!(state.commits.len(), 3);
    assert_eq!(state.commits[0], state.commits[1]);
    assert_eq!(state.commits[2].0, claim.execution_run_id.unwrap());
    assert_eq!(state.commits[2].1, 2);
    assert_eq!(state.commits[2].2, "new-worker");
    assert_eq!(durable(&keeper).1, before.1);
    let recovered: (String, String) = db.query_row(
        "SELECT execution_run_id, identity_json FROM leaves JOIN lease_workspace_identities ON leaf_id=id",
        [], |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap();
    assert_eq!(saved, recovered);
    drop(state);
    assert!(store
        .claim_runnable("third", 112, 100, 4)
        .unwrap()
        .is_empty());
}

#[test]
fn execution_binding_requires_acknowledged_live_nonterminal_lease() {
    let (_temp, store, keeper) = fixture();
    keeper.state.lock().unwrap().lose_ack = true;
    assert!(store.claim_runnable("worker", 10, 100, 1).is_err());
    let run: String = rusqlite::Connection::open(&keeper.database)
        .unwrap()
        .query_row("SELECT execution_run_id FROM leaves", [], |r| r.get(0))
        .unwrap();
    assert!(store.retained_execution(&run, 11).is_err());
    store.reconcile_snapshot_commits().unwrap();
    let binding = store.retained_execution(&run, 11).unwrap().unwrap();
    assert_eq!(binding.lease.run_id, run);
    assert_eq!(binding.lease.generation, 1);
    assert_eq!(binding.lease.owner, "worker");
    assert_eq!(binding.lease.expires_ms, 110);
    assert_eq!(binding.snapshot.capability, "fixture-private-capability");
    let reopened = ObjectiveStore::open(&keeper.database).unwrap();
    let recovered = reopened.retained_execution(&run, 12).unwrap().unwrap();
    assert!(recovered.snapshot == binding.snapshot);
    let connection = rusqlite::Connection::open(&keeper.database).unwrap();
    connection
        .execute("UPDATE leaves SET lease_owner = 'replacement'", [])
        .unwrap();
    assert!(store.retained_execution(&run, 12).is_err());
    connection
        .execute("UPDATE leaves SET lease_owner = 'worker'", [])
        .unwrap();
    connection
        .execute(
            "UPDATE retained_workspace_snapshots SET run_id = 'wrong-run'",
            [],
        )
        .unwrap();
    assert!(store.retained_execution(&run, 12).is_err());
    connection
        .execute(
            "UPDATE retained_workspace_snapshots SET run_id = ?1",
            [&run],
        )
        .unwrap();
    assert!(store.retained_execution(&run, 12).unwrap().is_some());
    assert!(store.retained_execution(&run, 110).is_err());
    store
        .apply_control("snapshot", ControlAction::Cancel, "cancel", "operator", 13)
        .unwrap();
    assert!(store.retained_execution(&run, 14).is_err());
}

#[test]
fn lost_keeper_and_missing_configuration_never_prepare_replacement() {
    let (_temp, store, keeper) = fixture();
    store.claim_runnable("worker", 10, 100, 1).unwrap();
    let before = durable(&keeper).1;
    keeper.state.lock().unwrap().missing = true;
    assert!(store.claim_runnable("worker", 111, 100, 1).is_err());
    assert_eq!(keeper.state.lock().unwrap().prepares, 1);
    assert_eq!(durable(&keeper).1, before);
    let reopened = ObjectiveStore::open(&keeper.database).unwrap();
    assert!(reopened.claim_runnable("worker", 212, 100, 1).is_err());
    assert_eq!(durable(&keeper).0, 2);
}

#[test]
fn failed_preparation_rolls_back_claim_and_identity() {
    let (_temp, store, keeper) = fixture();
    keeper.state.lock().unwrap().reject_prepare = true;
    assert!(store.claim_runnable("worker", 10, 100, 1).is_err());
    let connection = rusqlite::Connection::open(&keeper.database).unwrap();
    let values: (i64, i64, i64) = connection
        .query_row(
            "SELECT attempt, (SELECT COUNT(*) FROM lease_workspace_identities),
         (SELECT COUNT(*) FROM retained_workspace_snapshots) FROM leaves",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(values, (0, 0, 0));
}

#[test]
fn durable_policy_prevents_unconfigured_reopen_even_after_ack() {
    let (_temp, store, keeper) = fixture();
    store.claim_runnable("worker", 10, 100, 1).unwrap();
    let reopened = ObjectiveStore::open(&keeper.database).unwrap();
    assert!(reopened.claim_runnable("worker", 111, 100, 1).is_err());
    assert_eq!(durable(&keeper).0, 1);
}

#[test]
fn cancel_after_lost_ack_does_not_retry_admission_or_block_claims() {
    let (_temp, store, keeper) = fixture();
    keeper.state.lock().unwrap().lose_ack = true;
    assert!(store.claim_runnable("worker", 10, 100, 1).is_err());
    store
        .apply_control("snapshot", ControlAction::Cancel, "cancel", "operator", 11)
        .unwrap();
    store.reconcile_snapshot_commits().unwrap();
    assert!(store
        .claim_runnable("worker", 12, 100, 1)
        .unwrap()
        .is_empty());
    assert_eq!(keeper.state.lock().unwrap().commits.len(), 1);
    assert_eq!(keeper.state.lock().unwrap().releases, 1);
}

#[test]
fn terminal_release_ack_loss_is_durable_and_never_recommits() {
    let (_temp, store, keeper) = fixture();
    store.claim_runnable("worker", 10, 100, 1).unwrap();
    store
        .apply_control("snapshot", ControlAction::Cancel, "cancel", "operator", 11)
        .unwrap();
    keeper.state.lock().unwrap().lose_release_ack = true;
    assert!(store.reconcile_snapshot_commits().is_err());
    let connection = rusqlite::Connection::open(&keeper.database).unwrap();
    let releases: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM retained_snapshot_releases",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        releases, 0,
        "lost acknowledgement was treated as cleanup proof"
    );
    drop(store);
    let store = ObjectiveStore::open(&keeper.database)
        .unwrap()
        .with_snapshot_admission(keeper.clone());
    store.reconcile_snapshot_commits().unwrap();
    store.reconcile_snapshot_commits().unwrap();
    assert_eq!(keeper.state.lock().unwrap().releases, 2);
    assert_eq!(keeper.state.lock().unwrap().commits.len(), 1);
}

#[test]
fn commit_recovery_uses_immutable_intent_not_mutable_lease() {
    let (_temp, store, keeper) = fixture();
    keeper.state.lock().unwrap().lose_ack = true;
    assert!(store.claim_runnable("worker", 10, 100, 1).is_err());
    rusqlite::Connection::open(&keeper.database)
        .unwrap()
        .execute(
            "UPDATE leaves SET lease_expires_ms = 999 WHERE id = 'leaf'",
            [],
        )
        .unwrap();
    store.reconcile_snapshot_commits().unwrap();
    let state = keeper.state.lock().unwrap();
    assert_eq!(state.commits[0], state.commits[1]);
    assert_eq!(state.commits[1].3, 110);
}
