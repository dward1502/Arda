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

pub(super) const MAX_PARALLEL_READ_ONLY_PER_WORKSPACE: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueRecordStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Failed,
    Cancelled,
    Other,
}

impl QueueRecordStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl QueueRecord {
    pub fn canonical_status(&self) -> QueueRecordStatus {
        match self.status.as_deref().map(normalize_task_status) {
            Some("pending" | "queued") => QueueRecordStatus::Pending,
            Some("in_progress") => QueueRecordStatus::InProgress,
            Some("blocked") => QueueRecordStatus::Blocked,
            Some("completed") => QueueRecordStatus::Completed,
            Some("failed") => QueueRecordStatus::Failed,
            Some("cancelled") => QueueRecordStatus::Cancelled,
            _ => QueueRecordStatus::Other,
        }
    }
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
        let authorized_reopens = (0..records.len())
            .map(|index| record_authorizes_reopen(&records, index))
            .collect::<Vec<_>>();
        let valid_objective_revisions = (0..records.len())
            .map(|index| record_is_valid_objective_revision(&records, index))
            .collect::<Vec<_>>();
        let (valid_revision_approvals, blocked_by_pending_revision) =
            objective_revision_approval_authority(&records, &valid_objective_revisions);
        let terminal_keys = records
            .iter()
            .enumerate()
            .filter(|(index, record)| {
                let ignored_pending_successor = !valid_revision_approvals[*index]
                    && !valid_objective_revisions[*index]
                    && blocked_by_pending_revision[*index];
                !ignored_pending_successor && record.canonical_status().is_terminal()
            })
            .flat_map(|(_, record)| [record.id.clone(), Self::effective_record_key(record)])
            .collect::<BTreeSet<_>>();
        let mut seen_ids = BTreeSet::<String>::new();
        let mut effective = Vec::<(usize, QueueRecord)>::new();
        for (index, mut record) in records.into_iter().enumerate().rev() {
            let authorized_reopen = authorized_reopens[index];
            if !valid_revision_approvals[index] && !valid_objective_revisions[index] {
                if blocked_by_pending_revision[index] {
                    continue;
                }
                if let Some(meta) = record.extra.get_mut("meta").and_then(Value::as_object_mut) {
                    meta.remove("approval_packet_id");
                    meta.insert(
                        "mutation_risk".into(),
                        Value::String("invalid-objective-revision-approval".into()),
                    );
                }
            }
            let nonterminal = !record.canonical_status().is_terminal();
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

    pub(super) fn is_exact_persisted_workbench_attempt(
        &self,
        expected: &QueueRecord,
    ) -> std::io::Result<bool> {
        let file = OpenOptions::new().read(true).open(&self.queue_path)?;
        file.lock_shared()?;
        let result = (|| {
            let records = read_queue_records(&file)?;
            let current_matches = TaskQueueAnalyzer::effective_records(records.clone())
                .iter()
                .any(|record| record == expected);
            let replay_valid = records
                .iter()
                .enumerate()
                .rfind(|(_, record)| *record == expected)
                .is_some_and(|(index, _)| {
                    record_is_exact_persisted_workbench_attempt(&records, index)
                });
            Ok(current_matches && replay_valid)
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    pub fn select_next_safe_local(&self) -> std::io::Result<Option<QueueRecord>> {
        let records = TaskQueueAnalyzer::new(&self.queue_path).load()?;
        let effective = dispatch_priority_order(TaskQueueAnalyzer::effective_records(records));
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
        let effective = dispatch_priority_order(TaskQueueAnalyzer::effective_records(records));
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

    /// Append an operator-authored priority change to the canonical queue.
    pub fn reprioritize(
        &self,
        task_id: &str,
        objective_id: &str,
        priority: &str,
        reason: &str,
    ) -> std::io::Result<QueueRecord> {
        if !matches!(priority, "critical" | "high" | "medium" | "low") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "priority must be one of critical, high, medium, or low",
            ));
        }
        if reason.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "reprioritization requires a non-empty operator reason",
            ));
        }
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
            let current = effective
                .into_iter()
                .find(|record| record.id == task_id)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "queue task not found")
                })?;
            if queue_objective_id(&current) != Some(objective_id) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "reprioritization objective does not match canonical task lineage",
                ));
            }
            if matches!(
                current.status.as_deref().map(normalize_task_status),
                Some("completed" | "cancelled" | "failed")
            ) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "terminal queue tasks cannot be reprioritized",
                ));
            }
            if current.priority.as_deref() == Some(priority) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "task already has the requested priority",
                ));
            }
            let previous_priority = current.priority.clone();
            let reopen_contract = current
                .extra
                .get("contract")
                .and_then(Value::as_str)
                .filter(|contract| is_authorized_reopen_contract(contract))
                .map(str::to_owned);
            let mut appended = current;
            appended.priority = Some(priority.to_owned());
            if let Some(reopen_contract) = reopen_contract {
                appended.extra.insert(
                    "reprioritized_from_contract".into(),
                    Value::String(reopen_contract),
                );
            }
            appended.extra.insert(
                "contract".into(),
                Value::String("arda.workbench.queue_reprioritization.v1".into()),
            );
            appended.extra.insert(
                "previous_priority".into(),
                previous_priority.map(Value::String).unwrap_or(Value::Null),
            );
            appended
                .extra
                .insert("operator_reason".into(), Value::String(reason.into()));
            appended.extra.insert(
                "reprioritized_at_utc".into(),
                Value::String(Utc::now().to_rfc3339()),
            );
            serde_json::to_writer(&mut file, &appended)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()?;
            Ok(appended)
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    /// Append an operator-authored objective correction pending fresh approval.
    pub fn revise_objective(
        &self,
        task_id: &str,
        objective_id: &str,
        revised_objective: &str,
        reason: &str,
    ) -> std::io::Result<QueueRecord> {
        let revised_objective = revised_objective.trim();
        if revised_objective.is_empty() || reason.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "objective revision requires a non-empty objective and operator reason",
            ));
        }
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
            let current = effective
                .into_iter()
                .find(|record| record.id == task_id)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "queue task not found")
                })?;
            if queue_objective_id(&current) != Some(objective_id) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "objective revision does not match canonical task lineage",
                ));
            }
            if matches!(
                current.status.as_deref().map(normalize_task_status),
                Some("completed" | "failed" | "cancelled")
            ) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "terminal queue tasks cannot be revised",
                ));
            }
            let previous_objective = current.title.clone().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "canonical task omitted its operator objective",
                )
            })?;
            if previous_objective == revised_objective {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "task already has the requested objective",
                ));
            }
            let mut appended = current;
            appended.title = Some(revised_objective.to_owned());
            let meta = appended
                .extra
                .get_mut("meta")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "canonical task omitted governed queue metadata",
                    )
                })?;
            meta.insert(
                "mutation_risk".into(),
                Value::String("operator-revision-pending".into()),
            );
            meta.insert(
                "action_class".into(),
                Value::String("objective_revision_pending_approval".into()),
            );
            for field in [
                "approval_packet_id",
                "governance_authorization_id",
                "governance_gate",
            ] {
                meta.remove(field);
            }
            appended.extra.insert(
                "contract".into(),
                Value::String("arda.workbench.objective_revision.v1".into()),
            );
            appended.extra.insert(
                "previous_objective".into(),
                Value::String(previous_objective),
            );
            appended.extra.insert(
                "operator_reason".into(),
                Value::String(reason.trim().to_owned()),
            );
            appended.extra.insert(
                "objective_revised_at_utc".into(),
                Value::String(Utc::now().to_rfc3339()),
            );
            serde_json::to_writer(&mut file, &appended)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()?;
            Ok(appended)
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    /// Append fresh operator approval for the current revised objective.
    pub fn approve_revised_objective(
        &self,
        task_id: &str,
        objective_id: &str,
        approval_packet_id: &str,
        reviewed_by: &str,
        reason: &str,
    ) -> std::io::Result<QueueRecord> {
        let approval_packet_id = approval_packet_id.trim();
        let reviewed_by = reviewed_by.trim();
        let reason = reason.trim();
        if approval_packet_id.is_empty() || reviewed_by.is_empty() || reason.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "approval packet id, reviewer, and reason must be non-empty",
            ));
        }
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
            let valid_objective_revisions = (0..records.len())
                .map(|index| record_is_valid_objective_revision(&records, index))
                .collect::<Vec<_>>();
            let effective = TaskQueueAnalyzer::effective_records(records.clone());
            let mut appended = effective
                .into_iter()
                .find(|record| {
                    record.id == task_id && queue_objective_id(record) == Some(objective_id)
                })
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "revised queue task not found",
                    )
                })?;
            if appended.extra.get("contract").and_then(Value::as_str)
                != Some("arda.workbench.objective_revision.v1")
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "task is not pending objective revision approval",
                ));
            }
            records
                .iter()
                .enumerate()
                .rposition(|(index, record)| {
                    valid_objective_revisions[index]
                        && record.id == task_id
                        && queue_objective_id(record) == Some(objective_id)
                })
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "task is not pending a structurally valid objective revision",
                    )
                })?;
            if approval_packet_precedes_index(&records, records.len(), approval_packet_id) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "approval packet was already used in the canonical queue ledger",
                ));
            }
            let meta = appended
                .extra
                .get_mut("meta")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "revised task omitted governed queue metadata",
                    )
                })?;
            meta.insert(
                "mutation_risk".into(),
                Value::String("operator-approved".into()),
            );
            meta.insert(
                "action_class".into(),
                Value::String("approved_autopilot_plan_step".into()),
            );
            meta.insert(
                "approval_packet_id".into(),
                Value::String(approval_packet_id.into()),
            );
            appended.extra.insert(
                "contract".into(),
                Value::String("arda.workbench.objective_revision_approval.v1".into()),
            );
            appended
                .extra
                .insert("reviewed_by".into(), Value::String(reviewed_by.into()));
            appended
                .extra
                .insert("operator_reason".into(), Value::String(reason.into()));
            appended.extra.insert(
                "objective_approved_at_utc".into(),
                Value::String(Utc::now().to_rfc3339()),
            );
            serde_json::to_writer(&mut file, &appended)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            writeln!(file)?;
            file.sync_data()?;
            Ok(appended)
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
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
    #[cfg(test)]
    pub(super) fn claim_next_approved(&self) -> std::io::Result<Option<ApprovedQueueClaim>> {
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
                let effective =
                    dispatch_priority_order(TaskQueueAnalyzer::effective_records(records));
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
                let effective =
                    dispatch_priority_order(TaskQueueAnalyzer::effective_records(records));
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
                let effective = dispatch_priority_order(TaskQueueAnalyzer::effective_records(
                    read_queue_records(&file)?,
                ));
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
        expected: &QueueRecord,
        excluded_task_ids: &BTreeSet<String>,
    ) -> std::io::Result<Option<ApprovedQueueClaim>> {
        ScheduleLedger::new(&self.schedule_path).with_effective(|schedules| {
            let mut file = OpenOptions::new()
                .read(true)
                .append(true)
                .open(&self.queue_path)?;
            file.lock_exclusive()?;
            let result = (|| {
                let effective = dispatch_priority_order(TaskQueueAnalyzer::effective_records(
                    read_queue_records(&file)?,
                ));
                let now = Utc::now();
                let selected = effective
                    .iter()
                    .find(|record| {
                        !excluded_task_ids.contains(&record.id)
                            && record.status.as_deref().map(normalize_task_status)
                                == Some("in_progress")
                            && authoritative_schedule_eligible(record, schedules, now)
                            && approved_workbench_metadata(record)
                    })
                    .or_else(|| {
                        effective.iter().find(|record| {
                            !excluded_task_ids.contains(&record.id)
                                && claimable_status(record)
                                && authoritative_schedule_eligible(record, schedules, now)
                                && approved_workbench_metadata(record)
                                && mutation_lease_available(record, &effective, now)
                        })
                    });
                let Some(task) = selected.filter(|record| *record == expected) else {
                    return Ok(None);
                };
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

    #[cfg(test)]
    fn append_attempt(&self, task: &QueueRecord) -> std::io::Result<ActiveQueueExecutionAttempt> {
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

    #[cfg(test)]
    pub(super) fn append_attempt_fixture(
        &self,
        task: &QueueRecord,
    ) -> std::io::Result<ActiveQueueExecutionAttempt> {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.queue_path)?;
        file.lock_exclusive()?;
        let result = (|| {
            let attempt = execution_attempt(task);
            append_attempt_to_writer(&mut file, task, &attempt)?;
            file.sync_data()?;
            Ok(attempt)
        })();
        let unlock = FileExt::unlock(&file);
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
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
    if record.extra.get("contract").and_then(Value::as_str)
        == Some("arda.workbench.objective_revision.v1")
    {
        return false;
    }
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
    let mut conflicting_active = effective.iter().filter(|active| {
        active.id != candidate.id
            && holds_active_mutation_lease(active, now)
            && mutation_targets_conflict(candidate, active)
    });
    if !has_read_only_execution_authority(candidate) {
        return conflicting_active.next().is_none();
    }

    let mut active_readers = 0;
    for active in conflicting_active {
        if !has_read_only_execution_authority(active) {
            return false;
        }
        active_readers += 1;
        if active_readers >= MAX_PARALLEL_READ_ONLY_PER_WORKSPACE {
            return false;
        }
    }
    true
}

pub(super) fn has_read_only_execution_authority(record: &QueueRecord) -> bool {
    let Some(meta) = record.extra.get("meta").and_then(Value::as_object) else {
        return false;
    };
    if meta.get("authority_class").and_then(Value::as_str) != Some("read_only") {
        return false;
    }

    let decomposable_objective = meta.get("objective_leaf").and_then(Value::as_bool) != Some(true)
        && meta
            .get("acceptance_artifact")
            .and_then(Value::as_str)
            .is_some_and(|path| !path.trim().is_empty())
        && meta
            .get("acceptance_markers")
            .and_then(Value::as_array)
            .is_some_and(|markers| !markers.is_empty());
    !decomposable_objective
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
    let appended_at_utc = Utc::now();
    ActiveQueueExecutionAttempt {
        contract: "arda.prometheus.active_queue_execution_attempt.v1".to_owned(),
        executor: "arda_workbench.queue_executor".to_owned(),
        task_id: task.id.clone(),
        status: "claimed".to_owned(),
        action_class,
        hades_projection_repair: false,
        appended_at_utc,
        workbench_run_id: workbench_run_id(
            &task.id,
            task.extra
                .get("retry_sequence")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        lease_expires_at_utc: appended_at_utc + Duration::minutes(20),
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

fn record_is_exact_persisted_workbench_attempt(records: &[QueueRecord], index: usize) -> bool {
    let Some(current) = records.get(index) else {
        return false;
    };
    let Some(started_at) = current.started_at_utc.as_deref().and_then(parse_utc) else {
        return false;
    };
    let Some(predecessor) = TaskQueueAnalyzer::effective_records(records[..index].to_vec())
        .into_iter()
        .find(|record| record.id == current.id)
    else {
        return false;
    };
    let action_class = predecessor
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("action_class"))
        .and_then(Value::as_str)
        .unwrap_or("approved_autopilot_plan_step")
        .to_owned();
    let attempt = ActiveQueueExecutionAttempt {
        contract: "arda.prometheus.active_queue_execution_attempt.v1".into(),
        executor: "arda_workbench.queue_executor".into(),
        task_id: predecessor.id.clone(),
        status: "claimed".into(),
        action_class,
        hades_projection_repair: false,
        appended_at_utc: started_at,
        workbench_run_id: workbench_run_id(
            &predecessor.id,
            predecessor
                .extra
                .get("retry_sequence")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        lease_expires_at_utc: started_at + Duration::minutes(20),
    };
    let mut encoded = Vec::new();
    if append_attempt_to_writer(&mut encoded, &predecessor, &attempt).is_err() {
        return false;
    }
    serde_json::from_slice::<QueueRecord>(&encoded).is_ok_and(|expected| expected == *current)
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

fn dispatch_priority_order(mut records: Vec<QueueRecord>) -> Vec<QueueRecord> {
    records.sort_by_key(|record| match record.priority.as_deref() {
        Some("critical") => 0,
        Some("high") => 1,
        Some("medium") => 2,
        Some("low") => 3,
        _ => 4,
    });
    records
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

#[cfg(test)]
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

fn is_authorized_reopen_contract(contract: &str) -> bool {
    matches!(
        contract,
        "arda.workbench.queue_retry.v1"
            | "arda.workbench.queue_continuation.v1"
            | "arda.workbench.executable_continuation.v1"
            | "arda.schedule.queue_activation.v1"
    )
}

fn records_share_effective_alias(left: &QueueRecord, right: &QueueRecord) -> bool {
    let left_source = TaskQueueAnalyzer::effective_record_key(left);
    let right_source = TaskQueueAnalyzer::effective_record_key(right);
    [left.id.as_str(), left_source.as_str()]
        .iter()
        .any(|left_alias| {
            [right.id.as_str(), right_source.as_str()]
                .iter()
                .any(|right_alias| left_alias == right_alias)
        })
}

fn prior_same_alias_index(records: &[QueueRecord], index: usize) -> Option<usize> {
    (0..index)
        .rev()
        .find(|prior_index| records_share_effective_alias(&records[index], &records[*prior_index]))
}

fn same_reopen_identity(current: &QueueRecord, prior: &QueueRecord) -> bool {
    current.id == prior.id
        && TaskQueueAnalyzer::effective_record_key(current) == prior.id
        && current.title == prior.title
        && current.owner == prior.owner
        && current.priority == prior.priority
        && queue_objective_id(current) == queue_objective_id(prior)
        && approved_workbench_metadata(current)
}

fn same_reopen_payload(current: &QueueRecord, prior: &QueueRecord) -> bool {
    same_reopen_identity(current, prior) && current.extra.get("meta") == prior.extra.get("meta")
}

fn executable_continuation_meta_matches(current: &QueueRecord, prior: &QueueRecord) -> bool {
    let Some(mut expected) = prior.extra.get("meta").and_then(Value::as_object).cloned() else {
        return false;
    };
    for field in [
        "continuation_decision",
        "continuation_sequence",
        "retry_sequence",
        "revision_sequence",
        "revision_directive",
    ] {
        let Some(value) = current.extra.get(field).cloned() else {
            return false;
        };
        expected.insert(field.into(), value);
    }
    current.extra.get("meta") == Some(&Value::Object(expected))
}

fn extra_timestamp(record: &QueueRecord, field: &str) -> bool {
    record
        .extra
        .get(field)
        .and_then(Value::as_str)
        .and_then(parse_utc)
        .is_some()
}

fn sequence(record: &QueueRecord, field: &str) -> u64 {
    record
        .extra
        .get(field)
        .and_then(Value::as_u64)
        .or_else(|| record.extra["meta"].get(field).and_then(Value::as_u64))
        .unwrap_or(0)
}

fn validated_root_reopen_contract(records: &[QueueRecord], index: usize) -> Option<&str> {
    let current = &records[index];
    let contract = current.extra.get("contract").and_then(Value::as_str)?;
    if !is_authorized_reopen_contract(contract) {
        return None;
    }
    let prior_index = prior_same_alias_index(records, index)?;
    let prior = &records[prior_index];
    let current_status = current.status.as_deref().map(normalize_task_status);
    let prior_status = prior.status.as_deref().map(normalize_task_status);
    let valid = match contract {
        "arda.workbench.queue_retry.v1" => {
            same_reopen_payload(current, prior)
                && prior_status == Some("failed")
                && prior.result.as_deref() != Some("cancelled")
                && current_status == Some("queued")
                && current.result.is_none()
                && current.extra.get("executor").and_then(Value::as_str)
                    == Some("arda_workbench.queue_executor")
                && current.extra.get("retry_sequence").and_then(Value::as_u64)
                    == Some(sequence(prior, "retry_sequence") + 1)
                && extra_timestamp(current, "retried_at_utc")
        }
        "arda.workbench.queue_continuation.v1" => {
            same_reopen_payload(current, prior)
                && prior_status == Some("in_progress")
                && current_status == Some("in_progress")
                && current.extra.get("executor").and_then(Value::as_str)
                    == Some("arda_workbench.queue_executor")
                && current
                    .extra
                    .get("workbench_run_id")
                    .and_then(Value::as_str)
                    .filter(|run_id| !run_id.trim().is_empty())
                    == prior.extra.get("workbench_run_id").and_then(Value::as_str)
                && current
                    .extra
                    .get("completed_stage")
                    .and_then(Value::as_str)
                    .is_some_and(|stage| !stage.trim().is_empty())
                && current
                    .extra
                    .get("continuation_decision")
                    .and_then(Value::as_str)
                    .is_some_and(|decision| !decision.trim().is_empty())
                && extra_timestamp(current, "recorded_at_utc")
        }
        "arda.workbench.executable_continuation.v1" => {
            let decision = current
                .extra
                .get("continuation_decision")
                .and_then(Value::as_str);
            let queued_wait_activation = prior.extra.get("contract").and_then(Value::as_str)
                == Some("arda.workbench.executable_continuation.v1")
                && validated_root_reopen_contract(records, prior_index)
                    == Some("arda.workbench.executable_continuation.v1")
                && prior_status == Some("blocked")
                && current_status == Some("queued")
                && decision == Some("wait_until")
                && same_reopen_identity(current, prior)
                && current.extra == prior.extra;
            queued_wait_activation
                || (same_reopen_identity(current, prior)
                    && matches!(prior_status, Some("failed" | "cancelled"))
                    && matches!(current_status, Some("queued" | "blocked"))
                    && matches!(
                        decision,
                        Some("retry_same_task" | "revise_task" | "wait_until")
                    )
                    && (decision == Some("wait_until")) == (current_status == Some("blocked"))
                    && current
                        .extra
                        .get("parent_workbench_run_id")
                        .and_then(Value::as_str)
                        .filter(|run_id| !run_id.trim().is_empty())
                        == prior.extra.get("workbench_run_id").and_then(Value::as_str)
                    && current
                        .extra
                        .get("continuation_sequence")
                        .and_then(Value::as_u64)
                        == Some(sequence(prior, "continuation_sequence") + 1)
                    && current.extra.get("retry_sequence").and_then(Value::as_u64)
                        == Some(sequence(prior, "retry_sequence") + 1)
                    && current
                        .extra
                        .get("revision_sequence")
                        .and_then(Value::as_u64)
                        == Some(
                            sequence(prior, "revision_sequence")
                                + u64::from(decision == Some("revise_task")),
                        )
                    && current
                        .extra
                        .get("revision_directive")
                        .and_then(Value::as_str)
                        .is_some()
                    && executable_continuation_meta_matches(current, prior)
                    && current
                        .queued_at_utc
                        .as_deref()
                        .and_then(parse_utc)
                        .is_some())
        }
        "arda.schedule.queue_activation.v1" => {
            same_reopen_payload(current, prior)
                && prior_status == Some("completed")
                && prior.result.as_deref() == Some("completed")
                && current_status == Some("queued")
                && current.result.is_none()
                && current
                    .extra
                    .get("scheduled_for_utc")
                    .and_then(Value::as_str)
                    .and_then(parse_utc)
                    .is_some()
                && current
                    .extra
                    .get("schedule_mode")
                    .and_then(Value::as_str)
                    .is_some_and(|mode| matches!(mode, "once" | "deferred" | "recurring"))
                && extra_timestamp(current, "recorded_at_utc")
        }
        _ => false,
    };
    valid.then_some(contract)
}

fn reprioritization_preserves_predecessor(
    current: &QueueRecord,
    prior: &QueueRecord,
    root_authority: &str,
) -> bool {
    let canonical_priority = matches!(
        current.priority.as_deref(),
        Some("critical" | "high" | "medium" | "low")
    );
    let previous_priority_matches =
        current
            .extra
            .get("previous_priority")
            .is_some_and(|previous| match prior.priority.as_deref() {
                Some(priority) => previous.as_str() == Some(priority),
                None => previous.is_null(),
            });
    let valid_reason = current
        .extra
        .get("operator_reason")
        .and_then(Value::as_str)
        .is_some_and(|reason| !reason.trim().is_empty());
    let valid_timestamp = extra_timestamp(current, "reprioritized_at_utc");
    let same_top_level_payload = current.id == prior.id
        && TaskQueueAnalyzer::effective_record_key(current)
            == TaskQueueAnalyzer::effective_record_key(prior)
        && current.title == prior.title
        && current.owner == prior.owner
        && current.status == prior.status
        && current.result == prior.result
        && current.queued_at_utc == prior.queued_at_utc
        && current.completed_at_utc == prior.completed_at_utc
        && current.started_at_utc == prior.started_at_utc;
    let mut current_extra = current.extra.clone();
    let mut prior_extra = prior.extra.clone();
    for field in [
        "contract",
        "reprioritized_from_contract",
        "previous_priority",
        "operator_reason",
        "reprioritized_at_utc",
    ] {
        current_extra.remove(field);
        prior_extra.remove(field);
    }
    canonical_priority
        && current.priority != prior.priority
        && previous_priority_matches
        && valid_reason
        && valid_timestamp
        && same_top_level_payload
        && current
            .extra
            .get("reprioritized_from_contract")
            .and_then(Value::as_str)
            == Some(root_authority)
        && current_extra == prior_extra
}

fn record_authorizes_reopen(records: &[QueueRecord], index: usize) -> bool {
    let record = &records[index];
    let Some(contract) = record.extra.get("contract").and_then(Value::as_str) else {
        return false;
    };
    if is_authorized_reopen_contract(contract) {
        return validated_root_reopen_contract(records, index).is_some();
    }
    if contract != "arda.workbench.queue_reprioritization.v1" {
        return false;
    }
    let Some(root_authority) = record
        .extra
        .get("reprioritized_from_contract")
        .and_then(Value::as_str)
        .filter(|contract| is_authorized_reopen_contract(contract))
    else {
        return false;
    };

    let mut cursor = index;
    while let Some(prior_index) = prior_same_alias_index(records, cursor) {
        if !reprioritization_preserves_predecessor(
            &records[cursor],
            &records[prior_index],
            root_authority,
        ) {
            return false;
        }
        let prior = &records[prior_index];
        match prior.extra.get("contract").and_then(Value::as_str) {
            Some(prior_contract) if is_authorized_reopen_contract(prior_contract) => {
                return validated_root_reopen_contract(records, prior_index)
                    == Some(root_authority);
            }
            Some("arda.workbench.queue_reprioritization.v1")
                if prior
                    .extra
                    .get("reprioritized_from_contract")
                    .and_then(Value::as_str)
                    == Some(root_authority) =>
            {
                cursor = prior_index;
            }
            _ => return false,
        }
    }
    false
}

fn parse_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn record_is_valid_objective_revision(records: &[QueueRecord], index: usize) -> bool {
    let Some(current) = records.get(index) else {
        return false;
    };
    if current.extra.get("contract").and_then(Value::as_str)
        != Some("arda.workbench.objective_revision.v1")
    {
        return false;
    }
    let Some(prior_index) = previous_same_task_record_index(records, index) else {
        return false;
    };
    let prior = &records[prior_index];
    let Some(revised_objective) = current
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let previous_objective = prior.title.as_deref().unwrap_or_default();
    if current.id != prior.id
        || revised_objective == previous_objective
        || current
            .extra
            .get("previous_objective")
            .and_then(Value::as_str)
            != Some(previous_objective)
    {
        return false;
    }
    let Some(reason) = current
        .extra
        .get("operator_reason")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return false;
    };
    let Some(revised_at) = current
        .extra
        .get("objective_revised_at_utc")
        .and_then(Value::as_str)
        .filter(|value| parse_utc(value).is_some())
    else {
        return false;
    };
    let mut expected = prior.clone();
    expected.title = Some(revised_objective.to_string());
    let Some(meta) = expected
        .extra
        .get_mut("meta")
        .and_then(Value::as_object_mut)
    else {
        return false;
    };
    meta.insert(
        "mutation_risk".into(),
        Value::String("operator-revision-pending".into()),
    );
    meta.insert(
        "action_class".into(),
        Value::String("objective_revision_pending_approval".into()),
    );
    for field in [
        "approval_packet_id",
        "governance_authorization_id",
        "governance_gate",
    ] {
        meta.remove(field);
    }
    expected.extra.insert(
        "contract".into(),
        Value::String("arda.workbench.objective_revision.v1".into()),
    );
    expected.extra.insert(
        "previous_objective".into(),
        Value::String(previous_objective.into()),
    );
    expected
        .extra
        .insert("operator_reason".into(), Value::String(reason.into()));
    expected.extra.insert(
        "objective_revised_at_utc".into(),
        Value::String(revised_at.into()),
    );
    serde_json::to_value(expected).ok() == serde_json::to_value(current).ok()
}

fn record_is_valid_objective_revision_approval(
    records: &[QueueRecord],
    valid_objective_revisions: &[bool],
    prior_index: usize,
    index: usize,
) -> bool {
    let Some(current) = records.get(index) else {
        return false;
    };
    if current.extra.get("contract").and_then(Value::as_str)
        != Some("arda.workbench.objective_revision_approval.v1")
    {
        return false;
    }
    if !valid_objective_revisions
        .get(prior_index)
        .copied()
        .unwrap_or(false)
    {
        return false;
    }
    let prior = &records[prior_index];
    let Some(meta) = current.extra.get("meta").and_then(Value::as_object) else {
        return false;
    };
    let Some(approval_packet_id) = meta
        .get("approval_packet_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return false;
    };
    if approval_packet_id.trim() != approval_packet_id {
        return false;
    }
    if approval_packet_precedes_index(records, index, approval_packet_id) {
        return false;
    }
    let Some(reviewed_by) = current
        .extra
        .get("reviewed_by")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return false;
    };
    let Some(reason) = current
        .extra
        .get("operator_reason")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return false;
    };
    let Some(approved_at) = current
        .extra
        .get("objective_approved_at_utc")
        .and_then(Value::as_str)
        .filter(|value| parse_utc(value).is_some())
    else {
        return false;
    };
    let mut expected = prior.clone();
    let Some(expected_meta) = expected
        .extra
        .get_mut("meta")
        .and_then(Value::as_object_mut)
    else {
        return false;
    };
    expected_meta.insert(
        "mutation_risk".into(),
        Value::String("operator-approved".into()),
    );
    expected_meta.insert(
        "action_class".into(),
        Value::String("approved_autopilot_plan_step".into()),
    );
    expected_meta.insert(
        "approval_packet_id".into(),
        Value::String(approval_packet_id.into()),
    );
    expected.extra.insert(
        "contract".into(),
        Value::String("arda.workbench.objective_revision_approval.v1".into()),
    );
    expected
        .extra
        .insert("reviewed_by".into(), Value::String(reviewed_by.into()));
    expected
        .extra
        .insert("operator_reason".into(), Value::String(reason.into()));
    expected.extra.insert(
        "objective_approved_at_utc".into(),
        Value::String(approved_at.into()),
    );
    serde_json::to_value(expected).ok() == serde_json::to_value(current).ok()
}

fn approval_packet_precedes_index(
    records: &[QueueRecord],
    exclusive_end: usize,
    approval_packet_id: &str,
) -> bool {
    if exclusive_end > records.len() {
        return false;
    }
    records[..exclusive_end].iter().any(|candidate| {
        candidate
            .extra
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|meta| meta.get("approval_packet_id"))
            .and_then(Value::as_str)
            .map(str::trim)
            == Some(approval_packet_id.trim())
    })
}

fn objective_revision_approval_authority(
    records: &[QueueRecord],
    valid_objective_revisions: &[bool],
) -> (Vec<bool>, Vec<bool>) {
    let mut pending_revisions = Vec::<usize>::new();
    let mut authority = Vec::with_capacity(records.len());
    let mut blocked_by_pending_revision = Vec::with_capacity(records.len());
    for index in 0..records.len() {
        if valid_objective_revisions
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            pending_revisions.retain(|pending| {
                !records_share_effective_alias(&records[index], &records[*pending])
            });
            pending_revisions.push(index);
            authority.push(false);
            blocked_by_pending_revision.push(false);
            continue;
        }
        let is_revision_approval = records[index].extra.get("contract").and_then(Value::as_str)
            == Some("arda.workbench.objective_revision_approval.v1");
        if is_revision_approval {
            let pending_revision =
                pending_revisions.iter().rev().copied().find(|pending| {
                    records_share_effective_alias(&records[index], &records[*pending])
                });
            let valid = pending_revision
                .map(|prior_index| {
                    record_is_valid_objective_revision_approval(
                        records,
                        valid_objective_revisions,
                        prior_index,
                        index,
                    )
                })
                .unwrap_or(false);
            blocked_by_pending_revision.push(pending_revision.is_some() && !valid);
            if valid {
                pending_revisions.retain(|pending| {
                    !records_share_effective_alias(&records[index], &records[*pending])
                });
            }
            authority.push(valid);
            continue;
        }
        let blocked = pending_revisions
            .iter()
            .any(|pending| records_share_effective_alias(&records[index], &records[*pending]));
        authority.push(!blocked);
        blocked_by_pending_revision.push(blocked);
    }
    (authority, blocked_by_pending_revision)
}

fn previous_same_task_record_index(records: &[QueueRecord], index: usize) -> Option<usize> {
    let current = records.get(index)?;
    records[..index].iter().rposition(|candidate| {
        candidate.id == current.id
            || TaskQueueAnalyzer::effective_record_key(candidate) == current.id
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_queue_status_owns_alias_and_terminal_semantics() {
        let status = |value: &str| {
            serde_json::from_value::<QueueRecord>(json!({"id": "task", "status": value}))
                .unwrap()
                .canonical_status()
        };

        assert_eq!(status("queued"), QueueRecordStatus::Pending);
        assert_eq!(status("running"), QueueRecordStatus::InProgress);
        assert_eq!(status("done"), QueueRecordStatus::Completed);
        assert!(status("done").is_terminal());
        assert!(status("failed").is_terminal());
        assert!(!status("blocked").is_terminal());
    }

    #[test]
    fn persisted_workbench_attempt_requires_exact_writer_replay() {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "approval_packet_id".into(),
            Value::String("approval-exact-replay".into()),
        );
        meta.insert(
            "action_class".into(),
            Value::String("approved_autopilot_plan_step".into()),
        );
        meta.insert("protected_marker".into(), Value::String("original".into()));
        let mut extra = serde_json::Map::new();
        extra.insert("meta".into(), Value::Object(meta));
        let predecessor = QueueRecord {
            id: "exact-replay-task".into(),
            title: Some("Original title".into()),
            owner: Some("prometheus".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            result: None,
            queued_at_utc: Some("2026-08-28T23:00:00Z".into()),
            completed_at_utc: None,
            started_at_utc: None,
            extra,
        };
        let started_at = parse_utc("2026-08-28T23:10:00Z").unwrap();
        let attempt = ActiveQueueExecutionAttempt {
            contract: "arda.prometheus.active_queue_execution_attempt.v1".into(),
            executor: "arda_workbench.queue_executor".into(),
            task_id: predecessor.id.clone(),
            status: "claimed".into(),
            action_class: "approved_autopilot_plan_step".into(),
            hades_projection_repair: false,
            appended_at_utc: started_at,
            workbench_run_id: workbench_run_id(&predecessor.id, 0),
            lease_expires_at_utc: started_at + Duration::minutes(20),
        };
        let mut encoded = Vec::new();
        append_attempt_to_writer(&mut encoded, &predecessor, &attempt).unwrap();
        let persisted: QueueRecord = serde_json::from_slice(&encoded).unwrap();

        assert!(record_is_exact_persisted_workbench_attempt(
            &[predecessor.clone(), persisted.clone()],
            1,
        ));

        let mut noncanonical_run_id = persisted.clone();
        noncanonical_run_id.extra.insert(
            "workbench_run_id".into(),
            Value::String("queue-exact-replay-task-attempt-0002".into()),
        );
        assert!(!record_is_exact_persisted_workbench_attempt(
            &[predecessor.clone(), noncanonical_run_id],
            1,
        ));

        let mut forged = persisted;
        forged.title = Some("Forged title".into());
        assert!(!record_is_exact_persisted_workbench_attempt(
            &[predecessor, forged],
            1,
        ));
    }

    #[test]
    fn execution_attempt_lease_is_exactly_twenty_minutes() {
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "canonical-lease-task",
            "status": "queued",
            "meta": {"action_class": "approved_autopilot_plan_step"}
        }))
        .unwrap();

        let attempt = execution_attempt(&task);

        assert_eq!(
            attempt.lease_expires_at_utc,
            attempt.appended_at_utc + Duration::minutes(20),
        );
    }
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
    fn effective_records_reject_forged_reprioritization_reopen() {
        let terminal = QueueRecord {
            id: "terminal".into(),
            status: Some("completed".into()),
            result: Some("completed".into()),
            ..blank("terminal")
        };
        let mut forged_extra = source_record_extra("terminal");
        forged_extra.insert(
            "contract".into(),
            json!("arda.workbench.queue_reprioritization.v1"),
        );
        forged_extra.insert(
            "reprioritized_from_contract".into(),
            json!("arda.workbench.queue_retry.v1"),
        );
        let forged = QueueRecord {
            id: "forged-reprioritization".into(),
            priority: Some("critical".into()),
            status: Some("queued".into()),
            extra: forged_extra,
            ..blank("forged-reprioritization")
        };

        let effective = TaskQueueAnalyzer::effective_records(vec![terminal, forged]);

        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].id, "terminal");
        assert_eq!(effective[0].status.as_deref(), Some("completed"));
    }

    #[test]
    fn effective_records_reject_self_asserted_reopen_contract_roots() {
        for contract in [
            "arda.workbench.queue_retry.v1",
            "arda.workbench.queue_continuation.v1",
            "arda.workbench.executable_continuation.v1",
            "arda.schedule.queue_activation.v1",
        ] {
            let terminal = QueueRecord {
                id: "terminal".into(),
                status: Some("failed".into()),
                result: Some("failed".into()),
                ..blank("terminal")
            };
            let mut forged_extra = source_record_extra("terminal");
            forged_extra.insert("contract".into(), json!(contract));
            let forged = QueueRecord {
                id: "terminal".into(),
                status: Some("queued".into()),
                extra: forged_extra,
                ..blank("terminal")
            };

            let effective = TaskQueueAnalyzer::effective_records(vec![terminal, forged]);

            assert_eq!(effective.len(), 1, "{contract}");
            assert_eq!(effective[0].status.as_deref(), Some("failed"), "{contract}");
        }
    }

    #[test]
    fn effective_records_reject_mismatched_and_broken_reopen_authority_chains() {
        let authorized_retry_records = || {
            let mut terminal = QueueRecord {
                id: "terminal".into(),
                priority: Some("low".into()),
                status: Some("failed".into()),
                result: Some("failed".into()),
                extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
                ..blank("terminal")
            };
            terminal
                .extra
                .insert("source_record_id".into(), json!("terminal"));
            let mut retry_extra = terminal.extra.clone();
            retry_extra.insert("contract".into(), json!("arda.workbench.queue_retry.v1"));
            retry_extra.insert("executor".into(), json!("arda_workbench.queue_executor"));
            retry_extra.insert("retry_sequence".into(), json!(1));
            retry_extra.insert("retried_at_utc".into(), json!(Utc::now().to_rfc3339()));
            let retry = QueueRecord {
                status: Some("queued".into()),
                result: None,
                extra: retry_extra,
                ..terminal.clone()
            };
            (terminal, retry)
        };
        let reprioritization = |prior: &QueueRecord, authority: &str| {
            let mut extra = prior.extra.clone();
            extra.insert(
                "contract".into(),
                json!("arda.workbench.queue_reprioritization.v1"),
            );
            extra.insert("reprioritized_from_contract".into(), json!(authority));
            QueueRecord {
                priority: Some("critical".into()),
                extra,
                ..prior.clone()
            }
        };

        let (terminal, retry) = authorized_retry_records();
        let mismatched = reprioritization(&retry, "arda.workbench.executable_continuation.v1");
        let effective =
            TaskQueueAnalyzer::effective_records(vec![terminal.clone(), retry.clone(), mismatched]);
        assert_eq!(effective[0].priority.as_deref(), Some("low"));
        assert_eq!(
            effective[0].extra.get("contract").and_then(Value::as_str),
            Some("arda.workbench.queue_retry.v1")
        );

        let interrupted = QueueRecord {
            extra: source_record_extra("terminal"),
            ..retry.clone()
        };
        let after_interruption = reprioritization(&interrupted, "arda.workbench.queue_retry.v1");
        let effective = TaskQueueAnalyzer::effective_records(vec![
            terminal.clone(),
            retry.clone(),
            interrupted,
            after_interruption,
        ]);
        assert_eq!(effective[0].priority.as_deref(), Some("low"));
        assert_eq!(
            effective[0].extra.get("contract").and_then(Value::as_str),
            Some("arda.workbench.queue_retry.v1")
        );

        let mut forged_root = retry.clone();
        forged_root.extra.remove("executor");
        forged_root.extra.remove("retry_sequence");
        forged_root.extra.remove("retried_at_utc");
        let after_forgery = reprioritization(&forged_root, "arda.workbench.queue_retry.v1");
        let effective =
            TaskQueueAnalyzer::effective_records(vec![terminal, forged_root, after_forgery]);
        assert_eq!(effective[0].status.as_deref(), Some("failed"));
        assert_eq!(effective[0].priority.as_deref(), Some("low"));
    }

    #[test]
    fn effective_records_reject_forged_repeated_reprioritization_transitions() {
        let mut terminal = QueueRecord {
            id: "terminal".into(),
            title: Some("Canonical title".into()),
            owner: Some("prometheus".into()),
            priority: Some("low".into()),
            status: Some("failed".into()),
            result: Some("failed".into()),
            queued_at_utc: Some("2026-08-28T12:00:00Z".into()),
            completed_at_utc: Some("2026-08-28T12:01:00Z".into()),
            started_at_utc: Some("2026-08-28T12:00:30Z".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
        };
        terminal
            .extra
            .insert("source_record_id".into(), json!("terminal"));
        let mut retry = QueueRecord {
            status: Some("queued".into()),
            result: None,
            ..terminal.clone()
        };
        retry
            .extra
            .insert("contract".into(), json!("arda.workbench.queue_retry.v1"));
        retry
            .extra
            .insert("executor".into(), json!("arda_workbench.queue_executor"));
        retry.extra.insert("retry_sequence".into(), json!(1));
        retry
            .extra
            .insert("retried_at_utc".into(), json!(Utc::now().to_rfc3339()));
        let mut genuine = QueueRecord {
            priority: Some("high".into()),
            extra: retry.extra.clone(),
            ..retry.clone()
        };
        genuine.extra.insert(
            "contract".into(),
            json!("arda.workbench.queue_reprioritization.v1"),
        );
        genuine.extra.insert(
            "reprioritized_from_contract".into(),
            json!("arda.workbench.queue_retry.v1"),
        );
        genuine
            .extra
            .insert("previous_priority".into(), json!("low"));
        genuine
            .extra
            .insert("operator_reason".into(), json!("operator escalation"));
        genuine.extra.insert(
            "reprioritized_at_utc".into(),
            json!(Utc::now().to_rfc3339()),
        );
        assert_eq!(
            TaskQueueAnalyzer::effective_records(vec![
                terminal.clone(),
                retry.clone(),
                genuine.clone()
            ])[0]
                .priority
                .as_deref(),
            Some("high")
        );

        let mut forged = Vec::new();
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.title = Some("Forged title".into());
        forged.push(("title", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.owner = Some("forged-owner".into());
        forged.push(("owner", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.status = Some("in_progress".into());
        forged.push(("status", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.result = Some("forged-result".into());
        forged.push(("result", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.queued_at_utc = Some("2026-08-28T13:00:00Z".into());
        forged.push(("timestamp", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.extra["meta"]["approval_packet_id"] = json!("forged-approval");
        forged.push(("approval lineage", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.extra["meta"]["source_objective_packet_id"] = json!("forged-objective");
        forged.push(("objective lineage", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.extra.insert("previous_priority".into(), json!("low"));
        forged.push(("previous priority", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.extra.insert("operator_reason".into(), json!("   "));
        forged.push(("operator reason", row));
        let mut row = genuine.clone();
        row.priority = Some("critical".into());
        row.extra
            .insert("reprioritized_at_utc".into(), json!("not-a-timestamp"));
        forged.push(("reprioritized timestamp", row));
        let mut row = genuine.clone();
        row.priority = Some("urgent".into());
        row.extra.insert("previous_priority".into(), json!("high"));
        forged.push(("canonical priority", row));

        for (field, forged) in forged {
            let effective = TaskQueueAnalyzer::effective_records(vec![
                terminal.clone(),
                retry.clone(),
                genuine.clone(),
                forged,
            ]);
            assert_eq!(effective.len(), 1, "{field}");
            assert_eq!(effective[0].title, genuine.title, "{field}");
            assert_eq!(effective[0].owner, genuine.owner, "{field}");
            assert_eq!(effective[0].status, genuine.status, "{field}");
            assert_eq!(effective[0].result, genuine.result, "{field}");
            assert_eq!(effective[0].priority, genuine.priority, "{field}");
            assert_eq!(effective[0].extra, genuine.extra, "{field}");
        }
    }

    #[test]
    fn effective_records_reject_executable_continuation_with_forged_meta_lineage() {
        let mut terminal = QueueRecord {
            id: "terminal".into(),
            title: Some("Canonical title".into()),
            owner: Some("prometheus".into()),
            priority: Some("high".into()),
            status: Some("failed".into()),
            result: Some("failed".into()),
            queued_at_utc: Some("2026-08-28T12:00:00Z".into()),
            completed_at_utc: Some("2026-08-28T12:01:00Z".into()),
            started_at_utc: Some("2026-08-28T12:00:30Z".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
        };
        terminal
            .extra
            .insert("source_record_id".into(), json!("terminal"));
        terminal
            .extra
            .insert("workbench_run_id".into(), json!("queue-terminal"));
        let mut meta = terminal.extra["meta"].as_object().unwrap().clone();
        meta.insert("continuation_decision".into(), json!("retry_same_task"));
        meta.insert("continuation_sequence".into(), json!(1));
        meta.insert("retry_sequence".into(), json!(1));
        meta.insert("revision_sequence".into(), json!(0));
        meta.insert("revision_directive".into(), json!("retry safely"));
        meta.insert("approval_packet_id".into(), json!("forged-approval"));
        let continuation = QueueRecord {
            status: Some("queued".into()),
            result: None,
            queued_at_utc: Some("2026-08-28T12:02:00Z".into()),
            completed_at_utc: None,
            started_at_utc: None,
            extra: serde_json::Map::from_iter([
                (
                    "contract".into(),
                    json!("arda.workbench.executable_continuation.v1"),
                ),
                ("source_record_id".into(), json!("terminal")),
                ("continuation_decision".into(), json!("retry_same_task")),
                ("continuation_sequence".into(), json!(1)),
                ("retry_sequence".into(), json!(1)),
                ("revision_sequence".into(), json!(0)),
                ("parent_workbench_run_id".into(), json!("queue-terminal")),
                ("workbench_run_id".into(), json!("queue-terminal-attempt-1")),
                ("revision_directive".into(), json!("retry safely")),
                ("meta".into(), Value::Object(meta)),
            ]),
            ..terminal.clone()
        };

        let effective =
            TaskQueueAnalyzer::effective_records(vec![terminal.clone(), continuation.clone()]);

        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].status, terminal.status);
        assert_eq!(effective[0].result, terminal.result);
        assert_eq!(effective[0].extra, terminal.extra);

        for (decision, status, forged_revision) in [
            ("retry_same_task", "queued", 1),
            ("wait_until", "blocked", 1),
            ("revise_task", "queued", 0),
        ] {
            let mut forged = continuation.clone();
            forged.status = Some(status.into());
            forged.extra["continuation_decision"] = json!(decision);
            forged.extra["revision_sequence"] = json!(forged_revision);
            forged.extra["meta"]["continuation_decision"] = json!(decision);
            forged.extra["meta"]["revision_sequence"] = json!(forged_revision);
            forged.extra["meta"]["approval_packet_id"] =
                terminal.extra["meta"]["approval_packet_id"].clone();

            let effective = TaskQueueAnalyzer::effective_records(vec![terminal.clone(), forged]);

            assert_eq!(effective.len(), 1, "{decision}");
            assert_eq!(effective[0].status, terminal.status, "{decision}");
            assert_eq!(effective[0].result, terminal.result, "{decision}");
            assert_eq!(effective[0].extra, terminal.extra, "{decision}");
        }
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
    fn approved_claim_allows_two_read_only_tasks_for_the_same_project() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "project-a-read-first".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "read_only",
            ),
            ..blank("project-a-read-first")
        };
        let second = QueueRecord {
            id: "project-a-read-second".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "read_only",
            ),
            ..blank("project-a-read-second")
        };
        let third = QueueRecord {
            id: "project-a-read-third".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "read_only",
            ),
            ..blank("project-a-read-third")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap(),
                serde_json::to_string(&third).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&first, &second, &third] {
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
                .expect("first read-only claim")
                .task
                .id,
            first.id
        );
        assert_eq!(
            executor
                .claim_next_approved()
                .unwrap()
                .expect("second read-only claim")
                .task
                .id,
            second.id
        );
        assert!(
            executor.claim_next_approved().unwrap().is_none(),
            "the queue layer must enforce the same two-reader bound as workspace slots"
        );
        assert_eq!(
            executor.append_attempt(&third).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied,
            "direct append must not bypass the queue-layer reader bound"
        );
    }

    #[test]
    fn read_only_execution_authority_is_byte_exact() {
        let canonical = QueueRecord {
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "read_only",
            ),
            ..blank("canonical-read-only")
        };
        let padded = QueueRecord {
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                " read_only ",
            ),
            ..blank("padded-read-only")
        };

        assert!(has_read_only_execution_authority(&canonical));
        assert!(
            !has_read_only_execution_authority(&padded),
            "non-canonical authority values must remain mutation-exclusive"
        );
    }

    #[test]
    fn decomposable_objective_is_never_read_only_execution_authority() {
        let mut objective = QueueRecord {
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "read_only",
            ),
            ..blank("decomposable-objective")
        };
        let meta = objective
            .extra
            .get_mut("meta")
            .and_then(Value::as_object_mut)
            .expect("approved metadata object");
        meta.insert("acceptance_artifact".into(), json!("artifact.json"));
        meta.insert("acceptance_markers".into(), json!(["complete"]));

        assert!(
            !has_read_only_execution_authority(&objective),
            "objective decomposition materializes queue rows and must remain mutation-exclusive"
        );
    }

    #[test]
    fn approved_claim_blocks_mutation_while_read_only_task_is_active() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let read_only = QueueRecord {
            id: "project-a-read".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "read_only",
            ),
            ..blank("project-a-read")
        };
        let mutation = QueueRecord {
            id: "project-a-mutation".into(),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target_with_authority(
                "project-a",
                "/worktrees/project-a",
                "execute_with_approval",
            ),
            ..blank("project-a-mutation")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&read_only).unwrap(),
                serde_json::to_string(&mutation).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(&active_path, "{\"active\":[]}").unwrap();
        for task in [&read_only, &mutation] {
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
            .expect("read-only claim");
        assert!(executor.claim_next_approved().unwrap().is_none());
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

    #[test]
    fn operator_reprioritization_appends_and_changes_approved_selection_order() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let earlier = QueueRecord {
            id: "earlier-medium".into(),
            title: Some("Earlier medium task".into()),
            owner: Some("prometheus".into()),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("earlier-medium")
        };
        let later = QueueRecord {
            id: "later-medium".into(),
            title: Some("Later medium task".into()),
            owner: Some("prometheus".into()),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            result: Some("awaiting_dispatch".into()),
            queued_at_utc: Some("2026-08-28T12:00:00Z".into()),
            completed_at_utc: Some("2026-08-28T12:01:00Z".into()),
            started_at_utc: Some("2026-08-28T12:00:30Z".into()),
            extra: {
                let mut extra = approved_workbench_extra_for_target("project-b", "/worktrees/b");
                extra.insert("source_record_id".into(), json!("later-medium-source"));
                extra.insert("objective_id".into(), json!("objective-1"));
                extra
            },
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&earlier).unwrap(),
                serde_json::to_string(&later).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        for task in [&earlier, &later] {
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

        let appended = executor
            .reprioritize(
                "later-medium",
                "objective-1",
                "critical",
                "operator raised urgency",
            )
            .expect("reprioritize canonical task");

        assert_eq!(appended.id, later.id);
        assert_eq!(appended.title, later.title);
        assert_eq!(appended.owner, later.owner);
        assert_eq!(appended.status, later.status);
        assert_eq!(appended.result, later.result);
        assert_eq!(appended.queued_at_utc, later.queued_at_utc);
        assert_eq!(appended.completed_at_utc, later.completed_at_utc);
        assert_eq!(appended.started_at_utc, later.started_at_utc);
        assert_eq!(appended.priority.as_deref(), Some("critical"));
        assert_eq!(appended.extra.get("meta"), later.extra.get("meta"));
        assert_eq!(
            appended.extra.get("source_record_id"),
            later.extra.get("source_record_id")
        );
        assert_eq!(
            appended.extra.get("objective_id"),
            later.extra.get("objective_id")
        );
        assert_eq!(
            appended.extra.get("contract").and_then(Value::as_str),
            Some("arda.workbench.queue_reprioritization.v1")
        );
        assert_eq!(
            appended
                .extra
                .get("operator_reason")
                .and_then(Value::as_str),
            Some("operator raised urgency")
        );
        assert_eq!(
            appended
                .extra
                .get("previous_priority")
                .and_then(Value::as_str),
            Some("medium")
        );
        assert_eq!(
            executor
                .select_next_approved()
                .unwrap()
                .expect("approved selection")
                .id,
            later.id
        );
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .count(),
            3
        );
    }

    #[test]
    fn reprioritized_authorized_reopens_remain_effective_after_terminal_alias() {
        let contracts = [
            "arda.workbench.queue_retry.v1",
            "arda.workbench.queue_continuation.v1",
            "arda.workbench.executable_continuation.v1",
            "arda.schedule.queue_activation.v1",
        ];
        for (index, contract) in contracts.into_iter().enumerate() {
            let dir = tempfile::tempdir().expect("create tempdir");
            let queue_path = dir.path().join("queue.jsonl");
            let active_path = dir.path().join("queue_active.json");
            let task_id = format!("reopened-task-{index}");
            let mut predecessor = QueueRecord {
                id: task_id.clone(),
                priority: Some("low".into()),
                status: Some("failed".into()),
                result: Some("failed".into()),
                extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
                ..blank(&task_id)
            };
            predecessor
                .extra
                .insert("source_record_id".into(), json!(predecessor.id));
            let mut ledger_records = Vec::new();
            let mut reopen_extra = predecessor.extra.clone();
            reopen_extra.insert("contract".into(), json!(contract));
            let mut reopened = QueueRecord {
                status: Some("queued".into()),
                result: None,
                extra: reopen_extra,
                ..predecessor.clone()
            };
            match contract {
                "arda.workbench.queue_retry.v1" => {
                    reopened
                        .extra
                        .insert("executor".into(), json!("arda_workbench.queue_executor"));
                    reopened.extra.insert("retry_sequence".into(), json!(1));
                    reopened
                        .extra
                        .insert("retried_at_utc".into(), json!(Utc::now().to_rfc3339()));
                }
                "arda.workbench.queue_continuation.v1" => {
                    ledger_records.push(QueueRecord {
                        status: Some("completed".into()),
                        result: Some("completed".into()),
                        ..predecessor.clone()
                    });
                    predecessor.status = Some("in_progress".into());
                    predecessor.result = None;
                    predecessor
                        .extra
                        .insert("workbench_run_id".into(), json!("queue-reopened-task"));
                    reopened.status = Some("in_progress".into());
                    reopened.extra = predecessor.extra.clone();
                    reopened.extra.insert("contract".into(), json!(contract));
                    reopened
                        .extra
                        .insert("executor".into(), json!("arda_workbench.queue_executor"));
                    reopened
                        .extra
                        .insert("completed_stage".into(), json!("provider_dispatch"));
                    reopened
                        .extra
                        .insert("continuation_decision".into(), json!("continue"));
                    reopened
                        .extra
                        .insert("recorded_at_utc".into(), json!(Utc::now().to_rfc3339()));
                }
                "arda.workbench.executable_continuation.v1" => {
                    predecessor
                        .extra
                        .insert("workbench_run_id".into(), json!("queue-reopened-task"));
                    reopened.queued_at_utc = Some(Utc::now().to_rfc3339());
                    reopened.extra.insert(
                        "parent_workbench_run_id".into(),
                        json!("queue-reopened-task"),
                    );
                    reopened
                        .extra
                        .insert("continuation_decision".into(), json!("retry_same_task"));
                    reopened
                        .extra
                        .insert("continuation_sequence".into(), json!(1));
                    reopened.extra.insert("retry_sequence".into(), json!(1));
                    reopened.extra.insert("revision_sequence".into(), json!(0));
                    reopened
                        .extra
                        .insert("revision_directive".into(), json!("retry safely"));
                    reopened.extra.insert(
                        "workbench_run_id".into(),
                        json!("queue-reopened-task-attempt-1"),
                    );
                    reopened.extra["meta"]["continuation_decision"] = json!("retry_same_task");
                    reopened.extra["meta"]["continuation_sequence"] = json!(1);
                    reopened.extra["meta"]["retry_sequence"] = json!(1);
                    reopened.extra["meta"]["revision_sequence"] = json!(0);
                    reopened.extra["meta"]["revision_directive"] = json!("retry safely");
                }
                "arda.schedule.queue_activation.v1" => {
                    predecessor.status = Some("completed".into());
                    predecessor.result = Some("completed".into());
                    reopened
                        .extra
                        .insert("scheduled_for_utc".into(), json!(Utc::now().to_rfc3339()));
                    reopened
                        .extra
                        .insert("schedule_mode".into(), json!("recurring"));
                    reopened
                        .extra
                        .insert("recorded_at_utc".into(), json!(Utc::now().to_rfc3339()));
                }
                _ => unreachable!(),
            }
            ledger_records.push(predecessor);
            ledger_records.push(reopened.clone());
            std::fs::write(
                &queue_path,
                ledger_records
                    .iter()
                    .map(|record| serde_json::to_string(record).unwrap())
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n",
            )
            .expect("write queue fixture");
            std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
            append_test_schedule(
                &queue_path,
                &reopened.id,
                "objective-1",
                ScheduleMode::Immediate,
                ScheduleState::Scheduled,
                None,
            );
            let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

            executor
                .reprioritize(
                    &reopened.id,
                    "objective-1",
                    "critical",
                    "reopened work now blocks the objective",
                )
                .expect("reprioritize authorized reopen");
            executor
                .reprioritize(
                    &reopened.id,
                    "objective-1",
                    "high",
                    "repeated operator priority adjustment",
                )
                .expect("reprioritize authorized reopen again");
            let effective = TaskQueueAnalyzer::effective_records(
                TaskQueueAnalyzer::new(&queue_path)
                    .load()
                    .expect("load queue"),
            );
            let current = effective
                .iter()
                .find(|record| record.id == reopened.id)
                .expect("reopened task remains effective");

            assert_eq!(current.priority.as_deref(), Some("high"), "{contract}");
            assert_eq!(
                current.extra.get("contract").and_then(Value::as_str),
                Some("arda.workbench.queue_reprioritization.v1"),
                "{contract}"
            );
            assert_eq!(
                current
                    .extra
                    .get("reprioritized_from_contract")
                    .and_then(Value::as_str),
                Some(contract),
                "{contract}"
            );
        }
    }

    #[test]
    fn dispatch_priority_order_covers_all_buckets_and_legacy_tail() {
        let records = vec![
            QueueRecord {
                id: "missing".into(),
                ..blank("missing")
            },
            QueueRecord {
                id: "low".into(),
                priority: Some("low".into()),
                ..blank("low")
            },
            QueueRecord {
                id: "medium".into(),
                priority: Some("medium".into()),
                ..blank("medium")
            },
            QueueRecord {
                id: "unknown".into(),
                priority: Some("urgent".into()),
                ..blank("unknown")
            },
            QueueRecord {
                id: "high".into(),
                priority: Some("high".into()),
                ..blank("high")
            },
            QueueRecord {
                id: "critical".into(),
                priority: Some("critical".into()),
                ..blank("critical")
            },
        ];

        let ids = dispatch_priority_order(records)
            .into_iter()
            .map(|record| record.id)
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["critical", "high", "medium", "low", "missing", "unknown"]
        );
    }

    #[test]
    fn equal_priority_selection_and_claim_preserve_effective_ledger_order() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "equal-first".into(),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("equal-first")
        };
        let second = QueueRecord {
            id: "equal-second".into(),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/b"),
            ..blank("equal-second")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
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
                .select_next_approved()
                .unwrap()
                .expect("approved selection")
                .id,
            first.id
        );
        assert_eq!(
            executor
                .claim_next_approved()
                .unwrap()
                .expect("approved claim")
                .task
                .id,
            first.id
        );
    }

    #[test]
    fn candidate_claim_rejects_stale_priority_selection_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let first = QueueRecord {
            id: "stale-first".into(),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("stale-first")
        };
        let second = QueueRecord {
            id: "fresh-critical".into(),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/b"),
            ..blank("fresh-critical")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
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
        let stale = executor
            .next_approved_reconciling_orphans_excluding(&BTreeSet::new())
            .unwrap()
            .expect("initial candidate");
        assert_eq!(stale.id, first.id);
        executor
            .reprioritize(
                &second.id,
                "objective-1",
                "critical",
                "priority changed while target lock was acquired",
            )
            .expect("reprioritize competing candidate");
        let before_claim = std::fs::read(&queue_path).expect("read queue before stale claim");

        assert!(executor
            .claim_approved_candidate(&stale, &BTreeSet::new())
            .unwrap()
            .is_none());
        assert_eq!(std::fs::read(&queue_path).unwrap(), before_claim);
    }

    #[test]
    fn candidate_claim_rejects_same_id_target_replacement_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let original = QueueRecord {
            id: "same-id-replaced-target".into(),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("same-id-replaced-target")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&original).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &original.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let selected = executor
            .next_approved_reconciling_orphans_excluding(&BTreeSet::new())
            .unwrap()
            .expect("initial candidate");
        assert_eq!(
            serde_json::to_value(&selected).unwrap(),
            serde_json::to_value(&original).unwrap()
        );

        let replacement = QueueRecord {
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/b"),
            ..original.clone()
        };
        let mut queue = OpenOptions::new().append(true).open(&queue_path).unwrap();
        writeln!(queue, "{}", serde_json::to_string(&replacement).unwrap()).unwrap();
        queue.sync_data().unwrap();
        let before_claim = std::fs::read(&queue_path).expect("read queue before stale claim");

        assert!(executor
            .claim_approved_candidate(&selected, &BTreeSet::new())
            .unwrap()
            .is_none());
        assert_eq!(std::fs::read(&queue_path).unwrap(), before_claim);
    }

    #[test]
    fn approved_atomic_claim_uses_priority_order() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let low = QueueRecord {
            id: "earlier-low".into(),
            priority: Some("low".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("earlier-low")
        };
        let high = QueueRecord {
            id: "later-high".into(),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/b"),
            ..blank("later-high")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&low).unwrap(),
                serde_json::to_string(&high).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        for task in [&low, &high] {
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

        let claim = executor
            .claim_next_approved()
            .unwrap()
            .expect("approved claim");

        assert_eq!(claim.task.id, high.id);
    }

    #[test]
    fn orphan_aware_selection_uses_priority_order() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let low = QueueRecord {
            id: "orphan-earlier-low".into(),
            priority: Some("low".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("orphan-earlier-low")
        };
        let high = QueueRecord {
            id: "orphan-later-high".into(),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/b"),
            ..blank("orphan-later-high")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&low).unwrap(),
                serde_json::to_string(&high).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        for task in [&low, &high] {
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

        let selected = executor
            .next_approved_reconciling_orphans_excluding(&BTreeSet::new())
            .unwrap()
            .expect("approved candidate");

        assert_eq!(selected.id, high.id);
    }

    #[test]
    fn orphan_aware_atomic_claim_uses_priority_order() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let low = QueueRecord {
            id: "orphan-claim-earlier-low".into(),
            priority: Some("low".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("orphan-claim-earlier-low")
        };
        let high = QueueRecord {
            id: "orphan-claim-later-high".into(),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-b", "/worktrees/b"),
            ..blank("orphan-claim-later-high")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&low).unwrap(),
                serde_json::to_string(&high).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        for task in [&low, &high] {
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

        let claim = executor
            .claim_next_approved_reconciling_orphans_excluding(&BTreeSet::new())
            .unwrap()
            .expect("approved claim");

        assert_eq!(claim.task.id, high.id);
    }

    #[test]
    fn operator_reprioritization_rejects_wrong_objective_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "objective-bound-task".into(),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("objective-bound-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        let before = std::fs::read(&queue_path).expect("read original ledger");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let error = executor
            .reprioritize(
                &task.id,
                "wrong-objective",
                "critical",
                "unauthorized priority change",
            )
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
    }

    #[test]
    fn operator_reprioritization_rejects_unknown_priority_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "priority-bound-task".into(),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("priority-bound-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        let before = std::fs::read(&queue_path).expect("read original ledger");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let error = executor
            .reprioritize(&task.id, "objective-1", "urgent", "unknown priority")
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
    }

    #[test]
    fn operator_reprioritization_rejects_terminal_task_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "completed-priority-task".into(),
            priority: Some("medium".into()),
            status: Some("completed".into()),
            result: Some("completed".into()),
            extra: approved_workbench_extra(),
            ..blank("completed-priority-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        let before = std::fs::read(&queue_path).expect("read original ledger");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let error = executor
            .reprioritize(
                &task.id,
                "objective-1",
                "critical",
                "terminal task must remain immutable",
            )
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
    }

    #[test]
    fn operator_reprioritization_requires_reason_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "reason-bound-task".into(),
            priority: Some("medium".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("reason-bound-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        let before = std::fs::read(&queue_path).expect("read original ledger");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let error = executor
            .reprioritize(&task.id, "objective-1", "critical", "   ")
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
    }

    #[test]
    fn operator_reprioritization_rejects_existing_priority_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "already-critical-task".into(),
            priority: Some("critical".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra(),
            ..blank("already-critical-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        let before = std::fs::read(&queue_path).expect("read original ledger");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        let error = executor
            .reprioritize(
                &task.id,
                "objective-1",
                "critical",
                "priority is already critical",
            )
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
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

    fn approved_workbench_extra_for_target_with_authority(
        project_id: &str,
        worktree_path: &str,
        authority_class: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut extra = approved_workbench_extra_for_target(project_id, worktree_path);
        extra
            .get_mut("meta")
            .and_then(Value::as_object_mut)
            .expect("approved metadata object")
            .insert("authority_class".into(), json!(authority_class));
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

    #[test]
    fn operator_objective_revision_invalidates_stale_approval_before_dispatch() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let mut task = QueueRecord {
            id: "revision-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("revision-task")
        };
        task.extra
            .insert("source_record_id".into(), json!("revision-task"));
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
        assert!(executor.select_next_approved().unwrap().is_some());

        let revised = executor
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the intended outcome",
            )
            .expect("append objective revision");

        assert_eq!(revised.title.as_deref(), Some("Revised operator objective"));
        assert_eq!(
            revised.extra.get("contract").and_then(Value::as_str),
            Some("arda.workbench.objective_revision.v1")
        );
        assert_eq!(
            revised
                .extra
                .get("previous_objective")
                .and_then(Value::as_str),
            Some("Original operator objective")
        );
        let meta = revised.extra["meta"].as_object().expect("revision meta");
        assert_eq!(
            meta.get("mutation_risk").and_then(Value::as_str),
            Some("operator-revision-pending")
        );
        assert!(meta.get("approval_packet_id").is_none());
        assert!(executor.select_next_approved().unwrap().is_none());
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .expect("queue ledger")
                .lines()
                .count(),
            2
        );
    }

    #[test]
    fn ordinary_record_after_objective_revision_cannot_restore_or_block_fresh_approval() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "stale-approval-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("stale-approval-task")
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
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the intended outcome",
            )
            .expect("append objective revision");

        let mut stale = task;
        stale.title = Some("Revised operator objective".into());
        let mut file = OpenOptions::new()
            .append(true)
            .open(&queue_path)
            .expect("open queue");
        serde_json::to_writer(&mut file, &stale).expect("write stale approval copy");
        writeln!(file).expect("terminate stale approval record");

        assert!(
            executor
                .select_next_approved()
                .expect("select approved task")
                .is_none(),
            "ordinary post-revision record restored stale approval"
        );
        executor
            .approve_revised_objective(
                "stale-approval-task",
                "objective-1",
                "approval-2",
                "operator@example.test",
                "fresh approval after ignored stale successor",
            )
            .expect("ordinary stale successor must not block fresh approval");
        let approved = executor
            .select_next_approved()
            .expect("select freshly approved revision")
            .expect("freshly approved revision dispatches");
        assert_eq!(
            approved.title.as_deref(),
            Some("Revised operator objective")
        );
    }

    #[test]
    fn terminal_record_after_objective_revision_cannot_block_fresh_approval() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "terminal-successor-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("terminal-successor-task")
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
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the intended outcome",
            )
            .expect("append objective revision");

        let mut terminal = task;
        terminal.status = Some("completed".into());
        terminal.result = Some("completed".into());
        let mut file = OpenOptions::new()
            .append(true)
            .open(&queue_path)
            .expect("open queue");
        serde_json::to_writer(&mut file, &terminal).expect("write stale terminal successor");
        writeln!(file).expect("terminate stale terminal successor");

        executor
            .approve_revised_objective(
                "terminal-successor-task",
                "objective-1",
                "approval-2",
                "operator@example.test",
                "fresh approval after ignored terminal successor",
            )
            .expect("terminal successor must not block fresh approval");
        let approved = executor
            .select_next_approved()
            .expect("select freshly approved revision")
            .expect("freshly approved revision dispatches");
        assert_eq!(
            approved.title.as_deref(),
            Some("Revised operator objective")
        );
    }

    #[test]
    fn invalid_operator_objective_revision_rejects_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "revision-task".into(),
            title: Some("Original operator objective".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("revision-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        for (objective_id, revised_objective, reason) in [
            (
                "wrong-objective",
                "Revised objective",
                "operator correction",
            ),
            ("objective-1", "", "operator correction"),
            ("objective-1", "Revised objective", ""),
            (
                "objective-1",
                "Original operator objective",
                "operator correction",
            ),
        ] {
            assert!(
                executor
                    .revise_objective(&task.id, objective_id, revised_objective, reason,)
                    .is_err(),
                "accepted invalid revision ({objective_id:?}, {revised_objective:?}, {reason:?})"
            );
            assert_eq!(
                std::fs::read_to_string(&queue_path)
                    .expect("queue ledger")
                    .lines()
                    .count(),
                1,
                "invalid revision appended a queue record"
            );
        }
    }

    #[test]
    fn terminal_task_objective_revision_rejects_without_append() {
        for status in ["completed", "failed", "cancelled"] {
            let dir = tempfile::tempdir().expect("create tempdir");
            let queue_path = dir.path().join("queue.jsonl");
            let active_path = dir.path().join("queue_active.json");
            let task = QueueRecord {
                id: format!("revision-{status}"),
                title: Some("Original operator objective".into()),
                status: Some(status.into()),
                result: Some(status.into()),
                extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
                ..blank("terminal-revision-task")
            };
            std::fs::write(
                &queue_path,
                format!("{}\n", serde_json::to_string(&task).unwrap()),
            )
            .expect("write queue fixture");
            std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
            let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

            assert!(
                executor
                    .revise_objective(
                        &task.id,
                        "objective-1",
                        "Revised operator objective",
                        "operator correction",
                    )
                    .is_err(),
                "accepted terminal {status} revision"
            );
            assert_eq!(
                std::fs::read_to_string(&queue_path)
                    .expect("queue ledger")
                    .lines()
                    .count(),
                1,
                "terminal {status} revision appended"
            );
        }
    }

    #[test]
    fn forged_objective_revision_cannot_reuse_prior_approval() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "forged-revision-task".into(),
            title: Some("Approved operator objective".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("forged-revision-task")
        };
        let mut forged = task.clone();
        forged.title = Some("Unapproved forged objective".into());
        forged.extra.insert(
            "contract".into(),
            json!("arda.workbench.objective_revision.v1"),
        );
        forged.extra.insert(
            "previous_objective".into(),
            json!("Approved operator objective"),
        );
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&task).unwrap(),
                serde_json::to_string(&forged).unwrap()
            ),
        )
        .expect("write forged queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        assert!(
            ActiveQueueExecutor::with_paths(&queue_path, &active_path)
                .select_next_approved()
                .expect("select approved task")
                .is_none(),
            "forged objective revision reused stale approval"
        );
    }

    #[test]
    fn fresh_operator_approval_releases_revised_objective_for_dispatch() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "approval-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("approval-task")
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
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the outcome",
            )
            .expect("revise objective");
        assert!(executor.select_next_approved().unwrap().is_none());

        let approved = executor
            .approve_revised_objective(
                &task.id,
                "objective-1",
                "revision-approval-2",
                "operator@example.test",
                "revised objective accepted",
            )
            .expect("approve revision");

        assert_eq!(
            approved.title.as_deref(),
            Some("Revised operator objective")
        );
        assert_eq!(
            approved.extra.get("contract").and_then(Value::as_str),
            Some("arda.workbench.objective_revision_approval.v1")
        );
        assert_eq!(
            approved.extra["meta"]["approval_packet_id"].as_str(),
            Some("revision-approval-2")
        );
        let selected = executor
            .select_next_approved()
            .expect("select approved task")
            .expect("revised task is dispatchable");
        assert_eq!(selected.id, task.id);
        assert_eq!(selected.title, approved.title);
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .expect("queue ledger")
                .lines()
                .count(),
            3
        );
    }

    #[test]
    fn stale_approval_packet_cannot_approve_revised_objective() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "stale-revision-approval-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("stale-revision-approval-task")
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
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the outcome",
            )
            .expect("revise objective");

        assert!(
            executor
                .approve_revised_objective(
                    &task.id,
                    "objective-1",
                    "approval-1",
                    "operator@example.test",
                    "attempted stale approval reuse",
                )
                .is_err(),
            "pre-revision approval packet authorized the revised objective"
        );
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .expect("queue ledger")
                .lines()
                .count(),
            2,
            "stale approval rejection appended a queue record"
        );
        assert!(executor.select_next_approved().unwrap().is_none());
    }

    #[test]
    fn approval_packet_must_be_globally_fresh_across_tasks() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let mut unrelated_extra = approved_workbench_extra_for_target("project-a", "/worktrees/a");
        unrelated_extra["meta"]["approval_packet_id"] = Value::String("approval-global".into());
        let unrelated = QueueRecord {
            id: "unrelated-task".into(),
            title: Some("Unrelated approved objective".into()),
            status: Some("queued".into()),
            extra: unrelated_extra,
            ..blank("unrelated-task")
        };
        let mut task_extra = approved_workbench_extra_for_target("project-b", "/worktrees/b");
        task_extra["meta"]["approval_packet_id"] = Value::String("approval-b".into());
        let task = QueueRecord {
            id: "global-packet-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: task_extra,
            ..blank("global-packet-task")
        };
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&unrelated).unwrap(),
                serde_json::to_string(&task).unwrap()
            ),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        executor
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the outcome",
            )
            .expect("revise objective");

        assert!(
            executor
                .approve_revised_objective(
                    &task.id,
                    "objective-1",
                    " approval-global ",
                    "operator@example.test",
                    "attempted cross-task packet reuse",
                )
                .is_err(),
            "approval packet reused by another task authorized the revision"
        );
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .expect("queue ledger")
                .lines()
                .count(),
            3,
            "globally stale approval rejection appended a queue record"
        );
    }

    #[test]
    fn objective_revision_approval_packet_used_by_another_task_after_revision_is_rejected() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "interleaved-packet-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("interleaved-packet-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        executor
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the outcome",
            )
            .expect("revise objective");
        let mut unrelated_extra = approved_workbench_extra_for_target("project-b", "/worktrees/b");
        unrelated_extra["meta"]["approval_packet_id"] =
            Value::String("approval-interleaved".into());
        let unrelated = QueueRecord {
            id: "interleaved-unrelated-task".into(),
            title: Some("Unrelated approved objective".into()),
            status: Some("queued".into()),
            extra: unrelated_extra,
            ..blank("interleaved-unrelated-task")
        };
        let mut ledger = std::fs::read_to_string(&queue_path).expect("queue ledger");
        ledger.push_str(&serde_json::to_string(&unrelated).unwrap());
        ledger.push('\n');
        std::fs::write(&queue_path, ledger).expect("append unrelated approval");

        assert!(
            executor
                .approve_revised_objective(
                    &task.id,
                    "objective-1",
                    " approval-interleaved ",
                    "operator@example.test",
                    "attempted interleaved packet reuse",
                )
                .is_err(),
            "approval packet used after revision by another task authorized the revision"
        );
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .expect("queue ledger")
                .lines()
                .count(),
            3,
            "interleaved stale approval rejection appended a queue record"
        );
    }

    #[test]
    fn objective_revision_replay_rejects_packet_used_after_revision_before_approval() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "interleaved-replay-task".into(),
            title: Some("Original operator objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("interleaved-replay-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let revision = executor
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised operator objective",
                "operator corrected the outcome",
            )
            .expect("revise objective");
        let mut unrelated_extra = approved_workbench_extra_for_target("project-b", "/worktrees/b");
        unrelated_extra["meta"]["approval_packet_id"] =
            Value::String("approval-interleaved".into());
        let unrelated = QueueRecord {
            id: "interleaved-replay-unrelated".into(),
            title: Some("Unrelated approved objective".into()),
            status: Some("queued".into()),
            extra: unrelated_extra,
            ..blank("interleaved-replay-unrelated")
        };
        let mut forged_approval = revision.clone();
        forged_approval.extra["meta"]["mutation_risk"] = Value::String("operator-approved".into());
        forged_approval.extra["meta"]["action_class"] =
            Value::String("approved_autopilot_plan_step".into());
        forged_approval.extra["meta"]["approval_packet_id"] =
            Value::String("approval-interleaved".into());
        forged_approval.extra.insert(
            "contract".into(),
            Value::String("arda.workbench.objective_revision_approval.v1".into()),
        );
        forged_approval.extra.insert(
            "reviewed_by".into(),
            Value::String("operator@example.test".into()),
        );
        forged_approval.extra.insert(
            "operator_reason".into(),
            Value::String("forged interleaved approval".into()),
        );
        forged_approval.extra.insert(
            "objective_approved_at_utc".into(),
            Value::String("2030-01-01T00:01:00Z".into()),
        );
        let mut ledger = std::fs::read_to_string(&queue_path).expect("queue ledger");
        ledger.push_str(&serde_json::to_string(&unrelated).unwrap());
        ledger.push('\n');
        ledger.push_str(&serde_json::to_string(&forged_approval).unwrap());
        ledger.push('\n');
        std::fs::write(&queue_path, ledger).expect("append forged approval chain");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        assert!(
            executor
                .select_next_approved()
                .expect("select task")
                .is_none(),
            "replay accepted packet used after revision before its approval"
        );
    }

    #[test]
    fn non_revision_task_approval_rejects_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "invalid-approval-task".into(),
            title: Some("Already approved objective".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("invalid-approval-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);

        assert!(
            executor
                .approve_revised_objective(
                    &task.id,
                    "objective-1",
                    "approval-2",
                    "operator@example.test",
                    "fresh approval",
                )
                .is_err(),
            "approved a task that was not revision-pending"
        );
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .expect("queue ledger")
                .lines()
                .count(),
            1
        );
    }

    #[test]
    fn blank_revision_approval_fields_reject_without_append() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "blank-approval-task".into(),
            title: Some("Original objective".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("blank-approval-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        executor
            .revise_objective(
                &task.id,
                "objective-1",
                "Revised objective",
                "operator correction",
            )
            .expect("revise objective");

        for (approval, reviewer, reason) in [
            ("", "operator@example.test", "fresh approval"),
            ("approval-2", "", "fresh approval"),
            ("approval-2", "operator@example.test", ""),
        ] {
            assert!(
                executor
                    .approve_revised_objective(&task.id, "objective-1", approval, reviewer, reason,)
                    .is_err(),
                "accepted blank revision approval field"
            );
            assert_eq!(
                std::fs::read_to_string(&queue_path)
                    .expect("queue ledger")
                    .lines()
                    .count(),
                2
            );
        }
    }

    #[test]
    fn forged_revision_approval_without_pending_predecessor_is_not_dispatchable() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "forged-approval-task".into(),
            title: Some("Original approved objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("forged-approval-task")
        };
        let mut forged = task.clone();
        forged.title = Some("Forged revised objective".into());
        forged.extra.insert(
            "contract".into(),
            Value::String("arda.workbench.objective_revision_approval.v1".into()),
        );
        forged.extra.insert(
            "reviewed_by".into(),
            Value::String("attacker@example.test".into()),
        );
        forged.extra.insert(
            "operator_reason".into(),
            Value::String("forged approval".into()),
        );
        forged.extra.insert(
            "objective_approved_at_utc".into(),
            Value::String("2030-01-01T00:00:00Z".into()),
        );
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&task).unwrap(),
                serde_json::to_string(&forged).unwrap()
            ),
        )
        .expect("write forged queue fixture");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        assert!(
            ActiveQueueExecutor::with_paths(&queue_path, &active_path)
                .select_next_approved()
                .expect("select task")
                .is_none(),
            "forged revision approval was dispatchable"
        );
    }

    #[test]
    fn approval_after_forged_revision_metadata_is_not_dispatchable() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "forged-revision-task".into(),
            title: Some("Original approved objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("forged-revision-task")
        };
        let mut forged_revision = task.clone();
        forged_revision.title = Some("Forged revised objective".into());
        forged_revision.status = Some("pending".into());
        forged_revision.extra["meta"]["mutation_risk"] =
            Value::String("operator-revision-pending".into());
        forged_revision.extra["meta"]["physical_root"] =
            Value::String("/worktrees/attacker".into());
        forged_revision.extra["meta"]
            .as_object_mut()
            .unwrap()
            .remove("action_class");
        forged_revision.extra["meta"]
            .as_object_mut()
            .unwrap()
            .remove("approval_packet_id");
        forged_revision.extra.insert(
            "contract".into(),
            Value::String("arda.workbench.objective_revision.v1".into()),
        );
        forged_revision.extra.insert(
            "previous_objective".into(),
            Value::String("Original approved objective".into()),
        );
        forged_revision.extra.insert(
            "operator_reason".into(),
            Value::String("forged revision".into()),
        );
        forged_revision.extra.insert(
            "objective_revised_at_utc".into(),
            Value::String("2030-01-01T00:00:00Z".into()),
        );
        let mut forged_approval = forged_revision.clone();
        forged_approval.extra["meta"]["mutation_risk"] = Value::String("operator-approved".into());
        forged_approval.extra["meta"]["action_class"] =
            Value::String("approved_autopilot_plan_step".into());
        forged_approval.extra["meta"]["approval_packet_id"] = Value::String("approval-2".into());
        forged_approval.extra.insert(
            "contract".into(),
            Value::String("arda.workbench.objective_revision_approval.v1".into()),
        );
        forged_approval.extra.insert(
            "reviewed_by".into(),
            Value::String("operator@example.test".into()),
        );
        forged_approval
            .extra
            .insert("operator_reason".into(), Value::String("accepted".into()));
        forged_approval.extra.insert(
            "objective_approved_at_utc".into(),
            Value::String("2030-01-01T00:01:00Z".into()),
        );
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n{}\n",
                serde_json::to_string(&task).unwrap(),
                serde_json::to_string(&forged_revision).unwrap(),
                serde_json::to_string(&forged_approval).unwrap()
            ),
        )
        .expect("write forged revision chain");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );

        assert!(
            ActiveQueueExecutor::with_paths(&queue_path, &active_path)
                .select_next_approved()
                .expect("select task")
                .is_none(),
            "approval after forged revision metadata was dispatchable"
        );
    }

    #[test]
    fn forged_approval_metadata_after_valid_revision_is_not_dispatchable() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "forged-approval-meta-task".into(),
            title: Some("Original approved objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("forged-approval-meta-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write approved task");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let revision = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .revise_objective(
                &task.id,
                "objective-1",
                "Valid revised objective",
                "operator correction",
            )
            .expect("append valid revision");
        let mut forged_approval = revision;
        forged_approval.extra["meta"]["mutation_risk"] = Value::String("operator-approved".into());
        forged_approval.extra["meta"]["action_class"] =
            Value::String("approved_autopilot_plan_step".into());
        forged_approval.extra["meta"]["approval_packet_id"] = Value::String("approval-2".into());
        forged_approval.extra["meta"]["physical_root"] =
            Value::String("/worktrees/attacker".into());
        forged_approval.extra.insert(
            "contract".into(),
            Value::String("arda.workbench.objective_revision_approval.v1".into()),
        );
        forged_approval.extra.insert(
            "reviewed_by".into(),
            Value::String("operator@example.test".into()),
        );
        forged_approval
            .extra
            .insert("operator_reason".into(), Value::String("accepted".into()));
        forged_approval.extra.insert(
            "objective_approved_at_utc".into(),
            Value::String(Utc::now().to_rfc3339()),
        );
        let mut file = OpenOptions::new()
            .append(true)
            .open(&queue_path)
            .expect("open queue");
        serde_json::to_writer(&mut file, &forged_approval).expect("write forged approval");
        writeln!(file).expect("terminate forged approval");

        assert!(
            ActiveQueueExecutor::with_paths(&queue_path, &active_path)
                .select_next_approved()
                .expect("select task")
                .is_none(),
            "approval forged protected metadata after a valid revision"
        );
        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        executor
            .approve_revised_objective(
                &task.id,
                "objective-1",
                "approval-3",
                "operator@example.test",
                "fresh approval after ignored forged successor",
            )
            .expect("forged successor must not block fresh approval");
        let approved = executor
            .select_next_approved()
            .expect("select freshly approved revision")
            .expect("freshly approved revision dispatches");
        assert_eq!(approved.title.as_deref(), Some("Valid revised objective"));
    }

    #[test]
    fn replay_rejects_whitespace_disguised_stale_approval_packet() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        let active_path = dir.path().join("queue_active.json");
        let task = QueueRecord {
            id: "whitespace-stale-approval-task".into(),
            title: Some("Original approved objective".into()),
            priority: Some("high".into()),
            status: Some("queued".into()),
            extra: approved_workbench_extra_for_target("project-a", "/worktrees/a"),
            ..blank("whitespace-stale-approval-task")
        };
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&task).unwrap()),
        )
        .expect("write approved task");
        std::fs::write(&active_path, "{\"active\":[]}").expect("write projection");
        append_test_schedule(
            &queue_path,
            &task.id,
            "objective-1",
            ScheduleMode::Immediate,
            ScheduleState::Scheduled,
            None,
        );
        let revision = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .revise_objective(
                &task.id,
                "objective-1",
                "Valid revised objective",
                "operator correction",
            )
            .expect("append valid revision");
        let mut forged_approval = revision;
        forged_approval.extra["meta"]["mutation_risk"] = Value::String("operator-approved".into());
        forged_approval.extra["meta"]["action_class"] =
            Value::String("approved_autopilot_plan_step".into());
        forged_approval.extra["meta"]["approval_packet_id"] = Value::String(" approval-1 ".into());
        forged_approval.extra.insert(
            "contract".into(),
            Value::String("arda.workbench.objective_revision_approval.v1".into()),
        );
        forged_approval.extra.insert(
            "reviewed_by".into(),
            Value::String("operator@example.test".into()),
        );
        forged_approval
            .extra
            .insert("operator_reason".into(), Value::String("accepted".into()));
        forged_approval.extra.insert(
            "objective_approved_at_utc".into(),
            Value::String(Utc::now().to_rfc3339()),
        );
        let mut file = OpenOptions::new()
            .append(true)
            .open(&queue_path)
            .expect("open queue");
        serde_json::to_writer(&mut file, &forged_approval).expect("write forged approval");
        writeln!(file).expect("terminate forged approval");

        assert!(
            ActiveQueueExecutor::with_paths(&queue_path, &active_path)
                .select_next_approved()
                .expect("select task")
                .is_none(),
            "whitespace-disguised stale approval packet was dispatchable"
        );
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
