use arda_engine::objectives::{
    ControlAction, LeafStage, NewLeaf, NewObjective, ObjectiveState, ObjectiveStore,
    ProjectAuthority, ReceiptStage, ScheduleSpec, StageReceipt,
};
mod admission_recovery;
#[path = "objective_store/recovery_fence.rs"]
mod recovery_fence;

#[test]
fn repeated_stops_are_monotonic_but_exact_replays_and_resume_are_not_new_stops() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("stop-fence", "stop-ingress");
    let owner = input.operator_id.clone();
    store.create_authenticated_objective(input, 100).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let generation = || {
        db.query_row(
            "SELECT stop_generation FROM objectives WHERE id='stop-fence'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
    };
    assert_eq!(generation(), 0);
    for (key, action, expected) in [
        ("pause-1", ControlAction::Pause, 1),
        ("pause-1", ControlAction::Pause, 1),
        ("pause-2", ControlAction::Pause, 2),
        ("resume", ControlAction::Resume, 2),
        ("pause-3", ControlAction::Pause, 3),
        ("cancel", ControlAction::Cancel, 4),
        ("cancel", ControlAction::Cancel, 4),
    ] {
        store
            .apply_control("stop-fence", action, key, &owner, 101)
            .unwrap();
        assert_eq!(generation(), expected);
        let recorded: i64 = db
            .query_row(
                "SELECT stop_generation FROM controls WHERE idempotency_key=?1",
                [key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(recorded, expected);
    }
    assert!(store
        .apply_control("stop-fence", ControlAction::Pause, "rejected", &owner, 101)
        .is_err());
    assert_eq!(generation(), 4);
    drop(store);
    ObjectiveStore::open(&path).unwrap();
    assert_eq!(generation(), 4);
}

#[test]
fn stop_fence_migration_preserves_legacy_controls_without_inventing_sequence() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("legacy-stop", "legacy-stop-ingress");
    let owner = input.operator_id.clone();
    store.create_authenticated_objective(input, 100).unwrap();
    store
        .apply_control(
            "legacy-stop",
            ControlAction::Pause,
            "legacy-pause",
            &owner,
            101,
        )
        .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let before: String = db
        .query_row("SELECT action_json FROM controls", [], |row| row.get(0))
        .unwrap();
    db.execute_batch("ALTER TABLE controls DROP COLUMN stop_generation; ALTER TABLE objectives DROP COLUMN stop_generation;").unwrap();
    drop(store);
    let reopened = ObjectiveStore::open(&path).unwrap();
    let saved: (String, Option<i64>, i64) = db.query_row("SELECT action_json, stop_generation, (SELECT stop_generation FROM objectives) FROM controls", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    assert_eq!(saved, (before, None, 0));
    reopened
        .apply_control(
            "legacy-stop",
            ControlAction::Pause,
            "new-pause",
            &owner,
            101,
        )
        .unwrap();
    let generation: i64 = db
        .query_row("SELECT stop_generation FROM objectives", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(generation, 1);
}

#[test]
fn stop_generation_overflow_rolls_back_state_and_audit() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("stop-overflow", "overflow-ingress");
    let owner = input.operator_id.clone();
    store.create_authenticated_objective(input, 100).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE objectives SET stop_generation=?1", [i64::MAX])
        .unwrap();
    assert!(store
        .apply_control(
            "stop-overflow",
            ControlAction::Cancel,
            "overflow",
            &owner,
            101
        )
        .is_err());
    let state: String = db
        .query_row("SELECT state FROM objectives", [], |row| row.get(0))
        .unwrap();
    assert_eq!(state, "pending_approval");
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM controls", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    assert!(db
        .execute("UPDATE objectives SET stop_generation=-1", [])
        .is_err());
    assert!(db
        .execute("UPDATE objectives SET stop_generation=1.5", [])
        .is_err());
}

#[test]
fn original_admission_survives_restart_and_controls_without_reconstruction() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective("admission", "admission-key");
    input.leaves[0].execution = Some(arda_engine::objectives::LeafExecutionSpec {
        objective: input.text.clone(),
        execution_prompt: "fixture evidence bytes: \"quoted\"\nVairë — untrusted advisory".into(),
        verification_prompt: "fixture verification evidence".into(),
        review_prompt: "fixture review evidence".into(),
        approval_envelope: serde_json::json!({"fixture": true}),
        objective_plan_receipt: "fixture-plan-receipt".into(),
    });
    store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    store
        .apply_control(
            "admission",
            ControlAction::Reprioritize { priority: 77 },
            "priority-change",
            &input.operator_id,
            110,
        )
        .unwrap();
    drop(store);
    let reopened = ObjectiveStore::open_existing(&path).unwrap();
    assert_eq!(
        reopened
            .authenticated_admission("admission-key", &input.operator_id)
            .unwrap(),
        Some(input.clone())
    );
    assert!(reopened
        .authenticated_admission("admission-key", "other-owner")
        .is_err());
    assert!(reopened
        .authenticated_admission("missing", &input.operator_id)
        .unwrap()
        .is_none());
    let mut changed = input.clone();
    changed.text.push_str(" changed retry");
    assert!(reopened
        .create_authenticated_objective(changed, 120)
        .is_err());
    assert_eq!(
        reopened
            .authenticated_admission("admission-key", &input.operator_id)
            .unwrap(),
        Some(input.clone())
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    let mut tampered = input.clone();
    tampered.text.push_str(" forged snapshot");
    db.execute(
        "UPDATE objective_admissions SET input_json=?1 WHERE objective_id='admission'",
        [serde_json::to_string(&tampered).unwrap()],
    )
    .unwrap();
    assert!(reopened
        .authenticated_admission("admission-key", &input.operator_id)
        .is_err());
    db.execute(
        "DELETE FROM objective_admissions WHERE objective_id='admission'",
        [],
    )
    .unwrap();
    assert!(
        reopened
            .authenticated_admission("admission-key", &input.operator_id)
            .is_err(),
        "missing legacy snapshots must not be reconstructed from mutable objective state"
    );
}

#[test]
fn admission_snapshot_rolls_back_with_failed_objective_creation() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER fixture_reject_leaf BEFORE INSERT ON leaves BEGIN SELECT RAISE(ABORT, 'fixture leaf failure'); END;").unwrap();
    let input = objective("atomic-admission", "atomic-admission-key");
    assert!(store
        .create_authenticated_objective(input.clone(), 100)
        .is_err());
    let counts: (i64, i64) = db
        .query_row(
            "SELECT (SELECT COUNT(*) FROM objectives), (SELECT COUNT(*) FROM objective_admissions)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (0, 0));
    assert!(store
        .authenticated_admission(&input.idempotency_key, &input.operator_id)
        .unwrap()
        .is_none());
    db.execute_batch("DROP TRIGGER fixture_reject_leaf")
        .unwrap();
    store
        .create_authenticated_objective(input.clone(), 110)
        .unwrap();
    assert_eq!(
        store
            .authenticated_admission(&input.idempotency_key, &input.operator_id)
            .unwrap(),
        Some(input)
    );
}

#[test]
fn deferred_schedule_gates_admission_and_consumes_once_across_restart() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective("deferred", "deferred-message"), 100)
        .unwrap();
    let schedule = ScheduleSpec {
        id: "deferred-wake".into(),
        objective_id: "deferred".into(),
        next_wake_ms: 500,
        recurrence: None,
        idempotency_key: "deferred-schedule".into(),
    };
    store.put_schedule(schedule.clone(), 110).unwrap();
    store
        .apply_control(
            "deferred",
            ControlAction::Approve { revision: 1 },
            "approve-deferred",
            "operator:primary",
            120,
        )
        .unwrap();
    assert!(
        store
            .claim_runnable("worker", 499, 100, 1)
            .unwrap()
            .is_empty(),
        "future schedule admitted early"
    );
    let first = store
        .claim_runnable("worker", 500, 100, 1)
        .unwrap()
        .remove(0);
    assert!(
        store.due_schedules(500, 10).unwrap().is_empty(),
        "one-shot wake was not consumed"
    );
    drop(store);
    let store = ObjectiveStore::open(path).unwrap();
    assert_eq!(store.put_schedule(schedule.clone(), 550).unwrap(), schedule);
    assert!(
        store.due_schedules(550, 10).unwrap().is_empty(),
        "replay rearmed consumed wake"
    );
    let resumed = store
        .claim_runnable("worker", 601, 100, 1)
        .unwrap()
        .remove(0);
    assert_eq!(first.execution_run_id, resumed.execution_run_id);
}

