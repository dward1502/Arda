//! Explicitly invoked isolated qualification. Synthetic authority/records, real OS checks.
//! The external operator owns service stop/mask/restore; these tests never do so.
use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
#[ignore = "requires isolated managed unit and explicit public-OS qualification"]
fn prepare_public_os_fixture() {
    let parent = std::path::PathBuf::from(std::env::var_os("ARDA_M4_OS_PARENT").unwrap());
    assert!(parent.is_absolute());
    assert!(parent
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("m4-public-os-"));
    let temp = tempfile::tempdir_in(&parent).unwrap();
    let (temp, store, mut manifest) = super::super::tests::fixture_in(temp);
    let root = temp.keep(); // Retain failed qualification evidence as well as success.
    let durable = root.join("keeper");
    let runtime = root.join("runtime");
    for path in [&durable, &runtime] {
        std::fs::create_dir(path).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    keeper_storage::initialize(&durable).unwrap();
    let db = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    let owner: String = db
        .query_row("SELECT id FROM owner_identity", [], |r| r.get(0))
        .unwrap();
    let binding = keeper_managed::capture(&db, &runtime).unwrap().unwrap();
    for (name, bytes) in [("endpoint.lock", ""), ("endpoint.owner", owner.as_str())] {
        let path = runtime.join(name);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    for target in &mut manifest.targets {
        db.execute("INSERT INTO snapshots VALUES(?1,'synthetic-damaged-workspace','synthetic-damaged-identity','lost',NULL)", [&target.run_id]).unwrap();
        keeper_managed::save(&db, &target.run_id, Some(&binding)).unwrap();
        target.keeper_owner = owner.clone();
        target.keeper_record_digest =
            keeper_reconcile::abandonment_record(&db, &owner, &target.run_id, &runtime)
                .unwrap()
                .2;
    }
    let provenance = b"SYNTHETIC ISOLATED TEST RECORDS; NOT INSTALLED AUTHORITY";
    manifest.mutation_provenance_digest = keeper_managed::digest(provenance);
    let payload = keeper_managed::digest(b"isolated-public-os-test-event");
    store
        .bind_gateway_event("isolated-public-os-test", &payload)
        .unwrap();
    store
        .authorize_abandonment(
            "isolated-public-os-test",
            &payload,
            "operator",
            &manifest,
            chrono::Utc::now().timestamp_millis(),
        )
        .unwrap();
    std::fs::write(root.join("provenance"), provenance).unwrap();
    std::fs::write(
        root.join("baseline.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("managed-binding.json"),
        serde_json::to_vec_pretty(&binding).unwrap(),
    )
    .unwrap();
    drop(db);
    drop(store);
    std::fs::write(
        parent.join("fixture-root"),
        root.as_os_str().as_encoded_bytes(),
    )
    .unwrap();
    println!("PUBLIC_OS_FIXTURE_PREPARED_SYNTHETIC_AUTHORITY");
}

#[test]
#[ignore = "readback of explicitly invoked isolated public-OS qualification"]
fn verify_public_os_fixture() {
    let root = std::path::PathBuf::from(std::env::var_os("ARDA_M4_OS_ROOT").unwrap());
    assert!(root
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("m4-public-os-"));
    let manifest: AbandonmentManifest =
        serde_json::from_slice(&std::fs::read(root.join("baseline.json")).unwrap()).unwrap();
    let engine = rusqlite::Connection::open_with_flags(
        root.join("data/arda/objectives.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let keeper = rusqlite::Connection::open_with_flags(
        root.join("keeper/owner.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let first: Value =
        serde_json::from_slice(&std::fs::read(root.join("first.json")).unwrap()).unwrap();
    let replay: Value =
        serde_json::from_slice(&std::fs::read(root.join("replay.json")).unwrap()).unwrap();
    assert_eq!(first, replay);
    assert_eq!(first["records"].as_array().unwrap().len(), 5);
    assert_eq!(first["engine_reservations_retired"], true);
    assert_eq!(first["worker_cleanup_ack"], false);
    assert_eq!(first["cleanup_verified"], false);
    assert_eq!(first["artifacts_retained"], true);
    assert_eq!(
        engine
            .query_row(
                "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        5
    );
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
            .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    for target in &manifest.targets {
        assert_eq!(
            record_digest(&engine_record(&engine, target).unwrap()).unwrap(),
            target.engine_record_digest
        );
        assert_eq!(
            keeper_reconcile::abandonment_record(
                &keeper,
                &target.keeper_owner,
                &target.run_id,
                &root.join("runtime")
            )
            .unwrap()
            .2,
            target.keeper_record_digest
        );
        let raw: String = engine
            .query_row(
                "SELECT record_json FROM retained_snapshot_operator_abandonments WHERE leaf_id=?1",
                [&target.leaf_id],
                |r| r.get(0),
            )
            .unwrap();
        let record: Value = serde_json::from_str(&raw).unwrap();
        assert!(first["records"].as_array().unwrap().contains(&record));
    }
    println!("PUBLIC_OS_EXACT_REPLAY_AND_HISTORY_VERIFIED");
}
