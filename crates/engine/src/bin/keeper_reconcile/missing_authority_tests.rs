use super::*;

fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE snapshots(run TEXT PRIMARY KEY,workspace TEXT,identity TEXT,state TEXT,authority TEXT);
        CREATE TABLE owner_identity(id TEXT);
        CREATE TABLE snapshot_managed_ownership(run TEXT PRIMARY KEY,binding TEXT);
        CREATE TABLE snapshot_reconciliations(request_id TEXT PRIMARY KEY,run TEXT UNIQUE,receipt TEXT);
        INSERT INTO owner_identity VALUES('owner');
        INSERT INTO snapshots VALUES('run','/historical/missing','identity','lost',NULL);").unwrap();
    let binding = managed::Binding {
        owner: "owner".into(),
        uid: 1,
        machine: "machine".into(),
        boot: "boot".into(),
        unit: "fixture.service".into(),
        invocation: "invocation".into(),
        cgroup: "/fixture".into(),
        cgroup_device: 1,
        cgroup_inode: 2,
        runtime: "/runtime".into(),
    };
    db.execute(
        "INSERT INTO snapshot_managed_ownership VALUES('run',?1)",
        [serde_json::to_string(&binding).unwrap()],
    )
    .unwrap();
    let original = record(&db, "owner", "run", Path::new("/runtime")).unwrap();
    let receipt = Receipt {
        version: 1,
        disposition: "terminal_revocation".into(),
        proof: "managed_cgroup_teardown_or_reboot".into(),
        worker_cleanup_ack: false,
        artifacts_retained: true,
        cleanup_verified: false,
        request_id: "request".into(),
        operator: "operator".into(),
        reason: "test-only terminal fixture".into(),
        owner: "owner".into(),
        run: "run".into(),
        record_digest: hash(&original).unwrap(),
        evidence_digest: "test-only-stop-evidence".into(),
        prior_state: "lost".into(),
        allocation_prior_state: None,
    };
    db.execute(
        "INSERT INTO snapshot_reconciliations VALUES('request','run',?1)",
        [serde_json::to_string(&receipt).unwrap()],
    )
    .unwrap();
    db.execute("UPDATE snapshots SET state='reconciled_revoked'", [])
        .unwrap();
    db
}
fn query(
    db: &Connection,
) -> Result<arda_engine::objectives::terminal_revocation::TerminalRevocationProof> {
    query_missing_authority_revocation(
        db,
        "run",
        "/historical/missing",
        "identity",
        Path::new("/runtime"),
    )
}

#[test]
fn null_authority_receipt_query_is_read_only_and_replayable() {
    let db = fixture();
    let before = db.total_changes();
    let proof = query(&db).unwrap();
    assert_eq!(proof.run, "run");
    assert_eq!(proof.workspace, "/historical/missing");
    assert_eq!(proof.identity, "identity");
    assert!(!proof.receipt.worker_cleanup_ack);
    assert!(!proof.receipt.cleanup_verified);
    assert!(proof.receipt.artifacts_retained);
    assert_eq!(proof, query(&db).unwrap());
    assert_eq!(db.total_changes(), before);
    assert!(db
        .query_row("SELECT authority IS NULL FROM snapshots", [], |r| r
            .get::<_, bool>(0))
        .unwrap());
}

#[test]
fn wrong_query_bindings_and_nonnull_authority_fail_closed() {
    let db = fixture();
    for (run, root, identity, runtime) in [
        ("other", "/historical/missing", "identity", "/runtime"),
        ("run", "/wrong", "identity", "/runtime"),
        ("run", "/historical/missing", "wrong", "/runtime"),
        ("run", "/historical/missing", "identity", "/wrong"),
    ] {
        assert!(
            query_missing_authority_revocation(&db, run, root, identity, Path::new(runtime))
                .is_err()
        );
    }
    db.execute("UPDATE snapshots SET authority='not even JSON'", [])
        .unwrap();
    assert!(query(&db).is_err());
    db.execute("UPDATE snapshots SET authority=NULL,state='lost'", [])
        .unwrap();
    assert!(query(&db).is_err());
    db.execute("UPDATE snapshots SET state='reconciled_revoked';", [])
        .unwrap();
    db.execute("UPDATE owner_identity SET id='wrong'", [])
        .unwrap();
    assert!(query(&db).is_err());
}

