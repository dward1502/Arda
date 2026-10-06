use super::*;

#[test]
fn partial_keeper_set_refuses_before_cessation_or_insert() {
    for count in [1, 4] {
        let (mut source, auth) = fixture();
        let receipts = apply(
            &mut source,
            &auth,
            "owner",
            Path::new("/fixture"),
            b"provenance",
            |_| Ok(()),
        )
        .unwrap();
        let (mut db, _) = fixture();
        keeper_storage::migrate_abandonments(&db).unwrap();
        for receipt in &receipts[..count] {
            db.execute(
                "INSERT INTO snapshot_operator_abandonments VALUES(?1,?2)",
                rusqlite::params![
                    receipt["target"]["run_id"].as_str().unwrap(),
                    serde_json::to_string(receipt).unwrap()
                ],
            )
            .unwrap();
        }
        let before = db.total_changes();
        let mut calls = 0;
        let error = apply(
            &mut db,
            &auth,
            "owner",
            Path::new("/fixture"),
            b"provenance",
            |_| {
                calls += 1;
                Ok(())
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("partial"), "{error}");
        assert_eq!(calls, 0);
        assert_eq!(before, db.total_changes());
    }
}

pub(super) fn crash(point: &str) {
    if std::env::var("ARDA_TEST_ABANDONMENT_CRASH").ok().as_deref() == Some(point) {
        unsafe { libc::_exit(86) }
    }
}
#[test]
fn abrupt_keeper_abandonment_exit_is_all_or_none_and_replays() {
    let test = concat!(
        module_path!(),
        "::abrupt_keeper_abandonment_exit_is_all_or_none_and_replays"
    )
    .split_once("::")
    .unwrap()
    .1;
    if let Some(path) = std::env::var_os("ARDA_TEST_ABANDONMENT_DB") {
        let (_, auth) = fixture();
        let mut db = Connection::open(path).unwrap();
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .unwrap();
        apply(
            &mut db,
            &auth,
            "owner",
            Path::new("/fixture"),
            b"provenance",
            |_| Ok(()),
        )
        .unwrap();
        panic!("expected abrupt exit was not reached");
    }
    for point in ["insert", "precommit", "committed"] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("keeper.sqlite3");
        let (initial, auth) = fixture();
        initial
            .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
            .unwrap();
        drop(initial);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env("ARDA_TEST_ABANDONMENT_DB", &path)
            .env("ARDA_TEST_ABANDONMENT_CRASH", point)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(86),
            "{point}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let mut db = Connection::open(&path).unwrap();
        let n: i64 = db
            .query_row(
                "SELECT count(*) FROM snapshot_operator_abandonments",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, if point == "committed" { 5 } else { 0 });
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM snapshots WHERE state='lost' AND authority IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
        let receipts = apply(
            &mut db,
            &auth,
            "owner",
            Path::new("/fixture"),
            b"provenance",
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(receipts.len(), 5);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM snapshot_operator_abandonments",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
    }
}

use arda_engine::objectives::abandonment::{AbandonmentManifest, AbandonmentTarget};
use rusqlite::params;

fn fixture() -> (Connection, AbandonmentAuthorization) {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE snapshots(run TEXT PRIMARY KEY,workspace TEXT,identity TEXT,state TEXT,authority TEXT);
        CREATE TABLE snapshot_managed_ownership(run TEXT PRIMARY KEY,binding TEXT);
        CREATE TABLE runtime_allocations(run TEXT PRIMARY KEY,template_digest TEXT,source TEXT,state TEXT,device INTEGER,inode INTEGER,policy TEXT);
        CREATE TABLE owner_identity(id TEXT PRIMARY KEY);
        INSERT INTO owner_identity VALUES('owner');").unwrap();
    let mut targets = Vec::new();
    for n in 0..5 {
        let run = format!("historical-run-{n}");
        let binding = managed::Binding {
            owner: "owner".into(),
            uid: 1000,
            machine: "machine".into(),
            boot: "old-boot".into(),
            unit: "keeper.service".into(),
            invocation: "old-invocation".into(),
            cgroup: "/user.slice/fixture".into(),
            cgroup_device: 1,
            cgroup_inode: 2,
            runtime: "/fixture".into(),
        };
        db.execute(
            "INSERT INTO snapshots VALUES(?1,'damaged-workspace','damaged-identity','lost',NULL)",
            [&run],
        )
        .unwrap();
        db.execute(
            "INSERT INTO snapshot_managed_ownership VALUES(?1,?2)",
            params![run, serde_json::to_string(&binding).unwrap()],
        )
        .unwrap();
        db.execute("INSERT INTO runtime_allocations VALUES(?1,'template','historical-source','allocated',1,2,'policy')",[&run]).unwrap();
        let (record, binding, _) =
            keeper_reconcile::abandonment_record(&db, "owner", &run, Path::new("/fixture"))
                .unwrap();
        // Independent compatibility oracle: historical `reconcile inspect`
        // serializes Record and Binding in declaration order, not Value order.
        let inspected_bytes = format!("{{\"run\":{},\"workspace\":{},\"identity\":{},\"state\":{},\"authority\":{},\"allocation\":{},\"binding\":{}}}",
            record["run"],record["workspace"],record["identity"],record["state"],record["authority"],record["allocation"],serde_json::to_string(&binding).unwrap());
        targets.push(AbandonmentTarget {
            objective_id: format!("objective-{n}"),
            leaf_id: format!("leaf-{n}"),
            run_id: run,
            keeper_owner: "owner".into(),
            engine_record_digest: "a".repeat(64),
            keeper_record_digest: managed::digest(inspected_bytes.as_bytes()),
        });
    }
    (
        db,
        AbandonmentAuthorization {
            event_id: "event".into(),
            payload_digest: "b".repeat(64),
            operator_id: "operator".into(),
            authorized_at_ms: 1,
            expires_at_ms: i64::MAX,
            manifest: AbandonmentManifest {
                version: 1,
                reason: "bounded fixture".into(),
                mutation_provenance_digest: managed::digest(b"provenance"),
                original_paired_lineage_unrecoverable: true,
                relinquish_paused_resumability: true,
                artifacts_retained: true,
                worker_cleanup_ack: false,
                cleanup_verified: false,
                targets,
            },
        },
    )
}
// Characterization of the existing wall-clock commit gate, not installed acceptance.
#[test]
fn expiry_during_second_cessation_pass_rolls_back_all_tombstones() {
    let (mut db, mut auth) = fixture();
    auth.expires_at_ms = chrono::Utc::now().timestamp_millis() + 100;
    let mut calls = 0;
    let result = apply(
        &mut db,
        &auth,
        "owner",
        Path::new("/fixture"),
        b"provenance",
        |_| {
            calls += 1;
            if calls == 10 {
                // Cross the real clock boundary immediately before commit.
                let remaining = auth.expires_at_ms - chrono::Utc::now().timestamp_millis();
                if remaining > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(remaining as u64 + 1));
                }
            }
            Ok(())
        },
    );
    assert_eq!(calls, 10);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("expired before keeper commit"));
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM snapshot_operator_abandonments",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM snapshots WHERE state='lost' AND authority IS NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        5
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM runtime_allocations WHERE state='allocated'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        5
    );
}

