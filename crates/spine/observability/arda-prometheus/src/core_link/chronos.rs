use super::*;

pub(super) fn write_chronos_status_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("chronos_status.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let runtime_path = core_root.join("state").join("chronos_runtime.json");
    let runtime = read_json_file(runtime_path).unwrap_or_else(|| {
        json!({
            "schema_version": "unknown",
            "generated_at_utc": Value::Null,
            "status": "unknown",
            "mode": "runtime_projection_missing",
            "capabilities": [],
            "feed_summary": {},
            "audit_runner": {
                "status": "unknown",
                "ready_task_count": 0,
                "scheduled_tasks": [],
                "configured_audit_classes": [],
                "receipt_count": 0
            },
            "next_integration_steps": ["Run arda-chronos or refresh core/state/chronos_runtime.json."]
        })
    });

    let projection = json!({
        "schema_version": "arda.chronos-status.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "prometheus_core_link chronos_status_projection",
        "source_runtime_schema": runtime.get("schema_version").cloned().unwrap_or_else(|| json!("unknown")),
        "source_runtime_generated_at_utc": runtime.get("generated_at_utc").cloned().unwrap_or(Value::Null),
        "source_runtime_mode": runtime.get("mode").cloned().unwrap_or_else(|| json!("unknown")),
        "status": runtime.get("status").cloned().unwrap_or_else(|| json!("unknown")),
        "mode": "local_runtime_visibility_projection",
        "activation_boundary": {
            "approved": true,
            "approval_source": "core/projects/tasks/queue.jsonl",
            "packet_id": "CHRONOS-P3",
            "policy": "local status visibility only; no service restart, scheduling mutation, credential use, or external send"
        },
        "capabilities": runtime.get("capabilities").cloned().unwrap_or_else(|| json!([])),
        "feed_summary": runtime.get("feed_summary").cloned().unwrap_or_else(|| json!({})),
        "audit_runner": runtime.get("audit_runner").cloned().unwrap_or_else(|| json!({
            "status": "unknown",
            "ready_task_count": 0,
            "scheduled_tasks": [],
            "configured_audit_classes": [],
            "receipt_count": 0
        })),
        "next_integration_steps": runtime.get("next_integration_steps").cloned().unwrap_or_else(|| json!([])),
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&projection).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
