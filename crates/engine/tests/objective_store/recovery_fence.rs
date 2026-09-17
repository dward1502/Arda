use super::*;
use arda_core::run_graph::{NodeId, RunId};
use arda_engine::runs::{RecoveryBindings, RecoveryGrant, RECOVERY_WINDOW_MS};

fn fixture() -> (TempDir, ObjectiveStore, RecoveryGrant, String) {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objective.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("fenced", "fenced-ingress");
    let owner = input.operator_id.clone();
    let leaf = input.leaves[0].id.clone();
    store.create_authenticated_objective(input, 1).unwrap();
    store
        .apply_control(
            "fenced",
            ControlAction::Approve { revision: 1 },
            "approve",
            &owner,
            2,
        )
        .unwrap();
    store
        .apply_control("fenced", ControlAction::Pause, "pause", &owner, 3)
        .unwrap();
    // Establish only the lineage needed by this control-fence test. This is not
    // a retained-admission fixture or evidence that a live run is recoverable.
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE leaves SET execution_run_id='run', stage='verify', authority='read_only' WHERE id=?1", [&leaf]).unwrap();
    let digest = format!("sha256:{}", "a".repeat(64));
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let grant = RecoveryGrant {
        authenticated_event_id: "test-event".into(),
        authenticated_payload_digest: digest.clone(),
        activated_at_unix_ms: now,
        expires_at_unix_ms: now + RECOVERY_WINDOW_MS,
        bindings: RecoveryBindings {
            objective_id: "fenced".into(),
            objective_revision: 1,
            stop_generation: 1,
            leaf_id: leaf,
            run_id: RunId::new("run").unwrap(),
            failed_event_sequence: 17,
            failed_event_digest: digest.clone(),
            execute_receipt_digest: digest.clone(),
            project_contract_digest: digest.clone(),
            context_capsule_digest: digest.clone(),
            context_use_receipt_digest: digest.clone(),
            retained_authority_digest: digest,
            verify_node_id: NodeId::new("verify").unwrap(),
            review_node_id: NodeId::new("review").unwrap(),
            close_node_id: NodeId::new("close").unwrap(),
            prior_verify_starts: 2,
            provider_start_ceilings: std::collections::BTreeMap::from([
                ("verify".into(), 3),
                ("review".into(), 2),
            ]),
        },
    };
    (temp, store, grant, owner)
}

#[test]
fn fence_rejects_changed_lineage_stops_and_time_before_callback() {
    let (_temp, store, grant, owner) = fixture();
    store
        .with_recovery_control_fence(&owner, &grant, |_| Ok(()))
        .unwrap();
    let mut invalid = grant.clone();
    invalid.bindings.run_id = RunId::new("other").unwrap();
    assert!(store
        .with_recovery_control_fence(&owner, &invalid, |_| -> anyhow::Result<()> {
            panic!("invalid lineage entered callback")
        })
        .is_err());
    invalid = grant.clone();
    invalid.activated_at_unix_ms = 0;
    invalid.expires_at_unix_ms = RECOVERY_WINDOW_MS;
    assert!(store
        .with_recovery_control_fence(&owner, &invalid, |_| -> anyhow::Result<()> {
            panic!("expired grant entered callback")
        })
        .is_err());
    assert!(store
        .with_recovery_control_fence("other-owner", &grant, |_| -> anyhow::Result<()> {
            panic!("wrong owner entered callback")
        })
        .is_err());
    store
        .apply_control("fenced", ControlAction::Pause, "later-pause", &owner, 3)
        .unwrap();
    assert!(store
        .with_recovery_control_fence(&owner, &grant, |_| -> anyhow::Result<()> {
            panic!("superseded grant entered callback")
        })
        .is_err());
}