#[test]
fn recurring_schedule_advances_without_replaying_missed_ticks_or_terminal_work() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("recurring", "recurring-message"), 100)
        .unwrap();
    let schedule = ScheduleSpec {
        id: "recurring-wake".into(),
        objective_id: "recurring".into(),
        next_wake_ms: 500,
        recurrence: Some("PT1S".into()),
        idempotency_key: "recurring-schedule".into(),
    };
    store.put_schedule(schedule.clone(), 110).unwrap();
    store
        .apply_control(
            "recurring",
            ControlAction::Approve { revision: 1 },
            "approve-recurring",
            "operator:primary",
            120,
        )
        .unwrap();
    store
        .apply_control(
            "recurring",
            ControlAction::Pause,
            "pause-recurring",
            "operator:primary",
            130,
        )
        .unwrap();
    assert!(store
        .claim_runnable("worker", 2500, 100, 1)
        .unwrap()
        .is_empty());
    assert_eq!(
        store
            .schedule("recurring-wake")
            .unwrap()
            .unwrap()
            .next_wake_ms,
        500
    );
    store
        .apply_control(
            "recurring",
            ControlAction::Resume,
            "resume-recurring",
            "operator:primary",
            2600,
        )
        .unwrap();
    assert_eq!(
        store.claim_runnable("worker", 2600, 100, 1).unwrap().len(),
        1
    );
    assert_eq!(
        store
            .schedule("recurring-wake")
            .unwrap()
            .unwrap()
            .next_wake_ms,
        3500
    );
    assert_eq!(
        store.put_schedule(schedule, 2601).unwrap().next_wake_ms,
        3500
    );
    assert!(store.due_schedules(2601, 10).unwrap().is_empty());
    assert_eq!(store.next_wake_ms(2701).unwrap(), Some(3500));
    assert_eq!(
        store.claim_runnable("worker", 3500, 100, 1).unwrap().len(),
        1
    );
    assert_eq!(
        store
            .schedule("recurring-wake")
            .unwrap()
            .unwrap()
            .next_wake_ms,
        4500
    );
    store
        .apply_control(
            "recurring",
            ControlAction::Cancel,
            "cancel-recurring",
            "operator:primary",
            3700,
        )
        .unwrap();
    assert!(store
        .claim_runnable("worker", 4500, 100, 1)
        .unwrap()
        .is_empty());
    assert_eq!(
        store
            .schedule("recurring-wake")
            .unwrap()
            .unwrap()
            .next_wake_ms,
        4500
    );
}

#[test]
fn schedule_rejects_invalid_or_overflowing_recurrence() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("validation", "validation-message"), 100)
        .unwrap();
    for recurrence in [
        "",
        "PT0S",
        "PT-1S",
        "tomorrow",
        "P1M",
        "PT9223372036854775807H",
    ] {
        assert!(
            store
                .put_schedule(
                    ScheduleSpec {
                        id: "invalid".into(),
                        objective_id: "validation".into(),
                        next_wake_ms: 500,
                        recurrence: Some(recurrence.into()),
                        idempotency_key: "invalid".into()
                    },
                    110
                )
                .is_err(),
            "accepted {recurrence}"
        );
    }
}

