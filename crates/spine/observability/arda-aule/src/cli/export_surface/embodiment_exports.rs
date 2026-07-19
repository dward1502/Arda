#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::*;

const SOTERION_GLYPHS: &[&str] = &["∇", "⚡", "♥", "◈", "↝", "𓀀", "𓆣", "𓂀"];

pub(crate) fn export_embodied_interface_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/embodied_interface.json");
    let realm_data = read_toml_or(
        &root.join("core/realm/arda.toml"),
        toml::Value::Table(Default::default()),
    );
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let edge_targets = read_toml_or(
        &root.join("core/edge/targets.toml"),
        toml::Value::Table(Default::default()),
    );
    let realms = realm_data
        .get("realms")
        .and_then(toml::Value::as_table)
        .and_then(|rows| rows.get("definition"))
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let embodied_target = load_edge_target(&edge_targets, "node-pi5-warden");
    let avatar_target = load_edge_target(&edge_targets, "node-pi5-citadel-avatar");
    let embodied_bootstrap_state = load_bootstrap_state(&operator_actions, "node-pi5-warden");
    let avatar_bootstrap_state = if avatar_target
        .get("tailscale_ip")
        .and_then(Value::as_str)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
        && avatar_target
            .get("ssh_user")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    {
        "tailscale_visible_ssh_ready"
    } else {
        "planned_discovery"
    };

    let realm_nodes = realms
        .into_iter()
        .filter_map(|realm| {
            let realm = realm.as_table()?;
            let realm_id = realm.get("id").and_then(toml::Value::as_str)?;
            let geometry = geometry_for_realm(realm_id);
            Some(json!({
                "realm_id": realm_id,
                "name": json_from_toml(realm.get("name")),
                "color": json_from_toml(realm.get("color")),
                "agents": json_from_toml(realm.get("agents")),
                "shape": geometry.0,
                "motion": geometry.1,
                "activation_rule": "derive from live runtime state, queue pressure, and agent heartbeat rather than decorative timers",
            }))
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.embodied-interface.v1",
        "generated_at_utc": now_utc(),
        "authority": "core_realm_identity + embodied_interface_design",
        "hardware_targets": [
            {
                "device": "raspberry_pi_5_guardhouse",
                "role": "home network guardhouse and CEO-callable utility node",
                "node": embodied_target,
                "bootstrap_state": embodied_bootstrap_state,
            },
            {
                "device": "raspberry_pi_5_ai_hat_plus",
                "role": "dedicated citadel avatar controller",
                "node": avatar_target,
                "bootstrap_state": avatar_bootstrap_state,
            },
            {
                "device": "peppers_ghost_avatar_enclosure",
                "role": "visual projection and LED diffusion shell",
            },
        ],
        "rendering_rules": {
            "truth_binding": "only animate state changes grounded in sovereign runtime truth",
            "color_source": "core/realm/arda.toml realm colors",
            "geometry_family": "sacred_geometry_and_platonic_solids",
            "fallback_mode": "idle_resonance_glow",
        },
        "realms": realm_nodes,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_tauri_embodiment_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/tauri_embodiment.json");
    let payload = json!({
        "schema_version": "arda.tauri-embodiment.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_source_src_d48f6cc9",
        "source_id": "src_d48f6cc9",
        "stack": {
            "backend": "rust_tauri",
            "frontend": "vite_react",
            "scene": "react_three_fiber",
            "timeline": "theatre_js",
            "preferred_renderer": "three_js",
            "webgpu_branch_recommended": true,
        },
        "rendering_doctrine": {
            "event_driven": true,
            "bind_to_runtime_truth": true,
            "avoid_decorative_only_motion": true,
            "raw_webgl_only_for_bottlenecks": true,
        },
        "embodied_mapping": {
            "avatar_motion": "agent_events_and_governance_states",
            "geometry_motion": "realm_activity_and_queue_pressure",
            "led_behavior": "control_plane_and_monitoring_signals",
            "peppers_ghost": "projection_shell_over_same_contract",
        },
        "practical_notes": {
            "software_render_fallback_ok": true,
            "gpu_feature_flags_may_be_needed": true,
            "x11_dev_fallback_possible": true,
            "webxr_future_safe": true,
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_legion_hierarchy_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/legion_hierarchy.json");
    let payload = json!({
        "schema_version": "arda.legion-hierarchy.v1",
        "generated_at_utc": now_utc(),
        "authority": "prometheus_governance_evaluation",
        "decision": "selective_only",
        "doctrine": {
            "default_mode": "flat_sovereign_crate",
            "military_hierarchy_in_base_systems": false,
            "marketplace_or_client_overlays_only": true,
            "requires_governance_validation": true,
        },
        "validator_harnesses": {
            "triad_required": true,
            "bacon_lite_required": true,
            "joulework_required": true,
            "love_equation_required": true,
        },
        "rank_mapping": {
            "legate": {
                "source": "prometheus_or_ceo_overlay",
                "role": "strategic coordinator for bounded campaign",
            },
            "centurion": {
                "source": "hermes_charon_specialists",
                "role": "specialist relay and coordination node",
            },
            "legionary": {
                "source": "apollo_execution_workers",
                "role": "execution cohort under bounded plan",
            },
        },
        "allowed_domains": [
            {
                "domain": "marketing_campaigns",
                "allowed": true,
                "reason": "bounded campaign orchestration benefits from explicit hierarchy",
            },
            {
                "domain": "finance_risk_drills",
                "allowed": true,
                "reason": "high-stakes simulations benefit from disciplined delegation",
            },
            {
                "domain": "business_ops_campaigns",
                "allowed": true,
                "reason": "coordinated multi-step execution can use temporary chain of command",
            },
            {
                "domain": "personal_companion_flows",
                "allowed": false,
                "reason": "human intimacy and adaptability are better served by softer coordination",
            },
            {
                "domain": "core_sovereign_control_plane",
                "allowed": false,
                "reason": "core authority remains realm-based, not militarized",
            },
        ],
        "activation_requirements": [
            "bounded_scope_declared",
            "time_window_declared",
            "jw_budget_declared",
            "receipt_capture_enabled",
            "operator_visibility_in_arda",
        ],
        "operator_guidance": {
            "use_for": [
                "temporary campaign structures",
                "product or client execution swarms",
                "high-stakes domain drills",
            ],
            "do_not_use_for": [
                "base system authority",
                "default crate topology",
                "human relationship surfaces",
            ],
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "decision": payload.get("decision").cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_task_agent_boundaries_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/task_agent_boundaries.json");
    let runtime_budget_policy = read_json_or(
        &root.join("core/state/runtime_budget_policy.json"),
        json!({}),
    );
    let dossier_standard = read_json_or(
        &root.join("core/state/project_dossier_standard.json"),
        json!({}),
    );
    let intake_contract = read_json_or(
        &root.join("core/state/imported_memory_intake_contract.json"),
        json!({}),
    );
    let classification = read_json_or(
        &root.join("core/state/portfolio_classification_posture.json"),
        json!({}),
    );
    let lifecycle = read_json_or(
        &root.join("core/state/project_intake_lifecycle.json"),
        json!({}),
    );
    let intake_governance = read_json_or(
        &root.join("core/state/project_intake_governance.json"),
        json!({}),
    );
    let tasks = latest_task_state(&read_jsonl_objects(
        &root.join("core/projects/tasks/queue.jsonl"),
    ));

    let mut queued = tasks
        .iter()
        .filter(|task| task.get("status").and_then(Value::as_str) == Some("queued"))
        .cloned()
        .collect::<Vec<_>>();
    queued.sort_by_key(queue_sort_key);
    queued.reverse();

    let mut in_progress = tasks
        .iter()
        .filter(|task| task.get("status").and_then(Value::as_str) == Some("in_progress"))
        .cloned()
        .collect::<Vec<_>>();
    in_progress.sort_by_key(queue_sort_key);
    in_progress.reverse();

    let mut recent_completed = tasks
        .iter()
        .filter(|task| task.get("status").and_then(Value::as_str) == Some("completed"))
        .cloned()
        .collect::<Vec<_>>();
    recent_completed.sort_by_key(queue_sort_key);
    recent_completed.reverse();
    recent_completed.truncate(8);

    let sampled = in_progress
        .iter()
        .take(6)
        .chain(queued.iter().take(6))
        .chain(recent_completed.iter())
        .cloned()
        .collect::<Vec<_>>();
    let mut soterion_present_total = 0usize;
    let mut joulework_required_total = 0usize;
    for task in &sampled {
        let governance = build_boundary(task)
            .get("governance_requirements")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if governance
            .get("soterion_trace_present")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            soterion_present_total += 1;
        }
        if governance
            .get("joulework_budget_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            joulework_required_total += 1;
        }
    }

    let active_boundaries = in_progress
        .iter()
        .take(6)
        .chain(queued.iter().take(6))
        .map(build_boundary)
        .collect::<Vec<_>>();
    let recent_completed_boundaries = recent_completed
        .iter()
        .map(build_boundary)
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.task-agent-boundaries.v1",
        "generated_at_utc": now_utc(),
        "authority": "task_agent_boundary_export",
        "queue_layers": {
            "runtime_queue": {
                "path": "core/queue/queue.jsonl",
                "role": "ephemeral_runtime_work_packets",
                "intended_for": [
                    "generic operational jobs",
                    "short-lived execution work",
                    "runtime queue observability",
                ],
            },
            "project_task_queue": {
                "path": "core/projects/tasks/queue.jsonl",
                "role": "authoritative_strategic_pivot_and_project_ledger",
                "intended_for": [
                    "session pivots",
                    "architectural and strategic work",
                    "bounded executor handoff surfaces",
                    "authoritative task history",
                ],
            },
            "boundary_rule": "Use core/queue for ephemeral runtime jobs and core/projects/tasks for strategic/project work; do not collapse them into one surface unless executor doctrine is redesigned.",
        },
        "intake_governance": {
            "project_dossier_standard_path": "core/state/project_dossier_standard.json",
            "imported_memory_intake_contract_path": "core/state/imported_memory_intake_contract.json",
            "portfolio_classification_posture_path": "core/state/portfolio_classification_posture.json",
            "project_intake_lifecycle_path": "core/state/project_intake_lifecycle.json",
            "project_intake_governance_path": "core/state/project_intake_governance.json",
            "required_dossier_fields_total": dossier_standard.get("required_fields").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "intake_states_total": intake_contract.get("states").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "classification_labels_total": classification.get("labels").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "lifecycle_stages_total": lifecycle.get("stages").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            "tracked_dossiers_total": intake_governance.get("summary").and_then(|v| v.get("tracked_dossiers_total")).cloned().unwrap_or(Value::Null),
            "execution_eligible_total": intake_governance.get("summary").and_then(|v| v.get("execution_eligible_total")).cloned().unwrap_or(Value::Null),
        },
        "summary": {
            "queued_total": queued.len(),
            "in_progress_total": in_progress.len(),
            "recent_completed_total": recent_completed.len(),
            "sampled_tasks_total": sampled.len(),
            "soterion_trace_present_total": soterion_present_total,
            "soterion_trace_missing_total": sampled.len().saturating_sub(soterion_present_total),
            "joulework_budget_required_total": joulework_required_total,
            "local_joulework_usage_percent": runtime_budget_policy.get("user_plan_budget").and_then(|v| v.get("local_joulework_usage_percent")).cloned().unwrap_or(Value::Null),
        },
        "active_boundaries": active_boundaries,
        "recent_completed_boundaries": recent_completed_boundaries,
        "isolation_rules": [
            "Each active task must cite its own evidence surfaces before acting.",
            "Generic hostnames or agent names are not enough to merge contexts across tasks.",
            "Task owner and scope take precedence over ambient session flow when parallel work is in progress.",
            "Imported memory must pass summary -> dossier -> bounded plan -> review before execution eligibility.",
            "No project should enter strategic execution without a canonical dossier anchor.",
        ],
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_soterion_joulework_enforcement_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/soterion_joulework_enforcement.json");
    let task_boundaries = read_json_or(
        &root.join("core/state/task_agent_boundaries.json"),
        json!({}),
    );
    let runtime_budget_policy = read_json_or(
        &root.join("core/state/runtime_budget_policy.json"),
        json!({}),
    );
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );
    let hades_status = read_json_or(
        &root.join("core/metrics/by_crate/hades/status.json"),
        json!({}),
    );
    let charon_status = read_json_or(
        &root.join("core/metrics/by_crate/charon/status.json"),
        json!({}),
    );
    let tasks = latest_task_state(&read_jsonl_objects(
        &root.join("core/projects/tasks/queue.jsonl"),
    ));

    let mut recent_tasks = tasks
        .into_iter()
        .filter(|row| {
            matches!(
                row.get("status").and_then(Value::as_str),
                Some("queued" | "in_progress" | "completed")
            )
        })
        .collect::<Vec<_>>();
    recent_tasks.sort_by_key(|row| {
        row.get("queued_at_utc")
            .and_then(Value::as_str)
            .or_else(|| row.get("completed_at_utc").and_then(Value::as_str))
            .unwrap_or("")
            .to_string()
    });
    recent_tasks.reverse();
    recent_tasks.truncate(12);

    let mut sampled_rows = Vec::new();
    let mut missing_soterion = Vec::new();
    for row in &recent_tasks {
        let has_soterion = task_has_soterion_trace(row);
        let requires_joule = task_requires_joulework(row);
        let entry = json!({
            "task_id": row.get("id").cloned().unwrap_or(Value::Null),
            "title": row.get("title").cloned().unwrap_or(Value::Null),
            "owner": row.get("owner").cloned().unwrap_or(Value::Null),
            "status": row.get("status").cloned().unwrap_or(Value::Null),
            "priority": row.get("priority").cloned().unwrap_or(Value::Null),
            "soterion_trace_present": has_soterion,
            "joulework_budget_required": requires_joule,
        });
        if !has_soterion {
            missing_soterion.push(entry.clone());
        }
        sampled_rows.push(entry);
    }

    let local_joule_percent = runtime_budget_policy
        .get("user_plan_budget")
        .and_then(|v| v.get("local_joulework_usage_percent"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let local_joule_pressure = local_joule_percent >= 80.0;
    let malformed_hades = hades_status
        .get("malformed_joulework_records")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let providers_ready = charon_status
        .get("providers_ready")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let providers_degraded = charon_status
        .get("providers_degraded")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let payload = json!({
        "schema_version": "arda.soterion-joulework-enforcement.v1",
        "generated_at_utc": now_utc(),
        "authority": "task_agent_boundaries + runtime_budget_policy + hades_status + charon_status",
        "doctrine": {
            "meaningful_tasks_should_carry_soterion_trace": true,
            "high_cost_or_high_priority_work_requires_joulework_budget_awareness": true,
            "hades_joulework_integrity_must_remain_clean": true,
            "charon_routing_must_observe_joule_pressure_before_expensive_work": true,
        },
        "summary": {
            "sampled_tasks_total": sampled_rows.len(),
            "soterion_trace_present_total": sampled_rows.iter().filter(|row| row.get("soterion_trace_present").and_then(Value::as_bool) == Some(true)).count(),
            "soterion_trace_missing_total": missing_soterion.len(),
            "joulework_budget_required_total": sampled_rows.iter().filter(|row| row.get("joulework_budget_required").and_then(Value::as_bool) == Some(true)).count(),
            "local_joulework_usage_percent": ((local_joule_percent * 100.0).round() / 100.0),
            "local_joule_pressure": local_joule_pressure,
            "hades_malformed_joulework_records": malformed_hades,
            "charon_providers_ready": providers_ready,
            "charon_providers_degraded": providers_degraded,
        },
        "task_trace_audit": {
            "source": "core/projects/tasks/queue.jsonl",
            "recent_tasks": sampled_rows,
            "missing_soterion_trace": missing_soterion,
        },
        "runtime_enforcement": {
            "task_boundaries_summary": task_boundaries.get("summary").cloned().unwrap_or_else(|| json!({})),
            "runtime_governor_budget_lane": runtime_governor.get("capability_lanes").and_then(|v| v.get("user_and_provider_budget_pressure")).cloned().unwrap_or_else(|| json!({})),
            "hades_status": {
                "pending_actions": hades_status.get("pending_actions").cloned().unwrap_or(Value::Null),
                "orphans_active": hades_status.get("orphans_active").cloned().unwrap_or(Value::Null),
                "malformed_joulework_records": malformed_hades,
            },
            "charon_status": {
                "providers_ready": providers_ready,
                "providers_degraded": providers_degraded,
                "recent_route_failures": charon_status.get("recent_route_failures").cloned().unwrap_or(Value::Null),
                "runtime_build_cache_status": charon_status.get("runtime_build_cache_status").cloned().unwrap_or(Value::Null),
            },
        },
        "verdicts": [
            {
                "id": "task_trace_coverage",
                "status": if missing_soterion.is_empty() { "healthy" } else { "attention_required" },
                "reason": if missing_soterion.is_empty() {
                    "Recent sampled tasks carry Soterion trace markers."
                } else {
                    "Recent tasks without Soterion glyph trace remain in the ledger."
                },
            },
            {
                "id": "local_joule_budget",
                "status": if local_joule_pressure { "attention_required" } else { "healthy" },
                "reason": format!("Local JouleWork usage is {:.2}% of soft cap.", local_joule_percent),
            },
            {
                "id": "hades_joule_integrity",
                "status": if malformed_hades > 0 { "attention_required" } else { "healthy" },
                "reason": format!("HADES malformed JouleWork records: {malformed_hades}."),
            },
        ],
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "sampled_tasks_total": sampled_rows.len(),
    }))
}

pub(crate) fn export_imported_capability_reconciliation_impl() -> Result<Value> {
    let root = workspace_root();
    let numenor_root = numenor_prime_root();
    let state = root.join("core/state");
    let package_activation =
        read_json_or(&state.join("package_runtime_activation.json"), json!({}));
    let package_enablement = read_json_or(&state.join("package_enablement.json"), json!({}));
    let crawl_contract = read_json_or(&state.join("crawl4ai_runtime_contract.json"), json!({}));
    let community_sources = read_json_or(&state.join("hermes_community_sources.json"), json!({}));
    let hermes_discord_runtime =
        read_json_or(&state.join("hermes_discord_runtime.json"), json!({}));
    let extension_backlog =
        read_json_or(&state.join("extension_activation_backlog.json"), json!({}));
    let github_integration = read_json_or(&state.join("github_repo_integration.json"), json!({}));

    let tool_map = package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("tool")
                .and_then(Value::as_str)
                .map(|tool| (tool.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let github_tool_map = github_integration
        .get("registry_tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("tool")
                .and_then(Value::as_str)
                .map(|tool| (tool.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let runtime_surfaces = package_activation
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let crawl_pkg = tool_map
        .get("crawl4ai")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let crawl_runtime = runtime_surfaces
        .get("crawl4ai")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let athena_payload = json!({
        "schema_version": "arda.athena-imported-capability-roadmap.v1",
        "generated_at_utc": now_utc(),
        "authority": "rank2_capability_reconciliation",
        "status": "roadmap_ready_for_execution_review",
        "mission": "Use imported crawl memory to improve ATHENA ingest capability without violating current runtime truth.",
        "current_truth": {
            "policy_readiness": crawl_pkg.get("policy_readiness").cloned().unwrap_or(Value::Null),
            "integration_state": crawl_pkg.get("integration_state").cloned().unwrap_or(Value::Null),
            "activation_status": crawl_pkg.get("activation_status").cloned().unwrap_or(Value::Null),
            "runtime_status": crawl_runtime.get("status").cloned().unwrap_or(Value::Null),
            "live_primary_designated": crawl_contract.get("doctrine").and_then(|v| v.get("crawl4ai_is_live_primary_ingest")).cloned().unwrap_or(Value::Null),
        },
        "decision": {
            "posture": "activate_bounded_runtime",
            "why": "crawl4ai is already policy-ready, doctrinally primary for ATHENA crawling, and runtime-contract aligned; the gap is service activation, not conceptual fit."
        },
        "execution_steps": [
            "start crawl4ai through sovereign runtime surface",
            "verify runtime status shows running and ready=true",
            "run a bounded ATHENA crawl and confirm markdown lands correctly",
            "only then treat crawl4ai as live operational capability rather than activation frontier"
        ],
        "deferments": [
            "do not let scrapling shim promotion demote crawl4ai without new gate review"
        ],
        "source_lineage": [
            numenor_root.join("Elros/missions/MISSION_CRAWL4AI.md").display().to_string(),
            numenor_root.join("Knowledge/Ingest/crawl4ai_2026-02-21.md").display().to_string(),
            numenor_root.join("Knowledge/Research/Tools/crawl4ai_research.md").display().to_string()
        ]
    });

    let discord_pkg = tool_map
        .get("discord-mcp")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let discord_backlog = extension_backlog
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.get("tool").and_then(Value::as_str) == Some("discord-mcp"))
        .cloned()
        .collect::<Vec<_>>();
    let hermes_payload = json!({
        "schema_version": "arda.hermes-imported-capability-roadmap.v1",
        "generated_at_utc": now_utc(),
        "authority": "rank2_capability_reconciliation",
        "status": "roadmap_ready_for_execution_review",
        "mission": "Use imported Discord memory to clarify Hermes comms posture without treating community or MCP surfaces as mandatory runtime dependencies.",
        "current_truth": {
            "policy_readiness": discord_pkg.get("policy_readiness").cloned().unwrap_or(Value::Null),
            "integration_state": discord_pkg.get("integration_state").cloned().unwrap_or(Value::Null),
            "activation_status": discord_pkg.get("activation_status").cloned().unwrap_or(Value::Null),
            "community_discord_mode": community_sources.get("doctrine").and_then(|v| v.get("discord_mode")).cloned().unwrap_or(Value::Null),
            "community_sources_are_observation_inputs": community_sources.get("doctrine").and_then(|v| v.get("community_sources_are_observation_inputs")).cloned().unwrap_or(Value::Null),
            "bridge_delivery_posture": hermes_discord_runtime.get("delivery").and_then(|v| v.get("posture")).cloned().unwrap_or(Value::Null),
            "bridge_provider_configured": hermes_discord_runtime.get("provider").and_then(|v| v.get("configured")).cloned().unwrap_or(Value::Null),
            "bridge_provider_online": hermes_discord_runtime.get("provider").and_then(|v| v.get("online")).cloned().unwrap_or(Value::Null),
            "bridge_recent_failed_total": hermes_discord_runtime.get("delivery").and_then(|v| v.get("recent_failed_total")).cloned().unwrap_or(Value::Null),
            "backlog_phase": discord_backlog.first().and_then(|row| row.get("phase")).cloned().unwrap_or(Value::Null),
        },
        "decision": {
            "posture": "keep_bounded_optional_bridge_and_defer_mcp",
            "why": "Discord is now treated as a Hermes-owned optional bridge rather than a sovereign base layer. Keep the live bridge compact and policy-guarded, reflect degraded delivery truth honestly when the provider is offline, and keep discord-mcp deferred until the bridge proves insufficient."
        },
        "execution_steps": [
            "keep current Hermes Discord bridge compact and governed",
            "track bridge delivery posture explicitly instead of treating offline delivery as deferred doctrine",
            "define the narrow use cases where discord-mcp would add real value beyond the existing Hermes bridge",
            "promote it only if those use cases require a bounded runtime/product surface",
            "otherwise keep Discord in the bounded optional bridge lane"
        ],
        "deferments": [
            "do not treat community Discord maps as runtime dependencies",
            "do not activate discord-mcp just because research exists"
        ],
        "source_lineage": [
            numenor_root.join("Elros/missions/MISSION_DISCORD_MCP.md").display().to_string(),
            numenor_root.join("Knowledge/Ingest/discord-mcp_2026-02-21.md").display().to_string(),
            numenor_root.join("Knowledge/Research/Tools/discord_mcp_research.md").display().to_string()
        ]
    });

    let memo_payload = json!({
        "schema_version": "arda.imported-tool-fit-decision-memo.v1",
        "generated_at_utc": now_utc(),
        "authority": "rank2_capability_reconciliation",
        "status": "decision_memo_ready_for_execution_review",
        "tools": [
            {
                "tool": "crawl4ai",
                "fit_decision": "activate_now_bounded",
                "reason": "Policy-ready, runtime-contract aligned, doctrinally primary for ATHENA crawling; missing piece is simply service activation and verification.",
                "current_status": crawl_runtime.get("status").cloned().unwrap_or(Value::Null),
            },
            {
                "tool": "discord-mcp",
                "fit_decision": "defer_until_bounded_use_case",
                "reason": "Imported research is real, but Hermes now has a bounded optional Discord bridge already. discord-mcp should only be promoted if a concrete use case exceeds that bridge.",
                "current_status": discord_pkg.get("activation_status").cloned().unwrap_or(Value::Null),
            },
            {
                "tool": "arscontexta",
                "fit_decision": "reference_only_for_now",
                "reason": "Good knowledge-system research, but current package surfaces still mark it observed_only/reference_only with no bounded runtime need.",
                "current_status": tool_map.get("arscontexta").and_then(|row| row.get("activation_status")).cloned().unwrap_or(Value::Null),
            },
            {
                "tool": "llmfit",
                "fit_decision": "keep_as_active_signal",
                "reason": "Already providing value as model-selection input. This is tuning work, not first activation work.",
                "current_status": tool_map.get("llmfit").and_then(|row| row.get("activation_status")).cloned().unwrap_or(Value::Null),
            },
            {
                "tool": "agentsmd",
                "fit_decision": "treat_as_repo_discipline_not_runtime",
                "reason": "Its value is explicit agent-readable repo guidance. It supports governance and operator discipline, not a separate runtime activation lane.",
                "current_status": github_tool_map.get("agentsmd").and_then(|row| row.get("package_enablement")).and_then(|v| v.get("activation_status")).cloned().unwrap_or(Value::Null),
            }
        ],
        "ordering": [
            "crawl4ai",
            "llmfit",
            "agentsmd",
            "discord-mcp",
            "arscontexta"
        ]
    });

    let athena_out = state.join("athena_imported_capability_roadmap.json");
    let hermes_out = state.join("hermes_imported_capability_roadmap.json");
    let memo_out = state.join("imported_tool_fit_decision_memo.json");
    write_pretty_json(&athena_out, &athena_payload)?;
    write_pretty_json(&hermes_out, &hermes_payload)?;
    write_pretty_json(&memo_out, &memo_payload)?;
    Ok(json!({
        "athena": rel(&athena_out, &root),
        "hermes": rel(&hermes_out, &root),
        "memo": rel(&memo_out, &root),
    }))
}

fn load_edge_target(edge_targets: &toml::Value, target_id: &str) -> Value {
    edge_targets
        .get("node")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|node| {
            let node = node.as_table()?;
            if node.get("id").and_then(toml::Value::as_str) != Some(target_id) {
                return None;
            }
            Some(json!({
                "node_id": json_from_toml(node.get("id")),
                "hostname": json_from_toml(node.get("hostname")),
                "tailscale_ip": json_from_toml(node.get("tailscale_ip")),
                "ssh_user": json_from_toml(node.get("ssh_user")),
                "node_class": json_from_toml(node.get("node_class")),
                "role": json_from_toml(node.get("role")),
                "enrollment_status": json_from_toml(node.get("enrollment_status")),
                "llm_runtime": json_from_toml(node.get("llm_runtime")),
            }))
        })
        .unwrap_or_else(|| json!({}))
}

fn load_bootstrap_state(operator_actions: &Value, target_id: &str) -> &'static str {
    let actions = operator_actions
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for action in actions {
        if serde_json::to_string(&action)
            .ok()
            .map(|raw| raw.contains(target_id))
            .unwrap_or(false)
        {
            return "tailscale_visible_ssh_not_ready";
        }
    }
    "tailscale_visible_ssh_ready"
}

fn geometry_for_realm(realm_id: &str) -> (&'static str, &'static str) {
    match realm_id {
        "command" => ("tetrahedron", "axial_pulse"),
        "knowledge" => ("octahedron", "orbital_spin"),
        "operations" => ("cube", "step_lattice"),
        "finance" => ("dodecahedron", "breathing_expansion"),
        "communications" => ("icosahedron", "signal_ripple"),
        "governance" => ("merkaba", "counter_rotation"),
        "monitoring" => ("cube_frame", "sentinel_sweep"),
        _ => ("sphere", "steady_glow"),
    }
}

fn latest_task_state(tasks: &[Value]) -> Vec<Value> {
    let mut latest = BTreeMap::new();
    for task in tasks {
        let Some(task_id) = task.get("id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if !task_id.is_empty() {
            latest.insert(task_id.to_string(), task.clone());
        }
    }
    latest.into_values().collect()
}

fn queue_sort_key(task: &Value) -> String {
    task.get("queued_at_utc")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn build_boundary(task: &Value) -> Value {
    let meta = task
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let scope = meta
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("unscoped");
    let owner = task
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let surfaces = scope_surfaces(scope);
    let glyphs = task
        .get("glyphs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|glyph| glyph.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let soterion_trace_present = glyphs
        .iter()
        .any(|glyph| SOTERION_GLYPHS.contains(&glyph.as_str()));
    let joulework_required = task_requires_joulework(task);

    json!({
        "task_id": task.get("id").cloned().unwrap_or(Value::Null),
        "title": task.get("title").cloned().unwrap_or(Value::Null),
        "owner": owner,
        "status": task.get("status").cloned().unwrap_or(Value::Null),
        "priority": task.get("priority").cloned().unwrap_or(Value::Null),
        "scope": scope,
        "origin": meta.get("origin").cloned().unwrap_or(Value::Null),
        "notes": task.get("notes").cloned().unwrap_or(Value::Null),
        "reference_bundle": {
            "authoritative_task_surface": "core/projects/tasks/queue.jsonl",
            "evidence_surfaces": surfaces,
            "owner_boundary": format!("{owner}_owned_scope"),
            "context_isolation_required": true,
        },
        "governance_requirements": {
            "soterion_trace_required": true,
            "soterion_trace_present": soterion_trace_present,
            "soterion_glyphs": glyphs,
            "joulework_budget_required": joulework_required,
            "joulework_budget_surface": if joulework_required { json!("core/state/runtime_budget_policy.json") } else { Value::Null },
            "love_equation_review_required": task.get("priority").and_then(Value::as_str) == Some("critical"),
        },
    })
}

fn task_has_soterion_trace(task: &Value) -> bool {
    task.get("glyphs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|glyph| SOTERION_GLYPHS.contains(&glyph))
}

fn task_requires_joulework(task: &Value) -> bool {
    let scope = task
        .get("meta")
        .and_then(|v| v.get("scope"))
        .and_then(Value::as_str)
        .unwrap_or("unscoped");
    let priority = task.get("priority").and_then(Value::as_str).unwrap_or("");
    matches!(priority, "high" | "critical") || !matches!(scope, "unscoped" | "auto_emission")
}

fn scope_surfaces(scope: &str) -> Value {
    let surfaces = match scope {
        "athena_hades_integration" => vec![
            "core/state/autonomy_resume.json",
            "core/metrics/manifest.json",
            "core/state/athena_runtime.json",
            "core/state/hades_lifecycle.json",
        ],
        "github_corpus" => vec![
            "core/state/github_repo_integration.json",
            "core/state/package_enablement.json",
            "core/state/package_runtime_activation.json",
        ],
        "package_activation" => vec![
            "core/state/package_enablement.json",
            "core/state/package_runtime_activation.json",
            "core/state/github_repo_integration.json",
        ],
        "edge_registry" => vec![
            "core/state/fleet_identity_reconciliation.json",
            "core/state/fleet_runtime.json",
            "core/state/warden_edge_contract.json",
            "core/state/edge_enrollment_plan.json",
            "core/state/edge_identity_remediation_contract.json",
        ],
        "hades_queue" => vec![
            "core/metrics/by_crate/hades/status.json",
            "core/metrics/by_crate/hades/queue.json",
            "core/state/hades_lifecycle.json",
        ],
        "hermes_projection" => vec![
            "core/state/hermes_command.json",
            "core/state/matrix_boardrooms.json",
            "core/state/federated_comms_runtime.json",
        ],
        "citadel_avatar" | "edge_worker" => vec![
            "core/state/fleet_runtime.json",
            "core/state/fleet_identity_reconciliation.json",
            "core/state/warden_edge_contract.json",
            "core/state/edge_enrollment_plan.json",
            "core/state/edge_identity_remediation_contract.json",
        ],
        "edge_runtime" => vec![
            "core/state/package_runtime_activation.json",
            "core/state/federated_comms_runtime.json",
            "core/state/fleet_runtime.json",
            "core/state/edge_enrollment_plan.json",
        ],
        "network_native_onboarding" => vec![
            "core/state/openfang_alignment.json",
            "core/state/warden_edge_contract.json",
            "core/state/fleet_runtime.json",
            "core/state/fleet_identity_reconciliation.json",
            "core/state/edge_enrollment_plan.json",
            "core/state/network_native_node_onboarding_contract.json",
        ],
        "agent_framework" => vec![
            "config/opencode_agent_routes.toml",
            "core/state/package_runtime_activation.json",
            "core/state/github_repo_integration.json",
        ],
        "aipkg_marketplace" => vec![
            "core/state/openfang_alignment.json",
            "core/state/aipkg_contract.json",
            "core/state/aipkg_marketplace_separation_contract.json",
            "core/state/extension_surface_contract.json",
            "core/state/extension_activation_backlog.json",
            "core/state/package_runtime_activation.json",
        ],
        _ => vec![
            "core/projects/tasks/queue.jsonl",
            "core/state/autonomy_resume.json",
        ],
    };
    json!(surfaces)
}

fn json_from_toml(value: Option<&toml::Value>) -> Value {
    value
        .and_then(|value| serde_json::to_value(value).ok())
        .unwrap_or(Value::Null)
}
