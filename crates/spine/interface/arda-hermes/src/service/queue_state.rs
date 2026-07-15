use super::*;
use arda_core::error::Result;
use chrono::Utc;
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
    PathBuf::from("data/hermes/task_queue.jsonl")
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

fn write_queue_values(path: &PathBuf, values: &[Value]) -> Result<()> {
    let mut content = String::new();
    for value in values {
        content.push_str(&serde_json::to_string(value)?);
        content.push('\n');
    }
    fs::write(path, content)?;
    Ok(())
}

fn mark_completed(value: &mut Value, executed_by: &str) {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("status".to_string(), Value::String("completed".to_string()));
        obj.insert(
            "completed_at".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );
        obj.insert(
            "completed_by".to_string(),
            Value::String(executed_by.to_string()),
        );
        obj.insert(
            "completion_source".to_string(),
            Value::String("hermes_decision".to_string()),
        );
    }
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
        task_id: &str,
        executed_by: &str,
    ) -> Result<QueueMutationResult> {
        let path = default_task_queue_path();
        let mut values = read_queue_values(&path)?;
        let mut found = false;
        let mut updated = false;
        let mut title = String::new();

        for value in values.iter_mut() {
            if entry_task_id(value) == Some(task_id) {
                found = true;
                title = entry_title(value).unwrap_or_default();
                if entry_status(value) == Some("queued") {
                    mark_completed(value, executed_by);
                    updated = true;
                }
                break;
            }
        }

        if updated {
            write_queue_values(&path, &values)?;
        }

        Ok(QueueMutationResult {
            found,
            updated,
            task_id: task_id.to_string(),
            title,
        })
    }

    pub(super) fn drain_queued_tasks(
        &self,
        limit: usize,
        executed_by: &str,
    ) -> Result<QueueDrainResult> {
        let path = default_task_queue_path();
        let mut values = read_queue_values(&path)?;
        let mut completed = Vec::new();
        let mut attempted = 0usize;

        for value in values.iter_mut() {
            if completed.len() >= limit {
                break;
            }
            if entry_status(value) != Some("queued") {
                continue;
            }
            attempted += 1;
            let task_id = entry_task_id(value).unwrap_or_default().to_string();
            let title = entry_title(value).unwrap_or_default();
            mark_completed(value, executed_by);
            completed.push(QueueMutationResult {
                found: true,
                updated: true,
                task_id,
                title,
            });
        }

        if !completed.is_empty() {
            write_queue_values(&path, &values)?;
        }

        let remaining = values
            .iter()
            .filter(|v| entry_status(v) == Some("queued"))
            .count();

        Ok(QueueDrainResult {
            attempted,
            completed,
            remaining,
        })
    }
}
