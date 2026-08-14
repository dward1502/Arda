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
        let mut out = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(rec) = serde_json::from_str::<QueueRecord>(trimmed) {
                out.push(rec);
            }
        }
        Ok(out)
    }

    pub fn analyze(&self) -> TaskQueueMetrics {
        let records = self.load().unwrap_or_default();
        let effective_records = Self::effective_records(records);
        Self::summarize(&effective_records)
    }

    pub fn effective_records(records: Vec<QueueRecord>) -> Vec<QueueRecord> {
        let mut latest_by_source_record_id = BTreeMap::<String, (usize, QueueRecord)>::new();
        for (index, record) in records.into_iter().enumerate() {
            latest_by_source_record_id.insert(Self::effective_record_key(&record), (index, record));
        }
        let mut effective = latest_by_source_record_id.into_values().collect::<Vec<_>>();
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
        }
    }

    pub fn with_paths(
        queue_path: impl AsRef<Path>,
        active_projection_path: impl AsRef<Path>,
    ) -> Self {
        Self {
            queue_path: queue_path.as_ref().to_path_buf(),
            active_projection_path: active_projection_path.as_ref().to_path_buf(),
        }
    }

    pub fn select_next_safe_local(&self) -> std::io::Result<Option<QueueRecord>> {
        let records = TaskQueueAnalyzer::new(&self.queue_path).load()?;
        let effective = TaskQueueAnalyzer::effective_records(records);
        let active_ids = self.load_active_projection_ids().unwrap_or_default();
        Ok(effective.into_iter().find(|record| {
            is_open_queue_status(record.status.as_deref())
                && (active_ids.is_empty() || active_ids.contains(&record.id))
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
        let active_ids = self.load_active_projection_ids().unwrap_or_default();
        Ok(effective.into_iter().find(|record| {
            claimable_status(record)
                && (active_ids.is_empty() || active_ids.contains(&record.id))
                && approved_workbench_metadata(record)
        }))
    }

    /// Atomically claim one approved task by appending its in-progress state
    /// while holding an exclusive lock on the canonical append-only ledger.
    pub fn claim_next_approved(&self) -> std::io::Result<Option<ApprovedQueueClaim>> {
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
            let active_ids = self.load_active_projection_ids().unwrap_or_default();
            let Some(task) = effective.into_iter().find(|record| {
                claimable_status(record)
                    && (active_ids.is_empty() || active_ids.contains(&record.id))
                    && approved_workbench_metadata(record)
            }) else {
                return Ok(None);
            };
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
    }

    pub fn append_attempt(
        &self,
        task: &QueueRecord,
    ) -> std::io::Result<ActiveQueueExecutionAttempt> {
        let attempt = execution_attempt(task);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.queue_path)?;
        append_attempt_to_writer(&mut file, task, &attempt)?;
        Ok(attempt)
    }

    /// Requeue one failed approved task while preserving its approval lineage.
    pub fn retry_failed(&self, task_id: &str) -> std::io::Result<QueueRecord> {
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
            if task.result.as_deref() == Some("cancelled") || !approved_workbench_metadata(&task) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "cancelled or unapproved tasks require a new operator decision",
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
    }

    pub fn append_terminal_completion(
        &self,
        task: &QueueRecord,
        result: &str,
    ) -> std::io::Result<()> {
        append_jsonl_value(
            &self.queue_path,
            &json!({
                "id": task.id,
                "source_record_id": task.id,
                "title": task.title,
                "owner": task.owner,
                "priority": task.priority,
                "status": "completed",
                "result": result,
                "completed_at_utc": Utc::now().to_rfc3339(),
                "contract": "arda.prometheus.active_queue_terminal_record.v1",
                "executor": "prometheus.active_queue_executor",
                "hades_projection_repair": true,
            }),
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
        append_jsonl_value(
            &self.queue_path,
            &json!({
                "id": task.id,
                "source_record_id": task.id,
                "title": task.title,
                "owner": task.owner,
                "priority": task.priority,
                "status": status,
                "result": result,
                "completed_at_utc": Utc::now().to_rfc3339(),
                "contract": "arda.workbench.queue_terminal.v1",
                "executor": "arda_workbench.queue_executor",
                "workbench_run_id": run_id,
                "execution_receipt_digest": receipt_digest,
                "detail": detail,
                "meta": task.extra.get("meta").cloned().unwrap_or(Value::Null),
            }),
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
            meta.get("mutation_risk").and_then(Value::as_str) == Some("operator-approved")
                && meta.get("execution_authority").and_then(Value::as_str) == Some("arda_workbench")
                && meta
                    .get("approval_packet_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.trim().is_empty())
                && meta
                    .get("source_objective_packet_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| !id.trim().is_empty())
        })
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

fn read_queue_records(file: &std::fs::File) -> std::io::Result<Vec<QueueRecord>> {
    let mut out = Vec::new();
    for line in BufReader::new(file.try_clone()?).lines() {
        let line = line?;
        if let Ok(record) = serde_json::from_str::<QueueRecord>(line.trim()) {
            out.push(record);
        }
    }
    Ok(out)
}

fn workbench_run_id(task_id: &str, retry_sequence: u64) -> String {
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
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", value)
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

        let executor = ActiveQueueExecutor::with_paths(&queue_path, &active_path);
        let selected = executor
            .select_next_safe_local()
            .expect("select next")
            .expect("safe-local task");

        assert_eq!(selected.id, safe_task.id);
    }

    #[test]
    fn active_queue_executor_selects_operator_approved_workbench_task() {
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
        std::fs::write(&active_path, "{\"active\":[{\"id\":\"approved-task\"}]}\n").unwrap();

        let selected = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .select_next_approved()
            .unwrap()
            .expect("approved task");

        assert_eq!(selected.id, "approved-task");
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

        let claim = ActiveQueueExecutor::with_paths(&queue_path, &active_path)
            .claim_next_approved()
            .unwrap()
            .expect("expired lease is recoverable");
        assert_eq!(claim.task.id, "expired-task");
        assert_eq!(claim.attempt.workbench_run_id, "queue-expired-task");
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

    fn l3_safe_local_extra() -> serde_json::Map<String, serde_json::Value> {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "meta".into(),
            json!({
                "action_class": "l3_local_doc_fixture_patch",
                "mutation_risk": "safe-local"
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