#[test]
fn concurrent_stop_waits_for_fenced_operation_then_invalidates_grant() {
    let (_temp, store, grant, owner) = fixture();
    let handle = store
        .with_recovery_control_fence(&owner, &grant, |_| {
            let other = store.clone();
            let owner = owner.clone();
            let (started, entered) = std::sync::mpsc::channel();
            let (done, finished) = std::sync::mpsc::channel();
            let handle = std::thread::spawn(move || {
                started.send(()).unwrap();
                other
                    .apply_control(
                        "fenced",
                        ControlAction::Pause,
                        "concurrent-pause",
                        &owner,
                        3,
                    )
                    .unwrap();
                done.send(()).unwrap();
            });
            entered.recv().unwrap();
            assert!(matches!(
                finished.recv_timeout(std::time::Duration::from_millis(50)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ));
            Ok((handle, finished))
        })
        .unwrap();
    handle
        .1
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    handle.0.join().unwrap();
    assert!(store
        .with_recovery_control_fence(&owner, &grant, |_| Ok(()))
        .is_err());
}

#[test]
fn admission_intent_survives_restart_and_journal_success_before_sqlite_failure() {
    let (temp, store, grant, owner) = fixture();
    assert!(store
        .record_recovery_admission_intent(&owner, &grant, |_, _| Ok(()))
        .is_err());
    store
        .bind_gateway_event(
            &grant.authenticated_event_id,
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
        )
        .unwrap();
    let saved = store
        .record_recovery_admission_intent(&owner, &grant, |_, _| Ok(()))
        .unwrap();
    let reopened = ObjectiveStore::open_existing(temp.path().join("objective.sqlite3")).unwrap();
    let mut shifted = grant.clone();
    shifted.activated_at_unix_ms -= 1;
    shifted.expires_at_unix_ms -= 1;
    assert_eq!(
        reopened
            .record_recovery_admission_intent(&owner, &shifted, |_, _| Ok(()))
            .unwrap(),
        saved
    );
    let journal =
        arda_engine::runs::RunStore::open(temp.path().join("runs"), grant.bindings.run_id.clone())
            .unwrap();
    let failed: anyhow::Result<()> = reopened.with_recorded_recovery_admission(
        &owner,
        &grant.authenticated_event_id,
        |_, saved| {
            journal.activate_recovery(saved.clone())?;
            anyhow::bail!("injected crash after durable journal activation")
        },
    );
    assert!(failed.is_err());
    let replay = reopened
        .with_recorded_recovery_admission(&owner, &grant.authenticated_event_id, |_, saved| {
            Ok(journal.activate_recovery(saved.clone())?)
        })
        .unwrap();
    assert_eq!(replay, grant);
    reopened
        .apply_control(
            "fenced",
            ControlAction::Pause,
            "superseding-pause",
            &owner,
            3,
        )
        .unwrap();
    assert!(reopened
        .with_recorded_recovery_admission(
            &owner,
            &grant.authenticated_event_id,
            |_, _| -> anyhow::Result<()> { panic!("revoked admission entered callback") }
        )
        .is_err());
    shifted.bindings.stop_generation += 1;
    assert!(reopened
        .record_recovery_admission_intent(&owner, &shifted, |_, _| Ok(()))
        .is_err());
}

#[test]
fn admission_validation_failure_conflicts_and_envelope_tampering_fail_closed() {
    let (temp, store, grant, owner) = fixture();
    store
        .bind_gateway_event(
            &grant.authenticated_event_id,
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
        )
        .unwrap();
    assert!(store
        .record_recovery_admission_intent(&owner, &grant, |_, _| anyhow::bail!("policy rejected"))
        .is_err());
    let db = rusqlite::Connection::open(temp.path().join("objective.sqlite3")).unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM recovery_admissions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
    store
        .record_recovery_admission_intent(&owner, &grant, |_, _| Ok(()))
        .unwrap();
    let mut changed = grant.clone();
    changed.bindings.context_capsule_digest = format!("sha256:{}", "b".repeat(64));
    assert!(store
        .record_recovery_admission_intent(&owner, &changed, |_, _| Ok(()))
        .is_err());
    changed = grant.clone();
    changed.authenticated_event_id = "other-consent".into();
    store
        .bind_gateway_event(
            &changed.authenticated_event_id,
            changed
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
        )
        .unwrap();
    assert!(store
        .record_recovery_admission_intent(&owner, &changed, |_, _| Ok(()))
        .is_err());
    db.execute("UPDATE recovery_admissions SET run_id='tampered'", [])
        .unwrap();
    assert!(store
        .with_recorded_recovery_admission(
            &owner,
            &grant.authenticated_event_id,
            |_, _| -> anyhow::Result<()> { panic!("tampered envelope entered callback") }
        )
        .is_err());
}

#[test]
fn targeted_claim_reuses_retained_identity_and_generation_after_lost_ack() {
    targeted_claim_fixture(false);
}

#[test]
fn recovery_projection_is_atomic_and_preserves_completed_execution() {
    targeted_claim_fixture(true);
}

fn targeted_claim_fixture(project_receipts: bool) {
    use arda_engine::objectives::{RetainedSnapshot, SnapshotAdmission};
    struct Keeper {
        fail_commit: std::sync::atomic::AtomicBool,
        allow_release: std::sync::atomic::AtomicBool,
        releases: std::sync::atomic::AtomicUsize,
    }
    impl SnapshotAdmission for Keeper {
        fn prepare(
            &self,
            _: &str,
            _: &std::path::Path,
            _: &str,
        ) -> anyhow::Result<RetainedSnapshot> {
            panic!("recovery must not prepare a snapshot")
        }
        fn commit(
            &self,
            _: &RetainedSnapshot,
            _: &str,
            _: i64,
            _: &str,
            _: i64,
        ) -> anyhow::Result<()> {
            if self
                .fail_commit
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                anyhow::bail!("lost acknowledgement");
            }
            Ok(())
        }
        fn release(&self, _: &RetainedSnapshot, _: &str) -> anyhow::Result<()> {
            use std::sync::atomic::Ordering::SeqCst;
            assert!(
                self.allow_release.load(SeqCst),
                "released unfinished snapshot"
            );
            if self.releases.fetch_add(1, SeqCst) == 0 {
                anyhow::bail!("fixture lost terminal release acknowledgement");
            }
            Ok(())
        }
    }
    let (temp, store, grant, owner) = fixture();
    let keeper = std::sync::Arc::new(Keeper {
        fail_commit: std::sync::atomic::AtomicBool::new(true),
        allow_release: std::sync::atomic::AtomicBool::new(false),
        releases: std::sync::atomic::AtomicUsize::new(0),
    });
    let store = store.with_snapshot_admission(keeper.clone());
    let db = rusqlite::Connection::open(temp.path().join("objective.sqlite3")).unwrap();
    // Storage-only fixture: these synthetic rows do not establish evidence or
    // policy acceptance. The production caller must independently validate both.
    let execution = serde_json::json!({"objective":"fixture", "execution_prompt":"inspect", "verification_prompt":"verify", "review_prompt":"review", "approval_envelope":{}, "objective_plan_receipt":"fixture"}).to_string();
    db.execute(
        "UPDATE leaves SET stage='execute',attempt=5,context_bound=1,execution_json=?1 WHERE id=?2",
        rusqlite::params![execution, grant.bindings.leaf_id],
    )
    .unwrap();
    db.execute(
        "INSERT INTO lease_workspace_identities VALUES (?1,'fixture-identity')",
        [&grant.bindings.leaf_id],
    )
    .unwrap();
    let snapshot = RetainedSnapshot {
        endpoint: "fixture".into(),
        capability: "fixture-only".into(),
        manifest_digest: grant.bindings.retained_authority_digest.clone(),
    };
    db.execute(
        "INSERT INTO retained_workspace_snapshots VALUES (?1,'run',?2,5)",
        rusqlite::params![
            grant.bindings.leaf_id,
            serde_json::to_string(&snapshot).unwrap()
        ],
    )
    .unwrap();
    db.execute(
        "INSERT INTO resident_context_bindings VALUES ('run','fixture','{}',NULL)",
        [],
    )
    .unwrap();
    store
        .bind_gateway_event(
            &grant.authenticated_event_id,
            grant
                .authenticated_payload_digest
                .strip_prefix("sha256:")
                .unwrap(),
        )
        .unwrap();
    store
        .record_recovery_admission_intent(&owner, &grant, |_, _| Ok(()))
        .unwrap();
    assert!(store
        .claim_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            "recovery-worker",
            3_600_000,
            |_, _| Ok(())
        )
        .is_err());
    let claim = store
        .claim_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            "recovery-worker",
            3_600_000,
            |_, _| Ok(()),
        )
        .unwrap();
    assert_eq!(claim.attempt, 6);
    assert_eq!(claim.stage, arda_engine::objectives::LeafStage::Execute);
    assert_eq!(
        claim.lease_expires_ms,
        i64::try_from(grant.expires_at_unix_ms).unwrap()
    );
    assert_eq!(claim.execution_run_id.as_deref(), Some("run"));
    let replay = store
        .claim_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            "recovery-worker",
            1_000,
            |_, _| Ok(()),
        )
        .unwrap();
    assert_eq!(replay.attempt, claim.attempt);
    assert_eq!(replay.lease_expires_ms, claim.lease_expires_ms);
    assert!(store
        .claim_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            "other-worker",
            1_000,
            |_, _| Ok(())
        )
        .is_err());
    let state: String = db
        .query_row(
            "SELECT state FROM objectives WHERE id='fenced'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "paused");
    let siblings: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM leaves WHERE id!=?1 AND (attempt!=0 OR lease_owner IS NOT NULL)",
            [&grant.bindings.leaf_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(siblings, 0);
    let saved: String = db
        .query_row(
            "SELECT capability_json FROM retained_workspace_snapshots",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(serde_json::from_str::<RetainedSnapshot>(&saved).unwrap() == snapshot);
    let lease = arda_engine::objectives::snapshot_protocol::Lease {
        run_id: "run".into(),
        generation: claim.attempt,
        owner: claim.lease_owner.clone(),
        expires_ms: claim.lease_expires_ms,
    };
    assert!(store
        .retained_execution("run", i64::try_from(grant.activated_at_unix_ms).unwrap())
        .is_err());
    store
        .with_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            &lease,
            |_, g, retained| {
                assert!(retained.snapshot == snapshot);
                assert!(retained.lease == lease);
                assert_eq!(g, &grant);
                Ok(())
            },
        )
        .unwrap();
    use arda_core::run_graph::NodeState;
    use arda_engine::runs::{AppendOutcome, RunEventDraft, RunEventKind, RunStore};
    let journal =
        RunStore::open(temp.path().join("journal"), grant.bindings.run_id.clone()).unwrap();
    for (index, state) in [
        NodeState::Running,
        NodeState::Failed,
        NodeState::Running,
        NodeState::Failed,
        NodeState::Ready,
    ]
    .into_iter()
    .enumerate()
    {
        journal
            .append(RunEventDraft {
                node_id: grant.bindings.verify_node_id.clone(),
                idempotency_key: format!("prior-{index}"),
                kind: RunEventKind::NodeTransition { state },
                receipt_digest: None,
            })
            .unwrap();
    }
    journal.activate_recovery(grant.clone()).unwrap();
    let start = RunEventDraft {
        node_id: grant.bindings.verify_node_id.clone(),
        idempotency_key: "verify:provider-running:3".into(),
        kind: RunEventKind::NodeTransition {
            state: NodeState::Running,
        },
        receipt_digest: None,
    };
    let before = journal.recover().unwrap().events.len();
    assert!(journal
        .append_recovery_start_before(&grant, start.clone(), 0)
        .is_err());
    assert!(store
        .append_retained_recovery_start(
            &owner,
            &grant.authenticated_event_id,
            &lease,
            &journal,
            start.clone(),
            |_, _, _| anyhow::bail!("current policy denied")
        )
        .is_err());
    assert_eq!(journal.recover().unwrap().events.len(), before);
    assert!(matches!(
        store
            .append_retained_recovery_start(
                &owner,
                &grant.authenticated_event_id,
                &lease,
                &journal,
                start.clone(),
                |_, _, _| Ok(())
            )
            .unwrap(),
        AppendOutcome::Appended { .. }
    ));
    assert!(matches!(
        store
            .append_retained_recovery_start(
                &owner,
                &grant.authenticated_event_id,
                &lease,
                &journal,
                start.clone(),
                |_, _, _| Ok(())
            )
            .unwrap(),
        AppendOutcome::AlreadyApplied { .. }
    ));
    assert_eq!(journal.recover().unwrap().events.len(), before + 1);
    for field in ["generation", "owner", "run", "expiry"] {
        let mut bad = lease.clone();
        match field {
            "generation" => bad.generation -= 1,
            "owner" => bad.owner = "other".into(),
            "run" => bad.run_id = "other".into(),
            _ => bad.expires_ms += 1,
        }
        assert!(store
            .with_retained_recovery(
                &owner,
                &grant.authenticated_event_id,
                &bad,
                |_, _, _| -> anyhow::Result<()> { panic!("mismatched lease authorized") }
            )
            .is_err());
    }
    db.execute(
        "UPDATE resident_context_bindings SET deleted_by_operator_ms=1",
        [],
    )
    .unwrap();
    assert!(store
        .with_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            &lease,
            |_, _, _| -> anyhow::Result<()> { panic!("deleted context authorized") }
        )
        .is_err());
    db.execute(
        "UPDATE resident_context_bindings SET deleted_by_operator_ms=NULL",
        [],
    )
    .unwrap();
    db.execute(
        "UPDATE retained_snapshot_lease_intents SET recovery_event_id=NULL",
        [],
    )
    .unwrap();
    assert!(store
        .with_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            &lease,
            |_, _, _| -> anyhow::Result<()> { panic!("ordinary intent authorized") }
        )
        .is_err());
    db.execute(
        "UPDATE retained_snapshot_lease_intents SET recovery_event_id=?1",
        [&grant.authenticated_event_id],
    )
    .unwrap();
    use arda_engine::objectives::{ReceiptStage, StageReceipt};
    let mut predecessor = None;
    let receipts: Vec<_> = [
        ReceiptStage::Execute,
        ReceiptStage::Verify,
        ReceiptStage::Review,
        ReceiptStage::Close,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, stage)| {
        let digest = if index == 0 {
            grant.bindings.execute_receipt_digest.clone()
        } else {
            format!("sha256:{index:064x}")
        };
        let receipt = StageReceipt {
            contract: "arda.hermes_execution_receipt.v4".into(),
            stage,
            digest: digest.clone(),
            predecessor_digest: predecessor.clone(),
            run_path: format!("data/runs/fixture/{index}.json"),
            provider: "synthetic-provider".into(),
            model: "synthetic-model".into(),
            started_at_ms: 1,
            completed_at_ms: 2,
            verdict: "fixture".into(),
            context_outcome_receipt_id: None,
            context_outcome_receipt_digest: None,
            binding_digest: None,
        };
        predecessor = Some(digest);
        receipt
    })
    .collect();
    if project_receipts {
        assert!(!store
            .reconcile_completed_recovery(
                &owner,
                &grant.authenticated_event_id,
                &receipts,
                |_, _, _| panic!("unfinished chain accepted")
            )
            .unwrap());
        let mut broken = receipts.clone();
        broken[1].predecessor_digest = None;
        assert!(store
            .record_retained_recovery_receipts(
                &owner,
                &grant.authenticated_event_id,
                &lease,
                &broken,
                |_, _, _, _| Ok(())
            )
            .is_err());
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM stage_receipts", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(store
            .record_retained_recovery_receipts(
                &owner,
                &grant.authenticated_event_id,
                &lease,
                &receipts,
                |_, _, _, _| anyhow::bail!("fixture current-policy rejection")
            )
            .is_err());
        let now = chrono::Utc::now().timestamp_millis();
        store
            .record_stage_receipt(
                &grant.bindings.leaf_id,
                &lease.owner,
                lease.generation,
                receipts[0].clone(),
                now,
            )
            .unwrap();
        store
            .record_retained_recovery_receipts(
                &owner,
                &grant.authenticated_event_id,
                &lease,
                &receipts,
                |_, _, _, _| Ok(()),
            )
            .unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM stage_receipts", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            4
        );
        let original_time: i64 = db
            .query_row(
                "SELECT recorded_at_ms FROM stage_receipts WHERE leaf_id=?1 AND stage='execute'",
                [&grant.bindings.leaf_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(original_time, now);
        let completed: (String, Option<String>, Option<i64>) = db
            .query_row(
                "SELECT stage,lease_owner,lease_expires_ms FROM leaves WHERE id=?1",
                [&grant.bindings.leaf_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(completed, ("complete".into(), None, None));
        let mut different = receipts.clone();
        different[1].model = "different-model".into();
        assert!(store
            .reconcile_completed_recovery(
                &owner,
                &grant.authenticated_event_id,
                &different,
                |_, _, _| panic!("different chain accepted")
            )
            .is_err());
        assert!(store
            .reconcile_completed_recovery(
                "wrong-owner",
                &grant.authenticated_event_id,
                &receipts,
                |_, _, _| panic!("wrong owner accepted")
            )
            .is_err());
        assert!(store
            .reconcile_completed_recovery(
                &owner,
                &grant.authenticated_event_id,
                &receipts,
                |_, _, _| anyhow::bail!("fixture canonical evidence rejected")
            )
            .is_err());
        keeper
            .allow_release
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(store
            .reconcile_completed_recovery(
                &owner,
                &grant.authenticated_event_id,
                &receipts,
                |_, _, _| Ok(())
            )
            .is_err());
        // Cleanup remains permitted after a later stop; it never revives work.
        store
            .apply_control(
                "fenced",
                ControlAction::Pause,
                "stop-after-completion",
                &owner,
                chrono::Utc::now().timestamp_millis(),
            )
            .unwrap();
        assert!(store
            .reconcile_completed_recovery(
                &owner,
                &grant.authenticated_event_id,
                &receipts,
                |_, _, _| Ok(())
            )
            .unwrap());
        assert!(store
            .reconcile_completed_recovery(
                &owner,
                &grant.authenticated_event_id,
                &receipts,
                |_, _, _| Ok(())
            )
            .unwrap());
        assert_eq!(keeper.releases.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(
            db.query_row(
                "SELECT state FROM objectives WHERE id='fenced'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "paused"
        );
        return;
    }
    store
        .apply_control(
            "fenced",
            ControlAction::Pause,
            "stop-recovery-after-claim",
            &owner,
            5,
        )
        .unwrap();
    assert!(store
        .record_retained_recovery_receipts(
            &owner,
            &grant.authenticated_event_id,
            &lease,
            &receipts,
            |_, _, _, _| panic!("superseded recovery projected receipts")
        )
        .is_err());
    assert!(store
        .with_retained_recovery(
            &owner,
            &grant.authenticated_event_id,
            &lease,
            |_, _, _| -> anyhow::Result<()> { panic!("superseded recovery authorized") }
        )
        .is_err());
}

#[test]
fn failed_fenced_operation_rolls_back_sqlite_changes() {
    let (temp, store, grant, owner) = fixture();
    let result: anyhow::Result<()> = store.with_recovery_control_fence(&owner, &grant, |tx| {
        tx.execute(
            "UPDATE objectives SET stop_generation=stop_generation+1 WHERE id='fenced'",
            [],
        )?;
        anyhow::bail!("injected operation failure")
    });
    assert!(result.is_err());
    let db = rusqlite::Connection::open(temp.path().join("objective.sqlite3")).unwrap();
    let generation: i64 = db
        .query_row(
            "SELECT stop_generation FROM objectives WHERE id='fenced'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(generation, 1);
    store
        .with_recovery_control_fence(&owner, &grant, |_| Ok(()))
        .unwrap();
}
