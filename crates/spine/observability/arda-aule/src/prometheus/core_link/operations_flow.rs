#![cfg(feature = "full-cli")]
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    collect_file_paths, escalation_dedupe_key, latest_jsonl_entries_by_id, read_all_jsonl,
    read_json_file, read_recent_jsonl, rel_path, summarize_field_count_value,
    summarize_field_counts, CORE_STATE_SCHEMA_VERSION,
};

pub fn write_operations_flow_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("operations_flow.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let ops = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("prometheus")
            .join("ops_dashboard.json"),
    )
    .unwrap_or_else(|| json!({}));
    let queue_summary = read_json_file(core_root.join("state").join("queue_summary.json"))
        .unwrap_or_else(|| json!({}));
    let governance = read_json_file(core_root.join("state").join("governance_runtime.json"))
        .unwrap_or_else(|| json!({}));
    let lockdown = read_json_file(core_root.join("state").join("control_plane_lockdown.json"))
        .unwrap_or_else(|| json!({}));

    let project_queue_queued = ops
        .get("queue_observability")
        .and_then(|value| value.get("summary"))
        .and_then(|value| value.get("projects_queue_queued"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_known_work_items = ops
        .get("queue_observability")
        .and_then(|value| value.get("summary"))
        .and_then(|value| value.get("total_known_work_items"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let pending_escalations = ops
        .get("prometheus")
        .and_then(|value| value.get("pending_escalations"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let subsystem_health = json!({
        "athena": ops.get("athena").cloned().unwrap_or_else(|| json!({})),
        "manwe": ops.get("manwe").cloned().unwrap_or_else(|| json!({})),
        "hades": ops.get("hades").cloned().unwrap_or_else(|| json!({})),
        "hermes": ops.get("hermes").cloned().unwrap_or_else(|| json!({})),
        "mnemosyne": ops.get("mnemosyne").cloned().unwrap_or_else(|| json!({})),
        "apollo": ops.get("apollo").cloned().unwrap_or_else(|| json!({})),
        "plutus": ops.get("plutus").cloned().unwrap_or_else(|| json!({})),
        "prometheus": ops.get("prometheus").cloned().unwrap_or_else(|| json!({}))
    });

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "operations_flow_projection",
        "dashboard": ops,
        "queue_summary": queue_summary,
        "governance_runtime": governance,
        "control_plane_lockdown": lockdown,
        "derived": {
            "queue_posture": {
                "projects_queue_queued": project_queue_queued,
                "total_known_work_items": total_known_work_items,
                "pending_escalations": pending_escalations
            },
            "subsystem_health": subsystem_health,
            "control_plane_ready": lockdown
                .get("status")
                .and_then(|value| value.get("lockdown_ready"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "autonomy_ready": governance
                .get("goal")
                .and_then(|value| value.get("autonomy_ready"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        },
        "arda_hints": {
            "primary_panel": "operations_flow",
            "boardroom_section": "runtime_and_queue",
            "alert_on_pending_escalations": pending_escalations > 0,
            "alert_on_nonzero_project_queue": project_queue_queued > 0
        }
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub fn write_hades_lifecycle_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("hades_lifecycle.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let status = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("hades")
            .join("status.json"),
    )
    .unwrap_or_else(|| json!({}));
    let queue = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("hades")
            .join("queue.json"),
    )
    .unwrap_or_else(|| json!([]));
    let joulework = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hades")
            .join("joulework.jsonl"),
        16,
    );
    let hades_log = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hades")
            .join("hades_log.jsonl"),
        24,
    );
    let warden_queue = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hades")
            .join("warden_queue.jsonl"),
        16,
    );
    let athena_handoffs = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hades")
            .join("athena_handoff_queue.jsonl"),
        16,
    );

    let repair_events = hades_log
        .iter()
        .filter(|entry| entry.get("event").and_then(Value::as_str) == Some("repair_detected"))
        .count();
    let orphan_events = hades_log
        .iter()
        .filter(|entry| entry.get("event").and_then(Value::as_str) == Some("orphan_found"))
        .count();
    let handoff_fallbacks = athena_handoffs
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("queued_fallback"))
        .count();
    let consistency_holds = hades_log
        .iter()
        .filter(|entry| {
            entry.get("event").and_then(Value::as_str) == Some("soterion_consistency_hold")
        })
        .count();
    let mut scope_counts = serde_json::Map::new();
    for scope in [
        "system_continuity",
        "boardroom_council",
        "human_context",
        "edge_runtime",
    ] {
        let count = hades_log
            .iter()
            .filter(|entry| {
                entry
                    .get("details")
                    .and_then(|details| details.get("memory_scope"))
                    .and_then(Value::as_str)
                    == Some(scope)
            })
            .count();
        scope_counts.insert(scope.to_string(), json!(count));
    }

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "hades_lifecycle_projection",
        "status": status,
        "queue": queue,
        "recent_activity": {
            "log": hades_log,
            "joulework": joulework,
            "warden_handoffs": warden_queue,
            "athena_handoffs": athena_handoffs,
            "counts": {
                "repair_events": repair_events,
                "orphan_events": orphan_events,
                "athena_fallback_handoffs": handoff_fallbacks,
                "soterion_consistency_holds": consistency_holds
            },
            "policy_summary": {
                "memory_scope_counts": scope_counts,
                "has_scope_aware_policy": true,
                "has_soterion_consistency_checks": true
            }
        },
        "arda_hints": {
            "primary_panel": "lifecycle_maintenance",
            "pending_actions": queue.as_array().map(|items| items.len()).unwrap_or(0),
            "alert_on_fallback_handoffs": handoff_fallbacks > 0
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub fn write_queue_summary_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("queue_summary.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let queue_path =
        crate::prometheus::queue_authority::canonical_project_task_queue(&workspace_root);
    let project_tasks = latest_jsonl_entries_by_id(&queue_path);
    let recent_project_tasks = compact_task_rows(
        &project_tasks
            .iter()
            .rev()
            .take(32)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>(),
    );
    let open_project_tasks = compact_task_rows(
        &project_tasks
            .iter()
            .filter(|task| is_open_task_status(task.get("status").and_then(Value::as_str)))
            .take(32)
            .cloned()
            .collect::<Vec<_>>(),
    );
    let open_project_tasks_total = project_tasks
        .iter()
        .filter(|task| is_open_task_status(task.get("status").and_then(Value::as_str)))
        .count();
    let recent_runtime_queue =
        read_recent_jsonl(&workspace_root.join("core/queue/queue.jsonl"), 32);
    let compact_runtime_queue = recent_runtime_queue
        .iter()
        .map(compact_queue_row)
        .collect::<Vec<_>>();
    let plans = collect_file_paths(&workspace_root.join("core/projects/Plans"), "md");

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "queue_summary_projection",
        "agent_reading_policy": {
            "default_surface": "core/state/queue_active.json",
            "summary_surface": "core/state/queue_summary.json",
            "raw_ledger": rel_path(queue_path, &workspace_root),
            "raw_ledger_role": "compacted_active_ledger_and_append_target",
            "guidance": "Agents should read queue_active.json for active task selection, then queue_summary.json for counts. Do not bulk-read queue.jsonl; open it only for exact id evidence, append validation, or targeted append."
        },
        "project_tasks": {
            "total_effective": project_tasks.len(),
            "counts_by_status": summarize_field_counts(&project_tasks, "status"),
            "counts_by_owner": summarize_field_counts(&project_tasks, "owner"),
            "counts_by_priority": summarize_field_counts(&project_tasks, "priority"),
            "open_total": open_project_tasks_total,
            "open_compact_limit": 32,
            "open_compact": open_project_tasks,
            "recent_compact": recent_project_tasks
        },
        "runtime_queue": {
            "counts_by_status": summarize_field_counts(&recent_runtime_queue, "status"),
            "counts_by_owner": summarize_field_counts(&recent_runtime_queue, "owner"),
            "recent_compact": compact_runtime_queue
        },
        "plans": {
            "count": plans.len(),
            "paths": plans
        },
        "arda_hints": {
            "primary_panel": "task_board",
            "boardroom_section": "execution_queue",
            "alert_on_queued_tasks": summarize_field_count_value(&project_tasks, "status", "queued") > 0,
            "alert_on_failed_tasks": summarize_field_count_value(&project_tasks, "result", "failed") > 0
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub fn write_queue_active_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("queue_active.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let queue_path =
        crate::prometheus::queue_authority::canonical_project_task_queue(&workspace_root);
    let raw_tasks = read_all_jsonl(&queue_path);
    let project_tasks = latest_jsonl_entries_by_id(&queue_path);
    let mut active_tasks = project_tasks
        .iter()
        .filter(|task| is_open_task_status(task.get("status").and_then(Value::as_str)))
        .map(compact_task_row)
        .collect::<Vec<_>>();
    active_tasks.sort_by(|left, right| {
        let left_priority = priority_rank(
            left.get("priority")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        let right_priority = priority_rank(
            right
                .get("priority")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        right_priority.cmp(&left_priority).then_with(|| {
            left.get("queued_at_utc")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .cmp(
                    right
                        .get("queued_at_utc")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
        })
    });

    let snapshot = json!({
        "schema_version": "annunimas.queue_active.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "queue_active_projection",
        "source": rel_path(queue_path, &workspace_root),
        "mutation_policy": "read_only_latest_by_id_projection_no_queue_compaction",
        "raw_ledger_rows_total": raw_tasks.len(),
        "latest_task_ids_total": project_tasks.len(),
        "active_task_count": active_tasks.len(),
        "agent_reading_policy": {
            "default_surface": "core/state/queue_active.json",
            "fallback_surface": "core/state/queue_summary.json",
            "hygiene_surface": "core/state/queue_hygiene.json",
            "raw_queue_policy": "Do not read the raw queue for task discovery. Use this compact active projection first; open the raw queue only for exact id evidence or appending a task record."
        },
        "tasks": active_tasks
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn is_open_task_status(status: Option<&str>) -> bool {
    matches!(
        status,
        Some("pending" | "queued" | "in_progress" | "running" | "active")
    )
}

fn compact_task_rows(tasks: &[Value]) -> Vec<Value> {
    tasks.iter().map(compact_task_row).collect()
}

fn compact_task_row(task: &Value) -> Value {
    json!({
        "id": task.get("id").cloned().unwrap_or(Value::Null),
        "title": task.get("title").cloned().unwrap_or(Value::Null),
        "owner": task.get("owner").cloned().unwrap_or(Value::Null),
        "priority": task.get("priority").cloned().unwrap_or(Value::Null),
        "status": task.get("status").cloned().unwrap_or(Value::Null),
        "result": task.get("result").cloned().unwrap_or(Value::Null),
        "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
        "completed_at_utc": task.get("completed_at_utc").cloned().unwrap_or(Value::Null),
        "origin": task
            .get("meta")
            .and_then(|meta| meta.get("origin"))
            .cloned()
            .unwrap_or(Value::Null),
        "scope": task
            .get("meta")
            .and_then(|meta| meta.get("scope"))
            .cloned()
            .unwrap_or(Value::Null),
    })
}

fn compact_queue_row(row: &Value) -> Value {
    json!({
        "id": row
            .get("id")
            .or_else(|| row.get("task_id"))
            .cloned()
            .unwrap_or(Value::Null),
        "owner": row.get("owner").cloned().unwrap_or(Value::Null),
        "status": row.get("status").cloned().unwrap_or(Value::Null),
        "queued_at_utc": row.get("queued_at_utc").cloned().unwrap_or(Value::Null),
    })
}

pub fn write_escalation_runtime_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("escalation_runtime.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let escalations_path = workspace_root
        .join("data")
        .join("prometheus")
        .join("escalations.jsonl");
    let escalations = read_all_jsonl(&escalations_path);
    let mut latest_by_id: HashMap<String, Value> = HashMap::new();
    for escalation in &escalations {
        if let Some(id) = escalation.get("escalation_id").and_then(Value::as_str) {
            latest_by_id.insert(id.to_string(), escalation.clone());
        }
    }
    let latest_rows = latest_by_id.values().cloned().collect::<Vec<_>>();
    let pending = latest_rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("pending"))
        .cloned()
        .collect::<Vec<_>>();
    let mut deduped: HashMap<String, Value> = HashMap::new();
    let mut duplicate_pending_count = 0usize;
    let mut reason_buckets: HashMap<String, usize> = HashMap::new();
    for row in &pending {
        if let Some(reason) = row.get("reason").and_then(Value::as_str) {
            *reason_buckets.entry(reason.to_string()).or_insert(0) += 1;
        }
        let key = escalation_dedupe_key(row);
        if deduped.insert(key, row.clone()).is_some() {
            duplicate_pending_count += 1;
        }
    }
    let mut reason_bucket_rows = reason_buckets
        .into_iter()
        .map(|(reason, count)| json!({ "reason": reason, "count": count }))
        .collect::<Vec<_>>();
    reason_bucket_rows.sort_by(|a, b| {
        b.get("count")
            .and_then(Value::as_u64)
            .cmp(&a.get("count").and_then(Value::as_u64))
    });
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "escalation_runtime_projection",
        "paths": {
            "escalations_path": rel_path(escalations_path.clone(), &workspace_root),
        },
        "summary": {
            "pending_total": pending.len(),
            "pending_deduped": deduped.len(),
            "duplicate_pending_count": duplicate_pending_count,
        },
        "reason_buckets": reason_bucket_rows,
        "recent_pending": pending.iter().rev().take(16).cloned().collect::<Vec<_>>(),
        "arda_hints": {
            "primary_panel": "escalation_queue",
            "alert_on_duplicates": duplicate_pending_count > 0,
            "alert_on_pending": !pending.is_empty()
        }
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
