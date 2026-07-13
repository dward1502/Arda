use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::support::{count_jsonl_lines, read_recent_jsonl};
use super::{read_json_file, CORE_STATE_SCHEMA_VERSION};

pub fn write_hermes_command_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("hermes_command.json");
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
            .join("hermes")
            .join("status.json"),
    )
    .unwrap_or_else(|| json!({}));
    let providers = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("hermes")
            .join("providers.json"),
    )
    .unwrap_or_else(|| json!({}));
    let subcomponents = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("hermes")
            .join("subcomponents.json"),
    )
    .unwrap_or_else(|| json!([]));
    let boardroom = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("boardroom.jsonl"),
        16,
    );
    let interruptions = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("interruptions.jsonl"),
        16,
    );
    let reroute_metrics = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("reroute_metrics.jsonl"),
        16,
    );
    let reroute_acks = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("reroute_acks.jsonl"),
        16,
    );
    let decision_metrics = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("decision_metrics.jsonl"),
        16,
    );
    let council_sessions = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("hermes")
            .join("council_sessions.jsonl"),
        16,
    );
    let matrix_contract = read_json_file(workspace_root.join("core/state/matrix_boardrooms.json"))
        .unwrap_or_else(|| json!({}));
    let github_repo_integration =
        read_json_file(workspace_root.join("core/state/github_repo_integration.json"))
            .unwrap_or_else(|| json!({}));
    let fleet_bootstrap_recovery =
        read_json_file(workspace_root.join("core/state/fleet_bootstrap_recovery.json"))
            .unwrap_or_else(|| json!({}));

    let deferred_reroutes = reroute_metrics
        .iter()
        .filter(|entry| entry.get("event").and_then(Value::as_str) == Some("deferred"))
        .count();
    let denied_interrupts = interruptions
        .iter()
        .filter(|entry| entry.get("policy_authorized").and_then(Value::as_bool) == Some(false))
        .count();
    let open_councils = council_sessions
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("open"))
        .count();
    let rooms = matrix_contract
        .get("rooms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let activation = matrix_contract
        .get("activation_requirements")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let routing_contract = matrix_contract
        .get("routing_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let bridge_contracts = matrix_contract
        .get("bridge_contracts")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let matrix_enabled = matrix_contract
        .get("defaults")
        .and_then(|value| value.get("provider"))
        .and_then(Value::as_str)
        == Some("matrix");
    let matrix_ready = activation
        .get("federated_rooms_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let github_summary = github_repo_integration
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let framework_surfaces = github_repo_integration
        .get("framework_surfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let communications_registry_tools = github_repo_integration
        .get("registry_tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter(|tool| {
                    tool.get("package_enablement")
                        .and_then(|value| value.get("integration_lane"))
                        .and_then(Value::as_str)
                        .map(|lane| lane.contains("communications") || lane.contains("charon"))
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "hermes_command_projection",
        "status": status,
        "providers": providers,
        "subcomponents": subcomponents,
        "communications": {
            "boardroom_contract": {
                "source": "core/state/matrix_boardrooms.json",
                "provider": "matrix",
                "client_surface": matrix_contract
                    .get("defaults")
                    .and_then(|value| value.get("client_surface"))
                    .cloned()
                    .unwrap_or_else(|| json!("element")),
                "root_space": matrix_contract
                    .get("root_space")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
                "rooms": rooms,
                "routing_contract": routing_contract,
                "bridge_contracts": bridge_contracts,
                "activation_requirements": activation,
                "room_count": rooms.len(),
                "matrix_ready": matrix_ready
            }
        },
        "github_repo_integration": {
            "source": "core/state/github_repo_integration.json",
            "summary": github_summary,
            "framework_surfaces": framework_surfaces,
            "communications_registry_tools": communications_registry_tools
        },
        "fleet_recovery": {
            "summary": fleet_bootstrap_recovery
                .get("summary")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "targets": fleet_bootstrap_recovery
                .get("targets")
                .cloned()
                .unwrap_or_else(|| json!([]))
        },
        "recent_activity": {
            "boardroom": boardroom,
            "interruptions": interruptions,
            "reroute_metrics": reroute_metrics,
            "reroute_acks": reroute_acks,
            "decision_metrics": decision_metrics,
            "council_sessions": council_sessions,
            "counts": {
                "boardroom_posts": count_jsonl_lines(
                    &workspace_root.join("data").join("hermes").join("boardroom.jsonl")
                ),
                "boardroom_contract_rooms": rooms.len(),
                "deferred_reroutes": deferred_reroutes,
                "denied_interrupts": denied_interrupts,
                "open_councils": open_councils
            }
        },
        "arda_hints": {
            "primary_panel": "boardroom_and_comms",
            "boardroom_section": if matrix_enabled { "matrix_boardrooms" } else { "council_and_interrupts" },
            "alert_on_matrix_activation_gap": matrix_enabled && !matrix_ready,
            "alert_on_interrupt_denials": denied_interrupts > 0,
            "alert_on_reroute_backpressure": deferred_reroutes > 0,
            "alert_on_fleet_recovery_failure": fleet_bootstrap_recovery
                .get("summary")
                .and_then(|value| value.get("restart_failed_total"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
