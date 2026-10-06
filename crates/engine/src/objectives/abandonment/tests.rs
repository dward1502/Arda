use super::*;
use rusqlite::{params, Connection};
pub(crate) fn fixture() -> (tempfile::TempDir, ObjectiveStore, AbandonmentManifest) {
    fixture_in(tempfile::tempdir().unwrap())
}
pub(crate) fn fixture_in(
    temp: tempfile::TempDir,
) -> (tempfile::TempDir, ObjectiveStore, AbandonmentManifest) {
    let path = temp.path().join("data/arda/objectives.sqlite3");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let store = ObjectiveStore::initialize(&path).unwrap();
    let db = Connection::open(path).unwrap();
    db.execute("INSERT INTO objectives(id,source_id,ingress_key,payload_digest,operator_id,text,priority,revision,state,created_at_ms,updated_at_ms) VALUES('objective','source','key','digest','operator','fixture',1,1,'paused',1,1)", []).unwrap();
    let mut targets = Vec::new();
    for n in 0..5 {
        let leaf = format!("leaf-{n}");
        let run = format!("run-{n}-attempt-1");
        db.execute("INSERT INTO leaves(id,objective_id,workspace_root,authority,stage,attempt,updated_at_ms,execution_run_id,lease_owner,lease_expires_ms) VALUES(?1,'objective','/fixture','read_only','execute',3,1,?2,'expired-worker',5)",params![leaf,run]).unwrap();
        db.execute(
            "INSERT INTO retained_workspace_snapshots VALUES(?1,?2,'{}',1)",
            params![leaf, run],
        )
        .unwrap();
        db.execute(
            "INSERT INTO lease_workspace_identities VALUES(?1,'[]')",
            [&leaf],
        )
        .unwrap();
        targets.push(AbandonmentTarget {
            objective_id: "objective".into(),
            leaf_id: leaf,
            run_id: run,
            keeper_owner: "00000000-0000-4000-8000-000000000001".into(),
            engine_record_digest: String::new(),
            keeper_record_digest: "a".repeat(64),
        });
        let target = targets.last_mut().unwrap();
        target.engine_record_digest = record_digest(&engine_record(&db, target).unwrap()).unwrap();
    }
    (
        temp,
        store,
        AbandonmentManifest {
            version: 1,
            reason: "explicit bounded operator abandonment".into(),
            mutation_provenance_digest: "b".repeat(64),
            original_paired_lineage_unrecoverable: true,
            relinquish_paused_resumability: true,
            artifacts_retained: true,
            worker_cleanup_ack: false,
            cleanup_verified: false,
            targets,
        },
    )
}
#[test]
fn marker_replacement_cannot_split_runtime_maintenance_exclusion() {
    for maintenance_first in [false, true] {
        let (temp, runtime, _) = fixture();
        let path = temp.path().join("data/arda/objectives.sqlite3");
        let runtime = if maintenance_first {
            drop(runtime);
            None
        } else {
            Some(runtime)
        };
        let maintenance = if maintenance_first {
            Some(ObjectiveStore::open_existing_maintenance(&path).unwrap())
        } else {
            None
        };
        let marker = path.with_file_name("objectives.sqlite3.authority.json");
        let replacement = marker.with_extension("replacement");
        std::fs::copy(&marker, &replacement).unwrap();
        std::fs::rename(&replacement, &marker).unwrap();
        if maintenance_first {
            assert!(
                ObjectiveStore::open_existing(&path).is_err(),
                "replacement admitted runtime during maintenance"
            );
        } else {
            assert!(
                ObjectiveStore::open_existing_maintenance(&path).is_err(),
                "replacement admitted maintenance during runtime"
            );
        }
        drop(runtime);
        drop(maintenance);
    }
}

