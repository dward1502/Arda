//! Denial only. None of these checks grants a reservation exemption.
use super::*;
use rusqlite::Connection;

pub(crate) fn reject_in(
    db: &Connection,
    objective: Option<&str>,
    leaf: Option<&str>,
    run: Option<&str>,
) -> Result<()> {
    // Any matching row denies, including a partial set. Malformed bindings
    // fail closed globally; they never become permission to proceed.
    let mut statement = db.prepare("SELECT objective_id,leaf_id,run_id,record_json FROM retained_snapshot_operator_abandonments")?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (o, l, r, json) = row?;
        if objective == Some(o.as_str()) || leaf == Some(l.as_str()) || run == Some(r.as_str()) {
            bail!("operator abandonment permanently fences this historical scope");
        }
        let canonical: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaves l JOIN retained_workspace_snapshots s ON s.leaf_id=l.id
             WHERE l.id=?1 AND l.objective_id=?2 AND s.run_id=?3 AND l.execution_run_id=?3)",
            rusqlite::params![l, o, r], |row| row.get(0),
        )?;
        if !canonical {
            bail!("abandonment canonical binding conflicts with retained history");
        }
        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|_| anyhow::anyhow!("malformed abandonment record"))?;
        if value["target"]["objective_id"].as_str() != Some(o.as_str())
            || value["target"]["leaf_id"].as_str() != Some(l.as_str())
            || value["target"]["run_id"].as_str() != Some(r.as_str())
        {
            bail!("malformed abandonment relational binding");
        }
    }
    Ok(())
}

