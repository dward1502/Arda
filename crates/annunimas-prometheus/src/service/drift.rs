use crate::service::{
    append_jsonl, prometheus_home, queue_contains_task, sha256_file_if_exists, PrometheusService,
};
use annunimas_core::error::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

impl PrometheusService {
    pub fn latest_drift_report(&self) -> Option<serde_json::Value> {
        let path = prometheus_home().join("drift_report_last.json");
        let raw = fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn drift_detect_reconcile(&self, auto_open: bool) -> Result<serde_json::Value> {
        let tracked_paths = vec![
            "core/state/active_ruleset.json",
            "core/state/interrupt_authority.json",
            "core/state/destructive_quorum.json",
            "config/default.toml",
            "config/runtime.env.example",
        ];
        let baseline_path = prometheus_home().join("drift_baseline.json");
        let report_last_path = prometheus_home().join("drift_report_last.json");
        let report_history_path = prometheus_home().join("drift_reports.jsonl");
        let queue_path = PathBuf::from("core/projects/tasks/queue.jsonl");
        if let Some(parent) = baseline_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut current = serde_json::Map::new();
        for rel in &tracked_paths {
            let p = PathBuf::from(rel);
            let hash = sha256_file_if_exists(&p)?;
            let size_bytes = fs::metadata(&p).ok().map(|m| m.len());
            current.insert(
                rel.to_string(),
                serde_json::json!({
                    "hash": hash,
                    "size_bytes": size_bytes
                }),
            );
        }
        let current_value = serde_json::Value::Object(current.clone());
        let now = chrono::Utc::now().to_rfc3339();
        let baseline: Option<serde_json::Value> = fs::read_to_string(&baseline_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok());
        let baseline_created = baseline.is_none();
        if baseline_created {
            fs::write(
                &baseline_path,
                serde_json::to_string_pretty(&serde_json::json!({
                    "created_at_utc": now,
                    "files": current_value
                }))?,
            )?;
        }

        let mut drift = Vec::new();
        if let Some(base) = baseline
            .as_ref()
            .and_then(|v| v.get("files"))
            .and_then(|v| v.as_object())
        {
            for (path, snapshot) in &current {
                let current_hash = snapshot
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("missing");
                let baseline_hash = base
                    .get(path)
                    .and_then(|v| v.get("hash"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("missing");
                if current_hash != baseline_hash {
                    drift.push(serde_json::json!({
                        "path": path,
                        "baseline_hash": baseline_hash,
                        "current_hash": current_hash
                    }));
                }
            }
        }

        let mut queued_tasks = Vec::new();
        if auto_open && !drift.is_empty() {
            for item in &drift {
                let Some(path) = item.get("path").and_then(|v| v.as_str()) else {
                    continue;
                };
                let mut hasher = Sha256::new();
                hasher.update(path.as_bytes());
                let task_id = format!("tsk_drift_{}", &format!("{:x}", hasher.finalize())[..12]);
                if !queue_contains_task(&queue_path, &task_id)? {
                    let task = serde_json::json!({
                        "id": task_id,
                        "status": "queued",
                        "title": format!("Reconcile runtime drift for {}", path),
                        "priority": "high",
                        "owner": "prometheus",
                        "queued_at_utc": now,
                        "notes": format!("Auto-opened by drift detector; baseline mismatch for {}", path),
                        "meta": {"origin": "drift_detector", "path": path}
                    });
                    append_jsonl(&queue_path, &task)?;
                    queued_tasks.push(task);
                }
            }
        }

        let report = serde_json::json!({
            "generated_at_utc": now,
            "baseline_path": baseline_path,
            "baseline_created": baseline_created,
            "drift_count": drift.len(),
            "drift": drift,
            "auto_open": auto_open,
            "queued_tasks": queued_tasks,
            "tracked_files": current_value
        });
        fs::write(&report_last_path, serde_json::to_string_pretty(&report)?)?;
        append_jsonl(&report_history_path, &report)?;
        Ok(report)
    }
}
