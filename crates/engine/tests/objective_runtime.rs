#[tokio::test]
async fn failed_leaf_does_not_discard_successful_sibling_receipts() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["project-a", "project-b", "join"] {
        std::fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    let store = ObjectiveStore::open(dir.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve-runtime-1",
            "operator-1",
            101,
        )
        .unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store,
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: Some("inspect-a"),
        },
        "arda-runtime-failure",
        4,
        60_000,
    );

    let error = runtime.run_round(200).await.unwrap_err();

    assert!(format!("{error:#}").contains("deliberate failure for inspect-a"));
    assert_eq!(
        runtime.store().leaf("inspect-b").unwrap().unwrap().stage,
        LeafStage::Complete
    );
    assert!(runtime
        .store()
        .leaf("inspect-b")
        .unwrap()
        .unwrap()
        .current_receipt_digest
        .is_some());
}
use anyhow::Result;
use arda_engine::objectives::LeafStage;
use arda_engine::objectives::{
    ControlAction, LeafExecution, LeafExecutionResult, LeafExecutionSpec, NewLeaf, NewObjective,
    ObjectiveRuntime, ObjectiveState, ObjectiveStore, ProjectAuthority, ReceiptStage, StageReceipt,
};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn revision_cannot_leave_persisted_worker_prompts_bound_to_an_old_goal() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective(root.path());
    store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    let before = serde_json::to_value(store.objective(&input.id).unwrap()).unwrap();
    let error = store
        .apply_control(
            &input.id,
            ControlAction::Revise {
                text: "A different goal requiring different evidence".into(),
            },
            "revision-with-stale-execution",
            &input.operator_id,
            110,
        )
        .expect_err("revision must not silently retain old execution prompts");
    assert!(error.to_string().contains("persisted execution plan"));
    drop(store);
    let reopened = ObjectiveStore::open(&path).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.objective(&input.id).unwrap()).unwrap(),
        before
    );
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM controls WHERE idempotency_key = 'revision-with-stale-execution'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn quarantine_is_visible_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["project-a", "project-b", "join"] {
        std::fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    let path = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .put_schedule(
            arda_engine::objectives::ScheduleSpec {
                id: "quarantined".into(),
                objective_id: "objective-runtime-1".into(),
                next_wake_ms: 101,
                recurrence: Some("PT1S".into()),
                idempotency_key: "quarantined".into(),
            },
            100,
        )
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve-quarantine",
            "operator-1",
            101,
        )
        .unwrap();
    rusqlite::Connection::open(&path).unwrap().execute(
        "UPDATE schedules SET recurrence = 'invalid historical format' WHERE id = 'quarantined'", [],
    ).unwrap();
    drop(store);
    for _ in 0..2 {
        let mut runtime = ObjectiveRuntime::new(
            ObjectiveStore::open_existing(&path).unwrap(),
            RecordingExecutor {
                active: Arc::new(AtomicUsize::new(0)),
                maximum: Arc::new(AtomicUsize::new(0)),
                fail_leaf: None,
            },
            "quarantine-status",
            4,
            1000,
        );
        let status = runtime.subscribe_status();
        assert_eq!(status.borrow().quarantined_schedules, None);
        runtime.run_round(200).await.unwrap();
        assert_eq!(status.borrow().quarantined_schedules, Some(1));
        assert_eq!(status.borrow().last_error, Some("schedule_quarantined"));
        assert!(!status.borrow().ready);
        assert!(status.borrow().next_wake_ms.is_none());
    }
}

#[tokio::test]
async fn runtime_status_tracks_idle_and_retained_stop() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = ObjectiveRuntime::new(
        ObjectiveStore::open(dir.path().join("objectives.sqlite3")).unwrap(),
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        },
        "status",
        4,
        1000,
    );
    let status = runtime.subscribe_status();
    assert_eq!(status.borrow().phase, "not_started");
    assert_eq!(
        serde_json::to_value(status.borrow().clone()).unwrap()["ready"],
        false
    );
    runtime.run_round(100).await.unwrap();
    assert_eq!(status.borrow().phase, "waiting");
    assert_eq!(
        serde_json::to_value(status.borrow().clone()).unwrap()["ready"],
        true
    );
    let shutdown = arda_engine::supervisor::Shutdown::new();
    shutdown.trigger();
    runtime
        .run_until_shutdown(
            shutdown,
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        )
        .await;
    assert_eq!(status.borrow().phase, "stopped");
    assert_eq!(
        serde_json::to_value(status.borrow().clone()).unwrap()["ready"],
        false
    );
    assert!(status.borrow().active_leaves.is_empty());
}