#[test]
fn malformed_historical_schedule_does_not_block_unrelated_admission() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    for id in ["broken-schedule", "independent"] {
        store
            .create_authenticated_objective(objective(id, id), 100)
            .unwrap();
        store
            .apply_control(
                id,
                ControlAction::Approve { revision: 1 },
                &format!("approve-{id}"),
                "operator:primary",
                110,
            )
            .unwrap();
    }
    store
        .put_schedule(
            ScheduleSpec {
                id: "historical".into(),
                objective_id: "broken-schedule".into(),
                next_wake_ms: 120,
                recurrence: Some("PT1S".into()),
                idempotency_key: "historical".into(),
            },
            115,
        )
        .unwrap();
    // Older schemas accepted arbitrary nonempty recurrence strings.
    rusqlite::Connection::open(&path).unwrap().execute(
        "UPDATE schedules SET recurrence = 'unsupported historical format' WHERE id = 'historical'", [],
    ).unwrap();
    let claims = store
        .claim_runnable("worker", 130, 100, 4)
        .expect("one malformed historical schedule must not poison the whole round");
    assert_eq!(claims.len(), 2);
    assert!(claims
        .iter()
        .all(|claim| claim.objective_id == "independent"));
    assert!(
        store.due_schedules(130, 10).unwrap().is_empty(),
        "bad schedule causes hot-loop wakes"
    );
    drop(store);
    let store = ObjectiveStore::open(path).unwrap();
    assert!(store
        .claim_runnable("worker", 131, 100, 4)
        .unwrap()
        .is_empty());
    assert_eq!(store.next_wake_ms(131).unwrap(), Some(230));
    let error: String = rusqlite::Connection::open(temp.path().join("objectives.sqlite3"))
        .unwrap()
        .query_row(
            "SELECT error FROM schedule_errors WHERE schedule_id = 'historical'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(error.contains("fixed PT duration"));
}

#[test]
fn execution_identity_survives_lease_reclaim() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective("identity", "identity-message"), 100)
        .unwrap();
    store
        .apply_control(
            "identity",
            ControlAction::Approve { revision: 1 },
            "identity-approval",
            "operator:primary",
            110,
        )
        .unwrap();
    let first = store
        .claim_runnable("worker-one", 120, 100, 1)
        .unwrap()
        .remove(0);
    let first_json = serde_json::to_value(&first).unwrap();
    assert!(
        first_json["execution_run_id"].is_string(),
        "claim must carry a persisted execution identity"
    );
    drop(store);
    let reopened = ObjectiveStore::open(&path).unwrap();
    let next = reopened
        .claim_runnable("worker-two", 221, 100, 1)
        .unwrap()
        .remove(0);
    assert_eq!(first.leaf_id, next.leaf_id);
    assert_eq!(next.attempt, first.attempt + 1);
    assert_eq!(
        serde_json::to_value(&next).unwrap()["execution_run_id"],
        first_json["execution_run_id"]
    );
}

#[test]
fn objective_creation_rejects_dependency_cycles() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    let mut cyclic = objective("objective-cycle", "message-cycle");
    cyclic.leaves[0].dependencies = vec!["objective-cycle-join".to_owned()];

    let error = store
        .create_authenticated_objective(cyclic, 100)
        .unwrap_err();

    assert!(
        error.to_string().contains("contain a cycle"),
        "unexpected error: {error:#}"
    );
    assert!(store.list_objectives().unwrap().is_empty());
}

#[test]
fn revision_is_rejected_after_leaf_execution_begins() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    store
        .claim_runnable("worker-1", 120, 100, 1)
        .unwrap()
        .remove(0);

    let error = store
        .apply_control(
            "objective-1",
            ControlAction::Revise {
                text: "Unsafe mid-execution revision".to_owned(),
            },
            "revision-after-execution",
            "operator:primary",
            121,
        )
        .unwrap_err();

    assert!(
        error.to_string().contains("execution started"),
        "unexpected error: {error:#}"
    );
    assert_eq!(store.objective("objective-1").unwrap().unwrap().revision, 1);
}

#[test]
fn stage_receipts_require_canonical_digests_and_safe_relative_paths() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    let claim = store
        .claim_runnable("worker-1", 120, 100, 1)
        .unwrap()
        .remove(0);
    let receipt = StageReceipt {
        contract: "arda.hermes_execution_receipt.v4".to_owned(),
        stage: ReceiptStage::Execute,
        digest: "sha256:not-a-digest".to_owned(),
        predecessor_digest: None,
        run_path: "data/runs/run-1/execution-receipts/execute.json".to_owned(),
        provider: "provider-a".to_owned(),
        model: "model-a".to_owned(),
        started_at_ms: 121,
        completed_at_ms: 130,
        verdict: "succeeded".to_owned(),
        context_outcome_receipt_id: None,
        context_outcome_receipt_digest: None,
        binding_digest: None,
    };
    let digest_error = store
        .record_stage_receipt(
            &claim.leaf_id,
            "worker-1",
            claim.attempt,
            receipt.clone(),
            131,
        )
        .unwrap_err();
    assert!(digest_error.to_string().contains("lowercase-hex"));

    let path_error = store
        .record_stage_receipt(
            &claim.leaf_id,
            "worker-1",
            claim.attempt,
            StageReceipt {
                digest: format!("sha256:{}", "d".repeat(64)),
                run_path: "../outside.json".to_owned(),
                ..receipt
            },
            131,
        )
        .unwrap_err();
    assert!(path_error.to_string().contains("repository-relative"));
}
use sha2::{Digest, Sha256};
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

fn objective(id: &str, idempotency_key: &str) -> NewObjective {
    NewObjective {
        id: id.to_owned(),
        source_id: format!("source-{id}"),
        idempotency_key: idempotency_key.to_owned(),
        operator_id: "operator:primary".to_owned(),
        text: "Inspect both reviewed projects and join the result.".to_owned(),
        priority: 50,
        projects: vec![
            ProjectAuthority {
                project_id: "project-a".to_owned(),
                contract_digest: "sha256:project-a".to_owned(),
            },
            ProjectAuthority {
                project_id: "project-b".to_owned(),
                contract_digest: "sha256:project-b".to_owned(),
            },
        ],
        leaves: vec![
            NewLeaf {
                id: format!("{id}-a"),
                project_id: Some("project-a".to_owned()),
                workspace_root: "/work/a".to_owned(),
                authority: "read_only".to_owned(),
                dependencies: vec![],
                execution: None,
            },
            NewLeaf {
                id: format!("{id}-b"),
                project_id: Some("project-b".to_owned()),
                workspace_root: "/work/b".to_owned(),
                authority: "read_only".to_owned(),
                dependencies: vec![],
                execution: None,
            },
            NewLeaf {
                id: format!("{id}-join"),
                project_id: None,
                workspace_root: "/work/join".to_owned(),
                authority: "read_only".to_owned(),
                dependencies: vec![format!("{id}-a"), format!("{id}-b")],
                execution: None,
            },
        ],
    }
}