#[test]
fn fifo_authority_rejected_without_blocking() {
    const NAME: &str = "objectives::abandonment::tests::fifo_authority_rejected_without_blocking";
    if let Some(path) = std::env::var_os("ARDA_FIFO_AUTHORITY_TEST_DB") {
        assert!(ObjectiveStore::open_existing(&path).is_err());
        assert!(ObjectiveStore::open_existing_maintenance(&path).is_err());
        println!("FIFO_REJECTION_VERIFIED");
        return;
    }
    let (temp, runtime, _) = fixture();
    drop(runtime);
    let path = temp.path().join("data/arda/objectives.sqlite3");
    let marker = path.with_file_name("objectives.sqlite3.authority.json");
    std::fs::remove_file(&marker).unwrap();
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(marker.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", NAME, "--nocapture"])
        .env("ARDA_FIFO_AUTHORITY_TEST_DB", &path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("FIFO authority acquisition blocked");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("FIFO_REJECTION_VERIFIED"));
}

#[test]
fn maintenance_exclusion_cross_process() {
    const NAME: &str = "objectives::abandonment::tests::maintenance_exclusion_cross_process";
    if let Some(path) = std::env::var_os("ARDA_MAINTENANCE_LOCK_TEST_DB") {
        let mode = std::env::var("ARDA_MAINTENANCE_LOCK_TEST_MODE").unwrap();
        match mode.as_str() {
            "runtime-held" => {
                assert!(ObjectiveStore::open_existing_maintenance(&path).is_err());
                ObjectiveStore::open_existing(&path).unwrap();
            }
            "maintenance-held" => {
                assert!(ObjectiveStore::open_existing(&path).is_err());
                assert!(ObjectiveStore::open_existing_maintenance(&path).is_err());
            }
            "free" => {
                ObjectiveStore::open_existing_maintenance(&path).unwrap();
            }
            _ => panic!("unknown child mode"),
        }
        println!("EXCLUSION_CHILD_VERIFIED:{mode}");
        return;
    }
    let (temp, runtime, _) = fixture();
    let path = temp.path().join("data/arda/objectives.sqlite3");
    let child = |mode: &str| {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("ARDA_MAINTENANCE_LOCK_TEST_DB", &path)
            .env("ARDA_MAINTENANCE_LOCK_TEST_MODE", mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains(&format!("EXCLUSION_CHILD_VERIFIED:{mode}")));
    };
    child("runtime-held");
    drop(runtime);
    let maintenance = ObjectiveStore::open_existing_maintenance(&path).unwrap();
    child("maintenance-held");
    drop(maintenance);
    child("free");
}

#[test]
fn maintenance_excludes_runtime_handles_and_clones() {
    let (temp, store, _) = fixture();
    let path = temp.path().join("data/arda/objectives.sqlite3");
    let clone = store.clone();
    assert!(
        ObjectiveStore::open_existing_maintenance(&path).is_err(),
        "maintenance admitted a live runtime"
    );
    drop(store);
    assert!(
        ObjectiveStore::open_existing_maintenance(&path).is_err(),
        "clone lost runtime exclusion"
    );
    drop(clone);
    let maintenance = ObjectiveStore::open_existing_maintenance(&path).unwrap();
    assert!(
        ObjectiveStore::open_existing(&path).is_err(),
        "runtime entered maintenance"
    );
    assert!(
        ObjectiveStore::open_existing_maintenance(&path).is_err(),
        "second maintenance owner admitted"
    );
    drop(maintenance);
    ObjectiveStore::open_existing(&path).unwrap();
}

#[test]
fn authenticated_authorization_is_immutable_replayable_and_not_a_release() {
    let (temp, store, manifest) = fixture();
    let payload = "c".repeat(64);
    store.bind_gateway_event("event", &payload).unwrap();
    let saved = store
        .authorize_abandonment("event", &payload, "operator", &manifest, 10)
        .unwrap();
    let replay = store
        .authorize_abandonment("event", &payload, "operator", &manifest, 11)
        .unwrap();
    assert_eq!(saved, replay);
    let reopened =
        ObjectiveStore::open_existing(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
    assert_eq!(
        saved,
        reopened
            .authorize_abandonment("event", &payload, "operator", &manifest, 12)
            .unwrap()
    );
    let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
    for sql in ["UPDATE operator_abandonment_authorizations SET record_json='{}'", "DELETE FROM operator_abandonment_authorizations", "INSERT OR REPLACE INTO operator_abandonment_authorizations SELECT * FROM operator_abandonment_authorizations"] {
        assert!(db.execute(sql,[]).is_err(),"{sql}");
    }
    for table in [
        "retained_snapshot_releases",
        "retained_snapshot_terminal_revocations",
    ] {
        let count: i64 = db
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM leaves WHERE stage='execute' AND attempt=3",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        5
    );
}
#[test]
fn rejected_maintenance_preserves_legacy_schema_rows_and_journal_mode() {
    for legacy in [true, false] {
        let (temp, store, _) = fixture();
        drop(store);
        let path = temp.path().join("data/arda/objectives.sqlite3");
        let db = Connection::open(&path).unwrap();
        if legacy {
            db.execute_batch("DROP TABLE operator_abandonment_authorizations;")
                .unwrap();
        } else {
            db.execute_batch("PRAGMA journal_mode=DELETE;").unwrap();
        }
        let capture = |db: &Connection| {
            let schema: Vec<(String, String)> = db
                .prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            let rows:String=db.query_row("SELECT json_group_array(json_array(id,objective_id,workspace_root,stage,attempt,lease_owner,lease_expires_ms,execution_run_id)) FROM leaves",[],|r|r.get(0)).unwrap();
            let journal: String = db
                .query_row("PRAGMA journal_mode", [], |r| r.get(0))
                .unwrap();
            (schema, rows, journal)
        };
        let before = capture(&db);
        assert!(ObjectiveStore::open_existing_maintenance(&path).is_err());
        assert_eq!(before, capture(&db));
    }
}
#[test]
fn offline_abandonment_gate_fences_writer_and_rejects_expiry_or_drift() {
    let (temp, store, manifest) = fixture();
    let payload = "c".repeat(64);
    store.bind_gateway_event("event", &payload).unwrap();
    let auth = store
        .authorize_abandonment("event", &payload, "operator", &manifest, 10)
        .unwrap();
    let path = temp.path().join("data/arda/objectives.sqlite3");
    drop(store);
    let store = ObjectiveStore::open_existing_maintenance(&path).unwrap();
    let result = store
        .with_fenced_abandonment_authorization("event", "operator", 11, |actual| {
            assert_eq!(actual, &auth);
            let db = Connection::open(&path)?;
            db.busy_timeout(std::time::Duration::ZERO)?;
            assert!(db.execute("UPDATE leaves SET attempt=4", []).is_err());
            Ok(42)
        })
        .unwrap();
    assert_eq!(result, 42);
    for (event, operator, now) in [
        ("missing", "operator", 11),
        ("event", "other", 11),
        ("event", "operator", auth.expires_at_ms),
        ("event", "operator", 9),
    ] {
        assert!(store
            .with_fenced_abandonment_authorization(event, operator, now, |_| -> Result<()> {
                panic!("invalid authority reached maintenance")
            })
            .is_err());
    }
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE leaves SET workspace_root='/changed' WHERE id='leaf-0'",
            [],
        )
        .unwrap();
    assert!(store
        .with_fenced_abandonment_authorization("event", "operator", 12, |_| -> Result<()> {
            panic!("drift reached maintenance")
        })
        .is_err());
    drop(store);
    let store = ObjectiveStore::open_existing(&path).unwrap();
    // Result retrieval is distinct from application: expiry/drift do not renew
    // or erase the original authorization.
    assert_eq!(
        auth,
        store
            .authorize_abandonment(
                "event",
                &payload,
                "operator",
                &manifest,
                auth.expires_at_ms + 1
            )
            .unwrap()
    );
}
#[test]
fn reservation_fingerprint_rejects_workspace_and_lease_intent_drift() {
    for sql in ["UPDATE leaves SET workspace_root='/different-root' WHERE id='leaf-0'",
        "INSERT INTO retained_snapshot_lease_intents(leaf_id,generation,lease_owner,lease_expires_ms) VALUES('leaf-0',99,'later-worker',999)",
        "INSERT INTO retained_snapshot_releases(leaf_id) VALUES('leaf-0')"] {
        let (temp,store,manifest)=fixture();
        let payload="c".repeat(64);
        store.bind_gateway_event("event",&payload).unwrap();
        let db=Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        db.execute(sql,[]).unwrap();
        let error=store.authorize_abandonment("event",&payload,"operator",&manifest,10).unwrap_err();
        assert!(error.to_string().contains("Engine record changed"),"{error}");
        assert_eq!(db.query_row("SELECT COUNT(*) FROM operator_abandonment_authorizations",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}
#[test]
fn unauthorized_or_drifting_abandonment_is_rejected() {
    let (temp, store, manifest) = fixture();
    let payload = "c".repeat(64);
    assert!(store
        .authorize_abandonment("event", &payload, "operator", &manifest, 10)
        .is_err());
    store.bind_gateway_event("event", &payload).unwrap();
    assert!(store
        .authorize_abandonment("event", &"d".repeat(64), "operator", &manifest, 10)
        .is_err());
    assert!(store
        .authorize_abandonment("event", &payload, "wrong-operator", &manifest, 10)
        .is_err());
    let mut bad = manifest.clone();
    bad.worker_cleanup_ack = true;
    assert!(store
        .authorize_abandonment("event", &payload, "operator", &bad, 10)
        .is_err());
    let mut bad = manifest.clone();
    bad.targets[0].run_id = "derived-attempt-3".into();
    assert!(store
        .authorize_abandonment("event", &payload, "operator", &bad, 10)
        .is_err());
    let mut bad = manifest.clone();
    bad.targets[0].engine_record_digest = "f".repeat(64);
    assert!(store
        .authorize_abandonment("event", &payload, "operator", &bad, 10)
        .is_err());
    let mut bad = manifest.clone();
    bad.targets[1] = bad.targets[0].clone();
    assert!(store
        .authorize_abandonment("event", &payload, "operator", &bad, 10)
        .is_err());
    let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM operator_abandonment_authorizations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    store
        .authorize_abandonment("event", &payload, "operator", &manifest, 10)
        .unwrap();
    let mut changed = manifest;
    changed.reason = "different scope".into();
    assert!(store
        .authorize_abandonment("event", &payload, "operator", &changed, 11)
        .is_err());
}
