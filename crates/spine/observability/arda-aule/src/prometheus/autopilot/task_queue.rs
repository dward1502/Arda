#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Task queue analyzer over `core/projects/tasks/queue.jsonl`.

use chrono::{DateTime, Duration, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::schedule::{ScheduleLedger, ScheduleMode, ScheduleRecord, ScheduleState};
use crate::prometheus::queue_authority::canonical_project_task_queue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRecord {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub queued_at_utc: Option<String>,
    #[serde(default)]
    pub completed_at_utc: Option<String>,
    #[serde(default)]
    pub started_at_utc: Option<String>,
    #[serde(default, flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskQueueMetrics {
    pub total: usize,
    pub by_status: BTreeMap<String, usize>,
    pub by_owner: BTreeMap<String, usize>,
    pub by_priority: BTreeMap<String, usize>,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub aging_oldest_pending_secs: Option<i64>,
    pub bottleneck_owner: Option<String>,
    pub recent_completions_1h: usize,
    pub recent_completions_24h: usize,
    pub recent_failures_24h: usize,
    pub completion_rate_24h: f64,
}

pub struct TaskQueueAnalyzer {
    queue_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveQueueExecutionAttempt {
    pub contract: String,
    pub executor: String,
    pub task_id: String,
    pub status: String,
    pub action_class: String,
    pub hades_projection_repair: bool,
    pub appended_at_utc: DateTime<Utc>,
    pub workbench_run_id: String,
    pub lease_expires_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedQueueClaim {
    pub task: QueueRecord,
    pub attempt: ActiveQueueExecutionAttempt,
}

#[derive(Debug, Clone)]
pub struct ActiveQueueExecutor {
    queue_path: PathBuf,
    active_projection_path: PathBuf,
    schedule_path: PathBuf,
}

impl TaskQueueAnalyzer {
    pub fn new(queue_path: impl AsRef<Path>) -> Self {
        Self {
            queue_path: queue_path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.queue_path
    }

    pub fn load(&self) -> std::io::Result<Vec<QueueRecord>> {
        let f = std::fs::File::open(&self.queue_path)?;
        read_queue_records(&f)
    }

    pub fn analyze(&self) -> TaskQueueMetrics {
        let records = self.load().unwrap_or_default();
        let effective_records = Self::effective_records(records);
        Self::summarize(&effective_records)
    }

    pub fn effective_records(records: Vec<QueueRecord>) -> Vec<QueueRecord> {
        let terminal_keys = records
            .iter()
            .filter(|record| {
                matches!(
                    record.status.as_deref().map(normalize_task_status),
                    Some("completed" | "failed" | "cancelled")
                )
            })
            .flat_map(|record| [record.id.clone(), Self::effective_record_key(record)])
            .collect::<BTreeSet<_>>();
        let mut seen_ids = BTreeSet::<String>::new();
        let mut effective = Vec::<(usize, QueueRecord)>::new();
        for (index, record) in records.into_iter().enumerate().rev() {
            let authorized_reopen = matches!(
                record.extra.get("contract").and_then(Value::as_str),
                Some(
                    "arda.workbench.queue_retry.v1"
                        | "arda.workbench.queue_continuation.v1"
                        | "arda.workbench.executable_continuation.v1"
                        | "arda.schedule.queue_activation.v1"
                )
            );
            let nonterminal = !matches!(
                record.status.as_deref().map(normalize_task_status),
                Some("completed" | "failed" | "cancelled")
            );
            let source_record_id = Self::effective_record_key(&record);
            let aliases = [record.id.clone(), source_record_id];
            if nonterminal
                && aliases.iter().any(|alias| terminal_keys.contains(alias))
                && !authorized_reopen
            {
                continue;
            }
            let already_superseded = aliases.iter().any(|alias| seen_ids.contains(alias));
            seen_ids.extend(aliases);
            if !already_superseded {
                effective.push((index, record));
            }
        }
        effective.sort_by_key(|(index, _)| *index);
        effective.into_iter().map(|(_, record)| record).collect()
    }

    pub fn effective_record_key(record: &QueueRecord) -> String {
        record
            .extra
            .get("source_record_id")
            .and_then(serde_json::Value::as_str)
            .filter(|source_record_id| !source_record_id.trim().is_empty())
            .unwrap_or(record.id.as_str())
            .to_string()
    }

    pub fn summarize(records: &[QueueRecord]) -> TaskQueueMetrics {
        let mut m = TaskQueueMetrics {
            total: records.len(),
            ..TaskQueueMetrics::default()
        };
        let now = Utc::now();
        let one_hour = Duration::hours(1);
        let one_day = Duration::hours(24);
        let mut oldest_pending: Option<DateTime<Utc>> = None;

        for r in records {
            let raw_status = r.status.as_deref().unwrap_or("unknown");
            let status = normalize_task_status(raw_status).to_string();
            *m.by_status.entry(status.clone()).or_insert(0) += 1;
            if let Some(o) = &r.owner {
                *m.by_owner.entry(o.clone()).or_insert(0) += 1;
            }
            if let Some(p) = &r.priority {
                *m.by_priority.entry(p.clone()).or_insert(0) += 1;
            }

            match status.as_str() {
                "pending" | "queued" => {
                    m.pending += 1;
                    if let Some(ts) = r.queued_at_utc.as_deref().and_then(parse_utc) {
                        oldest_pending = Some(match oldest_pending {
                            Some(prev) if prev <= ts => prev,
                            _ => ts,
                        });
                    }
                }
                "in_progress" | "running" | "active" => m.in_progress += 1,
                "completed" | "done" => {
                    m.completed += 1;
                    if let Some(ts) = r.completed_at_utc.as_deref().and_then(parse_utc) {
                        if now - ts <= one_hour {
                            m.recent_completions_1h += 1;
                        }
                        if now - ts <= one_day {
                            m.recent_completions_24h += 1;
                        }
                    }
                }
                "failed" | "error" => {
                    m.failed += 1;
                    if let Some(ts) = r.completed_at_utc.as_deref().and_then(parse_utc) {
                        if now - ts <= one_day {
                            m.recent_failures_24h += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        m.aging_oldest_pending_secs = oldest_pending.map(|ts| (now - ts).num_seconds());
        m.bottleneck_owner = m
            .by_owner
            .iter()
            .max_by_key(|(_, c)| **c)
            .map(|(k, _)| k.clone());
        let denom_24h = m.recent_completions_24h + m.recent_failures_24h;
        m.completion_rate_24h = if denom_24h == 0 {
            1.0
        } else {
            m.recent_completions_24h as f64 / denom_24h as f64
        };
        m
    }
}

impl ActiveQueueExecutor {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            queue_path: canonical_project_task_queue(root),
            active_projection_path: root.join("core/state/queue_active.json"),
            schedule_path: root.join("core/projects/tasks/schedules.jsonl"),
        }
    }

    pub fn with_paths(
        queue_path: impl AsRef<Path>,
        active_projection_path: impl AsRef<Path>,
    ) -> Self {
        let queue_path = queue_path.as_ref().to_path_buf();
        let schedule_path = queue_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("schedules.jsonl");
        Self {
            queue_path,
            active_projection_path: active_projection_path.as_ref().to_path_buf(),
            schedule_path,
        }
    }

    pub fn select_next_safe_local(&self) -> std::io::Result<Option<QueueRecord>> {
        let records = TaskQueueAnalyzer::new(&self.queue_path).load()?;
        let effective = TaskQueueAnalyzer::effective_records(records);
        let active_ids = self.load_active_projection_ids().unwrap_or_default();
        let schedules = ScheduleLedger::new(&self.schedule_path).effective()?;
        Ok(effective.into_iter().find(|record| {
            is_open_queue_status(record.status.as_deref())
                && (active_ids.is_empty() || active_ids.contains(&record.id))
                && authoritative_schedule_eligible(record, &schedules, Utc::now())
                && record
                    .extra
                    .get("meta")
                    .and_then(Value::as_object)
                    .and_then(|meta| meta.get("action_class"))
                    .and_then(Value::as_str)
                    == Some("l3_local_doc_fixture_patch")
                && record
                    .extra
                    .get("meta")
                    .and_then(Value::as_object)
                    .and_then(|meta| meta.get("mutation_risk"))
                    .and_then(Value::as_str)
                    == Some("safe-local")
        }))
    }

    pub fn select_next_approved(&self) -> std::io::Result<Option<QueueRecord>> {
        let records = TaskQueueAnalyzer::new(&self.queue_path).load()?;
        let effective = TaskQueueAnalyzer::effective_records(records);
        let schedules = ScheduleLedger::new(&self.schedule_path).effective()?;
        let now = Utc::now();
        Ok(effective
            .iter()
            .find(|record| {
                claimable_status(record)
                    && authoritative_schedule_eligible(record, &schedules, now)
                    && approved_workbench_metadata(record)
                    && mutation_lease_available(record, &effective, now)
            })
            .cloned())
    }

    pub fn reconcile_schedules(&self, now: DateTime<Utc>) -> std::io::Result<usize> {
        let ledger = ScheduleLedger::new(&self.schedule_path);
        let effective =
            TaskQueueAnalyzer::effective_records(TaskQueueAnalyzer::new(&self.queue_path).load()?);
        let schedules = ledger.effective()?;
        for task in &effective {
            let Some(schedule) = schedules.get(&task.id) else {
                continue;
            };
            if !schedule_matches_queue_objective(task, schedule) {
                continue;
            }
            let completed_at = task.completed_at_utc.as_deref().and_then(parse_utc);
            let schedule_precedes_completion = match schedule.mode {
                ScheduleMode::Immediate => completed_at.is_some(),
                ScheduleMode::Once | ScheduleMode::Deferred | ScheduleMode::Recurring => schedule
                    .not_before_utc
                    .zip(completed_at)
                    .is_some_and(|(due, completed)| due <= completed),
            };
            if task.status.as_deref().map(normalize_task_status) == Some("completed")
                && task.result.as_deref() == Some("completed")
                && schedule.state == ScheduleState::Scheduled
                && schedule_precedes_completion
            {
                let completed_at = completed_at.expect("checked above");
                ledger.advance_after_completion_when(
                    &task.id,
                    completed_at,
                    |current_schedule| {
                        if let Some(parent) = self.queue_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        let file = OpenOptions::new()
                            .create(true)
                            .read(true)
                            .write(true)
                            .truncate(false)
                            .open(&self.queue_path)?;
                        file.lock_shared()?;
                        let result = (|| {
                            let current =
                                TaskQueueAnalyzer::effective_records(read_queue_records(&file)?)
                                    .into_iter()
                                    .find(|record| record.id == task.id);
                            Ok(current.is_some_and(|current| {
                                current.status.as_deref().map(normalize_task_status)
                                    == Some("completed")
                                    && current.result.as_deref() == Some("completed")
                                    && current.completed_at_utc.as_deref().and_then(parse_utc)
                                        == Some(completed_at)
                                    && schedule_matches_queue_objective(&current, current_schedule)
                            }))
                        })();
                        let unlock = FileExt::unlock(&file);
                        match (result, unlock) {
                            (Ok(current), Ok(())) => Ok(current),
                            (Err(error), _) | (_, Err(error)) => Err(error),
                        }
                    },
                )?;
            }
        }

        ledger.with_effective(|schedules| {
            if let Some(parent) = self.queue_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let effective = TaskQueueAnalyzer::effective_records(read_queue_records(&file)?);
                let mut activated = 0;
                for task in effective {
                    let Some(schedule) = schedules.get(&task.id) else {
                        continue;
                    };
                    if !schedule_matches_queue_objective(&task, schedule) {
                        continue;
                    }
                    if task.status.as_deref().map(normalize_task_status) != Some("completed")
                        || task.result.as_deref() != Some("completed")
                        || schedule.state != ScheduleState::Scheduled
                        || !schedule
                            .not_before_utc
                            .is_some_and(|not_before| not_before <= now)
                    {
                        continue;
                    }
                    serde_json::to_writer(
                        &mut file,
                        &json!({
                            "contract": "arda.schedule.queue_activation.v1",
                            "id": task.id,
                            "source_record_id": task.id,
                            "title": task.title,
                            "owner": task.owner,
                            "priority": task.priority,
                            "status": "queued",
                            "result": Value::Null,
                            "scheduled_for_utc": schedule.not_before_utc,
                            "schedule_mode": schedule.mode,
                            "source_objective_packet_id": schedule.objective_id,
                            "recorded_at_utc": now.to_rfc3339(),
                            "meta": task.extra.get("meta").cloned().unwrap_or(Value::Null),
                        }),
                    )
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
                    writeln!(file)?;
                    activated += 1;
                }
                file.sync_data()?;
                Ok(activated)
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(count), Ok(())) => Ok(count),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    /// Atomically claim one approved task by appending its in-progress state
    /// while holding an exclusive lock on the canonical append-only ledger.
    pub fn claim_next_approved(&self) -> std::io::Result<Option<ApprovedQueueClaim>> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            if let Some(parent) = self.queue_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let records = read_queue_records(&file)?;
                let effective = TaskQueueAnalyzer::effective_records(records);
                let now = Utc::now();
                let Some(task) = effective.iter().find(|record| {
                    claimable_status(record)
                        && authoritative_schedule_eligible(record, schedules, now)
                        && approved_workbench_metadata(record)
                        && mutation_lease_available(record, &effective, now)
                }) else {
                    return Ok(None);
                };
                let task = task.clone();
                let attempt = execution_attempt(&task);
                append_attempt_to_writer(&mut file, &task, &attempt)?;
                file.sync_data()?;
                Ok(Some(ApprovedQueueClaim { task, attempt }))
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    #[cfg(test)]
    pub(super) fn claim_next_approved_reconciling_orphans(
        &self,
    ) -> std::io::Result<Option<ApprovedQueueClaim>> {
        self.claim_next_approved_reconciling_orphans_excluding(&BTreeSet::new())
    }

    /// Recover an orphan or atomically claim new work while excluding tasks
    /// whose project/worktree execution locks are currently held elsewhere.
    /// The caller holds the short root-scoped coordinator lock for selection,
    /// then retains the returned target locks through dispatch.
    #[cfg(test)]
    pub(super) fn claim_next_approved_reconciling_orphans_excluding(
        &self,
        excluded_task_ids: &BTreeSet<String>,
    ) -> std::io::Result<Option<ApprovedQueueClaim>> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            if let Some(parent) = self.queue_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let records = read_queue_records(&file)?;
                let effective = TaskQueueAnalyzer::effective_records(records);
                if let Some(task) = effective.iter().find(|record| {
                    !excluded_task_ids.contains(&record.id)
                        && record.status.as_deref().map(normalize_task_status)
                            == Some("in_progress")
                        && authoritative_schedule_eligible(record, schedules, Utc::now())
                        && approved_workbench_metadata(record)
                }) {
                    return Ok(Some(ApprovedQueueClaim {
                        task: task.clone(),
                        attempt: attempt_from_claimed_task(task)?,
                    }));
                }
                let now = Utc::now();
                let Some(task) = effective.iter().find(|record| {
                    !excluded_task_ids.contains(&record.id)
                        && claimable_status(record)
                        && authoritative_schedule_eligible(record, schedules, now)
                        && approved_workbench_metadata(record)
                        && mutation_lease_available(record, &effective, now)
                }) else {
                    return Ok(None);
                };
                let task = task.clone();
                let attempt = execution_attempt(&task);
                append_attempt_to_writer(&mut file, &task, &attempt)?;
                file.sync_data()?;
                Ok(Some(ApprovedQueueClaim { task, attempt }))
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    /// Select eligible work without mutating the queue so the caller can
    /// acquire the physical execution-target lock before appending a claim.
    pub(super) fn next_approved_reconciling_orphans_excluding(
        &self,
        excluded_task_ids: &BTreeSet<String>,
    ) -> std::io::Result<Option<QueueRecord>> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            let file = OpenOptions::new().read(true).open(&self.queue_path)?;
            file.lock_shared()?;
            let result = (|| {
                let effective = TaskQueueAnalyzer::effective_records(read_queue_records(&file)?);
                if let Some(task) = effective.iter().find(|record| {
                    !excluded_task_ids.contains(&record.id)
                        && record.status.as_deref().map(normalize_task_status)
                            == Some("in_progress")
                        && authoritative_schedule_eligible(record, schedules, Utc::now())
                        && approved_workbench_metadata(record)
                }) {
                    return Ok(Some(task.clone()));
                }
                let now = Utc::now();
                Ok(effective
                    .iter()
                    .find(|record| {
                        !excluded_task_ids.contains(&record.id)
                            && claimable_status(record)
                            && authoritative_schedule_eligible(record, schedules, now)
                            && approved_workbench_metadata(record)
                            && mutation_lease_available(record, &effective, now)
                    })
                    .cloned())
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    /// Atomically revalidate and claim one previously selected exact task.
    /// Returning `None` means the candidate changed before the target lock was
    /// acquired; no queue record is appended in that case.
    pub(super) fn claim_approved_candidate(
        &self,
        task_id: &str,
    ) -> std::io::Result<Option<ApprovedQueueClaim>> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            let mut file = OpenOptions::new()
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let effective = TaskQueueAnalyzer::effective_records(read_queue_records(&file)?);
                let Some(task) = effective.iter().find(|record| record.id == task_id) else {
                    return Ok(None);
                };
                let now = Utc::now();
                if task.status.as_deref().map(normalize_task_status) == Some("in_progress")
                    && authoritative_schedule_eligible(task, schedules, now)
                    && approved_workbench_metadata(task)
                {
                    return Ok(Some(ApprovedQueueClaim {
                        task: task.clone(),
                        attempt: attempt_from_claimed_task(task)?,
                    }));
                }
                if !claimable_status(task)
                    || !authoritative_schedule_eligible(task, schedules, now)
                    || !approved_workbench_metadata(task)
                    || !mutation_lease_available(task, &effective, now)
                {
                    return Ok(None);
                }
                let task = task.clone();
                let attempt = execution_attempt(&task);
                append_attempt_to_writer(&mut file, &task, &attempt)?;
                file.sync_data()?;
                Ok(Some(ApprovedQueueClaim { task, attempt }))
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    pub fn append_attempt(
        &self,
        task: &QueueRecord,
    ) -> std::io::Result<ActiveQueueExecutionAttempt> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let effective = TaskQueueAnalyzer::effective_records(read_queue_records(&file)?);
                let current = effective
                    .iter()
                    .find(|record| record.id == task.id)
                    .cloned()
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::NotFound, "task not found")
                    })?;
                let now = Utc::now();
                if !claimable_status(&current)
                    || !authoritative_schedule_eligible(&current, schedules, now)
                    || (!approved_workbench_metadata(&current) && !safe_local_metadata(&current))
                    || !mutation_lease_available(&current, &effective, now)
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "task is not eligible for an atomic governed claim",
                    ));
                }
                let attempt = execution_attempt(&current);
                append_attempt_to_writer(&mut file, &current, &attempt)?;
                file.sync_data()?;
                Ok(attempt)
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    /// Requeue one failed approved task while preserving its approval lineage.
    pub fn retry_failed(&self, task_id: &str) -> std::io::Result<QueueRecord> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let effective = TaskQueueAnalyzer::effective_records(read_queue_records(&file)?);
                let task = effective
                    .into_iter()
                    .find(|record| record.id == task_id)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::NotFound, "task not found")
                    })?;
                if task.status.as_deref().map(normalize_task_status) != Some("failed") {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "only failed tasks may be retried",
                    ));
                }
                if task.result.as_deref() == Some("cancelled")
                    || !approved_workbench_metadata(&task)
                    || !authoritative_schedule_eligible(&task, schedules, Utc::now())
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "cancelled, unapproved, or ineligible tasks require a new operator decision",
                    ));
                }
                let retry_sequence = task
                    .extra
                    .get("retry_sequence")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + 1;
                let value = json!({
                    "id": task.id,
                    "source_record_id": task.id,
                    "title": task.title,
                    "owner": task.owner,
                    "priority": task.priority,
                    "status": "queued",
                    "retry_sequence": retry_sequence,
                    "retried_at_utc": Utc::now().to_rfc3339(),
                    "contract": "arda.workbench.queue_retry.v1",
                    "executor": "arda_workbench.queue_executor",
                    "meta": task.extra.get("meta").cloned().unwrap_or(Value::Null),
                });
                writeln!(file, "{value}")?;
                file.sync_data()?;
                serde_json::from_value(value)
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            })();
            let unlock = FileExt::unlock(&file);
            match (result, unlock) {
                (Ok(value), Ok(())) => Ok(value),
                (Err(error), _) | (_, Err(error)) => Err(error),
            }
        })
    }

    pub fn append_terminal_completion(
        &self,
        task: &QueueRecord,
        result: &str,
    ) -> std::io::Result<()> {
        let completed_at = Utc::now();
        let terminal = json!({
            "id": task.id,
            "source_record_id": task.id,
            "title": task.title,
            "owner": task.owner,
            "priority": task.priority,
            "status": "completed",
            "result": result,
            "completed_at_utc": completed_at.to_rfc3339(),
            "contract": "arda.prometheus.active_queue_terminal_record.v1",
            "executor": "prometheus.active_queue_executor",
            "hades_projection_repair": true,
        });
        ScheduleLedger::new(&self.schedule_path).with_completion_transition(
            &task.id,
            queue_objective_id(task).unwrap_or_default(),
            completed_at,
            || append_jsonl_value(&self.queue_path, &terminal),
        )
    }

    pub fn append_workbench_terminal(
        &self,
        task: &QueueRecord,
        status: &str,
        result: &str,
        run_id: &str,
        receipt_digest: Option<&str>,
        detail: Option<&str>,
    ) -> std::io::Result<()> {
        self.append_workbench_terminal_with_continuation(
            task,
            status,
            result,
            run_id,
            receipt_digest,
            detail,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append_workbench_terminal_with_continuation(
        &self,
        task: &QueueRecord,
        status: &str,
        result: &str,
        run_id: &str,
        receipt_digest: Option<&str>,
        detail: Option<&str>,
        continuation_decision: Option<&str>,
    ) -> std::io::Result<()> {
        let completed_at = Utc::now();
        let terminal = json!({
            "id": task.id,
            "source_record_id": task.id,
            "title": task.title,
            "owner": task.owner,
            "priority": task.priority,
            "status": status,
            "result": result,
            "completed_at_utc": completed_at.to_rfc3339(),
            "contract": "arda.workbench.queue_terminal.v1",
            "executor": "arda_workbench.queue_executor",
            "workbench_run_id": run_id,
            "execution_receipt_digest": receipt_digest,
            "closure_receipt_digest": receipt_digest,
            "continuation_decision": continuation_decision,
            "source_objective_packet_id": task.extra.get("meta").and_then(Value::as_object).and_then(|meta| meta.get("source_objective_packet_id")),
            "detail": detail,
            "meta": task.extra.get("meta").cloned().unwrap_or(Value::Null),
        });
        let objective_id = queue_objective_id(task).unwrap_or_default();
        if normalize_task_status(status) == "completed" {
            ScheduleLedger::new(&self.schedule_path).with_completion_transition(
                &task.id,
                objective_id,
                completed_at,
                || append_jsonl_value(&self.queue_path, &terminal),
            )?;
        } else if result.eq_ignore_ascii_case("cancelled") {
            ScheduleLedger::new(&self.schedule_path).with_cancellation_transition(
                &task.id,
                objective_id,
                completed_at,
                detail,
                || append_jsonl_value(&self.queue_path, &terminal),
            )?;
        } else {
            ScheduleLedger::new(&self.schedule_path).with_active_authority(
                &task.id,
                objective_id,
                || append_jsonl_value(&self.queue_path, &terminal),
            )?;
        }
        Ok(())
    }

    pub fn append_workbench_continuation(
        &self,
        task: &QueueRecord,
        run_id: &str,
        completed_stage: &str,
        receipt_digest: Option<&str>,
        continuation_decision: &str,
    ) -> std::io::Result<()> {
        let objective_id = queue_objective_id(task).unwrap_or_default();
        let continuation = json!({
            "contract": "arda.workbench.queue_continuation.v1",
            "id": task.id,
            "source_record_id": task.id,
            "title": task.title,
            "owner": task.owner,
            "priority": task.priority,
            "status": "in_progress",
            "executor": "arda_workbench.queue_executor",
            "workbench_run_id": run_id,
            "completed_stage": completed_stage,
            "execution_receipt_digest": receipt_digest,
            "continuation_decision": continuation_decision,
            "source_objective_packet_id": task.extra.get("meta").and_then(Value::as_object).and_then(|meta| meta.get("source_objective_packet_id")),
            "recorded_at_utc": Utc::now().to_rfc3339(),
            "meta": task.extra.get("meta").cloned().unwrap_or(Value::Null),
        });
        ScheduleLedger::new(&self.schedule_path).with_active_authority(
            &task.id,
            objective_id,
            || append_continuation_if_current(&self.queue_path, &task.id, run_id, &continuation),
        )
    }

    fn load_active_projection_ids(&self) -> std::io::Result<BTreeSet<String>> {
        let projection = std::fs::read_to_string(&self.active_projection_path)?;
        let value: Value = serde_json::from_str(&projection)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        let mut ids = BTreeSet::new();
        collect_projection_ids(&value, &mut ids);
        Ok(ids)
    }
}

fn collect_projection_ids(value: &Value, ids: &mut BTreeSet<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_projection_ids(item, ids);
            }
        }
        Value::Object(map) => {
            if let Some(id) = map
                .get("id")
                .or_else(|| map.get("task_id"))
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
            {
                ids.insert(id.to_owned());
            }
            for key in ["active", "tasks", "items", "queue", "records"] {
                if let Some(child) = map.get(key) {
                    collect_projection_ids(child, ids);
                }
            }
        }
        _ => {}
    }
}