#[test]
fn ingress_key_cannot_change_command_kind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective(dir.path());
    let key = input.idempotency_key.clone();
    let created = store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    let error = store
        .apply_control(
            &input.id,
            ControlAction::Approve { revision: 1 },
            &key,
            &input.operator_id,
            101,
        )
        .expect_err("creation event must not authorize a control");
    assert!(error.to_string().contains("idempotency conflict"));
    assert_eq!(
        store.objective(&input.id).unwrap().unwrap().state,
        created.state
    );
    store
        .apply_control(
            &input.id,
            ControlAction::Approve { revision: 1 },
            "control-only",
            &input.operator_id,
            102,
        )
        .unwrap();
    drop(store);
    let store = ObjectiveStore::open(&path).unwrap();
    let mut other = input.clone();
    other.id = "other-objective".into();
    other.idempotency_key = "control-only".into();
    for leaf in &mut other.leaves {
        leaf.id = format!("other-{}", leaf.id);
        for dependency in &mut leaf.dependencies {
            *dependency = format!("other-{dependency}");
        }
    }
    let error = store
        .create_authenticated_objective(other.clone(), 103)
        .expect_err("control event must not create an objective after restart");
    assert!(error.to_string().contains("idempotency conflict"));
    assert!(store.objective(&other.id).unwrap().is_none());
    store
        .create_authenticated_objective(input.clone(), 104)
        .unwrap();
    store
        .apply_control(
            &input.id,
            ControlAction::Approve { revision: 1 },
            "control-only",
            &input.operator_id,
            105,
        )
        .unwrap();
}

struct BlockOneExecutor(RecordingExecutor);

struct BreakTimerAfterExecution(std::path::PathBuf);

impl LeafExecution for BreakTimerAfterExecution {
    fn execute(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>> {
        let path = self.0.clone();
        Box::pin(async move {
            let result = RecordingExecutor {
                active: Arc::new(AtomicUsize::new(0)),
                maximum: Arc::new(AtomicUsize::new(0)),
                fail_leaf: None,
            }
            .execute(claim)
            .await?;
            rusqlite::Connection::open(path)?.execute_batch("DROP TABLE schedule_errors")?;
            Ok(result)
        })
    }
}

#[tokio::test]
async fn readiness_requires_timer_check_after_successful_execution() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective(dir.path());
    input.leaves.retain(|leaf| leaf.id == "inspect-a");
    store.create_authenticated_objective(input, 100).unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator-1",
            101,
        )
        .unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store.clone(),
        BreakTimerAfterExecution(path),
        "timer-check",
        1,
        1000,
    );
    let status = runtime.subscribe_status();
    let result = runtime.run_round(200).await;
    assert_eq!(
        store.leaf("inspect-a").unwrap().unwrap().stage,
        LeafStage::Complete
    );
    assert!(
        result.is_err(),
        "timer failure must fail the readiness check"
    );
    assert!(!status.borrow().ready);
    assert_eq!(status.borrow().last_error, Some("timer_lookup_failed"));
}

#[tokio::test]
async fn scheduler_readiness_fails_closed_on_store_loss_and_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let mut runtime = ObjectiveRuntime::new(
        ObjectiveStore::open(&path).unwrap(),
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        },
        "readiness",
        1,
        1000,
    );
    let status = runtime.subscribe_status();
    runtime.run_round(100).await.unwrap();
    assert!(status.borrow().ready);
    // Test-only schema outage: no live store or provider involved.
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("ALTER TABLE leaves RENAME TO unavailable_leaves")
        .unwrap();
    assert!(runtime.run_round(200).await.is_err());
    assert!(!status.borrow().ready);
    assert_eq!(status.borrow().pending_recovery, None);
    assert!(status.borrow().last_error.is_some());
    connection
        .execute_batch("ALTER TABLE unavailable_leaves RENAME TO leaves")
        .unwrap();
    runtime.run_round(300).await.unwrap();
    assert!(status.borrow().ready);
    assert_eq!(status.borrow().pending_recovery, Some(0));
    assert!(status.borrow().last_error.is_none());
}

