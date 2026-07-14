// sigil: REPAIR
//! Schema Drift Detector - Monitors data structure consistency
//! This binary checks the action queue for malformed entries and
//! unexpected fields that could indicate schema drift or data corruption.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

// Configuration thresholds
const MAX_MALFORMED_RATIO: f64 = 0.05; // 5% malformed
const EXPECTED_FIELDS: &[&str] = &["task_id", "queued_at_utc", "action"];

/// Task entry from the action queue
#[derive(Debug, Deserialize, Serialize)]
struct Task {
    task_id: String,
    queued_at_utc: String,
    action: String,
    file: Option<String>,
    authorized_by: Option<String>,
    reason: Option<String>,
    execute_after_utc: Option<String>,
    quorum_proof: Option<String>,
}

/// Alert to be posted to the informant queue
#[derive(Debug, Serialize)]
struct Alert {
    crate_name: String,
    event: String,
    event_type: String,
    severity: String,
    ts: String,
    details: Option<serde_json::Value>,
}

impl Alert {
    fn new(
        event: &str,
        event_type: &str,
        severity: &str,
        details: Option<serde_json::Value>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Alert {
            crate_name: "warden".to_string(),
            event: event.to_string(),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            ts: now,
            details,
        }
    }
}

fn read_tasks(path: &Path) -> std::io::Result<Vec<Task>> {
    let file_path = path.join("data/hades/action_queue.jsonl");
    if !file_path.exists() {
        let alt_path = path.join("data/hades/action_queue.jsonl");
        if alt_path.exists() {
            return read_tasks_from_file(&alt_path);
        } else {
            warn!("Action queue not found at {:?}", file_path);
            return Ok(Vec::new());
        }
    }
    read_tasks_from_file(&file_path)
}

fn read_tasks_from_file(file_path: &Path) -> std::io::Result<Vec<Task>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut tasks = Vec::new();
    let mut malformed_count = 0;
    let mut total_count = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        total_count += 1;
        match serde_json::from_str::<Task>(&line) {
            Ok(task) => tasks.push(task),
            Err(e) => {
                warn!("Malformed task JSON: {} - line: {}", e, line);
                malformed_count += 1;
            }
        }
    }

    let malformed_ratio = if total_count > 0 {
        malformed_count as f64 / total_count as f64
    } else {
        0.0
    };

    if malformed_ratio > MAX_MALFORMED_RATIO {
        error!(
            "High malformed ratio: {} > {}",
            malformed_ratio, MAX_MALFORMED_RATIO
        );
    }

    Ok(tasks)
}

fn check_for_unexpected_fields(tasks: &[Task]) -> HashMap<String, usize> {
    let expected_set: HashSet<&str> = EXPECTED_FIELDS.iter().copied().collect();
    let mut unexpected_counts: HashMap<String, usize> = HashMap::new();

    for task in tasks {
        let Ok(value) = serde_json::to_value(task) else {
            continue;
        };

        if let serde_json::Value::Object(map) = value {
            for (key, _) in &map {
                if !expected_set.contains(key.as_str()) {
                    let is_known_optional = [
                        "file",
                        "authorized_by",
                        "reason",
                        "execute_after_utc",
                        "quorum_proof",
                    ]
                    .contains(&key.as_str());

                    if !is_known_optional {
                        *unexpected_counts.entry(key.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    unexpected_counts
}

fn post_alert(root_path: &Path, alert: Alert) -> Result<(), Box<dyn std::error::Error>> {
    let queue_path = root_path.join("data/warden/informant_queue.jsonl");

    if let Some(parent) = queue_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string(&alert)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&queue_path)?;

    writeln!(file, "{}", json)?;

    info!("Posted schema drift alert: {:?}", alert);
    Ok(())
}

fn main() {
    let root_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let tasks = match read_tasks(&root_path) {
        Ok(tasks) => tasks,
        Err(err) => {
            error!("Failed to read action queue: {}", err);
            return;
        }
    };

    if tasks.is_empty() {
        info!("No tasks found for schema analysis");
        return;
    }

    info!("Analyzed {} tasks", tasks.len());

    // Check for unexpected fields
    let unexpected_fields = check_for_unexpected_fields(&tasks);

    if !unexpected_fields.is_empty() {
        let alert = Alert::new(
            "schema_drift_detected",
            "schema_drift",
            "warning",
            Some(serde_json::json!({
                "unexpected_fields": unexpected_fields,
                "message": "Unexpected fields detected in task schema"
            })),
        );

        if let Err(e) = post_alert(&root_path, alert) {
            error!("Failed to post alert: {}", e);
        }
    } else {
        info!("No unexpected fields detected");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_action_queue_file_returns_io_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("data/hades/action_queue.jsonl");

        let err = read_tasks_from_file(&missing).expect_err("missing file should return error");

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn malformed_rows_are_skipped_without_failing_read() {
        let dir = tempfile::tempdir().expect("tempdir");
        let queue_dir = dir.path().join("data/hades");
        fs::create_dir_all(&queue_dir).expect("queue dir");
        let queue = queue_dir.join("action_queue.jsonl");
        fs::write(
            &queue,
            concat!(
                "not json\n",
                r#"{"task_id":"task-1","queued_at_utc":"2026-05-20T00:00:00Z","action":"audit","file":null,"authorized_by":null,"reason":null,"execute_after_utc":null,"quorum_proof":null}"#,
                "\n"
            ),
        )
        .expect("queue write");

        let tasks = read_tasks(dir.path()).expect("read tasks");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "task-1");
    }
}