#[test]
fn keeper_abandonment_is_atomic_replayable_and_preserves_damaged_history() {
    let (mut db, auth) = fixture();
    let mut calls = 0;
    let first = apply(
        &mut db,
        &auth,
        "owner",
        Path::new("/fixture"),
        b"provenance",
        |_| {
            calls += 1;
            Ok(())
        },
    )
    .unwrap();
    assert!(calls >= 5);
    assert_eq!(first.len(), 5);
    let replay = apply(
        &mut db,
        &auth,
        "owner",
        Path::new("/fixture"),
        b"provenance",
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(first, replay);
    // Cross-crate producer/consumer compatibility; no retirement is performed.
    let digest =
        arda_engine::objectives::abandonment::receipts::validate_keeper_receipt_set(&auth, &first)
            .unwrap();
    let mut reordered = replay.clone();
    reordered.reverse();
    assert_eq!(
        digest,
        arda_engine::objectives::abandonment::receipts::validate_keeper_receipt_set(
            &auth, &reordered
        )
        .unwrap()
    );
    assert_eq!(db.query_row("SELECT count(*) FROM runtime_allocations WHERE source='historical-source' AND state='allocated' AND device=1 AND inode=2 AND policy='policy'",[],|r|r.get::<_,i64>(0)).unwrap(),5);
    assert_eq!(db.query_row("SELECT count(*) FROM snapshots WHERE state='lost' AND workspace='damaged-workspace' AND identity='damaged-identity' AND authority IS NULL",[],|r|r.get::<_,i64>(0)).unwrap(),5);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM snapshot_operator_abandonments",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        5
    );
    for receipt in first {
        assert_eq!(receipt["worker_cleanup_ack"], false);
        assert_eq!(receipt["cleanup_verified"], false);
        assert_eq!(receipt["artifacts_retained"], true);
        assert_eq!(receipt["engine_reservations_retired"], false);
    }
    let mut changed = auth.clone();
    changed.event_id = "another-event".into();
    assert!(apply(
        &mut db,
        &changed,
        "owner",
        Path::new("/fixture"),
        b"provenance",
        |_| Ok(())
    )
    .is_err());
}
#[test]
fn keeper_abandonment_rejects_drift_provenance_and_failed_cessation_without_partial_writes() {
    for mode in ["drift", "provenance", "cessation", "second-pass"] {
        let (mut db, auth) = fixture();
        if mode == "drift" {
            db.execute(
                "UPDATE snapshots SET identity='later' WHERE run='historical-run-4'",
                [],
            )
            .unwrap();
        }
        let provenance = if mode == "provenance" {
            b"wrong".as_slice()
        } else {
            b"provenance".as_slice()
        };
        let mut count = 0;
        let result = apply(
            &mut db,
            &auth,
            "owner",
            Path::new("/fixture"),
            provenance,
            |_| {
                count += 1;
                if (mode == "cessation" && count == 5) || (mode == "second-pass" && count == 10) {
                    bail!("worker still present")
                }
                Ok(())
            },
        );
        assert!(result.is_err(), "{mode}");
        let present:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='snapshot_operator_abandonments')",[],|r|r.get(0)).unwrap();
        if present {
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM snapshot_operator_abandonments",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        }
    }
}
