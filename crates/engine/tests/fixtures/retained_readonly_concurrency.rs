use super::*;

fn add(store: &ObjectiveStore, root: &Path, authority: &str) {
    store
        .create_authenticated_objective(
            NewObjective {
                id: "other".into(),
                source_id: "other-source".into(),
                idempotency_key: "other-ingress".into(),
                operator_id: "operator".into(),
                text: "Inspect independent source".into(),
                priority: 0,
                projects: vec![ProjectAuthority {
                    project_id: "second".into(),
                    contract_digest: "sha256:second".into(),
                }],
                leaves: vec![NewLeaf {
                    id: "other-leaf".into(),
                    project_id: Some("second".into()),
                    workspace_root: root.to_str().unwrap().into(),
                    authority: authority.into(),
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
            "approve-other",
            "operator",
            4,
        )
        .unwrap();
}

#[test]
fn scoped_reconciliation_leaves_other_intents_and_paused_objective_untouched() {
    let (temp, store, keeper) = fixture();
    let root = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    add(&store, &root, "read_only");
    keeper.state.lock().unwrap().lose_ack = true;
    assert!(store.claim_runnable("worker", 10, 100, 2).is_err());
    store
        .apply_control("snapshot", ControlAction::Pause, "pause", "operator", 20)
        .unwrap();
    let before = keeper.state.lock().unwrap().commits.len();
    assert!(store.reconcile_snapshot_commits_for_leaf("").is_err());
    assert!(store
        .reconcile_snapshot_commits_for_leaf("missing")
        .is_err());
    assert_eq!(keeper.state.lock().unwrap().commits.len(), before);
    store.reconcile_snapshot_commits_for_leaf("leaf").unwrap();
    store.reconcile_snapshot_commits_for_leaf("leaf").unwrap();
    assert_eq!(keeper.state.lock().unwrap().commits.len(), before + 1);
    let db = rusqlite::Connection::open(&keeper.database).unwrap();
    let generations: Vec<(String, i64)> = db.prepare("SELECT leaf_id, committed_generation FROM retained_workspace_snapshots ORDER BY leaf_id").unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap()
        .collect::<rusqlite::Result<_>>().unwrap();
    assert_eq!(
        generations,
        vec![("leaf".into(), 1), ("other-leaf".into(), 0)]
    );
    // Simulate already verified terminal projection; reconciliation is not a
    // receipt writer and must not complete or resume the parent objective.
    db.execute("UPDATE leaves SET stage='complete', lease_owner=NULL, lease_expires_ms=NULL WHERE id='leaf'", []).unwrap();
    store.reconcile_snapshot_commits_for_leaf("leaf").unwrap();
    store.reconcile_snapshot_commits_for_leaf("leaf").unwrap();
    assert_eq!(keeper.state.lock().unwrap().releases, 1);
    assert_eq!(keeper.state.lock().unwrap().commits.len(), before + 1);
    let state: String = db
        .query_row(
            "SELECT state FROM objectives WHERE id='snapshot'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "paused");
    store.reconcile_snapshot_commits().unwrap();
    assert_eq!(keeper.state.lock().unwrap().commits.len(), before + 2);
}

#[test]
fn fresh_retained_readers_share_capacity() {
    let (temp, store, keeper) = fixture();
    let root = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    add(&store, &root, "read_only");
    let claims = store.claim_runnable("worker", 10, 100, 2).unwrap();
    assert_eq!(claims.len(), 2);
    assert!(claims.iter().all(|c| c.attempt == 1));
    let state = keeper.state.lock().unwrap();
    assert_eq!(state.prepares, 2);
    assert_eq!(state.commits.len(), 2);
}

#[test]
fn fresh_reader_can_join_retained_reader_without_reopening_its_path() {
    let (temp, store, keeper) = fixture();
    let first = store.claim_runnable("first", 10, 100, 1).unwrap();
    assert_eq!(first.len(), 1);
    std::fs::remove_dir(temp.path().join("workspace")).unwrap();
    let root = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    add(&store, &root, "read_only");
    let second = store.claim_runnable("second", 11, 100, 1).unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].leaf_id, "other-leaf");
    assert_eq!(keeper.state.lock().unwrap().prepares, 2);
}

#[test]
fn retained_writer_and_unknown_authority_exclude_readers_even_when_paused_expired() {
    for authority in ["execute", "unknown"] {
        let (temp, store, keeper) = fixture();
        rusqlite::Connection::open(&keeper.database)
            .unwrap()
            .execute(
                "UPDATE leaves SET authority=?1 WHERE id='leaf'",
                [authority],
            )
            .unwrap();
        let root = temp.path().join("other");
        std::fs::create_dir(&root).unwrap();
        add(&store, &root, "read_only");
        assert_eq!(store.claim_runnable("first", 10, 100, 2).unwrap().len(), 1);
        assert!(store
            .claim_runnable("second", 11, 100, 2)
            .unwrap()
            .is_empty());
        store
            .apply_control("snapshot", ControlAction::Pause, "pause", "operator", 20)
            .unwrap();
        assert!(store
            .claim_runnable("second", 111, 100, 2)
            .unwrap()
            .is_empty());
        assert_eq!(keeper.state.lock().unwrap().prepares, 1);
    }
}

#[test]
fn reader_recovery_is_exclusive_in_both_directions() {
    let (temp, store, _) = fixture();
    assert_eq!(store.claim_runnable("first", 10, 100, 1).unwrap().len(), 1);
    let root = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    add(&store, &root, "read_only");
    assert_eq!(store.claim_runnable("second", 50, 100, 1).unwrap().len(), 1);
    assert!(store
        .claim_runnable("recover", 111, 100, 2)
        .unwrap()
        .is_empty());
    let recovery = store.claim_runnable("recover", 151, 100, 2).unwrap();
    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].attempt, 2);
    assert!(store
        .claim_runnable("other", 152, 100, 2)
        .unwrap()
        .is_empty());
}

#[test]
fn equal_paths_and_fresh_batch_aliases_remain_excluded() {
    let (temp, store, _) = fixture();
    add(&store, &temp.path().join("workspace"), "read_only");
    assert_eq!(store.claim_runnable("first", 10, 100, 2).unwrap().len(), 1);
    assert!(store
        .claim_runnable("second", 11, 100, 2)
        .unwrap()
        .is_empty());
    let (temp, store, _) = fixture();
    let alias = temp.path().join("alias");
    std::os::unix::fs::symlink(temp.path().join("workspace"), &alias).unwrap();
    add(&store, &alias, "read_only");
    assert_eq!(store.claim_runnable("first", 10, 100, 2).unwrap().len(), 1);
    // Read/read aliasing across separate calls is permitted, not evidence of
    // physical independence. Any writer still hits the global reservation.
    assert_eq!(store.claim_runnable("second", 11, 100, 1).unwrap().len(), 1);
}
