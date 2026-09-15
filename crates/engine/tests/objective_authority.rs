use arda_engine::objectives::ObjectiveStore;

#[test]
fn missing_or_incomplete_marker_never_authorizes_runtime_creation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("objectives.sqlite3");
    assert!(ObjectiveStore::open_existing(&path).is_err());
    assert!(!path.exists());
    drop(ObjectiveStore::initialize(&path).unwrap());
    let marker = root.path().join("objectives.sqlite3.authority.json");
    let saved = std::fs::read(&marker).unwrap();
    std::fs::remove_file(&marker).unwrap();
    assert!(ObjectiveStore::open_existing(&path).is_err());
    assert!(ObjectiveStore::open(&path).is_err());
    assert!(ObjectiveStore::open_existing(&path).is_err());
    std::fs::write(&marker, b"").unwrap();
    assert!(ObjectiveStore::open(&path).is_err());
    std::fs::write(&marker, saved).unwrap();
    assert!(ObjectiveStore::open_existing(&path).is_ok());
}

#[test]
fn legacy_adoption_preserves_history_and_empty_adoption_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("objectives.sqlite3");
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_arda-objective-store"))
            .args(["adopt", "--database"])
            .arg(&path)
            .output()
            .unwrap()
    };
    assert!(!run().status.success());
    assert!(!path.exists());
    drop(rusqlite::Connection::open(&path).unwrap());
    assert!(!run().status.success());
    drop(ObjectiveStore::open(&path).unwrap());
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE objective_store_authority;
        CREATE TABLE legacy_history(value TEXT);
        INSERT INTO legacy_history VALUES('fixture-receipt-retain');",
        )
        .unwrap();
    drop(connection);
    std::fs::remove_file(root.path().join("objectives.sqlite3.authority.json")).unwrap();
    assert!(run().status.success());
    assert!(ObjectiveStore::open_existing(&path).is_ok());
    let connection = rusqlite::Connection::open(&path).unwrap();
    let history: String = connection
        .query_row("SELECT value FROM legacy_history", [], |row| row.get(0))
        .unwrap();
    assert_eq!(history, "fixture-receipt-retain");
}

#[test]
fn provisioning_cli_and_fresh_process_check_never_reset_authority() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("objectives.sqlite3");
    let run = |action: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_arda-objective-store"))
            .args([action, "--database"])
            .arg(&path)
            .output()
            .unwrap()
    };
    assert!(!run("check").status.success());
    assert!(!path.exists());
    assert!(run("init").status.success());
    assert!(run("check").status.success());
    let bytes = std::fs::read(&path).unwrap();
    assert!(!run("init").status.success());
    assert_eq!(bytes, std::fs::read(&path).unwrap());
    let saved = root.path().join("saved.sqlite3");
    std::fs::rename(&path, &saved).unwrap();
    assert!(!run("check").status.success());
    assert!(!run("init").status.success());
    assert!(!path.exists());
    std::fs::copy(&saved, &path).unwrap();
    assert!(!run("check").status.success());
    assert!(!run("adopt").status.success());
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(saved, &path).unwrap();
    assert!(run("check").status.success());
}

#[test]
fn reopening_cannot_recreate_missing_authority() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("objectives.sqlite3");
    drop(ObjectiveStore::open(&path).unwrap());
    let saved = root.path().join("saved.sqlite3");
    std::fs::rename(&path, &saved).unwrap();
    assert!(ObjectiveStore::open(&path).is_err());
    assert!(!path.exists());
    std::fs::rename(saved, &path).unwrap();
    assert!(ObjectiveStore::open(&path).is_ok());
}

#[test]
fn replacement_is_rejected_by_reopened_and_resident_stores() {
    for copied_original in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        let saved = root.path().join("saved.sqlite3");
        std::fs::rename(&path, &saved).unwrap();
        if copied_original {
            std::fs::copy(&saved, &path).unwrap();
        } else {
            std::fs::write(&path, []).unwrap();
        }
        let before = std::fs::read(&path).unwrap();
        assert!(store.list_objectives().is_err());
        assert!(ObjectiveStore::open(&path).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(saved, &path).unwrap();
        assert!(store.list_objectives().is_ok());
        assert!(ObjectiveStore::open(&path).is_ok());
    }
}