fn approved_workbench_metadata(record: &QueueRecord) -> bool {
    record
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .is_some_and(|meta| {
            let operator_authorized = meta.get("mutation_risk").and_then(Value::as_str)
                == Some("operator-approved")
                && nonempty_meta_string(meta, "approval_packet_id");
            let governance_authorized = meta.get("mutation_risk").and_then(Value::as_str)
                == Some("governance-authorized-reversible")
                && governance_authorization_id(meta).is_some();
            (operator_authorized || governance_authorized)
                && meta.get("execution_authority").and_then(Value::as_str) == Some("arda_workbench")
                && meta
                    .get("source_objective_packet_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.trim().is_empty())
        })
}

pub(super) fn governance_authorization_id(meta: &serde_json::Map<String, Value>) -> Option<&str> {
    if meta.get("execution_authority").and_then(Value::as_str) != Some("arda_workbench")
        || meta.get("action_class").and_then(Value::as_str) != Some("approved_autopilot_plan_step")
    {
        return None;
    }
    match meta.get("governance_gate").and_then(Value::as_str) {
        Some("safe_autonomous" | "triad_quorum_approved") => {}
        _ => return None,
    }
    let packet_id = meta
        .get("source_objective_packet_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let action_class = meta
        .get("governance_action_class")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())?;
    let authorization_id = meta
        .get("governance_authorization_id")
        .and_then(Value::as_str)?;
    (authorization_id == format!("governance:{packet_id}:{action_class}"))
        .then_some(authorization_id)
}

fn nonempty_meta_string(meta: &serde_json::Map<String, Value>, key: &str) -> bool {
    meta.get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn mutation_lease_available(
    candidate: &QueueRecord,
    effective: &[QueueRecord],
    now: DateTime<Utc>,
) -> bool {
    !effective.iter().any(|active| {
        active.id != candidate.id
            && holds_active_mutation_lease(active, now)
            && mutation_targets_conflict(candidate, active)
    })
}

fn holds_active_mutation_lease(record: &QueueRecord, now: DateTime<Utc>) -> bool {
    if record.status.as_deref().map(normalize_task_status) != Some("in_progress") {
        return false;
    }
    record
        .extra
        .get("lease_expires_at_utc")
        .and_then(Value::as_str)
        .and_then(parse_utc)
        .is_none_or(|deadline| deadline > now)
}

fn mutation_targets_conflict(left: &QueueRecord, right: &QueueRecord) -> bool {
    let left_project = mutation_target_field(left, "project_id");
    let right_project = mutation_target_field(right, "project_id");
    let project_conflict = match (left_project, right_project) {
        (Some(left), Some(right)) => left == right,
        // Legacy tasks without a durable project identity share one
        // conservative default lease rather than bypassing mutual exclusion.
        _ => true,
    };
    let worktree_conflict = mutation_target_field(left, "worktree_path")
        .zip(mutation_target_field(right, "worktree_path"))
        .is_some_and(|(left, right)| Path::new(left) == Path::new(right));
    project_conflict || worktree_conflict
}

fn mutation_target_field<'a>(record: &'a QueueRecord, key: &str) -> Option<&'a str> {
    record
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn claimable_status(record: &QueueRecord) -> bool {
    match record.status.as_deref().map(normalize_task_status) {
        Some("pending" | "queued") => true,
        Some("in_progress") => record
            .extra
            .get("lease_expires_at_utc")
            .and_then(Value::as_str)
            .and_then(parse_utc)
            .is_some_and(|deadline| deadline <= Utc::now()),
        _ => false,
    }
}

fn authoritative_schedule_eligible(
    record: &QueueRecord,
    schedules: &BTreeMap<String, ScheduleRecord>,
    now: DateTime<Utc>,
) -> bool {
    let Some(schedule) = schedules.get(&record.id) else {
        return false;
    };
    if !schedule_matches_queue_objective(record, schedule) {
        return false;
    }
    match schedule.state {
        ScheduleState::Paused | ScheduleState::Cancelled | ScheduleState::Completed => false,
        ScheduleState::Scheduled => match schedule.mode {
            ScheduleMode::Immediate => true,
            ScheduleMode::Once | ScheduleMode::Deferred | ScheduleMode::Recurring => schedule
                .not_before_utc
                .is_some_and(|not_before| not_before <= now),
        },
    }
}

fn schedule_matches_queue_objective(record: &QueueRecord, schedule: &ScheduleRecord) -> bool {
    queue_objective_id(record) == Some(schedule.objective_id.as_str())
}

pub(super) fn queue_objective_id(record: &QueueRecord) -> Option<&str> {
    record
        .extra
        .get("source_objective_packet_id")
        .and_then(Value::as_str)
        .or_else(|| {
            record
                .extra
                .get("meta")
                .and_then(Value::as_object)
                .and_then(|meta| meta.get("source_objective_packet_id"))
                .and_then(Value::as_str)
        })
}

fn execution_attempt(task: &QueueRecord) -> ActiveQueueExecutionAttempt {
    let action_class = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("action_class"))
        .and_then(Value::as_str)
        .unwrap_or("approved_autopilot_plan_step")
        .to_owned();
    ActiveQueueExecutionAttempt {
        contract: "arda.prometheus.active_queue_execution_attempt.v1".to_owned(),
        executor: "arda_workbench.queue_executor".to_owned(),
        task_id: task.id.clone(),
        status: "claimed".to_owned(),
        action_class,
        hades_projection_repair: false,
        appended_at_utc: Utc::now(),
        workbench_run_id: workbench_run_id(
            &task.id,
            task.extra
                .get("retry_sequence")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        lease_expires_at_utc: Utc::now() + Duration::minutes(20),
    }
}

fn attempt_from_claimed_task(task: &QueueRecord) -> std::io::Result<ActiveQueueExecutionAttempt> {
    let meta = task.extra.get("meta").and_then(Value::as_object);
    let meta_str = |key: &str| {
        meta.and_then(|meta| meta.get(key))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
    };
    // Prefer top-level attempt fields written by this executor, then fall back
    // to governed approval metadata so claims recorded by other governed
    // surfaces (e.g. Workbench approvals) reconcile without wedging the queue.
    let field = |key: &str| -> Option<&str> {
        task.extra
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| meta_str(key))
    };
    let parse_timestamp = |key: &str, value: &str| {
        parse_utc(value).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("claimed task `{}` has invalid `{key}`", task.id),
            )
        })
    };
    let appended_at = match task.started_at_utc.as_deref() {
        Some(value) if !value.trim().is_empty() => parse_timestamp("started_at_utc", value)?,
        // Legacy claims without an executor start stamp are treated as
        // discovered now; their lease is bounded below either way.
        _ => Utc::now(),
    };
    let lease_expires_at = field("lease_expires_at_utc")
        .map(|value| parse_timestamp("lease_expires_at_utc", value))
        .unwrap_or(Ok(appended_at + Duration::minutes(20)))?;
    Ok(ActiveQueueExecutionAttempt {
        contract: field("contract")
            .unwrap_or("arda.prometheus.active_queue_execution_attempt.v1")
            .to_owned(),
        executor: field("executor")
            .unwrap_or("arda_workbench.queue_executor")
            .to_owned(),
        task_id: task.id.clone(),
        status: "claimed".to_owned(),
        action_class: field("action_class")
            .unwrap_or("approved_autopilot_plan_step")
            .to_owned(),
        hades_projection_repair: false,
        appended_at_utc: appended_at,
        workbench_run_id: resolve_claimed_run_id(task)?,
        lease_expires_at_utc: lease_expires_at,
    })
}

