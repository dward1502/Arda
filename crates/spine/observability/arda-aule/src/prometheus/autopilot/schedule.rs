#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Append-only schedule authority for governed queue work.

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const SCHEDULE_RECORD_CONTRACT: &str = "arda.workbench.schedule_record.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    Immediate,
    Once,
    Deferred,
    Recurring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleState {
    Scheduled,
    Paused,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRecord {
    pub contract: String,
    pub task_id: String,
    pub objective_id: String,
    pub mode: ScheduleMode,
    pub state: ScheduleState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_before_utc: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_seconds: Option<u64>,
    pub recorded_at_utc: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScheduleLedger {
    path: PathBuf,
}

impl ScheduleLedger {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn append(&self, record: &ScheduleRecord) -> io::Result<()> {
        validate_record(record)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;
        file.lock_exclusive()?;
        let result = (|| {
            if let Some(previous) = read_effective(&file)?.get(&record.task_id) {
                validate_transition(previous, record, io::ErrorKind::InvalidInput)?;
            }
            serde_json::to_writer(&mut file, record)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn effective(&self) -> io::Result<BTreeMap<String, ScheduleRecord>> {
        let file = match OpenOptions::new().read(true).open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(error) => return Err(error),
        };
        file.lock_shared()?;
        let result = read_effective(&file);
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(records), Ok(())) => Ok(records),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn with_effective<T>(
        &self,
        operation: impl FnOnce(&BTreeMap<String, ScheduleRecord>) -> io::Result<T>,
    ) -> io::Result<T> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.path)?;
        file.lock_shared()?;
        let result = read_effective(&file).and_then(|records| operation(&records));
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn advance_after_completion(
        &self,
        task_id: &str,
        completed_at_utc: DateTime<Utc>,
    ) -> io::Result<ScheduleRecord> {
        self.advance_after_completion_when(task_id, completed_at_utc, |_| Ok(true))?
            .ok_or_else(|| io::Error::other("completion transition unexpectedly vetoed"))
    }

    pub fn advance_after_completion_when(
        &self,
        task_id: &str,
        completed_at_utc: DateTime<Utc>,
        still_current: impl FnOnce(&ScheduleRecord) -> io::Result<bool>,
    ) -> io::Result<Option<ScheduleRecord>> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;
        file.lock_exclusive()?;
        let result = (|| {
            let mut record = read_effective(&file)?
                .remove(task_id)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "schedule not found"))?;
            if record.state != ScheduleState::Scheduled {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "only active schedules may advance",
                ));
            }
            if !still_current(&record)? {
                return Ok(None);
            }
            if record.mode == ScheduleMode::Recurring {
                let (Some(due), Some(interval_seconds)) =
                    (record.not_before_utc, record.interval_seconds)
                else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "recurring schedule requires a due time and interval",
                    ));
                };
                if interval_seconds == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "recurring schedule interval must be positive",
                    ));
                }
                let interval_seconds = i64::try_from(interval_seconds).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "schedule interval is too large",
                    )
                })?;
                let intervals = if due > completed_at_utc {
                    0
                } else {
                    (completed_at_utc - due).num_seconds() / interval_seconds + 1
                };
                record.not_before_utc = Some(
                    due + chrono::Duration::seconds(
                        interval_seconds.checked_mul(intervals).ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidInput, "schedule advance overflow")
                        })?,
                    ),
                );
                record.reason = Some("recurring completion advanced schedule".into());
            } else {
                record.state = ScheduleState::Completed;
                record.reason = Some("one-shot schedule completed".into());
            }
            record.recorded_at_utc = completed_at_utc;
            validate_record(&record)?;
            serde_json::to_writer(&mut file, &record)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()?;
            Ok(Some(record))
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(record), Ok(())) => Ok(record),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn with_completion_transition<T>(
        &self,
        task_id: &str,
        objective_id: &str,
        completed_at_utc: DateTime<Utc>,
        append_queue_terminal: impl FnOnce() -> io::Result<T>,
    ) -> io::Result<T> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;
        file.lock_exclusive()?;
        let result = (|| {
            let mut record = read_effective(&file)?
                .remove(task_id)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "schedule not found"))?;
            if record.objective_id != objective_id {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "schedule objective lineage does not match queue task",
                ));
            }
            if record.state != ScheduleState::Scheduled {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "only active schedules may advance",
                ));
            }
            let terminal = append_queue_terminal()?;
            advance_record(&mut record, completed_at_utc)?;
            serde_json::to_writer(&mut file, &record)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()?;
            Ok(terminal)
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn with_active_authority<T>(
        &self,
        task_id: &str,
        objective_id: &str,
        operation: impl FnOnce() -> io::Result<T>,
    ) -> io::Result<T> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.path)?;
        file.lock_exclusive()?;
        let result = (|| {
            let record = read_effective(&file)?
                .remove(task_id)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "schedule not found"))?;
            validate_active_authority(&record, objective_id)?;
            operation()
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn with_cancellation_transition<T>(
        &self,
        task_id: &str,
        objective_id: &str,
        cancelled_at_utc: DateTime<Utc>,
        reason: Option<&str>,
        append_queue_terminal: impl FnOnce() -> io::Result<T>,
    ) -> io::Result<T> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;
        file.lock_exclusive()?;
        let result = (|| {
            let mut record = read_effective(&file)?
                .remove(task_id)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "schedule not found"))?;
            if record.objective_id != objective_id {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "schedule objective lineage does not match queue task",
                ));
            }
            if record.state == ScheduleState::Cancelled {
                return append_queue_terminal();
            }
            if record.state != ScheduleState::Scheduled {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "only active schedules may be cancelled",
                ));
            }
            let previous = record.clone();
            record.state = ScheduleState::Cancelled;
            record.recorded_at_utc = cancelled_at_utc;
            record.reason = reason.map(str::to_owned);
            validate_record(&record)?;
            validate_transition(&previous, &record, io::ErrorKind::InvalidInput)?;
            serde_json::to_writer(&mut file, &record)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()?;
            append_queue_terminal()
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }
}

