#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Value};

use super::*;

fn github_by_tool(data: &Value) -> std::collections::BTreeMap<String, Value> {
    data.get("registry_tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("tool")
                .and_then(Value::as_str)
                .map(|tool| (tool.to_string(), row.clone()))
        })
        .collect()
}

pub(crate) fn export_remote_operator_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/remote_operator_contract.json");
    let fleet = read_json_or(
        &root.join("core/state/fleet_capability_ranking.json"),
        json!({}),
    );
    let model_control = read_json_or(
        &root.join("core/state/model_control_surface.json"),
        json!({}),
    );
    let opencode = read_json_or(
        &root.join("core/state/opencode_productization_contract.json"),
        json!({}),
    );
    let governor = read_json_or(
        &root.join("core/state/opencode_route_governor.json"),
        json!({}),
    );
    let edge_lab = read_json_or(
        &root.join("core/state/aipkg_edge_lab_contract.json"),
        json!({}),
    );
    let remote_operator_receipts = read_json_or(
        &root.join("data/remote_operator/workstation_last_result.json"),
        json!({}),
    );

    let providers = model_control
        .get("charon_providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let backbone = fleet
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("node_id").and_then(Value::as_str) == Some("node-backbone-server-01"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let operator = fleet
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("role").and_then(Value::as_str) == Some("operator_control"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let openrouter = providers
        .get("openrouter")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let edge_backbone = providers
        .get("edge_backbone")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let local_fallback = providers
        .get("local_fallback")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.remote-operator-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "fleet_capability_ranking + model_control_surface + opencode_productization_contract",
        "mission": {
            "goal": "Allow ARDA and OpenCode to be used from any enrolled operator node while execution lands on the sovereign system by default, with optional local inference contribution when the operator node is suitable.",
            "operator_device_is_not_execution_authority": true,
        },
        "topology": {
            "primary_operator_node": {
                "node_id": operator.get("node_id").cloned().unwrap_or(Value::Null),
                "display_name": operator.get("display_name").cloned().unwrap_or(Value::Null),
                "intended_role": "arda_hud + opencode shell + optional local fallback inference",
            },
            "primary_execution_node": {
                "node_id": backbone.get("node_id").cloned().unwrap_or(Value::Null),
                "display_name": backbone.get("display_name").cloned().unwrap_or(Value::Null),
                "transport": edge_lab.get("runtime_contract").and_then(|v| v.get("primary_transport")).cloned().unwrap_or_else(|| json!("tailscale")),
                "remote_use_ready": edge_lab.get("edge_target").and_then(|v| v.get("remote_use_ready")).and_then(Value::as_bool).unwrap_or(false),
            },
        },
        "execution_doctrine": {
            "default_dispatch": "remote_system_execution",
            "governor_authority": "charon + opencode_route_governor",
            "hud_authority": "observer_and_override_only",
            "opencode_shell_role": "consumer_of_route_contracts_not_hidden_authority",
            "tailscale_is_required_for_offsite_backbone": true,
        },
        "tool_surface": {
            "workstation_command_surface": {
                "script": "scripts/remote/operator/remote_operator_workstation.sh",
                "authority": "remote_operator_workstation + remote_operator_sync + remote_operator_probe + remote_operator_charon",
                "transport": "tailscale_ssh_with_managed_operator_alias",
                "receipt_log": "data/remote_operator/workstation_attempts.jsonl",
                "last_result_path": "data/remote_operator/workstation_last_result.json",
                "actions": [
                    {
                        "action": "status",
                        "class": "inspection",
                        "effect": "show_remote_operator_workstation_config"
                    },
                    {
                        "action": "probe",
                        "class": "inspection",
                        "effect": "check_remote_operator_network_and_endpoint_health"
                    },
                    {
                        "action": "charon-smoke",
                        "class": "governed_remote_execution",
                        "effect": "verify_charon_routing_path_from_operator_node"
                    },
                    {
                        "action": "sync",
                        "class": "code_sync",
                        "effect": "push_repo_to_workstation_without_remote_check"
                    },
                    {
                        "action": "sync-check",
                        "class": "code_sync_and_verification",
                        "effect": "push_repo_to_workstation_then_run_focused_remote_cli_check"
                    },
                    {
                        "action": "pull",
                        "class": "code_sync",
                        "effect": "pull_repo_from_workstation_to_operator_node"
                    },
                    {
                        "action": "cargo-check-cli",
                        "class": "verification",
                        "effect": "run_remote_cli_focused_cargo_check"
                    },
                    {
                        "action": "cargo-check-athena",
                        "class": "verification",
                        "effect": "run_remote_athena_focused_cargo_check"
                    },
                    {
                        "action": "cargo-check-prometheus",
                        "class": "verification",
                        "effect": "run_remote_prometheus_focused_cargo_check"
                    },
                    {
                        "action": "athena-status",
                        "class": "runtime_inspection",
                        "effect": "run_remote_athena_status_surface"
                    },
                    {
                        "action": "export-athena-plan",
                        "class": "export",
                        "effect": "regenerate_remote_athena_integration_plan"
                    },
                    {
                        "action": "git-status",
                        "class": "inspection",
                        "effect": "show_remote_repo_worktree_state"
                    },
                    {
                        "action": "command",
                        "class": "break_glass_remote_command",
                        "effect": "run_explicit_remote_shell_command",
                        "guardrails": [
                            "operator_or_governance_approved_only",
                            "prefer_named_actions_first"
                        ]
                    }
                ],
                "latest_receipt": remote_operator_receipts,
            }
        },
        "routing_policy": {
            "default_execution_targets": ["edge_backbone", "litellm_gateway", "local_fallback"],
            "operator_local_inference_allowed_when": [
                "task_is_privacy_restricted_or_operator_requested",
                "local_fallback_provider_is_healthy",
                "workload_fits_operator_node_memory_and_gpu_constraints",
            ],
            "operator_local_inference_not_preferred_for": [
                "deep_reasoning_bursts",
                "parallel_background_work",
                "always_on_backbone_services",
            ],
            "cloud_fallback": {
                "provider_id": "openrouter",
                "auth_ready": openrouter.get("auth_ready").and_then(Value::as_bool).unwrap_or(false),
                "preferred_models": ["nvidia/nemotron-3-super-120b-a12b:free", "openrouter/auto"],
            },
        },
        "arda_hud_implications": {
            "long_term_model": "run_hud_on_any_operator_node_observe_state_from_arda_and_dispatch_work_to_system_nodes",
            "needed_panels": ["execution_target_preview", "operator_node_status", "backbone_reachability", "local_inference_eligibility", "workstation_action_surface"],
        },
        "readiness": {
            "opencode_ready": opencode.get("summary").and_then(|v| v.get("runtime_ready")).and_then(Value::as_bool).unwrap_or(false),
            "route_governor_present": governor != json!({}),
            "backbone_provider_healthy": edge_backbone.get("healthy").and_then(Value::as_bool).unwrap_or(false),
            "local_fallback_healthy": local_fallback.get("healthy").and_then(Value::as_bool).unwrap_or(false),
            "openrouter_auth_ready": openrouter.get("auth_ready").and_then(Value::as_bool).unwrap_or(false),
            "workstation_action_surface_ready": !remote_operator_receipts.is_null(),
        },
        "next_productization_steps": [
            "Expose execution-target preview in ARDA so the operator can see laptop vs backbone vs cloud before dispatch.",
            "Let CHARON publish operator-node eligibility for local inference based on health and memory posture.",
            "Keep OpenCode as the primary governed shell for system operation; use Codex as specialist repo surgery when needed.",
        ],
        "summary": {
            "remote_backbone_ready": edge_lab.get("edge_target").and_then(|v| v.get("remote_use_ready")).and_then(Value::as_bool).unwrap_or(false),
            "openrouter_auth_ready": openrouter.get("auth_ready").and_then(Value::as_bool).unwrap_or(false),
            "primary_operator_node": operator.get("node_id").cloned().unwrap_or(Value::Null),
            "primary_execution_node": backbone.get("node_id").cloned().unwrap_or(Value::Null),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "remote_backbone_ready": payload.get("summary").and_then(|v| v.get("remote_backbone_ready")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_tool_garage_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/tool_garage_contract.json");
    let remote_operator = read_json_or(
        &root.join("core/state/remote_operator_contract.json"),
        json!({}),
    );
    let active_ruleset = read_json_or(&root.join("core/state/active_ruleset.json"), json!({}));

    let workstation_actions = remote_operator
        .get("tool_surface")
        .and_then(|value| value.get("workstation_command_surface"))
        .and_then(|value| value.get("actions"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let payload = json!({
        "schema_version": "arda.tool-garage-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "remote_operator_contract + active_ruleset + system_constitution",
        "mission": {
            "goal": "Provide one governed registry of callable capabilities across universal and subsystem-specific tool families.",
            "do_not_duplicate_shell_habits_as_hidden_authority": true,
        },
        "doctrine": {
            "tool_calls_must_be_named_and_receipt_backed": true,
            "placement_must_be_explicit": true,
            "governance_lane_must_be_explicit": true,
            "prefer_named_tool_actions_before_break_glass_shell": true,
        },
        "families": [
            {
                "family_id": "universal_workstation",
                "kind": "universal",
                "placement": "workstation",
                "surface": "scripts/remote/operator/remote_operator_workstation.sh",
                "receipt_log": "data/remote_operator/workstation_attempts.jsonl",
                "governance_lane": "bounded_remote_operator_actions",
                "actions": workstation_actions,
            },
            {
                "family_id": "remote_operator_transport",
                "kind": "universal",
                "placement": "operator_terminal",
                "surface": "scripts/remote/operator/remote_operator_sync.sh + scripts/remote/operator/remote_operator_probe.sh + scripts/remote/operator/remote_operator_charon.sh",
                "receipt_log": "data/remote_operator/",
                "governance_lane": "remote_operator_transport",
                "actions": [
                    {
                        "action": "sync",
                        "class": "code_sync"
                    },
                    {
                        "action": "probe",
                        "class": "inspection"
                    },
                    {
                        "action": "charon-smoke",
                        "class": "governed_remote_execution"
                    }
                ]
            }
        ],
        "subsystem_tool_families_planned": [
            {
                "family_id": "athena_tools",
                "kind": "subsystem_specific",
                "placement": "workstation_first",
                "planned_actions": ["ingest", "deep_process", "policy_promote", "generate_planning_tasks"]
            },
            {
                "family_id": "hermes_tools",
                "kind": "subsystem_specific",
                "placement": "operator_terminal + workstation",
                "planned_actions": ["voice_ingress", "boardroom_dispatch", "discord_ingress"]
            },
            {
                "family_id": "charon_tools",
                "kind": "subsystem_specific",
                "placement": "workstation",
                "planned_actions": ["route_probe", "provider_reload", "governor_apply"]
            }
        ],
        "governance_mapping": {
            "break_glass_actions_require_explicit_operator_or_policy_approval": true,
            "remote_build_and_export_actions_prefer_workstation": active_ruleset
                .get("policy")
                .and_then(|value| value.get("build_verification_routing"))
                .cloned()
                .unwrap_or_else(|| json!({})),
        },
        "summary": {
            "tool_families_total": 2,
            "registered_workstation_actions_total": workstation_actions.len(),
            "has_universal_workstation_surface": true,
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "registered_workstation_actions_total": payload.get("summary").and_then(|value| value.get("registered_workstation_actions_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_communication_adapter_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/communication_adapter_contract.json");
    let federated = read_json_or(
        &root.join("core/state/federated_comms_runtime.json"),
        json!({}),
    );
    let extension_contract = read_json_or(
        &root.join("core/state/extension_surface_contract.json"),
        json!({}),
    );
    let extension_backlog = read_json_or(
        &root.join("core/state/extension_activation_backlog.json"),
        json!({}),
    );
    let github = github_by_tool(&read_json_or(
        &root.join("core/state/github_repo_integration.json"),
        json!({}),
    ));

    let comm_lane = extension_contract
        .get("extension_lanes")
        .and_then(|v| v.get("communication_adapters"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let comm_tools = comm_lane
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let backlog_entries = extension_backlog
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.get("lane").and_then(Value::as_str) == Some("communication_adapters"))
        .collect::<Vec<_>>();

    let adapters = comm_tools
        .into_iter()
        .filter_map(|tool| {
            let tool_name = tool.get("tool").and_then(Value::as_str)?.to_string();
            let row = github.get(&tool_name).cloned().unwrap_or_else(|| json!({}));
            let backlog_row = backlog_entries
                .iter()
                .find(|item| item.get("tool").and_then(Value::as_str) == Some(tool_name.as_str()))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let package = row.get("package_enablement").cloned().unwrap_or_else(|| json!({}));
            let runtime = row.get("runtime").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "tool": tool_name,
                "repo_url": tool.get("repo_url").cloned().unwrap_or(Value::Null),
                "activation_status": tool.get("activation_status").cloned().unwrap_or(Value::Null),
                "phase": backlog_row.get("phase").cloned().unwrap_or(Value::Null),
                "integration_lane": tool.get("integration_lane").cloned().unwrap_or(Value::Null),
                "runtime_status": runtime.get("status").cloned().unwrap_or(Value::Null),
                "next_action": tool.get("next_action").cloned().unwrap_or_else(|| package.get("next_action").cloned().unwrap_or(Value::Null)),
                "policy_confidence": package.get("policy_confidence").cloned().unwrap_or(Value::Null),
                "system_surfaces": row.get("system_surfaces").cloned().unwrap_or_else(|| json!([])),
            }))
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.communication-adapter-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "hermes_adapter_materialization",
        "ownership": {
            "owner": "hermes",
            "routing_surface": "core/state/hermes_command.json",
            "transport_surface": "core/state/federated_comms_runtime.json",
        },
        "doctrine": {
            "adapters_optional": true,
            "matrix_boardroom_is_primary_human_agent_room_surface": true,
            "mcp_or_connector_adapters_must_be_policy_guarded": true,
            "discord_not_sovereign_base_layer": true,
            "discord_bridge_allowed_when_policy_guarded": true,
            "browser_sessions_governed_on_demand": true,
        },
        "transport_contract": {
            "internal_backbone": federated.get("internal_backbone").cloned().unwrap_or(Value::Null),
            "trusted_internal": federated.get("trusted_internal").cloned().unwrap_or(Value::Null),
            "adapter_strategy": federated.get("adapters").and_then(|v| v.get("strategy")).cloned().unwrap_or(Value::Null),
            "discord_mode": federated.get("discord_mode").cloned().unwrap_or(Value::Null),
            "boardroom_source": federated.get("boardrooms").and_then(|v| v.get("source")).cloned().unwrap_or(Value::Null),
            "client_adapter_room_id": federated.get("boardrooms").and_then(|v| v.get("routing_contract")).and_then(|v| v.get("client_adapter_room_id")).cloned().unwrap_or(Value::Null),
        },
        "adapters": adapters,
        "activation_order": [
            "stabilize matrix-boardroom adapter room as the sovereign bridge envelope",
            "keep playwright-mcp as governed on-demand browser capability",
            "keep Hermes-owned Discord bridge bounded and optional",
            "promote discord-mcp only if the Hermes bridge proves insufficient for a bounded use case",
        ],
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_opencode_productization_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/opencode_productization_contract.json");
    let routes = read_toml_or(
        &root.join("config/opencode_agent_routes.toml"),
        toml::Value::Table(Default::default()),
    );
    let enablement = read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let backlog = read_json_or(
        &root.join("core/state/extension_activation_backlog.json"),
        json!({}),
    );

    let package_row = enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("oh-my-opencode"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_surface = runtime
        .get("surfaces")
        .and_then(|v| v.get("oh_my_opencode"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let backlog_entries = backlog
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.get("tool").and_then(Value::as_str) == Some("oh-my-opencode"))
        .collect::<Vec<_>>();
    let agents = routes
        .get("agents")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();
    let agents_total = agents.len();

    let lane_map = [
        ("workflow_templates", "apollo_prometheus", "Promote OpenCode agent route recipes into bounded workflow templates and crate-spawn scaffolds.", vec!["config/opencode_agent_routes.toml", "core/state/package_runtime_activation.json", "core/state/extension_surface_contract.json"]),
        ("client_shells_and_embodiment", "arda", "Use OpenCode as a governed agent shell and embodiment consumer rather than a source of machine truth.", vec!["config/opencode_agent_routes.toml", "core/state/embodied_interface.json", "core/state/package_runtime_activation.json"]),
        ("skills_plugins_and_optional_extensions", "prometheus", "Bound optional OpenCode skills and plugin behavior behind sovereign route and package contracts.", vec!["config/opencode_agent_routes.toml", "core/state/package_enablement.json", "core/state/package_runtime_activation.json"]),
    ];

    let lanes = lane_map
        .into_iter()
        .map(|(lane_name, owner, purpose, write_through)| {
            let matching = backlog_entries.iter().find(|row| row.get("lane").and_then(Value::as_str) == Some(lane_name)).cloned().unwrap_or_else(|| json!({}));
            json!({
                "lane": lane_name,
                "owner": owner,
                "purpose": purpose,
                "activation_status": package_row.get("activation_status").cloned().unwrap_or(Value::Null),
                "integration_state": package_row.get("integration_state").cloned().unwrap_or(Value::Null),
                "runtime_status": matching.get("runtime_status").cloned().unwrap_or_else(|| runtime_surface.get("status").cloned().unwrap_or(Value::Null)),
                "next_action": matching.get("next_action").cloned().unwrap_or_else(|| package_row.get("next_action").cloned().unwrap_or(Value::Null)),
                "write_through": write_through,
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.opencode-productization-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "opencode_route_contract + package_runtime_activation + extension_activation_backlog",
        "tool": {
            "name": "oh-my-opencode",
            "repo": package_row.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": package_row.get("repo_url").cloned().unwrap_or(Value::Null),
            "binary_path": package_row.get("binary_path").cloned().unwrap_or(Value::Null),
            "activation_status": package_row.get("activation_status").cloned().unwrap_or(Value::Null),
            "integration_state": package_row.get("integration_state").cloned().unwrap_or(Value::Null),
            "runtime_status": runtime_surface.get("status").cloned().unwrap_or(Value::Null),
            "route_contract_ready": runtime_surface.get("route_contract_ready").cloned().unwrap_or(Value::Null),
            "route_contract_path": runtime_surface.get("route_contract_path").cloned().unwrap_or(Value::Null),
        },
        "doctrine": {
            "ui_optional": true,
            "agent_first_contract": true,
            "routes_define_usage_not_hud_state": true,
            "workflow_templates_must_promote_into_sovereign_recipes": true,
            "client_shell_use_is_consumer_not_authority": true,
            "skills_and_plugins_remain_bounded_extensions": true,
        },
        "route_defaults": routes.get("defaults").cloned().unwrap_or(toml::Value::Table(Default::default())),
        "agents": agents.into_iter().filter_map(|(agent_name, agent_cfg)| {
            let agent_cfg = agent_cfg.as_table()?.clone();
            Some(json!({
                "agent": agent_name,
                "task_type": agent_cfg.get("task_type").cloned().unwrap_or(toml::Value::String(String::new())),
                "model_profile": agent_cfg.get("model_profile").cloned().unwrap_or(toml::Value::String(String::new())),
                "provider": agent_cfg.get("provider").cloned().unwrap_or(toml::Value::String(String::new())),
                "inference_origin": agent_cfg.get("inference_origin").cloned().unwrap_or(toml::Value::String(String::new())),
            }))
        }).collect::<Vec<_>>(),
        "lanes": lanes,
        "recommended_productization_sequence": [
            "stabilize route mappings for existing OpenCode agents",
            "promote high-value route recipes into APOLLO or crate-spawn workflow surfaces",
            "bind embodiment/client-shell usage through ARDA or other consumers without making them authoritative",
            "keep optional skills/plugins behind bounded package and route contracts",
        ],
        "summary": {
            "agents_total": agents_total,
            "lanes_total": 3,
            "route_contract_ready": runtime_surface.get("route_contract_ready").and_then(Value::as_bool).unwrap_or(false),
            "runtime_ready": runtime_surface.get("status").and_then(Value::as_str) == Some("ready"),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_playwright_mcp_productization_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/playwright_mcp_productization_contract.json");
    let enablement = read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let adapter_contract = read_json_or(
        &root.join("core/state/communication_adapter_contract.json"),
        json!({}),
    );
    let backlog = read_json_or(
        &root.join("core/state/extension_activation_backlog.json"),
        json!({}),
    );

    let package_row = enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("playwright-mcp"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_surface = runtime
        .get("surfaces")
        .and_then(|v| v.get("playwright_mcp"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let adapter_row = adapter_contract
        .get("adapters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("playwright-mcp"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let backlog_row = backlog
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("playwright-mcp"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.playwright-mcp-productization-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_runtime_activation + communication_adapter_contract + extension_activation_backlog",
        "tool": {
            "name": "playwright-mcp",
            "repo": package_row.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": package_row.get("repo_url").cloned().unwrap_or(Value::Null),
            "activation_status": package_row.get("activation_status").cloned().unwrap_or(Value::Null),
            "integration_state": package_row.get("integration_state").cloned().unwrap_or(Value::Null),
            "runtime_status": runtime_surface.get("status").cloned().unwrap_or(Value::Null),
            "command": runtime_surface.get("command").cloned().unwrap_or(Value::Null),
            "runtime_mode": runtime_surface.get("runtime_mode").cloned().unwrap_or(Value::Null),
        },
        "doctrine": {
            "ephemeral_stdio_only": true,
            "approval_required_for_browser_session": runtime_surface.get("approval_required").and_then(Value::as_bool).unwrap_or(false),
            "network_requires_explicit_allow": runtime_surface.get("network_requires_explicit_allow").and_then(Value::as_bool).unwrap_or(false),
            "governed_browser_capability_not_background_daemon": true,
            "ui_optional": true,
            "agent_first_contract": true,
        },
        "bridge_contract": {
            "crate_surface": "crates/arda-mcp/src/browser.rs",
            "launcher": "scripts/runtime/playwright_mcp_bridge.sh",
            "profile_dir": runtime_surface.get("profile_dir").cloned().unwrap_or(Value::Null),
            "artifact_dir": runtime_surface.get("artifact_dir").cloned().unwrap_or(Value::Null),
            "log_path": runtime_surface.get("log_path").cloned().unwrap_or(Value::Null),
            "env_contract": package_row.get("runtime_env_contract").cloned().unwrap_or_else(|| json!([])),
        },
        "productization_lanes": [
            {
                "lane": "mcp_browser_session",
                "owner": "hermes_charon",
                "purpose": "Expose browser navigation/page interaction as a governed on-demand MCP session tool.",
                "runtime_status": adapter_row.get("runtime_status").cloned().unwrap_or_else(|| runtime_surface.get("status").cloned().unwrap_or(Value::Null)),
                "next_action": adapter_row.get("next_action").cloned().unwrap_or_else(|| package_row.get("next_action").cloned().unwrap_or(Value::Null)),
                "write_through": [
                    "scripts/runtime/playwright_mcp_bridge.sh",
                    "core/state/package_runtime_activation.json",
                    "core/state/communication_adapter_contract.json",
                ],
            },
            {
                "lane": "workflow_execution",
                "owner": "apollo",
                "purpose": "Use browser sessions as explicit workflow steps rather than ambient daemon capability.",
                "runtime_status": backlog_row.get("runtime_status").cloned().unwrap_or_else(|| json!("contract_ready")),
                "next_action": "bind approved browser sessions into APOLLO recipes and explicit task flows",
                "write_through": [
                    "core/state/extension_surface_contract.json",
                    "core/state/playwright_mcp_productization_contract.json",
                    "core/state/package_runtime_activation.json",
                ],
            },
            {
                "lane": "ui_agent_consumers",
                "owner": "prometheus_arda",
                "purpose": "Let ARDA or other agents consume the same browser-session contract without becoming the source of authority.",
                "runtime_status": "contract_ready",
                "next_action": "ingest the same governed browser contract through system snapshot/source-map consumers",
                "write_through": [
                    "core/state/system_snapshot.json",
                    "core/state/system_source_map.json",
                    "core/state/playwright_mcp_productization_contract.json",
                ],
            },
        ],
        "recommended_activation_sequence": [
            "keep Playwright MCP on-demand and approval-gated",
            "use the supervised bridge only for explicit governed browser sessions",
            "promote recurring browser steps into APOLLO workflow recipes instead of long-lived background service",
            "expose the same contract to UI and agent consumers through sovereign state exports",
        ],
        "summary": {
            "runtime_ready": runtime_surface.get("status").and_then(Value::as_str) == Some("contract_ready"),
            "approval_required": runtime_surface.get("approval_required").and_then(Value::as_bool).unwrap_or(false),
            "lanes_total": 3,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_nanoclaw_productization_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/nanoclaw_productization_contract.json");
    let enablement = read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let backlog = read_json_or(
        &root.join("core/state/extension_activation_backlog.json"),
        json!({}),
    );
    let governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );

    let package_row = enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("nanoclaw"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_surface = runtime
        .get("surfaces")
        .and_then(|v| v.get("nanoclaw"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let backlog_row = backlog
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("nanoclaw"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let fleet_nodes = governor
        .get("capability_lanes")
        .and_then(|v| v.get("fleet_uptime_and_downtime"))
        .and_then(|v| v.get("nodes"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edge_target_id = runtime_surface
        .get("edge_target")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let edge_target = fleet_nodes
        .into_iter()
        .find(|row| row.get("target_id").and_then(Value::as_str) == Some(edge_target_id))
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.nanoclaw-productization-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_runtime_activation + package_enablement + runtime_governor_contract",
        "tool": {
            "name": "nanoclaw",
            "repo": package_row.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": package_row.get("repo_url").cloned().unwrap_or(Value::Null),
            "binary_path": package_row.get("binary_path").cloned().unwrap_or(Value::Null),
            "activation_status": package_row.get("activation_status").cloned().unwrap_or(Value::Null),
            "integration_state": package_row.get("integration_state").cloned().unwrap_or(Value::Null),
            "runtime_status": runtime_surface.get("status").cloned().unwrap_or(Value::Null),
            "control_mode": runtime_surface.get("control_mode").cloned().unwrap_or(Value::Null),
            "edge_transport": runtime_surface.get("edge_transport").cloned().unwrap_or(Value::Null),
            "edge_target": runtime_surface.get("edge_target").cloned().unwrap_or(Value::Null),
        },
        "doctrine": {
            "headless_and_tailscale_mode_is_primary": true,
            "whatsapp_auth_is_optional_not_global_gate": true,
            "edge_target_visibility_controls_promotion": true,
            "agent_first_contract": true,
            "ui_optional": true,
        },
        "runtime_contract": {
            "launcher": "scripts/runtime/nanoclaw_runtime.sh",
            "project_root": runtime_surface.get("project_root").cloned().unwrap_or(Value::Null),
            "entrypoint": runtime_surface.get("entrypoint").cloned().unwrap_or(Value::Null),
            "pid_file": runtime_surface.get("pid_file").cloned().unwrap_or(Value::Null),
            "log_path": runtime_surface.get("log_path").cloned().unwrap_or(Value::Null),
            "error_log_path": runtime_surface.get("error_log_path").cloned().unwrap_or(Value::Null),
            "env_contract": package_row.get("runtime_env_contract").cloned().unwrap_or_else(|| json!([])),
        },
        "edge_dependency": {
            "target_id": if edge_target_id.is_empty() { Value::Null } else { json!(edge_target_id) },
            "target_visible": runtime_surface.get("edge_target_visible").cloned().unwrap_or(Value::Null),
            "tailscale_ready": runtime_surface.get("tailscale_ready").cloned().unwrap_or(Value::Null),
            "control_mode": runtime_surface.get("control_mode").cloned().unwrap_or(Value::Null),
            "transport": runtime_surface.get("edge_transport").cloned().unwrap_or(Value::Null),
            "fleet_target": edge_target,
        },
        "productization_lanes": [
            {
                "lane": "edge_runtime",
                "owner": "charon_warden",
                "purpose": "Promote NanoClaw as a bounded edge runtime tied to a visible Tailnet target rather than unmanaged local auth state.",
                "runtime_status": runtime_surface.get("status").cloned().unwrap_or(Value::Null),
                "next_action": package_row.get("next_action").cloned().unwrap_or(Value::Null),
                "write_through": ["scripts/runtime/nanoclaw_runtime.sh", "core/state/package_runtime_activation.json", "core/edge/targets.toml"],
            },
            {
                "lane": "headless_message_control",
                "owner": "hermes",
                "purpose": "Keep message/control flows in headless or Tailscale mode without reintroducing WhatsApp as a global readiness gate.",
                "runtime_status": runtime_surface.get("status").cloned().unwrap_or(Value::Null),
                "next_action": "preserve headless control doctrine and promote only after the configured edge target is visible",
                "write_through": ["config/federated_comms.toml", "core/state/federated_comms_runtime.json", "core/state/nanoclaw_productization_contract.json"],
            },
            {
                "lane": "remediation_and_promotion",
                "owner": "prometheus",
                "purpose": "Turn runtime posture into explicit remediation/promotion criteria for operator, steward, and UI consumers.",
                "runtime_status": backlog_row.get("runtime_status").cloned().unwrap_or_else(|| runtime_surface.get("status").cloned().unwrap_or(Value::Null)),
                "next_action": "promote from contract-ready to actively governed runtime when the configured edge target is visible and the runtime is started intentionally",
                "write_through": ["core/state/package_enablement.json", "core/state/fleet_steward_actions.json", "core/state/nanoclaw_productization_contract.json"],
            },
        ],
        "recommended_promotion_sequence": [
            "preserve headless or tailscale mode as the primary doctrine",
            "verify the configured edge target is visible on the mesh",
            "start NanoClaw intentionally through the governed launcher",
            "promote runtime posture into steward and routing consumers only after live runtime confirmation",
        ],
        "summary": {
            "runtime_ready": runtime_surface.get("runtime_ready").and_then(Value::as_bool).unwrap_or(false),
            "edge_target_visible": runtime_surface.get("edge_target_visible").and_then(Value::as_bool).unwrap_or(false),
            "tailscale_ready": runtime_surface.get("tailscale_ready").and_then(Value::as_bool).unwrap_or(false),
            "lanes_total": 3,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}