#[test]
fn exhausted_attempts_fail_only_after_the_active_lease_expires() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective("capped", "capped-ingress");
    input.leaves.truncate(1);
    store.create_authenticated_objective(input, 1).unwrap();
    store
        .apply_control(
            "capped",
            ControlAction::Approve { revision: 1 },
            "approve-capped",
            "operator:primary",
            2,
        )
        .unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    for attempt in 1..=5 {
        let claims = store
            .claim_runnable("worker", now + attempt * 100, 100, 1)
            .unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].attempt, attempt);
    }
    let reader = ObjectiveStore::open(&path).unwrap();
    assert_eq!(
        reader.objective("capped").unwrap().unwrap().state,
        ObjectiveState::Running
    );
    assert!(store
        .claim_runnable("worker", now + 601, 100, 1)
        .unwrap()
        .is_empty());
    assert_eq!(
        store.objective("capped").unwrap().unwrap().state,
        ObjectiveState::Failed
    );
}

#[cfg(unix)]
#[test]
fn unavailable_workspace_policy_preserves_reservations_without_advancing_attempts() {
    use std::os::unix::fs::PermissionsExt;
    assert_ne!(
        unsafe { libc::geteuid() },
        0,
        "permission regression requires an unprivileged user"
    );
    for (mode, missing_child) in [
        (0o000, false),
        (0o400, false),
        (0o100, false),
        (0o000, true),
        (0o400, true),
        (0o100, true),
    ] {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let store = ObjectiveStore::open(temp.path().join("state.sqlite3")).unwrap();
        let mut input = objective("unavailable", "unavailable");
        input.leaves.truncate(1);
        input.leaves[0].workspace_root = if missing_child {
            workspace.join("absent").display().to_string()
        } else {
            workspace.display().to_string()
        };
        store.create_authenticated_objective(input, 1).unwrap();
        store
            .apply_control(
                "unavailable",
                ControlAction::Approve { revision: 1 },
                "approval",
                "operator:primary",
                2,
            )
            .unwrap();
        std::fs::set_permissions(&workspace, std::fs::Permissions::from_mode(mode)).unwrap();
        let result = store.claim_runnable("worker", 3, 100, 1);
        std::fs::set_permissions(&workspace, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err(), "mode {mode:o} must block admission");
        assert_eq!(store.leaf("unavailable-a").unwrap().unwrap().attempt, 0);
        std::fs::set_permissions(&workspace, std::fs::Permissions::from_mode(0o500)).unwrap();
        let claim = store.claim_runnable("worker", 4, 100, 1).unwrap().remove(0);
        assert_eq!(claim.attempt, 1);
        std::fs::set_permissions(&workspace, std::fs::Permissions::from_mode(mode)).unwrap();
        let recovery = store.claim_runnable("recovery", 105, 100, 1);
        std::fs::set_permissions(&workspace, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(recovery.is_err());
        assert_eq!(store.leaf("unavailable-a").unwrap().unwrap().attempt, 1);
    }
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("not-created/child");
    let path = temp.path().join("missing.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective("missing", "missing");
    input.leaves.truncate(1);
    input.leaves[0].workspace_root = missing.display().to_string();
    store.create_authenticated_objective(input, 1).unwrap();
    store
        .apply_control(
            "missing",
            ControlAction::Approve { revision: 1 },
            "approval",
            "operator:primary",
            2,
        )
        .unwrap();
    assert_eq!(store.claim_runnable("worker", 3, 100, 1).unwrap().len(), 1);
    assert!(
        !missing.exists(),
        "reservation must not create the workspace"
    );
    drop(store);
    std::fs::create_dir_all(&missing).unwrap();
    let store = ObjectiveStore::open(&path).unwrap();
    assert!(store.claim_runnable("recovery", 104, 100, 1).is_err());
    assert_eq!(store.leaf("missing-a").unwrap().unwrap().attempt, 1);
}

#[cfg(unix)]
#[test]
fn live_workspace_identity_change_blocks_admission_after_reopen() {
    for replace_directory in [false, true] {
        let temp = TempDir::new().unwrap();
        let original = temp.path().join("original");
        let other = temp.path().join("other");
        let alias = temp.path().join("alias");
        std::fs::create_dir(&original).unwrap();
        std::fs::create_dir(&other).unwrap();
        std::os::unix::fs::symlink(&original, &alias).unwrap();
        let path = temp.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        let mut holder = objective("holder", "holder");
        holder.leaves.truncate(1);
        holder.leaves[0].workspace_root = alias.display().to_string();
        store.create_authenticated_objective(holder, 1).unwrap();
        store
            .apply_control(
                "holder",
                ControlAction::Approve { revision: 1 },
                "approve-holder",
                "operator:primary",
                2,
            )
            .unwrap();
        assert_eq!(store.claim_runnable("first", 3, 100, 1).unwrap().len(), 1);
        let old_directory = temp.path().join("old-directory");
        if replace_directory {
            std::fs::rename(&original, &old_directory).unwrap();
            std::fs::create_dir(&original).unwrap();
        } else {
            std::fs::remove_file(&alias).unwrap();
            std::os::unix::fs::symlink(&other, &alias).unwrap();
        }
        let mut candidate = objective("candidate", "candidate");
        candidate.leaves.truncate(1);
        candidate.leaves[0].workspace_root = if replace_directory {
            old_directory.display().to_string()
        } else {
            original.display().to_string()
        };
        let candidate_id = candidate.leaves[0].id.clone();
        store.create_authenticated_objective(candidate, 4).unwrap();
        store
            .apply_control(
                "candidate",
                ControlAction::Approve { revision: 1 },
                "approve-candidate",
                "operator:primary",
                5,
            )
            .unwrap();
        drop(store);
        let reopened = ObjectiveStore::open(&path).unwrap();
        let result = reopened.claim_runnable("second", 6, 100, 1);
        assert!(
            result.is_err(),
            "changed leased root must fail closed: {result:?}"
        );
        assert_eq!(reopened.leaf(&candidate_id).unwrap().unwrap().attempt, 0);
        assert!(
            reopened.claim_runnable("recovery", 104, 100, 1).is_err(),
            "expiry must not silently replace the original identity"
        );
        if replace_directory {
            std::fs::remove_dir(&original).unwrap();
            std::fs::rename(&old_directory, &original).unwrap();
        } else {
            std::fs::remove_file(&alias).unwrap();
            std::os::unix::fs::symlink(&original, &alias).unwrap();
        }
        let recovered = reopened.claim_runnable("recovery", 104, 100, 1).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].objective_id, "holder");
        assert_eq!(recovered[0].workspace_root, alias.display().to_string());
    }
}