/// Resolve the Workbench run id for a claimed task from its executor attempt
/// fields or governed approval metadata.
fn resolve_claimed_run_id(task: &QueueRecord) -> std::io::Result<String> {
    let meta = task.extra.get("meta").and_then(Value::as_object);
    let meta_str = |key: &str| {
        meta.and_then(|meta| meta.get(key))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
    };
    if let Some(run_id) = task
        .extra
        .get("workbench_run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| meta_str("workbench_run_id"))
    {
        return Ok(run_id.to_owned());
    }
    if meta_str("approval_packet_id").is_some() || meta_str("governance_authorization_id").is_some()
    {
        // Governed approval lineage without an explicit run id: use the same
        // canonical derivation as a fresh attempt so recovery targets the
        // same run namespace instead of inventing a parallel one.
        return Ok(workbench_run_id(&task.id, 0));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!(
            "claimed task `{}` omitted `workbench_run_id` and approval lineage",
            task.id
        ),
    ))
}

fn append_attempt_to_writer(
    writer: &mut impl Write,
    task: &QueueRecord,
    attempt: &ActiveQueueExecutionAttempt,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        json!({
            "id": task.id,
            "source_record_id": task.id,
            "title": task.title,
            "owner": task.owner,
            "priority": task.priority,
            "status": "in_progress",
            "started_at_utc": attempt.appended_at_utc.to_rfc3339(),
            "contract": attempt.contract,
            "executor": attempt.executor,
            "action_class": attempt.action_class,
            "workbench_run_id": attempt.workbench_run_id,
            "lease_expires_at_utc": attempt.lease_expires_at_utc.to_rfc3339(),
            "meta": task.extra.get("meta").cloned().unwrap_or(Value::Null),
        })
    )
}