#[tokio::test]
async fn missing_database_does_not_get_recreated_by_resident_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let saved = dir.path().join("saved.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store.clone(),
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        },
        "missing-file",
        1,
        1000,
    );
    let status = runtime.subscribe_status();
    runtime.run_round(101).await.unwrap();
    std::fs::rename(&path, &saved).unwrap();
    assert!(runtime.run_round(102).await.is_err());
    assert_eq!(status.borrow().quarantined_schedules, None);
    assert!(!status.borrow().ready);
    assert!(store.list_objectives().is_err());
    assert!(
        !path.exists(),
        "routine access must not recreate lost authority"
    );
    assert!(!path.with_extension("sqlite3-wal").exists());
    assert!(!path.with_extension("sqlite3-shm").exists());
    std::fs::rename(&saved, &path).unwrap();
    runtime.run_round(103).await.unwrap();
    assert!(status.borrow().ready);
    assert_eq!(store.list_objectives().unwrap().len(), 1);
}

#[tokio::test]
async fn resident_wakes_for_separately_opened_store_and_dependent_completion() {
    assert_resident_wakeup(false).await;
}

#[tokio::test]
async fn resident_wakes_at_due_time_before_polling_fallback() {
    assert_resident_wakeup(true).await;
}

async fn assert_resident_wakeup(deferred: bool) {
    use arda_engine::supervisor::Shutdown;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let writer = ObjectiveStore::open(&path).unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store,
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        },
        "notification-worker",
        4,
        60_000,
    );
    let stop = Shutdown::new();
    let signal = stop.clone();
    let task = tokio::spawn(async move {
        runtime
            .run_until_shutdown(signal, Duration::from_secs(30), Duration::from_secs(1))
            .await
    });
    // Let the empty initial round enter its long fallback wait.
    tokio::time::sleep(Duration::from_millis(50)).await;
    writer
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    if deferred {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        writer
            .put_schedule(
                arda_engine::objectives::ScheduleSpec {
                    id: "deferred-wake".into(),
                    objective_id: "objective-runtime-1".into(),
                    next_wake_ms: now + 200,
                    recurrence: None,
                    idempotency_key: "deferred-wake".into(),
                },
                now,
            )
            .unwrap();
    }
    writer
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "notification-approve",
            "operator-1",
            101,
        )
        .unwrap();
    let completed = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if writer
                .objective("objective-runtime-1")
                .unwrap()
                .unwrap()
                .state
                == ObjectiveState::Completed
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    stop.trigger();
    task.await.unwrap();
    assert!(
        completed.is_ok(),
        "accepted mutations/dependent work waited for the polling fallback"
    );
}

#[tokio::test]
async fn resident_shutdown_drains_started_work_but_never_admits_after_pre_stop() {
    use arda_engine::objectives::ObjectiveRuntimeStop;
    use arda_engine::supervisor::Shutdown;
    use std::time::Duration;
    for pre_stopped in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        store
            .create_authenticated_objective(objective(dir.path()), 100)
            .unwrap();
        store
            .apply_control(
                "objective-runtime-1",
                ControlAction::Approve { revision: 1 },
                "approve",
                "operator-1",
                101,
            )
            .unwrap();
        let active = Arc::new(AtomicUsize::new(0));
        let mut runtime = ObjectiveRuntime::new(
            store,
            RecordingExecutor {
                active: active.clone(),
                maximum: Arc::new(AtomicUsize::new(0)),
                fail_leaf: None,
            },
            "resident-graceful",
            4,
            60_000,
        );
        let shutdown = Shutdown::new();
        if pre_stopped {
            shutdown.trigger();
        }
        let signal = shutdown.clone();
        let task = tokio::spawn(async move {
            runtime
                .run_until_shutdown(signal, Duration::from_secs(30), Duration::from_secs(1))
                .await
        });
        if !pre_stopped {
            tokio::time::timeout(Duration::from_secs(2), async {
                while active.load(Ordering::SeqCst) == 0 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            shutdown.trigger();
        }
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), task)
                .await
                .unwrap()
                .unwrap(),
            ObjectiveRuntimeStop::Drained
        );
        let observed = ObjectiveStore::open(&path).unwrap();
        for leaf in ["inspect-a", "inspect-b"] {
            let record = observed.leaf(leaf).unwrap().unwrap();
            if pre_stopped {
                assert_eq!(record.attempt, 0);
            } else {
                assert_eq!(record.stage, LeafStage::Complete);
            }
        }
        assert_eq!(observed.leaf("join").unwrap().unwrap().attempt, 0);
    }
}

