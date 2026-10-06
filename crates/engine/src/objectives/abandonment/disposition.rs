//! Candidate Engine disposition persistence. Not wired to a maintenance command
//! or scheduler: keeper evidence and cessation must be verified by that caller.
use super::*;
use rusqlite::{params, Transaction};
use serde_json::Value;

#[allow(dead_code)]
pub(super) fn persist(
    tx: &mut Transaction<'_>,
    auth: &AbandonmentAuthorization,
    keeper_receipts: &[Value],
    now: i64,
) -> Result<Vec<Value>> {
    let (records, existing) = checked_records(tx, auth, keeper_receipts, now)?;
    if existing {
        return Ok(records);
    }
    // Roll back a prefix even if the outer caller commits after our error.
    let savepoint = tx.savepoint()?;
    for record in &records {
        let target: AbandonmentTarget = serde_json::from_value(record["target"].clone())?;
        savepoint.execute("INSERT INTO retained_snapshot_operator_abandonments(leaf_id,run_id,objective_id,event_id,record_json) VALUES(?1,?2,?3,?4,?5)",params![target.leaf_id,target.run_id,target.objective_id,auth.event_id,serde_json::to_string(record)?])?;
    }
    savepoint.commit()?;
    Ok(records)
}

/// Shared read-only immutable validation. The boolean distinguishes durable
/// replay from a prepared first application; callers must not confuse the two.
pub(super) fn checked_records(
    tx: &rusqlite::Connection,
    auth: &AbandonmentAuthorization,
    keeper_receipts: &[Value],
    now: i64,
) -> Result<(Vec<Value>, bool)> {
    use sha2::{Digest, Sha256};
    let set_digest = receipts::validate_keeper_receipt_set(auth, keeper_receipts)?;
    let stored: String = tx.query_row(
        "SELECT record_json FROM operator_abandonment_authorizations WHERE event_id=?1",
        [&auth.event_id],
        |r| r.get(0),
    )?;
    let bound: String = tx.query_row(
        "SELECT payload_digest FROM gateway_event_bindings WHERE event_id=?1",
        [&auth.event_id],
        |r| r.get(0),
    )?;
    if serde_json::from_str::<AbandonmentAuthorization>(&stored)? != *auth
        || bound != auth.payload_digest
    {
        bail!("Engine disposition durable authorization mismatch");
    }
    let authorization_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(auth)?));
    let prior: Vec<Value> = {
        let mut statement = tx.prepare("SELECT leaf_id,run_id,objective_id,event_id,record_json FROM retained_snapshot_operator_abandonments WHERE event_id=?1 ORDER BY run_id")?;
        let raw = statement
            .query_map([&auth.event_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut records = Vec::new();
        for (leaf, run, objective, event, json) in raw {
            let record: Value = serde_json::from_str(&json)?;
            if record["target"]["leaf_id"].as_str() != Some(leaf.as_str())
                || record["target"]["run_id"].as_str() != Some(run.as_str())
                || record["target"]["objective_id"].as_str() != Some(objective.as_str())
                || record["authorization"]["event_id"].as_str() != Some(event.as_str())
            {
                bail!("Engine disposition relational binding mismatch");
            }
            records.push(record);
        }
        records
    };
    let applied_at = if prior.is_empty() {
        now
    } else {
        prior[0]["applied_at_ms"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("invalid Engine disposition timestamp"))?
    };
    if applied_at < auth.authorized_at_ms || applied_at >= auth.expires_at_ms {
        bail!("Engine disposition authorization expired or not yet valid");
    }
    let mut expected = Vec::new();
    for target in &auth.manifest.targets {
        // Replay validates immutable admission-time evidence, not mutable live
        // history. This grants no exemption from current reservation checks.
        let historical = if prior.is_empty() {
            engine_record(tx, target)?
        } else {
            prior
                .iter()
                .find(|record| record["target"]["run_id"].as_str() == Some(&target.run_id))
                .and_then(|record| record.get("engine_record"))
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("partial Engine disposition set or missing original evidence")
                })?
        };
        if historical[8].as_str() != Some(&auth.operator_id)
            || record_digest(&historical)? != target.engine_record_digest
        {
            bail!("Engine disposition reservation fingerprint changed");
        }
        let receipt = keeper_receipts
            .iter()
            .find(|r| r["target"]["run_id"].as_str() == Some(&target.run_id))
            .expect("validated exact set");
        expected.push(serde_json::json!({
            "version":1,"disposition":"operator_abandonment","target":target,"engine_record":historical,
            "authorization":auth,"authorization_digest":authorization_digest,
            "keeper_receipt":receipt,"keeper_receipt_digest":record_digest(receipt)?,
            "keeper_receipt_set_digest":set_digest,"applied_at_ms":applied_at,
            "artifacts_retained":true,"relinquished_resumability":true,
            "engine_reservations_retired":true,"worker_cleanup_ack":false,"cleanup_verified":false
        }));
    }
    expected.sort_by(|a, b| {
        a["target"]["run_id"]
            .as_str()
            .cmp(&b["target"]["run_id"].as_str())
    });
    if !prior.is_empty() {
        if prior != expected {
            bail!("partial or conflicting Engine disposition set");
        }
        return Ok((prior, true));
    }
    Ok((expected, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, TransactionBehavior};

    fn fixture() -> (
        tempfile::TempDir,
        Connection,
        AbandonmentAuthorization,
        Vec<Value>,
    ) {
        let (temp, store, manifest) = super::super::tests::fixture();
        let payload = "c".repeat(64);
        store.bind_gateway_event("event", &payload).unwrap();
        let auth = store
            .authorize_abandonment("event", &payload, "operator", &manifest, 10)
            .unwrap();
        let digest = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(serde_json::to_vec(&auth).unwrap()))
        };
        // Synthetic records test persistence only, never keeper cessation.
        let receipts = auth.manifest.targets.iter().map(|target| serde_json::json!({
            "version":1,"disposition":"operator_abandonment","application_id":digest,
            "authorization_event_id":auth.event_id,"authorization_digest":digest,
            "operator_id":auth.operator_id,"target":target,"keeper_record":{"run":target.run_id},
            "mutation_provenance_digest":auth.manifest.mutation_provenance_digest,
            "original_paired_lineage_unrecoverable":true,"cessation_proof":"managed_cgroup_teardown_or_reboot",
            "worker_cleanup_ack":false,"cleanup_verified":false,"artifacts_retained":true,
            "engine_reservations_retired":false
        })).collect();
        let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        db.pragma_update(None, "foreign_keys", "ON").unwrap();
        (temp, db, auth, receipts)
    }

    #[test]
    fn trusted_application_refuses_other_event_target_before_keeper() {
        let (_temp, mut db, auth, receipts) = fixture();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let records = persist(&mut tx, &auth, &receipts, 11).unwrap();
        tx.rollback().unwrap();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let mut other = auth.clone();
        other.event_id = "other-event".into();
        tx.execute(
            "INSERT INTO operator_abandonment_authorizations VALUES(?1,?2)",
            params![other.event_id, serde_json::to_string(&other).unwrap()],
        )
        .unwrap();
        tx.execute("INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-4','run-4-attempt-1','objective','other-event',?1)", [serde_json::to_string(&records[4]).unwrap()]).unwrap();
        let mut calls = 0;
        let before = tx.total_changes();
        assert!(super::super::application::apply_in(
            &mut tx,
            &auth.event_id,
            &auth.operator_id,
            &auth.manifest,
            || 11,
            |_, _| {
                calls += 1;
                Ok(receipts.clone())
            }
        )
        .is_err());
        assert_eq!(calls, 0, "must refuse before keeper effects");
        assert_eq!(before, tx.total_changes());
    }

    #[test]
    fn trusted_application_checks_scope_and_expiry_before_keeper_then_replays() {
        let (_temp, mut db, auth, receipts) = fixture();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let records = super::super::application::apply_in(
            &mut tx,
            &auth.event_id,
            &auth.operator_id,
            &auth.manifest,
            || 11,
            |bound, first| {
                assert_eq!(bound, &auth);
                assert!(first);
                Ok(receipts.clone())
            },
        )
        .unwrap();
        assert_eq!(records.len(), 5);
        let before = tx.total_changes();
        assert_eq!(
            records,
            super::super::application::apply_in(
                &mut tx,
                &auth.event_id,
                &auth.operator_id,
                &auth.manifest,
                || auth.expires_at_ms + 1,
                |_, first| {
                    assert!(!first);
                    Ok(receipts.clone())
                }
            )
            .unwrap()
        );
        assert_eq!(before, tx.total_changes());
        tx.rollback().unwrap();
        for mode in ["expiry", "operator", "baseline", "post_keeper_expiry"] {
            let mut tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let mut baseline = auth.manifest.clone();
            if mode == "baseline" {
                baseline.targets[4].keeper_record_digest = "f".repeat(64);
            }
            let mut calls = 0;
            let mut ticks = 0;
            let result = super::super::application::apply_in(
                &mut tx,
                &auth.event_id,
                if mode == "operator" {
                    "wrong"
                } else {
                    &auth.operator_id
                },
                &baseline,
                || {
                    ticks += 1;
                    if mode == "expiry" || (mode == "post_keeper_expiry" && ticks > 1) {
                        auth.expires_at_ms
                    } else {
                        11
                    }
                },
                |_, _| {
                    calls += 1;
                    Ok(receipts.clone())
                },
            );
            assert!(result.is_err(), "{mode}");
            assert_eq!(calls, usize::from(mode == "post_keeper_expiry"));
            assert_eq!(
                tx.query_row(
                    "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            tx.rollback().unwrap();
        }
    }

    #[test]
    fn reservation_verifier_requires_complete_unchanged_set_without_writes() {
        let (_temp, mut db, auth, receipts) = fixture();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        assert!(super::super::reservations::verified_in(&tx)
            .unwrap()
            .is_empty());
        persist(&mut tx, &auth, &receipts, 11).unwrap();
        let before = tx.total_changes();
        let verified = super::super::reservations::verified_in(&tx).unwrap();
        assert_eq!(
            verified,
            auth.manifest
                .targets
                .iter()
                .map(|t| (t.leaf_id.clone(), t.run_id.clone()))
                .collect()
        );
        assert_eq!(before, tx.total_changes());
        // Immutable replay remains valid, but cannot exempt changed reservations.
        tx.execute(
            "UPDATE leaves SET workspace_root='/changed' WHERE id='leaf-4'",
            [],
        )
        .unwrap();
        assert!(persist(&mut tx, &auth, &receipts, auth.expires_at_ms + 1).is_ok());
        assert!(super::super::reservations::verified_in(&tx).is_err());
    }

    #[test]
    fn reservation_verifier_refuses_partial_sets() {
        let (_temp, mut db, auth, receipts) = fixture();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let records = persist(&mut tx, &auth, &receipts, 11).unwrap();
        tx.rollback().unwrap();
        for count in [1, 4] {
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            for record in &records[..count] {
                let target: AbandonmentTarget =
                    serde_json::from_value(record["target"].clone()).unwrap();
                tx.execute(
                    "INSERT INTO retained_snapshot_operator_abandonments VALUES(?1,?2,?3,?4,?5)",
                    params![
                        target.leaf_id,
                        target.run_id,
                        target.objective_id,
                        auth.event_id,
                        serde_json::to_string(record).unwrap()
                    ],
                )
                .unwrap();
            }
            let before = tx.total_changes();
            assert!(super::super::reservations::verified_in(&tx).is_err());
            assert_eq!(before, tx.total_changes());
            tx.rollback().unwrap();
        }
    }

    #[test]
    fn reservation_verifier_checks_all_immutable_members_and_current_rows() {
        for mode in [
            "flags",
            "set_digest",
            "applied_time",
            "historical",
            "keeper",
            "objective",
            "identity",
            "gateway",
        ] {
            let (_temp, mut db, auth, receipts) = fixture();
            let mut tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let mut records = persist(&mut tx, &auth, &receipts, 11).unwrap();
            tx.rollback().unwrap();
            match mode {
                "flags" => records[4]["cleanup_verified"] = true.into(),
                "set_digest" => records[4]["keeper_receipt_set_digest"] = "f".repeat(64).into(),
                "applied_time" => records[4]["applied_at_ms"] = auth.expires_at_ms.into(),
                "historical" => records[4]["engine_record"][0] = "other".into(),
                "keeper" => records[4]["keeper_receipt"]["target"]["run_id"] = "other".into(),
                _ => (),
            }
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            for record in &records {
                let target: AbandonmentTarget =
                    serde_json::from_value(record["target"].clone()).unwrap();
                tx.execute(
                    "INSERT INTO retained_snapshot_operator_abandonments VALUES(?1,?2,?3,?4,?5)",
                    params![
                        target.leaf_id,
                        target.run_id,
                        target.objective_id,
                        auth.event_id,
                        serde_json::to_string(record).unwrap()
                    ],
                )
                .unwrap();
            }
            match mode {
                "objective" => {
                    tx.execute("UPDATE objectives SET state='cancelled'", [])
                        .unwrap();
                }
                "identity" => {
                    tx.execute("UPDATE lease_workspace_identities SET identity_json='{}' WHERE leaf_id='leaf-4'", []).unwrap();
                }
                // Test-only missing durable ingress binding.
                "gateway" => {
                    tx.execute(
                        "DELETE FROM gateway_event_bindings WHERE event_id='event'",
                        [],
                    )
                    .unwrap();
                }
                _ => (),
            }
            let before = tx.total_changes();
            assert!(
                super::super::reservations::verified_in(&tx).is_err(),
                "{mode}"
            );
            assert_eq!(before, tx.total_changes());
            tx.rollback().unwrap();
        }
    }

    #[test]
    fn disposition_replay_rejects_relational_identifiers_disagreeing_with_json() {
        for field in ["leaf", "run"] {
            let (_temp, mut db, auth, receipts) = fixture();
            let mut tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let records = persist(&mut tx, &auth, &receipts, 11).unwrap();
            tx.rollback().unwrap();
            // Direct fixture corruption, not an operation available to production.
            for (index, record) in records.iter().enumerate() {
                let target: AbandonmentTarget =
                    serde_json::from_value(record["target"].clone()).unwrap();
                let leaf = if field == "leaf" {
                    format!("leaf-{}", (index + 1) % 5)
                } else {
                    target.leaf_id
                };
                let run = if field == "run" {
                    format!("wrong-{index}")
                } else {
                    target.run_id
                };
                db.execute(
                    "INSERT INTO retained_snapshot_operator_abandonments VALUES(?1,?2,?3,?4,?5)",
                    params![
                        leaf,
                        run,
                        target.objective_id,
                        auth.event_id,
                        serde_json::to_string(record).unwrap()
                    ],
                )
                .unwrap();
            }
            let changes = db.total_changes();
            let mut tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let error = persist(&mut tx, &auth, &receipts, 12).unwrap_err();
            assert!(error.to_string().contains("relational binding"), "{error}");
            tx.commit().unwrap();
            assert_eq!(changes, db.total_changes());
        }
    }

    #[test]
    fn disposition_replay_retains_original_engine_evidence_after_live_drift() {
        let (_temp, mut db, auth, receipts) = fixture();
        let original = engine_record(&db, &auth.manifest.targets[0]).unwrap();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let first = persist(&mut tx, &auth, &receipts, 11).unwrap();
        tx.commit().unwrap();
        // Simulate a later independent history mutation, not retirement permission.
        db.execute(
            "UPDATE leaves SET workspace_root='/changed' WHERE id='leaf-0'",
            [],
        )
        .unwrap();
        let changes = db.total_changes();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let replay = persist(&mut tx, &auth, &receipts, auth.expires_at_ms + 1).unwrap();
        assert_eq!(first, replay);
        assert_eq!(replay[0]["engine_record"], original);
        assert_ne!(
            engine_record(&tx, &auth.manifest.targets[0]).unwrap(),
            original
        );
        tx.commit().unwrap();
        assert_eq!(changes, db.total_changes());
    }

    #[test]
    fn disposition_set_is_immutable_replayable_and_preserves_history() {
        let (_temp, mut db, auth, receipts) = fixture();
        let before = engine_record(&db, &auth.manifest.targets[0]).unwrap();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let first = persist(&mut tx, &auth, &receipts, 11).unwrap();
        assert_eq!(first.len(), 5);
        tx.commit().unwrap();
        assert_eq!(
            before,
            engine_record(&db, &auth.manifest.targets[0]).unwrap()
        );
        let changes = db.total_changes();
        let mut tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        assert_eq!(
            first,
            persist(&mut tx, &auth, &receipts, auth.expires_at_ms + 1).unwrap()
        );
        tx.commit().unwrap();
        assert_eq!(changes, db.total_changes());
        for sql in [
            "UPDATE retained_snapshot_operator_abandonments SET record_json='{}'",
            "DELETE FROM retained_snapshot_operator_abandonments",
            "INSERT OR REPLACE INTO retained_snapshot_operator_abandonments SELECT * FROM retained_snapshot_operator_abandonments",
        ] {assert!(db.execute(sql,[]).is_err(),"{sql}");}
        assert_eq!(
            db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM leaves WHERE stage='execute' AND attempt=3",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
    }

    #[test]
    fn disposition_rejects_durable_auth_drift_and_partial_existing_sets() {
        for mode in ["auth", "fingerprint", "partial", "receipt"] {
            let (_temp, mut db, mut auth, mut receipts) = fixture();
            if mode == "auth" {
                auth.operator_id = "other".into();
            }
            if mode == "fingerprint" {
                db.execute(
                    "UPDATE leaves SET workspace_root='/changed' WHERE id='leaf-4'",
                    [],
                )
                .unwrap();
            }
            if mode == "receipt" {
                receipts[4]["cleanup_verified"] = true.into();
            }
            if mode == "partial" {
                db.execute("INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0','run-0-attempt-1','objective','event','{\"applied_at_ms\":11}')",[]).unwrap();
            }
            let before: i64 = db
                .query_row(
                    "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let mut tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            assert!(persist(&mut tx, &auth, &receipts, 11).is_err(), "{mode}");
            tx.commit().unwrap();
            assert_eq!(
                before,
                db.query_row(
                    "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn disposition_failure_on_fifth_target_and_expiry_leave_no_partial_set() {
        for fail_insert in [true, false] {
            let (_temp, mut db, auth, receipts) = fixture();
            if fail_insert {
                db.execute_batch("CREATE TRIGGER fail_fifth BEFORE INSERT ON retained_snapshot_operator_abandonments WHEN NEW.leaf_id='leaf-4' BEGIN SELECT RAISE(ABORT,'injected fifth failure'); END;").unwrap();
            }
            let mut tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let error = persist(
                &mut tx,
                &auth,
                &receipts,
                if fail_insert { 11 } else { auth.expires_at_ms },
            )
            .unwrap_err();
            assert!(
                error.to_string().contains(if fail_insert {
                    "injected fifth failure"
                } else {
                    "expired"
                }),
                "{error}"
            );
            // Even a caller that commits after an error must not retain a prefix.
            tx.commit().unwrap();
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM retained_snapshot_operator_abandonments",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        }
    }
}