#[cfg(unix)]
#[test]
fn physical_workspace_aliases_are_excluded_across_claims_and_restart() {
    let temp = TempDir::new().unwrap();
    let physical = temp.path().join("physical");
    let alias = temp.path().join("alias");
    std::fs::create_dir(&physical).unwrap();
    std::os::unix::fs::symlink(&physical, &alias).unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective("aliases", "aliases-ingress");
    input.leaves.truncate(2);
    input.leaves[0].workspace_root = physical.display().to_string();
    input.leaves[1].workspace_root = alias.display().to_string();
    store.create_authenticated_objective(input, 1).unwrap();
    store
        .apply_control(
            "aliases",
            ControlAction::Approve { revision: 1 },
            "approve-aliases",
            "operator:primary",
            2,
        )
        .unwrap();
    let claims = store.claim_runnable("first", 3, 100, 2).unwrap();
    assert_eq!(
        claims.len(),
        1,
        "one physical directory may hold only one lease"
    );
    drop(store);
    let reopened = ObjectiveStore::open(&path).unwrap();
    assert!(reopened
        .claim_runnable("second", 4, 100, 2)
        .unwrap()
        .is_empty());
    assert_eq!(
        reopened
            .claim_runnable("recovery", 104, 100, 2)
            .unwrap()
            .len(),
        1
    );
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires unshare user/mount namespaces and mount"]
fn bind_mount_workspace_overlap_is_excluded() {
    use std::process::Command;
    struct MountGuard(std::path::PathBuf);
    impl Drop for MountGuard {
        fn drop(&mut self) {
            let result = Command::new("umount").arg(&self.0).status();
            let unmounted = result.as_ref().is_ok_and(|status| status.success());
            if std::thread::panicking() {
                if !unmounted {
                    eprintln!("bind fixture unmount failed: {result:?}");
                }
            } else {
                assert!(unmounted, "bind fixture unmount failed: {result:?}");
            }
        }
    }
    if std::env::var_os("ARDA_BIND_ROOT_TEST_CHILD").is_none() {
        let status = Command::new("unshare")
            .args(["--user", "--map-root-user", "--mount"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "bind_mount_workspace_overlap_is_excluded",
                "--ignored",
                "--nocapture",
            ])
            .env("ARDA_BIND_ROOT_TEST_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "mount namespace regression failed");
        return;
    }
    let temp = TempDir::new().unwrap();
    let physical = temp.path().join("physical");
    let container = temp.path().join("container with space");
    let alias = container.join("alias");
    std::fs::create_dir_all(physical.join("child")).unwrap();
    std::fs::create_dir_all(&alias).unwrap();
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(&physical)
        .arg(&alias)
        .status()
        .unwrap()
        .success());
    // Declared after TempDir so unwinding unmounts before deleting its tree.
    let mount_guard = MountGuard(alias.clone());
    let second_container = temp.path().join("second-container");
    let second_alias = second_container.join("nested");
    std::fs::create_dir_all(&second_alias).unwrap();
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(physical.join("child"))
        .arg(&second_alias)
        .status()
        .unwrap()
        .success());
    let second_mount_guard = MountGuard(second_alias);
    for (index, (first, second, expected)) in [
        (physical.clone(), alias.clone(), 1),
        (physical.clone(), alias.join("child"), 1),
        (alias.join("child"), physical.clone(), 1),
        (physical.join("child"), alias.join("other"), 2),
        (container.clone(), physical.clone(), 1),
        (physical.join("child"), container.clone(), 1),
        (container.clone(), temp.path().join("independent"), 2),
        (container.clone(), second_container.clone(), 1),
        (second_container, container.clone(), 1),
    ]
    .into_iter()
    .enumerate()
    {
        let path = temp.path().join(format!("bind-{index}.sqlite3"));
        let store = ObjectiveStore::open(&path).unwrap();
        let mut input = objective("bind", "bind-ingress");
        input.leaves.truncate(2);
        input.leaves[0].workspace_root = first.display().to_string();
        input.leaves[1].workspace_root = second.display().to_string();
        store.create_authenticated_objective(input, 1).unwrap();
        store
            .apply_control(
                "bind",
                ControlAction::Approve { revision: 1 },
                "approval",
                "operator:primary",
                2,
            )
            .unwrap();
        assert_eq!(
            store.claim_runnable("worker", 3, 100, 2).unwrap().len(),
            expected,
            "bind case {index}"
        );
        drop(store);
        assert!(ObjectiveStore::open(&path)
            .unwrap()
            .claim_runnable("second", 4, 100, 2)
            .unwrap()
            .is_empty());
    }
    // The workspace inode stays unchanged when a nested mount changes. Recovery
    // must compare the persisted admission topology, not capture a new baseline.
    let path = temp.path().join("topology.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective("topology", "topology-ingress");
    input.leaves.truncate(1);
    input.leaves[0].workspace_root = container.display().to_string();
    store.create_authenticated_objective(input, 1).unwrap();
    store
        .apply_control(
            "topology",
            ControlAction::Approve { revision: 1 },
            "approval",
            "operator:primary",
            2,
        )
        .unwrap();
    let claim = store.claim_runnable("worker", 3, 100, 1).unwrap().remove(0);
    drop(store);
    let nested = container.join("changed");
    std::fs::create_dir(&nested).unwrap();
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(&physical)
        .arg(&nested)
        .status()
        .unwrap()
        .success());
    let changed_guard = MountGuard(nested);
    let reopened = ObjectiveStore::open(&path).unwrap();
    assert!(
        reopened.claim_runnable("live", 4, 100, 1).is_err(),
        "nested mount changes must block live admission"
    );
    assert!(
        reopened.claim_runnable("recovery", 104, 100, 1).is_err(),
        "nested mount changes must block expired recovery"
    );
    assert_eq!(reopened.leaf(&claim.leaf_id).unwrap().unwrap().attempt, 1);
    drop(changed_guard);
    drop(second_mount_guard);
    drop(mount_guard);
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(&physical)
        .arg(&alias)
        .status()
        .unwrap()
        .success());
    let mount_guard = MountGuard(alias.clone());
    assert!(std::panic::catch_unwind(move || {
        let _guard = mount_guard;
        panic!("exercise mount fixture unwind cleanup");
    })
    .is_err());
    use std::os::unix::fs::MetadataExt;
    let original = std::fs::metadata(&physical).unwrap();
    let unmounted = std::fs::metadata(&alias).unwrap();
    assert_ne!(
        (original.dev(), original.ino()),
        (unmounted.dev(), unmounted.ino()),
        "panic cleanup must remove the bind mount"
    );
}

