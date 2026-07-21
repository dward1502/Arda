#![cfg(feature = "full-cli")]
use super::{
    merge_fleet_nodes, read_edge_targets, read_fleet_config_meta, read_fleet_config_nodes,
    read_json_file, read_toml_as_json, CORE_STATE_SCHEMA_VERSION,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn write_fleet_runtime_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("fleet_runtime.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let fleet_scan = read_json_file(workspace_root.join("data/prometheus/fleet_control_last.json"))
        .unwrap_or_else(|| json!({"status":"unknown","network":{"tailscale_ok":false}}));
    let local_informant =
        read_json_file(workspace_root.join("data/fleet/informants/local_last.json"))
            .unwrap_or_else(
                || json!({"tailscale_ok":false,"ollama_ok":false,"llm_local_models":[]}),
            );
    let configured_targets = read_edge_targets(&workspace_root.join("core/edge/targets.toml"))
        .or_else(|| read_edge_targets(&workspace_root.join("core/edge/targets.example.toml")))
        .unwrap_or_default();
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let merged_nodes = merge_fleet_nodes(
        &configured_nodes,
        &fleet_scan
            .get("fleet_nodes_full")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "fleet_runtime_projection",
        "subsystem_model": {
            "name": "fleet_observation",
            "separate_from_warden": true,
            "warden_boundary": "guardian_enforcement_and_quarantine",
            "purpose": "node_and_model_observation_for_agnostic_consumers"
        },
        "status": fleet_scan.get("status").cloned().unwrap_or_else(|| json!("unknown")),
        "network": fleet_scan.get("network").cloned().unwrap_or_else(|| json!({})),
        "fleet_config": read_fleet_config_meta(&workspace_root.join("config/fleet.toml")),
        "inventory": {
            "configured_targets_count": configured_targets.len(),
            "configured_nodes_count": configured_nodes.len(),
            "merged_nodes_count": merged_nodes.len(),
            "configured_targets": configured_targets,
            "configured_nodes": configured_nodes,
            "merged_nodes": merged_nodes
        },
        "local_informant": local_informant,
        "llm_inventory": fleet_scan.get("llm_inventory").cloned().unwrap_or_else(|| json!({}))
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_fleet_nodes_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("fleet_nodes.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let fleet_scan = read_json_file(workspace_root.join("data/prometheus/fleet_control_last.json"))
        .unwrap_or_else(|| json!({"fleet_nodes_full":[]}));
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let merged_nodes = merge_fleet_nodes(
        &configured_nodes,
        &fleet_scan
            .get("fleet_nodes_full")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let matched_total = merged_nodes
        .iter()
        .filter(|node| node.get("matched").and_then(Value::as_bool) == Some(true))
        .count();
    let online_total = merged_nodes
        .iter()
        .filter(|node| node.get("online").and_then(Value::as_bool) == Some(true))
        .count();
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "fleet_nodes_projection",
        "counts": {
            "configured_total": configured_nodes.len(),
            "matched_total": matched_total,
            "online_total": online_total,
            "unconfigured_observed_total": merged_nodes
                .iter()
                .filter(|node| node.get("configured").is_some_and(Value::is_null))
                .count()
        },
        "nodes": merged_nodes
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_fleet_models_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("fleet_models.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let fleet_scan = read_json_file(workspace_root.join("data/prometheus/fleet_control_last.json"))
        .unwrap_or_else(|| json!({"llm_inventory":{"local_ollama_models":[]}}));
    let local_informant =
        read_json_file(workspace_root.join("data/fleet/informants/local_last.json"))
            .unwrap_or_else(|| json!({"llm_local_models":[]}));
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "fleet_models_projection",
        "local_models": {
            "command_tower": fleet_scan
                .get("llm_inventory")
                .and_then(|value| value.get("local_ollama_models"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            "informant": local_informant.get("llm_local_models").cloned().unwrap_or_else(|| json!([]))
        },
        "provider_inventory": fleet_scan
            .get("llm_inventory")
            .and_then(|value| value.get("provider_inventory"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        "configured_nodes": configured_nodes
            .into_iter()
            .map(|node| {
                json!({
                    "id": node.get("id").cloned().unwrap_or(Value::Null),
                    "display_name": node.get("display_name").cloned().unwrap_or(Value::Null),
                    "hostname": node.get("hostname").cloned().unwrap_or(Value::Null),
                    "llm_runtime": node.get("llm_runtime").cloned().unwrap_or(Value::Null)
                })
            })
            .collect::<Vec<_>>()
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_fleet_hardware_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("fleet_hardware.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let local_informant =
        read_json_file(workspace_root.join("data/fleet/informants/local_last.json"))
            .unwrap_or_else(|| json!({}));
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "fleet_hardware_projection",
        "local_node": {
            "node_id": local_informant.get("node_id").cloned().unwrap_or(Value::Null),
            "hostname": local_informant.get("hostname").cloned().unwrap_or(Value::Null),
            "hardware": local_informant.get("hardware").cloned().unwrap_or_else(|| json!({})),
            "hardware_errors": local_informant
                .get("hardware_errors")
                .cloned()
                .unwrap_or_else(|| json!({}))
        },
        "configured_nodes": configured_nodes
            .into_iter()
            .map(|node| {
                json!({
                    "id": node.get("id").cloned().unwrap_or(Value::Null),
                    "display_name": node.get("display_name").cloned().unwrap_or(Value::Null),
                    "hostname": node.get("hostname").cloned().unwrap_or(Value::Null),
                    "node_class": node.get("node_class").cloned().unwrap_or(Value::Null),
                    "llm_runtime": node.get("llm_runtime").cloned().unwrap_or(Value::Null),
                    "enrollment_status": node.get("enrollment_status").cloned().unwrap_or(Value::Null)
                })
            })
            .collect::<Vec<_>>()
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_fleet_health_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("fleet_health.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let fleet_scan = read_json_file(workspace_root.join("data/prometheus/fleet_control_last.json"))
        .unwrap_or_else(|| json!({"status":"unknown","network":{"tailscale_ok":false}}));
    let local_informant =
        read_json_file(workspace_root.join("data/fleet/informants/local_last.json"))
            .unwrap_or_else(|| json!({"tailscale_ok":false,"ollama_ok":false}));
    let merged_nodes = merge_fleet_nodes(
        &read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default(),
        &fleet_scan
            .get("fleet_nodes_full")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let cleanup_summary = summarize_connection_cleanup(
        fleet_scan.get("connection_cleanup").unwrap_or(&Value::Null),
        &configured_nodes,
    );
    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "fleet_health_projection",
        "status": fleet_scan.get("status").cloned().unwrap_or_else(|| json!("unknown")),
        "network": fleet_scan.get("network").cloned().unwrap_or_else(|| json!({})),
        "local_probe": {
            "tailscale_ok": local_informant.get("tailscale_ok").and_then(Value::as_bool).unwrap_or(false),
            "ollama_ok": local_informant.get("ollama_ok").and_then(Value::as_bool).unwrap_or(false),
            "tailscale_error": local_informant.get("tailscale_error").cloned().unwrap_or(Value::Null),
            "ollama_error": local_informant.get("ollama_error").cloned().unwrap_or(Value::Null)
        },
        "counts": {
            "configured_nodes_total": merged_nodes.iter().filter(|node| !node.get("configured").is_some_and(Value::is_null)).count(),
            "configured_nodes_online": merged_nodes.iter().filter(|node| {
                !node.get("configured").is_some_and(Value::is_null)
                    && node.get("online").and_then(Value::as_bool) == Some(true)
            }).count(),
            "observed_unconfigured_total": merged_nodes.iter().filter(|node| node.get("configured").is_some_and(Value::is_null)).count(),
            "stale_candidates_total": fleet_scan
                .get("connection_cleanup")
                .and_then(|value| value.get("stale_offline_total"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        },
        "connection_cleanup": fleet_scan.get("connection_cleanup").cloned().unwrap_or_else(|| json!({})),
        "cleanup_summary": cleanup_summary,
        "findings": [
            if fleet_scan
                .get("network")
                .and_then(|value| value.get("tailscale_ok"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            { "tailscale_mesh_ok" } else { "tailscale_mesh_degraded" },
            if local_informant.get("ollama_ok").and_then(Value::as_bool).unwrap_or(false)
            { "local_llm_inventory_visible" } else { "local_llm_inventory_unavailable" }
        ]
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn summarize_connection_cleanup(connection_cleanup: &Value, configured_nodes: &[Value]) -> Value {
    let stale_candidates = connection_cleanup
        .get("stale_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let stale_safe_review = stale_candidates
        .iter()
        .filter(|candidate| candidate.get("active").and_then(Value::as_bool) != Some(true))
        .map(|candidate| {
            let superseded_by = find_superseding_configured_node(candidate, configured_nodes);
            json!({
                "device_identity": {
                    "hostname": candidate.get("hostname").cloned().unwrap_or(Value::Null),
                    "dns_name": candidate.get("dns_name").cloned().unwrap_or(Value::Null),
                    "node_id": candidate.get("node_id").cloned().unwrap_or(Value::Null)
                },
                "hostname": candidate.get("hostname").cloned().unwrap_or(Value::Null),
                "dns_name": candidate.get("dns_name").cloned().unwrap_or(Value::Null),
                "node_id": candidate.get("node_id").cloned().unwrap_or(Value::Null),
                "last_seen": candidate.get("last_seen").cloned().unwrap_or(Value::Null),
                "last_seen_age_days": candidate.get("last_seen_age_days").cloned().unwrap_or(Value::Null),
                "tailscale_ips": candidate.get("tailscale_ips").cloned().unwrap_or_else(|| json!([])),
                "superseded_by_configured_node": superseded_by,
                "review_action": "confirm_retired_before_removing_from_tailscale_and_fleet_config"
            })
        })
        .collect::<Vec<_>>();
    let offline_recent_total = connection_cleanup
        .get("offline_recent_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stale_total = connection_cleanup
        .get("stale_offline_total")
        .or_else(|| connection_cleanup.get("stale_candidates_total"))
        .and_then(Value::as_u64)
        .unwrap_or(stale_candidates.len() as u64);
    json!({
        "status": if !stale_safe_review.is_empty() {
            "stale_review_available"
        } else if offline_recent_total > 0 {
            "recent_offline_observed"
        } else {
            "clear"
        },
        "stale_candidates_total": stale_total,
        "offline_recent_total": offline_recent_total,
        "safe_review_candidates_total": stale_safe_review.len(),
        "safe_review_candidates": stale_safe_review,
        "safe_action": if stale_total > 0 {
            "review_stale_inactive_nodes_before_deleting_tailscale_or_config_entries"
        } else {
            "no_cleanup_action_required"
        }
    })
}

fn find_superseding_configured_node(candidate: &Value, configured_nodes: &[Value]) -> Value {
    let candidate_hostname = candidate
        .get("hostname")
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase());
    let candidate_ips = candidate
        .get("tailscale_ips")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    configured_nodes
        .iter()
        .find_map(|node| {
            let enrollment = node
                .get("enrollment_status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if !(enrollment.starts_with("active") || enrollment == "controlled") {
                return None;
            }
            let config_hostname = node
                .get("hostname")
                .and_then(Value::as_str)
                .map(|value| value.to_ascii_lowercase());
            if config_hostname != candidate_hostname {
                return None;
            }
            let config_ip = node.get("tailscale_ip").and_then(Value::as_str);
            let same_ip = config_ip.is_some_and(|ip| {
                candidate_ips
                    .iter()
                    .any(|candidate_ip| candidate_ip.as_str() == Some(ip))
            });
            if same_ip {
                return None;
            }
            Some(json!({
                "id": node.get("id").cloned().unwrap_or(Value::Null),
                "role": node.get("role").cloned().unwrap_or(Value::Null),
                "display_name": node.get("display_name").cloned().unwrap_or(Value::Null),
                "hostname": node.get("hostname").cloned().unwrap_or(Value::Null),
                "tailscale_name": node.get("tailscale_name").cloned().unwrap_or(Value::Null),
                "tailscale_ip": node.get("tailscale_ip").cloned().unwrap_or(Value::Null),
                "enrollment_status": node.get("enrollment_status").cloned().unwrap_or(Value::Null)
            }))
        })
        .unwrap_or(Value::Null)
}

pub(super) fn write_fleet_backbone_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("fleet_backbone.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let edge_targets = read_edge_targets(&workspace_root.join("core/edge/targets.toml"))
        .or_else(|| read_edge_targets(&workspace_root.join("core/edge/targets.example.toml")))
        .unwrap_or_default();
    let model_profiles = read_toml_as_json(workspace_root.join("core/edge/model_profiles.toml"))
        .unwrap_or_else(|| json!({ "profiles": {} }));

    let configured_backbone = configured_nodes
        .iter()
        .find(|node| node.get("node_class").and_then(Value::as_str) == Some("backbone_compute"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let target_backbone = edge_targets
        .iter()
        .find(|node| node.get("node_class").and_then(Value::as_str) == Some("backbone_compute"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let deployment_phase = configured_backbone
        .get("deployment_phase")
        .or_else(|| target_backbone.get("deployment_phase"))
        .cloned()
        .unwrap_or_else(|| json!("unknown"));
    let placement_plan = configured_backbone
        .get("placement_plan")
        .or_else(|| target_backbone.get("placement_plan"))
        .cloned()
        .unwrap_or_else(|| json!("unspecified"));

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "fleet_backbone_projection",
        "status": configured_backbone
            .get("enrollment_status")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "deployment": {
            "phase": deployment_phase,
            "placement_plan": placement_plan
        },
        "backbone_node": configured_backbone,
        "edge_target": target_backbone,
        "routing_posture": {
            "manwes_role": configured_backbone.get("manwes_role").cloned().unwrap_or_else(|| json!("primary_router")),
            "oracle_role": configured_backbone.get("oracle_role").cloned().unwrap_or_else(|| json!("primary_reasoning")),
            "athena_role": configured_backbone.get("athena_role").cloned().unwrap_or_else(|| json!("deep_ingest_and_digest")),
            "plutus_role": configured_backbone.get("plutus_role").cloned().unwrap_or_else(|| json!("cost_and_joule_accounting"))
        },
        "model_profiles": model_profiles.get("profiles").cloned().unwrap_or_else(|| json!({})),
        "expected_capabilities": configured_backbone
            .get("capabilities")
            .cloned()
            .unwrap_or_else(|| target_backbone.get("capabilities").cloned().unwrap_or_else(|| json!([])))
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir");
        }
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn fleet_health_projection_counts_configured_and_unconfigured_nodes() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state dir");

        write_file(
            &dir.path().join("config/fleet.toml"),
            r#"
[[nodes]]
id = "node-backbone"
role = "backbone_inference"
hostname = "annunimas-server"
tailscale_ip = "100.64.0.2"
node_class = "backbone_compute"
enrollment_status = "active"
llm_runtime = "multi_gpu_sovereign_backbone"
"#,
        );
        write_file(
            &dir.path().join("data/prometheus/fleet_control_last.json"),
            r#"{
  "status": "degraded",
  "network": { "tailscale_ok": false },
  "connection_cleanup": {
    "offline_recent_total": 1,
    "stale_offline_total": 2,
    "stale_candidates": [
      { "hostname": "retired-node", "active": false, "last_seen_age_days": 20.0, "tailscale_ips": ["100.64.0.5"] },
      { "hostname": "active-node", "active": true, "last_seen_age_days": 20.0, "tailscale_ips": ["100.64.0.6"] }
    ]
  },
  "fleet_nodes_full": [
    { "id": "node-backbone", "hostname": "annunimas-server", "online": true, "tailscale_ips": ["100.64.0.2"] },
    { "id": "node-unconfigured", "hostname": "rogue-node", "online": false, "tailscale_ips": ["100.64.0.99"] }
  ]
}"#,
        );
        write_file(
            &dir.path().join("data/fleet/informants/local_last.json"),
            r#"{
  "tailscale_ok": true,
  "ollama_ok": false,
  "ollama_error": "socket timeout"
}"#,
        );

        write_fleet_health_projection(&core_root);

        let projection: Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_health.json"))
                .expect("read projection"),
        )
        .expect("projection json");

        assert_eq!(projection["authority"], "fleet_health_projection");
        assert_eq!(projection["status"], "degraded");
        assert_eq!(projection["counts"]["configured_nodes_total"], 1);
        assert_eq!(projection["counts"]["configured_nodes_online"], 1);
        assert_eq!(projection["counts"]["observed_unconfigured_total"], 1);
        assert_eq!(projection["counts"]["stale_candidates_total"], 2);
        assert_eq!(projection["local_probe"]["tailscale_ok"], true);
        assert_eq!(projection["local_probe"]["ollama_ok"], false);
        assert_eq!(
            projection["cleanup_summary"]["status"],
            "stale_review_available"
        );
        assert_eq!(
            projection["cleanup_summary"]["safe_review_candidates_total"],
            1
        );
        assert_eq!(
            projection["cleanup_summary"]["safe_review_candidates"][0]["hostname"],
            "retired-node"
        );
        assert_eq!(
            projection["cleanup_summary"]["safe_review_candidates"][0]
                ["superseded_by_configured_node"],
            Value::Null
        );
        assert_eq!(projection["findings"][0], "tailscale_mesh_degraded");
        assert_eq!(projection["findings"][1], "local_llm_inventory_unavailable");
    }

    #[test]
    fn fleet_backbone_projection_prefers_configured_backbone_and_target_fallbacks() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state dir");

        write_file(
            &dir.path().join("config/fleet.toml"),
            r#"
[[nodes]]
id = "node-backbone"
role = "backbone_inference"
hostname = "annunimas-server"
tailscale_ip = "100.64.0.2"
node_class = "backbone_compute"
enrollment_status = "active"
llm_runtime = "multi_gpu_sovereign_backbone"
placement_plan = "rack-a"
capabilities = ["routing", "reasoning"]
"#,
        );
        write_file(
            &core_root.join("edge/targets.toml"),
            r#"
[[node]]
id = "node-backbone"
role = "backbone_inference"
hostname = "annunimas-server"
tailscale_ip = "100.64.0.2"
ssh_user = "annunimas"
node_class = "backbone_compute"
enrollment_status = "active"
llm_runtime = "multi_gpu_sovereign_backbone"
notes = "phase-2 rack-z edge-routing"
"#,
        );
        write_file(
            &core_root.join("edge/model_profiles.toml"),
            r#"
[profiles.primary]
model = "qwen3"
"#,
        );

        write_fleet_backbone_projection(&core_root);

        let projection: Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_backbone.json"))
                .expect("read backbone projection"),
        )
        .expect("projection json");

        assert_eq!(projection["authority"], "fleet_backbone_projection");
        assert_eq!(projection["status"], "active");
        assert_eq!(projection["deployment"]["phase"], "unknown");
        assert_eq!(projection["deployment"]["placement_plan"], "unspecified");
        assert_eq!(projection["backbone_node"]["id"], "node-backbone");
        assert_eq!(projection["edge_target"]["id"], "node-backbone");
        assert_eq!(
            projection["routing_posture"]["manwes_role"],
            "primary_router"
        );
        assert_eq!(projection["expected_capabilities"], json!([]));
        assert_eq!(projection["model_profiles"]["primary"]["model"], "qwen3");
    }
}
