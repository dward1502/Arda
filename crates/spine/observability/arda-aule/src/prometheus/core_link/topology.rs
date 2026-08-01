#![cfg(feature = "full-cli")]
use super::{
    collect_file_entries_recursive, read_edge_targets, read_fleet_config_nodes, read_json_file,
    read_recent_jsonl, rel_path, CORE_STATE_SCHEMA_VERSION,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) fn write_runtime_topology_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("runtime_topology.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let configured_nodes =
        read_fleet_config_nodes(&workspace_root.join("config/fleet.toml")).unwrap_or_default();
    let edge_targets = read_edge_targets(&workspace_root.join("core/edge/targets.toml"))
        .or_else(|| read_edge_targets(&workspace_root.join("core/edge/targets.example.toml")))
        .unwrap_or_default();
    let local_informant =
        read_json_file(workspace_root.join("data/fleet/informants/local_last.json"))
            .unwrap_or_else(|| json!({}));
    let warden_edge_contract =
        read_json_file(core_root.join("state").join("warden_edge_contract.json"))
            .unwrap_or_else(|| json!({}));

    let guardhouse_node = configured_nodes
        .iter()
        .find(|node| node.get("role").and_then(Value::as_str) == Some("warden_guardhouse"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let guardhouse_target = edge_targets
        .iter()
        .find(|node| node.get("role").and_then(Value::as_str) == Some("warden_guardhouse"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let backbone_node = configured_nodes
        .iter()
        .find(|node| node.get("node_class").and_then(Value::as_str) == Some("backbone_compute"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let supervisor_order_raw = std::env::var("ARDA_SUPERVISOR_AGENT_ORDER")
        .unwrap_or_else(|_| "prometheus,manwe,hermes,hades,athena,mnemosyne".to_string());
    let local_supervisor_order = supervisor_order_raw
        .split(',')
        .map(|value: &str| value.trim().to_string())
        .filter(|value: &String| !value.is_empty())
        .collect::<Vec<String>>();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "runtime_topology_projection",
        "identity": {
            "sovereign_ceo_agent_id": "arandur",
            "sovereign_ceo_title": "Sovereign Executive Intelligence",
            "local_executable_ceo_runtime": "prometheus",
            "compatibility_shim": "arda-ceo reexports arda-aule"
        },
        "topology": {
            "control_plane_split": "sovereign_identity_vs_executable_runtime",
            "local_workstation_role": "same_host_control_plane",
            "edge_guardhouse_role": "warden_guardhouse",
            "backbone_role": "heavy_reasoning_and_multi_model_execution"
        },
        "local_control_plane": {
            "hostname": local_informant.get("hostname").cloned().unwrap_or(Value::Null),
            "node_id": local_informant.get("node_id").cloned().unwrap_or(Value::Null),
            "primary_runtime_agent": "prometheus",
            "supervisor_scope": "same_host_daemons_only",
            "supervisor_order": local_supervisor_order,
            "max_starts_per_pass": std::env::var("ARDA_SUPERVISOR_MAX_STARTS_PER_PASS").unwrap_or_else(|_| "1".to_string()),
            "start_stagger_ms": std::env::var("ARDA_SUPERVISOR_START_STAGGER_MS").unwrap_or_else(|_| "750".to_string()),
            "managed_agents": ["prometheus", "manwe", "hermes", "hades", "athena", "mnemosyne"],
            "deferred_or_remote_agents": ["warden", "oracle", "plutus", "apollo"],
            "ceo_authority_surface": "core/state/world.json"
        },
        "edge_guardhouse": {
            "configured_node": guardhouse_node,
            "edge_target": guardhouse_target,
            "authority_mode": "guardian_enforcement_and_queue_observation",
            "ack_mode": warden_edge_contract
                .get("edge_contract")
                .and_then(|value| value.get("ack_mode"))
                .cloned()
                .unwrap_or_else(|| json!("unknown")),
            "queue_path": warden_edge_contract
                .get("edge_contract")
                .and_then(|value| value.get("queue_path"))
                .cloned()
                .unwrap_or_else(|| json!("data/warden/informant_queue.jsonl"))
        },
        "backbone_compute": {
            "configured_node": backbone_node,
            "intended_role": "heavy_reasoning_and_backbone_model_execution"
        },
        "findings": [
            "arandur_remains_the_sovereign_ceo_identity",
            "prometheus_is_the_local_executable_ceo_control_plane",
            "warden_authority_is_assigned_to_the_edge_guardhouse_node",
            "local_agent_supervisor_is_bounded_to_same_host_daemons"
        ]
    });
    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

pub(super) fn write_manwe_router_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("manwe_router.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let status = read_manwe_http_json(&workspace_root, "/status")
        .and_then(|value| value.get("status").cloned())
        .or_else(|| {
            read_json_file(
                workspace_root
                    .join("core")
                    .join("metrics")
                    .join("by_crate")
                    .join("manwe")
                    .join("status.json"),
            )
        })
        .unwrap_or_else(|| json!({}));
    let providers = read_manwe_http_json(&workspace_root, "/providers")
        .map(normalize_manwe_providers_response)
        .or_else(|| {
            read_json_file(
                workspace_root
                    .join("core")
                    .join("metrics")
                    .join("by_crate")
                    .join("manwe")
                    .join("providers.json"),
            )
        })
        .unwrap_or_else(|| json!([]));
    let state = read_manwe_http_json(&workspace_root, "/state")
        .and_then(|value| value.get("state").cloned())
        .or_else(|| {
            read_json_file(
                workspace_root
                    .join("core")
                    .join("metrics")
                    .join("by_crate")
                    .join("manwe")
                    .join("state.json"),
            )
        })
        .unwrap_or_else(|| json!({}));
    let bootstrap = read_json_file(
        workspace_root
            .join("core")
            .join("state")
            .join("fleet_bootstrap.json"),
    )
    .unwrap_or_else(|| json!({}));
    let bootstrap_recovery = read_json_file(
        workspace_root
            .join("core")
            .join("state")
            .join("fleet_bootstrap_recovery.json"),
    )
    .unwrap_or_else(|| json!({}));
    let recent_events = read_recent_jsonl(
        &workspace_root
            .join("data")
            .join("manwe")
            .join("state.jsonl"),
        24,
    );

    let provider_rows = providers.as_array().cloned().unwrap_or_default();
    let provider_rows = provider_rows
        .into_iter()
        .map(enrich_manwe_provider_pressure)
        .collect::<Vec<_>>();
    let cooldowns = provider_rows
        .iter()
        .filter(|provider| provider.get("in_cooldown").and_then(Value::as_bool) == Some(true))
        .cloned()
        .collect::<Vec<_>>();
    let degraded = provider_rows
        .iter()
        .filter(|provider| {
            provider
                .get("consecutive_failures")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 2
                || provider
                    .get("error_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    >= 5
        })
        .cloned()
        .collect::<Vec<_>>();
    let local_provider = provider_rows
        .iter()
        .find(|provider| provider.get("id").and_then(Value::as_str) == Some("local_fallback"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let tool_context_floor = std::env::var("ARDA_MANWE_TOOL_EXECUTION_MIN_CONTEXT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value >= 16_000)
        .unwrap_or(64_000);
    let budget_pressure = provider_rows
        .iter()
        .filter(|provider| {
            provider
                .get("budget_pressure_level")
                .and_then(Value::as_str)
                .is_some_and(|level| level != "ok")
        })
        .cloned()
        .collect::<Vec<_>>();

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "manwe_router_projection",
        "status": status,
        "routing_defaults": {
            "privacy_tier": std::env::var("ARDA_ROUTE_PRIVACY_DEFAULT").unwrap_or_else(|_| "public".to_string()),
            "cost_tier": std::env::var("ARDA_ROUTE_COST_DEFAULT").unwrap_or_else(|_| "balanced".to_string()),
            "quality_tier": std::env::var("ARDA_ROUTE_QUALITY_DEFAULT").unwrap_or_else(|_| "balanced".to_string()),
            "origin_preference": std::env::var("ARDA_ROUTE_ORIGIN_DEFAULT").unwrap_or_else(|_| "auto".to_string()),
            "latency_sla_ms": std::env::var("ARDA_ROUTE_LATENCY_SLA_MS").ok().and_then(|v| v.parse::<u64>().ok())
        },
        "provider_pressure": {
            "providers": provider_rows,
            "cooldowns": cooldowns,
            "degraded": degraded,
            "budget_pressure": budget_pressure,
            "local_fallback": local_provider
        },
        "route_guardrails": {
            "tool_execution_min_context_window": tool_context_floor,
            "hermes_tool_routing": "Hermes/code/tool routes require tool-capable, non-visible-reasoning models at or above the context floor unless an explicit emergency low-context fallback flag is set."
        },
        "bootstrap_state": bootstrap,
        "bootstrap_recovery": bootstrap_recovery,
        "state_snapshot": state,
        "recent_events": recent_events,
        "arda_hints": {
            "primary_panel": "inference_router",
            "provider_count": providers.as_array().map(|items| items.len()).unwrap_or(0),
            "cooldown_count": cooldowns.len(),
            "degraded_count": degraded.len(),
            "recovery_failed_total": bootstrap_recovery
                .get("summary")
                .and_then(|value| value.get("restart_failed_total"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn read_manwe_http_json(workspace_root: &Path, path: &str) -> Option<Value> {
    if cfg!(test) || !workspace_root.join("core").join("realm").exists() {
        return None;
    }

    let addr = std::env::var("ARDA_MANWE_HTTP_HOST")
        .ok()
        .filter(|host| matches!(host.as_str(), "127.0.0.1" | "localhost"))
        .map(|host| {
            let port = std::env::var("ARDA_MANWE_HTTP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(5110);
            format!("{host}:{port}")
        })
        .unwrap_or_else(|| "127.0.0.1:5110".to_string());
    let addr = addr.parse::<SocketAddr>().ok()?;
    let timeout = Duration::from_millis(1_500);
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            Err(_) => return None,
        }
    }
    let response = String::from_utf8_lossy(&bytes);
    let (_headers, body) = response.split_once("\r\n\r\n")?;
    serde_json::from_str(body).ok()
}

fn normalize_manwe_providers_response(value: Value) -> Value {
    value
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .map(Value::Array)
        .unwrap_or(value)
}

fn enrich_manwe_provider_pressure(provider: Value) -> Value {
    let mut provider = provider;
    let enabled = provider
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_api_key = provider
        .get("has_api_key")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let healthy = provider
        .get("healthy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let in_cooldown = provider
        .get("in_cooldown")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let consecutive_failures = provider
        .get("consecutive_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let error_count = provider
        .get("error_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let requests_per_minute = provider.get("requests_per_minute").and_then(Value::as_u64);
    let requests_used_minute = provider
        .get("requests_used_minute")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let requests_per_day = provider.get("requests_per_day").and_then(Value::as_u64);
    let requests_used_day = provider
        .get("requests_used_day")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let minute_ratio = quota_ratio(requests_used_minute, requests_per_minute);
    let day_ratio = quota_ratio(requests_used_day, requests_per_day);
    let operational_state = if !enabled {
        "disabled"
    } else if !has_api_key {
        "missing_api_key"
    } else if requests_per_day.is_some_and(|max| requests_used_day >= max) {
        "rate_limited"
    } else if requests_per_minute.is_some_and(|max| requests_used_minute >= max) {
        "minute_quota_exhausted"
    } else if in_cooldown {
        "cooldown"
    } else if !healthy {
        "unhealthy"
    } else if consecutive_failures >= 2 || error_count >= 5 {
        "degraded"
    } else {
        "ready"
    };
    let budget_pressure_level = if in_cooldown
        || requests_per_day.is_some_and(|max| requests_used_day >= max)
        || requests_per_minute.is_some_and(|max| requests_used_minute >= max)
        || minute_ratio.is_some_and(|ratio| ratio >= 0.90)
        || day_ratio.is_some_and(|ratio| ratio >= 0.90)
    {
        "critical"
    } else if minute_ratio.is_some_and(|ratio| ratio >= 0.75)
        || day_ratio.is_some_and(|ratio| ratio >= 0.75)
    {
        "warning"
    } else {
        "ok"
    };
    let blocked = matches!(
        operational_state,
        "disabled"
            | "missing_api_key"
            | "rate_limited"
            | "minute_quota_exhausted"
            | "cooldown"
            | "unhealthy"
    );

    if let Some(map) = provider.as_object_mut() {
        map.insert("operational_state".to_string(), json!(operational_state));
        map.insert("blocked".to_string(), json!(blocked));
        map.insert(
            "minute_usage_ratio".to_string(),
            minute_ratio.map(Value::from).unwrap_or(Value::Null),
        );
        map.insert(
            "day_usage_ratio".to_string(),
            day_ratio.map(Value::from).unwrap_or(Value::Null),
        );
        map.insert(
            "budget_pressure_level".to_string(),
            json!(budget_pressure_level),
        );
    }
    provider
}

fn quota_ratio(used: u64, max: Option<u64>) -> Option<f64> {
    max.filter(|value| *value > 0)
        .map(|value| (used as f64 / value as f64).clamp(0.0, 1.0))
}

pub(super) fn write_output_topology_projection(core_root: &Path) {
    let snapshot_path = core_root.join("state").join("output_topology.json");
    let workspace_root = core_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let metrics_history = workspace_root.join("core").join("metrics").join("history");
    let data_root = workspace_root.join("data");
    let human_root = workspace_root.join("human");

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "output_topology_projection",
        "surfaces": [
            {
                "id": "sovereign_runtime",
                "path": rel_path(core_root.join("state"), &workspace_root),
                "classification": "operational_authority",
                "purpose": "machine-readable runtime truth consumed by ARDA and local agents"
            },
            {
                "id": "execution_ledgers",
                "path": rel_path(
                    crate::prometheus::queue_authority::canonical_project_task_queue(&workspace_root),
                    &workspace_root,
                ),
                "classification": "operational_authority",
                "purpose": "authoritative project/task ledger"
            },
            {
                "id": "runtime_queue",
                "path": rel_path(workspace_root.join("core/queue/queue.jsonl"), &workspace_root),
                "classification": "operational_authority",
                "purpose": "active runtime task queue"
            },
            {
                "id": "metrics_latest",
                "path": rel_path(workspace_root.join("core/metrics/by_crate"), &workspace_root),
                "classification": "operational_observability",
                "purpose": "latest exported metrics snapshots"
            },
            {
                "id": "metrics_history",
                "path": rel_path(metrics_history.clone(), &workspace_root),
                "classification": "accounted_history",
                "purpose": "immutable metrics snapshots for audit and longitudinal analysis"
            },
            {
                "id": "append_only_ledgers",
                "path": rel_path(data_root.clone(), &workspace_root),
                "classification": "operational_ledgers",
                "purpose": "raw append-only system stores and durable histories"
            },
            {
                "id": "human_visual_layer",
                "path": rel_path(human_root.clone(), &workspace_root),
                "classification": "human_visualization",
                "purpose": "graph-oriented human and machine thought surfaces"
            },
            {
                "id": "build_artifacts",
                "path": rel_path(workspace_root.join("target"), &workspace_root),
                "classification": "non_operational_rebuildable",
                "purpose": "local build cache and temp execution artifacts"
            }
        ],
        "counts": {
            "data_jsonl_files": collect_file_entries_recursive(&data_root, "jsonl").len(),
            "human_markdown_files": collect_file_entries_recursive(&human_root, "md").len(),
            "metrics_history_snapshots": fs::read_dir(&metrics_history).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).count()).unwrap_or(0)
        },
        "long_term_accounting_candidates": [
            {
                "path": rel_path(workspace_root.join("data/prometheus/autopilot"), &workspace_root),
                "reason": "valuable audit/history surface but not required as sovereign runtime truth",
                "recommended_action": "mirror_tree_compact",
                "priority": "high",
                "estimated_joulework": 1.0,
                "exclude_globs": ["bin/**"],
                "compress_globs": ["*.log"]
            },
            {
                "path": rel_path(workspace_root.join("data/prometheus/supervisor"), &workspace_root),
                "reason": "operator/support telemetry, not primary runtime authority",
                "recommended_action": "mirror_tree_compact",
                "priority": "high",
                "estimated_joulework": 0.8,
                "compress_globs": ["*.log"]
            },
            {
                "path": rel_path(workspace_root.join("core/metrics/history"), &workspace_root),
                "reason": "longitudinal accountability and replay, distinct from live runtime control",
                "recommended_action": "mirror_tree",
                "priority": "medium",
                "estimated_joulework": 2.0
            },
            {
                "path": rel_path(workspace_root.join("target"), &workspace_root),
                "reason": "rebuildable local artifacts that should stay outside long-term operational truth",
                "recommended_action": "snapshot_manifest",
                "priority": "low",
                "estimated_joulework": 0.5
            }
        ],
        "arda_hints": {
            "primary_panel": "output_topology",
            "boardroom_section": "state_vs_history",
            "alert_on_large_history_surface": fs::read_dir(&metrics_history).map(|entries| entries.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).count()).unwrap_or(0) > 20
        }
    });

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}
