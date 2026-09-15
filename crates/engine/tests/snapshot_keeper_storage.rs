#![cfg(target_os = "linux")]
#[path = "../src/bin/keeper_storage/mod.rs"]
#[allow(dead_code)] // Shared production module has additional binary callers.
mod storage;

fn private_tempdir(path: &str) -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir_in(path).unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

#[test]
fn owner_requires_explicit_persistent_initialization_and_preserves_missing_journal() {
    let volatile = private_tempdir("/dev/shm");
    assert!(storage::initialize(volatile.path()).is_err());
    assert!(!volatile.path().join("owner.sqlite3").exists());
    assert!(storage::validate_durable(std::path::Path::new("/usr")).is_err());
    let durable = private_tempdir("/var/tmp");
    let runtime = private_tempdir("/dev/shm");
    assert!(storage::open(durable.path(), runtime.path()).is_err());
    storage::initialize(durable.path()).unwrap();
    assert!(storage::initialize(durable.path()).is_err());
    let owner = storage::open(durable.path(), runtime.path()).unwrap();
    owner.0.execute("INSERT INTO snapshots(run,workspace,identity,state) VALUES('ambiguous','workspace','identity','preparing')", []).unwrap();
    drop(owner);
    let owner = storage::open(durable.path(), runtime.path()).unwrap();
    let state: String = owner
        .0
        .query_row("SELECT state FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(state, "lost");
    drop(owner);
    std::fs::remove_file(durable.path().join("owner.sqlite3")).unwrap();
    assert!(storage::open(durable.path(), runtime.path()).is_err());
    assert!(!durable.path().join("owner.sqlite3").exists());
    assert!(storage::initialize(durable.path()).is_err());
}
#[test]
fn endpoint_cannot_be_taken_by_a_different_durable_owner() {
    let first = private_tempdir("/var/tmp");
    let second = private_tempdir("/var/tmp");
    let runtime = private_tempdir("/dev/shm");
    storage::initialize(first.path()).unwrap();
    storage::initialize(second.path()).unwrap();
    let owner = storage::open(first.path(), runtime.path()).unwrap();
    let socket =
        std::os::unix::net::UnixListener::bind(runtime.path().join("keeper.sock")).unwrap();
    use std::os::unix::fs::MetadataExt;
    let inode = std::fs::metadata(runtime.path().join("keeper.sock"))
        .unwrap()
        .ino();
    assert!(storage::open(second.path(), runtime.path()).is_err());
    assert_eq!(
        std::fs::metadata(runtime.path().join("keeper.sock"))
            .unwrap()
            .ino(),
        inode
    );
    drop(owner);
    assert!(storage::open(second.path(), runtime.path()).is_err());
    assert!(storage::open(first.path(), runtime.path()).is_ok());
    drop(socket);
}