fn append_continuation_if_current(
    queue_path: &Path,
    task_id: &str,
    run_id: &str,
    continuation: &Value,
) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(queue_path)?;
    file.lock_exclusive()?;
    let result = (|| {
        let effective = TaskQueueAnalyzer::effective_records(read_queue_records(&file)?);
        let current = effective
            .iter()
            .find(|record| record.id == task_id)
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "queue task not found")
            })?;
        if current.status.as_deref().map(normalize_task_status) != Some("in_progress")
            || current
                .extra
                .get("workbench_run_id")
                .and_then(Value::as_str)
                != Some(run_id)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "queue continuation requires the current claimed Workbench run",
            ));
        }
        writeln!(file, "{continuation}")?;
        file.sync_data()
    })();
    let unlock = FileExt::unlock(&file);
    result?;
    unlock
}

fn read_queue_records(file: &std::fs::File) -> std::io::Result<Vec<QueueRecord>> {
    let mut out = Vec::new();
    for (index, line) in BufReader::new(file.try_clone()?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str::<QueueRecord>(line.trim()).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid queue record on line {}: {error}", index + 1),
            )
        })?;
        out.push(record);
    }
    Ok(out)
}

pub(super) fn workbench_run_id(task_id: &str, retry_sequence: u64) -> String {
    let normalized = task_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if retry_sequence == 0 {
        format!("queue-{normalized}")
    } else {
        format!("queue-{normalized}-attempt-{}", retry_sequence + 1)
    }
}

