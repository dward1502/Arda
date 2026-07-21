#![cfg(feature = "full-cli")]
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::support::summarize_json_file;
use super::{EdgeTargetsFile, FleetConfigFile};

pub(super) fn rel_path(path: PathBuf, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(&path)
        .display()
        .to_string()
}

pub(super) fn read_toml_as_json(path: PathBuf) -> Option<Value> {
    let mut content = fs::read_to_string(path).ok()?;
    if let Some(after) = content.split("```toml").nth(1) {
        if let Some(block) = after.split("```").next() {
            content = block.to_string();
        }
    }
    let value = content
        .parse::<toml::Value>()
        .or_else(|_| content.replace("\\n", "\n").parse::<toml::Value>())
        .ok()?;
    serde_json::to_value(value).ok()
}

pub(super) fn read_yaml_as_json(path: PathBuf) -> Option<Value> {
    let content = fs::read_to_string(&path).ok()?;
    let preview = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(12)
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    Some(json!({
        "path": path.display().to_string(),
        "line_count": content.lines().count(),
        "preview": preview
    }))
}

pub(super) fn summarize_markdown_file(path: &Path) -> Value {
    let content = fs::read_to_string(path).unwrap_or_default();
    let title = content
        .lines()
        .find(|line| line.trim_start().starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default()
        });
    let body_preview = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    json!({
        "path": path.display().to_string(),
        "title": title,
        "body_preview": body_preview
    })
}

pub(super) fn collect_markdown_file_summaries(root: &Path, limit: usize) -> Vec<Value> {
    let mut files = collect_file_entries(root, "md");
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.reverse();
    files
        .into_iter()
        .take(limit)
        .map(|(_, path)| summarize_markdown_file(&path))
        .collect()
}

pub(super) fn collect_markdown_file_summaries_recursive(root: &Path, limit: usize) -> Vec<Value> {
    let mut files = collect_file_entries_recursive(root, "md");
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.reverse();
    files
        .into_iter()
        .take(limit)
        .map(|(_, path)| summarize_markdown_file(&path))
        .collect()
}

pub(super) fn collect_json_file_summaries_recursive(root: &Path, limit: usize) -> Vec<Value> {
    let mut files = collect_file_entries_recursive(root, "json");
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.reverse();
    files
        .into_iter()
        .take(limit)
        .map(|(_, path)| summarize_json_file(&path))
        .collect()
}

pub(super) fn collect_file_paths(root: &Path, extension: &str) -> Vec<String> {
    collect_file_entries(root, extension)
        .into_iter()
        .map(|(_, path)| path.display().to_string())
        .collect()
}

fn collect_file_entries(root: &Path, extension: &str) -> Vec<(String, PathBuf)> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        files.push((entry.file_name().to_string_lossy().to_string(), path));
    }
    files
}

pub(super) fn collect_file_entries_recursive(
    root: &Path,
    extension: &str,
) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return files,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_file_entries_recursive(&path, extension));
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        files.push((path.display().to_string(), path));
    }
    files
}

pub(super) fn directory_size_bytes(root: &Path) -> u64 {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(_) => return 0,
    };
    if metadata.is_file() {
        return metadata.len();
    }
    if !metadata.is_dir() {
        return 0;
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    let mut total: u64 = 0;
    for entry in entries.flatten() {
        total = total.saturating_add(directory_size_bytes(&entry.path()));
    }
    total
}

pub(super) fn count_files_with_extension(root: &Path, extension: &str) -> usize {
    collect_file_entries(root, extension).len()
}

pub(super) fn read_json_file(path: PathBuf) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub(super) fn read_edge_targets(path: &Path) -> Option<Vec<Value>> {
    let content = fs::read_to_string(path).ok()?;
    let parsed = toml::from_str::<EdgeTargetsFile>(&content).ok()?;
    Some(
        parsed
            .node
            .unwrap_or_default()
            .into_iter()
            .map(|node| {
                json!({
                    "id": node.id,
                    "role": node.role,
                    "hostname": node.hostname,
                    "tailscale_ip": node.tailscale_ip,
                    "ssh_user": node.ssh_user,
                    "athena_enabled": node.athena_enabled.unwrap_or(false),
                    "hermes_enabled": node.hermes_enabled.unwrap_or(false),
                    "warden_enabled": node.warden_enabled.unwrap_or(false),
                    "manwe_enabled": node.manwe_enabled.unwrap_or(false),
                    "oracle_enabled": node.oracle_enabled.unwrap_or(false),
                    "plutus_enabled": node.plutus_enabled.unwrap_or(false),
                    "node_class": node.node_class,
                    "enrollment_status": node.enrollment_status,
                    "llm_runtime": node.llm_runtime,
                    "notes": node.notes
                })
            })
            .collect(),
    )
}