fn validate_active_authority(record: &ScheduleRecord, objective_id: &str) -> io::Result<()> {
    if record.objective_id != objective_id {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "schedule objective lineage does not match queue task",
        ));
    }
    if record.state != ScheduleState::Scheduled {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only active schedules authorize queue mutation",
        ));
    }
    Ok(())
}

fn advance_record(record: &mut ScheduleRecord, completed_at_utc: DateTime<Utc>) -> io::Result<()> {
    if record.mode == ScheduleMode::Recurring {
        let (Some(due), Some(interval_seconds)) = (record.not_before_utc, record.interval_seconds)
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "recurring schedule requires a due time and interval",
            ));
        };
        if interval_seconds == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "recurring schedule interval must be positive",
            ));
        }
        let interval_seconds = i64::try_from(interval_seconds).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "schedule interval is too large",
            )
        })?;
        let intervals = if due > completed_at_utc {
            0
        } else {
            (completed_at_utc - due).num_seconds() / interval_seconds + 1
        };
        record.not_before_utc = Some(
            due + chrono::Duration::seconds(interval_seconds.checked_mul(intervals).ok_or_else(
                || io::Error::new(io::ErrorKind::InvalidInput, "schedule advance overflow"),
            )?),
        );
        record.reason = Some("recurring completion advanced schedule".into());
    } else {
        record.state = ScheduleState::Completed;
        record.reason = Some("one-shot schedule completed".into());
    }
    record.recorded_at_utc = completed_at_utc;
    validate_record(record)
}

fn read_effective(file: &std::fs::File) -> io::Result<BTreeMap<String, ScheduleRecord>> {
    let mut effective: BTreeMap<String, ScheduleRecord> = BTreeMap::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: ScheduleRecord = serde_json::from_str(&line).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid schedule record on line {}: {error}", index + 1),
            )
        })?;
        validate_record(&record)?;
        if let Some(previous) = effective.get(&record.task_id) {
            validate_transition(previous, &record, io::ErrorKind::InvalidData).map_err(
                |error| {
                    io::Error::new(
                        error.kind(),
                        format!("invalid schedule transition on line {}: {error}", index + 1),
                    )
                },
            )?;
        }
        effective.insert(record.task_id.clone(), record);
    }
    Ok(effective)
}