#[cfg(unix)]
#[test]
fn migration_does_not_guess_the_identity_of_an_existing_lease() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut input = objective("legacy", "legacy");
    input.leaves.truncate(1);
    input.leaves[0].workspace_root = temp.path().display().to_string();
    store.create_authenticated_objective(input, 1).unwrap();
    store
        .apply_control(
            "legacy",
            ControlAction::Approve { revision: 1 },
            "approval",
            "operator:primary",
            2,
        )
        .unwrap();
    let claim = store
        .claim_runnable("old-worker", 3, 100, 1)
        .unwrap()
        .remove(0);
    drop(store);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("DROP TABLE lease_workspace_identities")
        .unwrap();
    let reopened = ObjectiveStore::open(&path).unwrap();
    assert!(reopened.claim_runnable("new-worker", 4, 100, 1).is_err());
    assert!(
        reopened.claim_runnable("recovery", 104, 100, 1).is_err(),
        "expired legacy identity must remain unknown"
    );
    assert_eq!(reopened.leaf(&claim.leaf_id).unwrap().unwrap().attempt, 1);
    let count: u64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM lease_workspace_identities",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[cfg(unix)]
#[test]
fn physical_workspace_exclusion_handles_nested_missing_and_relative_roots() {
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let physical = temp.path().join("physical");
    let alias = temp.path().join("alias");
    std::fs::create_dir(&physical).unwrap();
    std::os::unix::fs::symlink(&physical, &alias).unwrap();
    let cwd = std::env::current_dir().unwrap();
    for (index, second) in [
        alias.join("not-created"),
        alias.join("."),
        alias.strip_prefix(&cwd).unwrap().to_path_buf(),
    ]
    .into_iter()
    .enumerate()
    {
        let store =
            ObjectiveStore::open(temp.path().join(format!("case-{index}.sqlite3"))).unwrap();
        let mut input = objective("nested", "nested-ingress");
        input.leaves.truncate(2);
        input.leaves[0].workspace_root = physical.display().to_string();
        input.leaves[1].workspace_root = second.display().to_string();
        store.create_authenticated_objective(input, 1).unwrap();
        store
            .apply_control(
                "nested",
                ControlAction::Approve { revision: 1 },
                "approval",
                "operator:primary",
                2,
            )
            .unwrap();
        assert_eq!(store.claim_runnable("worker", 3, 100, 2).unwrap().len(), 1);
    }
}

#[cfg(unix)]
#[test]
fn concurrent_objectives_cannot_claim_physical_aliases() {
    let temp = TempDir::new().unwrap();
    let physical = temp.path().join("physical");
    let alias = temp.path().join("alias");
    std::fs::create_dir(&physical).unwrap();
    std::os::unix::fs::symlink(&physical, &alias).unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    for (id, root) in [("first", physical), ("second", alias)] {
        let mut input = objective(id, id);
        input.leaves.truncate(1);
        input.leaves[0].workspace_root = root.display().to_string();
        store.create_authenticated_objective(input, 1).unwrap();
        store
            .apply_control(
                id,
                ControlAction::Approve { revision: 1 },
                &format!("approve-{id}"),
                "operator:primary",
                2,
            )
            .unwrap();
    }
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|index| {
            let store = store.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                store
                    .claim_runnable(&format!("worker-{index}"), 3, 100, 2)
                    .unwrap()
                    .len()
            })
        })
        .collect::<Vec<_>>();
    let total: usize = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .sum();
    assert_eq!(total, 1);
}

#[cfg(unix)]
#[test]
fn blocked_aliases_do_not_starve_an_independent_workspace() {
    let temp = TempDir::new().unwrap();
    let physical = temp.path().join("physical");
    let alias = temp.path().join("alias");
    let independent = temp.path().join("independent");
    std::fs::create_dir(&physical).unwrap();
    std::fs::create_dir(&independent).unwrap();
    std::os::unix::fs::symlink(&physical, &alias).unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    let mut holder = objective("holder", "holder-ingress");
    holder.leaves.truncate(1);
    holder.leaves[0].workspace_root = physical.display().to_string();
    store.create_authenticated_objective(holder, 1).unwrap();
    store
        .apply_control(
            "holder",
            ControlAction::Approve { revision: 1 },
            "approve-holder",
            "operator:primary",
            2,
        )
        .unwrap();
    assert_eq!(store.claim_runnable("holder", 3, 100, 1).unwrap().len(), 1);
    let mut input = objective("waiting", "waiting-ingress");
    let template = input.leaves[0].clone();
    input.leaves = (0..45)
        .map(|index| NewLeaf {
            id: format!("blocked-{index:03}"),
            workspace_root: alias.display().to_string(),
            ..template.clone()
        })
        .collect();
    input.leaves.push(NewLeaf {
        id: "z-independent".into(),
        workspace_root: independent.display().to_string(),
        ..template
    });
    store.create_authenticated_objective(input, 4).unwrap();
    store
        .apply_control(
            "waiting",
            ControlAction::Approve { revision: 1 },
            "approve-waiting",
            "operator:primary",
            5,
        )
        .unwrap();
    let claimed = store.claim_runnable("next", 6, 100, 1).unwrap();
    assert_eq!(
        claimed.len(),
        1,
        "blocked aliases must not exhaust the candidate window"
    );
    assert_eq!(claimed[0].leaf_id, "z-independent");
}

#[test]
fn same_objective_cannot_lease_the_same_workspace_twice() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    let mut input = objective("shared", "shared-ingress");
    input.leaves[1].workspace_root = input.leaves[0].workspace_root.clone();
    input.leaves[0].authority = "mutation".into();
    input.leaves[1].authority = "mutation".into();
    store.create_authenticated_objective(input, 100).unwrap();
    store
        .apply_control(
            "shared",
            ControlAction::Approve { revision: 1 },
            "approve-shared",
            "operator:primary",
            110,
        )
        .unwrap();
    assert_eq!(
        store.claim_runnable("worker", 120, 100, 4).unwrap().len(),
        1
    );
}

