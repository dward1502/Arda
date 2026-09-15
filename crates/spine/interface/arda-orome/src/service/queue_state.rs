use super::*;
use arda_core::error::Result;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, Clone)]
pub(super) struct QueueMutationResult {
    pub(super) found: bool,
    pub(super) updated: bool,
    pub(super) task_id: String,
    pub(super) title: String,
}

#[derive(Debug, Clone)]
pub(super) struct QueueDrainResult {
    pub(super) attempted: usize,
    pub(super) completed: Vec<QueueMutationResult>,
    pub(super) remaining: usize,
}

#[derive(Debug, Clone)]
pub(super) struct QueuedTaskEntry {
    pub(super) task_id: String,
    pub(super) title: Option<String>,
}

pub(super) fn default_task_queue_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_TASK_QUEUE_PATH") {
        return PathBuf::from(custom);
    }
    PathBuf::from("core/projects/tasks/queue.jsonl")
}

fn read_queue_values(path: &PathBuf) -> Result<Vec<Value>> {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) => out.push(v),
            Err(e) => warn!(line=%line, error=?e, "Failed to parse queue task JSON"),
        }
    }
    Ok(out)
}

fn entry_task_id(value: &Value) -> Option<&str> {
    value.get("task_id").and_then(|v| v.as_str())
}

fn entry_title(value: &Value) -> Option<String> {
    value
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn entry_status(value: &Value) -> Option<&str> {
    value.get("status").and_then(|v| v.as_str())
}

impl HermesService {
    pub(super) fn load_queued_task_entries(&self, limit: usize) -> Result<Vec<QueuedTaskEntry>> {
        let path = default_task_queue_path();
        let values = read_queue_values(&path)?;
        let mut out = Vec::new();
        for value in values {
            if entry_status(&value) != Some("queued") {
                continue;
            }
            let Some(task_id) = entry_task_id(&value) else {
                continue;
            };
            out.push(QueuedTaskEntry {
                task_id: task_id.to_string(),
                title: entry_title(&value),
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    pub(super) fn complete_queued_task(
        &self,
        _task_id: &str,
        _executed_by: &str,
    ) -> Result<QueueMutationResult> {
        Err(ArdaError::Task(
            "legacy JSONL queue completion is retired; use authenticated Engine objective control"
                .to_string(),
        ))
    }

    pub(super) fn drain_queued_tasks(
        &self,
        _limit: usize,
        _executed_by: &str,
    ) -> Result<QueueDrainResult> {
        Err(ArdaError::Task(
            "legacy JSONL queue drain is retired; use authenticated Engine objective control"
                .to_string(),
        ))
    }
}
