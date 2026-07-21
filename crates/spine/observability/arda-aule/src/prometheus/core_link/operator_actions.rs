#![cfg(feature = "full-cli")]
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::support::latest_jsonl_entries_by_id;
use super::{read_json_file, CORE_STATE_SCHEMA_VERSION};

pub fn write_operator_actions_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("operator_actions.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let tasks = latest_jsonl_entries_by_id(&workspace_root.join("core/projects/tasks/queue.jsonl"));
    let package_runtime = read_json_file(
        core_root
            .join("state")
            .join("package_runtime_activation.json"),
    )
    .unwrap_or_else(|| json!({}));
    let fleet_reconciliation = read_json_file(
        core_root
            .join("state")
            .join("fleet_identity_reconciliation.json"),
    )
    .unwrap_or_else(|| json!({}));
    let fleet_bootstrap_recovery = read_json_file(
        core_root
            .join("state")
            .join("fleet_bootstrap_recovery.json"),
    )
    .unwrap_or_else(|| json!({}));

    let mut actions = Vec::new();
    for task in tasks.iter().filter(|task| {
        matches!(
            task.get("status").and_then(Value::as_str),
            Some("blocked") | Some("deferred")
        )
    }) {
        let origin = task
            .get("meta")
            .and_then(|value| value.get("origin"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        actions.push(json!({
            "title": task.get("title").and_then(Value::as_str).unwrap_or("Untitled action"),
            "owner": task.get("owner").and_then(Value::as_str).unwrap_or("unknown"),
            "status": task.get("status").and_then(Value::as_str).unwrap_or("unknown"),
            "kind": if origin == "external_blocker" { "external_blocker" } else { "task_blocker" },
            "note": task.get("notes").and_then(Value::as_str).unwrap_or("Human action required."),
        }));
    }

    if let Some(surfaces) = package_runtime.get("surfaces").and_then(Value::as_object) {
        for (tool, state) in surfaces {
            let status = state
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if matches!(status, "auth_required" | "configuration_required") {
                actions.push(json!({
                    "title": format!("{tool} requires human setup"),
                    "owner": "prometheus",
                    "status": status,
                    "kind": if status == "auth_required" { "auth_required" } else { "configuration_required" },
                    "note": format!(
                        "{}. {}",
                        state.get("project_root").and_then(Value::as_str).unwrap_or("External package surface"),
                        if status == "auth_required" {
                            "Authentication or account linking is required before autonomous activation can continue"
                        } else {
                            "Configuration is required before autonomous activation can continue"
                        }
                    ),
                }));
            }
        }
    }

    if let Some(candidates) = fleet_reconciliation
        .get("canonical_binding_candidates")
        .and_then(Value::as_array)
    {
        for candidate in candidates {
            let target_id = candidate
                .get("target_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown-target");
            let expected_hostname = candidate
                .get("expected_hostname")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let tailscale_names = candidate
                .get("candidate_tailscale_names")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none".to_string());
            actions.push(json!({
                "title": format!("{target_id} requires canonical edge identity binding"),
                "owner": "warden",
                "status": "operator_confirmation_required",
                "kind": "identity_binding",
                "note": format!(
                    "Bind `{target_id}` to expected hostname `{expected_hostname}` before enrollment. Candidate Tailscale names: {tailscale_names}."
                ),
            }));
        }
    }

    if let Some(clusters) = fleet_reconciliation
        .get("stale_hostname_clusters")
        .and_then(Value::as_array)
    {
        for cluster in clusters {
            let count = cluster.get("count").and_then(Value::as_u64).unwrap_or(0);
            if count <= 1 {
                continue;
            }
            let hostname = cluster
                .get("hostname")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let tailscale_ids = cluster
                .get("tailscale_node_ids")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none".to_string());
            actions.push(json!({
                "title": format!("Retire stale duplicate identities for {hostname}"),
                "owner": "warden",
                "status": "operator_confirmation_required",
                "kind": "identity_cleanup",
                "note": format!(
                    "Multiple stale nodes still share hostname `{hostname}`. Review and retire duplicates: {tailscale_ids}."
                ),
            }));
        }
    }

    if let Some(targets) = fleet_bootstrap_recovery
        .get("targets")
        .and_then(Value::as_array)
    {
        for target in targets {
            let Some(recovery) = target.get("recovery") else {
                continue;
            };
            let attempted = recovery
                .get("attempted")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let recovered = recovery
                .get("recovered")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !attempted || recovered {
                continue;
            }
            let target_id = target
                .get("target_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown-target");
            let attempt_note = recovery
                .get("attempts")
                .and_then(Value::as_array)
                .and_then(|attempts| attempts.last())
                .and_then(|attempt| attempt.get("restart"))
                .map(|restart| {
                    let ssh_target = restart
                        .get("ssh_target")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let error = restart
                        .get("error")
                        .and_then(Value::as_str)
                        .or_else(|| restart.get("stderr").and_then(Value::as_str))
                        .unwrap_or("restart failed");
                    format!("Latest recovery path targeted `{ssh_target}` and reported: {error}.")
                })
                .unwrap_or_else(|| {
                    "Recovery failed without a recorded restart payload.".to_string()
                });
            actions.push(json!({
                "title": format!("{target_id} requires fleet recovery remediation"),
                "owner": "manwe_hermes",
                "status": "operator_confirmation_required",
                "kind": "fleet_recovery_failed",
                "note": attempt_note,
            }));
        }
    }

    let external_blockers_total = actions
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("external_blocker"))
        .count();
    let auth_required_total = actions
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("auth_required"))
        .count();
    let configuration_required_total = actions
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("configuration_required"))
        .count();
    let identity_binding_total = actions
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("identity_binding"))
        .count();
    let identity_cleanup_total = actions
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("identity_cleanup"))
        .count();
    let fleet_recovery_failed_total = actions
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("fleet_recovery_failed"))
        .count();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "operator_actions_projection",
        "summary": {
            "human_needed_total": actions.len(),
            "external_blockers_total": external_blockers_total,
            "auth_required_total": auth_required_total,
            "configuration_required_total": configuration_required_total,
            "identity_binding_total": identity_binding_total,
            "identity_cleanup_total": identity_cleanup_total,
            "fleet_recovery_failed_total": fleet_recovery_failed_total
        },
        "actions": actions,
        "arda_hints": {
            "primary_panel": "operations_and_packages",
            "boardroom_section": "human_needed",
            "alert_on_human_needed": !actions.is_empty()
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