#[test]
fn expired_claim_resumes_the_persisted_stage() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("resume", "resume-ingress"), 100)
        .unwrap();
    store
        .apply_control(
            "resume",
            ControlAction::Approve { revision: 1 },
            "approve-resume",
            "operator:primary",
            110,
        )
        .unwrap();
    let first = store
        .claim_runnable("first", 120, 100, 1)
        .unwrap()
        .remove(0);
    let digest = format!("sha256:{}", "a".repeat(64));
    store
        .record_stage_receipt(
            &first.leaf_id,
            "first",
            first.attempt,
            StageReceipt {
                contract: "arda.hermes_execution_receipt.v4".into(),
                stage: ReceiptStage::Execute,
                digest: digest.clone(),
                predecessor_digest: None,
                run_path: "data/runs/resume/execution-receipts/execute.json".into(),
                provider: "test".into(),
                model: "test".into(),
                started_at_ms: 120,
                completed_at_ms: 130,
                verdict: "succeeded".into(),
                context_outcome_receipt_id: None,
                context_outcome_receipt_digest: None,
                binding_digest: None,
            },
            130,
        )
        .unwrap();
    let resumed = store.claim_runnable("second", 221, 100, 4).unwrap();
    let resumed = resumed
        .iter()
        .find(|claim| claim.leaf_id == first.leaf_id)
        .expect("expired verify stage must remain runnable");
    assert_eq!(resumed.stage, LeafStage::Verify);
    assert_eq!(
        resumed.current_receipt_digest.as_deref(),
        Some(digest.as_str())
    );
}

fn close_claim(store: &ObjectiveStore, leaf_id: &str, lease_owner: &str, now_ms: i64) {
    let attempt = store.leaf(leaf_id).unwrap().unwrap().attempt;
    let mut predecessor = None;
    for (offset, (stage, stage_name)) in [
        (ReceiptStage::Execute, "execute"),
        (ReceiptStage::Verify, "verify"),
        (ReceiptStage::Review, "review"),
        (ReceiptStage::Close, "close"),
    ]
    .into_iter()
    .enumerate()
    {
        let digest = format!(
            "sha256:{:x}",
            Sha256::digest(format!("{leaf_id}-{stage_name}").as_bytes())
        );
        store
            .record_stage_receipt(
                leaf_id,
                lease_owner,
                attempt,
                StageReceipt {
                    contract: "arda.hermes_execution_receipt.v4".to_owned(),
                    stage,
                    digest: digest.clone(),
                    predecessor_digest: predecessor,
                    run_path: format!("data/runs/{leaf_id}/execution-receipts/{stage_name}.json"),
                    provider: "provider-a".to_owned(),
                    model: "model-a".to_owned(),
                    started_at_ms: now_ms + offset as i64,
                    completed_at_ms: now_ms + offset as i64 + 1,
                    verdict: "succeeded".to_owned(),
                    context_outcome_receipt_id: None,
                    context_outcome_receipt_digest: None,
                    binding_digest: None,
                },
                now_ms + offset as i64 + 1,
            )
            .unwrap();
        predecessor = Some(digest);
    }
}

#[test]
fn objective_creation_is_atomic_idempotent_and_restart_durable() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let created = store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    let replayed = store
        .create_authenticated_objective(objective("objective-1", "message-1"), 101)
        .unwrap();

    assert_eq!(created, replayed);
    assert_eq!(created.state, ObjectiveState::PendingApproval);
    assert_eq!(created.project_ids, vec!["project-a", "project-b"]);
    assert_eq!(store.list_leaves("objective-1").unwrap().len(), 3);
    drop(store);

    let reopened = ObjectiveStore::open(&path).unwrap();
    let recovered = reopened.objective("objective-1").unwrap().unwrap();
    assert_eq!(recovered, created);
    assert_eq!(reopened.list_leaves("objective-1").unwrap().len(), 3);
}

#[test]
fn duplicate_ingress_key_with_different_payload_fails_closed() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();

    let error = store
        .create_authenticated_objective(objective("different-id", "message-1"), 101)
        .unwrap_err();
    assert!(error.to_string().contains("idempotency conflict"));
    assert!(store.objective("different-id").unwrap().is_none());
}

#[test]
fn transactional_claims_require_approval_respect_dependencies_and_do_not_duplicate() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    assert!(store
        .claim_runnable("worker-0", 110, 100, 8)
        .unwrap()
        .is_empty());
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            120,
        )
        .unwrap();
    drop(store);

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for owner in ["worker-1", "worker-2"] {
        let path = path.clone();
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            let store = ObjectiveStore::open(path).unwrap();
            barrier.wait();
            store.claim_runnable(owner, 130, 100, 2).unwrap()
        }));
    }
    barrier.wait();
    let mut claimed = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    claimed.sort_by(|left, right| left.leaf_id.cmp(&right.leaf_id));

    assert_eq!(claimed.len(), 2);
    assert_eq!(claimed[0].leaf_id, "objective-1-a");
    assert_eq!(claimed[1].leaf_id, "objective-1-b");
    assert!(claimed
        .iter()
        .all(|claim| claim.stage == LeafStage::Execute));
    assert!(!claimed
        .iter()
        .any(|claim| claim.leaf_id == "objective-1-join"));

    let store = ObjectiveStore::open(path).unwrap();
    for claim in &claimed {
        close_claim(&store, &claim.leaf_id, &claim.lease_owner, 140);
    }
    let join = store
        .claim_runnable("join-worker", 150, 100, 1)
        .unwrap()
        .remove(0);
    assert_eq!(join.leaf_id, "objective-1-join");
    assert_eq!(join.dependency_receipts.len(), 2);
    assert!(join
        .dependency_receipts
        .iter()
        .all(|receipt| receipt.stage == ReceiptStage::Close && receipt.verdict == "succeeded"));
    assert_eq!(
        join.dependency_receipts
            .iter()
            .map(|receipt| receipt.run_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "data/runs/objective-1-a/execution-receipts/close.json",
            "data/runs/objective-1-b/execution-receipts/close.json",
        ]
    );
}

