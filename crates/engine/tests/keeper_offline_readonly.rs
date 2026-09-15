#![cfg(target_os = "linux")]
#[path = "../src/bin/keeper_storage/mod.rs"]
#[allow(dead_code)] // Only the offline portion of the shared module is exercised.
mod storage;
use std::{
    collections::BTreeMap, ffi::OsString, fs, os::unix::fs::PermissionsExt, path::Path,
    process::Command,
};

fn inventory(path: &Path) -> BTreeMap<OsString, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn inspect_preserves_every_source_file_and_recovers_committed_wal() {
    // A subprocess exits without SQLite close/checkpoint to leave a genuine WAL.
    if let Ok(root) = std::env::var("ARDA_OFFLINE_WAL_FIXTURE") {
        let db = rusqlite::Connection::open(Path::new(&root).join("owner.sqlite3")).unwrap();
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA wal_autocheckpoint=0;",
        )
        .unwrap();
        db.execute("INSERT INTO snapshots(run,workspace,identity,state) VALUES('wal-only','/original','identity','preparing')", []).unwrap();
        std::process::exit(0);
    }
    for crash_wal in [false, true] {
        let durable = tempfile::tempdir_in("/var/tmp").unwrap();
        fs::set_permissions(durable.path(), fs::Permissions::from_mode(0o700)).unwrap();
        storage::initialize(durable.path()).unwrap();
        let owner = fs::read_to_string(durable.path().join("owner.identity")).unwrap();
        let runtime = durable.path().join("absent-runtime");
        if crash_wal {
            assert!(Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "inspect_preserves_every_source_file_and_recovers_committed_wal",
                    "--nocapture"
                ])
                .env("ARDA_OFFLINE_WAL_FIXTURE", durable.path())
                .status()
                .unwrap()
                .success());
            assert!(
                durable
                    .path()
                    .join("owner.sqlite3-wal")
                    .metadata()
                    .unwrap()
                    .len()
                    > 0
            );
        }
        let before = inventory(durable.path());
        {
            let offline = storage::offline::open(durable.path(), &runtime, &owner, false).unwrap();
            let count: i64 = offline
                .db
                .query_row(
                    "SELECT count(*) FROM snapshots WHERE run='wal-only'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, i64::from(crash_wal));
            assert_eq!(inventory(durable.path()), before);
        }
        assert_eq!(inventory(durable.path()), before);
        let output = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
            .args(["reconcile", "inspect", "--durable"])
            .arg(durable.path())
            .arg("--runtime")
            .arg(&runtime)
            .args(["--owner", &owner, "--run", "absent", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(inventory(durable.path()), before);
    }
}
