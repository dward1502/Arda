#![cfg(feature = "full-cli")]

use arda_aule::prometheus::autopilot::{
    promote_knowledge_tasks,
    schedule::{
        ScheduleLedger, ScheduleMode, ScheduleRecord, ScheduleState, SCHEDULE_RECORD_CONTRACT,
    },
    KnowledgeTriageConfig,
};

#[test]
fn historical_active_authority_cannot_run_writer_callbacks() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("schedules.jsonl");
    let record = ScheduleRecord {
        contract: SCHEDULE_RECORD_CONTRACT.into(),
        task_id: "task".into(),
        objective_id: "objective".into(),
        mode: ScheduleMode::Immediate,
        state: ScheduleState::Scheduled,
        not_before_utc: None,
        interval_seconds: None,
        recorded_at_utc: chrono::Utc::now(),
        reason: None,
    };
    let bytes = format!("{}\n", serde_json::to_string(&record).unwrap());
    std::fs::write(&path, &bytes).unwrap();
    let invoked = std::cell::Cell::new(false);
    let result = ScheduleLedger::new(&path).with_active_authority("task", "objective", || {
        invoked.set(true);
        Ok(())
    });
    assert!(
        !invoked.get(),
        "retired active authority invoked writer callback"
    );
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes.as_bytes());
}

#[test]
fn retired_schedule_callbacks_never_run_or_modify_history() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing/schedules.jsonl");
    let ledger = ScheduleLedger::new(&path);
    for historical in [false, true] {
        let bytes = b"retain even malformed legacy history\n";
        if historical {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
        }
        let now = chrono::Utc::now();
        let errors = [
            ledger.pause("task", "objective", now, "pause").unwrap_err(),
            ledger
                .resume("task", "objective", now, "resume")
                .unwrap_err(),
            ledger.advance_after_completion("task", now).unwrap_err(),
            ledger
                .advance_after_completion_when("task", now, |_| panic!("predicate invoked"))
                .unwrap_err(),
            ledger
                .with_completion_transition("task", "objective", now, || -> std::io::Result<()> {
                    panic!("completion invoked")
                })
                .unwrap_err(),
            ledger
                .with_cancellation_transition(
                    "task",
                    "objective",
                    now,
                    None,
                    || -> std::io::Result<()> { panic!("cancellation invoked") },
                )
                .unwrap_err(),
        ];
        for error in errors {
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            assert!(error.to_string().contains("retired"));
        }
        if historical {
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        } else {
            assert!(!path.parent().unwrap().exists());
        }
    }
}

#[test]
fn reading_missing_legacy_schedules_does_not_provision_history() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing/schedules.jsonl");
    let ledger = ScheduleLedger::new(&path);
    assert!(ledger.effective().unwrap().is_empty());
    ledger
        .with_effective(|records| {
            assert!(records.is_empty());
            Ok(())
        })
        .unwrap();
    assert_eq!(
        ledger
            .with_active_authority("task", "objective", || -> std::io::Result<()> {
                panic!("missing authority invoked callback")
            })
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn retired_library_writers_refuse_before_creating_queue_or_schedule_state() {
    let root = tempfile::tempdir().unwrap();
    let untouched = root.path().join("missing-root");
    let config = KnowledgeTriageConfig::for_root(&untouched)
        .with_dry_run(false)
        .with_approval_evidence("test:approval-does-not-revive-retired-authority");
    assert!(promote_knowledge_tasks(&config)
        .unwrap_err()
        .to_string()
        .contains("retired"));
    let schedule = ScheduleRecord {
        contract: SCHEDULE_RECORD_CONTRACT.into(),
        task_id: "test-task".into(),
        objective_id: "test-objective".into(),
        mode: ScheduleMode::Deferred,
        state: ScheduleState::Scheduled,
        not_before_utc: Some(chrono::Utc::now()),
        interval_seconds: None,
        recorded_at_utc: chrono::Utc::now(),
        reason: None,
    };
    assert!(ScheduleLedger::new(untouched.join("schedules.jsonl"))
        .append(&schedule)
        .unwrap_err()
        .to_string()
        .contains("retired"));
    assert!(!untouched.exists());
}