#[test]
fn expired_lease_is_recovered_after_restart() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut single_leaf = objective("objective-1", "message-1");
    single_leaf.leaves.truncate(1);
    store
        .create_authenticated_objective(single_leaf, 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    let first = store.claim_runnable("worker-1", 120, 10, 1).unwrap();
    assert_eq!(first.len(), 1);
    drop(store);

    let reopened = ObjectiveStore::open(&path).unwrap();
    assert!(reopened
        .claim_runnable("worker-2", 129, 10, 1)
        .unwrap()
        .is_empty());
    let recovered = reopened.claim_runnable("worker-2", 131, 10, 1).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].leaf_id, first[0].leaf_id);
    assert_eq!(recovered[0].attempt, 2);
}

#[test]
fn schedules_are_idempotent_restart_durable_and_pause_with_the_objective() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    let schedule = ScheduleSpec {
        id: "schedule-1".to_owned(),
        objective_id: "objective-1".to_owned(),
        next_wake_ms: 200,
        recurrence: Some("PT5M".to_owned()),
        idempotency_key: "schedule-message-1".to_owned(),
    };
    assert_eq!(store.put_schedule(schedule.clone(), 110).unwrap(), schedule);
    assert_eq!(store.put_schedule(schedule.clone(), 111).unwrap(), schedule);
    assert_eq!(
        store.due_schedules(200, 10).unwrap(),
        vec![schedule.clone()]
    );
    store
        .apply_control(
            "objective-1",
            ControlAction::Pause,
            "pause-1",
            "operator:primary",
            120,
        )
        .unwrap();
    assert!(store.due_schedules(300, 10).unwrap().is_empty());
    drop(store);

    let reopened = ObjectiveStore::open(path).unwrap();
    assert_eq!(reopened.schedule("schedule-1").unwrap(), Some(schedule));
}

#[test]
fn stage_progression_requires_exact_receipt_lineage() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    let claim = store
        .claim_runnable("worker-1", 120, 100, 1)
        .unwrap()
        .remove(0);

    let mut execute = StageReceipt {
        contract: "arda.hermes_execution_receipt.v4".to_owned(),
        stage: ReceiptStage::Execute,
        digest: format!("sha256:{}", "a".repeat(64)),
        predecessor_digest: None,
        run_path: "data/runs/run-1/execution-receipts/execute.json".to_owned(),
        provider: "provider-a".to_owned(),
        model: "model-a".to_owned(),
        started_at_ms: 121,
        completed_at_ms: 130,
        verdict: "succeeded".to_owned(),
        context_outcome_receipt_id: Some("context-outcome-1".to_owned()),
        context_outcome_receipt_digest: Some(format!("sha256:{}", "a".repeat(64))),
        binding_digest: None,
    };
    execute.binding_digest = Some(execute.computed_binding_digest().unwrap());
    let mut invalid_contract = execute.clone();
    invalid_contract.contract = "legacy.synthetic_receipt.v1".to_owned();
    assert!(store
        .record_stage_receipt(
            &claim.leaf_id,
            "worker-1",
            claim.attempt,
            invalid_contract,
            131
        )
        .unwrap_err()
        .to_string()
        .contains("arda.hermes_execution_receipt.v4"));
    store
        .record_stage_receipt(
            &claim.leaf_id,
            "worker-1",
            claim.attempt,
            execute.clone(),
            131,
        )
        .unwrap();
    let mut conflicting_outcome = execute;
    conflicting_outcome.context_outcome_receipt_id = Some("context-outcome-2".to_owned());
    conflicting_outcome.binding_digest =
        Some(conflicting_outcome.computed_binding_digest().unwrap());
    assert!(store
        .record_stage_receipt(
            &claim.leaf_id,
            "worker-1",
            claim.attempt,
            conflicting_outcome,
            131
        )
        .unwrap_err()
        .to_string()
        .contains("idempotency conflict"));

    let wrong = StageReceipt {
        contract: "arda.hermes_execution_receipt.v4".to_owned(),
        stage: ReceiptStage::Verify,
        digest: format!("sha256:{}", "b".repeat(64)),
        predecessor_digest: Some(format!("sha256:{}", "c".repeat(64))),
        run_path: "data/runs/run-1/execution-receipts/verify.json".to_owned(),
        provider: "provider-b".to_owned(),
        model: "model-b".to_owned(),
        started_at_ms: 132,
        completed_at_ms: 140,
        verdict: "succeeded".to_owned(),
        context_outcome_receipt_id: None,
        context_outcome_receipt_digest: None,
        binding_digest: None,
    };
    assert!(store
        .record_stage_receipt(&claim.leaf_id, "worker-1", claim.attempt, wrong, 141)
        .unwrap_err()
        .to_string()
        .contains("predecessor"));

    let verify = StageReceipt {
        predecessor_digest: Some(format!("sha256:{}", "a".repeat(64))),
        ..StageReceipt {
            contract: "arda.hermes_execution_receipt.v4".to_owned(),
            stage: ReceiptStage::Verify,
            digest: format!("sha256:{}", "b".repeat(64)),
            predecessor_digest: None,
            run_path: "data/runs/run-1/execution-receipts/verify.json".to_owned(),
            provider: "provider-b".to_owned(),
            model: "model-b".to_owned(),
            started_at_ms: 132,
            completed_at_ms: 140,
            verdict: "succeeded".to_owned(),
            context_outcome_receipt_id: None,
            context_outcome_receipt_digest: None,
            binding_digest: None,
        }
    };
    store
        .record_stage_receipt(&claim.leaf_id, "worker-1", claim.attempt, verify, 141)
        .unwrap();
    assert_eq!(
        store.leaf(&claim.leaf_id).unwrap().unwrap().stage,
        LeafStage::Review
    );
}

#[test]
fn revision_invalidates_approval_and_terminal_root_requires_every_leaf_close() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Revise {
                text: "Revised reviewed objective".to_owned(),
            },
            "revision-1",
            "operator:primary",
            120,
        )
        .unwrap();
    let revised = store.objective("objective-1").unwrap().unwrap();
    assert_eq!(revised.revision, 2);
    assert_eq!(revised.state, ObjectiveState::PendingApproval);
    assert!(store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "stale-approval",
            "operator:primary",
            121,
        )
        .unwrap_err()
        .to_string()
        .contains("revision"));
    assert!(store
        .close_objective("objective-1", "sha256:root", 130)
        .unwrap_err()
        .to_string()
        .contains("not complete"));
}