impl ObjectiveStore {
    /// Denial-only writer fence. Callers must separately establish execution
    /// authority. Never acquire this while already holding a RunStore lock.
    pub(crate) fn with_unabandoned_run<T>(
        &self,
        run: &str,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        reject_in(&tx, None, None, Some(run))?;
        let result = operation(&tx)?;
        tx.commit()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objectives::ControlAction;

    fn deny_fixture(db: &Connection) {
        db.execute_batch("INSERT INTO operator_abandonment_authorizations VALUES('event','{}');
            INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0','run-0-attempt-1','objective','event','{}');").unwrap();
    }

    #[test]
    fn abandonment_blocks_stale_context_preparation_and_reconciliation_failure() {
        use crate::objectives::{ClaimedLeaf, LeafStage};
        let (temp, store, manifest) = super::super::tests::fixture();
        let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        db.execute("UPDATE objectives SET state='running'", [])
            .unwrap();
        let claim = ClaimedLeaf {
            objective_id: "objective".into(),
            leaf_id: "leaf-0".into(),
            project_id: None,
            workspace_root: "/fixture".into(),
            authority: "read_only".into(),
            stage: LeafStage::Execute,
            attempt: 3,
            execution_run_id: Some("run-0-attempt-1".into()),
            lease_owner: "expired-worker".into(),
            lease_expires_ms: 5,
            current_receipt_digest: None,
            project_contract_digest: None,
            execution: None,
            dependency_receipts: vec![],
        };
        db.execute("UPDATE leaves SET context_bound=0 WHERE id='leaf-0'", [])
            .unwrap();
        assert!(store.can_prepare_resident_context(&claim).unwrap());
        deny_fixture(&db);
        let before = engine_record(&db, &manifest.targets[0]).unwrap();
        for result in [
            store.can_prepare_resident_context(&claim).map(|_| ()),
            store.fail_reconciliation(&claim, 3),
        ] {
            let error = result.unwrap_err();
            assert!(error.to_string().contains("abandonment"), "{error}");
        }
        assert_eq!(before, engine_record(&db, &manifest.targets[0]).unwrap());
    }

    #[test]
    fn abandonment_blocks_stage_receipt_replay_without_changing_evidence() {
        use crate::objectives::{ReceiptStage, StageReceipt};
        let (temp, store, _) = super::super::tests::fixture();
        let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        let receipt = StageReceipt {
            contract: "arda.hermes_execution_receipt.v4".into(),
            stage: ReceiptStage::Execute,
            digest: format!("sha256:{}", "a".repeat(64)),
            predecessor_digest: None,
            run_path: "data/runs/fixture".into(),
            provider: "fixture".into(),
            model: "fixture".into(),
            started_at_ms: 1,
            completed_at_ms: 2,
            verdict: "pass".into(),
            context_outcome_receipt_id: None,
            context_outcome_receipt_digest: None,
            binding_digest: None,
        };
        store
            .record_stage_receipt("leaf-0", "expired-worker", 3, receipt.clone(), 3)
            .unwrap();
        deny_fixture(&db);
        let error = store
            .record_stage_receipt("leaf-0", "expired-worker", 3, receipt, 4)
            .unwrap_err();
        assert!(error.to_string().contains("abandonment"), "{error}");
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM stage_receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn abandonment_blocks_objective_close_and_schedule_replays() {
        use crate::objectives::ScheduleSpec;
        let (temp, store, _) = super::super::tests::fixture();
        let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        let schedule = ScheduleSpec {
            id: "schedule".into(),
            objective_id: "objective".into(),
            next_wake_ms: 100,
            recurrence: None,
            idempotency_key: "schedule-key".into(),
        };
        store.put_schedule(schedule.clone(), 2).unwrap();
        db.execute("UPDATE leaves SET stage='complete'", [])
            .unwrap();
        store.close_objective("objective", "root", 3).unwrap();
        deny_fixture(&db);
        for result in [
            store.close_objective("objective", "root", 4).map(|_| ()),
            store
                .complete_objective_if_ready("objective", "root", 4)
                .map(|_| ()),
            store.put_schedule(schedule, 4).map(|_| ()),
        ] {
            let error = result.unwrap_err();
            assert!(error.to_string().contains("abandonment"), "{error}");
        }
        let updated: i64 = db
            .query_row(
                "SELECT updated_at_ms FROM objectives WHERE id='objective'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(updated, 3);
    }

    #[test]
    fn abandonment_reconciliation_preflights_entire_batch_before_keeper_effects() {
        use crate::objectives::snapshots::{RetainedSnapshot, SnapshotAdmission};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Keeper(Arc<AtomicUsize>);
        impl SnapshotAdmission for Keeper {
            fn prepare(&self, _: &str, _: &std::path::Path, _: &str) -> Result<RetainedSnapshot> {
                panic!("unexpected prepare")
            }
            fn commit(&self, _: &RetainedSnapshot, _: &str, _: i64, _: &str, _: i64) -> Result<()> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn release(&self, _: &RetainedSnapshot, _: &str) -> Result<()> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }
        for (scoped, malformed) in [(false, false), (true, false), (false, true), (true, true)] {
            let (temp, store, _) = super::super::tests::fixture();
            let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
            db.execute_batch("UPDATE leaves SET stage='cancelled';
                UPDATE retained_workspace_snapshots SET capability_json='{\"endpoint\":\"fixture\",\"capability\":\"fixture\",\"manifest_digest\":\"fixture\"}';
                INSERT INTO operator_abandonment_authorizations VALUES('event','{}');").unwrap();
            let record = if malformed {
                "{}".to_string()
            } else {
                serde_json::json!({"target": {"objective_id":"objective", "leaf_id":"leaf-4", "run_id":"run-4-attempt-1"}}).to_string()
            };
            db.execute("INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-4','run-4-attempt-1','objective','event',?1)", [&record]).unwrap();
            if !malformed {
                // Prove global validation and earlier rows do not cause refusal.
                reject_in(&db, None, None, None).unwrap();
                for n in 0..4 {
                    reject_in(
                        &db,
                        None,
                        Some(&format!("leaf-{n}")),
                        Some(&format!("run-{n}-attempt-1")),
                    )
                    .unwrap();
                }
            }
            let calls = Arc::new(AtomicUsize::new(0));
            let store = store.with_snapshot_admission(Arc::new(Keeper(calls.clone())));
            let result = if scoped {
                store.reconcile_snapshot_commits_for_leaf("leaf-4")
            } else {
                store.reconcile_snapshot_commits()
            };
            assert_eq!(
                calls.load(Ordering::SeqCst),
                0,
                "reconciliation contacted keeper before batch denial"
            );
            assert!(result.unwrap_err().to_string().contains("abandonment"));
            let releases: i64 = db
                .query_row("SELECT COUNT(*) FROM retained_snapshot_releases", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(releases, 0);
            let unchanged: i64 = db.query_row("SELECT COUNT(*) FROM retained_workspace_snapshots WHERE committed_generation=1", [], |r| r.get(0)).unwrap();
            assert_eq!(unchanged, 5);
        }
    }

    #[test]
    fn abandonment_prepare_refuses_before_saved_or_legacy_fallback() {
        let (temp, _store, _) = super::super::tests::fixture();
        let mut db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        db.execute_batch("INSERT INTO operator_abandonment_authorizations VALUES('event','{}');
            INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0','run-0-attempt-1','objective','event','{}');").unwrap();
        let tx = db.transaction().unwrap();
        let error = crate::objectives::snapshots::prepare(
            &tx,
            None,
            "leaf-0",
            std::path::Path::new("/fixture"),
            "identity",
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("abandonment"), "{error}");
    }

    #[test]
    fn abandonment_run_fence_excludes_retirement_and_rechecks_before_callback() {
        let (temp, store, _) = super::super::tests::fixture();
        let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        db.busy_timeout(std::time::Duration::ZERO).unwrap();
        db.execute(
            "INSERT INTO operator_abandonment_authorizations VALUES('event','{}')",
            [],
        )
        .unwrap();
        let sql = "INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0','run-0-attempt-1','objective','event','{}')";
        store
            .with_unabandoned_run("run-0-attempt-1", |_| {
                let error = db.execute(sql, []).unwrap_err();
                assert_eq!(
                    error.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::DatabaseBusy)
                );
                Ok(())
            })
            .unwrap();
        db.execute(sql, []).unwrap();
        let called = std::cell::Cell::new(false);
        let result = store.with_unabandoned_run("run-0-attempt-1", |_| {
            called.set(true);
            Ok(())
        });
        assert!(!called.get(), "abandoned run invoked fenced effect");
        assert!(result.unwrap_err().to_string().contains("abandonment"));
    }

    #[test]
    fn abandonment_denial_rejects_self_consistent_but_noncanonical_bindings() {
        for mode in ["objective", "run", "valid"] {
            let (temp, _store, _) = super::super::tests::fixture();
            let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
            db.pragma_update(None, "foreign_keys", "ON").unwrap();
            db.execute("INSERT INTO objectives(id,source_id,ingress_key,payload_digest,operator_id,text,priority,revision,state,created_at_ms,updated_at_ms) VALUES('other','other-source','other-key','digest','operator','fixture',1,1,'paused',1,1)", []).unwrap();
            db.execute(
                "INSERT INTO operator_abandonment_authorizations VALUES('event','{}')",
                [],
            )
            .unwrap();
            let objective = if mode == "objective" {
                "other"
            } else {
                "objective"
            };
            let run = if mode == "run" {
                "run-1-attempt-1"
            } else {
                "run-0-attempt-1"
            };
            let record = serde_json::json!({"target":{"objective_id":objective,"leaf_id":"leaf-0","run_id":run}});
            db.execute("INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0',?1,?2,'event',?3)", rusqlite::params![run,objective,record.to_string()]).unwrap();
            let changes = db.total_changes();
            if mode == "valid" {
                reject_in(&db, Some("other"), Some("leaf-1"), Some("run-1-attempt-1")).unwrap();
                assert!(reject_in(&db, None, None, Some("run-0-attempt-1")).is_err());
            } else {
                let error = if mode == "objective" {
                    reject_in(&db, Some("objective"), None, None).unwrap_err()
                } else {
                    reject_in(&db, None, None, Some("run-0-attempt-1")).unwrap_err()
                };
                assert!(
                    error.to_string().contains("abandonment canonical binding"),
                    "{error}"
                );
            }
            assert_eq!(changes, db.total_changes());
        }
    }

    #[test]
    fn abandonment_denies_control_replay_and_snapshot_fallback_without_history_writes() {
        let (temp, store, manifest) = super::super::tests::fixture();
        store
            .apply_control("objective", ControlAction::Pause, "pause", "operator", 10)
            .unwrap();
        let payload = "c".repeat(64);
        // Pause changes the fingerprint; authorize the actual current fixture.
        let db = Connection::open(temp.path().join("data/arda/objectives.sqlite3")).unwrap();
        let mut manifest = manifest;
        for target in &mut manifest.targets {
            target.engine_record_digest =
                record_digest(&engine_record(&db, target).unwrap()).unwrap();
        }
        store.bind_gateway_event("event", &payload).unwrap();
        store
            .authorize_abandonment("event", &payload, "operator", &manifest, 11)
            .unwrap();
        // A partial malformed disposition denies; it never grants retirement.
        db.execute("INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0','run-0-attempt-1','objective','event','{}')", []).unwrap();
        let before = engine_record(&db, &manifest.targets[0]).unwrap();
        for action in [
            ControlAction::Pause,
            ControlAction::Resume,
            ControlAction::Cancel,
            ControlAction::Reprioritize { priority: 9 },
            ControlAction::Approve { revision: 1 },
            ControlAction::Revise {
                text: "replacement".into(),
            },
            ControlAction::DeleteRecoveryContext {
                run_id: "run-0-attempt-1".into(),
            },
        ] {
            let error = store
                .apply_control("objective", action, "pause", "operator", 12)
                .unwrap_err();
            assert!(error.to_string().contains("abandonment"), "{error}");
        }
        let error = store
            .retained_execution("run-0-attempt-1", 12)
            .err()
            .expect("must refuse abandoned execution");
        assert!(error.to_string().contains("abandonment"), "{error}");
        assert_eq!(before, engine_record(&db, &manifest.targets[0]).unwrap());
        assert_eq!(
            db.query_row("SELECT count(*) FROM controls", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let reopened =
            ObjectiveStore::open_existing(temp.path().join("data/arda/objectives.sqlite3"))
                .unwrap();
        assert!(reopened
            .apply_control("objective", ControlAction::Pause, "pause", "operator", 13)
            .unwrap_err()
            .to_string()
            .contains("abandonment"));
    }
}
