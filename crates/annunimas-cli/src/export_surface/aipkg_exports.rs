use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::*;

pub(crate) fn export_aipkg_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/aipkg_contract.json");
    let manifest = read_json_or(
        &root.join("spec/aipkg/v0.1/manifest.example.json"),
        json!({}),
    );

    let payload = json!({
        "schema_version": "annunimas.aipkg.contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "aipkg_core_v0_freeze",
        "open_standard_split": {
            "core_law_open": true,
            "marketplace_economics_separate": true,
            "registry_required": false,
            "single_payment_rail_required": false,
        },
        "manifest_example": manifest,
        "normative_rules": {
            "zero_work_preflight_required": true,
            "receipt_signatures_required": true,
            "package_digest_required": true,
            "deny_by_default_runtime": true,
            "canonical_checksums_inventory_required": true,
            "triad_gate_required": true,
            "bacon_lite_required": true,
            "joulework_budget_required": true,
            "love_eq_guard_required": true,
            "soterion_trace_required": true,
        },
        "validator_harnesses": {
            "triad": {
                "required": true,
                "purpose": "logic, strategy, and evidence gate before install or execution",
            },
            "bacon_lite": {
                "required": true,
                "purpose": "lightweight empirical validator and confidence harness",
            },
            "joulework": {
                "required": true,
                "purpose": "budget and energy validator before work begins",
            },
            "love_equation": {
                "required": true,
                "purpose": "human and relational guard for packages that touch users or bonds",
            },
        },
        "profiles": ["wasm-wasi", "oci-sandboxed", "local-sovereign"],
        "extensions": ["crusader_security", "beacon_reputation", "marketplace_discovery"],
        "receipt_chain": [
            "preflight_receipt",
            "execution_receipt",
            "validation_receipt",
            "settlement_receipt",
            "denial_receipt",
        ],
        "annunimas_mapping": {
            "triad_gate": "core/state/governance_runtime.json",
            "joulework": "data/hades/joulework.jsonl",
            "love_eq": "core/metrics/by_crate/governance/signals.json",
            "soterion": "data/soterion_index.json",
            "operator_actions": "core/state/operator_actions.json",
        },
        "spec_paths": {
            "manifest_example": "spec/aipkg/v0.1/manifest.example.json",
            "execution_request_schema": "spec/aipkg/v0.1/execution-request.schema.json",
            "receipt_schema": "spec/aipkg/v0.1/receipt.schema.json",
            "container_law": "spec/aipkg/v0.1/AIPKG-CONTAINER-v0.1.md",
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_aipkg_edge_lab_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/aipkg_edge_lab_contract.json");
    let fleet = read_toml_or(
        &root.join("config/fleet.toml"),
        toml::Value::Table(Default::default()),
    );
    let providers = read_toml_or(
        &root.join("config/charon.providers.toml"),
        toml::Value::Table(Default::default()),
    );
    let aipkg = read_json_or(&root.join("core/state/aipkg_contract.json"), json!({}));
    let example = read_json_or(
        &root.join("spec/aipkg/v0.1/examples/edge-backbone-lab.manifest.json"),
        json!({}),
    );

    let backbone = fleet
        .get("nodes")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(toml::Value::as_str) == Some("node-backbone-server-01"))
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let edge_backbone = providers
        .get("provider")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(toml::Value::as_str) == Some("edge_backbone"))
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let openrouter = providers
        .get("provider")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(toml::Value::as_str) == Some("openrouter"))
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));

    let tailscale_url = edge_backbone
        .get("base_url")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_string();
    let tailscale_ready = tailscale_url.starts_with("http://100.");
    let openrouter_models = openrouter
        .get("model")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            model
                .get("id")
                .and_then(toml::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "annunimas.aipkg-edge-lab-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "aipkg_edge_lab_export",
        "purpose": {
            "primary_goal": "Use the backbone server as the first off-site `.aipkg` proving ground.",
            "transport_model": "tailscale_internal_mesh",
            "why_this_node": "The backbone server already anchors CHARON edge reasoning over its Tailscale address.",
        },
        "edge_target": {
            "target_id": backbone.get("id").and_then(toml::Value::as_str),
            "display_name": backbone.get("display_name").and_then(toml::Value::as_str),
            "tailscale_ip": backbone.get("tailscale_ip").and_then(toml::Value::as_str),
            "deployment_phase": backbone.get("deployment_phase").and_then(toml::Value::as_str),
            "placement_plan": backbone.get("placement_plan").and_then(toml::Value::as_str),
            "charon_role": backbone.get("charon_role").and_then(toml::Value::as_str),
            "oracle_role": backbone.get("oracle_role").and_then(toml::Value::as_str),
            "athena_role": backbone.get("athena_role").and_then(toml::Value::as_str),
            "remote_use_ready": tailscale_ready
                && backbone.get("tailscale_ip").and_then(toml::Value::as_str).is_some(),
        },
        "runtime_contract": {
            "primary_provider_id": edge_backbone.get("id").and_then(toml::Value::as_str),
            "primary_provider_base_url": tailscale_url,
            "primary_transport": "tailscale",
            "tailscale_required": true,
            "internet_required_when_offsite": true,
            "charon_reads_through_provider_contract": true,
            "openrouter_fallback_provider_id": openrouter.get("id").and_then(toml::Value::as_str),
            "openrouter_fallback_models": openrouter_models,
        },
        "aipkg_law": {
            "runtime_profiles": aipkg.get("profiles").cloned().unwrap_or_else(|| json!([])),
            "governance_required": aipkg.get("normative_rules").cloned().unwrap_or_else(|| json!({})),
            "receipt_chain": aipkg.get("receipt_chain").cloned().unwrap_or_else(|| json!([])),
        },
        "first_lab_package": {
            "manifest_path": "spec/aipkg/v0.1/examples/edge-backbone-lab.manifest.json",
            "package_id": example.get("package_id").cloned().unwrap_or(Value::Null),
            "runtime_profile": example.get("runtime_profile").cloned().unwrap_or(Value::Null),
            "intended_target": "node-backbone-server-01",
            "execution_shape": "preflight_first_then_remote_proving_ground",
        },
        "readiness_checks": [
            {
                "check": "tailscale_mesh_addressable",
                "status": if tailscale_ready { "ready" } else { "pending" },
                "evidence": tailscale_url,
            },
            {
                "check": "edge_backbone_provider_points_to_tailscale",
                "status": if tailscale_ready { "ready" } else { "pending" },
                "evidence": edge_backbone.get("base_url").and_then(toml::Value::as_str),
            },
            {
                "check": "cloud_fallback_available_for_offsite_degradation",
                "status": if openrouter.get("id").and_then(toml::Value::as_str).is_some() { "ready" } else { "pending" },
                "evidence": openrouter.get("id").and_then(toml::Value::as_str),
            },
            {
                "check": "aipkg_manifest_example_bound_to_edge_lab",
                "status": if example.get("package_id").is_some() { "ready" } else { "pending" },
                "evidence": example.get("package_id").cloned().unwrap_or(Value::Null),
            },
        ],
        "operator_message": "Backbone relocation is compatible with Annunimas doctrine as long as the node stays on Tailscale and CHARON keeps the Tailscale provider contract authoritative.",
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_aipkg_marketplace_separation_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/aipkg_marketplace_separation_contract.json");
    let openfang = read_json_or(&root.join("core/state/openfang_alignment.json"), json!({}));
    let aipkg = read_json_or(&root.join("core/state/aipkg_contract.json"), json!({}));
    let package_enablement = package_map(
        &read_json_or(&root.join("core/state/package_enablement.json"), json!({})),
        "tools",
        "tool",
    );
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    )
    .get("surfaces")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
    let extension_surface = read_json_or(
        &root.join("core/state/extension_surface_contract.json"),
        json!({}),
    );
    let extension_backlog = read_json_or(
        &root.join("core/state/extension_activation_backlog.json"),
        json!({}),
    );
    let github = package_map(
        &read_json_or(
            &root.join("core/state/github_repo_integration.json"),
            json!({}),
        ),
        "registry_tools",
        "tool",
    );

    let pattern = openfang
        .get("pattern_extraction")
        .and_then(|v| v.get("skills_marketplace_boundary"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let marketplace_extensions = registry_extension_rows(
        &extension_surface,
        &package_enablement,
        &package_runtime,
        &github,
    );

    let payload = json!({
        "schema_version": "annunimas.aipkg-marketplace-separation-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "aipkg_marketplace_separation_export",
        "source_alignment": {
            "source_id": openfang.get("source_id").cloned().unwrap_or(Value::Null),
            "source_url": openfang.get("source_url").cloned().unwrap_or(Value::Null),
            "follow_on": "aipkg_marketplace_separation",
            "openfang_pattern": pattern.get("openfang_pattern").cloned().unwrap_or(Value::Null),
            "annunimas_adaptation": pattern.get("annunimas_adaptation").cloned().unwrap_or(Value::Null),
        },
        "core_law": {
            "open_standard_split": aipkg.get("open_standard_split").cloned().unwrap_or_else(|| json!({})),
            "normative_rules": aipkg.get("normative_rules").cloned().unwrap_or_else(|| json!({})),
            "profiles": aipkg.get("profiles").cloned().unwrap_or_else(|| json!([])),
            "receipt_chain": aipkg.get("receipt_chain").cloned().unwrap_or_else(|| json!([])),
        },
        "separation_rules": [
            "The `.aipkg` manifest, receipts, validators, and runtime law remain sovereign core contract surfaces.",
            "Marketplace discovery, reputation, settlement, and payment rails remain optional overlays and cannot become install or execution prerequisites.",
            "Registry presence may assist discovery but cannot be required for package execution or validation.",
            "HUD, chat platform, or marketplace clients consume package law; they do not define it.",
        ],
        "marketplace_overlay": {
            "allowed_capabilities": [
                "discovery",
                "optional reputation signals",
                "optional settlement receipts",
                "catalog synchronization",
            ],
            "forbidden_hot_path_dependencies": [
                "registry_required_for_install",
                "single_payment_rail_required",
                "marketplace_identity_as_runtime_authority",
            ],
            "extension_examples": aipkg.get("extensions").cloned().unwrap_or_else(|| json!([])),
        },
        "bounded_extensions": marketplace_extensions,
        "activation_backlog_summary": extension_backlog.get("summary").cloned().unwrap_or_else(|| json!({})),
        "annunimas_mapping": aipkg.get("annunimas_mapping").cloned().unwrap_or_else(|| json!({})),
        "governance": openfang.get("governance_validators").cloned().unwrap_or_else(|| json!({})),
        "summary": {
            "bounded_extensions_total": marketplace_extensions.len(),
            "active_runtime_total": marketplace_extensions.iter().filter(|row| row.get("core_law_position").and_then(Value::as_str) == Some("bounded_runtime")).count(),
            "optional_overlay_total": marketplace_extensions.iter().filter(|row| row.get("marketplace_position").and_then(Value::as_str) == Some("discovery_only_optional")).count(),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_network_native_node_onboarding_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/network_native_node_onboarding_contract.json");
    let openfang = read_json_or(&root.join("core/state/openfang_alignment.json"), json!({}));
    let warden_edge = read_json_or(
        &root.join("core/state/warden_edge_contract.json"),
        json!({}),
    );
    let fleet_runtime = read_json_or(&root.join("core/state/fleet_runtime.json"), json!({}));
    let reconciliation = read_json_or(
        &root.join("core/state/fleet_identity_reconciliation.json"),
        json!({}),
    );
    let edge_plan = read_json_or(
        &root.join("core/state/edge_enrollment_plan.json"),
        json!({}),
    );
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));

    let configured = configured_targets(&fleet_runtime);
    let plans = keyed_rows(&edge_plan, "plans", "target_id");
    let bindings = keyed_rows(&reconciliation, "canonical_binding_candidates", "target_id");
    let actions = operator_actions_by_target(&operator_actions);

    let mut target_contracts = Vec::new();
    for target_id in [
        "node-pi5-citadel-avatar",
        "node-ser9-worker",
        "node-backbone-server-01",
    ] {
        let Some(plan) = plans.get(target_id) else {
            continue;
        };
        let Some(configured_target) = configured.get(target_id) else {
            continue;
        };
        target_contracts.push(build_target_contract(
            plan,
            configured_target,
            bindings.get(target_id),
            actions.get(target_id).cloned().unwrap_or_default(),
        ));
    }

    let pattern = openfang
        .get("pattern_extraction")
        .and_then(|v| v.get("wire_to_network_onboarding"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let payload = json!({
        "schema_version": "annunimas.network-native-node-onboarding-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "network_native_node_onboarding_contract_export",
        "source_alignment": {
            "source_id": openfang.get("source_id").cloned().unwrap_or(Value::Null),
            "source_url": openfang.get("source_url").cloned().unwrap_or(Value::Null),
            "follow_on": "network_native_node_onboarding_contract",
            "openfang_pattern": pattern.get("openfang_pattern").cloned().unwrap_or(Value::Null),
            "annunimas_adaptation": pattern.get("annunimas_adaptation").cloned().unwrap_or(Value::Null),
        },
        "doctrine": {
            "local_socket_authority_retained": true,
            "tailscale_is_internal_mesh_not_external_identity": true,
            "identity_binding_precedes_role_promotion": true,
            "generic_hostnames_are_not_canonical_identity": true,
            "human_operator_confirmation_required_for_stale_identity_cleanup": true,
            "arda_hud_not_required_for_onboarding": true,
        },
        "mesh_posture": {
            "mesh_reachable": warden_edge.get("mesh").and_then(|v| v.get("edge_ready")).cloned().unwrap_or(Value::Null),
            "ack_gap_present": warden_edge.get("mesh").and_then(|v| v.get("ack_gap_present")).cloned().unwrap_or(Value::Null),
            "transport_mode": warden_edge.get("edge_contract").and_then(|v| v.get("mode")).cloned().unwrap_or(Value::Null),
            "local_probe_status": warden_edge.get("mesh").and_then(|v| v.get("local_probe_ok")).cloned().unwrap_or(Value::Null),
        },
        "onboarding_contract": {
            "phases": [
                "bind_or_recover_identity",
                "retire_or_rename_colliding_stale_nodes",
                "assign_live_tailscale_ip_and_ssh_trust",
                "bootstrap_target_runtime",
                "promote_runtime_roles_after_informant_confirmation",
            ],
            "required_gates": [
                "triad_required",
                "bacon_lite_required",
                "joulework_required",
                "love_equation_required",
                "operator_confirmation_for_identity_binding",
            ],
            "runtime_boundaries": [
                "Unix sockets stay local to each node.",
                "Tailscale provides sovereign internal mesh transport above the local runtime layer.",
                "Role promotion requires matched fleet identity and evidence of live informant/runtime observation.",
            ],
        },
        "targets": target_contracts,
        "summary": {
            "tracked_targets_total": target_contracts.len(),
            "identity_binding_pending_total": target_contracts.iter().filter(|target| target.get("onboarding_stage").and_then(Value::as_str) == Some("identity_binding_pending")).count(),
            "operator_action_open_total": target_contracts.iter().filter(|target| target.get("operator_actions").and_then(Value::as_array).map(|v| !v.is_empty()).unwrap_or(false)).count(),
        },
        "governance": openfang.get("governance_validators").cloned().unwrap_or_else(|| json!({})),
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_edge_identity_remediation_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/edge_identity_remediation_contract.json");
    let fleet_recon = read_json_or(
        &root.join("core/state/fleet_identity_reconciliation.json"),
        json!({}),
    );
    let edge_plan = read_json_or(
        &root.join("core/state/edge_enrollment_plan.json"),
        json!({}),
    );
    let network_onboarding = read_json_or(
        &root.join("core/state/network_native_node_onboarding_contract.json"),
        json!({}),
    );
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let warden_edge = read_json_or(
        &root.join("core/state/warden_edge_contract.json"),
        json!({}),
    );

    let plans = keyed_rows(&edge_plan, "plans", "target_id");
    let onboarding_targets = keyed_rows(&network_onboarding, "targets", "target_id");
    let actions = operator_actions_by_target(&operator_actions);

    let mut targets = Vec::new();
    for target_id in ["node-pi5-citadel-avatar", "node-ser9-worker"] {
        let Some(plan) = plans.get(target_id) else {
            continue;
        };
        targets.push(remediation_target(
            target_id,
            plan,
            onboarding_targets.get(target_id),
            actions.get(target_id).cloned().unwrap_or_default(),
        ));
    }

    let stale_clusters = fleet_recon
        .get("stale_hostname_clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let payload = json!({
        "schema_version": "annunimas.edge-identity-remediation-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "edge_identity_remediation_export",
        "identity_doctrine": {
            "generic_hostname_is_not_identity": true,
            "bind_identity_before_bootstrap": true,
            "retire_stale_duplicates_before_promoting_new_role": true,
            "operator_confirmation_required_for_identity_cleanup": true,
            "mesh_transport_does_not_replace_identity_registry": true,
        },
        "mesh_context": {
            "transport_mode": warden_edge.get("edge_contract").and_then(|v| v.get("mode")).cloned().unwrap_or(Value::Null),
            "ack_gap_present": warden_edge.get("mesh").and_then(|v| v.get("ack_gap_present")).cloned().unwrap_or(Value::Null),
            "stale_hostname_clusters": stale_clusters.clone(),
            "unresolved_stale_total": fleet_recon.get("summary").and_then(|v| v.get("unresolved_stale_total")).cloned().unwrap_or(Value::Null),
        },
        "target_remediation": targets,
        "shared_actions": stale_clusters
            .into_iter()
            .filter(|cluster| cluster.is_object())
            .map(|cluster| json!({
                "hostname": cluster.get("hostname").cloned().unwrap_or(Value::Null),
                "tailscale_node_ids": cluster.get("tailscale_node_ids").cloned().unwrap_or_else(|| json!([])),
                "action": "retire_stale_duplicates",
            }))
            .collect::<Vec<_>>(),
        "summary": {
            "target_remediation_total": targets.len(),
            "operator_confirmation_targets_total": targets.iter().filter(|target| target.get("operator_confirmation_required").and_then(Value::as_bool).unwrap_or(false)).count(),
            "stale_hostname_clusters_total": fleet_recon.get("stale_hostname_clusters").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn package_map(data: &Value, list_key: &str, id_key: &str) -> BTreeMap<String, Value> {
    data.get(list_key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get(id_key)
                .and_then(Value::as_str)
                .map(|key| (key.to_string(), row.clone()))
        })
        .collect()
}

fn registry_extension_rows(
    extension_surface: &Value,
    enablement: &BTreeMap<String, Value>,
    runtime: &serde_json::Map<String, Value>,
    github: &BTreeMap<String, Value>,
) -> Vec<Value> {
    let mut output = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let lanes = extension_surface
        .get("extension_lanes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (lane_name, lane) in lanes {
        let Some(tools) = lane.get("tools").and_then(Value::as_array) else {
            continue;
        };
        for tool in tools {
            let Some(tool_name) = tool.get("tool").and_then(Value::as_str) else {
                continue;
            };
            if !seen.insert(tool_name.to_string()) {
                continue;
            }
            let pkg = enablement
                .get(tool_name)
                .cloned()
                .unwrap_or_else(|| json!({}));
            let rt = runtime.get(tool_name).cloned().unwrap_or_else(|| json!({}));
            let gh = github.get(tool_name).cloned().unwrap_or_else(|| json!({}));
            let activation_status = tool
                .get("activation_status")
                .and_then(Value::as_str)
                .or_else(|| pkg.get("activation_status").and_then(Value::as_str))
                .or_else(|| {
                    gh.get("package_enablement")
                        .and_then(|v| v.get("activation_status"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("planned");
            output.push(json!({
                "tool": tool_name,
                "lane": lane_name,
                "activation_status": activation_status,
                "core_law_position": if activation_status != "active_in_system" { "optional_extension" } else { "bounded_runtime" },
                "marketplace_position": "discovery_only_optional",
                "runtime_status": rt.get("status").cloned().unwrap_or(Value::Null),
                "repo_url": tool.get("repo_url").cloned().or_else(|| gh.get("repo_url").cloned()).unwrap_or(Value::Null),
                "next_action": tool.get("next_action").cloned().or_else(|| pkg.get("next_action").cloned()).unwrap_or(Value::Null),
            }));
        }
    }
    output
}

fn keyed_rows(data: &Value, list_key: &str, id_key: &str) -> BTreeMap<String, Value> {
    data.get(list_key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get(id_key)
                .and_then(Value::as_str)
                .map(|key| (key.to_string(), row.clone()))
        })
        .collect()
}

fn operator_actions_by_target(data: &Value) -> BTreeMap<String, Vec<Value>> {
    let mut mapped = BTreeMap::new();
    for entry in data
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let title = entry.get("title").and_then(Value::as_str).unwrap_or("");
        let Some((target_id, _)) = title.split_once(" requires canonical edge identity binding")
        else {
            continue;
        };
        let target_id = target_id.trim();
        if target_id.is_empty() {
            continue;
        }
        mapped
            .entry(target_id.to_string())
            .or_insert_with(Vec::new)
            .push(entry.clone());
    }
    mapped
}

fn configured_targets(fleet_runtime: &Value) -> BTreeMap<String, Value> {
    fleet_runtime
        .get("inventory")
        .and_then(|v| v.get("configured_targets"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect()
}

fn stage_from_plan(plan: &Value) -> &'static str {
    match plan
        .get("enrollment_status")
        .and_then(Value::as_str)
        .unwrap_or("")
    {
        "planned_discovery" => "identity_binding_pending",
        "active_staging" => "bootstrap_and_promotion",
        "active" => "active",
        _ => "planned",
    }
}

fn build_target_contract(
    plan: &Value,
    configured_target: &Value,
    binding: Option<&Value>,
    target_actions: Vec<Value>,
) -> Value {
    let next_steps = plan.get("next_steps").cloned().unwrap_or_else(|| json!([]));
    json!({
        "target_id": plan.get("target_id").cloned().unwrap_or(Value::Null),
        "hostname": plan.get("hostname").cloned().or_else(|| configured_target.get("hostname").cloned()).unwrap_or(Value::Null),
        "role": plan.get("role").cloned().or_else(|| configured_target.get("role").cloned()).unwrap_or(Value::Null),
        "node_class": plan.get("node_class").cloned().or_else(|| configured_target.get("node_class").cloned()).unwrap_or(Value::Null),
        "enrollment_status": plan.get("enrollment_status").cloned().or_else(|| configured_target.get("enrollment_status").cloned()).unwrap_or(Value::Null),
        "onboarding_stage": stage_from_plan(plan),
        "llm_runtime": configured_target.get("llm_runtime").cloned().unwrap_or(Value::Null),
        "binding_strategy": binding.and_then(|v| v.get("binding_strategy")).cloned().unwrap_or(Value::Null),
        "expected_hostname": binding.and_then(|v| v.get("expected_hostname")).cloned().unwrap_or(Value::Null),
        "candidate_tailscale_names": binding.and_then(|v| v.get("candidate_tailscale_names")).cloned().unwrap_or_else(|| json!([])),
        "candidate_stale_node_ids": binding.and_then(|v| v.get("candidate_stale_node_ids")).cloned().unwrap_or_else(|| json!([])),
        "operator_actions": target_actions.into_iter().map(|action| json!({
            "action": action.get("action").cloned().unwrap_or(Value::Null),
            "kind": action.get("kind").cloned().unwrap_or(Value::Null),
            "reason": action.get("reason").cloned().or_else(|| action.get("note").cloned()).unwrap_or(Value::Null),
            "status": action.get("status").cloned().unwrap_or(Value::Null),
            "title": action.get("title").cloned().unwrap_or(Value::Null),
            "note": action.get("note").cloned().unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "required_evidence": [
            "core/state/fleet_identity_reconciliation.json",
            "core/state/edge_enrollment_plan.json",
            "core/state/warden_edge_contract.json",
            "core/state/fleet_runtime.json",
        ],
        "execution_sequence": next_steps,
    })
}

fn remediation_target(
    target_id: &str,
    plan: &Value,
    onboarding: Option<&Value>,
    actions: Vec<Value>,
) -> Value {
    json!({
        "target_id": target_id,
        "hostname": plan.get("hostname").cloned().unwrap_or(Value::Null),
        "role": plan.get("role").cloned().unwrap_or(Value::Null),
        "node_class": plan.get("node_class").cloned().unwrap_or(Value::Null),
        "current_state": plan.get("enrollment_status").cloned().unwrap_or(Value::Null),
        "binding_strategy": plan.get("identity_binding").and_then(|v| v.get("binding_strategy")).cloned().unwrap_or(Value::Null),
        "expected_hostname": plan.get("identity_binding").and_then(|v| v.get("expected_hostname")).cloned().unwrap_or(Value::Null),
        "candidate_stale_node_ids": plan.get("identity_binding").and_then(|v| v.get("candidate_stale_node_ids")).cloned().unwrap_or_else(|| json!([])),
        "operator_confirmation_required": !actions.is_empty(),
        "remediation_sequence": plan.get("next_steps").cloned().unwrap_or_else(|| json!([])),
        "onboarding_stage": onboarding.and_then(|v| v.get("onboarding_stage")).cloned().unwrap_or(Value::Null),
    })
}