#[test]
fn altered_or_malformed_revocation_receipts_fail_closed() {
    for (key, value) in [
        ("version", serde_json::json!(2)),
        ("disposition", serde_json::json!("released")),
        ("proof", serde_json::json!("socket_absence")),
        ("worker_cleanup_ack", serde_json::json!(true)),
        ("cleanup_verified", serde_json::json!(true)),
        ("artifacts_retained", serde_json::json!(false)),
        ("record_digest", serde_json::json!("wrong")),
        ("owner", serde_json::json!("wrong")),
        ("run", serde_json::json!("wrong")),
        ("allocation_prior_state", serde_json::json!("allocated")),
        (
            "capability",
            serde_json::json!("unexpected-private-material"),
        ),
    ] {
        let db = fixture();
        let text: String = db
            .query_row("SELECT receipt FROM snapshot_reconciliations", [], |r| {
                r.get(0)
            })
            .unwrap();
        let mut receipt: serde_json::Value = serde_json::from_str(&text).unwrap();
        receipt[key] = value;
        db.execute(
            "UPDATE snapshot_reconciliations SET receipt=?1",
            [receipt.to_string()],
        )
        .unwrap();
        assert!(query(&db).is_err(), "accepted altered {key}");
    }
    for text in [
        "not json",
        r#"{"request_id":"historical","run":"run","operator":"operator","reason":"incomplete"}"#,
    ] {
        let db = fixture();
        db.execute("UPDATE snapshot_reconciliations SET receipt=?1", [text])
            .unwrap();
        let before = db.total_changes();
        assert!(query(&db).is_err());
        assert_eq!(db.total_changes(), before);
    }
    let db = fixture();
    db.execute("DELETE FROM snapshot_reconciliations", [])
        .unwrap();
    assert!(query(&db).is_err());
}

#[test]
fn allocation_lineage_must_match_durable_revocation_receipt() {
    for change in [
        None,
        Some("UPDATE runtime_allocations SET source='wrong'"),
        Some("UPDATE runtime_allocations SET device=99"),
        Some("UPDATE runtime_allocations SET inode=99"),
        Some("UPDATE runtime_allocations SET policy='wrong'"),
        Some("DELETE FROM runtime_allocations"),
        Some("UPDATE runtime_allocations SET state='allocated'"),
    ] {
        let db = fixture();
        db.execute_batch("CREATE TABLE runtime_allocations(run TEXT,template_digest TEXT,source TEXT,state TEXT,device INTEGER,inode INTEGER,policy TEXT); INSERT INTO runtime_allocations VALUES('run','template','source','allocated',1,2,'policy'); UPDATE snapshots SET state='lost';").unwrap();
        let original = record(&db, "owner", "run", Path::new("/runtime")).unwrap();
        let text: String = db
            .query_row("SELECT receipt FROM snapshot_reconciliations", [], |r| {
                r.get(0)
            })
            .unwrap();
        let mut receipt: Receipt = serde_json::from_str(&text).unwrap();
        receipt.record_digest = hash(&original).unwrap();
        receipt.allocation_prior_state = Some("allocated".into());
        db.execute(
            "UPDATE snapshot_reconciliations SET receipt=?1",
            [serde_json::to_string(&receipt).unwrap()],
        )
        .unwrap();
        db.execute_batch("UPDATE snapshots SET state='reconciled_revoked'; UPDATE runtime_allocations SET state='reconciled_revoked';").unwrap();
        if let Some(sql) = change {
            db.execute(sql, []).unwrap();
        }
        let before = db.total_changes();
        assert_eq!(query(&db).is_ok(), change.is_none(), "{change:?}");
        assert_eq!(db.total_changes(), before);
    }
}

#[test]
#[ignore = "explicit historical copies only; proves rejection, not installed acceptance"]
fn historical_copy_identity_mismatch_is_refused_without_writes() {
    let root = std::path::PathBuf::from(
        std::env::var("ARDA_TERMINAL_COPY_ROOT").expect("explicit isolated copy root"),
    );
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("gate1-terminal-copy-"));
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
    let engine = Connection::open_with_flags(root.join("objectives.sqlite3"), flags).unwrap();
    let keeper = Connection::open_with_flags(root.join("keeper.sqlite3"), flags).unwrap();
    let rows:Vec<(String,String,String)>=engine.prepare("SELECT s.leaf_id,s.run_id,i.identity_json FROM retained_workspace_snapshots s JOIN lease_workspace_identities i USING(leaf_id) WHERE s.run_id LIKE '%2c1b2ef727bfae22%'").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap();
    assert_eq!(rows.len(), 2);
    for (leaf, run, identity) in rows {
        let tuple: serde_json::Value = serde_json::from_str(&identity).unwrap();
        let binding = managed::load(&keeper, &run).unwrap();
        let error = query_missing_authority_revocation(
            &keeper,
            &run,
            tuple[1].as_str().unwrap(),
            &identity,
            &binding.runtime,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("terminal query binding mismatch"),
            "{error}"
        );
        println!("{leaf}: historical binding mismatch rejected");
    }
    assert_eq!(keeper.total_changes(), 0);
    assert_eq!(engine.total_changes(), 0);
}
