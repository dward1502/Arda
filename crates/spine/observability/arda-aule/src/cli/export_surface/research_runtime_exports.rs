#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;

use super::*;

const CATALOG_RULES: &[(&str, &str, &[&str], &str)] = &[
    (
        "src_33fa61b2",
        "llm_agent_landscape",
        &["reasoning", "code", "tooling", "orchestration", "agent"],
        "prometheus",
    ),
    (
        "src_ca2f031e",
        "agent_ecosystem_landscape",
        &["framework", "browser", "sandbox", "workflow", "agent"],
        "prometheus",
    ),
];

pub(crate) fn export_source_ecosystem_operationalization_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_ecosystem_operationalization.json");
    let registry = read_json_or(
        &root.join("core/state/source_ecosystem_registry.json"),
        json!({}),
    );
    let github = read_json_or(
        &root.join("core/state/github_repo_integration.json"),
        json!({}),
    );
    let sources = registry
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("source_id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let registry_tools = github
        .get("registry_tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut catalogs = Vec::new();
    let mut lane_counts = BTreeMap::new();
    for (source_id, lane, target_labels, owner) in CATALOG_RULES {
        let Some(source_row) = sources.get(*source_id) else {
            continue;
        };
        let catalog = build_catalog_operationalization(
            source_id,
            lane,
            target_labels,
            owner,
            source_row,
            &registry_tools,
        );
        *lane_counts.entry((*lane).to_string()).or_insert(0usize) += 1;
        catalogs.push(catalog);
    }

    let payload = json!({
        "schema_version": "arda.source-ecosystem-operationalization.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_ecosystem_registry + github_repo_integration",
        "mission": {
            "goal": "Turn ecosystem catalogs into recurring candidate extraction and portfolio ranking lanes instead of static reference lists.",
            "operator_rule": "Only already-digested and bounded candidates may enter the ranking surface automatically.",
        },
        "summary": {
            "catalogs_operationalized_total": catalogs.len(),
            "lanes": lane_counts,
            "promote_now_candidates_total": catalogs.iter().map(|row| row.get("promote_now").and_then(Value::as_array).map(Vec::len).unwrap_or(0)).sum::<usize>(),
            "research_watchlist_total": catalogs.iter().map(|row| row.get("research_watchlist").and_then(Value::as_array).map(Vec::len).unwrap_or(0)).sum::<usize>(),
        },
        "catalogs": catalogs,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "catalogs_operationalized_total": payload.get("summary").and_then(|v| v.get("catalogs_operationalized_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_hermes_discord_runtime_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/hermes_discord_runtime.json");
    let federated = read_json_or(
        &root.join("core/state/federated_comms_runtime.json"),
        json!({}),
    );
    let adapter = read_json_or(
        &root.join("core/state/communication_adapter_contract.json"),
        json!({}),
    );
    let roadmap = read_json_or(
        &root.join("core/state/hermes_imported_capability_roadmap.json"),
        json!({}),
    );
    let providers = read_json_or(
        &root.join("core/metrics/by_crate/hermes/providers.json"),
        json!({}),
    );
    let queue = read_json_or(
        &root.join("core/metrics/by_crate/hermes/queue.json"),
        json!({}),
    );
    let subcomponents = read_json_or(
        &root.join("core/metrics/by_crate/hermes/subcomponents.json"),
        json!([]),
    );

    let configured = string_set(providers.get("configured"));
    let online = string_set(providers.get("online"));
    let offline = string_set(providers.get("offline"));
    let discord_listener = subcomponents
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(Value::as_str) == Some("discord_listener"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let recent_outbound = queue
        .get("recent_outbound")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.get("provider").and_then(Value::as_str) == Some("discord"))
        .collect::<Vec<_>>();
    let failed_recent = recent_outbound
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("failed"))
        .cloned()
        .collect::<Vec<_>>();
    let dispatched_recent = recent_outbound
        .iter()
        .filter(|row| row.get("dispatched").and_then(Value::as_bool) == Some(true))
        .cloned()
        .collect::<Vec<_>>();

    let provider_configured = configured.contains("discord");
    let provider_online = online.contains("discord");
    let provider_offline = offline.contains("discord");
    let listener_status = discord_listener
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let delivery_posture = if provider_online && !dispatched_recent.is_empty() {
        "live"
    } else if provider_online && !failed_recent.is_empty() {
        "online_but_delivery_unverified"
    } else if provider_configured && provider_online {
        "online_no_recent_delivery_receipt"
    } else if provider_configured && (provider_offline || !failed_recent.is_empty()) {
        "configured_but_delivery_degraded"
    } else if provider_configured {
        "configured_unknown"
    } else {
        "not_configured"
    };

    let payload = json!({
        "schema_version": "arda.hermes-discord-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "federated_comms_runtime + communication_adapter_contract + hermes_metrics",
        "contract": {
            "discord_mode": federated.get("discord_mode").cloned().unwrap_or(Value::Null),
            "adapter_strategy": federated.get("adapters").and_then(|v| v.get("strategy")).cloned().unwrap_or(Value::Null),
            "bridge_enabled": federated.get("adapters").and_then(|v| v.get("discord_enabled")).and_then(Value::as_bool).unwrap_or(false),
            "matrix_primary_surface": adapter.get("doctrine").and_then(|v| v.get("matrix_boardroom_is_primary_human_agent_room_surface")).and_then(Value::as_bool).unwrap_or(false),
            "discord_not_sovereign_base_layer": adapter.get("doctrine").and_then(|v| v.get("discord_not_sovereign_base_layer")).and_then(Value::as_bool).unwrap_or(false),
        },
        "provider": {
            "configured": provider_configured,
            "online": provider_online,
            "offline": provider_offline,
            "listener_status": listener_status,
        },
        "delivery": {
            "posture": delivery_posture,
            "recent_outbound_total": recent_outbound.len(),
            "recent_failed_total": failed_recent.len(),
            "recent_dispatched_total": dispatched_recent.len(),
            "latest_attempt_at_utc": recent_outbound.first().and_then(|row| row.get("created_at_utc")).cloned().unwrap_or(Value::Null),
        },
        "rank2_alignment": {
            "roadmap_posture": roadmap.get("decision").and_then(|v| v.get("posture")).cloned().unwrap_or(Value::Null),
            "activation_status": roadmap.get("current_truth").and_then(|v| v.get("activation_status")).cloned().unwrap_or(Value::Null),
        },
        "summary": {
            "bridge_governed": provider_configured || federated.get("adapters").and_then(|v| v.get("discord_enabled")).and_then(Value::as_bool).unwrap_or(false),
            "delivery_posture": delivery_posture,
            "recent_failed_total": failed_recent.len(),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "delivery_posture": delivery_posture,
    }))
}

pub(crate) fn export_external_absorption_brief_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/external_absorption_brief.json");
    let comparison_set = vec![
        source_snapshot(&root, "src_ec2b8bd4", "Nous Hermes Agent"),
        source_snapshot(&root, "src_c2fef5eb", "Mission Control"),
        source_snapshot(&root, "src_8b5ab500", "Cloudflare Browser Rendering /crawl"),
        source_snapshot(&root, "src_8422185f", "APS Research Link"),
        source_snapshot(&root, "src_16c075a2", "Terminal-Bench"),
    ];

    let payload = json!({
        "schema_version": "arda.external-absorption-brief.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_books + sovereign_system_state",
        "mission": {
            "goal": "Convert externally observed systems into adopt/ignore/productize decisions for arda without diluting sovereignty.",
            "operator_rule": "Absorb mechanisms and workflows, not branding or entire foreign architectures.",
        },
        "comparison_set": comparison_set,
        "recent_intake_status": [
            {
                "source_id": "src_ec2b8bd4",
                "label": "Nous Hermes Agent",
                "status": "comparison_signal",
                "reason": "Strong overlap in memory, skills, delegation, and messaging, but not yet policy-ready implementation authority.",
            },
            {
                "source_id": "src_c2fef5eb",
                "label": "Mission Control",
                "status": "product_legibility_signal",
                "reason": "Useful as operator-surface and workflow-packaging signal, but not a sovereign architecture template.",
            },
            {
                "source_id": "src_8b5ab500",
                "label": "Cloudflare Browser Rendering /crawl",
                "status": "crawl_policy_signal",
                "reason": "Relevant to ATHENA async crawl posture, compliance, and job-based collection.",
            },
            {
                "source_id": "src_53a02919",
                "status": "routing_signal",
                "reason": "Useful for model-selection and long-context efficiency framing, not direct system architecture.",
            },
            {
                "source_id": "src_8422185f",
                "label": "APS Research Link",
                "status": "blocked_fetch_artifact",
                "reason": "Current ingest hit a Cloudflare verification page rather than paper body, so it is not trustworthy implementation evidence yet.",
            },
            {
                "source_id": "src_16c075a2",
                "label": "Terminal-Bench",
                "status": "policy_ready_anchor",
                "reason": "Already policy-ready and the strongest recent scholarly source for execution harness, routing, memory, and context controls.",
            },
        ],
        "adopt_now": [
            {
                "theme": "persistent agent memory and skill loops",
                "primary_source_id": "src_ec2b8bd4",
                "decision": "adapt",
                "reason": "Hermes Agent validates cross-session memory, skill self-improvement, and user-model continuity, which fit arda biosystem goals.",
                "target_surfaces": [
                    "core/state/async_user_intake_contract.json",
                    "core/state/research_workflow_contract.json",
                    "core/state/multi_domain_routing_contract.json",
                ],
                "productization_move": "Turn memory/skill continuity into a bounded operator-facing capability, not just a backend concept.",
            },
            {
                "theme": "terminal harness with context controls",
                "primary_source_id": "src_16c075a2",
                "decision": "adopt",
                "reason": "Terminal-Bench is already policy-ready and directly supports workload routing, memory retrieval, context compaction, and harnessed execution.",
                "target_surfaces": [
                    "core/state/model_control_surface.json",
                    "core/state/soterion_joulework_enforcement.json",
                    "core/state/project_task_executor.json",
                ],
                "productization_move": "Keep execution behind explicit safety and verification phases while improving context compaction across long-running work.",
            },
            {
                "theme": "bounded whole-site crawling",
                "primary_source_id": "src_8b5ab500",
                "decision": "adapt",
                "reason": "Cloudflare’s /crawl endpoint reinforces async crawl jobs, site guidance compliance, and incremental crawl controls already relevant to ATHENA.",
                "target_surfaces": [
                    "core/state/search_runtime_contract.json",
                    "core/state/athena_integration_plan.json",
                    "core/state/source_absorption_pipeline.json",
                ],
                "productization_move": "Improve ATHENA crawl policies and receipts rather than copying Cloudflare-specific APIs.",
            },
        ],
        "ignore_or_defer": [
            {
                "theme": "foreign all-in-one dashboard clones",
                "primary_source_id": "src_c2fef5eb",
                "decision": "defer",
                "reason": "Mission Control is useful as packaging signal, but arda should not fork into another full TypeScript control plane.",
                "what_to_keep": [
                    "panel legibility",
                    "operator workflow clarity",
                    "gateway abstraction ideas",
                ],
                "what_to_reject": [
                    "whole-dashboard reimplementation",
                    "duplicate orchestration authority outside sovereign state surfaces",
                ],
            },
            {
                "theme": "model-hype as architecture substitute",
                "primary_source_id": "src_53a02919",
                "decision": "defer",
                "reason": "Nemotron is useful as routing/model-selection signal, but it does not change the control-plane architecture by itself.",
                "what_to_keep": [
                    "reasoning-vs-efficiency tradeoff framing",
                    "long-context routing implications",
                ],
                "what_to_reject": [
                    "chasing vendor model announcements as product strategy",
                ],
            },
            {
                "theme": "blocked scholarly fetches without body evidence",
                "primary_source_id": "src_8422185f",
                "decision": "defer",
                "reason": "The APS source is currently only a verification/challenge artifact, not a paper extraction.",
                "what_to_keep": [
                    "pointer for later verified paper retrieval",
                ],
                "what_to_reject": [
                    "using challenge-page content as implementation evidence",
                ],
            },
        ],
        "productize_next": [
            {
                "name": "operator-visible persistent agent continuity",
                "driven_by": ["src_ec2b8bd4", "src_16c075a2"],
                "goal": "Expose persistent memory, skill evolution, and session continuity as a first-class arda operator capability.",
            },
            {
                "name": "legible multi-lane operator surface",
                "driven_by": ["src_c2fef5eb"],
                "goal": "Make ARDA/HUD read as clearly as public orchestration dashboards without ceding sovereign authority.",
            },
            {
                "name": "bounded async crawl orchestration",
                "driven_by": ["src_8b5ab500"],
                "goal": "Advance ATHENA toward asynchronous job-based crawl control with stronger scope/delta/compliance posture.",
            },
            {
                "name": "clear external-intake confidence ladder",
                "driven_by": [
                    "src_ec2b8bd4",
                    "src_c2fef5eb",
                    "src_53a02919",
                    "src_8422185f",
                    "src_16c075a2",
                ],
                "goal": "Distinguish policy-ready anchors from comparison signals, product signals, routing signals, and blocked-fetch artifacts so new intake can be routed without confusion.",
            },
        ],
        "executive_verdict": {
            "arda_is_deeper_than_the_compared_projects": true,
            "arda_is_less_legible_as_a_product": true,
            "next_competitive_gap_to_close": "turn sovereign depth into clearer operator-facing continuity, memory, and orchestration productization",
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_priority_human_contracts_impl() -> Result<Value> {
    let root = workspace_root();
    let group_dir = root.join("core/state/human_corpus_groups");
    let architecture = read_json_or(
        &group_dir.join("numenor_architecture_contracts.json"),
        json!({}),
    );
    let council = read_json_or(&group_dir.join("council_engine.json"), json!({}));
    let forge = read_json_or(&group_dir.join("forge_mind.json"), json!({}));
    let signal = read_json_or(&group_dir.join("signal_grid.json"), json!({}));
    let generated = now_utc();

    let contracts = vec![
        (
            "tool_harness_architecture_contract.json",
            json!({
                "schema_version": "arda.tool-harness-architecture-contract.v1",
                "generated_at_utc": generated,
                "authority": "numenor_architecture_contracts_extraction",
                "source_group": architecture.get("group_id").cloned().unwrap_or(Value::Null),
                "source_ids": architecture.get("source_ids").cloned().unwrap_or_else(|| json!([])),
                "doctrine": {
                    "stable_tool_interfaces_must_outlive_runtime_implementation": true,
                    "mutating_tools_require_trace_actor_idempotency": true,
                    "deterministic_error_envelopes_are_required": true,
                    "policy_and_budget_guards_precede_execution": true,
                },
                "pipeline_layers": [
                    "validate_input",
                    "policy_gate",
                    "budget_guard",
                    "idempotency_gate",
                    "execute",
                    "normalize_output",
                    "observe",
                    "record",
                ],
                "required_metadata": [
                    "tool_id",
                    "version",
                    "owner",
                    "description",
                    "input_schema_ref",
                    "output_schema_ref",
                    "risk_level",
                    "side_effect_class",
                    "retry_policy",
                    "timeout_policy",
                    "policy_requirements",
                    "redaction_hints",
                ],
                "deterministic_errors": [
                    "missing_trace_id",
                    "missing_actor",
                    "missing_idempotency_key",
                    "invalid_request",
                    "policy_deny",
                    "timeout",
                    "temporary_unavailable",
                    "not_found",
                    "internal_error",
                ],
                "service_registry_contract": {
                    "required_fields": [
                        "service_id",
                        "display_name",
                        "zone",
                        "criticality",
                        "profile_modes",
                        "startup_order",
                        "shutdown_order",
                        "healthcheck",
                        "commands",
                        "dependencies",
                        "resource_class",
                        "max_retries",
                        "cooldown_seconds",
                    ],
                    "api_endpoints": [
                        "GET /health",
                        "GET /services",
                        "POST /services/{service_id}/start",
                        "POST /services/{service_id}/stop",
                        "POST /services/{service_id}/restart",
                        "GET /profiles",
                        "POST /profiles/{profile_id}/switch",
                        "GET /events",
                        "POST /policy/evaluate",
                    ],
                },
                "fixture_replay_contract": {
                    "default_dry_run": true,
                    "network_calls_allowed": false,
                    "process_spawning_allowed": false,
                    "deterministic_id_rule": "stable hash from fixture_id",
                    "required_assertions": [
                        "response_fields",
                        "error_code",
                        "policy_decision",
                        "event_result",
                        "state_mutation_flag",
                    ],
                },
                "crate_candidates": architecture.get("extraction").and_then(|v| v.get("crate_candidates")).cloned().unwrap_or(Value::Null),
            }),
        ),
        (
            "business_intelligence_suite_contract.json",
            json!({
                "schema_version": "arda.business-intelligence-suite-contract.v1",
                "generated_at_utc": generated,
                "authority": "council_engine_extraction",
                "source_group": council.get("group_id").cloned().unwrap_or(Value::Null),
                "source_ids": council.get("source_ids").cloned().unwrap_or_else(|| json!([])),
                "positioning": {
                    "suite_role": "bounded business intelligence and legal operating layer",
                    "not_a_substitute_for_licensed_professionals": true,
                    "primary_value": "prepare operator with specialist-context insight before high-stakes decisions",
                },
                "seats": [
                    "economist",
                    "attorney",
                    "cfo",
                    "tax_strategist",
                    "contract_specialist",
                    "strategist",
                    "operator",
                ],
                "query_modes": [
                    "single_seat_query",
                    "dual_seat_query",
                    "full_council_brief",
                    "devils_advocate_mode",
                    "scenario_stress_test",
                    "document_review_mode",
                ],
                "required_outputs": [
                    "seat_opinions",
                    "points_of_agreement",
                    "points_of_tension",
                    "synthesis_recommendation",
                    "licensed_professional_escalation_flag",
                ],
                "guardrails": {
                    "legal_financial_tax_outputs_require_caveat": true,
                    "licensed_escalation_on_high_stakes": true,
                    "document_review_must_preserve_source_context": true,
                },
                "crate_candidate": "arda-council",
            }),
        ),
        (
            "engineering_suite_contract.json",
            json!({
                "schema_version": "arda.engineering-suite-contract.v1",
                "generated_at_utc": generated,
                "authority": "forge_mind_extraction",
                "source_group": forge.get("group_id").cloned().unwrap_or(Value::Null),
                "source_ids": forge.get("source_ids").cloned().unwrap_or_else(|| json!([])),
                "domains": [
                    "software_systems",
                    "hardware_integration",
                    "physical_fabrication",
                    "systems_research",
                    "technical_documentation",
                ],
                "doctrine": {
                    "engineering_spans_software_hardware_and_fabrication": true,
                    "documentation_is_a_first_class_output": true,
                    "research_must_feed_build_and_verification": true,
                },
                "workflow_primitives": [
                    "research_to_build_flow",
                    "documentation_authority_role",
                    "software_hardware_fabrication_split",
                ],
                "downstream_contracts": [
                    "fabrication_research_contract",
                    "technical_documentation_contract",
                ],
                "crate_candidate": "arda-forge-mind",
            }),
        ),
        (
            "social_pipeline_contract.json",
            json!({
                "schema_version": "arda.social-pipeline-contract.v1",
                "generated_at_utc": generated,
                "authority": "signal_grid_extraction",
                "source_group": signal.get("group_id").cloned().unwrap_or(Value::Null),
                "source_ids": signal.get("source_ids").cloned().unwrap_or_else(|| json!([])),
                "doctrine": {
                    "shared_pipeline_multi_brand_execution": true,
                    "brand_voice_isolation_is_required": true,
                    "community_signal_routes_back_into_systems": true,
                    "human_override_required_for_crisis_or_negative_engagement": true,
                },
                "pipeline_primitives": [
                    "multi_brand_voice_isolation",
                    "shared_social_pipeline",
                    "community_and_content_routing",
                    "zero_burnout_workflow",
                ],
                "routing_rules": {
                    "product_feedback": "route_to_product_or_operations",
                    "sales_signal": "route_to_business_intelligence_or_crm",
                    "repeated_questions": "route_to_content_gap_brief",
                    "negative_sentiment_or_crisis": "pause_and_alert_human",
                },
                "analytics_contract": {
                    "metrics_are_only_tracked_if_they_imply_action": true,
                    "weekly_brief_required": true,
                    "real_time_spike_detection_required": true,
                },
                "brand_voice_isolation_contract": {
                    "content_never_bleeds_between_brands": true,
                    "brand_config_owns_voice_and_audience_rules": true,
                },
                "crate_candidate": "arda-signal-grid",
            }),
        ),
    ];

    for (name, payload) in &contracts {
        write_pretty_json(&root.join("core/state").join(name), payload)?;
    }
    Ok(json!({
        "generated_at_utc": generated,
        "contracts_total": contracts.len(),
    }))
}

pub(crate) fn export_priority_human_crate_spawn_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/priority_human_crate_spawn_registry.json");
    let crates = vec![
        crate_row(
            &root,
            "arda-tool-harness",
            "core/state/tool_harness_architecture_contract.json",
        ),
        crate_row(
            &root,
            "arda-service-registry",
            "core/state/tool_harness_architecture_contract.json",
        ),
        crate_row(
            &root,
            "arda-council",
            "core/state/business_intelligence_suite_contract.json",
        ),
        crate_row(
            &root,
            "arda-forge-mind",
            "core/state/engineering_suite_contract.json",
        ),
        crate_row(
            &root,
            "arda-signal-grid",
            "core/state/social_pipeline_contract.json",
        ),
    ];
    let payload = json!({
        "schema_version": "arda.priority-human-crate-spawn-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "priority_human_contract_consumption",
        "summary": {
            "crates_total": crates.len(),
            "present_total": crates.iter().filter(|row| row.get("present").and_then(Value::as_bool) == Some(true)).count(),
        },
        "crates": crates,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "present_total": payload.get("summary").and_then(|v| v.get("present_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_crawl4ai_runtime_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/crawl4ai_runtime_contract.json");
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let athena_plan = read_json_or(
        &root.join("core/state/athena_integration_plan.json"),
        json!({}),
    );

    let crawl4ai = package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("crawl4ai"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime = package_runtime
        .get("surfaces")
        .and_then(|v| v.get("crawl4ai"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.crawl4ai-runtime-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_enablement + package_runtime_activation + athena_integration_plan",
        "tool": {
            "name": "crawl4ai",
            "repo": crawl4ai.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": crawl4ai.get("repo_url").cloned().unwrap_or(Value::Null),
            "activation_status": crawl4ai.get("activation_status").cloned().unwrap_or(Value::Null),
            "integration_state": crawl4ai.get("integration_state").cloned().unwrap_or(Value::Null),
            "runtime_status": runtime.get("status").cloned().unwrap_or(Value::Null),
            "base_url": runtime.get("base_url").cloned().unwrap_or(Value::Null),
            "image": runtime.get("image").cloned().unwrap_or(Value::Null),
            "container_name": runtime.get("container_name").cloned().unwrap_or(Value::Null),
        },
        "doctrine": {
            "crawl4ai_is_live_primary_ingest": true,
            "activation_requires_verified_athena_crawl": true,
            "service_lifecycle_belongs_to_sovereign_runtime_surfaces": true,
            "scrapling_promotion_does_not_demote_crawl4ai_until_gates_pass": true,
        },
        "runtime_contract": {
            "launcher": "scripts/runtime/crawl4ai_service.sh",
            "runtime_env": ["arda_CRAWL4AI_URL"],
            "health_requirements": [
                "runtime surface must report running",
                "runtime surface must report ready=true",
                "ATHENA doctrine must still designate crawl4ai as live primary",
            ],
            "write_through": [
                "core/state/package_runtime_activation.json",
                "core/state/package_enablement.json",
                "core/state/athena_integration_plan.json",
            ],
        },
        "operating_posture": {
            "service_running": runtime.get("status").and_then(Value::as_str) == Some("running"),
            "service_ready": runtime.get("ready").and_then(Value::as_bool).unwrap_or(false),
            "service_ok": runtime.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "live_primary_designated": athena_plan.get("current_posture").and_then(|v| v.get("crawl_primary")).and_then(|v| v.get("tool")).and_then(Value::as_str) == Some("crawl4ai"),
            "next_action": crawl4ai.get("next_action").cloned().unwrap_or(Value::Null),
        },
        "summary": {
            "active_in_system": crawl4ai.get("activation_status").and_then(Value::as_str) == Some("active_in_system"),
            "runtime_ok": runtime.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "base_url": runtime.get("base_url").cloned().unwrap_or(Value::Null),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_litellm_routing_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/litellm_routing_contract.json");
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let manwe_router = read_json_or(&root.join("core/state/manwe_router.json"), json!({}));
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );

    let litellm = find_tool(&package_enablement, "litellm");
    let runtime = package_runtime
        .get("surfaces")
        .and_then(|v| v.get("litellm"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let provider = manwe_router
        .get("provider_pressure")
        .and_then(|v| v.get("providers"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(Value::as_str) == Some("litellm_gateway"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let tool_row = runtime_governor
        .get("capability_lanes")
        .and_then(|v| v.get("tool_activation_and_health"))
        .and_then(|v| v.get("tools"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("litellm"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.litellm-routing-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_enablement + package_runtime_activation + manwe_router + runtime_governor_contract",
        "tool": {
            "name": "litellm",
            "repo": litellm.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": litellm.get("repo_url").cloned().unwrap_or(Value::Null),
            "provider_id": litellm.get("provider_id").cloned().unwrap_or(Value::Null),
            "activation_status": litellm.get("activation_status").cloned().unwrap_or(Value::Null),
            "integration_state": litellm.get("integration_state").cloned().unwrap_or(Value::Null),
            "runtime_status": runtime.get("status").cloned().unwrap_or(Value::Null),
            "proxy_url": runtime.get("proxy_url").cloned().unwrap_or(Value::Null),
            "models_ready": runtime.get("models_ready").cloned().unwrap_or(Value::Null),
        },
        "provider_contract": {
            "provider_id": "litellm_gateway",
            "configured": litellm.get("provider_configured").cloned().unwrap_or(Value::Null),
            "healthy": provider.get("healthy").cloned().unwrap_or(Value::Null),
            "enabled": provider.get("enabled").cloned().unwrap_or(Value::Null),
            "base_url": provider.get("base_url").cloned().unwrap_or(Value::Null),
            "models": provider.get("models").cloned().unwrap_or(Value::Null),
        },
        "routing_contract": {
            "role": "normalized gateway for MANWE and downstream consumers",
            "preferred_for": ["planning", "context_heavy", "provider_normalization"],
            "provider_priority": [
                "litellm_gateway",
                "local_fallback",
                "edge_backbone",
            ],
            "model_control_surface": "core/state/model_control_surface.json",
            "manwe_router": "core/state/manwe_router.json",
        },
        "governor_binding": {
            "input_surface_present": runtime_governor.get("input_surfaces").and_then(Value::as_object).map(|m| m.contains_key("package_enablement")).unwrap_or(false),
            "package_tool_projection_present": tool_row != json!({}),
            "governor_next_action": tool_row.get("next_action").cloned().unwrap_or(Value::Null),
            "writes_through": [
                "core/state/manwe_router.json",
                "core/state/model_control_surface.json",
                "core/state/runtime_governor_contract.json",
            ],
        },
        "summary": {
            "active_in_system": litellm.get("activation_status").and_then(Value::as_str) == Some("active_in_system"),
            "runtime_ok": runtime.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "provider_healthy": provider.get("healthy").cloned().unwrap_or(Value::Null),
            "models_ready": runtime.get("models_ready").cloned().unwrap_or(Value::Null),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_llmfit_routing_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/llmfit_routing_contract.json");
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let manwe_router = read_json_or(&root.join("core/state/manwe_router.json"), json!({}));
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );

    let llmfit = find_tool(&package_enablement, "llmfit");
    let runtime = package_runtime
        .get("surfaces")
        .and_then(|v| v.get("llmfit"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let tool_row = runtime_governor
        .get("capability_lanes")
        .and_then(|v| v.get("tool_activation_and_health"))
        .and_then(|v| v.get("tools"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("llmfit"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let package_runtime_signals = manwe_router
        .get("state_snapshot")
        .and_then(|v| v.get("package_runtime_signals"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let llmfit_signals = json!({
        "backend": package_runtime_signals.get("llmfit_backend").cloned().unwrap_or(Value::Null),
        "recommendation_count": package_runtime_signals.get("llmfit_recommendation_count").cloned().unwrap_or(Value::Null),
        "local_max_params_b": package_runtime_signals.get("llmfit_local_max_params_b").cloned().unwrap_or(Value::Null),
        "top_model_names": package_runtime_signals.get("llmfit_top_model_names").cloned().unwrap_or(Value::Null),
    });

    let payload = json!({
        "schema_version": "arda.llmfit-routing-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_enablement + package_runtime_activation + manwe_router + runtime_governor_contract",
        "tool": {
            "name": "llmfit",
            "repo": llmfit.get("repo").cloned().unwrap_or(Value::Null),
            "repo_url": llmfit.get("repo_url").cloned().unwrap_or(Value::Null),
            "activation_status": llmfit.get("activation_status").cloned().unwrap_or(Value::Null),
            "integration_state": llmfit.get("integration_state").cloned().unwrap_or(Value::Null),
            "runtime_status": runtime.get("status").cloned().unwrap_or(Value::Null),
            "binary_path": llmfit.get("binary_path").cloned().unwrap_or(Value::Null),
        },
        "doctrine": {
            "llmfit_is_an_active_signal_not_a_daemon_runtime": true,
            "llmfit_recommendations_shape_routing_but_do_not_override_sovereign_controls": true,
            "governor_consumes_llmfit_as_policy_input": true,
        },
        "routing_contract": {
            "signal_source": "core/state/manwe_router.json",
            "signals": llmfit_signals,
            "intended_effects": [
                "tune route heuristics from local fit recommendations",
                "bound route classes against available local parameter budgets",
                "keep provider selection auditable instead of implicit",
            ],
            "writes_through": [
                "core/state/manwe_router.json",
                "core/state/model_control_surface.json",
            ],
        },
        "governor_contract": {
            "runtime_governor_surface": "core/state/runtime_governor_contract.json",
            "package_tool_projection_present": tool_row != json!({}),
            "governor_next_action": tool_row.get("next_action").cloned().unwrap_or(Value::Null),
        },
        "summary": {
            "active_signal": llmfit.get("activation_status").and_then(Value::as_str) == Some("active_signal"),
            "runtime_ok": runtime.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "recommendation_count": llmfit_signals.get("recommendation_count").cloned().unwrap_or(Value::Null),
            "backend": llmfit_signals.get("backend").cloned().unwrap_or(Value::Null),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn build_catalog_operationalization(
    source_id: &str,
    lane: &str,
    target_labels: &[&str],
    owner: &str,
    source_row: &Value,
    registry_tools: &[Value],
) -> Value {
    let target_labels = target_labels
        .iter()
        .map(|label| (*label).to_string())
        .collect::<Vec<_>>();
    let target_set = target_labels
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut ranked = Vec::new();
    for row in registry_tools {
        let row_labels = labels_for(row);
        let overlap = row_labels
            .into_iter()
            .filter(|label| target_set.contains(label))
            .collect::<Vec<_>>();
        if overlap.is_empty() {
            continue;
        }
        ranked.push(json!({
            "tool": row.get("tool").cloned().unwrap_or(Value::Null),
            "source_id": row.get("source_id").cloned().unwrap_or(Value::Null),
            "repo_url": row.get("repo_url").cloned().unwrap_or(Value::Null),
            "activation_status": row.get("package_enablement").and_then(|v| v.get("activation_status")).cloned().unwrap_or(Value::Null),
            "integration_lane": row.get("package_enablement").and_then(|v| v.get("integration_lane")).cloned().unwrap_or(Value::Null),
            "policy_confidence": row.get("package_enablement").and_then(|v| v.get("policy_confidence")).cloned().unwrap_or(Value::Null),
            "labels": labels_for(row),
            "matched_labels": overlap,
            "score": candidate_score(row),
            "next_action": row.get("package_enablement").and_then(|v| v.get("next_action")).cloned().unwrap_or(Value::Null),
        }));
    }
    ranked.sort_by(|a, b| {
        let a_score = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let b_score = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.get("tool")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(b.get("tool").and_then(Value::as_str).unwrap_or(""))
            })
    });
    let promote_now = ranked
        .iter()
        .filter(|row| {
            matches!(
                row.get("activation_status").and_then(Value::as_str),
                Some("active_in_system" | "governed_on_demand" | "active_signal")
            )
        })
        .take(6)
        .cloned()
        .collect::<Vec<_>>();
    let research_watch = ranked
        .iter()
        .filter(|row| row.get("activation_status").and_then(Value::as_str) == Some("planned"))
        .take(6)
        .cloned()
        .collect::<Vec<_>>();

    json!({
        "source_id": source_id,
        "title": source_row.get("title").cloned().unwrap_or(Value::Null),
        "url": source_row.get("url").cloned().unwrap_or(Value::Null),
        "lane": lane,
        "status": "operationalized",
        "selection_rule": "match catalog focus labels against already-digested registry tools, then sort by bounded product posture plus ATHENA confidence",
        "ranked_candidates_total": ranked.len(),
        "promote_now": promote_now,
        "research_watchlist": research_watch,
        "recurring_batches": [
            {
                "name": "candidate_extraction",
                "owner": "athena",
                "cadence": "on_absorption_refresh",
                "inputs": [
                    "core/state/github_repo_integration.json",
                    "core/state/source_ecosystem_registry.json",
                ],
                "outputs": [
                    "curated candidate shortlist",
                    "ATHENA follow-up ingest candidates",
                ],
            },
            {
                "name": "portfolio_ranking",
                "owner": owner,
                "cadence": "daily_or_on_major_digest",
                "inputs": [
                    "core/state/source_ecosystem_registry.json",
                    "core/state/source_absorption_pipeline.json",
                ],
                "outputs": [
                    "promote_now portfolio",
                    "signal_only watchlist",
                ],
            },
        ],
    })
}

fn labels_for(row: &Value) -> Vec<String> {
    let text = [
        row.get("tool").and_then(Value::as_str).unwrap_or(""),
        row.get("repo").and_then(Value::as_str).unwrap_or(""),
        row.get("repo_url").and_then(Value::as_str).unwrap_or(""),
        row.get("package_enablement")
            .and_then(|v| v.get("integration_lane"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        row.get("disposition")
            .and_then(|v| v.get("mode"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    ]
    .join(" ")
    .to_ascii_lowercase();

    let mut labels = std::collections::BTreeSet::new();
    if ["agent", "llm", "reason"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("reasoning".to_string());
    }
    if ["coder", "code", "mcp"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("code".to_string());
    }
    if ["tool", "runtime", "router", "provider"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("tooling".to_string());
    }
    if ["workflow", "orchestration", "coord"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("orchestration".to_string());
    }
    if ["framework", "eliza", "agentforge"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("framework".to_string());
    }
    if ["browser", "playwright"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("browser".to_string());
    }
    if ["sandbox", "docker", "container"]
        .iter()
        .any(|token| text.contains(token))
    {
        labels.insert("sandbox".to_string());
    }
    if text.contains("agent") {
        labels.insert("agent".to_string());
    }
    labels.into_iter().collect()
}

fn candidate_score(row: &Value) -> Value {
    let enablement = row
        .get("package_enablement")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let activation = enablement
        .get("activation_status")
        .and_then(Value::as_str)
        .unwrap_or("");
    let integration_state = enablement
        .get("integration_state")
        .and_then(Value::as_str)
        .unwrap_or("");
    let confidence = row
        .get("athena")
        .and_then(|v| v.get("confidence"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let mut score = confidence * 100.0;
    score += match activation {
        "active_in_system" => 40.0,
        "governed_on_demand" => 28.0,
        "active_signal" => 24.0,
        "planned" => 8.0,
        _ => 0.0,
    };
    score += match integration_state {
        "ready_for_activation" => 18.0,
        "observed_only" => 4.0,
        _ => 0.0,
    };
    if row
        .get("runtime")
        .and_then(|v| v.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        score += 12.0;
    }
    json!(((score * 100.0).round() / 100.0))
}

fn source_snapshot(root: &std::path::Path, source_id: &str, label: &str) -> Value {
    let data = read_book_tail(root, source_id);
    json!({
        "source_id": source_id,
        "label": label,
        "title": data.get("title").cloned().unwrap_or(Value::Null),
        "policy_readiness": data.get("policy_readiness").cloned().unwrap_or(Value::Null),
        "confidence": data.get("confidence").cloned().unwrap_or(Value::Null),
        "implementation_brief": data.get("implementation_brief").cloned().unwrap_or(Value::Null),
    })
}

fn read_book_tail(root: &std::path::Path, source_id: &str) -> Value {
    let path = root.join(format!("data/athena/books/{source_id}.jsonl"));
    if !path.exists() {
        return json!({});
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return json!({});
    };
    let mut last = json!({});
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            last = value;
        }
    }
    last.get("data").cloned().unwrap_or_else(|| json!({}))
}

fn crate_row(root: &std::path::Path, name: &str, source_contract: &str) -> Value {
    let crate_root = root.join("crates").join(name);
    json!({
        "crate_name": name,
        "source_contract": source_contract,
        "present": crate_root.exists(),
        "path": rel(&crate_root, root),
        "contract_path": rel(&crate_root.join("src/contract.rs"), root),
        "service_path": rel(&crate_root.join("src/service.rs"), root),
        "readme_path": rel(&crate_root.join("README.md"), root),
        "tests_path": rel(&crate_root.join("tests/contract_smoke.rs"), root),
    })
}

fn find_tool(package_enablement: &Value, tool: &str) -> Value {
    package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some(tool))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn string_set(value: Option<&Value>) -> std::collections::BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}
