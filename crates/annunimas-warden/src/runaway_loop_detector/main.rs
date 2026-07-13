// sigil: REPAIR
//! Runaway Loop Detector - Detects when tasks are being generated too rapidly
//! This binary monitors the action queue and alerts if a single action type
//! exceeds a threshold within a short time window, indicating a potential
//! runaway loop in task generation.

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

// Configuration thresholds (could be made configurable via env vars)
const THRESHOLD_PER_MINUTE: usize = 10;
const LOOKBACK_DURATION: Duration = Duration::from_secs(60);

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

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Task>(&line) {
            Ok(task) => tasks.push(task),
            Err(e) => {
                warn!("Failed to parse task JSON: {} - {}", e, line);
            }
        }
    }

    Ok(tasks)
}

fn parse_utc_timestamp(ts: &str) -> Option<SystemTime> {
    match chrono::DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => {
            let utc_dt: chrono::DateTime<chrono::Utc> = dt.with_timezone(&chrono::Utc);
            Some(utc_dt.into())
        }
        Err(e) => {
            warn!("Failed to parse timestamp {}: {}", ts, e);
            None
        }
    }
}

fn post_alert(root_path: &Path, alert: Alert) -> Result<(), Box<dyn std::error::Error>> {
    let queue_path = root_path.join("data/warden/informant_queue.jsonl");

    // Ensure directory exists
    if let Some(parent) = queue_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Append alert as JSON line
    let json = serde_json::to_string(&alert)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&queue_path)?;

    writeln!(file, "{}", json)?;

    info!("Posted runaway loop alert: {:?}", alert);
    Ok(())
}

fn main() {
    let root_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Get all tasks
    let tasks = match read_tasks(&root_path) {
        Ok(tasks) => tasks,
        Err(err) => {
            error!("Failed to read action queue: {}", err);
            return;
        }
    };

    if tasks.is_empty() {
        info!("No tasks found in action queue");
        return;
    }

    // Filter tasks from the last minute
    let now = SystemTime::now();
    let cutoff = now - LOOKBACK_DURATION;

    let recent_tasks: Vec<&Task> = tasks
        .iter()
        .filter_map(|task| parse_utc_timestamp(&task.queued_at_utc).map(|ts| (task, ts)))
        .filter(|(_, ts)| ts >= &cutoff)
        .map(|(task, _)| task)
        .collect();

    if recent_tasks.is_empty() {
        info!("No recent tasks in the last minute");
        return;
    }

    // Count tasks by action type
    let mut action_counts: HashMap<String, usize> = HashMap::new();
    for task in &recent_tasks {
        *action_counts.entry(task.action.clone()).or_insert(0) += 1;
    }

    // Check for any action exceeding threshold
    for (action, count) in &action_counts {
        if *count >= THRESHOLD_PER_MINUTE {
            let alert = Alert::new(
                "runaway_loop_detected",
                "runaway_loop",
                "warning",
                Some(serde_json::json!({
                    "action": action,
                    "count": count,
                    "threshold": THRESHOLD_PER_MINUTE,
                    "time_window_seconds": LOOKBACK_DURATION.as_secs(),
                    "message": "High volume of tasks detected, possible runaway loop"
                })),
            );

            if let Err(e) = post_alert(&root_path, alert) {
                error!("Failed to post alert: {}", e);
            }
        }
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