#[tokio::test]
async fn resident_shutdown_bounds_pending_round_and_preserves_finished_sibling() {
    use arda_engine::objectives::ObjectiveRuntimeStop;
    use arda_engine::supervisor::Shutdown;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator-1",
            101,
        )
        .unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store,
        BlockOneExecutor(RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        }),
        "resident-drain",
        4,
        60_000,
    );
    let status = runtime.subscribe_status();
    let shutdown = Shutdown::new();
    let signal = shutdown.clone();
    let mut task = tokio::spawn(async move {
        runtime
            .run_until_shutdown(signal, Duration::from_secs(30), Duration::from_millis(50))
            .await
    });
    let observed = ObjectiveStore::open(&path).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while observed.leaf("inspect-b").unwrap().unwrap().stage != LeafStage::Complete {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let sibling = observed.leaf("inspect-b").unwrap().unwrap();
    let pending = observed.leaf("inspect-a").unwrap().unwrap();
    assert_eq!(status.borrow().phase, "executing");
    assert_eq!(status.borrow().active_leaves, vec!["inspect-a"]);
    shutdown.trigger();
    let stopped = tokio::time::timeout(Duration::from_millis(500), &mut task).await;
    if stopped.is_err() {
        task.abort();
        let _ = task.await;
    }
    assert_eq!(
        stopped
            .expect("resident drain exceeded its deadline")
            .unwrap(),
        ObjectiveRuntimeStop::Interrupted
    );
    assert_eq!(observed.leaf("inspect-b").unwrap().unwrap(), sibling);
    assert_eq!(observed.leaf("inspect-a").unwrap().unwrap(), pending);
    assert_eq!(status.borrow().phase, "interrupted");
    assert!(status.borrow().active_leaves.is_empty());
    assert_eq!(
        observed.leaf("join").unwrap().unwrap().attempt,
        0,
        "shutdown must not claim another round"
    );
    drop(observed);
    let reopened = ObjectiveStore::open(&path).unwrap();
    let reclaimed = reopened
        .claim_runnable("restart", pending.lease_expires_ms.unwrap() + 1, 60_000, 4)
        .unwrap();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].leaf_id, "inspect-a");
}

struct DelayedReconciliation {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    success: bool,
}

struct RetryProbe {
    probes: Arc<AtomicUsize>,
    executions: Arc<AtomicUsize>,
    unavailable: bool,
}

impl LeafExecution for RetryProbe {
    fn reconcile(
        &self,
        _: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<Option<LeafExecutionResult>>> + Send>> {
        panic!("budgeted retries must use retry inspection, not exhausted reconciliation")
    }

    fn inspect_retry(
        &self,
        _: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<Option<LeafExecutionResult>>> + Send>> {
        self.probes.fetch_add(1, Ordering::SeqCst);
        let unavailable = self.unavailable;
        Box::pin(async move {
            if unavailable {
                anyhow::bail!("receipt service unavailable");
            }
            Ok(None)
        })
    }

    fn execute(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        }
        .execute(claim)
    }
}

#[tokio::test]
async fn bound_retry_continues_only_after_successful_receipt_lookup() {
    for unavailable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        store
            .create_authenticated_objective(objective(dir.path()), 100)
            .unwrap();
        store
            .apply_control(
                "objective-runtime-1",
                ControlAction::Approve { revision: 1 },
                "approve",
                "operator-1",
                101,
            )
            .unwrap();
        let original = store
            .claim_runnable("previous", 200, 100, 1)
            .unwrap()
            .remove(0);
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE leaves SET context_bound = 1 WHERE id = ?1",
                [&original.leaf_id],
            )
            .unwrap();
        let probes = Arc::new(AtomicUsize::new(0));
        let executions = Arc::new(AtomicUsize::new(0));
        let mut runtime = ObjectiveRuntime::new(
            store.clone(),
            RetryProbe {
                probes: probes.clone(),
                executions: executions.clone(),
                unavailable,
            },
            "resident",
            4,
            1000,
        );
        let status = runtime.subscribe_status();
        // Reopening and getting an empty batch must not hide an unexpired
        // interrupted lease, whether receipt-bound or not.
        runtime.run_round(250).await.unwrap();
        let waiting = serde_json::to_value(status.borrow().clone()).unwrap();
        assert_eq!(waiting["ready"], false);
        assert_eq!(waiting["pending_recovery"], 1);
        let result = runtime.run_round(301).await;
        assert_eq!(
            status.borrow().phase,
            if unavailable { "degraded" } else { "waiting" }
        );
        assert_eq!(status.borrow().last_error.is_some(), unavailable);
        let snapshot = serde_json::to_value(status.borrow().clone()).unwrap();
        assert_eq!(snapshot["ready"], !unavailable);
        assert_eq!(snapshot["pending_recovery"], usize::from(unavailable));
        assert_eq!(probes.load(Ordering::SeqCst), 1);
        assert_eq!(executions.load(Ordering::SeqCst), usize::from(!unavailable));
        assert_eq!(result.is_err(), unavailable);
        assert_eq!(store.leaf("inspect-b").unwrap().unwrap().attempt, 0);
        // Inspect the stable run through a later reclaim when lookup failed.
        if unavailable {
            // An empty successful round during the retry lease cannot turn
            // a failed receipt probe into a ready scheduler.
            runtime.run_round(400).await.unwrap();
            assert_eq!(
                serde_json::to_value(status.borrow().clone()).unwrap()["ready"],
                false
            );
            let next = store
                .claim_runnable("retry", 1400, 1000, 1)
                .unwrap()
                .remove(0);
            assert_eq!(next.execution_run_id, original.execution_run_id);
        }
    }
}

