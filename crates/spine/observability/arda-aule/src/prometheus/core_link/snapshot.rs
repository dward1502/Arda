#![cfg(feature = "full-cli")]
use super::{read_json_file, CORE_STATE_SCHEMA_VERSION};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(super) fn write_arda_snapshot(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("arda_snapshot.json");
    let generic_snapshot_path = core_root.join("state").join("system_snapshot.json");
    let state_root = core_root.join("state");
    let sections = json!({
        "world": read_json_file(state_root.join("world.json")).unwrap_or_else(|| json!({})),
        "system_manifest": read_json_file(state_root.join("system_manifest.json")).unwrap_or_else(|| json!({})),
        "system_control": read_json_file(state_root.join("system_control.json")).unwrap_or_else(|| json!({})),
        "athena": read_json_file(state_root.join("athena_runtime.json")).unwrap_or_else(|| json!({})),
        "guardhouse": read_json_file(state_root.join("warden_guardhouse.json")).unwrap_or_else(|| json!({})),
        "policy_authority": read_json_file(state_root.join("warden_policy_authority.json")).unwrap_or_else(|| json!({})),
        "edge_contract": read_json_file(state_root.join("warden_edge_contract.json")).unwrap_or_else(|| json!({})),
        "nightly_doctrine": read_json_file(state_root.join("warden_nightly_doctrine.json")).unwrap_or_else(|| json!({})),
        "manwe": read_json_file(state_root.join("manwe_router.json")).unwrap_or_else(|| json!({})),
        "hades": read_json_file(state_root.join("hades_lifecycle.json")).unwrap_or_else(|| json!({})),
        "hermes": read_json_file(state_root.join("hermes_command.json")).unwrap_or_else(|| json!({})),
        "mnemosyne": read_json_file(state_root.join("mnemosyne_continuity.json")).unwrap_or_else(|| json!({})),
        "memory_identity": read_json_file(state_root.join("memory_identity.json")).unwrap_or_else(|| json!({})),
        "memory_activity": read_json_file(state_root.join("memory_activity.json")).unwrap_or_else(|| json!({})),
        "memory_scopes": read_json_file(state_root.join("memory_scopes.json")).unwrap_or_else(|| json!({})),
        "apollo": read_json_file(state_root.join("apollo_runtime.json")).unwrap_or_else(|| json!({})),
        "plutus": read_json_file(state_root.join("plutus_runtime.json")).unwrap_or_else(|| json!({})),
        "oracle": read_json_file(state_root.join("oracle_runtime.json")).unwrap_or_else(|| json!({})),
        "business": read_json_file(state_root.join("business_runtime.json")).unwrap_or_else(|| json!({})),
        "personal": read_json_file(state_root.join("personal_runtime.json")).unwrap_or_else(|| json!({})),
        "human_context": read_json_file(state_root.join("human_context.json")).unwrap_or_else(|| json!({})),
        "queue_summary": read_json_file(state_root.join("queue_summary.json")).unwrap_or_else(|| json!({})),
        "repo_reorganization": read_json_file(state_root.join("repo_reorganization.json")).unwrap_or_else(|| json!({})),
        "output_topology": read_json_file(state_root.join("output_topology.json")).unwrap_or_else(|| json!({})),
        "output_accounting": read_json_file(state_root.join("output_accounting.json")).unwrap_or_else(|| json!({})),
        "package_health": read_json_file(state_root.join("package_health.json")).unwrap_or_else(|| json!({})),
        "package_enablement": read_json_file(state_root.join("package_enablement.json")).unwrap_or_else(|| json!({})),
        "package_runtime_activation": read_json_file(state_root.join("package_runtime_activation.json")).unwrap_or_else(|| json!({})),
        "operator_actions": read_json_file(state_root.join("operator_actions.json")).unwrap_or_else(|| json!({})),
        "extension_surface_contract": read_json_file(state_root.join("extension_surface_contract.json")).unwrap_or_else(|| json!({})),
        "extension_activation_backlog": read_json_file(state_root.join("extension_activation_backlog.json")).unwrap_or_else(|| json!({})),
        "communication_adapter_contract": read_json_file(state_root.join("communication_adapter_contract.json")).unwrap_or_else(|| json!({})),
        "aipkg_marketplace_separation_contract": read_json_file(state_root.join("aipkg_marketplace_separation_contract.json")).unwrap_or_else(|| json!({})),
        "network_native_node_onboarding_contract": read_json_file(state_root.join("network_native_node_onboarding_contract.json")).unwrap_or_else(|| json!({})),
        "edge_enrollment_plan": read_json_file(state_root.join("edge_enrollment_plan.json")).unwrap_or_else(|| json!({})),
        "task_agent_boundaries": read_json_file(state_root.join("task_agent_boundaries.json")).unwrap_or_else(|| json!({})),
        "runtime_settings": read_json_file(state_root.join("runtime_settings.json")).unwrap_or_else(|| json!({})),
        "control_plane_lockdown": read_json_file(state_root.join("control_plane_lockdown.json")).unwrap_or_else(|| json!({})),
        "governance_runtime": read_json_file(state_root.join("governance_runtime.json")).unwrap_or_else(|| json!({})),
        "operations_flow": read_json_file(state_root.join("operations_flow.json")).unwrap_or_else(|| json!({})),
        "paperclip_alignment": read_json_file(state_root.join("paperclip_alignment.json")).unwrap_or_else(|| json!({})),
        "escalation_runtime": read_json_file(state_root.join("escalation_runtime.json")).unwrap_or_else(|| json!({})),
        "soterion_render_contract": read_json_file(state_root.join("soterion_render_contract.json")).unwrap_or_else(|| json!({})),
        "storage_pressure": read_json_file(state_root.join("storage_pressure.json")).unwrap_or_else(|| json!({})),
        "fleet_runtime": read_json_file(state_root.join("fleet_runtime.json")).unwrap_or_else(|| json!({})),
        "fleet_nodes": read_json_file(state_root.join("fleet_nodes.json")).unwrap_or_else(|| json!({})),
        "fleet_models": read_json_file(state_root.join("fleet_models.json")).unwrap_or_else(|| json!({})),
        "fleet_health": read_json_file(state_root.join("fleet_health.json")).unwrap_or_else(|| json!({})),
        "fleet_bootstrap_recovery": read_json_file(state_root.join("fleet_bootstrap_recovery.json")).unwrap_or_else(|| json!({})),
        "fleet_hardware": read_json_file(state_root.join("fleet_hardware.json")).unwrap_or_else(|| json!({})),
        "fleet_backbone": read_json_file(state_root.join("fleet_backbone.json")).unwrap_or_else(|| json!({}))
    });
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "arda_snapshot_projection",
        "primary_entrypoint": true,
        "sections": sections,
        "status": {
            "world_ready": sections["world"].get("system").and_then(|v| v.get("status")).and_then(Value::as_str) == Some("READY"),
            "athena_projection_ready": sections["athena"].get("authority").and_then(Value::as_str) == Some("athena_runtime_projection"),
            "business_projection_ready": sections["business"].get("authority").and_then(Value::as_str) == Some("business_runtime_projection"),
            "personal_projection_ready": sections["personal"].get("authority").and_then(Value::as_str) == Some("personal_runtime_projection"),
            "human_projection_ready": sections["human_context"].get("authority").and_then(Value::as_str) == Some("human_context_projection"),
            "queue_projection_ready": sections["queue_summary"].get("authority").and_then(Value::as_str) == Some("queue_summary_projection"),
            "repo_phase_1_complete": sections["repo_reorganization"].get("status").and_then(Value::as_str) == Some("completed"),
            "output_topology_ready": sections["output_topology"].get("authority").and_then(Value::as_str) == Some("output_topology_projection"),
            "output_accounting_ready": sections["output_accounting"].get("authority").and_then(Value::as_str) == Some("output_accounting_projection"),
            "package_observation_ready": sections["package_health"].get("authority").and_then(Value::as_str) == Some("package_observation_export"),
            "package_enablement_ready": sections["package_enablement"].get("authority").and_then(Value::as_str) == Some("package_enablement_projection"),
            "package_runtime_activation_ready": sections["package_runtime_activation"].get("authority").and_then(Value::as_str) == Some("package_enablement + live wrapper/probe checks"),
            "operator_actions_ready": sections["operator_actions"].get("authority").and_then(Value::as_str) == Some("operator_actions_projection"),
            "extension_surface_contract_ready": sections["extension_surface_contract"].get("authority").and_then(Value::as_str) == Some("framework_digestion_materialization"),
            "extension_activation_backlog_ready": sections["extension_activation_backlog"].get("authority").and_then(Value::as_str) == Some("extension_activation_backlog_export"),
            "communication_adapter_contract_ready": sections["communication_adapter_contract"].get("authority").and_then(Value::as_str) == Some("hermes_adapter_materialization"),
            "aipkg_marketplace_separation_contract_ready": sections["aipkg_marketplace_separation_contract"].get("authority").and_then(Value::as_str) == Some("aipkg_marketplace_separation_export"),
            "network_native_node_onboarding_contract_ready": sections["network_native_node_onboarding_contract"].get("authority").and_then(Value::as_str) == Some("network_native_node_onboarding_contract_export"),
            "edge_enrollment_plan_ready": sections["edge_enrollment_plan"].get("authority").and_then(Value::as_str) == Some("edge_enrollment_plan_export"),
            "task_agent_boundaries_ready": sections["task_agent_boundaries"].get("authority").and_then(Value::as_str) == Some("task_agent_boundary_export"),
            "runtime_settings_ready": sections["runtime_settings"].get("authority").and_then(Value::as_str) == Some("runtime_settings_projection"),
            "control_plane_lockdown_ready": sections["control_plane_lockdown"]["status"]["lockdown_ready"].as_bool() == Some(true),
            "governance_runtime_ready": sections["governance_runtime"].get("authority").and_then(Value::as_str) == Some("governance_runtime_projection"),
            "operations_flow_ready": sections["operations_flow"].get("authority").and_then(Value::as_str) == Some("operations_flow_projection"),
            "paperclip_alignment_ready": sections["paperclip_alignment"].get("authority").and_then(Value::as_str) == Some("paperclip_alignment_projection"),
            "escalation_runtime_ready": sections["escalation_runtime"].get("authority").and_then(Value::as_str) == Some("escalation_runtime_projection"),
            "soterion_render_contract_ready": sections["soterion_render_contract"].get("authority").and_then(Value::as_str) == Some("soterion_render_projection"),
            "fleet_runtime_ready": sections["fleet_runtime"].get("authority").and_then(Value::as_str) == Some("fleet_runtime_projection"),
            "fleet_hardware_ready": sections["fleet_hardware"].get("authority").and_then(Value::as_str) == Some("fleet_hardware_projection"),
            "fleet_backbone_ready": sections["fleet_backbone"].get("authority").and_then(Value::as_str) == Some("fleet_backbone_projection")
        }
    });
    let _ = fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
    let _ = fs::write(
        generic_snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
