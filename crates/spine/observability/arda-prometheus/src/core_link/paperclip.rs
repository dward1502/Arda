use super::{
    latest_jsonl_entries_by_source_id, latest_task_rows_by_id, read_json_file,
    CORE_STATE_SCHEMA_VERSION,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn write_paperclip_alignment_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("paperclip_alignment.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let governance = read_json_file(core_root.join("state").join("governance_runtime.json"))
        .unwrap_or_else(|| json!({}));
    let lockdown = read_json_file(core_root.join("state").join("control_plane_lockdown.json"))
        .unwrap_or_else(|| json!({}));
    let charon = read_json_file(core_root.join("state").join("charon_router.json"))
        .unwrap_or_else(|| json!({}));
    let hermes = read_json_file(core_root.join("state").join("hermes_command.json"))
        .unwrap_or_else(|| json!({}));
    let hades = read_json_file(core_root.join("state").join("hades_lifecycle.json"))
        .unwrap_or_else(|| json!({}));
    let athena = read_json_file(core_root.join("state").join("athena_runtime.json"))
        .unwrap_or_else(|| json!({}));
    let queue_summary = read_json_file(core_root.join("state").join("queue_summary.json"))
        .unwrap_or_else(|| json!({}));
    let output_topology = read_json_file(core_root.join("state").join("output_topology.json"))
        .unwrap_or_else(|| json!({}));
    let output_accounting = read_json_file(core_root.join("state").join("output_accounting.json"))
        .unwrap_or_else(|| json!({}));
    let plan_map =
        read_json_file(core_root.join("state").join("plan_map.json")).unwrap_or_else(|| json!({}));

    let evidence_source_ids = [
        "src_a5e43b15",
        "src_21116bd0",
        "src_23b9ebad",
        "src_c06996a5",
        "src_dd9de6e2",
        "src_1382c99a",
    ];
    let evidence_sources = latest_jsonl_entries_by_source_id(
        &workspace_root
            .join("data")
            .join("athena")
            .join("policy_readiness.jsonl"),
        &evidence_source_ids,
    );
    let evidence_rows = evidence_source_ids
        .iter()
        .map(|source_id| {
            let row = evidence_sources
                .get(*source_id)
                .cloned()
                .unwrap_or_else(|| json!({}));
            let gate = row.get("gate").cloned().unwrap_or_else(|| json!({}));
            json!({
                "source_id": source_id,
                "policy_readiness": row.get("policy_readiness").cloned().unwrap_or(json!("unknown")),
                "confidence": gate
                    .get("observed")
                    .and_then(|value| value.get("confidence"))
                    .cloned()
                    .unwrap_or(json!(0.0)),
                "opposition_coverage": gate
                    .get("observed")
                    .and_then(|value| value.get("opposition_coverage"))
                    .cloned()
                    .unwrap_or(json!(0)),
                "triad_passed": gate
                    .get("observed")
                    .and_then(|value| value.get("triad_passed"))
                    .cloned()
                    .unwrap_or(json!(false))
            })
        })
        .collect::<Vec<_>>();
    let evidence_policy_ready = evidence_rows
        .iter()
        .filter(|row| row.get("policy_readiness").and_then(Value::as_str) == Some("policy_ready"))
        .count();

    let comparison_task_ids = [
        "tsk_20260311_compare_paperclip_board_governance_and_budget_de",
        "tsk_20260311_assess_paperclip_heartbeat_and_bring_your_own_ag",
        "tsk_20260311_map_paperclip_deployment_auth_modes_to_arda",
        "tsk_20260311_evaluate_paperclip_multi_company_isolation_and_i",
    ];
    let comparison_tasks = latest_task_rows_by_id(
        &workspace_root
            .join("core")
            .join("projects")
            .join("tasks")
            .join("queue.jsonl"),
        &comparison_task_ids,
    );
    let comparison_task_rows = comparison_task_ids
        .iter()
        .map(|task_id| {
            comparison_tasks
                .get(*task_id)
                .cloned()
                .unwrap_or_else(|| json!({}))
        })
        .collect::<Vec<_>>();
    let comparison_tasks_open = comparison_task_rows
        .iter()
        .filter(|row| {
            !matches!(
                row.get("status").and_then(Value::as_str),
                Some("completed") | Some("cancelled")
            )
        })
        .count();

    let boardroom_posts = hermes
        .get("recent_activity")
        .and_then(|value| value.get("counts"))
        .and_then(|value| value.get("boardroom_posts"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let open_councils = hermes
        .get("recent_activity")
        .and_then(|value| value.get("counts"))
        .and_then(|value| value.get("open_councils"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let providers_online = hermes
        .get("providers")
        .and_then(|value| value.get("online"))
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let provider_count = charon
        .get("arda_hints")
        .and_then(|value| value.get("provider_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let local_fallback_ready = charon
        .get("provider_pressure")
        .and_then(|value| value.get("local_fallback"))
        .and_then(|value| value.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let route_defaults = charon
        .get("routing_defaults")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let deployment_runtime_ready = lockdown
        .get("status")
        .and_then(|value| value.get("runtime_socket_contract_present"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let deployment_socket_ready = lockdown
        .get("status")
        .and_then(|value| value.get("required_sockets_live"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let deployment_quorum_ready = lockdown
        .get("status")
        .and_then(|value| value.get("destructive_quorum_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let queue_open = queue_summary
        .get("project_tasks")
        .and_then(|value| value.get("counts_by_status"))
        .and_then(|value| value.get("queued"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let ledger_history_snapshots = output_topology
        .get("counts")
        .and_then(|value| value.get("metrics_history_snapshots"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mirrored_files = output_accounting
        .get("summary")
        .and_then(|value| value.get("mirrored_files"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let open_plan_count = plan_map
        .get("plans")
        .and_then(Value::as_array)
        .map(|plans| {
            plans
                .iter()
                .filter(|plan| {
                    plan.get("open_task_count")
                        .or_else(|| plan.get("openTaskCount"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        > 0
                })
                .count()
        })
        .unwrap_or(0);

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "paperclip_alignment_projection",
        "evidence": {
            "task_ids": comparison_task_ids,
            "source_ids": evidence_source_ids,
            "policy_ready_sources": evidence_policy_ready,
            "expected_policy_ready_sources": evidence_source_ids.len(),
            "sources": evidence_rows,
            "comparison_tasks": comparison_task_rows
        },
        "governance_alignment": {
            "paperclip_pattern": {
                "board_governance": true,
                "human_override_power": true,
                "budget_delegation": true,
                "company_control_plane": true
            },
            "arda_surface": {
                "active_ruleset": governance
                    .get("contracts")
                    .and_then(|value| value.get("active_ruleset"))
                    .and_then(|value| value.get("active_ruleset"))
                    .cloned()
                    .unwrap_or(json!("unknown")),
                "boardroom_posts": boardroom_posts,
                "open_councils": open_councils,
                "destructive_quorum_enabled": deployment_quorum_ready,
                "permission_profile_active": lockdown
                    .get("status")
                    .and_then(|value| value.get("permission_profile_active"))
                    .cloned()
                    .unwrap_or(json!(false)),
                "autonomy_ready": governance
                    .get("goal")
                    .and_then(|value| value.get("autonomy_ready"))
                    .cloned()
                    .unwrap_or(json!(false))
            },
            "derived": {
                "alignment_ready": boardroom_posts > 0 && deployment_quorum_ready,
                "follow_on_task_open": comparison_tasks
                    .get("tsk_20260311_compare_paperclip_board_governance_and_budget_de")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    != Some("completed")
            }
        },
        "edge_runtime_alignment": {
            "paperclip_pattern": {
                "adapter_types": ["process", "http"],
                "heartbeat_modes": ["scheduled", "event_driven"],
                "context_delivery": ["thin_ping", "fat_payload"],
                "bring_your_own_agents": true
            },
            "arda_surface": {
                "provider_count": provider_count,
                "local_fallback_ready": local_fallback_ready,
                "online_comms_providers": providers_online,
                "route_defaults": route_defaults,
                "recent_local_fallback_routes": charon
                    .get("status")
                    .and_then(|value| value.get("recent_local_fallback_routes"))
                    .cloned()
                    .unwrap_or(json!(0)),
                "boardroom_active": hermes
                    .get("status")
                    .and_then(|value| value.get("boardroom_active"))
                    .cloned()
                    .unwrap_or(json!(false))
            },
            "derived": {
                "alignment_ready": provider_count > 0 && local_fallback_ready && providers_online > 0,
                "follow_on_task_open": comparison_tasks
                    .get("tsk_20260311_assess_paperclip_heartbeat_and_bring_your_own_ag")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    != Some("completed")
            }
        },
        "deployment_posture_alignment": {
            "paperclip_pattern": {
                "modes": ["local_trusted", "authenticated_private", "authenticated_public"],
                "bootstrap_claim_flow": true,
                "private_network_posture": true,
                "public_hardening": true
            },
            "arda_surface": {
                "runtime_socket_contract_present": deployment_runtime_ready,
                "required_sockets_live": deployment_socket_ready,
                "destructive_quorum_enabled": deployment_quorum_ready,
                "active_permission_profile": lockdown
                    .get("contracts")
                    .and_then(|value| value.get("permission_profiles"))
                    .and_then(|value| value.get("active_profile"))
                    .cloned()
                    .unwrap_or(json!("unknown")),
                "orphan_repairs_pending": hades
                    .get("arda_hints")
                    .and_then(|value| value.get("pending_actions"))
                    .cloned()
                    .unwrap_or(json!(0))
            },
            "derived": {
                "alignment_ready": deployment_runtime_ready && deployment_socket_ready && deployment_quorum_ready,
                "follow_on_task_open": comparison_tasks
                    .get("tsk_20260311_map_paperclip_deployment_auth_modes_to_arda")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    != Some("completed")
            }
        },
        "ledger_topology_alignment": {
            "paperclip_pattern": {
                "multi_company_isolation": true,
                "immutable_audit_log": true,
                "template_export": true,
                "snapshot_export": true
            },
            "arda_surface": {
                "athena_policy_ready_count": athena
                    .get("status")
                    .and_then(|value| value.get("policy_ready_count"))
                    .cloned()
                    .unwrap_or(json!(0)),
                "task_queue_open": queue_open,
                "plan_nodes": plan_map
                    .get("plans")
                    .and_then(Value::as_array)
                    .map(|plans| plans.len())
                    .unwrap_or(0),
                "history_snapshots": ledger_history_snapshots,
                "mirrored_files": mirrored_files,
                "human_plan_root": plan_map
                    .get("human_plan_root")
                    .cloned()
                    .unwrap_or_else(|| json!("human/plans")),
                "core_plan_root": plan_map
                    .get("core_plan_root")
                    .cloned()
                    .unwrap_or_else(|| json!("core/projects/Plans"))
            },
            "derived": {
                "alignment_ready": evidence_policy_ready == evidence_source_ids.len()
                    && ledger_history_snapshots > 0
                    && mirrored_files > 0,
                "open_plan_count": open_plan_count,
                "follow_on_task_open": comparison_tasks
                    .get("tsk_20260311_evaluate_paperclip_multi_company_isolation_and_i")
                    .and_then(|value| value.get("status"))
                    .and_then(Value::as_str)
                    != Some("completed")
            }
        },
        "derived": {
            "comparison_tasks_open": comparison_tasks_open,
            "evidence_ready": evidence_policy_ready == evidence_source_ids.len(),
            "paperclip_readiness": {
                "governance": boardroom_posts > 0 && deployment_quorum_ready,
                "edge_runtime": provider_count > 0 && local_fallback_ready,
                "deployment": deployment_runtime_ready && deployment_socket_ready,
                "ledger_topology": ledger_history_snapshots > 0 && mirrored_files > 0
            }
        },
        "arda_hints": {
            "primary_panel": "paperclip_alignment",
            "boardroom_section": "paperclip_alignment",
            "alert_on_open_tasks": comparison_tasks_open > 0,
            "alert_on_evidence_gap": evidence_policy_ready != evidence_source_ids.len()
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
