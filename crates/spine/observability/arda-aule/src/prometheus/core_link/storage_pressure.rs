#![cfg(feature = "full-cli")]
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::{read_json_file, storage_root_entry, CORE_STATE_SCHEMA_VERSION};

pub fn write_storage_pressure_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("storage_pressure.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let compaction = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("compaction_last.json"),
    )
    .unwrap_or_else(|| json!({"stores":[],"backup_cleanup":{"deleted":[],"kept":[]}}));
    let audit = read_json_file(core_root.join("metrics").join("audit_latest.json"))
        .unwrap_or_else(|| json!({"storage_pressure":{"oversize_present":false}}));
    let pressure_guard = read_json_file(
        workspace_root
            .join("core")
            .join("state")
            .join("runtime_admission_pressure.json"),
    )
    .unwrap_or_else(|| json!({"status":"unknown","observed":{},"rationale":{"supersedes_legacy_pressure_guard_path":"data/prometheus/pressure_guard_last.json"}}));
    let workspace_roots = vec![
        storage_root_entry(
            &workspace_root,
            "target",
            "rebuildable_artifacts",
            "cargo build outputs",
        ),
        storage_root_entry(
            &workspace_root,
            "data",
            "operational_ledgers",
            "runtime stores and append-only ledgers",
        ),
        storage_root_entry(
            &workspace_root,
            "core/metrics/history",
            "accounted_history",
            "audited metrics history snapshots",
        ),
        storage_root_entry(
            &workspace_root,
            "data/accounting/output_mirror",
            "accounting_mirror",
            "non-destructive long-term output mirror",
        ),
        storage_root_entry(
            &workspace_root,
            "human",
            "human_visual_layer",
            "human-facing markdown and graph notes",
        ),
    ];
    let rebuildable_bytes = workspace_roots
        .iter()
        .find(|entry| entry["path"] == "target")
        .and_then(|entry| entry["bytes"].as_u64())
        .unwrap_or(0);
    let operational_bytes = workspace_roots
        .iter()
        .find(|entry| entry["path"] == "data")
        .and_then(|entry| entry["bytes"].as_u64())
        .unwrap_or(0);
    let history_bytes = workspace_roots
        .iter()
        .find(|entry| entry["path"] == "core/metrics/history")
        .and_then(|entry| entry["bytes"].as_u64())
        .unwrap_or(0);
    let accounting_bytes = workspace_roots
        .iter()
        .find(|entry| entry["path"] == "data/accounting/output_mirror")
        .and_then(|entry| entry["bytes"].as_u64())
        .unwrap_or(0);
    let total_observed_workspace_bytes: u64 = workspace_roots
        .iter()
        .filter_map(|entry| entry["bytes"].as_u64())
        .sum();
    let reclaim_candidates = workspace_roots
        .iter()
        .filter_map(|entry| {
            let bytes = entry["bytes"].as_u64().unwrap_or(0);
            let path = entry["path"].as_str().unwrap_or("");
            let class = entry["classification"].as_str().unwrap_or("");
            if bytes == 0 {
                return None;
            }
            let recommended_action = match (path, class) {
                ("target", "rebuildable_artifacts") => "prune_rebuildable",
                ("core/metrics/history", "accounted_history") => "retention_guard",
                ("data/accounting/output_mirror", "accounting_mirror") => "compact_mirror",
                _ => "observe_only",
            };
            if recommended_action == "observe_only" {
                return None;
            }
            Some(json!({
                "path": path,
                "classification": class,
                "bytes": bytes,
                "recommended_action": recommended_action,
            }))
        })
        .collect::<Vec<_>>();
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "storage_pressure_projection",
        "compaction": compaction,
        "pressure_guard": pressure_guard,
        "audit_storage_pressure": audit.get("storage_pressure").cloned().unwrap_or_else(|| json!({})),
        "workspace_roots": workspace_roots,
        "reclaim_candidates": reclaim_candidates,
        "summary": {
            "total_observed_workspace_bytes": total_observed_workspace_bytes,
            "rebuildable_bytes": rebuildable_bytes,
            "operational_bytes": operational_bytes,
            "history_bytes": history_bytes,
            "accounting_mirror_bytes": accounting_bytes,
        },
        "status": {
            "oversize_present": audit
                .get("storage_pressure")
                .and_then(|v| v.get("oversize_present"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "backup_cleanup_configured": true,
            "disk_alert_active": pressure_guard
                .get("status")
                .and_then(Value::as_str)
                .map(|status| status == "alert")
                .unwrap_or(false)
        }
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
