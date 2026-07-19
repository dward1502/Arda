#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

use super::*;

pub(crate) fn export_matrix_boardroom_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let config_path = root.join("config/matrix_boardrooms.toml");
    let out_path = root.join("core/state/matrix_boardrooms.json");
    let config = read_toml_or(&config_path, toml::Value::Table(Default::default()));
    let payload = json!({
        "schema_version": "arda.matrix-boardrooms.v1",
        "generated_at_utc": now_utc(),
        "authority": "matrix_element_boardroom_contract",
        "config_path": rel(&config_path, &root),
        "defaults": config.get("defaults").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "root_space": config.get("root_space").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "rooms": config.get("rooms").cloned().unwrap_or(toml::Value::Array(vec![])),
        "adapters": config.get("adapters").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "adapter_policy": config.get("adapter_policy").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "routing_contract": config.get("routing_contract").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "bridge_contracts": config.get("bridge_contracts").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "activation_requirements": config.get("activation_requirements").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "implementation_targets": {
            "runtime_surface": "core/state/federated_comms_runtime.json",
            "doctrine_surface": "core/state/federated_comms.json",
            "command_surface": "core/state/hermes_command.json",
            "primary_owner": "hermes",
        },
        "governance_validators": {
            "triad_required": true,
            "bacon_lite_required": true,
            "joulework_required": true,
            "love_equation_required": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_federated_comms_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/federated_comms.json");
    let runtime_out = root.join("core/state/federated_comms_runtime.json");
    let config_path = root.join("config/federated_comms.toml");
    let config = read_toml_or(&config_path, toml::Value::Table(Default::default()));
    let matrix_boardrooms =
        read_json_or(&root.join("core/state/matrix_boardrooms.json"), json!({}));
    let matrix = config
        .get("matrix")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let element = config
        .get("element")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let adapters = config
        .get("adapters")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let contract = config
        .get("contract")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let bitmesh = config
        .get("bitmesh")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let nanoclaw = config
        .get("nanoclaw")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let selinux = config
        .get("selinux")
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let routing_contract = matrix_boardrooms
        .get("routing_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let bridge_contracts = matrix_boardrooms
        .get("bridge_contracts")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let activation_requirements = matrix_boardrooms
        .get("activation_requirements")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let boardroom_rooms = matrix_boardrooms
        .get("rooms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let payload = json!({
        "schema_version": "arda.federated-comms.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_source_src_56a1f7ca",
        "source_id": "src_56a1f7ca",
        "layers": [
            {"id":"local_sovereign","transport":["unix_socket","loopback_http"],"role":"core authority and same-host control plane"},
            {"id":"trusted_internal","transport":["tailscale","internal_http","a2a","mcp"],"role":"device-to-device sovereign operations"},
            {"id":"federated_rooms","transport":["matrix"],"role":"human and agent boardrooms, rooms, and bridges"},
            {"id":"economic_discovery","transport":["fetchai_style_discovery"],"role":"optional discovery, settlement, and marketplace coordination"},
            {"id":"future_bitmesh","transport":["libp2p","offline_relay","anonymity_preserving_routing"],"role":"future anonymous or off-grid mesh layer"},
        ],
        "doctrine": {
            "whatsapp_in_core": false,
            "discord_optional_bridge": true,
            "discord_not_primary_surface": true,
            "tailscale_current_internal_backbone": true,
            "selinux_expected_on_bluefin": true,
            "matrix_preferred_federation_surface": true,
            "element_preferred_client_surface": true,
            "adapters_must_remain_agnostic": true,
            "blockchain_in_hot_path": false,
        },
        "nanoclaw_alignment": {
            "required_mode": "local_or_tailscale_or_matrix",
            "whatsapp_required": false,
        },
        "boardroom_contract": {
            "source": "core/state/matrix_boardrooms.json",
            "root_space": matrix_boardrooms.get("root_space").cloned().unwrap_or_else(|| json!({})),
            "routing_contract": routing_contract,
            "bridge_contracts": bridge_contracts,
            "room_ids": boardroom_rooms.iter().filter_map(|room| room.get("id").and_then(Value::as_str)).collect::<Vec<_>>(),
        },
        "governance_validators": {
            "triad_required": true,
            "bacon_lite_required": true,
            "joulework_required": true,
            "love_equation_required": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;

    let runtime = json!({
        "schema_version": "arda.federated-comms-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "config/federated_comms.toml + federated_comms_doctrine",
        "config_path": rel(&config_path, &root),
        "internal_backbone": contract.get("internal_backbone").and_then(toml::Value::as_str).unwrap_or("tailscale"),
        "local_control": contract.get("local_control").cloned().unwrap_or(toml::Value::Array(vec![toml::Value::String("unix_socket".into()), toml::Value::String("loopback_http".into())])),
        "trusted_internal": contract.get("trusted_internal").cloned().unwrap_or(toml::Value::Array(vec![toml::Value::String("internal_http".into()), toml::Value::String("a2a".into()), toml::Value::String("mcp".into())])),
        "discord_mode": contract.get("discord_mode").and_then(toml::Value::as_str).unwrap_or("deferred"),
        "matrix": {
            "enabled": matrix.get("enabled").and_then(toml::Value::as_bool).unwrap_or(false),
            "homeserver_url": matrix.get("homeserver_url").and_then(toml::Value::as_str).unwrap_or(""),
            "bot_user_id": matrix.get("bot_user_id").and_then(toml::Value::as_str).unwrap_or(""),
            "default_space": matrix.get("default_space").and_then(toml::Value::as_str).unwrap_or(""),
            "access_token_env": matrix.get("access_token_env").and_then(toml::Value::as_str).unwrap_or("ARDA_MATRIX_ACCESS_TOKEN"),
            "ready": matrix.get("enabled").and_then(toml::Value::as_bool).unwrap_or(false)
                && matrix.get("homeserver_url").and_then(toml::Value::as_str).map(|s| !s.is_empty()).unwrap_or(false)
                && activation_requirements.get("element_client_ready").and_then(Value::as_bool).unwrap_or(false),
        },
        "element": {
            "preferred_client": element.get("preferred_client").and_then(toml::Value::as_bool).unwrap_or(true),
            "desktop_enabled": element.get("desktop_enabled").and_then(toml::Value::as_bool).unwrap_or(false),
            "web_url": element.get("web_url").and_then(toml::Value::as_str).unwrap_or("https://app.element.io/"),
        },
        "boardrooms": {
            "source": "core/state/matrix_boardrooms.json",
            "room_count": boardroom_rooms.len(),
            "root_space_alias": matrix_boardrooms.get("root_space").and_then(|v| v.get("alias")).and_then(Value::as_str).unwrap_or(""),
            "operator_entrypoint": matrix_boardrooms.get("root_space").and_then(|v| v.get("operator_entrypoint")).and_then(Value::as_str).unwrap_or(""),
            "routing_contract": matrix_boardrooms.get("routing_contract").cloned().unwrap_or_else(|| json!({})),
            "bridge_contracts": matrix_boardrooms.get("bridge_contracts").cloned().unwrap_or_else(|| json!({})),
            "activation_requirements": activation_requirements,
        },
        "adapters": {
            "strategy": contract.get("adapter_strategy").and_then(toml::Value::as_str).unwrap_or("agnostic_bridges"),
            "discord_enabled": adapters.get("discord_enabled").and_then(toml::Value::as_bool).unwrap_or(false)
                || bridge_contracts.get("discord").and_then(|v| v.get("enabled")).and_then(Value::as_bool).unwrap_or(false),
            "generic_webhook_enabled": adapters.get("generic_webhook_enabled").and_then(toml::Value::as_bool).unwrap_or(false)
                || bridge_contracts.get("webhook").and_then(|v| v.get("enabled")).and_then(Value::as_bool).unwrap_or(false),
        },
        "nanoclaw": {
            "control_mode": nanoclaw.get("control_mode").and_then(toml::Value::as_str).unwrap_or("tailscale_internal"),
            "aligned": matches!(nanoclaw.get("control_mode").and_then(toml::Value::as_str), Some("local_only" | "tailscale_internal" | "matrix_federated")),
        },
        "selinux": {
            "expected_on_bluefin": selinux.get("expected_on_bluefin").and_then(toml::Value::as_bool).unwrap_or(true),
            "runtime_socket_root": selinux.get("runtime_socket_root").and_then(toml::Value::as_str).unwrap_or("/run/user/$UID/arda"),
        },
        "bitmesh": {
            "mode": bitmesh.get("mode").and_then(toml::Value::as_str).unwrap_or("planned"),
            "transports": bitmesh.get("transports").cloned().unwrap_or(toml::Value::Array(vec![
                toml::Value::String("libp2p".into()),
                toml::Value::String("offline_relay".into()),
                toml::Value::String("anonymity_preserving_routing".into()),
            ])),
        },
        "layer_activation": {
            "local_sovereign_ready": true,
            "trusted_internal_ready": contract.get("internal_backbone").and_then(toml::Value::as_str).unwrap_or("tailscale") == "tailscale",
            "federated_rooms_ready": matrix.get("enabled").and_then(toml::Value::as_bool).unwrap_or(false)
                && matrix.get("homeserver_url").and_then(toml::Value::as_str).map(|s| !s.is_empty()).unwrap_or(false)
                && matrix_boardrooms.get("activation_requirements").and_then(|v| v.get("element_client_ready")).and_then(Value::as_bool).unwrap_or(false),
            "economic_discovery_active": contract.get("blockchain_hot_path").and_then(toml::Value::as_bool).unwrap_or(false),
            "future_bitmesh_ready": matches!(bitmesh.get("mode").and_then(toml::Value::as_str), Some("pilot" | "active")),
        },
    });
    write_pretty_json(&runtime_out, &runtime)?;
    Ok(json!({ "out": rel(&out_path, &root), "runtime_out": rel(&runtime_out, &root) }))
}

fn package_runtime_key(tool: &str) -> &str {
    match tool {
        "playwright-mcp" => "playwright_mcp",
        "oh-my-opencode" => "oh_my_opencode",
        other => other,
    }
}

fn latest_by_source(rows: &[Value]) -> BTreeMap<String, Value> {
    let mut latest = BTreeMap::new();
    for row in rows {
        let Some(source_id) = row.get("source_id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if !source_id.is_empty() {
            latest.insert(source_id.to_string(), row.clone());
        }
    }
    latest
}

fn framework_entry(root: &std::path::Path, name: &str, path: &str) -> Option<Value> {
    let payload = read_json_or(&root.join(path), json!({}));
    if payload == json!({}) {
        return None;
    }
    Some(json!({
        "name": name,
        "source_id": payload.get("source_id").cloned().unwrap_or(Value::Null),
        "source_url": payload.get("source_url").cloned().unwrap_or(Value::Null),
        "path": path,
        "implementation_targets": payload.get("implementation_targets").cloned().unwrap_or(Value::Null),
        "arda_adaptation": payload.get("arda_adaptation").cloned().unwrap_or(Value::Null),
        "pattern_extraction": payload.get("pattern_extraction").cloned().unwrap_or(Value::Null),
        "follow_ons": payload.get("arda_follow_ons").cloned().unwrap_or(Value::Null),
    }))
}

fn infer_disposition(
    tool_row: &Value,
    runtime_row: &Value,
    framework_sources: &BTreeSet<String>,
) -> Value {
    let integration_state = tool_row
        .get("integration_state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let activation_status = tool_row
        .get("activation_status")
        .and_then(Value::as_str)
        .unwrap_or("");
    let source_id = tool_row
        .get("source_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lane = tool_row
        .get("integration_lane")
        .and_then(Value::as_str)
        .unwrap_or("research");
    let (mode, rationale) = if activation_status == "active_in_system" {
        (
            "active_in_system",
            "This repo is already live inside arda runtime contracts.",
        )
    } else if activation_status == "active_signal" {
        ("active_signal", "This repo is already feeding sovereign decision signals rather than waiting on first activation.")
    } else if activation_status == "governed_on_demand" {
        (
            "governed_on_demand",
            "This repo is integrated as an on-demand governed capability, not a resident daemon.",
        )
    } else if activation_status == "blocked_on_auth" {
        ("blocked_on_auth", "The runtime contract fits, but activation is correctly blocked on missing auth or enrollment.")
    } else if integration_state == "ready_for_activation" {
        (
            "activate_now",
            "Evidence, runtime visibility, and env contracts are already aligned.",
        )
    } else if integration_state == "configuration_ready" {
        (
            "contract_ready",
            "Evidence and runtime are present; only configuration or credentials remain.",
        )
    } else if integration_state == "evidence_ready" {
        ("implementation_lane", "ATHENA digestion is strong enough; next work is runtime or product surface implementation.")
    } else if !source_id.is_empty() && framework_sources.contains(source_id) {
        (
            "framework_reference",
            "This repo is integrated primarily as a comparative framework reference.",
        )
    } else if matches!(
        lane,
        "agent_skills" | "knowledge" | "research" | "mcp_communications"
    ) {
        ("extension_candidate", "The repo is digested and mapped, but should remain an optional extension until bounded runtime surfaces exist.")
    } else {
        ("research_only", "The repo is observed in-system but not yet promoted into a bounded runtime or package contract.")
    };
    json!({
        "mode": mode,
        "rationale": rationale,
        "runtime_status": runtime_row.get("status").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn export_github_repo_integration_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/github_repo_integration.json");
    let registry = read_toml_or(
        &root.join("docs/registry.toml"),
        toml::Value::Table(Default::default()),
    );
    let source_rows = latest_by_source(&read_jsonl_objects(
        &root.join("data/knowledge/athena/index/sources.jsonl"),
    ));
    let github_sources = source_rows
        .into_iter()
        .filter(|(_, row)| {
            row.get("source_type")
                .and_then(Value::as_str)
                .map(|v| v.eq_ignore_ascii_case("githubrepo"))
                .unwrap_or(false)
        })
        .collect::<BTreeMap<_, _>>();
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let package_surfaces = package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let frameworks = [
        framework_entry(&root, "OpenFang", "core/state/openfang_alignment.json"),
        framework_entry(&root, "AgentForge", "core/state/agentforge_alignment.json"),
        framework_entry(&root, "eliza", "core/state/eliza_alignment.json"),
        framework_entry(&root, "Tauri", "core/state/tauri_embodiment.json"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let framework_sources = frameworks
        .iter()
        .filter_map(|entry| {
            entry
                .get("source_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();

    let mut tools = Vec::new();
    let mut linked_source_ids = BTreeSet::new();
    let mut ready_count = 0usize;
    let mut observed_only_count = 0usize;
    for tool_row in package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let tool = tool_row
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let source_id = tool_row
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let source_row = github_sources
            .get(source_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let runtime_row = package_surfaces
            .get(package_runtime_key(tool))
            .cloned()
            .unwrap_or(Value::Null);
        if !source_id.is_empty() {
            linked_source_ids.insert(source_id.to_string());
        }
        if tool_row.get("integration_state").and_then(Value::as_str) == Some("ready_for_activation")
        {
            ready_count += 1;
        }
        if tool_row.get("integration_state").and_then(Value::as_str) == Some("observed_only") {
            observed_only_count += 1;
        }
        let extra_surface = if source_id == "src_cebb6abe" && !frameworks.is_empty() {
            frameworks
                .first()
                .and_then(|v| v.get("path").and_then(Value::as_str))
                .map(|s| s.to_string())
        } else {
            None
        };
        let mut system_surfaces = vec![
            "core/state/package_enablement.json".to_string(),
            "core/state/package_runtime_activation.json".to_string(),
            "core/state/operator_actions.json".to_string(),
        ];
        if let Some(surface) = extra_surface {
            system_surfaces.push(surface);
        }
        system_surfaces.dedup();
        tools.push(json!({
            "tool": tool,
            "repo": tool_row.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": tool_row.get("repo_url").cloned().unwrap_or(Value::Null),
            "source_id": if source_id.is_empty() { Value::Null } else { json!(source_id) },
            "athena": {
                "status": source_row.get("status").cloned().unwrap_or(Value::Null),
                "confidence": source_row.get("confidence").cloned().unwrap_or(Value::Null),
                "triad_passed": source_row.get("triad_passed").cloned().unwrap_or(Value::Null),
                "book_ref": source_row.get("book_ref").cloned().unwrap_or(Value::Null),
                "human_ref": source_row.get("human_ref").cloned().unwrap_or(Value::Null),
                "updated_at_utc": source_row.get("ts_utc").cloned().unwrap_or(Value::Null),
            },
            "package_enablement": {
                "integration_lane": tool_row.get("integration_lane").cloned().unwrap_or(Value::Null),
                "integration_state": tool_row.get("integration_state").cloned().unwrap_or(Value::Null),
                "activation_status": tool_row.get("activation_status").cloned().unwrap_or(Value::Null),
                "policy_readiness": tool_row.get("policy_readiness").cloned().unwrap_or(Value::Null),
                "policy_confidence": tool_row.get("policy_confidence").cloned().unwrap_or(Value::Null),
                "next_action": tool_row.get("next_action").cloned().unwrap_or(Value::Null),
            },
            "runtime": runtime_row,
            "disposition": infer_disposition(tool_row, &package_surfaces.get(package_runtime_key(tool)).cloned().unwrap_or_else(|| json!({})), &framework_sources),
            "system_surfaces": system_surfaces,
        }));
    }

    let mut research_backlog = Vec::new();
    let mut sorted_sources = github_sources.into_iter().collect::<Vec<_>>();
    sorted_sources.sort_by_key(|(_, row)| {
        row.get("ts_utc")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    });
    sorted_sources.reverse();
    for (source_id, row) in sorted_sources {
        if linked_source_ids.contains(&source_id) || framework_sources.contains(&source_id) {
            continue;
        }
        research_backlog.push(json!({
            "source_id": source_id,
            "url": row.get("url").cloned().unwrap_or(Value::Null),
            "status": row.get("status").cloned().unwrap_or(Value::Null),
            "confidence": row.get("confidence").cloned().unwrap_or(Value::Null),
            "triad_passed": row.get("triad_passed").cloned().unwrap_or(Value::Null),
            "book_ref": row.get("book_ref").cloned().unwrap_or(Value::Null),
            "human_ref": row.get("human_ref").cloned().unwrap_or(Value::Null),
            "updated_at_utc": row.get("ts_utc").cloned().unwrap_or(Value::Null),
            "disposition": {
                "mode": "research_backlog",
                "rationale": "ATHENA has already digested this GitHub source; it is now explicitly captured in the sovereign corpus backlog even if it is not yet promoted into a registry tool or framework contract.",
            },
            "system_surfaces": ["core/state/github_repo_integration.json","core/state/athena_runtime.json"],
        }));
    }

    let registry_tools = registry
        .get("tools")
        .and_then(toml::Value::as_table)
        .map(|table| table.len())
        .unwrap_or(tools.len());
    let payload = json!({
        "schema_version": "arda.github-repo-integration.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_github_repo_integration_export",
        "summary": {
            "github_sources_total": research_backlog.len() + linked_source_ids.len() + framework_sources.len(),
            "integration_coverage_total": research_backlog.len() + linked_source_ids.len() + framework_sources.len(),
            "registry_tools_total": registry_tools,
            "registry_linked_total": linked_source_ids.len(),
            "framework_linked_total": framework_sources.len(),
            "ready_for_activation_total": ready_count,
            "observed_only_total": observed_only_count,
            "research_backlog_total": research_backlog.len(),
        },
        "registry_tools": tools,
        "framework_surfaces": frameworks,
        "research_backlog_top": research_backlog.into_iter().take(25).collect::<Vec<_>>(),
        "arda_hints": {
            "primary_panel": "operations_and_packages",
            "boardroom_section": "github_corpus_integration",
            "alert_on_research_backlog": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_embodied_controller_runtime_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/embodied_controller_runtime.json");
    let embodied = read_json_or(&root.join("core/state/embodied_interface.json"), json!({}));
    let fleet = read_json_or(
        &root.join("core/metrics/by_crate/prometheus/fleet_control.json"),
        json!({}),
    );
    let fleet_runtime = read_json_or(&root.join("core/state/fleet_runtime.json"), json!({}));
    let mut statuses = BTreeMap::new();
    let informant_dir = root.join("data/fleet/informants");
    if informant_dir.exists() {
        for entry in std::fs::read_dir(&informant_dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if let Some(target_id) = name.strip_suffix("_controller_last.json") {
                let payload = read_json_or(&path, json!({}));
                if payload != json!({}) {
                    statuses.insert(target_id.to_string(), payload);
                }
            }
        }
    }
    let mut fleet_by_target = BTreeMap::new();
    for row in fleet
        .get("fleet_nodes_active_view")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let target_id = row
            .get("node_declared_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !target_id.is_empty() {
            fleet_by_target.insert(target_id.to_string(), row.clone());
        }
    }
    for group in [
        fleet.get("fleet_nodes_full"),
        fleet_runtime
            .get("inventory")
            .and_then(|v| v.get("merged_nodes")),
    ] {
        for row in group.and_then(Value::as_array).into_iter().flatten() {
            let configured = row.get("configured").cloned().unwrap_or_else(|| json!({}));
            let observed = row.get("observed").cloned().unwrap_or_else(|| json!({}));
            let target_id = configured
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !target_id.is_empty() && !fleet_by_target.contains_key(target_id) {
                let mut merged = observed;
                merged["node_declared_id"] = json!(target_id);
                fleet_by_target.insert(target_id.to_string(), merged);
            }
        }
    }
    let hardware_targets = embodied
        .get("hardware_targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| {
            row.get("node")
                .and_then(|v| v.get("node_id"))
                .and_then(Value::as_str)
                .map(|s| !s.is_empty())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let controllers = hardware_targets.into_iter().map(|row| {
        let node = row.get("node").cloned().unwrap_or_else(|| json!({}));
        let target_id = node.get("node_id").and_then(Value::as_str).unwrap_or_default();
        let node_status = statuses.get(target_id).cloned().unwrap_or_else(|| json!({}));
        json!({
            "target_node": node,
            "bootstrap_state": row.get("bootstrap_state").cloned().unwrap_or(Value::Null),
            "fleet_active_view": fleet_by_target.get(target_id).cloned().unwrap_or_else(|| json!({})),
            "controller_status": node_status,
            "readiness": {
                "ssh_ready": row.get("bootstrap_state").and_then(Value::as_str) == Some("tailscale_visible_ssh_ready"),
                "controller_dirs_ready": node_status.get("service_dirs").and_then(|v| v.get("app")).and_then(Value::as_bool).unwrap_or(false)
                    && node_status.get("service_dirs").and_then(|v| v.get("state")).and_then(Value::as_bool).unwrap_or(false)
                    && node_status.get("service_dirs").and_then(|v| v.get("logs")).and_then(Value::as_bool).unwrap_or(false),
                "venv_ready": node_status.get("venv_present").and_then(Value::as_bool).unwrap_or(false),
                "contract_synced": node_status.get("contract_present").and_then(Value::as_bool).unwrap_or(false),
                "node_toolchain_ready": !node_status.get("python").cloned().unwrap_or(Value::Null).is_null(),
            }
        })
    }).collect::<Vec<_>>();
    let primary = controllers.first().cloned().unwrap_or_else(|| json!({}));
    let payload = json!({
        "schema_version": "arda.embodied-controller-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "embodied_controller_export",
        "target_node": primary.get("target_node").cloned().unwrap_or(Value::Null),
        "bootstrap_state": primary.get("bootstrap_state").cloned().unwrap_or(Value::Null),
        "fleet_active_view": primary.get("fleet_active_view").cloned().unwrap_or_else(|| json!({})),
        "controller_status": primary.get("controller_status").cloned().unwrap_or_else(|| json!({})),
        "readiness": primary.get("readiness").cloned().unwrap_or_else(|| json!({})),
        "controllers": controllers,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_edge_enrollment_plan_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/edge_enrollment_plan.json");
    let targets = read_toml_or(
        &root.join("core/edge/targets.toml"),
        toml::Value::Table(Default::default()),
    )
    .get("node")
    .and_then(toml::Value::as_array)
    .cloned()
    .unwrap_or_default()
    .into_iter()
    .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
    .collect::<Vec<_>>();
    let fleet_recon = read_json_or(
        &root.join("core/state/fleet_identity_reconciliation.json"),
        json!({}),
    );
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let embodied = read_json_or(&root.join("core/state/embodied_interface.json"), json!({}));
    let candidate_by_target = fleet_recon
        .get("canonical_binding_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("target_id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let unmatched_by_target = fleet_recon
        .get("unmatched_configured_targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("target_id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let hardware_targets = embodied
        .get("hardware_targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("node")
                .and_then(|v| v.get("node_id"))
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let action_titles = operator_actions
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("title").and_then(Value::as_str).map(str::to_string))
        .collect::<Vec<_>>();
    let planned_targets = targets
        .into_iter()
        .filter(|target| {
            matches!(
                target.get("enrollment_status").and_then(Value::as_str),
                Some("planned_discovery" | "active_staging")
            )
        })
        .collect::<Vec<_>>();

    let plans = planned_targets.into_iter().map(|target| {
        let target_id = target.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
        let candidate = candidate_by_target.get(&target_id).cloned();
        let unmatched = unmatched_by_target.get(&target_id).cloned();
        let hardware_target = hardware_targets.get(&target_id).cloned();
        let needs_binding = unmatched.as_ref().and_then(|v| v.get("needs_identity_binding")).and_then(Value::as_bool).unwrap_or(false);
        let bootstrap_state = hardware_target.as_ref().and_then(|v| v.get("bootstrap_state")).cloned().unwrap_or(Value::Null);
        let mut steps = Vec::new();
        if needs_binding {
            steps.push(json!({"step":"Confirm canonical hostname and Tailscale identity before enrollment","status":"pending","evidence":"core/state/fleet_identity_reconciliation.json"}));
        }
        if candidate.as_ref().and_then(|v| v.get("candidate_stale_node_ids")).and_then(Value::as_array).map(|rows| !rows.is_empty()).unwrap_or(false) {
            steps.push(json!({"step":"Retire or rename stale duplicate identities that could collide with enrollment","status":"pending","evidence":"core/state/operator_actions.json"}));
        }
        if target_id == "node-pi5-citadel-avatar" {
            let ssh_ready = target.get("tailscale_ip").and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false)
                && target.get("ssh_user").and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false);
            steps.push(json!({"step":"Assign the live Tailscale IP and establish SSH trust for the Pi5 AI HAT controller","status": if ssh_ready { "completed" } else { "pending" }, "evidence":"core/edge/targets.toml"}));
            steps.push(json!({"step":"Run embodied controller bootstrap once the canonical identity is bound","status": if bootstrap_state.as_str()==Some("tailscale_visible_ssh_ready") { "ready" } else { "pending" }, "evidence":"scripts/runtime/bootstrap_embodied_controller.sh"}));
        }
        if target_id == "node-ser9-worker" {
            let worker_live = target.get("tailscale_ip").and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false)
                && matches!(target.get("enrollment_status").and_then(Value::as_str), Some("active" | "active_staging"));
            steps.push(json!({"step":"Recover networking or enroll the SER9 host under the expected hostname","status": if worker_live { "completed" } else { "pending" }, "evidence":"core/state/fleet_identity_reconciliation.json"}));
            steps.push(json!({"step":"Populate Tailscale IP and promote runtime roles after first successful informant scan","status": if worker_live { "completed" } else { "pending" }, "evidence":"core/edge/targets.toml"}));
        }
        json!({
            "target_id": target_id,
            "hostname": target.get("hostname").cloned().unwrap_or(Value::Null),
            "role": target.get("role").cloned().unwrap_or(Value::Null),
            "node_class": target.get("node_class").cloned().unwrap_or(Value::Null),
            "enrollment_status": target.get("enrollment_status").cloned().unwrap_or(Value::Null),
            "tailscale_ip": target.get("tailscale_ip").cloned().unwrap_or(Value::Null),
            "bootstrap_state": bootstrap_state,
            "identity_binding": candidate.unwrap_or(Value::Null),
            "operator_action_open": action_titles.iter().any(|title| title.contains(&target_id)),
            "next_steps": steps,
        })
    }).collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.edge-enrollment-plan.v1",
        "generated_at_utc": now_utc(),
        "authority": "edge_enrollment_plan_export",
        "summary": {
            "planned_targets_total": plans.len(),
            "identity_binding_required_total": plans.iter().filter(|plan| !plan.get("identity_binding").cloned().unwrap_or(Value::Null).is_null()).count(),
            "operator_action_open_total": plans.iter().filter(|plan| plan.get("operator_action_open").and_then(Value::as_bool).unwrap_or(false)).count(),
        },
        "plans": plans,
        "guidance": {
            "bind_identity_before_bootstrap": true,
            "do_not_promote_generic_hostnames": true,
            "use_task_boundaries_for_parallel_edge_work": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}
