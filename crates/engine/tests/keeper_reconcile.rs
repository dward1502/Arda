#![cfg(target_os = "linux")]
#[path = "../src/bin/keeper_storage/mod.rs"]
#[allow(dead_code)] // Shared production module has additional binary callers.
mod storage;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn reconciliation_cli_help_and_missing_arguments() {
    let binary = env!("CARGO_BIN_EXE_arda-snapshot-keeper");
    for args in [
        vec!["reconcile", "--help"],
        vec!["reconcile", "revoke", "--help"],
    ] {
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = Command::new(binary)
        .args(["reconcile", "inspect"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
}

#[test]
fn offline_inspection_preserves_uncertain_rows_and_rejects_owner_conflicts() {
    let durable = tempfile::tempdir_in("/var/tmp").unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    for path in [durable.path(), runtime.path()] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    storage::initialize(durable.path()).unwrap();
    let owner = fs::read_to_string(durable.path().join("owner.identity")).unwrap();
    {
        let (db, _owner, _endpoint) = storage::open(durable.path(), runtime.path()).unwrap();
        db.execute("INSERT INTO snapshots(run,workspace,identity,state) VALUES('uncertain','/original','identity','preparing')", []).unwrap();
        assert!(storage::offline::open(durable.path(), runtime.path(), &owner, false).is_err());
    }
    let before = fs::read(durable.path().join("owner.sqlite3")).unwrap();
    {
        let offline =
            storage::offline::open(durable.path(), runtime.path(), &owner, false).unwrap();
        let state: String = offline
            .db
            .query_row(
                "SELECT state FROM snapshots WHERE run='uncertain'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(state, "preparing");
        assert!(offline
            .db
            .execute("UPDATE snapshots SET state='released'", [])
            .is_err());
    }
    assert_eq!(
        before,
        fs::read(durable.path().join("owner.sqlite3")).unwrap()
    );
    assert!(storage::offline::open(
        durable.path(),
        runtime.path(),
        &uuid::Uuid::new_v4().to_string(),
        true
    )
    .is_err());
    let output = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .args(["reconcile", "inspect", "--durable"])
        .arg(durable.path())
        .arg("--runtime")
        .arg(runtime.path())
        .args(["--owner", &owner, "--run", "uncertain", "--json"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "historical rows must not gain fabricated provenance"
    );
    assert_eq!(
        before,
        fs::read(durable.path().join("owner.sqlite3")).unwrap()
    );
    let missing = durable.path().join("absent");
    assert!(storage::offline::open(&missing, runtime.path(), &owner, true).is_err());
    assert!(!missing.exists());
}
