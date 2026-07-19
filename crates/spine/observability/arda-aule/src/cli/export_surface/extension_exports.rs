#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::*;

pub(crate) fn export_openfang_alignment_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/openfang_alignment.json");
    let payload = json!({
        "schema_version": "arda.openfang.alignment.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_policy_ready_source_src_cebb6abe",
        "source_id": "src_cebb6abe",
        "source_url": "https://github.com/RightNow-AI/openfang",
        "posture": {
            "aligned_domains": [
                "crate_native_system_decomposition",
                "network_native_onboarding",
                "agent_capability_packaging",
                "desktop_embodiment",
                "security_by_default_runtime",
            ],
            "misaligned_domains": [
                "whatsapp_first_channel_assumption",
                "marketplace_in_hot_path",
                "single_distribution_surface_bias",
            ],
        },
        "pattern_extraction": {
            "hands_to_spawnable_crates": {
                "openfang_pattern": "autonomous Hands as prebuilt capability packages with lifecycle verbs",
                "arda_adaptation": "crate-spawn templates with sovereign task, metrics, ARDA, and control-plane hooks from first boot",
                "priority": "high",
            },
            "wire_to_network_onboarding": {
                "openfang_pattern": "network-native OFP onboarding and mutual auth",
                "arda_adaptation": "keep Unix sockets local, Tailscale internal, and fold network-native node onboarding above the sovereign local layer",
                "priority": "high",
            },
            "skills_marketplace_boundary": {
                "openfang_pattern": "skills and marketplace framing around reusable packages",
                "arda_adaptation": "align with `.aipkg` as open package law and keep marketplace economics out of core truth",
                "priority": "high",
            },
            "desktop_embodiment": {
                "openfang_pattern": "Tauri desktop as a first-class native surface",
                "arda_adaptation": "carry forward into ARDA embodied interface and Pepper's Ghost controller surfaces",
                "priority": "medium",
            },
            "security_stack": {
                "openfang_pattern": "explicit signing, taint tracking, sandboxing, and audit trail",
                "arda_adaptation": "map to WARDEN, HADES, `.aipkg` receipts/signatures, and control-plane lockdown projections",
                "priority": "high",
            },
        },
        "arda_follow_ons": [
            "crate_spawn_blueprint_contract",
            "network_native_node_onboarding_contract",
            "aipkg_marketplace_separation",
            "arda_embodied_tauri_slice",
        ],
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

pub(crate) fn export_agentforge_alignment_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/agentforge_alignment.json");
    let payload = json!({
        "schema_version": "arda.agentforge.alignment.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_source_src_75bbe2e4 + official_readme_extraction",
        "source_id": "src_75bbe2e4",
        "source_url": "https://github.com/DataBassGit/AgentForge",
        "official_traits": {
            "core_model": "low_code_framework_for_agents_and_cognitive_architectures",
            "workflow_primitive": "declarative_cogs",
            "memory_model": "integrated_memory_nodes",
            "persona_model": "yaml_personas",
            "llm_posture": "model_agnostic",
            "storage_posture": "database_flexible_with_chromadb_reference",
        },
        "arda_adaptation": {
            "adopt": [
                "forge_style_creation_flow",
                "declarative_workflow_templates",
                "memory_node_contracts_for_spawned_systems",
            ],
            "adapt": [
                "yaml_personas_into_operator_product_profiles",
                "low_code_templates_into_crate_spawn_blueprints",
            ],
            "reject": [
                "framework_centrality_over_sovereign_core",
                "single_memory_backend_assumption",
            ],
        },
        "implementation_mappings": {
            "crate_boundaries": [
                {
                    "target": "crate_spawn_blueprint_flow",
                    "surfaces": [
                        "core/state/crate_spawn_contract.json",
                        "arda-cli utility create-crate-spawn-blueprint",
                    ],
                    "why": "AgentForge's forge metaphor maps cleanly to sovereign crate scaffolding rather than a permanent meta-framework.",
                },
                {
                    "target": "workflow_execution_contracts",
                    "surfaces": [
                        "core/state/operations_flow.json",
                        "crates/arda-apollo",
                    ],
                    "why": "Declarative cog-style flows should become bounded execution recipes owned by APOLLO.",
                },
                {
                    "target": "memory_node_topology",
                    "surfaces": [
                        "core/state/athena_runtime.json",
                        "core/state/mnemosyne_continuity.json",
                    ],
                    "why": "Integrated memory nodes belong in sovereign knowledge and continuity surfaces, not inside an external framework shell.",
                },
            ],
            "productization_sequence": [
                "freeze reusable forge metadata in crate_spawn_contract",
                "promote selected workflow recipes into productizable crate templates",
                "keep personas and memory policies as optional overlays instead of base doctrine",
            ],
            "stateful_boundaries": {
                "core_contracts": [
                    "core/state/crate_spawn_contract.json",
                    "core/state/agent_framework_alignment.json",
                ],
                "extension_lane": [
                    "core/state/package_enablement.json",
                    "core/state/package_runtime_activation.json",
                ],
            },
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_eliza_alignment_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/eliza_alignment.json");
    let payload = json!({
        "schema_version": "arda.eliza.alignment.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_source_src_4169b219 + official_readme_extraction",
        "source_id": "src_4169b219",
        "source_url": "https://github.com/elizaOS/eliza",
        "official_traits": {
            "core_model": "open_source_multi_agent_platform",
            "connectivity": ["discord", "telegram", "farcaster", "more"],
            "runtime_posture": "extensible_platform_with_cli_and_web_ui",
            "model_posture": "model_agnostic",
            "plugin_ecosystem": "large_plugin_registry",
        },
        "arda_adaptation": {
            "adopt": [
                "plugin_boundary_patterns",
                "connector_surfaces_as_optional_adapters",
                "client_surface_packaging_lessons",
            ],
            "adapt": [
                "persona_shells_into_productizable_client_experiences",
                "plugin_registry_patterns_into_aipkg_marketplace_extensions",
            ],
            "reject": [
                "identity_persona_as_core_doctrine",
                "chat_platform_connectors_as_sovereign_base_layer",
            ],
        },
        "implementation_mappings": {
            "crate_boundaries": [
                {
                    "target": "communication_adapter_plugins",
                    "surfaces": [
                        "core/state/federated_comms_runtime.json",
                        "core/state/matrix_boardrooms.json",
                        "crates/arda-hermes",
                    ],
                    "why": "eliza's connector ecosystem is useful when constrained to HERMES-owned adapter boundaries.",
                },
                {
                    "target": "packageable_runtime_extensions",
                    "surfaces": [
                        "core/state/aipkg_contract.json",
                        "core/state/package_enablement.json",
                    ],
                    "why": "Plugin registry lessons should become optional package extensions rather than implicit core dependencies.",
                },
                {
                    "target": "client_shells_and_embodiment",
                    "surfaces": [
                        "core/state/embodied_interface.json",
                        "core/state/tauri_embodiment.json",
                        "apps/arda-hud",
                    ],
                    "why": "Client-facing agent shells should package into product experiences without redefining sovereign identity.",
                },
            ],
            "runtime_partitioning": {
                "core_stays_small": true,
                "connectors_optional": true,
                "persona_layers_productized": true,
                "registry_hot_path_forbidden": true,
            },
            "delivery_sequence": [
                "stabilize Matrix/Element adapter contract under HERMES",
                "model connector plugins as optional package/runtime extensions",
                "carry character-shell lessons into product UI and embodiment surfaces without moving doctrine out of core",
            ],
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_agent_framework_alignment_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/agent_framework_alignment.json");
    let payload = json!({
        "schema_version": "arda.agent-framework-alignment.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_comparative_framework_pass",
        "frameworks": [
            {
                "name": "OpenFang",
                "source_id": "src_cebb6abe",
                "role": "capability-packaging and network-native onboarding reference",
                "adopt": [
                    "crate_spawn_blueprints",
                    "security_by_default_runtime",
                ],
                "adapt": [
                    "desktop_embodiment",
                    "packaged agent lifecycle verbs",
                ],
                "reject": [
                    "marketplace_in_hot_path",
                    "single_distribution_surface_bias",
                ],
            },
            {
                "name": "AgentForge",
                "source_id": "src_75bbe2e4",
                "role": "agent workflow topology and build/forge pattern reference",
                "adopt": [
                    "forge_style_creation_flow",
                    "composable_agent_pipeline_patterns",
                ],
                "adapt": [
                    "workflow_templates_into_productizable_crates",
                ],
                "reject": [
                    "framework_centrality_over_sovereign_core",
                ],
            },
            {
                "name": "eliza",
                "source_id": "src_4169b219",
                "role": "plugin/runtime and agent persona system reference",
                "adopt": [
                    "plugin_boundary_patterns",
                    "character_runtime_packaging_lessons",
                ],
                "adapt": [
                    "communication_surface_plugins",
                    "client_facing_agent_shells",
                ],
                "reject": [
                    "identity_persona_as_core_doctrine",
                ],
            },
        ],
        "governance_validators": {
            "triad_required": true,
            "bacon_lite_required": true,
            "joulework_required": true,
            "love_equation_required": true,
        },
        "implementation_targets": {
            "federated_boardrooms": [
                "core/state/federated_comms_runtime.json",
                "core/state/matrix_boardrooms.json",
                "core/state/hermes_command.json",
            ],
            "crate_and_package_lifecycle": [
                "core/state/crate_spawn_contract.json",
                "core/state/aipkg_contract.json",
                "core/state/package_runtime_activation.json",
            ],
            "product_shells_and_embodiment": [
                "core/state/embodied_interface.json",
                "core/state/tauri_embodiment.json",
                "apps/arda-hud",
            ],
        },
        "decision_summary": {
            "core_should_absorb": [
                "spawn_blueprint_contracts",
                "security_by_default_runtime_baselines",
                "strict_adapter_boundary_contracts",
            ],
            "extensions_should_absorb": [
                "connector_plugins",
                "client_persona_shells",
                "workflow_starter_templates",
            ],
            "must_remain_rejected": [
                "framework-first identity",
                "marketplace or registry hot-path dependence",
                "chat-platform assumptions as sovereign base layer",
            ],
        },
        "linked_runtime_exports": [
            "core/state/openfang_alignment.json",
            "core/state/agentforge_alignment.json",
            "core/state/eliza_alignment.json",
        ],
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_extension_surface_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/extension_surface_contract.json");
    let registry = read_toml_or(
        &root.join("docs/registry.toml"),
        toml::Value::Table(Default::default()),
    );
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let aipkg = read_json_or(&root.join("core/state/aipkg_contract.json"), json!({}));
    let framework = read_json_or(
        &root.join("core/state/agent_framework_alignment.json"),
        json!({}),
    );
    let agentforge = read_json_or(
        &root.join("core/state/agentforge_alignment.json"),
        json!({}),
    );
    let eliza = read_json_or(&root.join("core/state/eliza_alignment.json"), json!({}));
    let tools = tool_map(&package_enablement);

    let mut communication_adapters = Vec::new();
    let mut workflow_templates = Vec::new();
    let mut client_shells = Vec::new();
    let mut skills_and_plugins = Vec::new();

    let registry_tools = registry
        .get("tools")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();
    for (tool, meta) in registry_tools {
        let Some(meta) = meta.as_table() else {
            continue;
        };
        let row = pick_fields(&tool, meta, tools.get(&tool));
        let category = meta
            .get("category")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        let tool_type = meta
            .get("type")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();

        if matches!(category, "communication" | "browser") || tool_type == "mcp-server" {
            communication_adapters.push(row.clone());
        }
        if matches!(category, "agent-framework" | "agent-skills") || tool_type == "plugin" {
            skills_and_plugins.push(row.clone());
        }
        if category == "agent-framework" {
            workflow_templates.push(row.clone());
        }
        if matches!(category, "agent-framework" | "communication") {
            client_shells.push(row);
        }
    }

    let required_validators = aipkg
        .get("validator_harnesses")
        .and_then(Value::as_object)
        .map(|rows| rows.keys().cloned().map(Value::String).collect::<Vec<_>>())
        .unwrap_or_default();

    let payload = json!({
        "schema_version": "arda.extension-surface-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "framework_digestion_materialization",
        "doctrine": {
            "core_must_stay_small": true,
            "extensions_optional_by_default": true,
            "registry_hot_path_forbidden": true,
            "connector_plugins_live_under_hermes_or_package_boundaries": true,
            "workflow_templates_promote_into_crate_spawn_or_apollo_recipes": true,
            "client_persona_shells_are_product_layers_not_core_identity": true,
        },
        "governance_contract": {
            "runtime_profiles": aipkg.get("profiles").cloned().unwrap_or_else(|| json!([])),
            "extensions": aipkg.get("extensions").cloned().unwrap_or_else(|| json!([])),
            "required_validators": required_validators,
            "arda_mapping": aipkg.get("arda_mapping").cloned().unwrap_or_else(|| json!({})),
        },
        "framework_sources": {
            "agentforge": {
                "source_id": agentforge.get("source_id").cloned().unwrap_or(Value::Null),
                "targets": agentforge
                    .get("implementation_mappings")
                    .and_then(|v| v.get("crate_boundaries"))
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "productization_sequence": agentforge
                    .get("implementation_mappings")
                    .and_then(|v| v.get("productization_sequence"))
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            },
            "eliza": {
                "source_id": eliza.get("source_id").cloned().unwrap_or(Value::Null),
                "targets": eliza
                    .get("implementation_mappings")
                    .and_then(|v| v.get("crate_boundaries"))
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "delivery_sequence": eliza
                    .get("implementation_mappings")
                    .and_then(|v| v.get("delivery_sequence"))
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            },
            "framework_decision_summary": framework
                .get("decision_summary")
                .cloned()
                .unwrap_or_else(|| json!({})),
        },
        "extension_lanes": {
            "communication_adapters": {
                "owner": "hermes",
                "contract_surfaces": [
                    "core/state/federated_comms_runtime.json",
                    "core/state/matrix_boardrooms.json",
                    "core/state/package_runtime_activation.json",
                ],
                "tools": communication_adapters,
            },
            "workflow_templates": {
                "owner": "apollo_prometheus",
                "contract_surfaces": [
                    "core/state/crate_spawn_contract.json",
                    "core/state/operations_flow.json",
                    "core/state/agentforge_alignment.json",
                ],
                "tools": workflow_templates,
            },
            "client_shells_and_embodiment": {
                "owner": "arda",
                "contract_surfaces": [
                    "core/state/embodied_interface.json",
                    "core/state/tauri_embodiment.json",
                    "core/state/eliza_alignment.json",
                ],
                "tools": client_shells,
            },
            "skills_plugins_and_optional_extensions": {
                "owner": "prometheus",
                "contract_surfaces": [
                    "core/state/aipkg_contract.json",
                    "core/state/package_enablement.json",
                    "core/state/package_runtime_activation.json",
                ],
                "tools": skills_and_plugins,
            },
        },
        "integration_rules": [
            "Every extension must resolve to sovereign state exports before human-facing packaging.",
            "Connector or plugin registries may assist discovery but cannot become runtime law.",
            "Extensions must stay callable without requiring ARDA HUD presence.",
            "Package and plugin activation must remain bounded by operator actions and package/runtime contracts.",
        ],
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_extension_activation_backlog_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/extension_activation_backlog.json");
    let extension_contract = read_json_or(
        &root.join("core/state/extension_surface_contract.json"),
        json!({}),
    );
    let github = github_by_tool(&read_json_or(
        &root.join("core/state/github_repo_integration.json"),
        json!({}),
    ));
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let human_needed = operator_actions
        .get("summary")
        .and_then(|v| v.get("human_needed_total"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;

    let mut backlog = Vec::new();
    let lanes = extension_contract
        .get("extension_lanes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (lane, payload) in lanes {
        let Some(payload) = payload.as_object() else {
            continue;
        };
        let owner = payload
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let tools = payload
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for tool in tools {
            let Some(tool_obj) = tool.as_object() else {
                continue;
            };
            let activation = tool_obj
                .get("activation_status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if activation == "active_in_system" {
                continue;
            }
            let tool_name = tool_obj
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or_default();
            backlog.push(make_backlog_entry(
                &lane,
                owner,
                &tool,
                github.get(tool_name).unwrap_or(&Value::Null),
                human_needed,
            ));
        }
    }

    let payload = json!({
        "schema_version": "arda.extension-activation-backlog.v1",
        "generated_at_utc": now_utc(),
        "authority": "extension_activation_backlog_export",
        "summary": {
            "backlog_total": backlog.len(),
            "research_to_contract_total": backlog.iter().filter(|row| row.get("phase").and_then(Value::as_str) == Some("research_to_contract")).count(),
            "contract_ready_total": backlog.iter().filter(|row| row.get("phase").and_then(Value::as_str) == Some("contract_ready")).count(),
            "tune_and_productize_total": backlog.iter().filter(|row| row.get("phase").and_then(Value::as_str) == Some("tune_and_productize")).count(),
        },
        "entries": backlog,
        "rules": [
            "Extension backlog must resolve through sovereign contracts before runtime activation.",
            "Registry or plugin ecosystems may inform activation order but cannot bypass operator or governance gates.",
            "HUD presence is not a prerequisite for extension activation planning.",
        ],
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn tool_map(enablement: &Value) -> BTreeMap<String, Value> {
    enablement
        .get("tools")
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

fn github_by_tool(data: &Value) -> BTreeMap<String, Value> {
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

fn pick_fields(
    tool: &str,
    meta: &toml::map::Map<String, toml::Value>,
    enablement: Option<&Value>,
) -> Value {
    let repo = meta.get("repo").and_then(toml::Value::as_str);
    let repo_url = enablement
        .and_then(|row| row.get("repo_url"))
        .cloned()
        .or_else(|| repo.map(|repo| Value::String(format!("https://github.com/{repo}"))))
        .unwrap_or(Value::Null);

    json!({
        "tool": tool,
        "repo": json_from_toml(meta.get("repo")),
        "repo_url": repo_url,
        "type": json_from_toml(meta.get("type")),
        "category": json_from_toml(meta.get("category")),
        "activation_status": enablement.and_then(|row| row.get("activation_status")).cloned().unwrap_or(Value::Null),
        "integration_lane": enablement.and_then(|row| row.get("integration_lane")).cloned().unwrap_or(Value::Null),
        "next_action": enablement.and_then(|row| row.get("next_action")).cloned().unwrap_or(Value::Null),
    })
}

fn make_backlog_entry(
    lane: &str,
    owner: &str,
    tool: &Value,
    github_row: &Value,
    human_needed: bool,
) -> Value {
    let package = github_row
        .get("package_enablement")
        .and_then(Value::as_object)
        .cloned()
        .map(Value::Object)
        .unwrap_or_else(|| json!({}));
    let disposition = github_row
        .get("disposition")
        .and_then(Value::as_object)
        .cloned()
        .map(Value::Object)
        .unwrap_or_else(|| json!({}));
    let runtime = github_row
        .get("runtime")
        .and_then(Value::as_object)
        .cloned()
        .map(Value::Object)
        .unwrap_or_else(|| json!({}));
    let activation = tool
        .get("activation_status")
        .and_then(Value::as_str)
        .or_else(|| package.get("activation_status").and_then(Value::as_str))
        .unwrap_or("planned");
    let phase = match activation {
        "planned" => "research_to_contract",
        "governed_on_demand" => "contract_ready",
        "active_signal" => "tune_and_productize",
        _ => "active",
    };

    json!({
        "lane": lane,
        "owner": owner,
        "tool": tool.get("tool").cloned().unwrap_or(Value::Null),
        "repo_url": tool.get("repo_url").cloned().unwrap_or(Value::Null),
        "activation_status": activation,
        "phase": phase,
        "disposition_mode": disposition.get("mode").cloned().unwrap_or(Value::Null),
        "runtime_status": runtime.get("status").cloned().unwrap_or(Value::Null),
        "next_action": tool
            .get("next_action")
            .cloned()
            .or_else(|| package.get("next_action").cloned())
            .unwrap_or(Value::Null),
        "human_needed": human_needed,
        "system_surfaces": github_row.get("system_surfaces").cloned().unwrap_or_else(|| json!([])),
    })
}

fn json_from_toml(value: Option<&toml::Value>) -> Value {
    value
        .and_then(|entry| serde_json::to_value(entry).ok())
        .unwrap_or(Value::Null)
}