impl LeafExecution for DelayedReconciliation {
    fn inspect_retry(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<Option<LeafExecutionResult>>> + Send>> {
        // This fixture's normal retry is attempt two; its exhausted cases
        // must stay on receipt-only reconciliation.
        assert_eq!(claim.attempt, 2);
        self.reconcile(claim)
    }

    fn execute(
        &self,
        _: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>> {
        panic!("receipt-only recovery must not dispatch")
    }

    fn reconcile(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<Option<LeafExecutionResult>>> + Send>> {
        let started = self.started.clone();
        let release = self.release.clone();
        let success = self.success;
        Box::pin(async move {
            started.notify_one();
            release.notified().await;
            if success {
                RecordingExecutor {
                    active: Arc::new(AtomicUsize::new(0)),
                    maximum: Arc::new(AtomicUsize::new(0)),
                    fail_leaf: None,
                }
                .execute(claim)
                .await
                .map(Some)
            } else {
                Ok(None)
            }
        })
    }
}

#[tokio::test]
async fn receipt_reconciliation_finishes_before_fresh_admission() {
    for attempt in [1, 5] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        store
            .create_authenticated_objective(objective(dir.path()), 100)
            .unwrap();
        store
            .apply_control(
                "objective-runtime-1",
                ControlAction::Approve { revision: 1 },
                "approve",
                "operator-1",
                101,
            )
            .unwrap();
        let claimed = store.claim_runnable("previous", 200, 100, 1).unwrap();
        assert_eq!(claimed[0].leaf_id, "inspect-a");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE leaves SET attempt = ?1, context_bound = 1 WHERE id = 'inspect-a'",
                [attempt],
            )
            .unwrap();
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let mut runtime = ObjectiveRuntime::new(
            store.clone(),
            DelayedReconciliation {
                started: started.clone(),
                release: release.clone(),
                success: true,
            },
            "resident",
            4,
            1000,
        );
        let task = tokio::spawn(async move { runtime.run_round(301).await });
        let observed =
            tokio::time::timeout(std::time::Duration::from_secs(2), started.notified()).await;
        release.notify_one();
        let result = task.await;
        assert!(
            observed.is_ok(),
            "fresh dispatch prevented receipt-only reconciliation"
        );
        assert_eq!(result.unwrap().unwrap().len(), 1);
        assert_eq!(store.leaf("inspect-b").unwrap().unwrap().attempt, 0);
        assert_eq!(
            store.leaf("inspect-a").unwrap().unwrap().stage,
            LeafStage::Complete
        );
    }
}

#[tokio::test]
async fn unexpired_receipt_recovery_lease_blocks_fresh_admission() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator-1",
            101,
        )
        .unwrap();
    store.claim_runnable("previous", 200, 100, 1).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE leaves SET attempt = 5, context_bound = 1 WHERE id = 'inspect-a'",
            [],
        )
        .unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store.clone(),
        RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        },
        "resident",
        4,
        1000,
    );
    let status = runtime.subscribe_status();
    assert!(runtime.run_round(250).await.unwrap().is_empty());
    assert!(!status.borrow().ready);
    assert_eq!(status.borrow().pending_recovery, Some(1));
    assert_eq!(store.leaf("inspect-b").unwrap().unwrap().attempt, 0);
    assert_eq!(
        store
            .leaf("inspect-a")
            .unwrap()
            .unwrap()
            .lease_owner
            .as_deref(),
        Some("previous")
    );
    // Reopen the database as a new resident, retaining the old live lease.
    let reopened = ObjectiveStore::open(&path).unwrap();
    assert!(reopened
        .claim_runnable("restart", 299, 1000, 4)
        .unwrap()
        .is_empty());
    let release = Arc::new(tokio::sync::Notify::new());
    release.notify_one();
    let mut recovery = ObjectiveRuntime::new(
        reopened.clone(),
        DelayedReconciliation {
            started: Arc::new(tokio::sync::Notify::new()),
            release,
            success: true,
        },
        "restart",
        4,
        1000,
    );
    let restarted_status = recovery.subscribe_status();
    assert!(!restarted_status.borrow().ready);
    assert!(recovery.run_round(299).await.unwrap().is_empty());
    assert!(!restarted_status.borrow().ready);
    assert_eq!(restarted_status.borrow().pending_recovery, Some(1));
    assert_eq!(recovery.run_round(300).await.unwrap().len(), 1);
    assert!(restarted_status.borrow().ready);
    assert_eq!(restarted_status.borrow().pending_recovery, Some(0));
    assert_eq!(
        reopened.leaf("inspect-a").unwrap().unwrap().stage,
        LeafStage::Complete
    );
    let claims = reopened.claim_runnable("restart", 400, 1000, 4).unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].leaf_id, "inspect-b");
}

