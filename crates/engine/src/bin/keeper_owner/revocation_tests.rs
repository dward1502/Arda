use super::*;
use arda_engine::objectives::keeper_client::KeeperFailure;
use std::os::unix::fs::PermissionsExt;

#[test]
fn abandoned_run_is_denied_before_saved_state_and_allocation() {
    let temp = tempfile::tempdir().unwrap();
    let durable = temp.path().join("durable");
    let runtime = temp.path().join("runtime");
    for p in [&durable, &runtime] {
        fs::create_dir(p).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    crate::keeper_storage::initialize(&durable).unwrap();
    let (db, dl, rl) = crate::keeper_storage::open(&durable, &runtime).unwrap();
    // Test-only insertion is not an authenticated abandonment. The offline
    // writer must retain full evidence; denial uses the permanent run key.
    db.execute(
        "INSERT INTO snapshot_operator_abandonments VALUES('abandoned', '{}')",
        [],
    )
    .unwrap();
    for sql in [
        "UPDATE snapshot_operator_abandonments SET receipt_json='replaced' WHERE run='abandoned'",
        "DELETE FROM snapshot_operator_abandonments WHERE run='abandoned'",
        "INSERT OR REPLACE INTO snapshot_operator_abandonments VALUES('abandoned','replaced')",
    ] {
        assert!(db.execute(sql, []).is_err(), "{sql}");
    }
    // Keep the malformed historical row byte-identical even across startup's
    // ordinary lost-worker sweep. It must never become execution authority.
    db.execute("INSERT INTO snapshots VALUES('abandoned','wrong-workspace','wrong-identity','prepared',NULL)", []).unwrap();
    let mut owner = Owner {
        managed: None,
        reservations: None,
        runtime_policy: None,
        db,
        durable: durable.clone(),
        runtime: runtime.clone(),
        worker: "unused".into(),
        children: BTreeMap::new(),
        failed_qualifications: Default::default(),
    };
    fn assert_denied(owner: &mut Owner) {
        let snapshot = RetainedSnapshot {
            endpoint: "unused".into(),
            capability: "test".into(),
            manifest_digest: "a".repeat(64),
        };
        let requests = [
            KeeperRequest::Prepare {
                run: "abandoned".into(),
                workspace: "/absent".into(),
                identity: "invented".into(),
            },
            KeeperRequest::Commit {
                snapshot: snapshot.clone(),
                lease: arda_engine::objectives::snapshot_protocol::Lease {
                    run_id: "abandoned".into(),
                    generation: 3,
                    owner: "test".into(),
                    expires_ms: i64::MAX,
                },
            },
            KeeperRequest::Release {
                snapshot,
                run: "abandoned".into(),
            },
            KeeperRequest::QueryTerminalRevocation {
                run: "abandoned".into(),
                workspace: "wrong".into(),
                identity: "wrong".into(),
            },
        ];
        let before = owner.db.total_changes();
        for request in requests {
            let response = owner.handle(request).expect("typed refusal");
            assert!(!response.ok);
            assert_eq!(
                serde_json::to_value(response.failure).unwrap(),
                "operator_abandoned"
            );
            assert!(response.snapshot.is_none() && response.terminal_revocation.is_none());
        }
        let err = owner
            .run_policy("abandoned")
            .err()
            .expect("allocation must refuse even without template");
        assert!(err.to_string().contains("operator abandoned"), "{err}");
        assert_eq!(owner.db.total_changes(), before);
        assert!(owner.children.is_empty());
    }
    assert_denied(&mut owner);
    drop(owner);
    drop(dl);
    drop(rl);
    let (db, _dl, _rl) = crate::keeper_storage::open(&durable, &runtime).unwrap();
    let mut reopened = Owner {
        managed: None,
        reservations: None,
        runtime_policy: None,
        db,
        durable,
        runtime,
        worker: "unused".into(),
        children: BTreeMap::new(),
        failed_qualifications: Default::default(),
    };
    assert_denied(&mut reopened);
    let saved: (String, String, String, Option<String>) = reopened
        .db
        .query_row(
            "SELECT workspace,identity,state,authority FROM snapshots WHERE run='abandoned'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        saved,
        (
            "wrong-workspace".into(),
            "wrong-identity".into(),
            "prepared".into(),
            None
        )
    );
    reopened
        .db
        .execute("DELETE FROM snapshots WHERE run='abandoned'", [])
        .unwrap();
    assert_denied(&mut reopened);
}
#[test]
fn release_classifies_only_null_revoked_authority_and_keeps_capability_checks() {
    let temp = tempfile::tempdir().unwrap();
    let durable = temp.path().join("durable");
    let runtime = temp.path().join("runtime");
    for p in [&durable, &runtime] {
        fs::create_dir(p).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    crate::keeper_storage::initialize(&durable).unwrap();
    let (db, _dl, _rl) = crate::keeper_storage::open(&durable, &runtime).unwrap();
    let mut owner = Owner {
        managed: None,
        reservations: None,
        runtime_policy: None,
        db,
        durable,
        runtime,
        worker: "unused".into(),
        children: BTreeMap::new(),
        failed_qualifications: Default::default(),
    };
    owner.db.execute("INSERT INTO snapshots VALUES('run','/historical','identity','reconciled_revoked',NULL)",[]).unwrap();
    let snapshot = RetainedSnapshot {
        endpoint: "unused".into(),
        capability: "test-only".into(),
        manifest_digest: "a".repeat(64),
    };
    let release = || KeeperRequest::Release {
        snapshot: snapshot.clone(),
        run: "run".into(),
    };
    let before = owner.db.total_changes();
    let response = owner.handle(release()).unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.failure,
        Some(KeeperFailure::MissingRevokedAuthority)
    );
    assert!(response.snapshot.is_none() && response.terminal_revocation.is_none());
    assert_eq!(owner.db.total_changes(), before);
    owner
        .db
        .execute("UPDATE snapshots SET authority='malformed'", [])
        .unwrap();
    assert!(owner.handle(release()).is_err());
    owner
        .db
        .execute(
            "UPDATE snapshots SET authority=?1,state='released'",
            [serde_json::to_string(&snapshot).unwrap()],
        )
        .unwrap();
    let mut wrong = snapshot.clone();
    wrong.capability = "wrong".into();
    assert!(owner
        .handle(KeeperRequest::Release {
            snapshot: wrong,
            run: "run".into()
        })
        .is_err());
    assert!(owner.handle(release()).unwrap().ok);
    owner
        .db
        .execute("UPDATE snapshots SET authority=NULL,state='lost'", [])
        .unwrap();
    assert!(owner.handle(release()).is_err());
}