pub(super) fn read_fleet_config_nodes(path: &Path) -> Option<Vec<Value>> {
    let content = fs::read_to_string(path).ok()?;
    let parsed = toml::from_str::<FleetConfigFile>(&content).ok()?;
    Some(
        parsed
            .nodes
            .unwrap_or_default()
            .into_iter()
            .map(|node| {
                json!({
                    "id": node.id,
                    "role": node.role,
                    "hostname": node.hostname,
                    "display_name": node.display_name,
                    "tailscale_ip": node.tailscale_ip,
                    "tailscale_name": node.tailscale_name,
                    "ssh_user": node.ssh_user,
                    "node_class": node.node_class,
                    "enrollment_status": node.enrollment_status,
                    "llm_runtime": node.llm_runtime,
                    "manwe_provider_id": node.manwe_provider_id,
                    "base_url": node.base_url,
                    "health_url": node.health_url,
                    "models_url": node.models_url,
                    "expected_models": node.expected_models,
                    "startup_priority": node.startup_priority,
                    "restart_scope": node.restart_scope,
                    "restart_cmd": node.restart_cmd,
                    "notes": node.notes
                })
            })
            .collect(),
    )
}

pub(super) fn read_fleet_config_meta(path: &Path) -> Value {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return json!({}),
    };
    let parsed = match toml::from_str::<FleetConfigFile>(&content) {
        Ok(parsed) => parsed,
        Err(_) => return json!({}),
    };
    json!({
        "fleet": {
            "enabled": parsed.fleet.as_ref().and_then(|fleet| fleet.enabled).unwrap_or(true),
            "status_view_mode": parsed
                .fleet
                .as_ref()
                .and_then(|fleet| fleet.status_view_mode.clone())
                .unwrap_or_else(|| "active_only".to_string()),
            "stale_offline_threshold_days": parsed
                .fleet
                .as_ref()
                .and_then(|fleet| fleet.stale_offline_threshold_days)
                .unwrap_or(14),
            "include_recent_offline_in_status": parsed
                .fleet
                .as_ref()
                .and_then(|fleet| fleet.include_recent_offline_in_status)
                .unwrap_or(false)
        },
        "exports": {
            "prometheus_dir": parsed
                .exports
                .as_ref()
                .and_then(|exports| exports.prometheus_dir.clone()),
            "ceo_layer_prometheus_dir": parsed
                .exports
                .as_ref()
                .and_then(|exports| exports.ceo_layer_prometheus_dir.clone())
        }
    })
}

pub(super) fn merge_fleet_nodes(configured: &[Value], observed: &[Value]) -> Vec<Value> {
    let mut merged = Vec::new();
    let mut used = vec![false; observed.len()];

    for config in configured {
        let config_hostname = config
            .get("hostname")
            .and_then(Value::as_str)
            .map(|value| value.to_ascii_lowercase());
        let config_ip = config.get("tailscale_ip").and_then(Value::as_str);
        let matched_index = observed
            .iter()
            .enumerate()
            .filter_map(|(index, observed_node)| {
                let observed_hostname = observed_node
                    .get("hostname")
                    .and_then(Value::as_str)
                    .map(|value| value.to_ascii_lowercase());
                let observed_dns = observed_node
                    .get("dns_name")
                    .and_then(Value::as_str)
                    .map(|value| value.to_ascii_lowercase());
                let observed_ips = observed_node
                    .get("tailscale_ips")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();

                let exact_hostname_match = config_hostname
                    .as_ref()
                    .is_some_and(|hostname| observed_hostname.as_ref() == Some(hostname));
                let dns_prefix_match = config_hostname.as_ref().is_some_and(|hostname| {
                    observed_dns
                        .as_ref()
                        .is_some_and(|dns| dns.starts_with(hostname))
                });
                let ip_match = config_ip
                    .is_some_and(|ip| observed_ips.iter().any(|value| value.as_str() == Some(ip)));

                let score = if ip_match {
                    100
                } else if exact_hostname_match {
                    70
                } else if dns_prefix_match {
                    40
                } else {
                    0
                };
                if score == 0 {
                    return None;
                }
                let online_bonus = if observed_node
                    .get("online")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    10
                } else {
                    0
                };
                Some((index, score + online_bonus))
            })
            .max_by_key(|(_, score)| *score)
            .map(|(index, _)| index);

        let observed_value = matched_index.and_then(|index| {
            used[index] = true;
            observed.get(index).cloned()
        });

        merged.push(json!({
            "configured": config,
            "observed": observed_value,
            "matched": observed_value.is_some(),
            "online": observed_value
                .as_ref()
                .and_then(|value| value.get("online"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "display_name": config
                .get("display_name")
                .and_then(Value::as_str)
                .or_else(|| config.get("hostname").and_then(Value::as_str))
                .unwrap_or("unknown")
        }));
    }

    for (index, observed_node) in observed.iter().enumerate() {
        if used[index] {
            continue;
        }
        merged.push(json!({
            "configured": Value::Null,
            "observed": observed_node,
            "matched": false,
            "online": observed_node.get("online").and_then(Value::as_bool).unwrap_or(false),
            "display_name": observed_node
                .get("hostname")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        }));
    }

    merged
}
