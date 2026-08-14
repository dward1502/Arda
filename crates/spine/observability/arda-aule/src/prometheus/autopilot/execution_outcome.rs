#![cfg(feature = "full-cli")]
//! Replay-safe projection of terminal Workbench queue outcomes into Arda authorities.

use super::learning::LearningStore;
use super::task_queue::QueueRecord;
use anyhow::{Context, Result};
use arda_vaire::{InformantEvent, MnemosyneService};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const EXECUTION_OUTCOME_CONTRACT: &str = "arda.workbench.execution_outcome.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionOutcomeProjectionReceipt {
    pub contract: String,
    pub task_id: String,
    pub recommendation_id: String,
    pub approval_packet_id: String,
    pub workbench_run_id: String,
    pub status: String,
    pub result: String,
    pub execution_receipt_digest: Option<String>,
    pub evidence_path: String,
    pub learning_key: String,
    pub memory_id: Option<String>,
    pub governance_event_id: String,
    pub projected_at_utc: String,
}

pub fn project_terminal_outcome(
    root: impl AsRef<Path>,
    task: &QueueRecord,
    run_id: &str,
    status: &str,
    result: &str,
    receipt_digest: Option<&str>,
    detail: Option<&str>,
) -> Result<ExecutionOutcomeProjectionReceipt> {
    let root = root.as_ref();
    let receipt_path = outcome_receipt_path(root, run_id);
    if let Some(existing) = read_receipt(&receipt_path)? {
        return Ok(existing);
    }

    let lock_path = root.join("data/locks/workbench-outcome-projection.lock");
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.lock_exclusive()?;
    let result: Result<ExecutionOutcomeProjectionReceipt> = (|| -> Result<_> {
        if let Some(existing) = read_receipt(&receipt_path)? {
            return Ok(existing);
        }

        let meta = task.extra.get("meta").and_then(Value::as_object);
        let recommendation_id = meta
            .and_then(|value| value.get("source_objective_packet_id"))
            .and_then(Value::as_str)
            .unwrap_or("unknown-recommendation")
            .to_owned();
        let approval_packet_id = meta
            .and_then(|value| value.get("approval_packet_id"))
            .and_then(Value::as_str)
            .unwrap_or("unknown-approval")
            .to_owned();
        let task_type = meta
            .and_then(|value| value.get("action_class"))
            .and_then(Value::as_str)
            .unwrap_or("approved_autopilot_plan_step");
        let success = status == "completed" && result == "completed";
        let learning_key = format!("arda_workbench::{task_type}");

        let learning_store = LearningStore::new(root.join("data/ceo/learning.json"));
        let mut learning = learning_store.load();
        learning.observe("arda_workbench", task_type, success, 0.0, 0.0);
        learning_store.save(&learning)?;

        let memory = MnemosyneService::new(root.join("data/mnemosyne"))?
            .with_contract_memory_root(root.join("core/state/memory"));
        let memory_entry = memory.encode(InformantEvent {
            informant_id: "arda_workbench.queue_executor".into(),
            crate_name: "arda-aule".into(),
            event_type: "governed_execution_outcome".into(),
            ts_utc: Utc::now().to_rfc3339(),
            content: format!(
                "Governed Workbench run {run_id} for task {} ended status={status} result={result}; receipt={}; detail={}",
                task.id,
                receipt_digest.unwrap_or("none"),
                detail.unwrap_or("none")
            ),
            confidence_hint: Some(1.0),
            tags: vec![
                "governed".into(),
                "execution-receipt".into(),
                format!("task_id:{}", task.id),
                format!("recommendation_id:{recommendation_id}"),
                format!("approval_packet_id:{approval_packet_id}"),
                format!("workbench_run_id:{run_id}"),
            ],
        })?;
        let memory_id = memory_entry.map(|entry| entry.memory_id);

        let governance_event_id = format!("workbench-outcome:{run_id}");
        append_governance_event_once(
            &root.join("data/governance/workbench_execution_outcomes.jsonl"),
            &governance_event_id,
            json!({
                "contract": EXECUTION_OUTCOME_CONTRACT,
                "event_id": governance_event_id,
                "task_id": task.id,
                "recommendation_id": recommendation_id,
                "approval_packet_id": approval_packet_id,
                "workbench_run_id": run_id,
                "status": status,
                "result": result,
                "execution_receipt_digest": receipt_digest,
                "observed_at_utc": Utc::now().to_rfc3339(),
            }),
        )?;

        let relative_path = receipt_path
            .strip_prefix(root)
            .unwrap_or(&receipt_path)
            .to_string_lossy()
            .to_string();
        let receipt = ExecutionOutcomeProjectionReceipt {
            contract: EXECUTION_OUTCOME_CONTRACT.into(),
            task_id: task.id.clone(),
            recommendation_id,
            approval_packet_id,
            workbench_run_id: run_id.to_owned(),
            status: status.to_owned(),
            result: result.to_owned(),
            execution_receipt_digest: receipt_digest.map(str::to_owned),
            evidence_path: relative_path,
            learning_key,
            memory_id,
            governance_event_id,
            projected_at_utc: Utc::now().to_rfc3339(),
        };
        write_receipt_atomic(&receipt_path, &receipt)?;
        Ok(receipt)
    })();
    let unlock = FileExt::unlock(&lock);
    match (result, unlock) {
        (Ok(receipt), Ok(())) => Ok(receipt),
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error.into()),
    }
}