fn validate_record(record: &ScheduleRecord) -> io::Result<()> {
    if record.contract != SCHEDULE_RECORD_CONTRACT
        || record.task_id.trim().is_empty()
        || record.objective_id.trim().is_empty()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "schedule record requires the canonical contract and non-empty lineage",
        ));
    }
    if matches!(
        record.mode,
        ScheduleMode::Once | ScheduleMode::Deferred | ScheduleMode::Recurring
    ) && record.not_before_utc.is_none()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "timed schedule requires a due time",
        ));
    }
    if record.mode == ScheduleMode::Recurring
        && !record.interval_seconds.is_some_and(|value| value > 0)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "recurring schedule requires a due time and positive interval",
        ));
    }
    Ok(())
}

fn validate_transition(
    previous: &ScheduleRecord,
    next: &ScheduleRecord,
    error_kind: io::ErrorKind,
) -> io::Result<()> {
    if previous.objective_id != next.objective_id {
        return Err(io::Error::new(
            error_kind,
            "schedule task cannot change objective lineage",
        ));
    }
    if matches!(
        previous.state,
        ScheduleState::Cancelled | ScheduleState::Completed
    ) && previous != next
    {
        return Err(io::Error::new(
            error_kind,
            "terminal schedule record is immutable",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn append_only_schedule_ledger_replays_latest_state_per_task() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let due = Utc::now() + Duration::hours(1);
        let scheduled = ScheduleRecord {
            contract: SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "task-1".into(),
            objective_id: "objective-1".into(),
            mode: ScheduleMode::Deferred,
            state: ScheduleState::Scheduled,
            not_before_utc: Some(due),
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: None,
        };
        let mut paused = scheduled.clone();
        paused.state = ScheduleState::Paused;
        paused.recorded_at_utc = Utc::now();
        paused.reason = Some("operator pause".into());

        ledger.append(&scheduled).expect("append schedule");
        ledger.append(&paused).expect("append pause");

        let raw = std::fs::read_to_string(dir.path().join("schedules.jsonl"))
            .expect("read append-only ledger");
        assert_eq!(raw.lines().count(), 2);
        let effective = ledger.effective().expect("replay schedules");
        assert_eq!(effective["task-1"], paused);
    }

    #[test]
    fn recurring_schedule_advances_past_the_completed_tick() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let completed_at = Utc::now();
        let due = completed_at - Duration::minutes(3);
        ledger
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: "task-1".into(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Recurring,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(due),
                interval_seconds: Some(60),
                recorded_at_utc: due,
                reason: None,
            })
            .expect("append recurring schedule");

        let advanced = ledger
            .advance_after_completion("task-1", completed_at)
            .expect("advance recurring schedule");

        assert_eq!(advanced.state, ScheduleState::Scheduled);
        assert_eq!(advanced.not_before_utc, Some(due + Duration::minutes(4)));
        assert_eq!(
            ledger.effective().unwrap()["task-1"].not_before_utc,
            advanced.not_before_utc
        );
    }

    #[test]
    fn one_shot_schedule_becomes_terminal_after_completion() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let completed_at = Utc::now();
        ledger
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: "task-1".into(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Once,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(completed_at - Duration::minutes(1)),
                interval_seconds: None,
                recorded_at_utc: completed_at - Duration::minutes(1),
                reason: None,
            })
            .expect("append one-shot schedule");

        let completed = ledger
            .advance_after_completion("task-1", completed_at)
            .expect("complete one-shot schedule");

        assert_eq!(completed.state, ScheduleState::Completed);
        assert_eq!(ledger.effective().unwrap()["task-1"], completed);
    }

    #[test]
    fn completion_transition_predicate_can_abort_stale_advance() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let completed_at = Utc::now();
        let scheduled = ScheduleRecord {
            contract: SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "task-1".into(),
            objective_id: "objective-1".into(),
            mode: ScheduleMode::Recurring,
            state: ScheduleState::Scheduled,
            not_before_utc: Some(completed_at - Duration::minutes(1)),
            interval_seconds: Some(60),
            recorded_at_utc: completed_at - Duration::minutes(1),
            reason: None,
        };
        ledger
            .append(&scheduled)
            .expect("append recurring schedule");

        let advanced = ledger
            .advance_after_completion_when("task-1", completed_at, |_| Ok(false))
            .expect("veto stale completion transition");

        assert!(advanced.is_none());
        assert_eq!(ledger.effective().unwrap()["task-1"], scheduled);
    }

    #[test]
    fn completion_transition_rejects_missing_schedule_before_queue_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let queue_appended = std::cell::Cell::new(false);

        let error = ledger
            .with_completion_transition("task-1", "objective-1", Utc::now(), || {
                queue_appended.set(true);
                Ok(())
            })
            .expect_err("missing canonical authority must fail closed");

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(!queue_appended.get());
    }

    #[test]
    fn timed_schedule_requires_a_due_time() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let error = ledger
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: "task-1".into(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Deferred,
                state: ScheduleState::Scheduled,
                not_before_utc: None,
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: None,
            })
            .expect_err("reject timed schedule without due time");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn recurring_schedule_requires_a_positive_interval() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let error = ledger
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: "task-1".into(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Recurring,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(Utc::now()),
                interval_seconds: Some(0),
                recorded_at_utc: Utc::now(),
                reason: None,
            })
            .expect_err("reject zero recurring interval");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn schedule_task_cannot_change_objective_lineage() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let mut record = ScheduleRecord {
            contract: SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "task-1".into(),
            objective_id: "objective-1".into(),
            mode: ScheduleMode::Immediate,
            state: ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: None,
        };
        ledger.append(&record).expect("append initial schedule");
        record.objective_id = "objective-2".into();
        record.recorded_at_utc = Utc::now();

        let error = ledger
            .append(&record)
            .expect_err("reject cross-objective rewrite");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            ledger.effective().unwrap()["task-1"].objective_id,
            "objective-1"
        );
    }

    #[test]
    fn replay_rejects_hand_appended_objective_lineage_rewrite() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("schedules.jsonl");
        let ledger = ScheduleLedger::new(&path);
        let first = ScheduleRecord {
            contract: SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "task-1".into(),
            objective_id: "objective-1".into(),
            mode: ScheduleMode::Immediate,
            state: ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: None,
        };
        let mut rewritten = first.clone();
        rewritten.objective_id = "objective-2".into();
        rewritten.recorded_at_utc = Utc::now();
        std::fs::write(
            &path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&rewritten).unwrap()
            ),
        )
        .unwrap();

        let error = ledger
            .effective()
            .expect_err("replay must reject cross-objective rewrite");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn terminal_schedule_states_cannot_return_to_scheduled() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        let scheduled = ScheduleRecord {
            contract: SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "terminal-task".into(),
            objective_id: "objective-terminal".into(),
            mode: ScheduleMode::Once,
            state: ScheduleState::Scheduled,
            not_before_utc: Some(Utc::now()),
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: None,
        };
        let mut cancelled = scheduled.clone();
        cancelled.state = ScheduleState::Cancelled;
        cancelled.reason = Some("operator cancelled".into());

        ledger.append(&scheduled).expect("append scheduled");
        ledger.append(&cancelled).expect("append cancelled");

        let error = ledger
            .append(&scheduled)
            .expect_err("cancelled schedule must remain terminal");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn terminal_schedule_records_reject_same_state_mutation() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));

        for state in [ScheduleState::Cancelled, ScheduleState::Completed] {
            let task_id = format!("terminal-{state:?}");
            let terminal = ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task_id.clone(),
                objective_id: "objective-terminal".into(),
                mode: ScheduleMode::Immediate,
                state,
                not_before_utc: None,
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: Some("authoritative terminal reason".into()),
            };
            ledger.append(&terminal).expect("append terminal schedule");
            let mut mutated = terminal.clone();
            mutated.reason = Some("rewritten terminal reason".into());

            let error = ledger
                .append(&mutated)
                .expect_err("terminal schedule record must be immutable");

            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert_eq!(ledger.effective().unwrap()[&task_id], terminal);
        }
    }
}