#[tokio::test]
async fn stale_same_owner_reconciliation_cannot_write_success_or_failure() {
    for (success, reclaim) in [(false, true), (true, true), (false, false), (true, false)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        let mut input = objective(dir.path());
        input.leaves.retain(|leaf| leaf.id == "inspect-a");
        store.create_authenticated_objective(input, 100).unwrap();
        store
            .apply_control(
                "objective-runtime-1",
                ControlAction::Approve { revision: 1 },
                "approve",
                "operator-1",
                101,
            )
            .unwrap();
        store.claim_runnable("same-owner", 200, 100, 1).unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute("UPDATE leaves SET attempt = 5, context_bound = 1", [])
            .unwrap();
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let mut first = ObjectiveRuntime::new(
            store,
            DelayedReconciliation {
                started: started.clone(),
                release: release.clone(),
                success,
            },
            "same-owner",
            1,
            100,
        );
        let first_task = tokio::spawn(async move { first.run_round(301).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
            .await
            .unwrap();
        let next_task = if reclaim {
            let next_started = Arc::new(tokio::sync::Notify::new());
            let mut next = ObjectiveRuntime::new(
                ObjectiveStore::open(&path).unwrap(),
                DelayedReconciliation {
                    started: next_started.clone(),
                    release: Arc::new(tokio::sync::Notify::new()),
                    success: true,
                },
                "same-owner",
                1,
                60_000,
            );
            let next_task = tokio::spawn(async move { next.run_round(402).await });
            tokio::time::timeout(std::time::Duration::from_secs(2), next_started.notified())
                .await
                .unwrap();
            Some(next_task)
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            None
        };
        release.notify_one();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), first_task)
            .await
            .unwrap()
            .unwrap();
        if let Some(next_task) = next_task {
            next_task.abort();
            assert!(next_task.await.unwrap_err().is_cancelled());
        }
        let observed = ObjectiveStore::open(&path).unwrap();
        assert_eq!(
            observed
                .objective("objective-runtime-1")
                .unwrap()
                .unwrap()
                .state,
            ObjectiveState::Running,
            "stale failure must not finalize a reclaimed objective"
        );
        assert_eq!(
            observed.leaf("inspect-a").unwrap().unwrap().stage,
            LeafStage::Execute,
            "stale success must not advance the new generation"
        );
    }
}