fn append_jsonl_value(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    file.lock_exclusive()?;
    let result = writeln!(file, "{}", value).and_then(|()| file.sync_data());
    let unlock = FileExt::unlock(&file);
    match (result, unlock) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

fn safe_local_metadata(record: &QueueRecord) -> bool {
    let meta = record.extra.get("meta").and_then(Value::as_object);
    meta.and_then(|meta| meta.get("action_class"))
        .and_then(Value::as_str)
        == Some("l3_local_doc_fixture_patch")
        && meta
            .and_then(|meta| meta.get("mutation_risk"))
            .and_then(Value::as_str)
            == Some("safe-local")
}

fn is_open_queue_status(status: Option<&str>) -> bool {
    matches!(
        status.map(normalize_task_status),
        Some("pending" | "queued" | "in_progress")
    )
}

fn normalize_task_status(status: &str) -> &str {
    match status {
        "complete" | "done" => "completed",
        "active" | "running" => "in_progress",
        other => other,
    }
}

fn parse_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::autopilot::schedule::SCHEDULE_RECORD_CONTRACT;

    #[test]
    fn locked_queue_replay_rejects_malformed_records() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        std::fs::write(
            &queue_path,
            "{\"id\":\"task-a\",\"status\":\"queued\"}\n{malformed}\n",
        )
        .expect("write queue fixture");
        let file = OpenOptions::new()
            .read(true)
            .open(&queue_path)
            .expect("open queue");

        let error = read_queue_records(&file).expect_err("malformed queue must fail closed");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn analyzer_replay_rejects_malformed_records() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        std::fs::write(
            &queue_path,
            "{\"id\":\"valid\",\"status\":\"queued\"}\n{not-json}\n",
        )
        .expect("write malformed fixture");

        let error = TaskQueueAnalyzer::new(&queue_path)
            .load()
            .expect_err("canonical analyzer replay must fail closed");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("line 2"));
    }

    #[test]
    fn queue_append_waits_for_the_canonical_file_lock() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let locked = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&queue_path)
            .expect("open queue");
        locked.lock_exclusive().expect("lock queue");
        let (sender, receiver) = std::sync::mpsc::channel();
        let append_path = queue_path.clone();
        let worker = std::thread::spawn(move || {
            sender
                .send(append_jsonl_value(
                    &append_path,
                    &json!({"id": "task-a", "status": "completed"}),
                ))
                .expect("send append result");
        });

        assert!(receiver
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err());
        FileExt::unlock(&locked).expect("unlock queue");
        receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("append completed after unlock")
            .expect("append queue record");
        worker.join().expect("join append worker");
    }

    #[test]
    fn schedule_objective_must_match_queue_objective_lineage() {
        let task = QueueRecord {
            id: "lineage-task".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("lineage-task")
        };
        let schedules = BTreeMap::from([(
            task.id.clone(),
            ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "different-objective".into(),
                mode: ScheduleMode::Immediate,
                state: ScheduleState::Scheduled,
                not_before_utc: None,
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: None,
            },
        )]);

        assert!(!authoritative_schedule_eligible(
            &task,
            &schedules,
            Utc::now()
        ));
    }

    #[test]
    fn schedule_reconciliation_ignores_mismatched_objective_lineage() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let completed_at = Utc::now() - Duration::minutes(1);
        let task = QueueRecord {
            id: "mismatched-recurrence".into(),
            status: Some("completed".into()),
            result: Some("completed".into()),
            completed_at_utc: Some(completed_at.to_rfc3339()),
            extra: approved_workbench_extra(),
            ..blank("mismatched-recurrence")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let due = completed_at - Duration::minutes(1);
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        ledger
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "different-objective".into(),
                mode: ScheduleMode::Recurring,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(due),
                interval_seconds: Some(60),
                recorded_at_utc: due,
                reason: None,
            })
            .expect("append schedule");

        let activated = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .reconcile_schedules(Utc::now())
            .expect("reconcile schedules");

        assert_eq!(activated, 0);
        assert_eq!(
            ledger.effective().unwrap()[&task.id].not_before_utc,
            Some(due)
        );
    }

    #[test]
    fn append_attempt_rejects_future_schedule_authority() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "future-task".into(),
            status: Some("queued".into()),
            extra: l3_safe_local_extra(),
            ..blank("future-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        ScheduleLedger::new(dir.path().join("schedules.jsonl"))
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "objective-future".into(),
                mode: ScheduleMode::Deferred,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(Utc::now() + Duration::hours(1)),
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: None,
            })
            .expect("append future schedule");

        let error = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .append_attempt(&task)
            .expect_err("future task must not be claimed");
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn governed_retry_rejects_paused_schedule_authority() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "paused-retry".into(),
            status: Some("failed".into()),
            result: Some("verification_failed".into()),
            extra: approved_workbench_extra(),
            ..blank("paused-retry")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Paused,
            None,
        );

        let error = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .retry_failed(&task.id)
            .expect_err("paused schedule must block retry");

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    }
    #[test]
    fn summarize_buckets_records() {
        let recs = vec![
            QueueRecord {
                id: "a".into(),
                status: Some("pending".into()),
                owner: Some("ceo".into()),
                priority: Some("high".into()),
                queued_at_utc: Some("2026-01-01T00:00:00Z".into()),
                ..blank("a")
            },
            QueueRecord {
                id: "b".into(),
                status: Some("complete".into()),
                owner: Some("ceo".into()),
                ..blank("b")
            },
            QueueRecord {
                id: "c".into(),
                status: Some("failed".into()),
                ..blank("c")
            },
        ];
        let m = TaskQueueAnalyzer::summarize(&recs);
        assert_eq!(m.total, 3);
        assert_eq!(m.pending, 1);
        assert_eq!(m.completed, 1);
        assert_eq!(m.by_status.get("completed"), Some(&1));
        assert!(!m.by_status.contains_key("complete"));
        assert_eq!(m.failed, 1);
        assert_eq!(m.bottleneck_owner.as_deref(), Some("ceo"));
        assert!(m.aging_oldest_pending_secs.is_some());
    }

    #[test]
    fn effective_records_keep_latest_status_per_id() {
        let recs = vec![
            QueueRecord {
                id: "task-a".into(),
                status: Some("pending".into()),
                ..blank("task-a")
            },
            QueueRecord {
                id: "task-b".into(),
                status: Some("queued".into()),
                ..blank("task-b")
            },
            QueueRecord {
                id: "task-a".into(),
                status: Some("completed".into()),
                result: Some("completed".into()),
                ..blank("task-a")
            },
        ];

        let effective = TaskQueueAnalyzer::effective_records(recs);
        let m = TaskQueueAnalyzer::summarize(&effective);

        assert_eq!(m.total, 2);
        assert_eq!(m.pending, 1);
        assert_eq!(m.completed, 1);
        assert_eq!(m.by_status.get("queued"), Some(&1));
    }

    #[test]
    fn effective_records_keep_latest_status_per_source_record_id() {
        let recs = vec![
            QueueRecord {
                id: "raw-1".into(),
                status: Some("pending".into()),
                extra: source_record_extra("shared-source"),
                ..blank("raw-1")
            },
            QueueRecord {
                id: "raw-2".into(),
                status: Some("completed".into()),
                result: Some("completed".into()),
                extra: source_record_extra("shared-source"),
                ..blank("raw-2")
            },
        ];

        let effective = TaskQueueAnalyzer::effective_records(recs);
        let m = TaskQueueAnalyzer::summarize(&effective);

        assert_eq!(m.total, 1);
        assert_eq!(m.pending, 0);
        assert_eq!(m.completed, 1);
        assert_eq!(effective[0].id, "raw-2");
    }

    #[test]
    fn effective_records_fold_duplicate_id_after_source_key_correction() {
        let recs = vec![
            QueueRecord {
                id: "objective".into(),
                status: Some("queued".into()),
                ..blank("objective")
            },
            QueueRecord {
                id: "revised".into(),
                status: Some("queued".into()),
                extra: source_record_extra("objective"),
                ..blank("revised")
            },
            QueueRecord {
                id: "revised".into(),
                status: Some("queued".into()),
                extra: source_record_extra("revised"),
                ..blank("revised")
            },
        ];

        let effective = TaskQueueAnalyzer::effective_records(recs);

        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].id, "revised");
        assert_eq!(effective[0].status.as_deref(), Some("queued"));
    }

    #[test]
    fn effective_records_do_not_reopen_terminal_tasks_without_authorized_continuation() {
        let records = vec![
            QueueRecord {
                id: "terminal".into(),
                status: Some("completed".into()),
                result: Some("completed".into()),
                ..blank("terminal")
            },
            QueueRecord {
                id: "terminal".into(),
                status: Some("queued".into()),
                ..blank("terminal")
            },
        ];

        let effective = TaskQueueAnalyzer::effective_records(records);

        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].status.as_deref(), Some("completed"));
    }

    #[test]
    fn effective_records_do_not_reopen_terminal_tasks_through_source_aliases() {
        let records = vec![
            QueueRecord {
                id: "terminal".into(),
                status: Some("completed".into()),
                result: Some("completed".into()),
                ..blank("terminal")
            },
            QueueRecord {
                id: "forged-reopen".into(),
                status: Some("queued".into()),
                extra: source_record_extra("terminal"),
                ..blank("forged-reopen")
            },
        ];

        let effective = TaskQueueAnalyzer::effective_records(records);

        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].id, "terminal");
        assert_eq!(effective[0].status.as_deref(), Some("completed"));
    }

    #[test]
    fn analyze_uses_effective_queue_records() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let records = [
            QueueRecord {
                id: "superseded".into(),
                status: Some("pending".into()),
                ..blank("superseded")
            },
            QueueRecord {
                id: "live".into(),
                status: Some("queued".into()),
                ..blank("live")
            },
            QueueRecord {
                id: "superseded".into(),
                status: Some("completed".into()),
                result: Some("completed".into()),
                ..blank("superseded")
            },
        ];
        let content = records
            .iter()
            .map(|record| serde_json::to_string(record).expect("serialize queue record"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&queue_path, format!("{content}\n")).expect("write queue fixture");

        let m = TaskQueueAnalyzer::new(&queue_path).analyze();

        assert_eq!(m.total, 2);
        assert_eq!(m.pending, 1);
        assert_eq!(m.completed, 1);
    }

    #[test]
    fn active_queue_executor_selects_safe_local_task_from_hades_projection() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let safe_task = QueueRecord {
            id: "safe-local-task".into(),
            status: Some("pending".into()),
            extra: l3_safe_local_extra(),
            ..blank("safe-local-task")
        };
        let human_task = QueueRecord {
            id: "human-required-task".into(),
            status: Some("pending".into()),
            extra: l3_human_required_extra(),
            ..blank("human-required-task")
        };
        let content = [safe_task.clone(), human_task]
            .iter()
            .map(|record| serde_json::to_string(record).expect("serialize queue record"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&queue_path, format!("{content}\n")).expect("write queue fixture");
        std::fs::write(
            &active_path,
            serde_json::to_string(&json!({
                "contract": "arda.hades.queue_active_projection.v1",
                "active": [{"id": "safe-local-task"}, {"id": "human-required-task"}]
            }))
            .expect("serialize active projection"),
        )
        .expect("write active projection");
        append_test_schedule(
            &queue_path,
            &safe_task.id,
            "objective-l3",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let selected = executor
            .select_next_safe_local()
            .expect("select next")
            .expect("safe-local task");

        assert_eq!(selected.id, safe_task.id);
    }

    #[test]
    fn safe_local_selection_obeys_paused_schedule_authority() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "paused-safe-local".into(),
            status: Some("pending".into()),
            extra: l3_safe_local_extra(),
            ..blank("paused-safe-local")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(
            &active_path,
            "{\"active\":[{\"id\":\"paused-safe-local\"}]}",
        )
        .unwrap();
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-l3",
            ScheduleMode::Immediate,
            ScheduleState::Paused,
            None,
        );

        let selected = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .select_next_safe_local()
            .expect("select safely");

        assert!(selected.is_none());
    }

    #[test]
    fn approved_selection_uses_canonical_queue_when_generated_projection_is_stale() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let mut extra = serde_json::Map::new();
        extra.insert(
            "meta".into(),
            json!({
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "operator-approved",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "objective-1",
                "approval_packet_id": "approval-1"
            }),
        );
        let approved = QueueRecord {
            id: "approved-task".into(),
            status: Some("pending".into()),
            extra,
            ..blank("approved-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&approved).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"stale-task\"}]}\n").unwrap();

        append_test_schedule(
            &queue_path,
            &approved.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let selected = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .select_next_approved()
            .unwrap()
            .expect("approved task");

        assert_eq!(selected.id, "approved-task");
    }

    #[test]
    fn governed_selection_accepts_binding_reversible_authority() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let governed = QueueRecord {
            id: "governed-task".into(),
            status: Some("pending".into()),
            extra: serde_json::Map::from_iter([(
                "meta".into(),
                json!({
                    "action_class": "approved_autopilot_plan_step",
                    "mutation_risk": "governance-authorized-reversible",
                    "execution_authority": "arda_workbench",
                    "source_objective_packet_id": "objective-governed",
                    "governance_action_class": "local_refactors",
                    "governance_gate": "safe_autonomous",
                    "governance_authorization_id": "governance:objective-governed:local_refactors"
                }),
            )]),
            ..blank("governed-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&governed).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        append_test_schedule(
            &queue_path,
            &governed.id,
            "objective-governed",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let selected = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .select_next_approved()
            .unwrap()
            .expect("binding governance authority should be executable");

        assert_eq!(selected.id, "governed-task");
    }

    #[test]
    fn approved_selection_skips_work_deferred_until_a_future_tick() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let future = QueueRecord {
            id: "future-task".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("future-task")
        };
        let due = QueueRecord {
            id: "due-task".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("due-task")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&future).unwrap(),
                serde_json::to_string(&due).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        append_test_schedule(
            &queue_path,
            &future.id,
            "objective-1",
            ScheduleMode::Deferred,
            ScheduleState::Scheduled,
            Some(Utc::now() + Duration::hours(1)),
        );
        append_test_schedule(
            &queue_path,
            &due.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let selected = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .select_next_approved()
            .unwrap()
            .expect("due task");

        assert_eq!(selected.id, "due-task");
    }

    #[test]
    fn approved_selection_skips_paused_scheduled_work() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let mut extra = approved_workbench_extra();
        extra
            .get_mut("meta")
            .and_then(Value::as_object_mut)
            .expect("approved metadata")
            .insert("schedule".into(), json!({"state": "paused"}));
        let task = QueueRecord {
            id: "paused-task".into(),
            status: Some("queued".into()),
            extra,
            ..blank("paused-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();

        assert!(ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .select_next_approved()
            .unwrap()
            .is_none());
    }

    #[test]
    fn approved_selection_obeys_canonical_schedule_ledger() {
        use crate::prometheus::autopilot::schedule::{
            ScheduleLedger, ScheduleMode, ScheduleRecord, ScheduleState, SCHEDULE_RECORD_CONTRACT,
        };

        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "paused-by-ledger".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("paused-by-ledger")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        ScheduleLedger::new(dir.path().join("schedules.jsonl"))
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Deferred,
                state: ScheduleState::Paused,
                not_before_utc: Some(Utc::now() - Duration::minutes(1)),
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: Some("operator pause".into()),
            })
            .expect("append schedule");

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        assert!(executor.select_next_approved().unwrap().is_none());
        assert!(executor.claim_next_approved().unwrap().is_none());
        assert!(executor
            .claim_next_approved_reconciling_orphans()
            .unwrap()
            .is_none());
    }

    #[test]
    fn canonical_schedule_states_gate_selection_and_claims() {
        let now = Utc::now();
        let cases = [
            (
                "immediate",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
                None,
                true,
            ),
            (
                "due-once",
                ScheduleMode::Once,
                ScheduleState::Scheduled,
                Some(now - Duration::minutes(1)),
                None,
                true,
            ),
            (
                "due-deferred",
                ScheduleMode::Deferred,
                ScheduleState::Scheduled,
                Some(now - Duration::minutes(1)),
                None,
                true,
            ),
            (
                "future-recurring",
                ScheduleMode::Recurring,
                ScheduleState::Scheduled,
                Some(now + Duration::minutes(1)),
                Some(60),
                false,
            ),
            (
                "cancelled",
                ScheduleMode::Immediate,
                ScheduleState::Cancelled,
                None,
                None,
                false,
            ),
            (
                "completed",
                ScheduleMode::Immediate,
                ScheduleState::Completed,
                None,
                None,
                false,
            ),
        ];

        for (name, mode, state, not_before_utc, interval_seconds, expected) in cases {
            let dir = tempfile::tempdir().expect("create tempdir");
            let queue_path = dir.path().join("queue.jsonl");
            let active_path = dir.path().join("queue_active.json");
            let task_id = format!("schedule-state-{name}");
            let task = QueueRecord {
                id: task_id.clone(),
                status: Some("queued".into()),
                extra: approved_workbench_extra(),
                ..blank(&task_id)
            };
            std::fs::write(
                &queue_path,
                format!("{}\n", serde_json::to_string(&task).unwrap()),
            )
            .unwrap();
            std::fs::write(&active_path, "{\"active\":[]}").unwrap();
            ScheduleLedger::new(dir.path().join("schedules.jsonl"))
                .append(&ScheduleRecord {
                    contract: SCHEDULE_RECORD_CONTRACT.into(),
                    task_id,
                    objective_id: "objective-1".into(),
                    mode,
                    state,
                    not_before_utc,
                    interval_seconds,
                    recorded_at_utc: now,
                    reason: Some(format!("schedule state fixture: {name}")),
                })
                .unwrap();

            let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
            assert_eq!(
                executor.select_next_approved().unwrap().is_some(),
                expected,
                "selection eligibility mismatch for {name}"
            );
            assert_eq!(
                executor.claim_next_approved().unwrap().is_some(),
                expected,
                "claim eligibility mismatch for {name}"
            );
        }
    }

    #[test]
    fn approved_claim_requires_canonical_schedule_authority() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "unscheduled-task".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("unscheduled-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        assert!(executor.select_next_approved().unwrap().is_none());
        assert!(executor.claim_next_approved().unwrap().is_none());
        assert!(executor
            .claim_next_approved_reconciling_orphans()
            .unwrap()
            .is_none());
    }

    #[test]
    fn approved_claim_is_append_only_and_exactly_once() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "approved-task".into(),
            title: Some("Execute bounded approved task".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("approved-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"approved-task\"}]}\n").unwrap();
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let before = std::fs::read(&queue_path).unwrap();
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let claim = executor
            .claim_next_approved()
            .unwrap()
            .expect("approved claim");
        assert_eq!(claim.attempt.status, "claimed");
        assert_eq!(claim.attempt.workbench_run_id, "queue-approved-task");
        assert!(claim.attempt.lease_expires_at_utc > claim.attempt.appended_at_utc);
        assert!(executor.claim_next_approved().unwrap().is_none());

        let after = std::fs::read(&queue_path).unwrap();
        assert_eq!(&after[..before.len()], before.as_slice());
        let effective = TaskQueueAnalyzer::effective_records(
            TaskQueueAnalyzer::new(&queue_path).load().unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("in_progress"));
    }

    #[test]
    fn approved_claim_blocks_a_second_active_mutation_for_the_same_project() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "project-a-first".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/project-a"),
            ..blank("project-a-first")
        };
        let second = QueueRecord {
            id: "project-a-second".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/project-a-next"),
            ..blank("project-a-second")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&first, &second] {
            append_test_schedule(
                &queue_path,
                &task.id,
                "objective-1",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
            );
        }
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        assert_eq!(
            executor
                .claim_next_approved()
                .unwrap()
                .expect("first project claim")
                .task
                .id,
            first.id
        );
        assert!(executor.claim_next_approved().unwrap().is_none());
        assert_eq!(
            executor.append_attempt(&second).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn approved_claim_blocks_a_second_active_mutation_for_the_same_worktree() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "worktree-first".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/shared/worktree"),
            ..blank("worktree-first")
        };
        let second = QueueRecord {
            id: "worktree-second".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/shared/worktree"),
            ..blank("worktree-second")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&first, &second] {
            append_test_schedule(
                &queue_path,
                &task.id,
                "objective-1",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
            );
        }
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        executor
            .claim_next_approved()
            .unwrap()
            .expect("first worktree claim");
        assert!(executor.claim_next_approved().unwrap().is_none());
    }

    #[test]
    fn approved_claim_allows_distinct_projects_on_distinct_worktrees() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "parallel-first".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/project-a"),
            ..blank("parallel-first")
        };
        let second = QueueRecord {
            id: "parallel-second".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/project-b"),
            ..blank("parallel-second")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&first, &second] {
            append_test_schedule(
                &queue_path,
                &task.id,
                "objective-1",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
            );
        }
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let first_claim = executor
            .claim_next_approved()
            .unwrap()
            .expect("first project claim");
        let second_claim = executor
            .claim_next_approved()
            .unwrap()
            .expect("independent project claim");
        assert_eq!(first_claim.task.id, first.id);
        assert_eq!(second_claim.task.id, second.id);
    }

    #[test]
    fn orphan_reconciliation_can_exclude_a_busy_target_and_claim_distinct_work() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "busy-orphan".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/project-a"),
            ..blank("busy-orphan")
        };
        let second = QueueRecord {
            id: "available-project".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/project-b"),
            ..blank("available-project")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&first, &second] {
            append_test_schedule(
                &queue_path,
                &task.id,
                "objective-1",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
            );
        }
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let first_claim = executor
            .claim_next_approved()
            .unwrap()
            .expect("first project claim");
        let excluded = BTreeSet::from([first_claim.task.id]);

        let next = executor
            .claim_next_approved_reconciling_orphans_excluding(&excluded)
            .unwrap()
            .expect("distinct project claim");

        assert_eq!(next.task.id, second.id);
        assert_eq!(next.attempt.task_id, second.id);
    }

    #[test]
    fn concurrent_approved_claims_serialize_the_same_project_lease() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "concurrent-first".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/first"),
            ..blank("concurrent-first")
        };
        let second = QueueRecord {
            id: "concurrent-second".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/second"),
            ..blank("concurrent-second")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&first, &second] {
            append_test_schedule(
                &queue_path,
                &task.id,
                "objective-1",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
            );
        }
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let queue_path = queue_path.clone();
            let active_path = active_path.clone();
            let barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                let executor = ActiveQueueExecutor::with_paths(queue_path, active_path);
                barrier.wait();
                executor.claim_next_approved().unwrap()
            }));
        }
        barrier.wait();
        let claims = workers
            .into_iter()
            .map(|worker| worker.join().expect("claim worker"))
            .collect::<Vec<_>>();

        assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
        assert_eq!(claims.iter().filter(|claim| claim.is_none()).count(), 1);
    }

    #[test]
    fn approved_claim_rejects_missing_approval_lineage() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "unscoped-task".into(),
            status: Some("queued".into()),
            extra: l3_human_required_extra(),
            ..blank("unscoped-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"unscoped-task\"}]}\n").unwrap();

        assert!(ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .claim_next_approved()
            .unwrap()
            .is_none());
    }

    #[test]
    fn approved_claim_recovers_an_expired_lease() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let meta = approved_workbench_extra().remove("meta").unwrap();
        let queued = json!({
            "id": "expired-task",
            "status": "queued",
            "meta": meta.clone()
        });
        let claimed = json!({
            "id": "expired-task",
            "status": "in_progress",
            "lease_expires_at_utc": (Utc::now() - Duration::minutes(1)).to_rfc3339(),
            "meta": meta
        });
        std::fs::write(&queue_path, format!("{queued}\n{claimed}\n")).unwrap();
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"expired-task\"}]}\n").unwrap();

        append_test_schedule(
            &queue_path,
            "expired-task",
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let claim = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .claim_next_approved()
            .unwrap()
            .expect("expired lease is recoverable");
        assert_eq!(claim.task.id, "expired-task");
        assert_eq!(claim.attempt.workbench_run_id, "queue-expired-task");
    }

    #[test]
    fn governed_workbench_claim_without_executor_stamps_reconciles() {
        // Regression: a Workbench approval flow appended an `in_progress`
        // record whose lineage lives only in `meta`. Orphan reconciliation
        // must recover it instead of failing the whole executor.
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let claimed = json!({
            "id": "digital-organism-s7-living-mesh-proof",
            "title": "Run and obtain acceptance for the living-mesh proof",
            "owner": "arandur",
            "priority": "high",
            "status": "in_progress",
            "result": "accepted_for_execution",
            "updated_at_utc": "2026-08-23T12:34:54.413630Z",
            "notes": "Explicit operator execution instruction admitted through governed Workbench approval; one existing canonical run only.",
            "meta": {
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "operator-approved",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "objective_packet:digital-organism-s7-living-mesh-proof:digital-organism-s7-living-mesh-proof",
                "approval_packet_id": "approval-stage7-operator-session",
                "workbench_run_id": "stage7-living-mesh-20260823",
                "operator_authorization_receipt": "operator-message:2026-08-23:execute-stage-7"
            }
        });
        std::fs::write(&queue_path, format!("{claimed}\n")).unwrap();
        std::fs::write(
            &active_path,
            "{\"active\":[{\"id\":\"digital-organism-s7-living-mesh-proof\"}]}\n",
        )
        .unwrap();
        append_test_schedule(
            &queue_path,
            "digital-organism-s7-living-mesh-proof",
            "objective_packet:digital-organism-s7-living-mesh-proof:digital-organism-s7-living-mesh-proof",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let before = std::fs::read(&queue_path).unwrap();

        let claim = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .claim_next_approved_reconciling_orphans()
            .unwrap()
            .expect("governed meta claim reconciles");

        assert_eq!(claim.task.id, "digital-organism-s7-living-mesh-proof");
        assert_eq!(
            claim.attempt.workbench_run_id,
            "stage7-living-mesh-20260823"
        );
        assert_eq!(claim.attempt.action_class, "approved_autopilot_plan_step");
        assert!(claim.attempt.lease_expires_at_utc > claim.attempt.appended_at_utc);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
    }

    #[test]
    fn approved_claim_recovers_before_lease_expiry_without_appending_a_second_attempt() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let meta = approved_workbench_extra().remove("meta").unwrap();
        let appended_at = Utc::now() - Duration::seconds(5);
        let lease_expires_at = Utc::now() + Duration::minutes(15);
        let queued = json!({
            "id": "orphaned-task",
            "status": "queued",
            "meta": meta.clone()
        });
        let claimed = json!({
            "id": "orphaned-task",
            "source_record_id": "orphaned-task",
            "status": "in_progress",
            "contract": "arda.prometheus.active_queue_execution_attempt.v1",
            "executor": "arda_workbench.queue_executor",
            "action_class": "approved_autopilot_plan_step",
            "hades_projection_repair": false,
            "started_at_utc": appended_at.to_rfc3339(),
            "workbench_run_id": "queue-orphaned-task",
            "lease_expires_at_utc": lease_expires_at.to_rfc3339(),
            "meta": meta
        });
        std::fs::write(&queue_path, format!("{queued}\n{claimed}\n")).unwrap();
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"orphaned-task\"}]}\n").unwrap();
        append_test_schedule(
            &queue_path,
            "orphaned-task",
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let before = std::fs::read(&queue_path).unwrap();

        let claim = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .claim_next_approved_reconciling_orphans()
            .unwrap()
            .expect("unexpired orphan is immediately recoverable");

        assert_eq!(claim.task.id, "orphaned-task");
        assert_eq!(claim.attempt.workbench_run_id, "queue-orphaned-task");
        assert_eq!(claim.attempt.appended_at_utc, appended_at);
        assert_eq!(claim.attempt.lease_expires_at_utc, lease_expires_at);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
    }

    #[test]
    fn governed_retry_preserves_lineage_and_allocates_distinct_run_id() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let meta = approved_workbench_extra().remove("meta").unwrap();
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                json!({
                    "id": "retry-task",
                    "status": "failed",
                    "result": "dispatch_failed",
                    "meta": meta
                })
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"retry-task\"}]}\n").unwrap();
        append_test_schedule(
            &queue_path,
            "retry-task",
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let retried = executor
            .retry_failed("retry-task")
            .expect("retry failed task");
        assert_eq!(retried.status.as_deref(), Some("queued"));
        assert_eq!(retried.extra["retry_sequence"], 1);
        let claim = executor
            .claim_next_approved()
            .expect("claim retry")
            .expect("retried task");
        assert_eq!(claim.attempt.workbench_run_id, "queue-retry-task-attempt-2");
        assert!(claim.task.extra["meta"]["approval_packet_id"].is_string());

        let recovered = executor
            .claim_next_approved_reconciling_orphans()
            .expect("recover claimed retry")
            .expect("claimed retry remains recoverable");
        assert_eq!(
            recovered.attempt.workbench_run_id,
            "queue-retry-task-attempt-2"
        );
    }

    #[test]
    fn governed_retry_rejects_cancelled_task() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                json!({
                    "id": "cancelled-task",
                    "status": "failed",
                    "result": "cancelled",
                    "meta": approved_workbench_extra().remove("meta").unwrap()
                })
            ),
        )
        .unwrap();
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        assert_eq!(
            executor.retry_failed("cancelled-task").unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn active_queue_executor_appends_same_id_attempt_and_terminal_records() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "safe-local-task".into(),
            title: Some("Patch docs fixture".into()),
            owner: Some("prometheus".into()),
            priority: Some("high".into()),
            status: Some("pending".into()),
            extra: l3_safe_local_extra(),
            ..blank("safe-local-task")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                serde_json::to_string(&task).expect("serialize queue record")
            ),
        )
        .expect("write queue fixture");
        std::fs::write(
            &active_path,
            "{\"active\":[{\"id\":\"safe-local-task\"}]}\n",
        )
        .expect("write active projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-l3",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let selected = executor
            .select_next_safe_local()
            .expect("select next")
            .expect("safe-local task");
        let attempt = executor.append_attempt(&selected).expect("append attempt");
        executor
            .append_terminal_completion(&selected, "completed")
            .expect("append terminal");

        assert_eq!(
            attempt.contract,
            "arda.prometheus.active_queue_execution_attempt.v1"
        );
        let records = TaskQueueAnalyzer::new(&queue_path)
            .load()
            .expect("reload queue");
        let effective = TaskQueueAnalyzer::effective_records(records);
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].id, "safe-local-task");
        assert_eq!(effective[0].status.as_deref(), Some("completed"));
        assert_eq!(effective[0].result.as_deref(), Some("completed"));
        assert_eq!(
            effective[0]
                .extra
                .get("hades_projection_repair")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            ScheduleLedger::new(dir.path().join("schedules.jsonl"))
                .effective()
                .unwrap()[&task.id]
                .state,
            ScheduleState::Completed
        );
    }

    #[test]
    fn terminal_completion_advances_the_authoritative_schedule() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "scheduled-task".into(),
            status: Some("in_progress".into()),
            extra: approved_workbench_extra(),
            ..blank("scheduled-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        ledger
            .append(&ScheduleRecord {
                contract: crate::prometheus::autopilot::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Once,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(Utc::now() - Duration::minutes(1)),
                interval_seconds: None,
                recorded_at_utc: Utc::now() - Duration::minutes(1),
                reason: None,
            })
            .expect("append schedule");

        ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .append_workbench_terminal(&task, "completed", "completed", "run-1", None, None)
            .expect("append terminal");

        assert_eq!(
            ledger.effective().unwrap()["scheduled-task"].state,
            ScheduleState::Completed
        );
    }

    #[test]
    fn terminal_completion_rejects_mismatched_schedule_lineage_before_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "terminal-lineage".into(),
            status: Some("in_progress".into()),
            extra: approved_workbench_extra(),
            ..blank("terminal-lineage")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let due = Utc::now() - chrono::Duration::minutes(1);
        append_test_schedule(
            &queue_path,
            &task.id,
            "different-objective",
            ScheduleMode::Once,
            ScheduleState::Scheduled,
            Some(due),
        );
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let before = std::fs::read_to_string(&queue_path).expect("read queue before");

        let error = executor
            .append_workbench_terminal(&task, "completed", "ok", "run-1", None, None)
            .expect_err("mismatched objective must reject terminal completion");

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read_to_string(&queue_path).unwrap(), before);
        assert_eq!(
            ScheduleLedger::new(dir.path().join("schedules.jsonl"))
                .effective()
                .unwrap()[&task.id]
                .not_before_utc,
            Some(due)
        );
    }

    #[test]
    fn terminal_cancellation_requires_canonical_schedule_authority() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "cancel-without-authority".into(),
            status: Some("in_progress".into()),
            extra: approved_workbench_extra(),
            ..blank("cancel-without-authority")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let before = std::fs::read_to_string(&queue_path).expect("read queue before");

        let error = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .append_workbench_terminal(
                &task,
                "failed",
                "cancelled",
                "run-cancel",
                None,
                Some("operator cancelled"),
            )
            .expect_err("missing schedule must reject cancellation");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(std::fs::read_to_string(&queue_path).unwrap(), before);
    }

    #[test]
    fn terminal_cancellation_blocks_later_completion() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "cancel-before-completion".into(),
            status: Some("in_progress".into()),
            extra: approved_workbench_extra(),
            ..blank("cancel-before-completion")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        executor
            .append_workbench_terminal(
                &task,
                "failed",
                "cancelled",
                "run-cancel",
                None,
                Some("operator cancelled"),
            )
            .expect("append governed cancellation");

        assert_eq!(
            ScheduleLedger::new(dir.path().join("schedules.jsonl"))
                .effective()
                .unwrap()[&task.id]
                .state,
            ScheduleState::Cancelled
        );
        let before_completion = std::fs::read_to_string(&queue_path).unwrap();
        let error = executor
            .append_workbench_terminal(
                &task,
                "completed",
                "completed",
                "run-cancel",
                Some("sha256:late"),
                None,
            )
            .expect_err("completion cannot overwrite cancellation");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(
            std::fs::read_to_string(&queue_path).unwrap(),
            before_completion
        );
        let effective = TaskQueueAnalyzer::effective_records(
            TaskQueueAnalyzer::new(&queue_path).load().unwrap(),
        );
        assert_eq!(effective[0].result.as_deref(), Some("cancelled"));
    }

    #[test]
    fn schedule_reconciliation_repairs_terminal_immediate_schedule() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let completed_at = Utc::now() - Duration::minutes(1);
        let task = QueueRecord {
            id: "immediate-task".into(),
            status: Some("completed".into()),
            result: Some("completed".into()),
            completed_at_utc: Some(completed_at.to_rfc3339()),
            extra: approved_workbench_extra(),
            ..blank("immediate-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        ledger
            .append(&ScheduleRecord {
                contract: crate::prometheus::autopilot::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Immediate,
                state: ScheduleState::Scheduled,
                not_before_utc: None,
                interval_seconds: None,
                recorded_at_utc: completed_at - Duration::minutes(1),
                reason: None,
            })
            .expect("append schedule");

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        assert_eq!(executor.reconcile_schedules(Utc::now()).unwrap(), 0);

        assert_eq!(
            ledger.effective().unwrap()["immediate-task"].state,
            ScheduleState::Completed
        );
    }

    #[test]
    fn schedule_reconciliation_repairs_terminal_then_reactivates_due_recurrence() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let now = Utc::now();
        let completed_at = now - Duration::minutes(2);
        let task = QueueRecord {
            id: "recurring-task".into(),
            status: Some("completed".into()),
            result: Some("completed".into()),
            completed_at_utc: Some(completed_at.to_rfc3339()),
            extra: approved_workbench_extra(),
            ..blank("recurring-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        let ledger = ScheduleLedger::new(dir.path().join("schedules.jsonl"));
        ledger
            .append(&ScheduleRecord {
                contract: crate::prometheus::autopilot::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "objective-1".into(),
                mode: ScheduleMode::Recurring,
                state: ScheduleState::Scheduled,
                not_before_utc: Some(now - Duration::minutes(3)),
                interval_seconds: Some(60),
                recorded_at_utc: now - Duration::minutes(3),
                reason: None,
            })
            .expect("append schedule");

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        assert_eq!(executor.reconcile_schedules(now).unwrap(), 1);

        let effective = TaskQueueAnalyzer::effective_records(
            TaskQueueAnalyzer::new(&queue_path).load().unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("queued"));
        assert_eq!(
            ledger.effective().unwrap()["recurring-task"].not_before_utc,
            Some(now - Duration::minutes(1))
        );
    }

    #[test]
    fn l3_bounded_mutation_harness_selects_edits_receipts_and_completes() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        let active_path = dir.path().join("core/state/queue_active.json");
        let fixture_path = dir.path().join("tests/fixtures/l3/p6_harness.md");
        let receipt_path = dir.path().join("data/prometheus/l3_e2e_receipt.json");
        std::fs::create_dir_all(queue_path.parent().expect("queue parent"))
            .expect("create queue parent");
        std::fs::create_dir_all(active_path.parent().expect("active parent"))
            .expect("create active parent");
        std::fs::create_dir_all(fixture_path.parent().expect("fixture parent"))
            .expect("create fixture parent");
        std::fs::create_dir_all(receipt_path.parent().expect("receipt parent"))
            .expect("create receipt parent");

        let task = QueueRecord {
            id: "tsk_plan_l3c_p6_ded5b63c05".into(),
            title: Some("Build L3 end-to-end autonomous mutation harness".into()),
            owner: Some("apollo,prometheus,chronos".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: l3_safe_local_extra(),
            ..blank("tsk_plan_l3c_p6_ded5b63c05")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                serde_json::to_string(&task).expect("serialize queue record")
            ),
        )
        .expect("write queue fixture");
        std::fs::write(
            &active_path,
            "{\"tasks\":[{\"id\":\"tsk_plan_l3c_p6_ded5b63c05\"}]}\n",
        )
        .expect("write active projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-l3",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let selected = executor
            .select_next_safe_local()
            .expect("select next")
            .expect("l3 safe-local task");
        let attempt = executor.append_attempt(&selected).expect("append attempt");

        std::fs::write(
            &fixture_path,
            "# L3 P6 Harness Fixture\n\nstatus: patched_by_tempfile_harness\n",
        )
        .expect("write bounded fixture");
        let verify_status = if fixture_path.exists() {
            "passed"
        } else {
            "failed"
        };
        assert_eq!(verify_status, "passed");

        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&json!({
                "schema_version": "arda.l3.bounded_mutation_receipt.v1",
                "task_id": selected.id,
                "class_id": "l3_local_doc_fixture_patch",
                "action_class": "local_refactors",
                "policy_mode": "block_on_fail",
                "changed_paths": ["tests/fixtures/l3/p6_harness.md"],
                "verify_command": "cargo test -p arda-prometheus l3",
                "verify_status": verify_status,
                "rollback": {
                    "strategy": "restore_previous_content_or_remove_fixture",
                    "command_hint": "git checkout -- tests/fixtures/l3/p6_harness.md || rm tests/fixtures/l3/p6_harness.md"
                },
                "queue_terminal_record_appended": true,
                "append_only_guard": "deferred_to_scripts/check_task_queue_append_only.sh",
                "attempt_contract": attempt.contract,
                "generated_at_utc": Utc::now().to_rfc3339()
            }))
            .expect("serialize receipt"),
        )
        .expect("write receipt");
        executor
            .append_terminal_completion(&selected, "completed")
            .expect("append terminal");

        let effective = TaskQueueAnalyzer::effective_records(
            TaskQueueAnalyzer::new(&queue_path)
                .load()
                .expect("load queue"),
        );
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].status.as_deref(), Some("completed"));
        let receipt: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&receipt_path).expect("read receipt"))
                .expect("parse receipt");
        assert_eq!(
            receipt
                .get("schema_version")
                .and_then(serde_json::Value::as_str),
            Some("arda.l3.bounded_mutation_receipt.v1")
        );
        assert_eq!(
            receipt
                .get("verify_status")
                .and_then(serde_json::Value::as_str),
            Some("passed")
        );
    }

    #[test]
    fn workbench_continuation_requires_canonical_schedule_authority() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "continuation-without-schedule",
            "title": "Do not reopen without schedule authority",
            "status": "in_progress",
            "meta": approved_workbench_extra()["meta"]
        }))
        .unwrap();
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_value(&task).unwrap()),
        )
        .unwrap();

        let error = ActiveQueueExecutor::new(dir.path())
            .append_workbench_continuation(
                &task,
                "run-without-schedule",
                "execute",
                None,
                "continue_test",
            )
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(TaskQueueAnalyzer::new(&queue_path).load().unwrap().len(), 1);
    }

    #[test]
    fn workbench_continuation_cannot_reopen_terminal_current_state() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "terminal-before-continuation",
            "title": "Do not reopen terminal work",
            "status": "in_progress",
            "meta": approved_workbench_extra()["meta"]
        }))
        .unwrap();
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_value(&task).unwrap()),
        )
        .unwrap();
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let executor = ActiveQueueExecutor::new(dir.path());
        executor
            .append_workbench_terminal(
                &task,
                "failed",
                "verification_failed",
                "terminal-run",
                None,
                Some("terminal before continuation"),
            )
            .unwrap();

        let error = executor
            .append_workbench_continuation(&task, "terminal-run", "review", None, "continue_close")
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        let effective = TaskQueueAnalyzer::effective_records(
            TaskQueueAnalyzer::new(&queue_path).load().unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("failed"));
    }

    fn l3_safe_local_extra() -> serde_json::Map<String, serde_json::Value> {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "meta".into(),
            json!({
                "action_class": "l3_local_doc_fixture_patch",
                "mutation_risk": "safe-local",
                "source_objective_packet_id": "objective-l3"
            }),
        );
        extra
    }

    fn approved_workbench_extra() -> serde_json::Map<String, serde_json::Value> {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "meta".into(),
            json!({
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "operator-approved",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "objective-1",
                "approval_packet_id": "approval-1"
            }),
        );
        extra
    }

    fn approved_workbench_extra_for_target(
        project_id: &str,
        worktree_path: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut extra = approved_workbench_extra();
        let meta = extra
            .get_mut("meta")
            .and_then(Value::as_object_mut)
            .expect("approved metadata object");
        meta.insert("project_id".into(), json!(project_id));
        meta.insert("worktree_path".into(), json!(worktree_path));
        extra
    }

    fn append_test_schedule(
        queue_path: &Path,
        task_id: &str,
        objective_id: &str,
        mode: ScheduleMode,
        state: ScheduleState,
        not_before_utc: Option<DateTime<Utc>>,
    ) {
        ScheduleLedger::new(queue_path.with_file_name("schedules.jsonl"))
            .append(&ScheduleRecord {
                contract: SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task_id.into(),
                objective_id: objective_id.into(),
                mode,
                state,
                not_before_utc,
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: Some("test fixture schedule authority".into()),
            })
            .expect("append test schedule");
    }

    #[test]
    fn forged_governance_metadata_is_not_workbench_approved() {
        let valid = json!({
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "governance-authorized-reversible",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "packet-1",
                "governance_action_class": "safe_local",
                "governance_gate": "safe_autonomous",
                "governance_authorization_id": "governance:packet-1:safe_local"
        });
        for (field, replacement) in [
            (
                "governance_authorization_id",
                json!("governance:another-packet:safe_local"),
            ),
            ("execution_authority", json!("untrusted_executor")),
            ("action_class", json!("human_required")),
            ("governance_gate", json!("review_required")),
        ] {
            let mut meta = valid.as_object().expect("metadata object").clone();
            meta.insert(field.into(), replacement);
            let mut task = blank("forged-governance-task");
            task.extra.insert("meta".into(), Value::Object(meta));

            assert!(
                !approved_workbench_metadata(&task),
                "accepted forged {field}"
            );
        }
    }

    fn l3_human_required_extra() -> serde_json::Map<String, serde_json::Value> {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "meta".into(),
            json!({
                "action_class": "human_required",
                "mutation_risk": "human-required"
            }),
        );
        extra
    }

    fn source_record_extra(source_record_id: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "source_record_id".into(),
            serde_json::Value::String(source_record_id.into()),
        );
        extra
    }

    fn blank(id: &str) -> QueueRecord {
        QueueRecord {
            id: id.into(),
            title: None,
            owner: None,
            priority: None,
            status: None,
            result: None,
            queued_at_utc: None,
            completed_at_utc: None,
            started_at_utc: None,
            extra: Default::default(),
        }
    }
}
