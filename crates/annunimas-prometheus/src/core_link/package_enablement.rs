use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::support::{
    collect_athena_repo_source_map, package_activation_status, package_integration_lane,
    package_next_action, package_provider_id, package_required_runtime_env_keys,
    package_required_shared_env_keys, package_runtime_surface_key, read_latest_policy_readiness,
    summarize_env_file,
};
use super::{read_json_file, read_toml_as_json, CORE_STATE_SCHEMA_VERSION};

pub fn write_package_enablement_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("package_enablement.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let registry = read_toml_as_json(workspace_root.join("docs").join("registry.toml"))
        .unwrap_or_else(|| json!({}));
    let package_health = read_json_file(core_root.join("state").join("package_health.json"))
        .unwrap_or_else(|| json!({}));
    let package_runtime = read_json_file(
        core_root
            .join("state")
            .join("package_runtime_activation.json"),
    )
    .unwrap_or_else(|| json!({}));
    let charon_config =
        read_toml_as_json(workspace_root.join("config").join("charon.providers.toml"))
            .unwrap_or_else(|| json!({}));
    let shared_env = summarize_env_file(&workspace_root.join("config/.env.example"));
    let runtime_env = summarize_env_file(&workspace_root.join("config/runtime.env.example"));
    let repo_sources =
        collect_athena_repo_source_map(&workspace_root.join("data").join("athena").join("books"));
    let readiness_by_source = read_latest_policy_readiness(
        &workspace_root
            .join("data")
            .join("athena")
            .join("policy_readiness.jsonl"),
    );

    let tools_meta = registry
        .get("tools")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let observed_tools = package_health
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let provider_ids = charon_config
        .get("provider")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|provider| {
            provider
                .get("id")
                .and_then(Value::as_str)
                .map(|value| value.to_string())
        })
        .collect::<Vec<_>>();
    let shared_keys = shared_env
        .get("keys")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();
    let runtime_keys = runtime_env
        .get("keys")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();
    let runtime_surfaces = package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut tools = tools_meta.into_iter().collect::<Vec<_>>();
    tools.sort_by(|a, b| a.0.cmp(&b.0));

    let tool_rows = tools
        .into_iter()
        .map(|(tool, meta)| {
            let repo = meta
                .get("repo")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let repo_url = if repo.is_empty() {
                None
            } else {
                Some(format!("https://github.com/{repo}"))
            };
            let observation = observed_tools
                .iter()
                .find(|entry| entry.get("tool").and_then(Value::as_str) == Some(tool.as_str()));
            let source_id = repo_url
                .as_ref()
                .and_then(|url| repo_sources.get(url))
                .cloned();
            let policy_entry = source_id
                .as_ref()
                .and_then(|source| readiness_by_source.get(source))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let policy_readiness = policy_entry
                .get("policy_readiness")
                .and_then(Value::as_str)
                .unwrap_or("untracked");
            let required_shared_keys = package_required_shared_env_keys(&tool);
            let required_runtime_keys = package_required_runtime_env_keys(&tool);
            let provider_id = package_provider_id(&tool);
            let provider_configured = provider_id
                .map(|id| provider_ids.iter().any(|value| value == id))
                .unwrap_or(false);
            let runtime_surface = runtime_surfaces
                .get(package_runtime_surface_key(&tool))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let shared_env_ready = required_shared_keys
                .iter()
                .all(|key| shared_keys.iter().any(|present| present == key));
            let runtime_env_ready = required_runtime_keys
                .iter()
                .all(|key| runtime_keys.iter().any(|present| present == key));
            let executable_visible = observation
                .and_then(|value| value.get("binary_path"))
                .and_then(Value::as_str)
                .map(|value| !value.is_empty())
                .unwrap_or(false)
                || observation
                    .and_then(|value| value.get("version_hint"))
                    .and_then(Value::as_str)
                    .map(|value| !value.is_empty())
                    .unwrap_or(false);
            let runtime_surface_ready = if provider_id.is_some() {
                provider_configured
            } else {
                executable_visible
            };
            let integration_state = if policy_readiness == "policy_ready"
                && runtime_surface_ready
                && shared_env_ready
                && runtime_env_ready
            {
                "ready_for_activation"
            } else if policy_readiness == "policy_ready" && runtime_surface_ready {
                "configuration_ready"
            } else if policy_readiness == "policy_ready" {
                "evidence_ready"
            } else if observation
                .and_then(|value| value.get("observation_status"))
                .and_then(Value::as_str)
                == Some("observed")
            {
                "observed_only"
            } else {
                "planned_only"
            };
            let activation_status = package_activation_status(
                &tool,
                integration_state,
                provider_configured,
                &runtime_surface,
            );

            json!({
                "tool": tool,
                "repo": repo,
                "repo_url": repo_url,
                "category": meta.get("category").cloned().unwrap_or(json!("unknown")),
                "registry_status": meta.get("status").cloned().unwrap_or(json!("unknown")),
                "tool_type": meta.get("type").cloned().unwrap_or(json!("unknown")),
                "integration_lane": package_integration_lane(&tool, &meta),
                "source_id": source_id,
                "policy_readiness": policy_readiness,
                "policy_confidence": policy_entry
                    .get("gate")
                    .and_then(|value| value.get("observed"))
                    .and_then(|value| value.get("confidence"))
                    .cloned()
                    .unwrap_or(json!(null)),
                "observation_status": observation
                    .and_then(|value| value.get("observation_status"))
                    .cloned()
                    .unwrap_or(json!("unobserved")),
                "binary_path": observation
                    .and_then(|value| value.get("binary_path"))
                    .cloned()
                    .unwrap_or(json!(null)),
                "version_hint": observation
                    .and_then(|value| value.get("version_hint"))
                    .cloned()
                    .unwrap_or(json!(null)),
                "provider_id": provider_id,
                "provider_configured": provider_configured,
                "shared_env_contract": required_shared_keys,
                "shared_env_ready": shared_env_ready,
                "runtime_env_contract": required_runtime_keys,
                "runtime_env_ready": runtime_env_ready,
                "executable_visible": executable_visible,
                "integration_state": integration_state,
                "activation_status": activation_status,
                "next_action": package_next_action(integration_state, activation_status, &tool),
            })
        })
        .collect::<Vec<_>>();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "package_enablement_projection",
        "summary": {
            "tools_total": tool_rows.len(),
            "policy_ready_total": tool_rows.iter().filter(|row| row.get("policy_readiness").and_then(Value::as_str) == Some("policy_ready")).count(),
            "ready_for_activation_total": tool_rows.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("ready_for_activation")).count(),
            "configuration_ready_total": tool_rows.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("configuration_ready")).count(),
            "evidence_ready_total": tool_rows.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("evidence_ready")).count(),
            "observed_only_total": tool_rows.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("observed_only")).count(),
        },
        "tools": tool_rows,
        "arda_hints": {
            "primary_panel": "package_enablement",
            "boardroom_section": "operations_and_packages",
            "alert_on_activation_gap": tool_rows.iter().any(|row| {
                row.get("policy_readiness").and_then(Value::as_str) == Some("policy_ready")
                    && row.get("integration_state").and_then(Value::as_str) != Some("ready_for_activation")
            })
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub fn write_package_health_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("package_health.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let snapshot = read_json_file(
        workspace_root
            .join("data")
            .join("prometheus")
            .join("package_health_last.json"),
    )
    .unwrap_or_else(|| {
        json!({
            "schema_version": "annunimas.package.health.v1",
            "authority": "package_observation_export",
            "summary": {"tools_total": 0, "critical_tools_total": 0, "critical_attention_required": []},
            "tools": []
        })
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
