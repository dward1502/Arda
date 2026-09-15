#![cfg(target_os = "linux")]
#[path = "../src/bin/keeper_storage/mod.rs"]
#[allow(dead_code)] // Shared production module has additional binary callers.
mod storage;
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn journal_and_sidecars_stay_on_directory_pin_across_replacement() {
    let temp = tempfile::tempdir_in("/var/tmp").unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let original = temp.path().join("owner");
    fs::create_dir(&original).unwrap();
    fs::set_permissions(&original, fs::Permissions::from_mode(0o700)).unwrap();
    storage::initialize(&original).unwrap();
    let original_identity = fs::read(original.join("owner.identity")).unwrap();
    let pin = fs::File::open(&original).unwrap();
    let retained = temp.path().join("retained");
    fs::rename(&original, &retained).unwrap();
    fs::create_dir(&original).unwrap();
    fs::set_permissions(&original, fs::Permissions::from_mode(0o700)).unwrap();
    storage::initialize(&original).unwrap();
    let (db, _lock, _endpoint) =
        storage::open_pinned(pin, fs::File::open(runtime.path()).unwrap()).unwrap();
    let actual: String = db
        .query_row("SELECT id FROM owner_identity", [], |row| row.get(0))
        .unwrap();
    assert_eq!(actual.as_bytes(), original_identity);
    let filename: String = db
        .query_row(
            "SELECT file FROM pragma_database_list WHERE name='main'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        filename.starts_with("/proc/self/fd/"),
        "VFS resolved the authority back to a pathname: {filename}"
    );
    db.execute_batch("CREATE TABLE pin_test(value TEXT); INSERT INTO pin_test VALUES('before');")
        .unwrap();
    let twice = temp.path().join("twice");
    fs::rename(&retained, &twice).unwrap();
    db.execute("INSERT INTO pin_test VALUES('after')", [])
        .unwrap();
    assert!(twice.join("owner.sqlite3-wal").exists());
    assert!(twice.join("owner.sqlite3-shm").exists());
    let replacement = rusqlite::Connection::open(original.join("owner.sqlite3")).unwrap();
    let leaked: i64 = replacement
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='pin_test'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leaked, 0);
    drop(db);
    let saved = rusqlite::Connection::open(twice.join("owner.sqlite3")).unwrap();
    let rows: i64 = saved
        .query_row("SELECT count(*) FROM pin_test", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 2);
}