impl LeafExecution for BlockOneExecutor {
    fn execute(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>> {
        if claim.leaf_id == "inspect-a" {
            Box::pin(std::future::pending())
        } else {
            self.0.execute(claim)
        }
    }
}

#[tokio::test]
async fn finished_sibling_is_durable_before_a_pending_round_is_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["project-a", "project-b", "join"] {
        std::fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    let database = dir.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&database).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve-runtime-1",
            "operator-1",
            101,
        )
        .unwrap();
    let mut runtime = ObjectiveRuntime::new(
        store,
        BlockOneExecutor(RecordingExecutor {
            active: Arc::new(AtomicUsize::new(0)),
            maximum: Arc::new(AtomicUsize::new(0)),
            fail_leaf: None,
        }),
        "interrupted-runtime",
        4,
        60_000,
    );
    let task = tokio::spawn(async move { runtime.run_round(200).await });
    let observer = ObjectiveStore::open(&database).unwrap();
    let persisted = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if observer.leaf("inspect-b").unwrap().unwrap().stage == LeafStage::Complete {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok();
    assert!(
        !task.is_finished(),
        "the other leaf should still be pending"
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(
        persisted,
        "a finished sibling must persist before the whole round completes"
    );
    drop(observer);
    let reopened = ObjectiveStore::open(&database).unwrap();
    assert_eq!(
        reopened.leaf("inspect-b").unwrap().unwrap().stage,
        LeafStage::Complete
    );
    let recovered = reopened
        .claim_runnable("recovered-runtime", 60_201, 60_000, 4)
        .unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(
        recovered[0].leaf_id, "inspect-a",
        "completed work must not be reclaimed"
    );
}

#[derive(Clone)]
struct RecordingExecutor {
    active: Arc<AtomicUsize>,
    maximum: Arc<AtomicUsize>,
    fail_leaf: Option<&'static str>,
}

impl LeafExecution for RecordingExecutor {
    fn execute(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>> {
        let active = Arc::clone(&self.active);
        let maximum = Arc::clone(&self.maximum);
        let fail_leaf = self.fail_leaf;
        Box::pin(async move {
            if fail_leaf == Some(claim.leaf_id.as_str()) {
                anyhow::bail!("deliberate failure for {}", claim.leaf_id);
            }
            assert!(claim.execution.is_some());
            assert!(claim.project_contract_digest.is_some());
            if claim.leaf_id == "join" {
                assert_eq!(claim.dependency_receipts.len(), 2);
                assert!(claim.dependency_receipts.iter().all(|receipt| {
                    receipt.contract == "arda.hermes_execution_receipt.v4"
                        && receipt.stage == ReceiptStage::Close
                        && receipt.verdict == "succeeded"
                }));
            } else {
                assert!(claim.dependency_receipts.is_empty());
            }
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(current, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            active.fetch_sub(1, Ordering::SeqCst);

            let mut predecessor = claim.current_receipt_digest.clone();
            let mut receipts = Vec::new();
            for (index, stage) in [
                ReceiptStage::Execute,
                ReceiptStage::Verify,
                ReceiptStage::Review,
                ReceiptStage::Close,
            ]
            .into_iter()
            .enumerate()
            {
                let seed = format!("{}-{index}", claim.leaf_id);
                let digest = format!("sha256:{:x}", Sha256::digest(seed.as_bytes()));
                receipts.push(StageReceipt {
                    contract: "arda.hermes_execution_receipt.v4".into(),
                    stage,
                    digest: digest.clone(),
                    predecessor_digest: predecessor,
                    run_path: format!(
                        "data/runs/{}/execution-receipts/{}.json",
                        claim.leaf_id, index
                    ),
                    provider: "test-provider".into(),
                    model: "test-model".into(),
                    started_at_ms: 200 + index as i64,
                    completed_at_ms: 201 + index as i64,
                    verdict: "succeeded".into(),
                    context_outcome_receipt_id: None,
                    context_outcome_receipt_digest: None,
                    binding_digest: None,
                });
                predecessor = Some(digest);
            }
            Ok(LeafExecutionResult { receipts })
        })
    }
}

fn execution_spec(leaf_id: &str) -> LeafExecutionSpec {
    LeafExecutionSpec {
        objective: format!("Execute {leaf_id}"),
        execution_prompt: format!("Execute only {leaf_id}"),
        verification_prompt: format!("Verify {leaf_id}"),
        review_prompt: format!("Review {leaf_id}"),
        approval_envelope: serde_json::json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-runtime-1",
                "approval_id": "operator-approval-runtime-1",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": format!("objective-runtime-1-{leaf_id}")
        }),
        objective_plan_receipt: format!("sha256:{}", "1".repeat(64)),
    }
}

fn objective(root: &std::path::Path) -> NewObjective {
    let project_a = "b22c0000-e29b-41d4-a716-446655440002";
    let project_b = "c33d0000-e29b-41d4-a716-446655440003";
    NewObjective {
        id: "objective-runtime-1".into(),
        source_id: "operator-objective-runtime-1".into(),
        idempotency_key: "ingress-runtime-1".into(),
        operator_id: "operator-1".into(),
        text: "Inspect two projects and join the evidence".into(),
        priority: 100,
        projects: vec![
            ProjectAuthority {
                project_id: project_a.into(),
                contract_digest: format!("sha256:{}", "a".repeat(64)),
            },
            ProjectAuthority {
                project_id: project_b.into(),
                contract_digest: format!("sha256:{}", "b".repeat(64)),
            },
        ],
        leaves: vec![
            NewLeaf {
                id: "inspect-a".into(),
                project_id: Some(project_a.into()),
                workspace_root: root.join("project-a").display().to_string(),
                authority: "read_only".into(),
                dependencies: vec![],
                execution: Some(execution_spec("inspect-a")),
            },
            NewLeaf {
                id: "inspect-b".into(),
                project_id: Some(project_b.into()),
                workspace_root: root.join("project-b").display().to_string(),
                authority: "read_only".into(),
                dependencies: vec![],
                execution: Some(execution_spec("inspect-b")),
            },
            NewLeaf {
                id: "join".into(),
                project_id: Some(project_a.into()),
                workspace_root: root.join("join").display().to_string(),
                authority: "read_only".into(),
                dependencies: vec!["inspect-a".into(), "inspect-b".into()],
                execution: Some(execution_spec("join")),
            },
        ],
    }
}

#[tokio::test]
async fn resident_runtime_accepts_work_after_idle_polling() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["project-a", "project-b", "join"] {
        std::fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    let store = ObjectiveStore::open(dir.path().join("objectives.sqlite3")).unwrap();
    let executor = RecordingExecutor {
        active: Arc::new(AtomicUsize::new(0)),
        maximum: Arc::new(AtomicUsize::new(0)),
        fail_leaf: None,
    };
    let mut runtime = ObjectiveRuntime::new(store, executor, "resident", 4, 60_000);
    for now in 0..10 {
        assert!(runtime.run_round(now).await.unwrap().is_empty());
    }
    runtime
        .store()
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    runtime
        .store()
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve-after-idle",
            "operator-1",
            101,
        )
        .unwrap();
    assert_eq!(runtime.run_round(200).await.unwrap().len(), 2);
    assert_eq!(runtime.run_round(300).await.unwrap().len(), 1);
    assert_eq!(
        runtime
            .store()
            .objective("objective-runtime-1")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
}

#[tokio::test]
async fn resident_runtime_joins_independent_leaves_and_rehydrates_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["project-a", "project-b", "join"] {
        std::fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    let database = dir.path().join("data/arda/objectives.sqlite3");
    let store = ObjectiveStore::open(&database).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve-runtime-1",
            "operator-1",
            101,
        )
        .unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let executor = RecordingExecutor {
        active,
        maximum: Arc::clone(&maximum),
        fail_leaf: None,
    };

    let mut runtime = ObjectiveRuntime::new(store, executor.clone(), "arda-runtime-1", 4, 60_000);
    let first = runtime.run_round(200).await.unwrap();

    assert_eq!(first.len(), 2);
    assert_eq!(maximum.load(Ordering::SeqCst), 2);
    assert_eq!(
        runtime
            .store()
            .objective("objective-runtime-1")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Running
    );
    drop(runtime);

    let mut restarted = ObjectiveRuntime::new(
        ObjectiveStore::open(&database).unwrap(),
        executor,
        "arda-runtime-2",
        4,
        60_000,
    );
    let second = restarted.run_round(300).await.unwrap();

    assert_eq!(second.len(), 1);
    assert_eq!(second[0].leaf_id, "join");
    assert_eq!(
        restarted
            .store()
            .objective("objective-runtime-1")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
    assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!dir
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
}
