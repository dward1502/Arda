//! Real two-store persistence/typed evidence; OS cessation is injected, not proven.
use super::*;

fn bounded_wait(
    child: std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::ExitStatus> {
    struct Reap(std::process::Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Reap(child);
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.0.try_wait()? {
            return Ok(status);
        }
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            "crash child timed out; killed and reaped"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn crash_wait_timeout_kills_and_reaps_child() {
    let child = std::process::Command::new("/usr/bin/sleep")
        .arg("60")
        .spawn()
        .unwrap();
    let pid = child.id();
    let started = std::time::Instant::now();
    assert!(bounded_wait(child, std::time::Duration::from_millis(50))
        .unwrap_err()
        .to_string()
        .contains("timed out"));
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "child must be reaped, not a zombie"
    );
}

fn crash_at(stage: &str, root: &Path) {
    if std::env::var("ARDA_APPLICATION_CRASH").as_deref() == Ok(stage) {
        let marker = std::env::var_os("ARDA_APPLICATION_MARKER").unwrap();
        std::fs::write(marker, root.as_os_str().as_encoded_bytes()).unwrap();
        std::process::exit(73);
    }
}

#[test]
#[ignore = "subprocess-only abrupt commit fixture"]
fn application_crash_child() {
    assert!(std::env::var_os("ARDA_APPLICATION_CRASH").is_some());
    exercise(None);
    panic!("crash hook not reached");
}

#[test]
fn abrupt_engine_commit_keeps_keeper_denial_distinct_from_retirement() {
    for (stage, expected_engine) in [("precommit", 0_i64), ("committed", 5)] {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("child-root");
        let module = module_path!().split_once("::").unwrap().1;
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("{module}::application_crash_child"),
                "--ignored",
                "--nocapture",
            ])
            .env("ARDA_APPLICATION_CRASH", stage)
            .env("ARDA_APPLICATION_MARKER", &marker)
            .env("ARDA_APPLICATION_PARENT", temp.path())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let status = bounded_wait(child, std::time::Duration::from_secs(30)).unwrap();
        assert_eq!(status.code(), Some(73));
        let root = std::path::PathBuf::from(std::fs::read_to_string(&marker).unwrap());
        let engine = rusqlite::Connection::open_with_flags(
            root.join("data/arda/objectives.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let keeper = rusqlite::Connection::open_with_flags(
            root.join("keeper.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            keeper
                .query_row(
                    "SELECT count(*) FROM snapshot_operator_abandonments",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            5
        );
        assert_eq!(
            engine
                .query_row(
                    "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            expected_engine
        );
        assert_eq!(
            engine
                .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            keeper
                .query_row(
                    "SELECT count(*) FROM snapshots WHERE state='lost' AND authority IS NULL",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            5
        );
        let baseline: AbandonmentManifest =
            serde_json::from_slice(&std::fs::read(root.join("baseline.json")).unwrap()).unwrap();
        for target in &baseline.targets {
            assert_eq!(
                record_digest(&engine_record(&engine, target).unwrap()).unwrap(),
                target.engine_record_digest
            );
            assert_eq!(
                keeper_reconcile::abandonment_record(
                    &keeper,
                    "owner",
                    &target.run_id,
                    Path::new("/fixture")
                )
                .unwrap()
                .2,
                target.keeper_record_digest
            );
        }
        // Parent TempDir owns the entire child subtree even on assertion failure.
        drop(engine);
        drop(keeper);
    }
}

#[test]
fn concrete_keeper_evidence_and_engine_commit_preserve_history_and_refuse_drift() {
    exercise(None);
}

#[test]
fn concrete_partial_keeper_sets_refuse_without_engine_or_keeper_writes() {
    for count in [1, 4] {
        exercise(Some(count));
    }
}

#[test]
fn live_lease_refuses_retirement_authorization_without_rewriting_it() {
    exercise_with_lease(None, true);
}

fn exercise(partial: Option<usize>) {
    exercise_with_lease(partial, false);
}

fn exercise_with_lease(partial: Option<usize>, live_lease: bool) {
    let temp = match std::env::var_os("ARDA_APPLICATION_PARENT") {
        Some(parent) => tempfile::tempdir_in(parent).unwrap(),
        None => tempfile::tempdir().unwrap(),
    };
    let (_root, store, mut manifest) = super::super::tests::fixture_in(temp);
    if live_lease {
        let db = store.connection().unwrap();
        db.execute("UPDATE leaves SET lease_expires_ms=?1", [i64::MAX - 1])
            .unwrap();
        for target in &mut manifest.targets {
            target.engine_record_digest =
                record_digest(&engine_record(&db, target).unwrap()).unwrap();
        }
    }
    let mut keeper = rusqlite::Connection::open(_root.path().join("keeper.sqlite3")).unwrap();
    keeper.execute_batch("CREATE TABLE snapshots(run TEXT PRIMARY KEY,workspace TEXT,identity TEXT,state TEXT,authority TEXT);
        CREATE TABLE snapshot_managed_ownership(run TEXT PRIMARY KEY,binding TEXT);
        CREATE TABLE owner_identity(id TEXT PRIMARY KEY); INSERT INTO owner_identity VALUES('owner');").unwrap();
    let runtime = Path::new("/fixture");
    for target in &mut manifest.targets {
        let binding = keeper_managed::Binding {
            owner: "owner".into(),
            uid: 1000,
            machine: "fixture".into(),
            boot: "old-boot".into(),
            unit: "keeper.service".into(),
            invocation: "old-invocation".into(),
            cgroup: "/fixture".into(),
            cgroup_device: 1,
            cgroup_inode: 2,
            runtime: runtime.into(),
        };
        keeper.execute("INSERT INTO snapshots VALUES(?1,'damaged-workspace','damaged-identity','lost',NULL)", [&target.run_id]).unwrap();
        keeper
            .execute(
                "INSERT INTO snapshot_managed_ownership VALUES(?1,?2)",
                rusqlite::params![target.run_id, serde_json::to_string(&binding).unwrap()],
            )
            .unwrap();
        target.keeper_owner = "owner".into();
        target.keeper_record_digest =
            keeper_reconcile::abandonment_record(&keeper, "owner", &target.run_id, runtime)
                .unwrap()
                .2;
    }
    manifest.mutation_provenance_digest = keeper_managed::digest(b"provenance");
    std::fs::write(
        _root.path().join("baseline.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    let payload = "c".repeat(64);
    store
        .bind_gateway_event("application-event", &payload)
        .unwrap();
    let authorization =
        store.authorize_abandonment("application-event", &payload, "operator", &manifest, now);
    if live_lease {
        assert!(authorization
            .unwrap_err()
            .to_string()
            .contains("without a current lease"));
        let db = store.connection().unwrap();
        for target in &manifest.targets {
            assert_eq!(
                record_digest(&engine_record(&db, target).unwrap()).unwrap(),
                target.engine_record_digest
            );
        }
        return;
    }
    let auth = authorization.unwrap();
    if let Some(count) = partial {
        let receipts =
            keeper_abandonment::apply(&mut keeper, &auth, "owner", runtime, b"provenance", |_| {
                Ok(())
            })
            .unwrap();
        // Deliberately corrupt only the fixture: production tombstones are immutable.
        keeper
            .execute_batch("DROP TABLE snapshot_operator_abandonments;")
            .unwrap();
        keeper_storage::migrate_abandonments(&keeper).unwrap();
        for receipt in &receipts[..count] {
            keeper
                .execute(
                    "INSERT INTO snapshot_operator_abandonments VALUES(?1,?2)",
                    rusqlite::params![
                        receipt["target"]["run_id"].as_str().unwrap(),
                        serde_json::to_string(receipt).unwrap()
                    ],
                )
                .unwrap();
        }
        let mut db = store.connection().unwrap();
        let mut tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let before = (tx.total_changes(), keeper.total_changes());
        let error = apply_in(
            &mut tx,
            &auth.event_id,
            &auth.operator_id,
            &manifest,
            || now,
            |auth, first| {
                keeper_evidence(
                    &mut keeper,
                    auth,
                    "owner",
                    runtime,
                    b"provenance",
                    first,
                    |_| panic!("partial set must precede cessation"),
                )
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("partial"), "{error}");
        assert_eq!(before, (tx.total_changes(), keeper.total_changes()));
        assert_eq!(
            tx.query_row(
                "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        return;
    }
    let mut db = store.connection().unwrap();
    let mut tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let error = apply_in(
        &mut tx,
        &auth.event_id,
        &auth.operator_id,
        &manifest,
        || now,
        |auth, first| {
            keeper_evidence(
                &mut keeper,
                auth,
                "owner",
                runtime,
                b"provenance",
                first,
                |_| bail!("injected cessation refusal"),
            )
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("cessation refusal"), "{error}");
    assert_eq!(
        keeper
            .query_row(
                "SELECT count(*) FROM snapshot_operator_abandonments",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        tx.query_row(
            "SELECT count(*) FROM retained_snapshot_operator_abandonments",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let records = apply_in(
        &mut tx,
        &auth.event_id,
        &auth.operator_id,
        &manifest,
        || now,
        |auth, first| {
            keeper_evidence(
                &mut keeper,
                auth,
                "owner",
                runtime,
                b"provenance",
                first,
                |_| Ok(()),
            )
        },
    )
    .unwrap();
    assert_eq!(records.len(), 5);
    assert_eq!(
        super::super::reservations::verified_in(&tx).unwrap().len(),
        5
    );
    crash_at("precommit", _root.path());
    tx.commit().unwrap();
    crash_at("committed", _root.path());
    let workspace = _root.path().join("independent-workspace");
    std::fs::create_dir(&workspace).unwrap();
    db.execute("INSERT INTO objectives(id,source_id,ingress_key,payload_digest,operator_id,text,priority,revision,state,created_at_ms,updated_at_ms) VALUES('fresh-objective','fresh-source','fresh-key','digest','operator','independent',1,1,'approved',1,1)", []).unwrap();
    db.execute("INSERT INTO leaves(id,objective_id,workspace_root,authority,stage,attempt,updated_at_ms) VALUES('fresh-leaf','fresh-objective',?1,'execute_with_approval','execute',0,1)", [workspace.to_str().unwrap()]).unwrap();
    let claimed = store.claim_runnable("new-worker", now, 10000, 1).unwrap();
    assert_eq!(
        claimed.len(),
        1,
        "verified retirement must not reserve independent work"
    );
    assert_eq!(claimed[0].leaf_id, "fresh-leaf");
    let before = db.total_changes();
    // Global reconciliation skips only the verified complete retirement set.
    store.reconcile_snapshot_commits().unwrap();
    assert!(store.reconcile_snapshot_commits_for_leaf("leaf-0").is_err());
    assert_eq!(before, db.total_changes());
    for target in &manifest.targets {
        assert_eq!(
            record_digest(&engine_record(&db, target).unwrap()).unwrap(),
            target.engine_record_digest
        );
    }
    assert_eq!(
        keeper
            .query_row(
                "SELECT count(*) FROM snapshots WHERE state='lost' AND authority IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        5
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let mut tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let replay = apply_in(
        &mut tx,
        &auth.event_id,
        &auth.operator_id,
        &manifest,
        || auth.expires_at_ms + 1,
        |auth, first| {
            keeper_evidence(
                &mut keeper,
                auth,
                "owner",
                runtime,
                b"provenance",
                first,
                |_| panic!("immutable replay cannot require new worker authority"),
            )
        },
    )
    .unwrap();
    assert_eq!(records, replay);
    keeper
        .execute(
            "UPDATE snapshots SET identity='changed' WHERE run=?1",
            [&manifest.targets[4].run_id],
        )
        .unwrap();
    assert!(apply_in(
        &mut tx,
        &auth.event_id,
        &auth.operator_id,
        &manifest,
        || now,
        |auth, first| keeper_evidence(
            &mut keeper,
            auth,
            "owner",
            runtime,
            b"provenance",
            first,
            |_| Ok(())
        )
    )
    .unwrap_err()
    .to_string()
    .contains("durable keeper evidence changed"));
    tx.rollback().unwrap();
    db.execute(
        "UPDATE leaves SET workspace_root='/changed' WHERE id='leaf-4'",
        [],
    )
    .unwrap();
    assert!(store
        .reconcile_snapshot_commits()
        .unwrap_err()
        .to_string()
        .contains("abandonment reservation verification failed"));
    assert_eq!(
        db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
