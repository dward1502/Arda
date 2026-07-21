#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use super::*;

pub(crate) fn export_fleet_bootstrap_state_impl() -> Result<Value> {
    let root = workspace_root();
    let fleet_path = root.join("config/fleet.toml");
    let bootstrap_out = root.join("core/state/fleet_bootstrap.json");
    let edge_out = root.join("core/state/edge_endpoint_verification.json");
    let recovery_out = root.join("core/state/fleet_bootstrap_recovery.json");
    let recovery_events = root.join("data/prometheus/fleet_bootstrap_recovery.jsonl");
    let fleet = read_toml_or(&fleet_path, toml::Value::Table(Default::default()));
    let tailnet = tailscale_status();
    let nodes = fleet
        .get("nodes")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut node_rows = nodes
        .iter()
        .filter_map(toml::Value::as_table)
        .map(|node| node_status(node, tailnet.as_ref(), &root))
        .collect::<Vec<_>>();
    node_rows.sort_by_key(|row| {
        row.get("startup_priority")
            .and_then(Value::as_i64)
            .unwrap_or(1000)
    });
    let live_nodes = node_rows
        .iter()
        .filter(|row| {
            row.get("has_live_endpoint")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let restart_attempted_total = node_rows
        .iter()
        .filter(|row| {
            row.get("recovery")
                .and_then(|recovery| recovery.get("attempted"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let recovered_total = node_rows
        .iter()
        .filter(|row| {
            row.get("recovery")
                .and_then(|recovery| recovery.get("recovered"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let restart_failed_total = node_rows
        .iter()
        .filter(|row| {
            row.get("recovery")
                .and_then(|recovery| recovery.get("attempted"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !row
                    .get("recovery")
                    .and_then(|recovery| recovery.get("recovered"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .count();
    let generated_at = now_utc();
    let unexpected_offline_total = node_rows
        .iter()
        .filter(|row| {
            !row.get("intentional_offline")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !row
                    .get("has_live_endpoint")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .count();
    let bootstrap = json!({
        "schema_version": "arda.fleet-bootstrap.v1",
        "generated_at_utc": generated_at,
        "authority": "fleet_bootstrap_state",
        "fleet_config_path": rel(&fleet_path, &root),
        "summary": {
            "nodes_total": node_rows.len(),
            "live_nodes_total": live_nodes.len(),
            "unexpected_offline_total": unexpected_offline_total,
            "restart_attempted_total": restart_attempted_total,
            "recovered_total": recovered_total,
            "restart_failed_total": restart_failed_total,
        },
        "core_belief": {
            "joulework": {"mode":"equation_authority_external"},
            "love_equation": {"mode":"equation_authority_external"},
            "philosophical_group": {"mode":"idea_only","variants":["triad","lite"]},
        },
        "startup_order": node_rows.iter().filter_map(|row| row.get("target_id").cloned()).collect::<Vec<_>>(),
        "live_targets": live_nodes.iter().map(|row| json!({
            "target_id": row.get("target_id").cloned().unwrap_or(Value::Null),
            "base_url": row.get("configured_base_url").cloned().unwrap_or(Value::Null),
            "models": row.get("observed_models").cloned().unwrap_or_else(|| json!([])),
        })).collect::<Vec<_>>(),
        "targets": node_rows,
    });
    let edge_payload = json!({
        "schema_version": "arda.edge-endpoint-verification.v3",
        "generated_at_utc": generated_at,
        "authority": "fleet_bootstrap_state",
        "probe_status": "ok",
        "summary": {
            "targets_total": bootstrap.get("targets").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "targets_with_live_endpoints_total": live_nodes.len(),
            "intentional_offline_total": bootstrap.get("targets").and_then(Value::as_array).into_iter().flatten().filter(|row| row.get("intentional_offline").and_then(Value::as_bool).unwrap_or(false)).count(),
            "targets_unexpected_offline_total": unexpected_offline_total,
            "restart_attempted_total": restart_attempted_total,
            "recovered_total": recovered_total,
            "restart_failed_total": restart_failed_total,
        },
        "targets": bootstrap.get("targets").cloned().unwrap_or_else(|| json!([])),
    });
    let recovery_payload = json!({
        "schema_version": "arda.fleet-bootstrap-recovery.v1",
        "generated_at_utc": generated_at,
        "authority": "fleet_bootstrap_state",
        "recovery_enabled": env_flag("ARDA_FLEET_BOOTSTRAP_RECOVER", false),
        "recover_wait_seconds": env_int("ARDA_FLEET_BOOTSTRAP_RECOVER_WAIT_SECONDS", 8),
        "recover_retries": env_int("ARDA_FLEET_BOOTSTRAP_RECOVER_RETRIES", 1),
        "summary": {
            "restart_attempted_total": restart_attempted_total,
            "recovered_total": recovered_total,
            "restart_failed_total": restart_failed_total,
        },
        "targets": bootstrap.get("targets").and_then(Value::as_array).into_iter().flatten().filter(|row| row.get("recovery").is_some()).map(|row| {
            json!({
                "target_id": row.get("target_id").cloned().unwrap_or(Value::Null),
                "manwe_provider_id": row.get("manwe_provider_id").cloned().unwrap_or(Value::Null),
                "status": row.get("status").cloned().unwrap_or(Value::Null),
                "recovery": row.get("recovery").cloned().unwrap_or(Value::Null),
            })
        }).collect::<Vec<_>>(),
    });
    write_pretty_json(&bootstrap_out, &bootstrap)?;
    write_pretty_json(&edge_out, &edge_payload)?;
    write_pretty_json(&recovery_out, &recovery_payload)?;
    append_recovery_events(
        &recovery_events,
        bootstrap
            .get("targets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    )?;
    Ok(json!({ "out": rel(&bootstrap_out, &root) }))
}

pub(crate) fn export_edge_endpoint_verification_impl() -> Result<Value> {
    export_fleet_bootstrap_state_impl()
}

fn env_flag(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_int(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(default)
}

fn tailscale_status() -> Option<Value> {
    let output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn normalize_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_end_matches('.').to_ascii_lowercase())
}

fn match_peer(status: Option<&Value>, node: &toml::value::Table) -> Option<Value> {
    let peers = status
        .and_then(|status| status.get("Peer"))
        .and_then(Value::as_object)?;
    let tailscale_name = normalize_name(node.get("tailscale_name").and_then(toml::Value::as_str));
    let tailscale_ip = normalize_name(node.get("tailscale_ip").and_then(toml::Value::as_str));
    let hostname = normalize_name(node.get("hostname").and_then(toml::Value::as_str));
    let mut best = None;
    let mut best_score = -1i64;
    for peer in peers.values() {
        let peer_dns = normalize_name(peer.get("DNSName").and_then(Value::as_str));
        let peer_host = normalize_name(peer.get("HostName").and_then(Value::as_str));
        let peer_ips = peer
            .get("TailscaleIPs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|ip| normalize_name(Some(ip)))
            .collect::<Vec<_>>();
        let mut score = 0i64;
        if tailscale_ip
            .as_ref()
            .map(|ip| peer_ips.contains(ip))
            .unwrap_or(false)
        {
            score = 100;
        }
        if tailscale_name.is_some() && tailscale_name == peer_dns {
            score = score.max(90);
        }
        if tailscale_name.is_some() && tailscale_name == peer_host {
            score = score.max(80);
        }
        if hostname.is_some() && hostname == peer_host {
            score = score.max(50);
        }
        if hostname.is_some() && hostname == peer_dns {
            score = score.max(40);
        }
        if score > best_score {
            best_score = score;
            best = Some(peer.clone());
        }
    }
    if best_score > 0 {
        best
    } else {
        None
    }
}

fn ssh_target_for_node(
    node: &toml::value::Table,
    status: Option<&Value>,
) -> (Option<String>, Option<Value>) {
    let peer = match_peer(status, node);
    if let Some(dns) = peer
        .as_ref()
        .and_then(|peer| normalize_name(peer.get("DNSName").and_then(Value::as_str)))
    {
        return (Some(dns), peer);
    }
    if let Some(name) = normalize_name(node.get("tailscale_name").and_then(toml::Value::as_str)) {
        return (Some(name), peer);
    }
    if let Some(ip) = node.get("tailscale_ip").and_then(toml::Value::as_str) {
        return (Some(ip.trim().to_string()), peer);
    }
    if let Some(hostname) = normalize_name(node.get("hostname").and_then(toml::Value::as_str)) {
        return (Some(hostname), peer);
    }
    (None, peer)
}

fn fetch_json(url: &str) -> (bool, Option<u16>, Option<String>, Value) {
    let output = Command::new("curl")
        .args([
            "-sS",
            "--connect-timeout",
            "2",
            "--max-time",
            "5",
            "-H",
            "Accept: application/json",
            "-w",
            "\n%{http_code}",
            url,
        ])
        .output();
    let Ok(output) = output else {
        return (false, None, Some("curl_failed".to_string()), json!({}));
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines().collect::<Vec<_>>();
    let code = lines.pop().and_then(|line| line.parse::<u16>().ok());
    let body = lines.join("\n");
    if !output.status.success() {
        return (
            false,
            code,
            Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            json!({}),
        );
    }
    let payload = serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({}));
    (code == Some(200), code, None, payload)
}

fn probe_node(node: &toml::value::Table) -> Value {
    let expected_models = node
        .get("expected_models")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let (health_ok, health_code, health_error, _) = fetch_json(
        node.get("health_url")
            .and_then(toml::Value::as_str)
            .unwrap_or_default(),
    );
    let (models_ok, models_code, models_error, models_payload) = fetch_json(
        node.get("models_url")
            .and_then(toml::Value::as_str)
            .unwrap_or_default(),
    );
    let observed_models = models_payload
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let expected_ready = expected_models.is_empty()
        || expected_models
            .iter()
            .any(|model| observed_models.iter().any(|observed| observed == model));
    let live = models_ok && expected_ready;
    let status = if live {
        "online"
    } else if models_ok {
        "degraded"
    } else {
        "offline"
    };
    json!({
        "status": status,
        "has_live_endpoint": live,
        "expected_models": expected_models,
        "observed_models": observed_models,
        "ports": [{
            "port": 1234,
            "health_http_code": health_code.map(|code| code.to_string()).unwrap_or_else(|| "000".to_string()),
            "models_http_code": models_code.map(|code| code.to_string()).unwrap_or_else(|| "000".to_string()),
            "health_ok": health_ok,
            "live": live,
            "health_error": health_error,
            "models_error": models_error,
            "models": observed_models,
        }],
    })
}

fn try_restart(node: &toml::value::Table, status: Option<&Value>, root: &Path) -> Option<Value> {
    if !env_flag("ARDA_FLEET_BOOTSTRAP_RECOVER", false) {
        return None;
    }
    let restart_cmd = node
        .get("restart_cmd")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|cmd| !cmd.is_empty())?;
    let scope = node
        .get("restart_scope")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let result = if scope == "local" {
        Command::new("bash")
            .args(["-lc", restart_cmd])
            .current_dir(root)
            .output()
            .ok()
    } else if scope == "ssh" {
        let ssh_user = node
            .get("ssh_user")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        let (ssh_target, peer) = ssh_target_for_node(node, status);
        let ssh_target = ssh_target?;
        if ssh_user.is_empty() {
            return Some(json!({"attempted": false, "ok": false, "error": "missing_ssh_target"}));
        }
        if peer
            .as_ref()
            .and_then(|peer| peer.get("Online"))
            .and_then(Value::as_bool)
            == Some(false)
        {
            return Some(
                json!({"attempted": false, "ok": false, "scope": scope, "command": restart_cmd, "ssh_target": ssh_target, "peer_online": false, "error": "tailscale_peer_offline"}),
            );
        }
        Command::new("tailscale")
            .args([
                "ssh",
                "--",
                &format!("{ssh_user}@{ssh_target}"),
                restart_cmd,
            ])
            .current_dir(root)
            .output()
            .ok()
    } else {
        return Some(
            json!({"attempted": false, "ok": false, "error": format!("unsupported_restart_scope:{}", if scope.is_empty() { "unset" } else { &scope })}),
        );
    }?;
    Some(json!({
        "attempted": true,
        "ok": result.status.success(),
        "scope": scope,
        "command": restart_cmd,
        "exit_code": result.status.code().unwrap_or(1),
        "stdout": String::from_utf8_lossy(&result.stdout).trim().chars().take(400).collect::<String>(),
        "stderr": String::from_utf8_lossy(&result.stderr).trim().chars().take(400).collect::<String>(),
    }))
}

fn node_status(node: &toml::value::Table, tailnet: Option<&Value>, root: &Path) -> Value {
    let intentional_offline = node
        .get("intentional_offline")
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);
    let peer = match_peer(tailnet, node);
    let (ssh_target, peer_for_transport) = ssh_target_for_node(node, tailnet);
    if intentional_offline {
        let expected_models = node
            .get("expected_models")
            .and_then(toml::Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        let offline_probe = json!({
            "status": "offline",
            "has_live_endpoint": false,
            "expected_models": expected_models,
            "observed_models": [],
            "ports": [],
            "probe_skipped": true,
            "probe_skip_reason": "intentional_offline",
        });
        return json!({
            "target_id": node.get("id").and_then(toml::Value::as_str).unwrap_or_default(),
            "display_name": node.get("display_name").and_then(toml::Value::as_str).or_else(|| node.get("hostname").and_then(toml::Value::as_str)).unwrap_or_default(),
            "provider_hint": toml_to_json_local(node.get("llm_runtime")),
            "manwe_provider_id": toml_to_json_local(node.get("manwe_provider_id")),
            "configured_base_url": toml_to_json_local(node.get("base_url")),
            "health_url": toml_to_json_local(node.get("health_url")),
            "models_url": toml_to_json_local(node.get("models_url")),
            "status": offline_probe.get("status").cloned().unwrap_or(Value::String("offline".to_string())),
            "has_live_endpoint": offline_probe.get("has_live_endpoint").cloned().unwrap_or(Value::Bool(false)),
            "expected_models": offline_probe.get("expected_models").cloned().unwrap_or_else(|| json!([])),
            "observed_models": offline_probe.get("observed_models").cloned().unwrap_or_else(|| json!([])),
            "startup_priority": toml_to_json_local(node.get("startup_priority")),
            "intentional_offline": true,
            "transport": {
                "tailscale_peer_found": peer.is_some(),
                "tailscale_peer_online": peer_for_transport.as_ref().and_then(|row| row.get("Online")).cloned().unwrap_or(Value::Null),
                "tailscale_dns_name": peer_for_transport.as_ref().and_then(|row| row.get("DNSName")).cloned().unwrap_or(Value::Null),
                "ssh_target": ssh_target,
            },
            "recovery": Value::Null,
            "initial_probe": {
                "status": offline_probe.get("status").cloned().unwrap_or(Value::Null),
                "has_live_endpoint": offline_probe.get("has_live_endpoint").cloned().unwrap_or(Value::Null),
                "observed_models": offline_probe.get("observed_models").cloned().unwrap_or_else(|| json!([])),
                "probe_skipped": Value::Bool(true),
                "probe_skip_reason": Value::String("intentional_offline".to_string()),
            },
            "ports": offline_probe.get("ports").cloned().unwrap_or_else(|| json!([])),
        });
    }
    let initial_probe = probe_node(node);
    let mut final_probe = initial_probe.clone();
    let mut recovery = json!({"attempted": false, "recovered": false, "attempts": []});
    if !initial_probe
        .get("has_live_endpoint")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !intentional_offline
    {
        let recover_retries = env_int("ARDA_FLEET_BOOTSTRAP_RECOVER_RETRIES", 1).max(1);
        for attempt_index in 0..recover_retries {
            let Some(restart) = try_restart(node, tailnet, root) else {
                break;
            };
            recovery["attempted"] = Value::Bool(true);
            let mut attempt = json!({"attempt": attempt_index + 1, "restart": restart});
            if attempt
                .get("restart")
                .and_then(|row| row.get("ok"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let reprobe = probe_node(node);
                attempt["reprobe"] = json!({
                    "status": reprobe.get("status").cloned().unwrap_or(Value::Null),
                    "has_live_endpoint": reprobe.get("has_live_endpoint").cloned().unwrap_or(Value::Null),
                    "observed_models": reprobe.get("observed_models").cloned().unwrap_or_else(|| json!([])),
                });
                final_probe = reprobe;
                if final_probe
                    .get("has_live_endpoint")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    recovery["recovered"] = Value::Bool(true);
                    recovery["recovered_after_attempt"] = Value::from(attempt_index + 1);
                    break;
                }
            }
            if let Some(attempts) = recovery.get_mut("attempts").and_then(Value::as_array_mut) {
                attempts.push(attempt);
            }
        }
    }
    json!({
        "target_id": node.get("id").and_then(toml::Value::as_str).unwrap_or_default(),
        "display_name": node.get("display_name").and_then(toml::Value::as_str).or_else(|| node.get("hostname").and_then(toml::Value::as_str)).unwrap_or_default(),
        "provider_hint": toml_to_json_local(node.get("llm_runtime")),
        "manwe_provider_id": toml_to_json_local(node.get("manwe_provider_id")),
        "configured_base_url": toml_to_json_local(node.get("base_url")),
        "health_url": toml_to_json_local(node.get("health_url")),
        "models_url": toml_to_json_local(node.get("models_url")),
        "status": final_probe.get("status").cloned().unwrap_or(Value::String("offline".to_string())),
        "has_live_endpoint": final_probe.get("has_live_endpoint").cloned().unwrap_or(Value::Bool(false)),
        "expected_models": final_probe.get("expected_models").cloned().unwrap_or_else(|| json!([])),
        "observed_models": final_probe.get("observed_models").cloned().unwrap_or_else(|| json!([])),
        "startup_priority": toml_to_json_local(node.get("startup_priority")),
        "intentional_offline": intentional_offline,
        "transport": {
            "tailscale_peer_found": peer.is_some(),
            "tailscale_peer_online": peer_for_transport.as_ref().and_then(|row| row.get("Online")).cloned().unwrap_or(Value::Null),
            "tailscale_dns_name": peer_for_transport.as_ref().and_then(|row| row.get("DNSName")).cloned().unwrap_or(Value::Null),
            "ssh_target": ssh_target,
        },
        "recovery": if recovery.get("attempted").and_then(Value::as_bool).unwrap_or(false) { recovery } else { Value::Null },
        "initial_probe": {
            "status": initial_probe.get("status").cloned().unwrap_or(Value::Null),
            "has_live_endpoint": initial_probe.get("has_live_endpoint").cloned().unwrap_or(Value::Null),
            "observed_models": initial_probe.get("observed_models").cloned().unwrap_or_else(|| json!([])),
        },
        "ports": final_probe.get("ports").cloned().unwrap_or_else(|| json!([])),
    })
}

fn append_recovery_events(path: &Path, node_rows: Vec<Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut handle = OpenOptions::new().create(true).append(true).open(path)?;
    for row in node_rows {
        let Some(recovery) = row.get("recovery") else {
            continue;
        };
        if !recovery
            .get("attempted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let payload = json!({
            "ts": now_utc(),
            "target_id": row.get("target_id").cloned().unwrap_or(Value::Null),
            "manwe_provider_id": row.get("manwe_provider_id").cloned().unwrap_or(Value::Null),
            "status": row.get("status").cloned().unwrap_or(Value::Null),
            "recovered": recovery.get("recovered").cloned().unwrap_or(Value::Bool(false)),
            "attempts": recovery.get("attempts").cloned().unwrap_or_else(|| json!([])),
        });
        handle.write_all(serde_json::to_string(&payload)?.as_bytes())?;
        handle.write_all(b"\n")?;
    }
    Ok(())
}

fn toml_to_json_local(value: Option<&toml::Value>) -> Value {
    match value {
        Some(toml::Value::String(value)) => Value::String(value.clone()),
        Some(toml::Value::Integer(value)) => Value::from(*value),
        Some(toml::Value::Float(value)) => json!(value),
        Some(toml::Value::Boolean(value)) => Value::Bool(*value),
        Some(toml::Value::Array(values)) => Value::Array(
            values
                .iter()
                .map(|value| toml_to_json_local(Some(value)))
                .collect(),
        ),
        Some(toml::Value::Table(table)) => Value::Object(
            table
                .iter()
                .map(|(key, value)| (key.clone(), toml_to_json_local(Some(value))))
                .collect(),
        ),
        Some(toml::Value::Datetime(value)) => Value::String(value.to_string()),
        None => Value::Null,
    }
}

#[allow(dead_code)]
fn load_topology_registry_nodes(path: &Path) -> Vec<Value> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    let mut nodes = Vec::new();
    let mut current = serde_json::Map::new();
    let mut current_section: Option<&str> = None;
    for raw_line in raw.lines() {
        let line = raw_line.trim_end();
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        if stripped == "nodes:" {
            current.clear();
            current_section = None;
            continue;
        }
        if stripped.starts_with("- id:") || stripped.starts_with("- node_id:") {
            if !current.is_empty() {
                nodes.push(Value::Object(current.clone()));
            }
            current = serde_json::Map::new();
            let value = stripped
                .split_once(':')
                .map(|(_, rhs)| rhs.trim().trim_matches('"').trim_matches('\''))
                .unwrap_or_default();
            current.insert("id".to_string(), json!(value));
            current_section = None;
            continue;
        }
        if stripped == "capabilities:" {
            current
                .entry("capabilities".to_string())
                .or_insert_with(|| json!({}));
            current_section = Some("capabilities");
            continue;
        }
        if stripped == "labels:" {
            current
                .entry("labels".to_string())
                .or_insert_with(|| json!({}));
            current_section = Some("labels");
            continue;
        }
        if stripped == "endpoints:" {
            current
                .entry("endpoints".to_string())
                .or_insert_with(|| json!({}));
            current_section = Some("endpoints");
            continue;
        }
        if !line.starts_with("  ") && !line.starts_with("- ") {
            current_section = None;
        }
        let Some((key, value)) = stripped.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let parsed = parse_yaml_scalar(value.trim());
        if let Some(section) = current_section {
            let entry = current
                .entry(section.to_string())
                .or_insert_with(|| json!({}));
            if let Some(map) = entry.as_object_mut() {
                map.insert(key.to_string(), parsed);
            }
        } else {
            current.insert(key.to_string(), parsed);
        }
    }
    if !current.is_empty() {
        nodes.push(Value::Object(current));
    }
    nodes
}

#[allow(dead_code)]
fn parse_yaml_scalar(raw: &str) -> Value {
    let value = raw.trim().trim_matches('"').trim_matches('\'');
    match value {
        "true" => json!(true),
        "false" => json!(false),
        _ => {
            if let Ok(parsed) = value.parse::<i64>() {
                json!(parsed)
            } else if let Ok(parsed) = value.parse::<f64>() {
                json!(parsed)
            } else {
                json!(value)
            }
        }
    }
}

pub(crate) fn export_fleet_steward_actions_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/fleet_steward_actions.json");
    let governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );
    let budget = read_json_or(
        &root.join("core/state/runtime_budget_policy.json"),
        json!({}),
    );
    let enablement = read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let edge_model_rollout =
        read_json_or(&root.join("core/state/edge_model_rollout.json"), json!({}));
    let edge_endpoint_verification = read_json_or(
        &root.join("core/state/edge_endpoint_verification.json"),
        json!({}),
    );
    let runtime_admission_recovery = read_json_or(
        &root.join("core/state/runtime_admission_recovery.json"),
        json!({}),
    );
    let runtime_admission_recovery_executor = read_json_or(
        &root.join("core/state/runtime_admission_recovery_executor.json"),
        json!({}),
    );
    let runtime_recovery_route_governor = read_json_or(
        &root.join("core/state/runtime_recovery_route_governor.json"),
        json!({}),
    );

    let mut actions = Vec::new();
    let fleet_nodes = governor
        .get("capability_lanes")
        .and_then(|v| v.get("fleet_uptime_and_downtime"))
        .and_then(|v| v.get("nodes"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let budget_summary = budget
        .get("capability_lanes")
        .and_then(|v| v.get("user_and_provider_budget_pressure"))
        .and_then(|v| v.get("summary"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let package_tools = enablement
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rollout_targets = edge_model_rollout
        .get("targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let endpoint_targets = edge_endpoint_verification
        .get("targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let runtime_recovery_actions = runtime_admission_recovery
        .get("recovery_actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let executor_runs = runtime_admission_recovery_executor
        .get("runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut executor_by_label = HashMap::new();
    for run in &executor_runs {
        if let Some(label) = run.get("label").and_then(Value::as_str) {
            executor_by_label.insert(label.to_string(), run.clone());
        }
    }
    let route_shift_already_executed = executor_runs.iter().any(|run| {
        run.get("kind").and_then(Value::as_str) == Some("route_shift")
            && run.get("status").and_then(Value::as_str) == Some("executed")
    });
    let rollout_by_target: HashMap<String, Value> = rollout_targets
        .iter()
        .filter_map(|target| {
            target
                .get("target_id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), target.clone()))
        })
        .collect();
    let endpoint_by_target: HashMap<String, Value> = endpoint_targets
        .iter()
        .filter_map(|target| {
            target
                .get("target_id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), target.clone()))
        })
        .collect();

    for node in fleet_nodes {
        if !node.is_object() || node.get("online").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let target_id = node
            .get("target_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        actions.push(json!({
            "action_id": format!("restore_{target_id}"),
            "kind": "fleet_recovery",
            "priority": "high",
            "owner": "warden",
            "title": format!("Restore offline node {target_id}"),
            "reason": format!("Configured node `{target_id}` is offline and should be recovered or demoted from active routing."),
            "writes_through": ["config/fleet.toml", "core/edge/targets.toml", "core/state/fleet_runtime.json"],
        }));
    }

    for target in &rollout_targets {
        let Some(target_id) = target.get("target_id").and_then(Value::as_str) else {
            continue;
        };
        let endpoint = endpoint_by_target
            .get(target_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let models_total = target
            .get("models_total")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let models_completed = target
            .get("models_completed")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if models_total > 0
            && models_completed >= models_total
            && endpoint.get("has_live_endpoint").and_then(Value::as_bool) != Some(true)
        {
            actions.push(json!({
                "action_id": format!("verify_local_endpoint_{target_id}"),
                "kind": "endpoint_verification",
                "priority": "medium",
                "owner": "manwe",
                "title": format!("Verify local inference endpoint on {target_id}"),
                "reason": format!("All contracted model artifacts are present on `{target_id}`; verify or promote its local serving endpoint into routing."),
                "writes_through": ["core/state/model_control_surface.json", "core/state/manwe_router.json", "config/model_route_matrix.toml"],
            }));
        }
    }

    for target in &endpoint_targets {
        let Some(target_id) = target.get("target_id").and_then(Value::as_str) else {
            continue;
        };
        let rollout = rollout_by_target
            .get(target_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let models_total = rollout
            .get("models_total")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let models_completed = rollout
            .get("models_completed")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let active_pull = rollout
            .get("active_pull")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if target
            .get("endpoint_contract_drift")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            actions.push(json!({
                "action_id": format!("reconcile_endpoint_contract_{target_id}"),
                "kind": "endpoint_contract_reconciliation",
                "priority": "medium",
                "owner": "manwe",
                "title": format!("Reconcile endpoint contract for {target_id}"),
                "reason": format!(
                    "`{target_id}` is serving at `{}`, but the configured provider URL is `{}`.",
                    target.get("observed_base_url").and_then(Value::as_str).unwrap_or("unknown"),
                    target.get("configured_base_url").and_then(Value::as_str).unwrap_or("unknown"),
                ),
                "writes_through": ["config/manwe.providers.toml", "core/state/manwe_router.json", "core/state/model_control_surface.json"],
            }));
        } else if target.get("has_live_endpoint").and_then(Value::as_bool) != Some(true) {
            actions.push(json!({
                "action_id": format!("launch_missing_endpoint_{target_id}"),
                "kind": "endpoint_launch",
                "priority": "medium",
                "owner": "manwe",
                "title": format!("Launch local endpoint for {target_id}"),
                "reason": format!("`{target_id}` has model/runtime state but no live local inference endpoint detected on the governed ports."),
                "writes_through": ["core/state/edge_endpoint_verification.json", "core/edge/model_profiles.toml", "config/manwe.providers.toml"],
            }));
        } else if active_pull && models_total > models_completed {
            actions.push(json!({
                "action_id": format!("continue_rollout_{target_id}"),
                "kind": "rollout_followthrough",
                "priority": "medium",
                "owner": "manwe",
                "title": format!("Continue model rollout on {target_id}"),
                "reason": format!("`{target_id}` has {models_completed}/{models_total} contracted artifacts present; keep rollout active until the assigned profile is complete."),
                "writes_through": ["core/state/edge_model_rollout.json", "core/edge/model_profiles.toml"],
            }));
        }
    }

    if budget_summary
        .get("local_joule_pressure")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        actions.push(json!({
            "action_id": "shift_heavy_reasoning_off_local_joule_pressure",
            "kind": "route_shift",
            "priority": "high",
            "owner": "manwe",
            "title": "Shift heavy reasoning away from local constrained lanes",
            "reason": "Local joulework is above the configured soft cap; prefer backbone and cloud fallback for heavy reasoning tasks.",
            "writes_through": ["config/model_route_matrix.toml", "core/state/model_control_surface.json", "core/state/manwe_router.json"],
        }));
    }
    if budget_summary
        .get("provider_budget_pressure_total")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        > 0
    {
        actions.push(json!({
            "action_id": "deprioritize_budget_stressed_providers",
            "kind": "provider_budget_guard",
            "priority": "high",
            "owner": "manwe",
            "title": "Deprioritize budget-stressed providers",
            "reason": "One or more providers exceeded soft or hard monthly limits and should be deprioritized in routing.",
            "writes_through": ["config/manwe.providers.toml", "core/state/manwe_router.json", "core/state/runtime_budget_policy.json"],
        }));
    }
    for recovery in &runtime_recovery_actions {
        let kind = recovery.get("kind").and_then(Value::as_str).unwrap_or("");
        if !matches!(kind, "route_shift" | "reroute_retry" | "deferred_retry") {
            continue;
        }
        if kind == "route_shift" && route_shift_already_executed {
            continue;
        }
        let label = recovery.get("label").and_then(Value::as_str).unwrap_or("");
        let executor_run = executor_by_label
            .get(label)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let executor_status = executor_run
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("");
        if executor_status == "executed" {
            continue;
        }
        let mut reason = format!(
            "Shed receipts observed for `{label}`; recommended action is `{}`.",
            recovery
                .get("recommended_action")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
        if executor_status == "skipped" {
            reason.push_str(&format!(
                " Automatic execution skipped: `{}`.",
                executor_run
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ));
        } else if executor_status == "failed" {
            reason.push_str(" Automatic execution failed and needs review.");
        }
        let priority =
            if executor_status == "failed" || matches!(kind, "route_shift" | "reroute_retry") {
                "high"
            } else {
                "medium"
            };
        actions.push(json!({
            "action_id": format!("runtime_recovery_{label}"),
            "kind": "runtime_admission_recovery",
            "priority": priority,
            "owner": recovery.get("owner").cloned().unwrap_or_else(|| json!("prometheus")),
            "title": recovery.get("title").cloned().unwrap_or_else(|| json!(format!("Recover shed runtime work for {label}"))),
            "reason": reason,
            "writes_through": recovery.get("writes_through").cloned().unwrap_or_else(|| json!(["core/state/runtime_admission_recovery.json"])),
        }));
    }
    let mut activation_frontier_total = 0;
    for tool in &package_tools {
        if tool.get("activation_status").and_then(Value::as_str) == Some("activation_frontier") {
            activation_frontier_total += 1;
            actions.push(json!({
                "action_id": format!("activation_frontier_{}", tool.get("tool").and_then(Value::as_str).unwrap_or("unknown")),
                "kind": "activation_review",
                "priority": "medium",
                "owner": "prometheus",
                "title": format!("Review activation frontier for {}", tool.get("tool").and_then(Value::as_str).unwrap_or("unknown")),
                "reason": tool.get("next_action").cloned().unwrap_or(Value::Null),
                "writes_through": ["core/state/package_enablement.json", "core/state/package_runtime_activation.json"],
            }));
        }
    }
    let human_needed_total = operator_actions
        .get("summary")
        .and_then(|v| v.get("human_needed_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if human_needed_total == 0 && activation_frontier_total == 0 && actions.is_empty() {
        actions.push(json!({
            "action_id": "steady_state_hold",
            "kind": "steady_state",
            "priority": "low",
            "owner": "prometheus",
            "title": "Hold current runtime posture",
            "reason": "Fleet, budget, and activation state are currently within policy bounds.",
            "writes_through": [],
        }));
    }
    let payload = json!({
        "schema_version": "arda.fleet-steward-actions.v1",
        "generated_at_utc": now_utc(),
        "authority": "fleet_steward_action_projection",
        "doctrine": {
            "actions_are_recommendations_not_implicit_mutations": true,
            "writes_must_flow_through_sovereign_config_and_state_surfaces": true,
            "ui_optional": true,
            "agent_consumable": true,
        },
        "source_surfaces": {
            "runtime_governor_contract": "core/state/runtime_governor_contract.json",
            "runtime_budget_policy": "core/state/runtime_budget_policy.json",
            "package_enablement": "core/state/package_enablement.json",
            "operator_actions": "core/state/operator_actions.json",
            "edge_model_rollout": "core/state/edge_model_rollout.json",
            "edge_endpoint_verification": "core/state/edge_endpoint_verification.json",
            "runtime_admission_recovery": "core/state/runtime_admission_recovery.json",
            "runtime_admission_recovery_executor": "core/state/runtime_admission_recovery_executor.json",
            "runtime_recovery_route_governor": "core/state/runtime_recovery_route_governor.json",
        },
        "summary": {
            "actions_total": actions.len(),
            "high_priority_total": actions.iter().filter(|a| a.get("priority").and_then(Value::as_str) == Some("high")).count(),
            "steady_state": actions.len() == 1 && actions[0].get("kind").and_then(Value::as_str) == Some("steady_state"),
            "executed_runtime_recoveries_total": executor_runs.iter().filter(|run| run.get("status").and_then(Value::as_str) == Some("executed")).count(),
        },
        "actions": actions,
        "runtime_recovery_status": {
            "governor_status": runtime_recovery_route_governor.get("status").cloned().unwrap_or(Value::Null),
            "governor_origin": runtime_recovery_route_governor.get("current_origin").cloned().unwrap_or(Value::Null),
            "executor_generated_at_utc": runtime_admission_recovery_executor.get("generated_at_utc").cloned().unwrap_or(Value::Null),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_fleet_steward_write_intents_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/fleet_steward_write_intents.json");
    let steward = read_json_or(
        &root.join("core/state/fleet_steward_actions.json"),
        json!({}),
    );
    let actions = steward
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let intents: Vec<Value> = actions
        .iter()
        .filter(|action| action.is_object())
        .map(|action| {
            let action_id = action.get("action_id").and_then(Value::as_str).unwrap_or("unknown");
            let kind = action.get("kind").and_then(Value::as_str).unwrap_or("unknown");
            let files = action
                .get("writes_through")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            json!({
                "intent_id": format!("intent_{action_id}"),
                "source_action_id": action_id,
                "kind": kind,
                "title": action.get("title").cloned().unwrap_or(Value::Null),
                "status": "proposed",
                "priority": action.get("priority").cloned().unwrap_or(Value::Null),
                "owner": action.get("owner").cloned().unwrap_or(Value::Null),
                "target_files": files.clone(),
                "approval_required": matches!(kind, "provider_budget_guard" | "route_shift" | "fleet_recovery"),
                "mutation_scope": if files.is_empty() { "no_mutation" } else { "config_and_projection" },
                "reason": action.get("reason").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let payload = json!({
        "schema_version": "arda.fleet-steward-write-intents.v1",
        "generated_at_utc": now_utc(),
        "authority": "fleet_steward_write_intent_projection",
        "doctrine": {
            "intents_are_proposals_not_mutations": true,
            "all_config_changes_must_reference_target_files": true,
            "high_impact_changes_require_approval": true,
            "ui_and_agent_consumers_share_the_same_intent_surface": true,
        },
        "source_surfaces": {
            "fleet_steward_actions": "core/state/fleet_steward_actions.json",
        },
        "summary": {
            "intents_total": intents.len(),
            "approval_required_total": intents
                .iter()
                .filter(|i| i.get("approval_required").and_then(Value::as_bool) == Some(true))
                .count(),
            "proposed_total": intents
                .iter()
                .filter(|i| i.get("status").and_then(Value::as_str) == Some("proposed"))
                .count(),
        },
        "intents": intents,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn ssh_probe(target: &str, remote: &str) -> (Option<i32>, String, String) {
    match Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=4",
            target,
            remote,
        ])
        .output()
    {
        Ok(output) => (
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ),
        Err(err) => (None, String::new(), err.to_string()),
    }
}

pub(crate) fn export_fleet_power_guard_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/fleet_power_guard.json");
    let previous = read_json_or(&out_path, json!({}));
    let targets = [
        ("node-ser9-worker", "citadel@100.103.125.88", "bluefin"),
        (
            "node-backbone-server-01",
            "ardaserver@100.118.123.88",
            "backbone-server-01",
        ),
        ("node-pi5-warden", "numenor@100.110.85.37", "raspberrypi"),
    ];
    let remote = "printf 'HOST\\n'; hostnamectl --static 2>/dev/null || hostname 2>/dev/null || true; printf 'MASKS\\n'; systemctl is-enabled sleep.target suspend.target hibernate.target hybrid-sleep.target 2>/dev/null || true; printf 'LOGIN\\n'; cat /etc/systemd/logind.conf.d/arda-no-sleep.conf 2>/dev/null || true";
    let mut observed = Vec::new();
    for (target_id, ssh_target, canonical_hostname) in targets {
        let (code, out, err) = ssh_probe(ssh_target, remote);
        if err.contains("Operation not permitted") {
            observed.push(json!({
                "target_id": target_id,
                "ssh_target": ssh_target,
                "canonical_hostname": canonical_hostname,
                "probe_status": "probe_blocked",
                "sleep_targets_masked": false,
                "logind_override_present": false,
                "logind_override": [],
                "error": "ssh probe blocked by sandbox restrictions",
            }));
            continue;
        }
        let mut section = "";
        let mut host: Option<String> = None;
        let mut masks = Vec::new();
        let mut login_lines = Vec::new();
        for line in out.lines().map(str::trim_end) {
            match line {
                "HOST" | "MASKS" | "LOGIN" => {
                    section = line;
                    continue;
                }
                _ => {}
            }
            if section == "HOST" && !line.is_empty() && host.is_none() {
                host = Some(line.to_string());
            } else if section == "MASKS" && !line.is_empty() {
                masks.push(line.to_string());
            } else if section == "LOGIN" && !line.is_empty() {
                login_lines.push(line.to_string());
            }
        }
        let masked_ok = !masks.is_empty() && masks.iter().all(|line| line == "masked");
        let override_ok = login_lines
            .iter()
            .any(|line| line.contains("HandleLidSwitch=ignore"))
            && login_lines
                .iter()
                .any(|line| line.contains("IdleAction=ignore"));
        observed.push(json!({
            "target_id": target_id,
            "ssh_target": ssh_target,
            "canonical_hostname": canonical_hostname,
            "observed_hostname": host,
            "probe_status": "ok",
            "sleep_targets_masked": masked_ok,
            "mask_states": masks,
            "logind_override_present": override_ok,
            "logind_override": login_lines,
            "error": if err.trim().is_empty() {
                Value::Null
            } else {
                json!(err.trim())
            },
            "return_code": code,
        }));
    }
    if !observed.is_empty()
        && observed.iter().all(|target| {
            target.get("probe_status").and_then(Value::as_str) == Some("probe_blocked")
        })
        && previous.get("targets").and_then(Value::as_array).is_some()
    {
        let mut payload = previous;
        payload["generated_at_utc"] = json!(now_utc());
        payload["authority"] = json!("fleet_power_guard_probe_cached_fallback");
        payload["probe_status"] = json!("cached_fallback");
        payload["last_probe_error"] = json!("ssh probe blocked by sandbox restrictions");
        write_pretty_json(&out_path, &payload)?;
        return Ok(json!({ "out": rel(&out_path, &root) }));
    }
    let payload = json!({
        "schema_version": "arda.fleet-power-guard.v1",
        "generated_at_utc": now_utc(),
        "authority": "fleet_power_guard_probe",
        "probe_status": "ok",
        "summary": {
            "targets_total": observed.len(),
            "targets_hardened_total": observed.iter().filter(|target|
                target.get("sleep_targets_masked").and_then(Value::as_bool) == Some(true)
                && target.get("logind_override_present").and_then(Value::as_bool) == Some(true)
            ).count(),
            "targets_probe_blocked_total": observed.iter().filter(|target|
                target.get("probe_status").and_then(Value::as_str) == Some("probe_blocked")
            ).count(),
        },
        "targets": observed,
        "doctrine": {
            "sleep_is_not_permitted_on_runtime_nodes": true,
            "logind_idle_actions_must_be_ignored": true,
            "systemd_sleep_targets_must_be_masked": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_fleet_identity_reconciliation_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/fleet_identity_reconciliation.json");
    let fleet = read_json_or(
        &root.join("core/metrics/by_crate/prometheus/fleet_control.json"),
        json!({}),
    );
    let targets_toml = read_toml_or(
        &root.join("core/edge/targets.toml"),
        toml::Value::Table(Default::default()),
    );
    let targets = targets_toml
        .get("node")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let registry_nodes = load_topology_registry_nodes(&root.join("config/topology_registry.yaml"));

    let mut target_by_ip = HashMap::new();
    let mut target_by_host = HashMap::new();
    let generic_hosts: BTreeSet<&str> = ["raspberrypi", "localhost"].into_iter().collect();
    for target in &targets {
        if let Some(tbl) = target.as_table() {
            if let Some(ip) = tbl.get("tailscale_ip").and_then(toml::Value::as_str) {
                target_by_ip.insert(ip.to_string(), target.clone());
            }
            if let Some(host) = tbl.get("hostname").and_then(toml::Value::as_str) {
                if !generic_hosts.contains(host) {
                    target_by_host.insert(host.to_string(), target.clone());
                }
            }
        }
    }

    let mut active = Vec::new();
    let mut stale = Vec::new();
    let mut unresolved = Vec::new();
    let mut stale_by_hostname: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for node in fleet
        .get("fleet_nodes_full")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let ips = node
            .get("tailscale_ips")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut matched: Option<toml::Value> = None;
        for ip in &ips {
            if let Some(ip_str) = ip.as_str() {
                if let Some(target) = target_by_ip.get(ip_str) {
                    matched = Some(target.clone());
                    break;
                }
            }
        }
        if matched.is_none() {
            if let Some(host) = node.get("hostname").and_then(Value::as_str) {
                if !generic_hosts.contains(host) {
                    matched = target_by_host.get(host).cloned();
                }
            }
        }
        let matched_target_id = matched
            .as_ref()
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("id"))
            .and_then(toml::Value::as_str);
        let matched_target_role = matched
            .as_ref()
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("role"))
            .and_then(toml::Value::as_str);
        let record = json!({
            "tailscale_node_id": node.get("node_id").cloned().unwrap_or(Value::Null),
            "hostname": node.get("hostname").cloned().unwrap_or(Value::Null),
            "dns_name": node.get("dns_name").cloned().unwrap_or(Value::Null),
            "tailscale_ips": ips,
            "online": node.get("online").and_then(Value::as_bool).unwrap_or(false),
            "matched_target_id": matched_target_id,
            "matched_target_role": matched_target_role,
            "informant_source": node.get("informant_source").cloned().unwrap_or(Value::Null),
        });
        if node.get("online").and_then(Value::as_bool).unwrap_or(false) {
            active.push(record);
        } else {
            stale.push(record.clone());
            if let Some(host) = node.get("hostname").and_then(Value::as_str) {
                stale_by_hostname
                    .entry(host.to_string())
                    .or_default()
                    .push(record.clone());
            }
            if matched.is_none() {
                unresolved.push(record);
            }
        }
    }

    let active_target_ids: BTreeSet<String> = active
        .iter()
        .filter_map(|node| {
            node.get("matched_target_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    let mut unmatched_targets = Vec::new();
    for target in &targets {
        let Some(tbl) = target.as_table() else {
            continue;
        };
        let Some(target_id) = tbl.get("id").and_then(toml::Value::as_str) else {
            continue;
        };
        if active_target_ids.contains(target_id) {
            continue;
        }
        let enrollment = tbl
            .get("enrollment_status")
            .and_then(toml::Value::as_str)
            .unwrap_or("");
        unmatched_targets.push(json!({
            "target_id": target_id,
            "role": tbl.get("role").and_then(toml::Value::as_str),
            "hostname": tbl.get("hostname").and_then(toml::Value::as_str),
            "enrollment_status": enrollment,
            "node_class": tbl.get("node_class").and_then(toml::Value::as_str),
            "tailscale_ip": tbl.get("tailscale_ip").and_then(toml::Value::as_str),
            "needs_identity_binding": matches!(enrollment, "planned_discovery" | "active_staging"),
        }));
    }

    let stale_hostname_clusters: Vec<Value> = stale_by_hostname
        .iter()
        .map(|(hostname, records)| {
            json!({
                "hostname": hostname,
                "count": records.len(),
                "tailscale_node_ids": records.iter().filter_map(|record| record.get("tailscale_node_id").cloned()).collect::<Vec<_>>(),
                "dns_names": records.iter().filter_map(|record| record.get("dns_name").cloned()).collect::<Vec<_>>(),
            })
        })
        .collect();

    let mut registry_by_name = HashMap::new();
    for node in registry_nodes {
        if let Some(name) = node.get("tailscale_name").and_then(Value::as_str) {
            registry_by_name.insert(name.to_string(), node.clone());
        }
    }
    let mut recommendations = Vec::new();
    let mut binding_candidates = Vec::new();
    if unmatched_targets.iter().any(|item| {
        item.get("target_id").and_then(Value::as_str) == Some("node-pi5-citadel-avatar")
            && item.get("needs_identity_binding").and_then(Value::as_bool) == Some(true)
    }) {
        let topology_hint = registry_by_name
            .get("raspberrypi-2")
            .or_else(|| registry_by_name.get("raspberrypi-aihat"))
            .cloned()
            .unwrap_or(Value::Null);
        binding_candidates.push(json!({
            "target_id": "node-pi5-citadel-avatar",
            "binding_strategy": "reserve_canonical_identity_before_enrollment",
            "expected_hostname": "raspberrypi-aihat",
            "candidate_tailscale_names": ["raspberrypi-aihat", "raspberrypi-2"],
            "candidate_stale_node_ids": stale.iter()
                .filter(|record| record.get("hostname").and_then(Value::as_str) == Some("raspberrypi"))
                .filter_map(|record| record.get("tailscale_node_id").cloned())
                .collect::<Vec<_>>(),
            "topology_registry_hint": topology_hint,
            "requires_operator_confirmation": true,
        }));
        recommendations.push(json!({
            "target_id": "node-pi5-citadel-avatar",
            "action": "bind_canonical_identity",
            "reason": "The planned CITADEL avatar Pi5 target still has no canonical Tailscale identity or informant binding.",
        }));
    }
    if unmatched_targets.iter().any(|item| {
        item.get("target_id").and_then(Value::as_str) == Some("node-ser9-worker")
            && item.get("needs_identity_binding").and_then(Value::as_bool) == Some(true)
    }) {
        binding_candidates.push(json!({
            "target_id": "node-ser9-worker",
            "binding_strategy": "recover_expected_hostname_or_enroll_new_node",
            "expected_hostname": "bluefin",
            "candidate_tailscale_names": ["bluefin", "beelink-ser9pro"],
            "candidate_stale_node_ids": [],
            "topology_registry_hint": registry_by_name
                .get("bluefin")
                .or_else(|| registry_by_name.get("beelink-ser9pro"))
                .cloned()
                .unwrap_or(Value::Null),
            "requires_operator_confirmation": true,
        }));
        recommendations.push(json!({
            "target_id": "node-ser9-worker",
            "action": "recover_or_enroll_worker_identity",
            "reason": "The SER9 worker remains in planned discovery with no matched active node or canonical IP binding.",
        }));
    }
    for cluster in &stale_hostname_clusters {
        if cluster.get("count").and_then(Value::as_u64).unwrap_or(0) > 1 {
            recommendations.push(json!({
                "hostname": cluster.get("hostname").cloned().unwrap_or(Value::Null),
                "action": "retire_stale_duplicates",
                "reason": "Multiple stale Tailscale identities share the same generic hostname and should be retired or renamed to restore canonical identity mapping.",
            }));
        }
    }

    let payload = json!({
        "schema_version": "arda.fleet-identity-reconciliation.v1",
        "generated_at_utc": now_utc(),
        "authority": "fleet_identity_export",
        "summary": {
            "active_total": active.len(),
            "stale_total": stale.len(),
            "unresolved_stale_total": unresolved.len(),
        },
        "active_nodes": active,
        "stale_nodes": stale,
        "unresolved_stale_nodes": unresolved,
        "unmatched_configured_targets": unmatched_targets,
        "canonical_binding_candidates": binding_candidates,
        "stale_hostname_clusters": stale_hostname_clusters,
        "recommended_actions": recommendations,
        "guidance": {
            "use_ip_first": true,
            "generic_hostname_is_not_identity": true,
            "second_pi_needs_canonical_target": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "unresolved_stale_total": payload["summary"]["unresolved_stale_total"],
    }))
}

pub(crate) fn export_fleet_capability_ranking_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/fleet_capability_ranking.json");
    let fleet = read_toml_or(
        &root.join("config/fleet.toml"),
        toml::Value::Table(Default::default()),
    );
    let topology_nodes = load_topology_registry_nodes(&root.join("config/topology_registry.yaml"));
    let mut topology_by_id = HashMap::new();
    for entry in topology_nodes {
        if let Some(node_id) = entry.get("node_id").and_then(Value::as_str) {
            topology_by_id.insert(node_id.to_string(), entry.clone());
        }
    }
    let nodes = fleet
        .get("nodes")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut ranking_rows = Vec::new();
    let operator_nodes: Vec<Value> = topology_by_id
        .values()
        .filter(|entry| {
            matches!(
                entry
                    .get("labels")
                    .and_then(|v| v.get("role"))
                    .and_then(Value::as_str),
                Some("control" | "operator_control")
            )
        })
        .cloned()
        .collect();
    for operator in operator_nodes {
        let local_caps = operator
            .get("capabilities")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let ram_mb = local_caps.get("ram_mb").and_then(Value::as_i64);
        let local_ram_gib = ram_mb.map(|ram| ((ram as f64) / 1024.0).round() as i64);
        let local_gpu_model = local_caps.get("gpu_model").and_then(Value::as_str);
        let local_gpu_vram_gib = if local_gpu_model
            .map(|model| model.to_ascii_lowercase().contains("3080"))
            .unwrap_or(false)
        {
            Some(10)
        } else {
            None
        };
        let mut constraints = Vec::new();
        if local_ram_gib.is_some_and(|ram| ram <= 16) {
            constraints.push(json!("current_system_ram_is_16_gib_class"));
        }
        if local_gpu_model.is_some() {
            constraints.push(json!(
                "local_nvml_runtime_not_currently_observable_from_arda_context"
            ));
        }
        ranking_rows.push(json!({
            "node_id": operator.get("node_id").cloned().unwrap_or(Value::Null),
            "display_name": operator
                .get("labels")
                .and_then(|v| v.get("host_alias"))
                .cloned()
                .or_else(|| operator.get("node_id").cloned())
                .unwrap_or(Value::Null),
            "role": "operator_control",
            "execution_authority": "secondary",
            "measured_or_observed": "topology_registry",
            "cpu_model": operator.get("cpu_model").cloned().unwrap_or(Value::Null),
            "cpu_cores": local_caps.get("cpu_cores").cloned().unwrap_or(Value::Null),
            "system_ram_gib": local_ram_gib,
            "gpu_model": local_gpu_model,
            "gpu_vram_gib": local_gpu_vram_gib,
            "network_tier": local_caps.get("network_tier").cloned().unwrap_or(Value::Null),
            "strengths": classify_strengths(
                "operator_laptop",
                local_ram_gib,
                local_gpu_vram_gib,
                local_caps.get("cpu_cores").and_then(Value::as_i64),
            ),
            "constraints": constraints,
            "best_for": [
                "arda_hud_operator_surface",
                "opencode_shell_usage",
                "local_privacy_restricted_tasks",
                "interactive_coding_when_local_inference_is_desired"
            ],
            "not_best_for": [
                "always_on_backbone_services",
                "high_parallel_multi_service_runtime"
            ],
            "upgrade_priority": "raise_operator_node_ram_when_below_32_gib",
        }));
    }
    for node in nodes {
        let Some(tbl) = node.as_table() else {
            continue;
        };
        let topo = topology_entry_for_fleet_node(tbl, &topology_by_id);
        let topo_caps = topo
            .get("capabilities")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut system_ram_gib = None;
        if tbl
            .get("memory_profile")
            .and_then(toml::Value::as_str)
            .is_some_and(|raw| raw.starts_with("128_gib"))
        {
            system_ram_gib = Some(128);
        }
        if system_ram_gib.is_none() {
            system_ram_gib = topo_caps
                .get("ram_mb")
                .and_then(Value::as_i64)
                .map(|ram| ((ram as f64) / 1024.0).round() as i64);
        }
        let mut cpu_cores = topo_caps.get("cpu_cores").and_then(Value::as_i64);
        if cpu_cores.is_none()
            && tbl
                .get("cpu_profile")
                .and_then(toml::Value::as_str)
                .is_some_and(|profile| profile.starts_with("threadripper_2950x"))
        {
            cpu_cores = Some(16);
        }
        let gpu_vram_gib = derive_gpu_vram_gib(tbl, &topo);
        let label =
            if tbl.get("id").and_then(toml::Value::as_str) == Some("node-backbone-server-01") {
                "backbone_server"
            } else {
                tbl.get("node_class")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("")
            };
        let node_id = tbl.get("id").and_then(toml::Value::as_str).unwrap_or("");
        ranking_rows.push(json!({
            "node_id": node_id,
            "display_name": tbl.get("display_name").and_then(toml::Value::as_str),
            "role": tbl.get("role").and_then(toml::Value::as_str),
            "execution_authority": if node_id == "node-backbone-server-01" { "primary" } else { "specialized" },
            "measured_or_observed": "fleet_toml + topology_registry",
            "cpu_model": tbl.get("cpu_profile").and_then(toml::Value::as_str),
            "cpu_cores": cpu_cores,
            "system_ram_gib": system_ram_gib,
            "gpu_model": tbl.get("gpu_profile").and_then(toml::Value::as_str).or_else(|| topo_caps.get("gpu_model").and_then(Value::as_str)),
            "gpu_vram_gib": gpu_vram_gib,
            "network_tier": topo_caps.get("network_tier").cloned().unwrap_or_else(|| json!("tailnet")),
            "strengths": classify_strengths(label, system_ram_gib, gpu_vram_gib, cpu_cores),
            "constraints": [],
            "best_for": match node_id {
                "node-backbone-server-01" => json!(["manwe_primary_routing","oracle_reasoning","athena_deep_digest","aipkg_proving_ground","offsite_always_on_runtime"]),
                "node-ser9-worker" => json!(["edge_worker_execution","background_tasks","supplemental_reasoning"]),
                "node-pi5-warden" => json!(["warden_monitoring","guardhouse_alerting"]),
                "node-pi5-citadel-avatar" => json!(["avatar_product_control","bounded_embodied_workflows"]),
                _ => json!([]),
            },
            "not_best_for": match node_id {
                "node-backbone-server-01" => json!(["casual_operator_ui_hosting"]),
                "node-ser9-worker" => json!(["primary_deep_reasoning"]),
                "node-pi5-warden" => json!(["heavy_model_inference"]),
                "node-pi5-citadel-avatar" => json!(["general_backbone_compute"]),
                _ => json!([]),
            },
        }));
    }
    let primary = ranking_rows
        .iter()
        .find(|row| row.get("node_id").and_then(Value::as_str) == Some("node-backbone-server-01"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let operator = ranking_rows
        .iter()
        .find(|row| row.get("role").and_then(Value::as_str) == Some("operator_control"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let payload = json!({
        "schema_version": "arda.fleet-capability-ranking.v1",
        "generated_at_utc": now_utc(),
        "authority": "fleet_toml + topology_registry + measured_operator_hardware",
        "decision": {
            "primary_backbone_node": primary.get("node_id").cloned().unwrap_or(Value::Null),
            "primary_operator_node": operator.get("node_id").cloned().unwrap_or(Value::Null),
            "conclusion": "The backbone server remains the strongest always-on system node because of high system RAM and multi-service headroom, while any enrolled operator-class node may still be preferable for interactive single-seat work when local inference is desired.",
        },
        "rankings": {
            "always_on_backbone": [
                "node-backbone-server-01",
                "node-ser9-worker",
                operator.get("node_id").cloned().unwrap_or(Value::Null),
            ],
            "interactive_local_operator": [
                operator.get("node_id").cloned().unwrap_or(Value::Null),
                "node-backbone-server-01",
                "node-ser9-worker",
            ],
            "single_gpu_inference_comfort": [
                operator.get("node_id").cloned().unwrap_or(Value::Null),
                "node-ser9-worker",
                "node-backbone-server-01",
            ],
            "parallel_service_headroom": [
                "node-backbone-server-01",
                "node-ser9-worker",
                operator.get("node_id").cloned().unwrap_or(Value::Null),
            ],
            "offsite_relocation_suitability": [
                "node-backbone-server-01",
                "node-ser9-worker",
            ],
        },
        "nodes": ranking_rows,
        "recommended_upgrades": [{
            "target_node": operator.get("node_id").cloned().unwrap_or(Value::Null),
            "action": "upgrade_operator_node_ram",
            "minimum_target_gib": 32,
            "preferred_target_gib": 64,
            "reason": "A GPU-capable operator node below 32 GiB RAM becomes constrained under simultaneous HUD, browser, local inference, and development workloads.",
        }],
        "summary": {
            "nodes_total": payload_nodes_total_placeholder(),
            "primary_backbone_node": primary.get("node_id").cloned().unwrap_or(Value::Null),
            "primary_operator_node": operator.get("node_id").cloned().unwrap_or(Value::Null),
        },
    });
    let mut payload = payload;
    payload["summary"]["nodes_total"] =
        json!(payload["nodes"].as_array().map(|v| v.len()).unwrap_or(0));
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "nodes_total": payload["summary"]["nodes_total"],
    }))
}

fn topology_entry_for_fleet_node(
    node: &toml::map::Map<String, toml::Value>,
    topology: &HashMap<String, Value>,
) -> Value {
    let node_id = node.get("id").and_then(toml::Value::as_str).unwrap_or("");
    let hostname = node
        .get("hostname")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let tailscale_ip = node
        .get("tailscale_ip")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let mapping = [
        ("node-backbone-server-01", "backbone-beelink"),
        ("node-ser9-worker", "edge-ser9-bluefin"),
        ("node-pi5-warden", "edge-raspberrypi"),
        ("node-pi5-citadel-avatar", "edge-raspberrypi-2"),
    ];
    if let Some((_, mapped)) = mapping.iter().find(|(id, _)| *id == node_id) {
        if let Some(entry) = topology.get(*mapped) {
            return entry.clone();
        }
    }
    for entry in topology.values() {
        let endpoints = entry.get("endpoints").and_then(Value::as_object);
        if endpoints
            .and_then(|v| v.get("host"))
            .and_then(Value::as_str)
            == Some(tailscale_ip)
        {
            return entry.clone();
        }
        if endpoints
            .and_then(|v| v.get("tailscale_name"))
            .and_then(Value::as_str)
            == Some(hostname)
        {
            return entry.clone();
        }
    }
    json!({})
}

fn derive_gpu_vram_gib(node: &toml::map::Map<String, toml::Value>, topo: &Value) -> Option<i64> {
    let gpu_profile = node
        .get("gpu_profile")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if gpu_profile.contains("2080") && gpu_profile.contains("8g") {
        return Some(8);
    }
    let gpu_model = topo
        .get("capabilities")
        .and_then(|v| v.get("gpu_model"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if gpu_model.contains("3080") {
        return Some(10);
    }
    None
}

fn classify_strengths(
    label: &str,
    system_ram_gib: Option<i64>,
    gpu_vram_gib: Option<i64>,
    cpu_cores: Option<i64>,
) -> Vec<&'static str> {
    let mut strengths = Vec::new();
    if label == "backbone_server" {
        strengths.extend([
            "parallel_services",
            "offsite_backbone",
            "deep_reasoning",
            "package_lab",
        ]);
    }
    if label == "operator_laptop" {
        strengths.extend([
            "local_operator_control",
            "single_gpu_interactive",
            "fallback_inference",
        ]);
    }
    if system_ram_gib.is_some_and(|ram| ram >= 64) {
        strengths.push("large_memory_headroom");
    }
    if gpu_vram_gib.is_some_and(|vram| vram >= 10) {
        strengths.push("strong_single_gpu_lane");
    }
    if cpu_cores.is_some_and(|cores| cores >= 16) {
        strengths.push("high_parallel_cpu");
    }
    strengths
}

fn payload_nodes_total_placeholder() -> usize {
    0
}