fn outcome_receipt_path(root: &Path, run_id: &str) -> PathBuf {
    let safe = run_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    root.join("audit/workbench-queue")
        .join(safe)
        .join("execution_receipt.json")
}

fn read_receipt(path: &Path) -> Result<Option<ExecutionOutcomeProjectionReceipt>> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(Some(serde_json::from_str(&raw).with_context(|| {
            format!("decode execution outcome receipt {}", path.display())
        })?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_receipt_atomic(path: &Path, receipt: &ExecutionOutcomeProjectionReceipt) -> Result<()> {
    let parent = path.parent().context("outcome receipt has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(receipt)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn append_governance_event_once(path: &Path, event_id: &str, value: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Ok(file) = std::fs::File::open(path) {
        let exists = BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .any(|line| {
                serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|entry| {
                        entry
                            .get("event_id")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .as_deref()
                    == Some(event_id)
            });
        if exists {
            return Ok(());
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, &value)?;
    writeln!(file)?;
    file.sync_data()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn approved_task() -> QueueRecord {
        QueueRecord {
            id: "task-1".into(),
            title: Some("bounded task".into()),
            owner: Some("arda-workbench".into()),
            priority: Some("low".into()),
            status: Some("completed".into()),
            result: Some("completed".into()),
            queued_at_utc: None,
            completed_at_utc: None,
            started_at_utc: None,
            extra: json!({
                "meta": {
                    "source_objective_packet_id": "recommendation-1",
                    "approval_packet_id": "approval-1",
                    "action_class": "approved_autopilot_plan_step"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        }
    }

    #[test]
    fn terminal_projection_is_replay_safe_across_all_ledgers() {
        let dir = tempfile::tempdir().unwrap();
        let task = approved_task();
        let first = project_terminal_outcome(
            dir.path(),
            &task,
            "queue-task-1",
            "completed",
            "completed",
            Some("sha256:receipt"),
            Some("done"),
        )
        .unwrap();
        let second = project_terminal_outcome(
            dir.path(),
            &task,
            "queue-task-1",
            "completed",
            "completed",
            Some("sha256:receipt"),
            Some("done"),
        )
        .unwrap();

        assert_eq!(first, second);
        let learning = LearningStore::new(dir.path().join("data/ceo/learning.json")).load();
        assert_eq!(
            learning.stats["arda_workbench::approved_autopilot_plan_step"].attempts,
            1
        );
        assert_eq!(
            std::fs::read_to_string(
                dir.path()
                    .join("data/governance/workbench_execution_outcomes.jsonl")
            )
            .unwrap()
            .lines()
            .count(),
            1
        );
        assert!(dir
            .path()
            .join("audit/workbench-queue/queue-task-1/execution_receipt.json")
            .exists());
        assert!(first.memory_id.is_some());
    }
}
