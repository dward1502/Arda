#![cfg(feature = "full-cli")]
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    read_env_assignment, read_json_file, read_toml_as_json, read_yaml_as_json, rel_path,
    summarize_env_file, CORE_STATE_SCHEMA_VERSION,
};

pub fn write_governance_runtime_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("governance_runtime.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let governance = read_json_file(
        workspace_root
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("governance")
            .join("signals.json"),
    )
    .unwrap_or_else(|| json!({}));
    let autonomy_runtime = read_json_file(core_root.join("state").join("autonomy_runtime.json"))
        .unwrap_or_else(|| json!({}));
    let active_ruleset = read_json_file(core_root.join("state").join("active_ruleset.json"))
        .unwrap_or_else(|| json!({}));
    let human_augmentation_runtime = read_json_file(
        core_root
            .join("state")
            .join("human_augmentation_runtime.json"),
    )
    .unwrap_or_else(|| json!({}));

    let signals = governance
        .get("signals")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let goal = governance.get("goal").cloned().unwrap_or_else(|| json!({}));
    let control = governance
        .get("control")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let autonomy_score = signals
        .get("autonomy_observation_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let autonomy_threshold = goal
        .get("autonomy_threshold")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let joulework = signals
        .get("avg_joulework")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let love_eq = signals
        .get("avg_love_eq")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let triad = signals
        .get("triad_pass_rate")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let bacon = signals
        .get("bacon_lite_recent_confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let thresholds = control
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut attention = goal
        .get("attention_required")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if autonomy_score < autonomy_threshold {
        attention.push(json!("autonomy_observation_below_threshold"));
    }
    if joulework
        < thresholds
            .get("joulework_min")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    {
        attention.push(json!("joulework_below_threshold"));
    }
    if love_eq
        < thresholds
            .get("love_equation_min")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    {
        attention.push(json!("love_equation_below_threshold"));
    }

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "governance_runtime_projection",
        "control": control,
        "goal": goal,
        "signals": signals,
        "contracts": {
            "autonomy_runtime": autonomy_runtime,
            "active_ruleset": active_ruleset,
            "human_augmentation_runtime": human_augmentation_runtime
        },
        "derived": {
            "autonomy_gap": (autonomy_threshold - autonomy_score).max(0.0),
            "signal_posture": {
                "joulework": joulework,
                "love_equation": love_eq,
                "triad_pass_rate": triad,
                "bacon_lite_confidence": bacon
            },
            "human_augmentation": {
                "pending_total": human_augmentation_runtime
                    .get("summary")
                    .and_then(|v| v.get("pending_total"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                "approved_total": human_augmentation_runtime
                    .get("summary")
                    .and_then(|v| v.get("approved_total"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            },
            "thresholds_met": {
                "autonomy": autonomy_score >= autonomy_threshold,
                "joulework": joulework >= thresholds.get("joulework_min").and_then(Value::as_f64).unwrap_or(0.0),
                "love_equation": love_eq >= thresholds.get("love_equation_min").and_then(Value::as_f64).unwrap_or(0.0),
                "provider_health": signals.get("provider_health").and_then(Value::as_f64).unwrap_or(0.0)
                    >= thresholds.get("provider_health_min").and_then(Value::as_f64).unwrap_or(0.0),
                "queue_health": signals.get("queue_health").and_then(Value::as_f64).unwrap_or(0.0)
                    >= thresholds.get("queue_health_min").and_then(Value::as_f64).unwrap_or(0.0)
            },
            "attention_required": attention
        },
        "arda_hints": {
            "primary_panel": "governance_runtime",
            "boardroom_section": "triad_joule_love",
            "alert_on_autonomy_gap": autonomy_score < autonomy_threshold,
            "alert_on_low_bacon_confidence": bacon < 0.5,
            "alert_on_pending_human_augmentation": human_augmentation_runtime
                .get("summary")
                .and_then(|v| v.get("pending_total"))
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

pub fn write_system_control_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("system_control.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let existing = read_json_file(snapshot_path.clone()).unwrap_or_else(|| json!({}));
    let package_registry = read_toml_as_json(workspace_root.join("docs").join("registry.toml"))
        .unwrap_or_else(|| json!({}));
    let usage_limits =
        read_yaml_as_json(workspace_root.join("config").join("llm_usage_limits.yaml"))
            .unwrap_or_else(|| json!({}));
    let manwe_config =
        read_toml_as_json(workspace_root.join("config").join("manwe.providers.toml"))
            .unwrap_or_else(|| json!({}));

    let snapshot = json!({
        "schema_version": "arda.system.control.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "system_control_projection",
        "active_ruleset": existing.get("active_ruleset").cloned().unwrap_or_else(|| json!("arda_totality")),
        "governance": existing.get("governance").cloned().unwrap_or_else(|| json!({})),
        "providers": existing.get("providers").cloned().unwrap_or_else(|| json!({})),
        "storage": existing.get("storage").cloned().unwrap_or_else(|| json!({})),
        "package_observation": existing.get("package_observation").cloned().unwrap_or_else(|| json!({})),
        "connected_sources": {
            "package_registry": package_registry,
            "llm_usage_limits": usage_limits,
            "manwe_provider_config": manwe_config
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub fn write_runtime_settings_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("runtime_settings.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let system_control = read_json_file(core_root.join("state").join("system_control.json"))
        .unwrap_or_else(|| json!({}));
    let package_health = read_json_file(core_root.join("state").join("package_health.json"))
        .unwrap_or_else(|| json!({}));
    let package_enablement =
        read_json_file(core_root.join("state").join("package_enablement.json"))
            .unwrap_or_else(|| json!({}));
    let env_example = summarize_env_file(&workspace_root.join("config/.env.example"));
    let runtime_env_example =
        summarize_env_file(&workspace_root.join("config/runtime.env.example"));
    let runtime_generated =
        summarize_env_file(&workspace_root.join("config/runtime.generated.env"));

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "runtime_settings_projection",
        "backing_store": rel_path(core_root.join("state/system_control.json"), &workspace_root),
        "governance": system_control.get("governance").cloned().unwrap_or_else(|| json!({})),
        "providers": system_control.get("providers").cloned().unwrap_or_else(|| json!({})),
        "storage": system_control.get("storage").cloned().unwrap_or_else(|| json!({})),
        "package_observation": {
            "control": system_control.get("package_observation").cloned().unwrap_or_else(|| json!({})),
            "health_summary": package_health.get("summary").cloned().unwrap_or_else(|| json!({})),
            "enablement_summary": package_enablement.get("summary").cloned().unwrap_or_else(|| json!({}))
        },
        "env_templates": {
            "shared": env_example,
            "runtime": runtime_env_example,
            "generated_runtime": runtime_generated
        },
        "arda_hints": {
            "primary_panel": "runtime_settings",
            "boardroom_section": "configuration_and_controls",
            "alert_on_missing_runtime_template": runtime_env_example.get("keys_total").and_then(Value::as_u64).unwrap_or(0) == 0
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub fn write_control_plane_lockdown_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("control_plane_lockdown.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let autonomy_runtime = read_json_file(core_root.join("state").join("autonomy_runtime.json"))
        .unwrap_or_else(|| json!({}));
    let destructive_quorum =
        read_json_file(core_root.join("state").join("destructive_quorum.json"))
            .unwrap_or_else(|| json!({}));
    let permission_profiles =
        read_json_file(core_root.join("state").join("permission_profiles.json"))
            .unwrap_or_else(|| json!({}));
    let control_plane_policy = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("control_plane")
            .join("policy_snapshot.json"),
    )
    .unwrap_or_else(|| json!({}));
    let runtime_generated_path = workspace_root.join("config").join("runtime.generated.env");
    let runtime_generated = summarize_env_file(&runtime_generated_path);
    let required_sockets = read_env_assignment(
        &runtime_generated_path,
        "ARDA_AUTONOMY_REQUIRED_LIVE_SOCKETS",
    )
    .map(|value| {
        value
            .split(':')
            .filter(|entry| !entry.trim().is_empty())
            .map(|entry| entry.trim().to_string())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    let socket_status = required_sockets
        .iter()
        .map(|socket| {
            let exists = Path::new(socket).exists();
            json!({
                "path": socket,
                "exists": exists
            })
        })
        .collect::<Vec<_>>();
    let active_profile = permission_profiles
        .get("active_profile")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let permissions_active = permission_profiles
        .get("profiles")
        .and_then(|profiles| profiles.get(active_profile))
        .is_some();
    let autonomy_mode = autonomy_runtime
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let destructive_quorum_enabled = destructive_quorum
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let control_plane_policy_present = control_plane_policy
        .get("authority")
        .and_then(Value::as_str)
        == Some("control_plane_policy_export");
    let runtime_generated_present = runtime_generated["exists"].as_bool().unwrap_or(false);
    let autonomy_runtime_present = autonomy_runtime.is_object()
        && !autonomy_runtime
            .as_object()
            .is_some_and(|map| map.is_empty());
    let permission_profile_active = !active_profile.is_empty() && permissions_active;
    let runtime_socket_contract_present = !required_sockets.is_empty();
    let required_sockets_live = !required_sockets.is_empty()
        && socket_status.iter().all(|entry| {
            entry
                .get("exists")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });
    let lockdown_ready = runtime_generated_present
        && control_plane_policy_present
        && destructive_quorum_enabled
        && permission_profile_active
        && autonomy_runtime_present
        && runtime_socket_contract_present
        && required_sockets_live;

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "control_plane_lockdown_projection",
        "contracts": {
            "autonomy_runtime": autonomy_runtime,
            "destructive_quorum": destructive_quorum,
            "permission_profiles": {
                "active_profile": active_profile,
                "profile_present": permissions_active,
                "raw": permission_profiles
            },
            "control_plane_policy": control_plane_policy,
            "runtime_generated_env": runtime_generated
        },
        "runtime_socket_contract": {
            "required_sockets": socket_status,
            "required_sockets_total": required_sockets.len(),
            "required_sockets_live": required_sockets_live
        },
        "status": {
            "autonomy_mode": autonomy_mode,
            "runtime_generated_present": runtime_generated_present,
            "control_plane_policy_present": control_plane_policy_present,
            "destructive_quorum_enabled": destructive_quorum_enabled,
            "permission_profile_active": permission_profile_active,
            "autonomy_runtime_present": autonomy_runtime_present,
            "runtime_socket_contract_present": runtime_socket_contract_present,
            "required_sockets_live": required_sockets_live,
            "lockdown_ready": lockdown_ready
        },
        "arda_hints": {
            "primary_panel": "control_plane_lockdown",
            "boardroom_section": "configuration_and_controls",
            "alert_on_lockdown_gap": !lockdown_ready,
            "alert_on_degraded_autonomy": autonomy_mode.eq_ignore_ascii_case("degraded")
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
