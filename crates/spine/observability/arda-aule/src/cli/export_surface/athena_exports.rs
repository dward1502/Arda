#![cfg(feature = "full-cli")]
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::*;

pub(crate) fn export_athena_integration_plan_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/athena_integration_plan.json");

    let athena_runtime = read_json_or(&root.join("core/state/athena_runtime.json"), json!({}));
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let github_integration = read_json_or(
        &root.join("core/state/github_repo_integration.json"),
        json!({}),
    );
    let extension_contract = read_json_or(
        &root.join("core/state/extension_surface_contract.json"),
        json!({}),
    );
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );
    let scrapling_runtime_contract = read_json_or(
        &root.join("core/state/scrapling_runtime_contract.json"),
        json!({}),
    );

    let packages = package_by_tool(&package_enablement);
    let runtimes = package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let github = github_by_tool(&github_integration);
    let extension_sources = extension_contract
        .get("framework_sources")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let athena_counts = athena_runtime
        .get("knowledge")
        .and_then(|value| value.get("counts"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let crawl4ai = packages
        .get("crawl4ai")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let litellm = packages
        .get("litellm")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let playwright = packages
        .get("playwright-mcp")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let nanoclaw = packages
        .get("nanoclaw")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let scrapling = github.get("crawl4ai").cloned().unwrap_or_else(|| json!({}));
    let scrapling_summary = scrapling_runtime_contract
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let scrapling_posture = scrapling_runtime_contract
        .get("current_posture")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.athena-integration-plan.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_runtime + package_enablement + github_repo_integration + extension_surface_contract",
        "campaign": {
            "name": "ATHENA sovereign integration execution",
            "owner": "athena_prometheus",
            "mission": "Turn ATHENA digestion into bounded runtime activation, implementation briefs, and downstream execution contracts across ingestion, frameworks, and workflow surfaces.",
        },
        "doctrine": {
            "crawl4ai_remains_live_primary_until_scrapling_runtime_is_bounded": true,
            "scrapling_is_preferred_long_term_ingest_direction": true,
            "deep_digestion_must_surface_in_human_and_machine_books": true,
            "policy_ready_evidence_should_promote_into_implementation_contracts": true,
            "framework_patterns_must_land_in_sovereign_crate_and_state_boundaries": true,
            "workstation_is_canonical_athena_execution_authority": true,
            "laptop_is_operator_ingress_and_optional_fallback": true,
            "task_emission_must_be_receipt_backed": true,
            "source_provenance_must_survive_task_emission": true,
            "ui_optional": true,
        },
        "current_posture": {
            "deep_queue_recent": athena_counts.get("deep_queue_recent").cloned().unwrap_or(Value::Null),
            "policy_ready_recent": athena_counts.get("policy_ready_recent").cloned().unwrap_or(Value::Null),
            "reference_only_recent": athena_counts.get("reference_only_recent").cloned().unwrap_or(Value::Null),
            "execution_authority": athena_runtime.get("status").and_then(|value| value.get("execution_authority")).cloned().unwrap_or(Value::Null),
            "execution_posture": athena_runtime.get("status").and_then(|value| value.get("execution_posture")).cloned().unwrap_or(Value::Null),
            "source_provenance_coverage_ratio": athena_runtime.get("status").and_then(|value| value.get("source_provenance_coverage_ratio")).cloned().unwrap_or(Value::Null),
            "task_emission_receipts_total": athena_runtime.get("status").and_then(|value| value.get("task_emission_receipts_total")).cloned().unwrap_or(Value::Null),
            "crawl_primary": {
                "tool": "crawl4ai",
                "activation_status": crawl4ai.get("activation_status").cloned().unwrap_or(Value::Null),
                "runtime_status": runtimes.get("crawl4ai").and_then(|v| v.get("status")).cloned().unwrap_or(Value::Null),
                "next_action": crawl4ai.get("next_action").cloned().unwrap_or(Value::Null),
            },
            "crawl_preferred_future": {
                "tool": "scrapling",
                "source_id": if scrapling.get("athena").and_then(|v| v.get("book_ref")).is_some() { json!("src_df11630e") } else { Value::Null },
                "integration_status": "prototype_integrated",
                "implementation_state": scrapling_posture.get("implementation_state").cloned().unwrap_or_else(|| json!("requires_bounded_runtime_contract")),
                "configured_primary": scrapling_summary.get("configured_primary").cloned().unwrap_or(Value::Null),
                "shim_backed": scrapling_summary.get("shim_backed").cloned().unwrap_or(Value::Null),
            },
            "downstream_runtime_signals": {
                "litellm": {
                    "activation_status": litellm.get("activation_status").cloned().unwrap_or(Value::Null),
                    "runtime_status": runtimes.get("litellm").and_then(|v| v.get("status")).cloned().unwrap_or(Value::Null),
                },
                "playwright_mcp": {
                    "activation_status": playwright.get("activation_status").cloned().unwrap_or(Value::Null),
                    "runtime_status": runtimes.get("playwright_mcp").and_then(|v| v.get("status")).cloned().unwrap_or(Value::Null),
                },
                "nanoclaw": {
                    "activation_status": nanoclaw.get("activation_status").cloned().unwrap_or(Value::Null),
                    "runtime_status": runtimes.get("nanoclaw").and_then(|v| v.get("status")).cloned().unwrap_or(Value::Null),
                },
            },
        },
        "integration_lanes": [
            {
                "lane": "ingest_runtime",
                "owner": "athena",
                "goal": "Stabilize dual-provider crawl ingestion while preserving a clear sovereign default.",
                "current_primary": "crawl4ai",
                "preferred_future": "scrapling",
                "write_through": [
                    "crates/arda-athena/src/ingest.rs",
                    "crates/arda-cli/src/main.rs",
                    "core/state/package_enablement.json",
                    "core/state/package_runtime_activation.json",
                    "core/state/scrapling_runtime_contract.json",
                ],
                "execution_frontier": [
                    "activate crawl4ai as verified live ingest runtime",
                    "materialize scrapling runtime contract beyond shim-only invocation",
                    "promote explicit provider-selection policy for ATHENA crawl flows",
                ],
            },
            {
                "lane": "evidence_to_implementation",
                "owner": "athena_prometheus",
                "goal": "Convert deep/policy-ready ATHENA evidence into reusable implementation briefs and deterministic planning tasks.",
                "write_through": [
                    "data/athena/books/",
                    "human/library/athena/sources/",
                    "core/projects/tasks/queue.jsonl",
                    "core/state/github_repo_integration.json",
                ],
                "execution_frontier": [
                    "emit implementation briefs for policy-ready sources",
                    "use ATHENA planning-task generation to seed bounded execution work",
                    "separate reference-only framework evidence from promotion-ready contracts",
                ],
            },
            {
                "lane": "framework_absorption",
                "owner": "prometheus_apollo_hermes",
                "goal": "Absorb useful framework patterns into sovereign crate, adapter, and workflow contracts instead of external shells.",
                "framework_targets": {
                    "agentforge": extension_sources.get("agentforge").and_then(|v| v.get("targets")).cloned().unwrap_or_else(|| json!([])),
                    "eliza": extension_sources.get("eliza").and_then(|v| v.get("targets")).cloned().unwrap_or_else(|| json!([])),
                },
                "write_through": [
                    "core/state/extension_surface_contract.json",
                    "core/state/crate_spawn_contract.json",
                    "core/state/communication_adapter_contract.json",
                    "core/state/operations_flow.json",
                ],
                "execution_frontier": [
                    "promote AgentForge learnings into crate-spawn and workflow recipes",
                    "promote eliza learnings into HERMES adapter and embodiment contracts",
                    "keep framework-first identity out of core doctrine",
                ],
            },
            {
                "lane": "runtime_and_execution_handoff",
                "owner": "athena_manwe_apollo",
                "goal": "Bind ATHENA outputs to active runtime and execution surfaces so evidence immediately affects routing and workflows.",
                "governor_inputs": runtime_governor.get("input_surfaces").cloned().unwrap_or_else(|| json!({})),
                "write_through": [
                    "core/state/runtime_governor_contract.json",
                    "core/state/package_enablement.json",
                    "core/state/package_runtime_activation.json",
                    "core/state/autonomy_resume.json",
                ],
                "execution_frontier": [
                    "keep litellm as the live reasoning/router integration lane",
                    "promote crawl runtime readiness into regular ATHENA ingest operation",
                    "surface generated implementation tasks into APOLLO and PROMETHEUS execution fronts",
                ],
            },
        ],
        "recommended_execution_sequence": [
            "finish crawl4ai activation so ATHENA ingest runtime is continuously usable",
            "materialize a Scrapling runtime contract and provider policy before switching live primary crawl direction",
            "emit implementation briefs from policy-ready ATHENA sources so evidence becomes executable work",
            "promote framework digestion into crate/adapter/workflow contracts instead of reference-only notes",
            "bind ATHENA-generated work directly into runtime governor and APOLLO execution surfaces",
        ],
        "summary": {
            "integration_lanes_total": 4,
            "policy_ready_recent": athena_counts.get("policy_ready_recent").cloned().unwrap_or(Value::Null),
            "reference_only_recent": athena_counts.get("reference_only_recent").cloned().unwrap_or(Value::Null),
            "crawl4ai_frontier_open": crawl4ai.get("activation_status").and_then(Value::as_str) == Some("activation_frontier"),
            "litellm_active": litellm.get("activation_status").and_then(Value::as_str) == Some("active_in_system"),
            "scrapling_contract_bounded": scrapling_posture.get("implementation_state").and_then(Value::as_str) == Some("bounded_contract_materialized"),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_async_user_intake_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/async_user_intake_contract.json");
    let payload = json!({
        "schema_version": "arda.async-user-intake-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "hermes_inbound + athena_ingest + project_task_queue",
        "mission": {
            "name": "Async User Intake",
            "goal": "Detect link-bearing or source-bearing inbound messages and hand them into ATHENA background processing while foreground conversation continues.",
        },
        "doctrine": {
            "foreground_conversation_must_not_block_on_intake": true,
            "only_bounded_source_like_inputs_auto_handoff": true,
            "detected_inputs_must_record_sender_source_and_time": true,
            "athena_owns_background_ingest_and_deepening": true,
            "planning_tasks_may_emit_after_policy_ready_evidence_exists": true,
            "intake_confidence_ladder_guides_follow_on_routing": true,
            "continuity_contract_keeps_background_work_linked_to_operator_context": true,
        },
        "input_surface": {
            "hermes_messages": "data/hermes/messages.jsonl",
            "eligible_direction": "inbound",
            "trigger_signals": ["url_detected", "source_like_payload", "link_drop_during_chat"],
            "confidence_ladder": "core/state/intake_confidence_ladder.json",
            "continuity_contract": "core/state/agent_continuity_contract.json",
        },
        "handoff_sequence": [
            "detect inbound candidate",
            "classify with intake confidence ladder",
            "emit bounded async intake task",
            "run ATHENA ingest",
            "run ATHENA deep analysis",
            "generate planning tasks if evidence is promotable",
            "record runtime and queue reconciliation surfaces",
        ],
        "governor_boundary": {
            "no_direct_execution_from_chat_classifier": true,
            "only_url_or_source_payloads_auto_handoff": true,
            "non_source_conversation_stays_in_chat_lane": true,
            "manual_override_remains_possible": true,
        },
        "summary": {
            "trigger_signals_total": 3,
            "handoff_steps_total": 7,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_athena_digest_pipeline_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/athena_digest_pipeline.json");
    let queue_rows = read_jsonl_objects_local(&root.join("core/projects/tasks/queue.jsonl"));
    let latest_tasks = latest_by_id(&queue_rows, "id");

    let plan_map = read_json_or(&root.join("core/state/plan_map.json"), json!({}));
    let project_task_executor = read_json_or(
        &root.join("core/state/project_task_executor.json"),
        json!({}),
    );
    let human_plan = read_json_or(
        &root.join("core/state/human_corpus_digest_plan.json"),
        json!({}),
    );
    let human_tasks = read_json_or(
        &root.join("core/state/human_corpus_digest_tasks.json"),
        json!({}),
    );
    let extraction_registry = read_json_or(
        &root.join("core/state/human_corpus_extraction_registry.json"),
        json!({}),
    );
    let human_reconciliation = read_json_or(
        &root.join("core/state/human_corpus_digest_reconciliation.json"),
        json!({}),
    );
    let source_pipeline = read_json_or(
        &root.join("core/state/source_absorption_pipeline.json"),
        json!({}),
    );
    let source_execution = read_json_or(
        &root.join("core/state/source_absorption_execution.json"),
        json!({}),
    );
    let source_portfolio = read_json_or(
        &root.join("core/state/source_absorption_portfolio.json"),
        json!({}),
    );
    let source_autopilot = read_json_or(
        &root.join("core/state/source_absorption_autopilot.json"),
        json!({}),
    );
    let source_executor = read_json_or(
        &root.join("core/state/source_absorption_executor.json"),
        json!({}),
    );

    let extraction_groups = latest_by_id(
        &extraction_registry
            .get("groups")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        "group_id",
    );
    let plan_nodes = plan_nodes_by_owner(&plan_map);

    let mut human_entries = Vec::new();
    for group in human_plan
        .get("plan_groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let group_id = group
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if group_id.is_empty() {
            continue;
        }
        let task_id = stable_human_task_id(group_id);
        let task = latest_tasks.get(&task_id);
        let extraction_group = extraction_groups.get(group_id);
        let (stage, execution_ready) = human_stage(task, extraction_group);
        let owner = group
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let source_ids = group
            .get("sources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|row| row.get("source_id").cloned())
            .collect::<Vec<_>>();
        human_entries.push(json!({
            "pipeline_id": format!("human:{group_id}"),
            "pipeline_family": "human_corpus_digest",
            "task_id": task_id,
            "group_id": group_id,
            "title": group.get("task_title").cloned().unwrap_or(Value::Null),
            "owner": owner,
            "lane": group.get("lane").cloned().unwrap_or(Value::Null),
            "source_ids": source_ids,
            "plan_binding": {
                "plan_id": group_id,
                "plan_type": "human_corpus_digest_group",
                "plan_surface": "core/state/human_corpus_digest_plan.json",
                "group_contract_surface": extraction_group.and_then(|v| v.get("group_path")).cloned().unwrap_or(Value::Null),
                "owner_plan_node": plan_nodes.get(owner).and_then(|v| v.get("human_plan_path")).cloned().unwrap_or(Value::Null),
            },
            "executor_binding": {
                "rule_id": "human_corpus_digest_handoff",
                "executor_surface": "core/state/project_task_executor.json",
                "receipt_surface": "core/state/athena_digest_pipeline.json",
            },
            "latest_task": task.cloned().unwrap_or(Value::Null),
            "execution_stage": stage,
            "execution_ready": execution_ready,
        }));
    }

    let portfolio_sources = latest_by_id(
        &source_portfolio
            .get("sources")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        "source_id",
    );
    let mut source_entries = Vec::new();
    for candidate in source_pipeline
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if candidate.get("disposition").and_then(Value::as_str) != Some("promote_now") {
            continue;
        }
        let source_id = candidate
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if source_id.is_empty() {
            continue;
        }
        let subsystem_targets = candidate
            .get("subsystem_targets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let mut anchor_tasks = Vec::new();
        for subsystem in subsystem_targets.iter().take(2) {
            if let Some(task) = latest_tasks.get(&stable_absorption_task_id(source_id, subsystem)) {
                anchor_tasks.push(task.clone());
            }
        }
        let downstream_rows = latest_tasks
            .values()
            .filter(|row| {
                row.get("meta")
                    .and_then(|v| v.get("origin"))
                    .and_then(Value::as_str)
                    == Some("source_absorption_executor")
                    && row
                        .get("meta")
                        .and_then(|v| v.get("source_id"))
                        .and_then(Value::as_str)
                        == Some(source_id)
                    && matches!(
                        row.get("status").and_then(Value::as_str),
                        Some("queued" | "in_progress" | "completed" | "blocked")
                    )
            })
            .take(6)
            .cloned()
            .collect::<Vec<_>>();
        let portfolio_row = portfolio_sources.get(source_id);
        let (stage, execution_ready) = absorption_stage(
            !anchor_tasks.is_empty(),
            portfolio_row.is_some(),
            !downstream_rows.is_empty(),
        );
        source_entries.push(json!({
            "pipeline_id": format!("source:{source_id}"),
            "pipeline_family": "source_absorption",
            "source_id": source_id,
            "title": candidate.get("title").cloned().unwrap_or(Value::Null),
            "owner_targets": subsystem_targets.iter().take(2).cloned().collect::<Vec<_>>(),
            "domain": candidate.get("domain").cloned().unwrap_or(Value::Null),
            "plan_binding": {
                "plan_id": source_id,
                "plan_type": "source_absorption_candidate",
                "plan_surface": "core/state/source_absorption_pipeline.json",
                "portfolio_surface": if portfolio_row.is_some() { json!("core/state/source_absorption_portfolio.json") } else { Value::Null },
            },
            "executor_binding": {
                "rule_id": "absorption_pipeline_autopilot",
                "executor_surface": "core/state/project_task_executor.json",
                "receipt_surface": "core/state/athena_digest_pipeline.json",
            },
            "latest_anchor_tasks": anchor_tasks,
            "latest_downstream_tasks": downstream_rows,
            "execution_stage": stage,
            "execution_ready": execution_ready,
        }));
    }

    let mut entries = human_entries;
    entries.extend(source_entries);
    entries.sort_by(|a, b| {
        (
            a.get("pipeline_family")
                .and_then(Value::as_str)
                .unwrap_or(""),
            a.get("owner").and_then(Value::as_str).unwrap_or(""),
            a.get("group_id")
                .or_else(|| a.get("source_id"))
                .and_then(Value::as_str)
                .unwrap_or(""),
        )
            .cmp(&(
                b.get("pipeline_family")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                b.get("owner").and_then(Value::as_str).unwrap_or(""),
                b.get("group_id")
                    .or_else(|| b.get("source_id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ))
    });

    let payload = json!({
        "schema_version": "arda.athena-digest-pipeline.v1",
        "generated_at_utc": now_utc(),
        "authority": "human_corpus_digest_plan + human_corpus_extraction_registry + source_absorption_pipeline + source_absorption_portfolio + project_task_queue",
        "doctrine": {
            "digest_outputs_bind_to_bounded_plans_before_execution": true,
            "executor_eligibility_is_explicit": true,
            "receipts_live_in_shared_state": true,
        },
        "summary": {
            "pipeline_entries_total": entries.len(),
            "human_corpus_entries_total": entries.iter().filter(|row| row.get("pipeline_family").and_then(Value::as_str) == Some("human_corpus_digest")).count(),
            "source_absorption_entries_total": entries.iter().filter(|row| row.get("pipeline_family").and_then(Value::as_str) == Some("source_absorption")).count(),
            "execution_ready_total": entries.iter().filter(|row| row.get("execution_ready").and_then(Value::as_bool) == Some(true)).count(),
            "contract_ready_total": entries.iter().filter(|row| row.get("execution_stage").and_then(Value::as_str) == Some("contract_ready")).count(),
            "downstream_bound_total": entries.iter().filter(|row| row.get("execution_stage").and_then(Value::as_str) == Some("downstream_bound")).count(),
        },
        "surfaces": {
            "plan_map": "core/state/plan_map.json",
            "project_task_executor": "core/state/project_task_executor.json",
            "human_corpus_digest_plan": "core/state/human_corpus_digest_plan.json",
            "human_corpus_digest_tasks": "core/state/human_corpus_digest_tasks.json",
            "human_corpus_extraction_registry": "core/state/human_corpus_extraction_registry.json",
            "human_corpus_digest_reconciliation": "core/state/human_corpus_digest_reconciliation.json",
            "source_absorption_pipeline": "core/state/source_absorption_pipeline.json",
            "source_absorption_execution": "core/state/source_absorption_execution.json",
            "source_absorption_portfolio": "core/state/source_absorption_portfolio.json",
            "source_absorption_autopilot": "core/state/source_absorption_autopilot.json",
            "source_absorption_executor": "core/state/source_absorption_executor.json",
        },
        "project_executor_summary": project_task_executor.get("summary").cloned().unwrap_or_else(|| json!({})),
        "human_corpus_summary": {
            "plan": human_plan.get("summary").cloned().unwrap_or_else(|| json!({})),
            "tasks": human_tasks.get("summary").cloned().unwrap_or_else(|| json!({})),
            "extraction_registry": extraction_registry.get("summary").cloned().unwrap_or_else(|| json!({})),
            "reconciliation": human_reconciliation.get("summary").cloned().unwrap_or_else(|| json!({})),
        },
        "source_absorption_summary": {
            "pipeline": source_pipeline.get("summary").cloned().unwrap_or_else(|| json!({})),
            "execution": source_execution.get("summary").cloned().unwrap_or_else(|| json!({})),
            "portfolio": source_portfolio.get("summary").cloned().unwrap_or_else(|| json!({})),
            "autopilot": source_autopilot.get("summary").cloned().unwrap_or_else(|| json!({})),
            "executor": source_executor.get("summary").cloned().unwrap_or_else(|| json!({})),
        },
        "entries": entries,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "pipeline_entries_total": payload["summary"]["pipeline_entries_total"],
    }))
}

pub(crate) fn export_autonomy_resume_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/autonomy_resume.json");
    let manifest = read_json_or(&root.join("core/metrics/manifest.json"), json!({}));
    let tasks = latest_task_state(&read_jsonl_objects_local(
        &root.join("core/projects/tasks/queue.jsonl"),
    ));
    let queued = select_tasks(&tasks, "queued");
    let blocked = select_tasks(&tasks, "blocked");
    write_compact_queue_summary(&root, &tasks)?;

    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let task_boundaries = read_json_or(
        &root.join("core/state/task_agent_boundaries.json"),
        json!({}),
    );
    let project_intake_governance = read_json_or(
        &root.join("core/state/project_intake_governance.json"),
        json!({}),
    );
    let athena_imported_capability_roadmap = read_json_or(
        &root.join("core/state/athena_imported_capability_roadmap.json"),
        json!({}),
    );
    let hermes_imported_capability_roadmap = read_json_or(
        &root.join("core/state/hermes_imported_capability_roadmap.json"),
        json!({}),
    );
    let hermes_discord_runtime = read_json_or(
        &root.join("core/state/hermes_discord_runtime.json"),
        json!({}),
    );
    let flywheel_packet_runtime = read_json_or(
        &root.join("core/state/flywheel_packet_runtime.json"),
        json!({}),
    );
    let imported_tool_fit_decision_memo = read_json_or(
        &root.join("core/state/imported_tool_fit_decision_memo.json"),
        json!({}),
    );

    let operator_action_total = operator_actions
        .get("summary")
        .and_then(|v| v.get("human_needed_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let active_frontier = json!({
        "campaign": "ATHENA/HADES integration continuation",
        "focus": "restart-safe continuity plus edge identity enrollment and bounded package/runtime posture",
        "comfortable": blocked.is_empty() && operator_action_total == 0,
    });
    let next_actions = queued
        .iter()
        .take(3)
        .map(|task| {
            json!({
                "owner": task.get("owner").cloned().unwrap_or(Value::Null),
                "task": task.get("title").cloned().unwrap_or(Value::Null),
                "task_id": task.get("id").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let pending_queue = queued
        .iter()
        .take(8)
        .map(|task| {
            json!({
                "id": task.get("id").cloned().unwrap_or(Value::Null),
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "owner": task.get("owner").cloned().unwrap_or(Value::Null),
                "priority": task.get("priority").cloned().unwrap_or(Value::Null),
                "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let blockers = blocked
        .iter()
        .take(5)
        .map(|task| {
            json!({
                "id": task.get("id").cloned().unwrap_or(Value::Null),
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "owner": task.get("owner").cloned().unwrap_or(Value::Null),
                "notes": task.get("notes").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();

    let latest_evidence_surfaces = build_autonomy_latest_evidence(&root);

    let payload = json!({
        "schema_version": "arda.autonomy-resume.v1",
        "generated_at_utc": now_utc(),
        "authority": "restart_resilience_capsule",
        "current_campaign": active_frontier.get("campaign").cloned().unwrap_or(Value::Null),
        "active_frontier": active_frontier,
        "next_actions": next_actions,
        "unresolved_blockers": blockers,
        "latest_evidence_surfaces": latest_evidence_surfaces,
        "last_stable_snapshot": {
            "metrics_snapshot_id": manifest.get("snapshot_id").cloned().unwrap_or(Value::Null),
            "metrics_history_path": manifest.get("history_path").cloned().unwrap_or(Value::Null),
            "athena_runtime_generated_at_utc": read_json_or(&root.join("core/state/athena_runtime.json"), json!({})).get("generated_at_utc").cloned().unwrap_or(Value::Null),
            "hades_lifecycle_generated_at_utc": read_json_or(&root.join("core/state/hades_lifecycle.json"), json!({})).get("generated_at_utc").cloned().unwrap_or(Value::Null),
        },
        "restart_confidence": {
            "can_resume_without_human_restatement": true,
            "score": 0.93,
            "basis": [
                "root protocol and sovereign realm surfaces are stable",
                "task ledger contains queued and completed frontier work",
                "core/state contains refreshed framework and runtime exports",
                "metrics manifest anchors latest stable snapshot",
                "task/agent boundary capsules prevent cross-task context bleed",
                "operator actions and edge identity reconciliation expose human-needed frontier work explicitly",
            ],
        },
        "machine_truth": {
            "queued_tasks_top": pending_queue,
            "blocked_tasks_top": blockers,
            "operator_action_summary": operator_actions.get("summary").cloned().unwrap_or(Value::Null),
            "active_boundary_summary": task_boundaries.get("summary").cloned().unwrap_or(Value::Null),
            "project_intake_governance_summary": project_intake_governance.get("summary").cloned().unwrap_or(Value::Null),
            "flywheel_packet_summary": flywheel_packet_runtime.get("summary").cloned().unwrap_or(Value::Null),
            "flywheel_packet_runtime": {
                "path": "core/state/flywheel_packet_runtime.json",
                "schema_version": flywheel_packet_runtime.get("schema_version").cloned().unwrap_or(Value::Null),
                "generated_at_utc": flywheel_packet_runtime.get("generated_at_utc").cloned().unwrap_or(Value::Null),
                "next_ready_packet": flywheel_packet_runtime.get("packets").and_then(Value::as_array).into_iter().flatten().find(|packet| packet.get("readiness").and_then(Value::as_str) == Some("ready")).cloned().unwrap_or(Value::Null),
            },
            "rank2_capability_summary": {
                "athena_posture": athena_imported_capability_roadmap.get("decision").and_then(|v| v.get("posture")).cloned().unwrap_or(Value::Null),
                "hermes_posture": hermes_imported_capability_roadmap.get("decision").and_then(|v| v.get("posture")).cloned().unwrap_or(Value::Null),
                "hermes_discord_delivery_posture": hermes_discord_runtime.get("summary").and_then(|v| v.get("delivery_posture")).cloned().unwrap_or(Value::Null),
                "tool_fit_tools_total": imported_tool_fit_decision_memo.get("tools").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
            },
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn write_compact_queue_summary(root: &Path, tasks: &[Value]) -> Result<()> {
    let out_path = root.join("core/state/queue_summary.json");
    let raw_tasks = read_jsonl_objects_local(&root.join("core/projects/tasks/queue.jsonl"));
    let runtime_queue = read_jsonl_objects_local(&root.join("core/queue/queue.jsonl"));
    let mut recent_tasks = tasks.to_vec();
    recent_tasks.reverse();
    recent_tasks.truncate(32);
    recent_tasks.reverse();

    let open_total = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.get("status").and_then(Value::as_str),
                Some("pending" | "queued" | "in_progress" | "running" | "active")
            )
        })
        .count();
    let open_tasks = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.get("status").and_then(Value::as_str),
                Some("pending" | "queued" | "in_progress" | "running" | "active")
            )
        })
        .take(32)
        .cloned()
        .collect::<Vec<_>>();

    let mut recent_runtime_queue = runtime_queue
        .iter()
        .rev()
        .take(32)
        .cloned()
        .collect::<Vec<_>>();
    recent_runtime_queue.reverse();

    let payload = json!({
        "schema_version": "arda.core.state.v1",
        "generated_at_utc": now_utc(),
        "authority": "queue_summary_projection",
        "agent_reading_policy": {
            "default_surface": "core/state/queue_active.json",
            "summary_surface": "core/state/queue_summary.json",
            "raw_ledger": "core/projects/tasks/queue.jsonl",
            "raw_ledger_role": "compacted_active_ledger_and_append_target",
            "guidance": "Agents should read queue_active.json for active task selection, then queue_summary.json for counts. Do not bulk-read queue.jsonl; open it only for exact id evidence, append validation, or targeted append."
        },
        "project_tasks": {
            "total_effective": tasks.len(),
            "raw_ledger_rows_total": raw_tasks.len(),
            "history_rows_total": raw_tasks.len().saturating_sub(tasks.len()),
            "counts_by_status": count_field(tasks, "status"),
            "counts_by_owner": count_field(tasks, "owner"),
            "counts_by_priority": count_field(tasks, "priority"),
            "open_total": open_total,
            "open_compact_limit": 32,
            "open_compact": open_tasks.iter().map(compact_task_row).collect::<Vec<_>>(),
            "recent_compact": recent_tasks.iter().map(compact_task_row).collect::<Vec<_>>(),
        },
        "runtime_queue": {
            "counts_by_status": count_field(&runtime_queue, "status"),
            "counts_by_owner": count_field(&runtime_queue, "owner"),
            "recent_compact": recent_runtime_queue.iter().map(compact_runtime_queue_row).collect::<Vec<_>>(),
        },
        "arda_hints": {
            "primary_panel": "task_board",
            "boardroom_section": "execution_queue",
            "alert_on_queued_tasks": tasks.iter().any(|task| task.get("status").and_then(Value::as_str) == Some("queued")),
            "alert_on_failed_tasks": tasks.iter().any(|task| task.get("result").and_then(Value::as_str) == Some("failed")),
        }
    });

    write_pretty_json(&out_path, &payload)
}

fn count_field(rows: &[Value], field: &str) -> Value {
    let mut counts = Map::new();
    for row in rows {
        let key = row
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let current = counts.get(&key).and_then(Value::as_u64).unwrap_or(0);
        counts.insert(key, json!(current + 1));
    }
    Value::Object(counts)
}

fn compact_task_row(task: &Value) -> Value {
    json!({
        "id": task.get("id").cloned().unwrap_or(Value::Null),
        "title": task.get("title").cloned().unwrap_or(Value::Null),
        "owner": task.get("owner").cloned().unwrap_or(Value::Null),
        "priority": task.get("priority").cloned().unwrap_or(Value::Null),
        "status": task.get("status").cloned().unwrap_or(Value::Null),
        "result": task.get("result").cloned().unwrap_or(Value::Null),
        "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
        "completed_at_utc": task.get("completed_at_utc").cloned().unwrap_or(Value::Null),
        "origin": task.get("meta").and_then(|meta| meta.get("origin")).cloned().unwrap_or(Value::Null),
        "scope": task.get("meta").and_then(|meta| meta.get("scope")).cloned().unwrap_or(Value::Null),
    })
}

fn compact_runtime_queue_row(row: &Value) -> Value {
    json!({
        "id": row.get("id").or_else(|| row.get("task_id")).cloned().unwrap_or(Value::Null),
        "owner": row.get("owner").cloned().unwrap_or(Value::Null),
        "status": row.get("status").cloned().unwrap_or(Value::Null),
        "queued_at_utc": row.get("queued_at_utc").cloned().unwrap_or(Value::Null),
    })
}

pub(crate) fn export_human_corpus_digest_plan_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/human_corpus_digest_plan.json");
    let registry = read_json_or(
        &root.join("core/state/human_corpus_registry.json"),
        json!({}),
    );
    let mut grouped = BTreeMap::new();

    for (wave_type, path) in [
        ("text", root.join("core/state/human_corpus_wave.json")),
        ("document", root.join("core/state/human_document_wave.json")),
    ] {
        let payload = read_json_or(&path, json!({}));
        for row in payload
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if !row.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                continue;
            }
            let source_id = preview_source_id(row);
            let canonical_path = row
                .get("canonical_path")
                .or_else(|| row.get("path"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let (lane, group_id, owner, task_title) = classify_human_corpus_group(canonical_path);
            let bucket = grouped.entry(group_id.to_string()).or_insert_with(|| {
                json!({
                    "group_id": group_id,
                    "lane": lane,
                    "owner": owner,
                    "task_title": task_title,
                    "sources": [],
                })
            });
            if let Some(sources) = bucket.get_mut("sources").and_then(Value::as_array_mut) {
                sources.push(json!({
                    "source_id": if source_id.is_empty() { Value::Null } else { Value::from(source_id) },
                    "canonical_path": if canonical_path.is_empty() { Value::Null } else { Value::from(canonical_path) },
                    "wave_type": wave_type,
                }));
            }
        }
    }

    let plan_groups = grouped.into_values().collect::<Vec<_>>();
    let payload = json!({
        "schema_version": "arda.human-corpus-digest-plan.v1",
        "generated_at_utc": now_utc(),
        "authority": "bounded_human_corpus_digest_planner",
        "doctrine": {
            "human_corpus_notes_are_not_direct_implementation_authority": true,
            "first_output_is_extraction_and_contract_formalization": true,
            "policy_ready_promotion_requires_follow_on_evidence": true,
        },
        "source_surfaces": {
            "registry": "core/state/human_corpus_registry.json",
            "text_wave": "core/state/human_corpus_wave.json",
            "document_wave": "core/state/human_document_wave.json",
        },
        "summary": {
            "groups_total": plan_groups.len(),
            "sources_total": plan_groups.iter().map(|group| group.get("sources").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)).sum::<usize>(),
            "high_priority_total": registry.get("summary").and_then(|v| v.get("high_priority_total")).cloned().unwrap_or(Value::Null),
        },
        "plan_groups": plan_groups,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "groups_total": payload.get("summary").and_then(|v| v.get("groups_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_human_corpus_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/human_corpus_registry.json");
    let local_sets = vec![
        ("human_notes", root.join("human/notes")),
        ("human_plans", root.join("human/plans")),
        ("system_docs", root.join("docs")),
        ("human_summaries", root.join("human/summaries")),
    ];
    let numenor_root = numenor_prime_root();
    let external_sets = vec![
        ("elros_vault", numenor_root.join("Elros")),
        (
            "numenor_architecture",
            numenor_root.join("Operations/Architecture"),
        ),
        (
            "numenor_reports_system",
            numenor_root.join("Operations/Reports/system"),
        ),
        (
            "numenor_reports_ceo_internal",
            numenor_root.join("Operations/Reports/CEO_INTERNAL"),
        ),
    ];

    let mut summaries = Vec::new();
    let mut entries = Vec::new();
    for (root_name, base) in local_sets.into_iter().chain(external_sets) {
        let (summary, items) = collect_human_corpus_root(&root, root_name, &base, 400);
        summaries.push(summary);
        entries.extend(items);
    }

    let payload = json!({
        "schema_version": "arda.human-corpus-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "human_and_numenor_corpus_inventory",
        "roots": summaries,
        "top_ingest_candidates": top_human_corpus_candidates(&entries, 18),
        "crate_idea_candidates": entries
            .iter()
            .filter(|row| row.get("root_id").and_then(Value::as_str) == Some("human_notes") && row.get("priority").and_then(Value::as_str) == Some("high"))
            .take(12)
            .cloned()
            .collect::<Vec<_>>(),
        "summary": {
            "roots_total": summaries.len(),
            "files_total": summaries.iter().map(|row| row.get("files_total").and_then(Value::as_u64).unwrap_or(0) as usize).sum::<usize>(),
            "markdown_total": summaries.iter().map(|row| row.get("markdown_total").and_then(Value::as_u64).unwrap_or(0) as usize).sum::<usize>(),
            "high_priority_total": summaries.iter().map(|row| row.get("high_priority_total").and_then(Value::as_u64).unwrap_or(0) as usize).sum::<usize>(),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "files_total": payload["summary"]["files_total"],
    }))
}

pub(crate) fn export_intake_confidence_ladder_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/intake_confidence_ladder.json");
    let external_brief = read_json_or(
        &root.join("core/state/external_absorption_brief.json"),
        json!({}),
    );
    let recent = external_brief
        .get("recent_intake_status")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let classification_map = recent
        .into_iter()
        .filter_map(|row| {
            let source_id = row.get("source_id").and_then(Value::as_str)?;
            Some((
                source_id.to_string(),
                json!({
                    "label": row.get("label").cloned().unwrap_or(Value::Null),
                    "status": row.get("status").cloned().unwrap_or(Value::Null),
                    "reason": row.get("reason").cloned().unwrap_or(Value::Null),
                }),
            ))
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let payload = json!({
        "schema_version": "arda.intake-confidence-ladder.v1",
        "generated_at_utc": now_utc(),
        "authority": "external_absorption_brief + async_intake",
        "mission": {
            "goal": "Classify newly ingested external sources into a confidence/action ladder so ATHENA and operators know how aggressively to route follow-on work.",
        },
        "ladder": [
            {"level": "policy_ready_anchor", "route_to": "promote_and_execute", "meaning": "validated source with direct implementation authority"},
            {"level": "comparison_signal", "route_to": "compare_and_extract", "meaning": "strong directional source that should inform design but not directly control it"},
            {"level": "product_legibility_signal", "route_to": "productize_operator_surface", "meaning": "source is primarily useful for packaging, UX legibility, or operator clarity"},
            {"level": "crawl_policy_signal", "route_to": "athena_ingestion_policy", "meaning": "source primarily affects crawl/runtime collection policy"},
            {"level": "routing_signal", "route_to": "manwe_model_strategy", "meaning": "source primarily affects routing/model selection logic"},
            {"level": "blocked_fetch_artifact", "route_to": "fetch_retry_or_hold", "meaning": "source body was not actually captured; do not treat it as evidence yet"},
            {"level": "unknown_new_source", "route_to": "athena_digest_first", "meaning": "new intake without prior comparison posture; digest before promotion"},
        ],
        "known_recent_sources": classification_map,
        "summary": {
            "ladder_levels_total": 7,
            "known_recent_sources_total": classification_map.len(),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_human_corpus_extraction_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/human_corpus_extraction_registry.json");
    let group_dir = root.join("core/state/human_corpus_groups");
    let plan = read_json_or(
        &root.join("core/state/human_corpus_digest_plan.json"),
        json!({}),
    );

    std::fs::create_dir_all(&group_dir)?;
    let mut rows = Vec::new();
    for group in plan
        .get("plan_groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let group_id = group
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if group_id.is_empty() {
            continue;
        }
        let shape = human_contract_shape(group_id);
        let payload = json!({
            "schema_version": "arda.human-corpus-group.v1",
            "generated_at_utc": now_utc(),
            "authority": "bounded_human_corpus_group_extraction",
            "group_id": group_id,
            "lane": group.get("lane").cloned().unwrap_or(Value::Null),
            "owner": group.get("owner").cloned().unwrap_or(Value::Null),
            "task_title": group.get("task_title").cloned().unwrap_or(Value::Null),
            "source_ids": group.get("sources").and_then(Value::as_array).into_iter().flatten().filter_map(|row| row.get("source_id").cloned()).collect::<Vec<_>>(),
            "source_paths": group.get("sources").and_then(Value::as_array).into_iter().flatten().filter_map(|row| row.get("canonical_path").cloned()).collect::<Vec<_>>(),
            "extraction": shape.clone(),
        });
        let group_path = group_dir.join(format!("{group_id}.json"));
        write_pretty_json(&group_path, &payload)?;
        rows.push(json!({
            "group_id": group_id,
            "lane": group.get("lane").cloned().unwrap_or(Value::Null),
            "owner": group.get("owner").cloned().unwrap_or(Value::Null),
            "group_path": rel(&group_path, &root),
            "source_ids": payload.get("source_ids").cloned().unwrap_or_else(|| json!([])),
            "contract_candidates": shape.get("contract_candidates").cloned().unwrap_or_else(|| json!([])),
            "crate_candidates": shape.get("crate_candidates").cloned().unwrap_or_else(|| json!([])),
        }));
    }

    let registry = json!({
        "schema_version": "arda.human-corpus-extraction-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "bounded_human_corpus_group_extraction",
        "summary": {
            "groups_total": rows.len(),
            "contract_candidates_total": rows.iter().map(|row| row.get("contract_candidates").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)).sum::<usize>(),
            "crate_candidates_total": rows.iter().map(|row| row.get("crate_candidates").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)).sum::<usize>(),
        },
        "groups": rows,
    });

    write_pretty_json(&out_path, &registry)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "groups_total": registry.get("summary").and_then(|v| v.get("groups_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_source_absorption_executor_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_absorption_executor.json");
    let pipeline = read_json_or(
        &root.join("core/state/source_absorption_pipeline.json"),
        json!({}),
    );
    let execution = read_json_or(
        &root.join("core/state/source_absorption_execution.json"),
        json!({}),
    );
    let tasks = latest_by_id(
        &read_jsonl_objects_local(&root.join("core/projects/tasks/queue.jsonl")),
        "id",
    );

    let queued = tasks
        .values()
        .filter(|task| {
            task.get("status").and_then(Value::as_str) == Some("queued")
                && task
                    .get("meta")
                    .and_then(|v| v.get("origin"))
                    .and_then(Value::as_str)
                    == Some("source_absorption_pipeline")
        })
        .cloned()
        .collect::<Vec<_>>();

    let candidate_index = pipeline
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let source_id = row.get("source_id").and_then(Value::as_str)?.to_string();
            Some((source_id, row.clone()))
        })
        .collect::<HashMap<_, _>>();

    let mut candidates_by_owner: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for task in &queued {
        let owner = task
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
        let source_id = meta
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let source_row = candidate_index
            .get(&source_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        candidates_by_owner
            .entry(owner.clone())
            .or_default()
            .push(json!({
                "task_id": task.get("id").cloned().unwrap_or(Value::Null),
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "owner": owner,
                "source_id": source_id,
                "domain": meta.get("domain").cloned().unwrap_or(Value::Null),
                "subsystem": meta.get("subsystem").cloned().unwrap_or(Value::Null),
                "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
                "auto_runnable": matches!(task.get("owner").and_then(Value::as_str), Some("athena" | "prometheus" | "manwe" | "hermes" | "apollo")),
                "source_title": source_row.get("title").cloned().unwrap_or(Value::Null),
                "source_url": source_row.get("url").cloned().unwrap_or(Value::Null),
                "rationale": source_row.get("rationale").cloned().unwrap_or(Value::Null),
            }));
    }

    let owner_batches = candidates_by_owner
        .into_iter()
        .map(|(owner, rows)| {
            json!({
                "owner": owner,
                "queued_total": rows.len(),
                "auto_runnable_total": rows.iter().filter(|row| row.get("auto_runnable").and_then(Value::as_bool).unwrap_or(false)).count(),
                "next_batch": rows.into_iter().take(3).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.source-absorption-executor.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_execution + project_task_queue",
        "doctrine": {
            "executor_reads_absorption_queue_not_chat_memory": true,
            "owner_batching_is_required": true,
            "safe_owner_classes_can_be_auto_run": true,
            "manual_review_remains_possible_for_any_batch": true,
        },
        "source_surfaces": {
            "pipeline": "core/state/source_absorption_pipeline.json",
            "execution": "core/state/source_absorption_execution.json",
            "queue": "core/projects/tasks/queue.jsonl",
        },
        "summary": {
            "queued_total": queued.len(),
            "owners_total": owner_batches.len(),
            "auto_runnable_total": owner_batches.iter().map(|batch| batch.get("auto_runnable_total").and_then(Value::as_u64).unwrap_or(0) as usize).sum::<usize>(),
            "promote_now_candidates_total": execution.get("summary").and_then(|v| v.get("promote_now_candidates_total")).cloned().unwrap_or(Value::Null),
        },
        "owner_batches": owner_batches,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "queued_total": payload.get("summary").and_then(|v| v.get("queued_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_source_absorption_pipeline_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_absorption_pipeline.json");
    let github_repo_integration = read_json_or(
        &root.join("core/state/github_repo_integration.json"),
        json!({}),
    );
    let athena_integration_plan = read_json_or(
        &root.join("core/state/athena_integration_plan.json"),
        json!({}),
    );
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let policy_rows = read_jsonl_objects_local(&root.join("data/athena/policy_readiness.jsonl"));

    let registry_candidates = build_registry_candidates(&github_repo_integration);
    let recent_primary_sources = build_recent_primary_sources(&policy_rows);
    let seen_source_ids = registry_candidates
        .iter()
        .filter_map(|row| row.get("source_id").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect::<std::collections::HashSet<_>>();
    let research_backlog_promotions =
        build_research_backlog_promotions(&github_repo_integration, &seen_source_ids, &root);
    let mut all_candidates = Vec::new();
    all_candidates.extend(registry_candidates);
    all_candidates.extend(recent_primary_sources);
    all_candidates.extend(research_backlog_promotions);

    let mut by_disposition = BTreeMap::new();
    let mut by_domain = BTreeMap::new();
    for row in &all_candidates {
        if let Some(disposition) = row.get("disposition").and_then(Value::as_str) {
            *by_disposition
                .entry(disposition.to_string())
                .or_insert(0usize) += 1;
        }
        if let Some(domain) = row.get("domain").and_then(Value::as_str) {
            *by_domain.entry(domain.to_string()).or_insert(0usize) += 1;
        }
    }

    let payload = json!({
        "schema_version": "arda.source-absorption-pipeline.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_policy_readiness + github_repo_integration + package_enablement",
        "mission": {
            "name": "Source Absorption Engine",
            "goal": "Turn external papers, repos, and references into sovereign adopt/adapt/reference decisions with subsystem targets and execution pressure.",
        },
        "doctrine": {
            "do_not_recreate_the_wheel": true,
            "absorb_principles_and_mechanisms_not_entire_external_identities": true,
            "domain_aware_not_dev_only": true,
            "policy_ready_sources_should_promote_into_contracts_or_tasks": true,
            "reference_only_sources_remain_visible_but_do_not_drive_implementation_by_default": true,
        },
        "domains": [
            {"id": "development", "description": "Code, runtime, packaging, tooling, infrastructure."},
            {"id": "research", "description": "Knowledge synthesis, papers, models, evaluation methods."},
            {"id": "operations", "description": "Routing, execution, runtime posture, task systems, control loops."},
            {"id": "communications", "description": "Messaging, adapters, boardrooms, channels, external interfaces."},
            {"id": "commercial", "description": "Business workflows, market intelligence, client delivery, revenue operations."},
            {"id": "governance", "description": "Safety, delegation boundaries, policy, audit, trust, decision control."},
            {"id": "embodied", "description": "Devices, edge systems, robotics, voice, sensors, physical agents."},
            {"id": "personal", "description": "Human augmentation, planning, daily work, memory, personal operations."},
        ],
        "dispositions": [
            {"id": "adopted", "meaning": "Already absorbed into sovereign runtime or state."},
            {"id": "adapted", "meaning": "Absorbed in bounded form as signal, contract, or governed-on-demand tool."},
            {"id": "promote_now", "meaning": "Policy-ready and should emit implementation work immediately."},
            {"id": "signal_only", "meaning": "Useful active signal but not yet a direct runtime adoption candidate."},
            {"id": "reference_only", "meaning": "Reference material until stronger evidence or execution pressure appears."},
        ],
        "current_posture": {
            "athena_execution_sequence": athena_integration_plan.get("recommended_execution_sequence").cloned().unwrap_or_else(|| json!([])),
            "tracked_tools_total": github_repo_integration.get("summary").and_then(|v| v.get("registry_tools_total")).cloned().unwrap_or(Value::Null),
            "research_backlog_total": github_repo_integration.get("summary").and_then(|v| v.get("research_backlog_total")).cloned().unwrap_or(Value::Null),
            "policy_ready_recent": athena_integration_plan.get("current_posture").and_then(|v| v.get("policy_ready_recent")).cloned().unwrap_or(Value::Null),
            "reference_only_recent": athena_integration_plan.get("current_posture").and_then(|v| v.get("reference_only_recent")).cloned().unwrap_or(Value::Null),
            "package_summary": package_enablement.get("summary").cloned().unwrap_or_else(|| json!({})),
        },
        "summary": {
            "candidates_total": all_candidates.len(),
            "registry_candidates_total": all_candidates.iter().filter(|row| row.get("source_kind").and_then(Value::as_str) == Some("github_repo")).count(),
            "recent_primary_sources_total": all_candidates.iter().filter(|row| row.get("source_kind").and_then(Value::as_str) != Some("github_repo") && row.get("integration_lane").and_then(Value::as_str) == Some("research_absorption")).count(),
            "research_backlog_promotions_total": all_candidates.iter().filter(|row| row.get("integration_lane").and_then(Value::as_str) == Some("research_backlog_promotion")).count(),
            "by_disposition": by_disposition,
            "by_domain": by_domain,
        },
        "automation_frontiers": [
            {
                "frontier": "promotion_engine",
                "goal": "Auto-emit subsystem tasks from promote_now sources instead of leaving implementation as a manual step.",
                "write_through": ["core/projects/tasks/queue.jsonl", "core/state/source_absorption_pipeline.json"],
            },
            {
                "frontier": "backlog_executor",
                "goal": "Let bounded internal execution loops consume absorbed source work without human babysitting.",
                "write_through": ["core/projects/tasks/queue.jsonl", "core/state/autonomy_resume.json"],
            },
        ],
        "candidates": all_candidates,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_source_absorption_portfolio_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_absorption_portfolio.json");
    let pipeline = read_json_or(
        &root.join("core/state/source_absorption_pipeline.json"),
        json!({}),
    );
    let latest = latest_by_id(
        &read_jsonl_objects_local(&root.join("core/projects/tasks/queue.jsonl")),
        "id",
    );
    let mut portfolio_sources = Vec::new();
    let mut pattern_counter = BTreeMap::new();

    for candidate in pipeline
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if candidate.get("disposition").and_then(Value::as_str) != Some("promote_now") {
            continue;
        }
        let source_id = candidate
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if source_id.is_empty() {
            continue;
        }
        let title = candidate
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(source_id);
        let url = candidate.get("url").and_then(Value::as_str);
        let pattern = candidate
            .get("absorption_pattern")
            .and_then(Value::as_str)
            .unwrap_or("repo_integration_candidate");
        let brief = source_brief(source_id, title, url, pattern, &root);
        let queued_tasks = queued_absorption_tasks(&latest, source_id);
        portfolio_sources.push(json!({
            "source_id": source_id,
            "title": title,
            "url": url,
            "domain": candidate.get("domain").cloned().unwrap_or(Value::Null),
            "disposition": candidate.get("disposition").cloned().unwrap_or(Value::Null),
            "policy_confidence": candidate.get("policy_confidence").cloned().unwrap_or(Value::Null),
            "subsystem_targets": candidate.get("subsystem_targets").cloned().unwrap_or_else(|| json!([])),
            "absorption_pattern": pattern,
            "rationale": candidate.get("rationale").cloned().unwrap_or(Value::Null),
            "implementation_brief": brief,
            "queued_absorption_tasks": queued_tasks,
            "downstream_task_templates": downstream_task_templates(source_id, title, pattern),
            "system_surfaces": candidate.get("system_surfaces").cloned().unwrap_or_else(|| json!([])),
        }));
        *pattern_counter.entry(pattern.to_string()).or_insert(0usize) += 1;
    }

    let payload = json!({
        "schema_version": "arda.source-absorption-portfolio.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_pipeline + athena_books + project_task_queue",
        "summary": {
            "promote_now_sources_total": portfolio_sources.len(),
            "queued_absorption_tasks_total": portfolio_sources.iter().map(|row| row.get("queued_absorption_tasks").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)).sum::<usize>(),
            "patterns": pattern_counter,
        },
        "sources": portfolio_sources,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(
        json!({ "out": rel(&out_path, &root), "sources_total": payload.get("summary").and_then(|v| v.get("promote_now_sources_total")).cloned().unwrap_or(Value::Null) }),
    )
}

pub(crate) fn export_source_ecosystem_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_ecosystem_registry.json");
    let portfolio = read_json_or(
        &root.join("core/state/source_absorption_portfolio.json"),
        json!({}),
    );
    let github_repo_integration = read_json_or(
        &root.join("core/state/github_repo_integration.json"),
        json!({}),
    );

    let mut ranked_registry = github_repo_integration
        .get("registry_tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.is_object())
        .collect::<Vec<_>>();
    ranked_registry.sort_by_key(ecosystem_candidate_rank);
    let ranked_registry_refs = ranked_registry
        .into_iter()
        .take(8)
        .map(|row| {
            json!({
                "tool": row.get("tool").cloned().unwrap_or(Value::Null),
                "source_id": row.get("source_id").cloned().unwrap_or(Value::Null),
                "repo_url": row.get("repo_url").cloned().unwrap_or(Value::Null),
                "activation_status": row.get("package_enablement").and_then(|v| v.get("activation_status")).cloned().unwrap_or(Value::Null),
                "integration_lane": row.get("package_enablement").and_then(|v| v.get("integration_lane")).cloned().unwrap_or(Value::Null),
                "policy_confidence": row.get("package_enablement").and_then(|v| v.get("policy_confidence")).cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();

    let catalog_sources = BTreeMap::from([
        (
            "src_33fa61b2",
            json!({"lane": "llm_agent_landscape", "focus": ["reasoning_agents", "coding_agents", "orchestration", "tooling"]}),
        ),
        (
            "src_ca2f031e",
            json!({"lane": "agent_ecosystem_landscape", "focus": ["hosted_agents", "frameworks", "browser_use", "sandboxes"]}),
        ),
    ]);

    let mut sources = Vec::new();
    let mut lane_counter = BTreeMap::new();
    for row in portfolio
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(source_id) = row.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(catalog) = catalog_sources.get(source_id) else {
            continue;
        };
        let lane = catalog
            .get("lane")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        *lane_counter.entry(lane.to_string()).or_insert(0usize) += 1;
        let promote_now = ranked_registry_refs
            .iter()
            .filter(|item| {
                matches!(
                    item.get("activation_status").and_then(Value::as_str),
                    Some("active_in_system" | "governed_on_demand" | "active_signal")
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let signal_only = ranked_registry_refs
            .iter()
            .filter(|item| item.get("activation_status").and_then(Value::as_str) == Some("planned"))
            .cloned()
            .collect::<Vec<_>>();
        sources.push(json!({
            "source_id": source_id,
            "title": row.get("title").cloned().unwrap_or(Value::Null),
            "url": row.get("url").cloned().unwrap_or(Value::Null),
            "domain": row.get("domain").cloned().unwrap_or(Value::Null),
            "lane": lane,
            "focus_areas": catalog.get("focus").cloned().unwrap_or_else(|| json!([])),
            "athena_curated_candidate_set": {
                "candidate_count": ranked_registry_refs.len(),
                "selection_basis": [
                    "registry-linked tools already digested by ATHENA",
                    "active/governed/runtime-ready tools rank ahead of planned-only tools",
                    "confidence remains secondary to bounded product posture",
                ],
                "top_candidates": ranked_registry_refs,
            },
            "prometheus_portfolio_ranking": {
                "promote_now": promote_now,
                "signal_only": signal_only,
                "portfolio_rule": "Prefer already-bounded sovereign candidates before promoting net-new external frameworks.",
            },
            "next_actions": [
                "Use the curated candidate set to seed future ATHENA ingest promotion rather than expanding the backlog indiscriminately.",
                "Keep PROMETHEUS portfolio ranking anchored to bounded product posture, not list popularity.",
            ],
        }));
    }

    let payload = json!({
        "schema_version": "arda.source-ecosystem-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_portfolio + github_repo_integration",
        "mission": {
            "name": "Ecosystem Catalog Absorption",
            "goal": "Turn list-style GitHub catalogs into curated ATHENA candidate sets and PROMETHEUS promotion portfolios.",
        },
        "summary": {
            "catalog_sources_total": sources.len(),
            "ranked_registry_candidates_total": ranked_registry_refs.len(),
            "lanes": lane_counter,
        },
        "sources": sources,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(
        json!({ "out": rel(&out_path, &root), "catalog_sources_total": payload.get("summary").and_then(|v| v.get("catalog_sources_total")).cloned().unwrap_or(Value::Null) }),
    )
}

pub(crate) fn export_community_signal_intake_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/community_signal_intake.json");
    let portfolio = read_json_or(
        &root.join("core/state/source_absorption_portfolio.json"),
        json!({}),
    );
    let comms = read_json_or(
        &root.join("core/state/communication_adapter_contract.json"),
        json!({}),
    );
    let source = portfolio
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("source_id").and_then(Value::as_str) == Some("src_dc355aed"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.community-signal-intake.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_portfolio + communication_adapter_contract",
        "campaign": {
            "name": "Community Signal Intake",
            "owner": "hermes_athena",
            "source_id": "src_dc355aed",
            "mission": "Treat community-server maps as governed signal sources rather than direct runtime dependencies.",
        },
        "doctrine": {
            "community_sources_are_signal_only": true,
            "community_presence_does_not_imply_runtime_adoption": true,
            "hermes_owns_external_community_policy": true,
            "athena_owns_signal_curation_and_retention": true,
        },
        "source": {
            "title": source.get("title").cloned().unwrap_or(Value::Null),
            "url": source.get("url").cloned().unwrap_or(Value::Null),
            "implementation_brief": source.get("implementation_brief").cloned().unwrap_or(Value::Null),
        },
        "intake_policy": {
            "signal_classes": [
                "builder_communities",
                "operator_communities",
                "tooling_announcements",
                "market_and_research_chatter",
            ],
            "allowlist_actions": [
                "capture references into ATHENA evidence",
                "surface notable opportunities into HERMES/Matrix boardroom flows",
                "preserve source provenance and observation timestamps",
            ],
            "disallowed_actions": [
                "treat community rosters as sovereign identity sources",
                "spawn adapters or bots solely because a community exists",
                "promote external Discord presence into default communications doctrine",
            ],
        },
        "hermes_policy": {
            "adapter_doctrine_ref": "core/state/communication_adapter_contract.json",
            "discord_mode": comms.get("transport_contract").and_then(|v| v.get("discord_mode")).cloned().unwrap_or(Value::Null),
            "boardroom_source": comms.get("transport_contract").and_then(|v| v.get("boardroom_source")).cloned().unwrap_or(Value::Null),
            "next_action": "Route any high-signal community observations through the Hermes-owned bounded Discord bridge and boardroom policies before external engagement.",
        },
        "athena_curation": {
            "retention_rule": "keep community observations as bounded signal records with source URL and captured-at timestamps",
            "promotion_rule": "only promote a community-derived item when it points to a concrete runtime, framework, or policy artifact",
            "human_surface": "human/library/athena/sources/src_dc355aed.md",
        },
        "summary": {
            "signal_classes_total": 4,
            "disallowed_actions_total": 3,
            "discord_mode": comms.get("transport_contract").and_then(|v| v.get("discord_mode")).cloned().unwrap_or(Value::Null),
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root), "source_id": "src_dc355aed" }))
}

pub(crate) fn export_research_workflow_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/research_workflow_contract.json");
    let portfolio = read_json_or(
        &root.join("core/state/source_absorption_portfolio.json"),
        json!({}),
    );
    let source = portfolio
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("source_id").and_then(Value::as_str) == Some("src_f226959a"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let brief = source
        .get("implementation_brief")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = json!({
        "schema_version": "arda.research-workflow-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_portfolio",
        "campaign": {
            "name": "Research Workflow Absorption",
            "owner": "apollo_athena",
            "source_id": "src_f226959a",
            "mission": "Translate autoresearch-style loops into bounded ATHENA evidence primitives and APOLLO execution stages.",
        },
        "source": {
            "title": source.get("title").cloned().unwrap_or(Value::Null),
            "url": source.get("url").cloned().unwrap_or(Value::Null),
            "implementation_brief": brief,
        },
        "athena_primitives": [
            { "primitive": "research_goal", "purpose": "declare the question and success boundary before autonomous work begins" },
            { "primitive": "evidence_harvest", "purpose": "collect sources and receipts into ATHENA books with traceable provenance" },
            { "primitive": "triad_validation", "purpose": "gate promotion through evidence, logic, and strategy rather than free-running recursion" },
            { "primitive": "implementation_brief", "purpose": "convert validated evidence into bounded deltas for downstream systems" },
        ],
        "apollo_workflow": {
            "workflow_name": "research_absorption_loop",
            "stages": [
                "define_goal",
                "harvest_sources",
                "deepen_and_validate",
                "emit_brief",
                "queue_downstream_contracts",
                "review_runtime_governor_signals",
            ],
            "checkpoints": [
                "stop if evidence remains reference_only",
                "stop if runtime budget pressure exceeds policy",
                "only emit downstream work once implementation brief exists",
            ],
        },
        "governor_boundary": {
            "unbounded_recursive_research_forbidden": true,
            "workflows_must_emit_stateful_checkpoints": true,
            "athena_and_apollo_share_the_same_evidence_anchor": true,
        },
        "summary": {
            "athena_primitives_total": 4,
            "apollo_stages_total": 6,
            "checkpoints_total": 3,
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root), "source_id": "src_f226959a" }))
}

pub(crate) fn export_apollo_research_workflow_runtime_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/apollo_research_workflow_runtime.json");
    let contract = read_json_or(
        &root.join("core/state/research_workflow_contract.json"),
        json!({}),
    );
    let apollo_runtime = read_json_or(&root.join("core/state/apollo_runtime.json"), json!({}));
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );

    let checkpoints = contract
        .get("apollo_workflow")
        .and_then(|value| value.get("checkpoints"))
        .cloned()
        .unwrap_or_else(|| json!([]));

    let workflow = json!({
        "id": "research_absorption_loop_v1",
        "name": "research_absorption_loop",
        "status": "Pending",
        "created_at": now_utc(),
        "steps": [
            workflow_step(
                "define_goal",
                "Define Goal",
                "capture_research_goal",
                json!({"required_fields": ["goal", "success_boundary", "source_anchor"]}),
                vec![],
            ),
            workflow_step(
                "harvest_sources",
                "Harvest Sources",
                "athena_harvest_sources",
                json!({"requires_receipts": true, "output": "harvest_receipts"}),
                vec!["define_goal"],
            ),
            workflow_step(
                "deepen_and_validate",
                "Deepen And Validate",
                "athena_deepen_and_triad_validate",
                json!({"stop_if_reference_only": true, "output": "validated_evidence"}),
                vec!["harvest_sources"],
            ),
            workflow_step(
                "emit_brief",
                "Emit Implementation Brief",
                "athena_emit_implementation_brief",
                json!({"output": "implementation_brief"}),
                vec!["deepen_and_validate"],
            ),
            workflow_step(
                "queue_downstream_contracts",
                "Queue Downstream Contracts",
                "prometheus_queue_contract_work",
                json!({"task_surface": "core/projects/tasks/queue.jsonl"}),
                vec!["emit_brief"],
            ),
            workflow_step(
                "review_runtime_governor_signals",
                "Review Runtime Governor Signals",
                "governor_budget_and_pressure_review",
                json!({"runtime_governor_contract": "core/state/runtime_governor_contract.json"}),
                vec!["queue_downstream_contracts"],
            ),
        ],
    });

    let payload = json!({
        "schema_version": "arda.apollo-research-workflow-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "research_workflow_contract + apollo_runtime",
        "campaign": contract.get("campaign").cloned().unwrap_or_else(|| json!({})),
        "workflow_engine_contract": {
            "workflow_struct_source": "crates/arda-apollo/src/workflow.rs",
            "execution_request_struct_source": "crates/arda-apollo/src/executor.rs",
            "apollo_runtime_paths": apollo_runtime
                .get("runtime")
                .and_then(|value| value.get("paths"))
                .cloned()
                .unwrap_or_else(|| json!({})),
        },
        "workflow": workflow,
        "execution_template": {
            "task_id": "research_absorption::<source_id>",
            "agent_id": "apollo",
            "priority": "High",
            "timeout_secs": 1800,
            "payload": {
                "workflow_kind": "research_absorption_loop",
                "source_id": "<source_id>",
                "goal": "<goal>",
                "success_boundary": "<success_boundary>",
                "source_anchor": "<source_url>",
                "checkpoints": checkpoints.clone(),
            },
        },
        "governor_gate": {
            "provider_budget_tracking": runtime_governor
                .get("capability_lanes")
                .and_then(|value| value.get("provider_budget_tracking"))
                .and_then(|value| value.get("summary"))
                .cloned()
                .unwrap_or(Value::Null),
            "stop_conditions": checkpoints.clone(),
        },
        "summary": {
            "workflow_name": workflow.get("name").cloned().unwrap_or(Value::Null),
            "stages_total": workflow
                .get("steps")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "checkpoints_total": checkpoints.as_array().map(Vec::len).unwrap_or(0),
            "runtime_path_ready": apollo_runtime
                .get("runtime")
                .and_then(|value| value.get("paths"))
                .and_then(Value::as_object)
                .map(|value| !value.is_empty())
                .unwrap_or(false),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "stages_total": payload["summary"]["stages_total"],
    }))
}

pub(crate) fn export_hermes_community_sources_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/hermes_community_sources.json");
    let intake = read_json_or(
        &root.join("core/state/community_signal_intake.json"),
        json!({}),
    );
    let adapter = read_json_or(
        &root.join("core/state/communication_adapter_contract.json"),
        json!({}),
    );
    let matrix = read_json_or(&root.join("core/state/matrix_boardrooms.json"), json!({}));

    let room_ids = matrix
        .get("boardroom_contract")
        .and_then(|value| value.get("rooms"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|room| room.get("id").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let sources = vec![
        json!({
            "source_id": "discord_builder_communities",
            "provider": "discord",
            "channels": ["boardroom", "discord"],
            "signal_class": "builder_communities",
            "route_to": "prometheus",
            "retention_mode": "bounded_signal_record",
        }),
        json!({
            "source_id": "discord_operator_communities",
            "provider": "discord",
            "channels": ["boardroom", "discord"],
            "signal_class": "operator_communities",
            "route_to": "prometheus",
            "retention_mode": "bounded_signal_record",
        }),
        json!({
            "source_id": "matrix_boardrooms",
            "provider": "matrix_boardroom",
            "channels": if room_ids.is_empty() {
                json!(["ops-boardroom", "ceo-boardroom"])
            } else {
                json!(room_ids)
            },
            "signal_class": "tooling_announcements",
            "route_to": "hermes",
            "retention_mode": "boardroom_signal_record",
        }),
        json!({
            "source_id": "external_market_chatter",
            "provider": "discord",
            "channels": ["boardroom", "discord"],
            "signal_class": "market_and_research_chatter",
            "route_to": "athena",
            "retention_mode": "bounded_signal_record",
        }),
    ];

    let payload = json!({
        "schema_version": "arda.hermes-community-sources.v1",
        "generated_at_utc": now_utc(),
        "authority": "community_signal_intake + communication_adapter_contract + matrix_boardrooms",
        "campaign": intake.get("campaign").cloned().unwrap_or_else(|| json!({})),
        "doctrine": {
            "community_sources_are_observation_inputs": true,
            "direct_runtime_adoption_forbidden": true,
            "boardroom_signal_routing_enabled": true,
            "hermes_discord_bridge_is_optional_and_policy_guarded": true,
            "discord_mode": adapter
                .get("transport_contract")
                .and_then(|value| value.get("discord_mode"))
                .cloned()
                .unwrap_or(Value::Null),
        },
        "sources": sources,
        "routing": {
            "default_discord_route": "prometheus",
            "artifact_link_route": "athena",
            "boardroom_source": adapter
                .get("transport_contract")
                .and_then(|value| value.get("boardroom_source"))
                .cloned()
                .unwrap_or(Value::Null),
        },
        "summary": {
            "sources_total": 4,
            "signal_classes_total": intake
                .get("intake_policy")
                .and_then(|value| value.get("signal_classes"))
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            "matrix_room_count": room_ids.len(),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "sources_total": payload["summary"]["sources_total"],
    }))
}

pub(crate) fn export_multi_domain_routing_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/multi_domain_routing_contract.json");
    let async_intake = read_json_or(
        &root.join("core/state/async_user_intake_contract.json"),
        json!({}),
    );
    let project_executor = read_json_or(
        &root.join("core/state/project_task_executor.json"),
        json!({}),
    );
    let source_absorption = read_json_or(
        &root.join("core/state/source_absorption_pipeline.json"),
        json!({}),
    );
    let _research_workflow = read_json_or(
        &root.join("core/state/research_workflow_contract.json"),
        json!({}),
    );
    let _communication_adapter = read_json_or(
        &root.join("core/state/communication_adapter_contract.json"),
        json!({}),
    );
    let _runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );
    let model_control = read_json_or(
        &root.join("core/state/model_control_surface.json"),
        json!({}),
    );
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let search_runtime = read_json_or(
        &root.join("core/state/search_runtime_contract.json"),
        json!({}),
    );
    let embodied_interface =
        read_json_or(&root.join("core/state/embodied_interface.json"), json!({}));

    let operator_load = operator_actions
        .get("summary")
        .and_then(|value| value.get("human_needed_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let executor_ready = project_executor
        .get("summary")
        .and_then(|value| value.get("rules_succeeded_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        >= 0;
    let async_ready = async_intake
        .get("summary")
        .and_then(|value| value.get("handoff_steps_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        >= 6;
    let provider_profiles = model_control
        .get("profile_catalog")
        .and_then(Value::as_object)
        .map(|catalog| catalog.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let available_targets = model_control
        .get("provider_catalog")
        .and_then(Value::as_object)
        .map(|catalog| catalog.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let promote_now_total = source_absorption
        .get("summary")
        .and_then(|value| value.get("by_disposition"))
        .and_then(|value| value.get("promote_now"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let domains = vec![
        domain_row(
            "development",
            "Implement, harden, and operate the sovereign software stack.",
            "chat_or_queue",
            "apollo",
            "prometheus",
            json!(["core/state/project_task_executor.json", "core/state/runtime_governor_contract.json", "core/state/model_control_surface.json"]),
            json!(["core/state/project_task_executor.json", "core/state/runtime_budget_policy.json", "core/state/opencode_route_governor.json"]),
            "backbone_reasoning",
            true,
            json!(["arda_hud", "opencode_agent_routes", "task_ledger"]),
            filter_targets(&available_targets, &["local_fallback", "edge_backbone", "litellm_gateway"]),
            readiness_label(executor_ready, async_ready, 3, operator_load),
        ),
        domain_row(
            "research",
            "Absorb sources, produce evidence, and emit implementation work from validated knowledge.",
            "athena_async_intake",
            "athena",
            "prometheus",
            json!(["core/state/async_user_intake_contract.json", "core/state/source_absorption_pipeline.json", "core/state/research_workflow_contract.json", "core/state/search_runtime_contract.json"]),
            json!(["core/state/async_user_intake_queue.json", "core/state/async_user_intake_runtime.json", "core/state/source_absorption_portfolio.json"]),
            "backbone_router",
            true,
            json!(["athena_book_override", "task_ledger", "arda_hud"]),
            filter_targets(&available_targets, &["edge_worker", "edge_backbone", "local_fallback"]),
            readiness_label(executor_ready, async_ready, 4, operator_load),
        ),
        domain_row(
            "operations",
            "Route maintenance, monitoring, remediation, and fleet posture work through bounded execution.",
            "operator_signal_or_alert",
            "warden",
            "manwe_prometheus",
            json!(["core/state/runtime_governor_contract.json", "core/state/operator_actions.json", "core/state/fleet_steward_actions.json"]),
            json!(["core/state/operator_actions.json", "core/state/fleet_power_guard.json", "core/state/edge_endpoint_verification.json"]),
            profile_or(&provider_profiles, "edge_heavy", "backbone_reasoning"),
            true,
            json!(["operator_actions", "fleet_steward_write_intents", "task_ledger"]),
            filter_targets(&available_targets, &["edge_backbone", "local_fallback", "litellm_gateway"]),
            readiness_label(executor_ready, false, 3, operator_load),
        ),
        domain_row(
            "communications",
            "Classify inbound human/network signals and hand them into the right sovereign queues without blocking dialogue.",
            "hermes",
            "hermes",
            "prometheus",
            json!(["core/state/communication_adapter_contract.json", "core/state/async_user_intake_contract.json"]),
            json!(["data/hermes/messages.jsonl", "core/state/community_signal_intake.json", "core/state/async_user_intake_runtime.json"]),
            "backbone_router",
            true,
            json!(["matrix_boardroom", "discord_boardroom", "task_ledger"]),
            filter_targets(&available_targets, &["edge_worker", "local_fallback", "edge_backbone"]),
            readiness_label(executor_ready, async_ready, 2, operator_load),
        ),
        domain_row(
            "governance",
            "Pressure-test plans and pivots with triad, joulework, and resonance before expensive commitment.",
            "plan_or_pivot",
            "oracle",
            "prometheus",
            json!(["core/state/runtime_governor_contract.json", "core/state/socratic_validator_contract.json"]),
            json!(["core/state/runtime_budget_policy.json", "core/state/operator_actions.json", "core/state/socratic_validator_audit.json"]),
            "backbone_reasoning",
            false,
            json!(["human_session", "arda_hud", "task_ledger"]),
            filter_targets(&available_targets, &["edge_backbone", "litellm_gateway", "local_fallback"]),
            "guided",
        ),
        domain_row(
            "commercial",
            "Support market, customer, and business process tasks once bounded commercial policies exist.",
            "future_async_intake",
            "apollo",
            "prometheus",
            json!(["core/state/multi_domain_routing_contract.json"]),
            json!(["core/state/source_absorption_pipeline.json"]),
            profile_or(&provider_profiles, "cloud_high", "backbone_reasoning"),
            false,
            json!(["human_session", "policy_gate", "task_ledger"]),
            filter_targets(&available_targets, &["litellm_gateway", "edge_backbone", "local_fallback"]),
            "planned",
        ),
        domain_row(
            "personal",
            "Support human augmentation tasks with bounded privacy, resonance, and override posture.",
            "future_async_intake",
            "apollo",
            "oracle",
            json!(["core/state/multi_domain_routing_contract.json"]),
            json!(["core/state/runtime_budget_policy.json"]),
            profile_or(&provider_profiles, "local_light", "backbone_router"),
            false,
            json!(["human_session", "privacy_override", "task_ledger"]),
            filter_targets(&available_targets, &["local_fallback", "edge_worker", "edge_backbone"]),
            "planned",
        ),
        domain_row(
            "embodied",
            "Coordinate physical or device-adjacent workflows through monitored, bounded interfaces.",
            "embodied_interface",
            "apollo",
            "warden",
            json!(["core/state/multi_domain_routing_contract.json", "core/state/runtime_governor_contract.json"]),
            json!(["core/state/embodied_interface.json", "core/state/fleet_power_guard.json"]),
            profile_or(&provider_profiles, "edge_heavy", "backbone_reasoning"),
            false,
            json!(["operator_console", "arda_hud", "task_ledger"]),
            filter_targets(&available_targets, &["edge_backbone", "edge_worker", "local_fallback"]),
            if embodied_interface.as_object().is_some_and(|object| !object.is_empty()) {
                "guided"
            } else {
                "planned"
            },
        ),
    ];

    let payload = json!({
        "schema_version": "arda.multi-domain-routing-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "manwe_prometheus_domain_routing_projection",
        "purpose": {
            "mission": "Map work beyond development into bounded domains with explicit intake, deliberation, routing, and execution ownership.",
            "current_bias": "development_first_bootstrap",
            "target_state": "human augmentation biosystem across multiple task domains",
        },
        "doctrine": {
            "domain_routing_must_write_through_sovereign_state": true,
            "async_intake_should_not_block_foreground_conversation": true,
            "human_override_points_must_remain_explicit": true,
            "domain_expansion_should_prefer_adapting_existing_primitives_over_rebuilding_from_zero": true,
        },
        "summary": {
            "domains_total": domains.len(),
            "ready_total": domains.iter().filter(|row| row.get("readiness").and_then(Value::as_str) == Some("ready")).count(),
            "guided_total": domains.iter().filter(|row| row.get("readiness").and_then(Value::as_str) == Some("guided")).count(),
            "planned_total": domains.iter().filter(|row| row.get("readiness").and_then(Value::as_str) == Some("planned")).count(),
            "promote_now_sources_total": promote_now_total,
            "async_user_intake_ready": async_ready,
            "search_runtime_activation_status": search_runtime
                .get("summary")
                .and_then(|value| value.get("activation_status"))
                .cloned()
                .unwrap_or(Value::Null),
        },
        "input_surfaces": {
            "async_user_intake_contract": "core/state/async_user_intake_contract.json",
            "project_task_executor": "core/state/project_task_executor.json",
            "source_absorption_pipeline": "core/state/source_absorption_pipeline.json",
            "research_workflow_contract": "core/state/research_workflow_contract.json",
            "communication_adapter_contract": "core/state/communication_adapter_contract.json",
            "runtime_governor_contract": "core/state/runtime_governor_contract.json",
            "model_control_surface": "core/state/model_control_surface.json",
            "operator_actions": "core/state/operator_actions.json",
        },
        "domains": domains,
        "next_expansion_order": [
            "operations",
            "communications",
            "governance",
            "embodied",
            "commercial",
            "personal",
        ],
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "domains_total": payload["summary"]["domains_total"],
    }))
}

pub(crate) fn export_socratic_validator_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let contract_out = root.join("core/state/socratic_validator_contract.json");
    let audit_out = root.join("core/state/socratic_validator_audit.json");
    let runtime_budget = read_json_or(
        &root.join("core/state/runtime_budget_policy.json"),
        json!({}),
    );
    let runtime_governor = read_json_or(
        &root.join("core/state/runtime_governor_contract.json"),
        json!({}),
    );
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let async_intake = read_json_or(
        &root.join("core/state/async_user_intake_contract.json"),
        json!({}),
    );
    let multi_domain_routing = read_json_or(
        &root.join("core/state/multi_domain_routing_contract.json"),
        json!({}),
    );
    let source_absorption = read_json_or(
        &root.join("core/state/source_absorption_pipeline.json"),
        json!({}),
    );
    let tasks = latest_by_id(
        &read_jsonl_objects_local(&root.join("core/projects/tasks/queue.jsonl")),
        "id",
    )
    .into_values()
    .collect::<Vec<_>>();

    let queued_pivots = tasks
        .iter()
        .filter(|row| {
            row.get("status").and_then(Value::as_str) == Some("queued")
                && row
                    .get("meta")
                    .and_then(|value| value.get("origin"))
                    .and_then(Value::as_str)
                    == Some("session_pivot")
        })
        .map(|row| {
            json!({
                "id": row.get("id").cloned().unwrap_or(Value::Null),
                "title": row.get("title").cloned().unwrap_or(Value::Null),
                "owner": row.get("owner").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let strategic_queued = queued_pivots
        .iter()
        .filter(|row| {
            !row.get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("async intake")
        })
        .cloned()
        .collect::<Vec<_>>();

    let operator_load = operator_actions
        .get("summary")
        .and_then(|value| value.get("human_needed_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let provider_budget_pressure = runtime_budget
        .get("summary")
        .and_then(|value| value.get("provider_budget_pressure_total"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let local_joule_percent = runtime_budget
        .get("user_plan_budget")
        .and_then(|value| value.get("local_joulework_usage_percent"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let local_joule_pressure = local_joule_percent >= 80.0;
    let promote_now_total = source_absorption
        .get("summary")
        .and_then(|value| value.get("by_disposition"))
        .and_then(|value| value.get("promote_now"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let contract = json!({
        "schema_version": "arda.socratic-validator-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "prometheus_oracle_deliberation_projection",
        "purpose": {
            "mission": "Pressure-test plans and pivots with bounded philosophical challenge before expensive execution.",
            "role": "non-executing deliberation module",
            "execution_authority": "none",
        },
        "doctrine": {
            "never_free_run": true,
            "only_invoked_for_defined_intent": true,
            "no_self_reinvocation": true,
            "max_rounds": 3,
            "max_questions": 7,
            "must_emit_structured_verdict": true,
            "must_write_audit_artifact": true,
            "must_handoff_to_prometheus_or_human_after_verdict": true,
        },
        "invocation_policy": {
            "allowed_intents": [
                "plan_clarification",
                "strategic_pivot",
                "high_cost_operation",
                "conflicting_priorities",
                "domain_expansion",
            ],
            "explicit_non_goals": [
                "general_chat_companion",
                "recursive_self-reflection",
                "direct_command_execution",
                "persistent_conversational_presence",
            ],
            "trigger_signals": [
                {"signal": "queued_session_pivots", "active": !strategic_queued.is_empty(), "observed": strategic_queued.len()},
                {"signal": "provider_budget_pressure", "active": provider_budget_pressure > 0, "observed": provider_budget_pressure},
                {"signal": "local_joule_pressure", "active": local_joule_pressure, "observed_percent": runtime_budget.get("user_plan_budget").and_then(|value| value.get("local_joulework_usage_percent")).cloned().unwrap_or(Value::Null)},
                {"signal": "operator_attention_needed", "active": operator_load > 0, "observed": operator_load},
                {"signal": "domain_expansion_frontier", "active": multi_domain_routing.get("summary").and_then(|value| value.get("planned_total")).and_then(Value::as_i64).unwrap_or(0) > 0, "observed": multi_domain_routing.get("summary").and_then(|value| value.get("planned_total")).cloned().unwrap_or(Value::Null)},
            ],
        },
        "evaluation_axes": {
            "triad": ["aurelius", "bacon", "sun_tzu"],
            "joulework": ["estimated_cost", "budget_pressure", "time_pressure"],
            "love_equation": ["alignment_score", "human_resonance", "mission_fit"],
        },
        "output_contract": {
            "verdict_fields": [
                "objective",
                "assumptions",
                "contradictions",
                "risks",
                "recommended_path",
                "human_override_needed",
                "triad_score",
                "joulework_estimate",
                "love_equation_score",
            ],
            "handoff_targets": ["prometheus", "athena", "apollo", "task_ledger"],
            "hud_visibility": {
                "panel": "decision_audit",
                "show_invocation_reason": true,
                "show_cost_and_alignment": true,
                "show_override_outcome": true,
            },
        },
        "input_surfaces": {
            "runtime_budget_policy": "core/state/runtime_budget_policy.json",
            "runtime_governor_contract": "core/state/runtime_governor_contract.json",
            "operator_actions": "core/state/operator_actions.json",
            "async_user_intake_contract": "core/state/async_user_intake_contract.json",
            "multi_domain_routing_contract": "core/state/multi_domain_routing_contract.json",
            "project_task_queue": "core/projects/tasks/queue.jsonl",
        },
        "summary": {
            "strategic_queued_total": strategic_queued.len(),
            "provider_budget_pressure_total": provider_budget_pressure,
            "local_joule_pressure_active": local_joule_pressure,
            "operator_attention_total": operator_load,
            "promote_now_sources_total": promote_now_total,
            "async_handoff_steps_total": async_intake.get("summary").and_then(|value| value.get("handoff_steps_total")).cloned().unwrap_or(Value::Null),
            "runtime_governor_provider_budget_lanes": runtime_governor
                .get("capability_lanes")
                .and_then(|value| value.get("provider_budget_tracking"))
                .and_then(|value| value.get("summary"))
                .and_then(|value| value.get("providers_with_daily_limits"))
                .cloned()
                .unwrap_or_else(|| json!({})),
        },
    });

    let audit = json!({
        "schema_version": "arda.socratic-validator-audit.v1",
        "generated_at_utc": now_utc(),
        "authority": "prometheus_oracle_deliberation_projection",
        "summary": {
            "invocations_total": 0,
            "open_verdicts_total": 0,
            "human_overrides_total": 0,
            "auto_handoffs_total": 0,
        },
        "status": {
            "armed": true,
            "mode": "bounded_not_yet_invoked",
            "current_trigger_pressure": {
                "strategic_queued_total": strategic_queued.len(),
                "provider_budget_pressure_total": provider_budget_pressure,
                "operator_attention_total": operator_load,
            },
        },
        "recent_invocations": [],
        "queued_pivot_context": strategic_queued.iter().take(5).cloned().collect::<Vec<_>>(),
        "arda_hints": {
            "primary_panel": "decision_audit",
            "show_empty_state": true,
            "invocation_reason_visible": true,
        },
    });

    write_pretty_json(&contract_out, &contract)?;
    write_pretty_json(&audit_out, &audit)?;
    Ok(json!({
        "contract_out": rel(&contract_out, &root),
        "audit_out": rel(&audit_out, &root),
        "strategic_queued_total": strategic_queued.len(),
    }))
}

pub(crate) fn export_agent_continuity_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/agent_continuity_contract.json");
    let external_brief = read_json_or(
        &root.join("core/state/external_absorption_brief.json"),
        json!({}),
    );
    let async_intake = read_json_or(
        &root.join("core/state/async_user_intake_contract.json"),
        json!({}),
    );
    let research_workflow = read_json_or(
        &root.join("core/state/research_workflow_contract.json"),
        json!({}),
    );
    let multi_domain = read_json_or(
        &root.join("core/state/multi_domain_routing_contract.json"),
        json!({}),
    );
    let socratic = read_json_or(
        &root.join("core/state/socratic_validator_contract.json"),
        json!({}),
    );

    let continuity_sources = external_brief
        .get("comparison_set")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            matches!(
                row.get("source_id").and_then(Value::as_str),
                Some("src_ec2b8bd4" | "src_16c075a2")
            )
        })
        .cloned()
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.agent-continuity-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "external_absorption_brief + async_intake + research_workflow + multi_domain_routing",
        "mission": {
            "goal": "Expose persistent agent continuity as a bounded sovereign capability spanning memory, skills, and cross-session task context.",
            "positioning": "arda continuity belongs to sovereign state and governed workflows, not a foreign monolithic agent runtime.",
        },
        "source_drivers": continuity_sources,
        "continuity_capabilities": [
            {
                "capability": "session_recall",
                "owner": "athena_mnemosyne",
                "purpose": "retrieve prior project/session context during active work without relying on chat scrollback",
                "target_surfaces": ["core/state/async_user_intake_contract.json", "core/state/research_workflow_contract.json"],
            },
            {
                "capability": "skill_evolution",
                "owner": "prometheus_apollo",
                "purpose": "turn repeated successful patterns into bounded reusable workflow or tool skills",
                "target_surfaces": ["core/state/source_absorption_pipeline.json", "core/state/project_task_executor.json"],
            },
            {
                "capability": "cross_channel_identity_continuity",
                "owner": "hermes",
                "purpose": "keep one bounded operator identity across chat, gateway, and async intake lanes",
                "target_surfaces": ["core/state/communication_adapter_contract.json", "core/state/async_user_intake_runtime.json"],
            },
            {
                "capability": "bounded_long_horizon_execution",
                "owner": "apollo_oracle",
                "purpose": "keep long-running work aligned through reminders, checkpoints, and harness controls",
                "target_surfaces": ["core/state/soterion_joulework_enforcement.json", "core/state/socratic_validator_contract.json"],
            },
        ],
        "governor_boundaries": {
            "memory_must_be_retrieval_based_not_hidden_prompt_bloat": true,
            "skill_updates_must_land_in_stateful_contracts_or_workflows": true,
            "continuity_must_respect_domain_and_privacy_boundaries": true,
            "long_horizon_execution_requires_harness_and_verification_checkpoints": true,
        },
        "handoff_path": [
            "foreground conversation or gateway event",
            "async intake handoff",
            "ATHENA/Mnemosyne retrieval",
            "APOLLO execution or workflow reuse",
            "Socratic checkpoint for expensive pivots",
            "stateful continuity update",
        ],
        "summary": {
            "source_drivers_total": continuity_sources.len(),
            "continuity_capabilities_total": 4,
            "handoff_steps_total": 6,
            "async_handoff_ready": async_intake
                .get("summary")
                .and_then(|value| value.get("handoff_steps_total"))
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0,
            "research_workflow_stages_total": research_workflow
                .get("summary")
                .and_then(|value| value.get("apollo_stages_total"))
                .cloned()
                .unwrap_or(Value::Null),
            "routing_domains_total": multi_domain
                .get("summary")
                .and_then(|value| value.get("domains_total"))
                .cloned()
                .unwrap_or(Value::Null),
            "governance_signal_dimensions_total": socratic
                .get("evaluation_axes")
                .and_then(Value::as_object)
                .map(|value| value.len())
                .unwrap_or(0),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

type EvidenceSpecs<'a> = &'a [(&'a str, &'a [(&'a str, &'a [&'a str])])];

fn build_autonomy_latest_evidence(root: &std::path::Path) -> Vec<Value> {
    let mut rows = Vec::new();
    let specs: EvidenceSpecs<'_> = &[
        (
            "core/metrics/manifest.json",
            &[
                ("snapshot_id", &["snapshot_id"]),
                ("generated_at_utc", &["generated_at_utc"]),
            ],
        ),
        (
            "core/state/athena_runtime.json",
            &[("generated_at_utc", &["generated_at_utc"])],
        ),
        (
            "core/state/hades_lifecycle.json",
            &[("generated_at_utc", &["generated_at_utc"])],
        ),
        (
            "core/state/project_task_executor.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("eligible_total", &["summary", "eligible_total"]),
                ("rules_ran_total", &["summary", "rules_ran_total"]),
            ],
        ),
        (
            "core/state/memory_governor.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("continuity_pressure", &["summary", "continuity_pressure"]),
                (
                    "recommended_actions_total",
                    &["summary", "recommended_actions_total"],
                ),
            ],
        ),
        (
            "core/state/athena_digest_pipeline.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "pipeline_entries_total",
                    &["summary", "pipeline_entries_total"],
                ),
                (
                    "execution_ready_total",
                    &["summary", "execution_ready_total"],
                ),
            ],
        ),
        (
            "core/state/operator_legibility_contract.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "productization_lanes_total",
                    &["summary", "productization_lanes_total"],
                ),
            ],
        ),
        (
            "core/state/fleet_capability_ranking.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "primary_backbone_node",
                    &["summary", "primary_backbone_node"],
                ),
                (
                    "primary_operator_node",
                    &["summary", "primary_operator_node"],
                ),
            ],
        ),
        (
            "core/state/athena_integration_plan.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "integration_lanes_total",
                    &["summary", "integration_lanes_total"],
                ),
            ],
        ),
        (
            "core/state/source_absorption_pipeline.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("candidates_total", &["summary", "candidates_total"]),
                (
                    "promote_now_total",
                    &["summary", "by_disposition", "promote_now"],
                ),
            ],
        ),
        (
            "core/state/runtime_budget_policy.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "provider_budget_pressure_total",
                    &["summary", "provider_budget_pressure_total"],
                ),
            ],
        ),
        (
            "core/state/runtime_admission_receipts.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("shed_events_total", &["summary", "shed_events_total"]),
                ("latest_shed_at_utc", &["summary", "latest_shed_at_utc"]),
            ],
        ),
        (
            "core/state/runtime_admission_recovery.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "recovery_actions_total",
                    &["summary", "recovery_actions_total"],
                ),
            ],
        ),
        (
            "core/state/runtime_admission_recovery_executor.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("executed_total", &["summary", "executed_total"]),
                ("failed_total", &["summary", "failed_total"]),
            ],
        ),
        (
            "core/state/runtime_recovery_route_governor.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("status", &["status"]),
                ("current_origin", &["current_origin"]),
                ("desired_origin", &["desired_origin"]),
            ],
        ),
        (
            "core/state/project_intake_governance.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "tracked_dossiers_total",
                    &["summary", "tracked_dossiers_total"],
                ),
                (
                    "execution_eligible_total",
                    &["summary", "execution_eligible_total"],
                ),
            ],
        ),
        (
            "core/state/soterion_joulework_enforcement.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "soterion_trace_missing_total",
                    &["summary", "soterion_trace_missing_total"],
                ),
                ("local_joule_pressure", &["summary", "local_joule_pressure"]),
            ],
        ),
        (
            "core/state/operator_actions.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                ("human_needed_total", &["summary", "human_needed_total"]),
            ],
        ),
        (
            "core/state/edge_model_rollout.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "targets_complete_total",
                    &["summary", "targets_complete_total"],
                ),
            ],
        ),
        (
            "core/state/edge_endpoint_verification.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "targets_with_live_endpoints_total",
                    &["summary", "targets_with_live_endpoints_total"],
                ),
            ],
        ),
        (
            "core/state/fleet_power_guard.json",
            &[
                ("generated_at_utc", &["generated_at_utc"]),
                (
                    "targets_hardened_total",
                    &["summary", "targets_hardened_total"],
                ),
            ],
        ),
    ];

    for (relative, fields) in specs {
        let data = read_json_or(&root.join(relative), json!({}));
        let mut row = Map::new();
        row.insert("path".to_string(), json!(relative));
        for (key, path) in *fields {
            row.insert(
                (*key).to_string(),
                value_at_path(&data, path).cloned().unwrap_or(Value::Null),
            );
        }
        rows.push(Value::Object(row));
    }
    rows
}

fn value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn package_by_tool(data: &Value) -> HashMap<String, Value> {
    data.get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let tool = row.get("tool").and_then(Value::as_str)?.to_string();
            Some((tool, row.clone()))
        })
        .collect()
}

fn github_by_tool(data: &Value) -> HashMap<String, Value> {
    data.get("registry_tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let tool = row.get("tool").and_then(Value::as_str)?.to_string();
            Some((tool, row.clone()))
        })
        .collect()
}

fn read_jsonl_objects_local(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
                .filter(|value| value.is_object())
                .collect()
        })
        .unwrap_or_default()
}

fn latest_by_id(rows: &[Value], key: &str) -> HashMap<String, Value> {
    let mut latest = HashMap::new();
    for row in rows {
        if let Some(id) = row.get(key).and_then(Value::as_str).map(str::trim) {
            if !id.is_empty() {
                latest.insert(id.to_string(), row.clone());
            }
        }
    }
    latest
}

fn stable_human_task_id(group_id: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    group_id.hash(&mut hasher);
    format!("tsk_humancorpus_{:012x}", hasher.finish() & 0xffffffffffff)
}

fn stable_absorption_task_id(source_id: &str, subsystem: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{source_id}:{subsystem}").hash(&mut hasher);
    format!("tsk_absorb_{:012x}", hasher.finish() & 0xffffffffffff)
}

fn plan_nodes_by_owner(plan_map: &Value) -> HashMap<String, Value> {
    let mut nodes = HashMap::new();
    for plan in plan_map
        .get("plans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(owner) = plan.get("owner").and_then(Value::as_str).map(str::trim) {
            if !owner.is_empty() {
                nodes.insert(owner.to_string(), plan.clone());
            }
        }
    }
    nodes
}

fn human_stage(task: Option<&Value>, extraction_group: Option<&Value>) -> (&'static str, bool) {
    if extraction_group.is_some()
        && task.and_then(|t| t.get("status")).and_then(Value::as_str) == Some("completed")
    {
        return ("contract_ready", true);
    }
    if extraction_group.is_some() {
        return ("contract_emitted", true);
    }
    if matches!(
        task.and_then(|t| t.get("status")).and_then(Value::as_str),
        Some("queued" | "in_progress" | "blocked")
    ) {
        return ("task_bound", true);
    }
    ("planned", false)
}

fn absorption_stage(
    has_anchor_tasks: bool,
    has_portfolio_row: bool,
    has_downstream_rows: bool,
) -> (&'static str, bool) {
    if has_downstream_rows {
        ("downstream_bound", true)
    } else if has_portfolio_row {
        ("portfolio_bound", true)
    } else if has_anchor_tasks {
        ("task_bound", true)
    } else {
        ("planned", false)
    }
}

fn latest_task_state(tasks: &[Value]) -> Vec<Value> {
    let mut latest = HashMap::new();
    for task in tasks {
        if let Some(task_id) = task.get("id").and_then(Value::as_str).map(str::trim) {
            if !task_id.is_empty() {
                latest.insert(task_id.to_string(), task.clone());
            }
        }
    }
    latest.into_values().collect()
}

fn preview_source_id(row: &Value) -> String {
    let preview = row
        .get("stdout_preview")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let marker = "\"id\": \"";
    preview
        .find(marker)
        .and_then(|idx| {
            let start = idx + marker.len();
            preview[start..]
                .find('"')
                .map(|end| preview[start..start + end].to_string())
        })
        .unwrap_or_default()
}

fn classify_human_corpus_group(
    canonical_path: &str,
) -> (&'static str, &'static str, &'static str, String) {
    let lower = canonical_path.to_lowercase();
    let title = std::path::Path::new(canonical_path)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("source");
    if lower.contains("master_operations_plan") || lower.contains("the_soterion") {
        return (
            "doctrine",
            "numenor_doctrine",
            "prometheus",
            format!("Absorb doctrine source: {title}"),
        );
    }
    if lower.contains("numenor_") || lower.contains("/architecture/") {
        return (
            "architecture",
            "numenor_architecture_contracts",
            "core",
            format!("Extract contract primitives from {title}"),
        );
    }
    if lower.contains("agent_framework_comparative") {
        return (
            "comparison",
            "agent_framework_comparison",
            "prometheus",
            "Update sovereign agent framework comparison from human plan".to_string(),
        );
    }
    if lower.contains("counsel_business_engine") {
        return (
            "suite_spec",
            "council_engine",
            "athena",
            "Extract Council Engine into bounded business-intelligence suite contract".to_string(),
        );
    }
    if lower.contains("atelier_art_creation") {
        return (
            "suite_spec",
            "atelier",
            "athena",
            "Extract Atelier into bounded creative suite contract".to_string(),
        );
    }
    if lower.contains("scriptorium_literature") {
        return (
            "suite_spec",
            "scriptorium",
            "athena",
            "Extract Scriptorium into bounded writing suite contract".to_string(),
        );
    }
    if lower.contains("forge_mind_engineering") {
        return (
            "suite_spec",
            "forge_mind",
            "athena",
            "Extract Forge Mind into bounded engineering suite contract".to_string(),
        );
    }
    if lower.contains("signal_grid_social_pipeline") {
        return (
            "suite_spec",
            "signal_grid",
            "athena",
            "Extract Signal Grid into bounded social pipeline contract".to_string(),
        );
    }
    if lower.contains("resonance_music_suite") {
        return (
            "suite_spec",
            "resonance",
            "athena",
            "Extract Resonance into bounded music suite contract".to_string(),
        );
    }
    if lower.contains("signal_engine_marketing") {
        return (
            "suite_spec",
            "signal_engine",
            "athena",
            "Extract Signal Engine into bounded marketing automation contract".to_string(),
        );
    }
    if lower.contains("between_two_fates_gdd") {
        return (
            "creative_world",
            "between_two_fates",
            "athena",
            "Extract Between Two Fates into world/game design contract".to_string(),
        );
    }
    (
        "misc",
        "unclassified",
        "athena",
        format!("Digest source {title}"),
    )
}

fn human_contract_shape(group_id: &str) -> Value {
    match group_id {
        "numenor_architecture_contracts" => json!({
            "extracted_primitives": ["tool_harness_pipeline", "daemon_api_contract", "fixture_runner_contract", "stub_handler_contract", "service_registry_schema"],
            "contract_candidates": ["tool_harness_contract", "service_registry_contract", "daemon_api_contract", "deterministic_fixture_replay_contract"],
            "crate_candidates": ["arda-tool-harness", "arda-service-registry"],
            "next_downstream_tasks": ["Formalize tool harness contract from Numenor architecture sources", "Formalize service registry and daemon API contracts"],
        }),
        "numenor_doctrine" => json!({
            "extracted_primitives": ["nation_model", "agent_as_employee_model", "archive_not_delete_posture", "symbolic_protocol_lineage"],
            "contract_candidates": ["operator_cosmology_contract", "archive_governance_doctrine"],
            "crate_candidates": [],
            "next_downstream_tasks": ["Absorb surviving doctrine primitives without importing obsolete org metaphors wholesale"],
        }),
        "agent_framework_comparison" => json!({
            "extracted_primitives": ["framework_comparison_matrix", "sovereignty_filter", "washable_extension_criteria"],
            "contract_candidates": ["agent_framework_comparison_contract"],
            "crate_candidates": [],
            "next_downstream_tasks": ["Refresh framework comparison with sovereign acceptance/rejection criteria"],
        }),
        "between_two_fates" => json!({
            "extracted_primitives": ["world_lore_bible", "quest_and_faction_structure", "ending_matrix", "production_scope_frame"],
            "contract_candidates": ["creative_world_contract", "game_design_memory_contract"],
            "crate_candidates": ["arda-storyworld"],
            "next_downstream_tasks": ["Formalize Between Two Fates design primitives into world/game contract surfaces"],
        }),
        "council_engine" => json!({
            "extracted_primitives": ["multi_seat_advisory_query", "domain_specific_specialist_seats", "document_review_mode", "scenario_stress_test_mode"],
            "contract_candidates": ["business_intelligence_suite_contract", "advisory_council_query_contract"],
            "crate_candidates": ["arda-council"],
            "next_downstream_tasks": ["Formalize Council Engine seat/query modes as bounded suite contract"],
        }),
        "atelier" => json!({
            "extracted_primitives": ["creative_pipeline_router", "tool_agnostic_art_pipeline", "aesthetic_profile_contract", "asset_output_pipeline"],
            "contract_candidates": ["creative_suite_contract", "asset_pipeline_contract"],
            "crate_candidates": ["arda-atelier"],
            "next_downstream_tasks": ["Formalize Atelier into creative suite contract with pipeline stages"],
        }),
        "forge_mind" => json!({
            "extracted_primitives": ["engineering_intelligence_domains", "software_hardware_fabrication_split", "documentation_authority_role", "research_to_build_flow"],
            "contract_candidates": ["engineering_suite_contract", "fabrication_research_contract"],
            "crate_candidates": ["arda-forge-mind"],
            "next_downstream_tasks": ["Formalize Forge Mind as engineering/fabrication suite contract"],
        }),
        "resonance" => json!({
            "extracted_primitives": ["tool_agnostic_music_pipeline", "composition_production_mastering_chain", "licensing_and_sync_lane"],
            "contract_candidates": ["music_suite_contract"],
            "crate_candidates": ["arda-resonance"],
            "next_downstream_tasks": ["Formalize Resonance as sovereign music suite contract"],
        }),
        "scriptorium" => json!({
            "extracted_primitives": ["long_form_writing_pipeline", "lore_and_research_corpus_link", "publishing_lane", "narrative_authoring_memory"],
            "contract_candidates": ["writing_suite_contract", "publishing_workflow_contract"],
            "crate_candidates": ["arda-scriptorium"],
            "next_downstream_tasks": ["Formalize Scriptorium as writing/lore/publishing suite contract"],
        }),
        "signal_engine" => json!({
            "extracted_primitives": ["autonomous_marketing_loop", "content_to_distribution_pipeline", "weekly_review_thresholds", "campaign_feedback_controls"],
            "contract_candidates": ["marketing_automation_contract", "review_threshold_contract"],
            "crate_candidates": ["arda-signal-engine"],
            "next_downstream_tasks": ["Formalize Signal Engine into bounded marketing automation contract"],
        }),
        "signal_grid" => json!({
            "extracted_primitives": ["multi_brand_voice_isolation", "shared_social_pipeline", "community_and_content_routing", "zero_burnout_workflow"],
            "contract_candidates": ["social_pipeline_contract", "brand_voice_isolation_contract"],
            "crate_candidates": ["arda-signal-grid"],
            "next_downstream_tasks": ["Formalize Signal Grid into bounded social pipeline contract"],
        }),
        _ => json!({
            "extracted_primitives": [],
            "contract_candidates": [],
            "crate_candidates": [],
            "next_downstream_tasks": [],
        }),
    }
}

fn classify_domain(
    tags: &[Value],
    integration_lane: &str,
    source_kind: &str,
) -> (&'static str, Vec<&'static str>) {
    let tag_set = tags
        .iter()
        .filter_map(Value::as_str)
        .map(|tag| tag.to_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let lane = integration_lane.to_lowercase();
    if lane.contains("communication") || lane.contains("boardroom") || tag_set.contains("matrix") {
        return ("communications", vec!["hermes", "communications"]);
    }
    if lane.contains("ingest") || lane.contains("crawl") || lane.contains("knowledge") {
        return ("knowledge", vec!["athena", "ingestion"]);
    }
    if lane.contains("model_selection") || lane.contains("provider") || lane.contains("routing") {
        return ("operations", vec!["manwe", "routing"]);
    }
    if lane.contains("edge") || lane.contains("runtime") {
        return ("operations", vec!["manwe", "runtime"]);
    }
    if lane.contains("workflow") || lane.contains("execution") {
        return ("operations", vec!["apollo", "workflow"]);
    }
    if source_kind == "scholarly_source" {
        return ("research", vec!["athena", "research"]);
    }
    ("operations", vec!["prometheus", "integration"])
}

fn disposition_for(
    policy_readiness: &str,
    activation_status: &str,
    integration_state: &str,
) -> &'static str {
    if activation_status == "active_in_system" {
        "adopted"
    } else if matches!(activation_status, "active_signal" | "governed_on_demand") {
        "adapted"
    } else if policy_readiness == "policy_ready" {
        "promote_now"
    } else if matches!(integration_state, "observed_only" | "blocked_on_auth") {
        "signal_only"
    } else {
        "reference_only"
    }
}

fn rationale_for(disposition: &str, policy_readiness: &str) -> &'static str {
    match disposition {
        "adopted" => "Already absorbed into sovereign runtime or package posture.",
        "adapted" => "Absorbed as a bounded signal or governed-on-demand capability rather than a continuously active runtime.",
        "promote_now" => "Evidence is policy-ready and should promote into a bounded sovereign contract or execution task.",
        "signal_only" => "Useful signal source, but not yet a bounded runtime or contract candidate.",
        _ if policy_readiness == "reference_only" => {
            "Retain as reference material until stronger evidence or implementation pressure appears."
        }
        _ => "Hold as research reference until a subsystem target is clearer.",
    }
}

fn classify_repo_absorption(
    title: &str,
    url: Option<&str>,
) -> (&'static str, Vec<&'static str>, &'static str) {
    let text = format!("{title} {}", url.unwrap_or("")).to_lowercase();
    if text.contains("scrapling") {
        return (
            "knowledge",
            vec!["athena", "prometheus"],
            "bounded_ingest_runtime",
        );
    }
    if text.contains("crawl4ai") {
        return (
            "knowledge",
            vec!["athena", "prometheus"],
            "continuous_ingest_runtime",
        );
    }
    if text.contains("autoresearch") {
        return (
            "operations",
            vec!["athena", "prometheus"],
            "research_workflow_pattern",
        );
    }
    if text.contains("searxng") || text.contains("search") {
        return ("knowledge", vec!["athena", "prometheus"], "search_runtime");
    }
    if text.contains("discord") || text.contains("community") || text.contains("server") {
        return (
            "communications",
            vec!["prometheus", "athena"],
            "community_signal_map",
        );
    }
    if text.contains("awesome-") {
        return (
            "development",
            vec!["prometheus", "athena"],
            "ecosystem_catalog",
        );
    }
    if url.is_some_and(|value| value.contains("github.com")) {
        return (
            "development",
            vec!["prometheus", "athena"],
            "repo_integration_candidate",
        );
    }
    ("research", vec!["athena", "prometheus"], "research_pattern")
}

fn build_registry_candidates(github_repo_integration: &Value) -> Vec<Value> {
    github_repo_integration
        .get("registry_tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| {
            let pkg = row
                .get("package_enablement")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let athena = row.get("athena").cloned().unwrap_or_else(|| json!({}));
            let policy_readiness = pkg
                .get("policy_readiness")
                .and_then(Value::as_str)
                .unwrap_or("reference_only");
            let activation_status = pkg
                .get("activation_status")
                .and_then(Value::as_str)
                .unwrap_or("planned");
            let integration_state = pkg
                .get("integration_state")
                .and_then(Value::as_str)
                .unwrap_or("observed_only");
            let tags = athena
                .get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| {
                    row.get("relevance_tags")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                });
            let (domain, subsystem_targets) = classify_domain(
                &tags,
                pkg.get("integration_lane")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                "github_repo",
            );
            let disposition =
                disposition_for(policy_readiness, activation_status, integration_state);
            json!({
                "source_id": row.get("source_id").cloned().unwrap_or(Value::Null),
                "source_kind": "github_repo",
                "title": row.get("tool").cloned().unwrap_or(Value::Null),
                "url": row.get("repo_url").cloned().unwrap_or(Value::Null),
                "domain": domain,
                "subsystem_targets": subsystem_targets,
                "policy_readiness": pkg.get("policy_readiness").cloned().unwrap_or(Value::Null),
                "policy_confidence": pkg.get("policy_confidence").cloned().unwrap_or(Value::Null),
                "integration_lane": pkg.get("integration_lane").cloned().unwrap_or(Value::Null),
                "integration_state": pkg.get("integration_state").cloned().unwrap_or(Value::Null),
                "activation_status": pkg.get("activation_status").cloned().unwrap_or(Value::Null),
                "disposition": disposition,
                "rationale": rationale_for(disposition, policy_readiness),
                "next_action": pkg.get("next_action").cloned().unwrap_or(Value::Null),
                "system_surfaces": row.get("system_surfaces").cloned().unwrap_or_else(|| json!([])),
                "absorption_pattern": "registry_linked_tool",
            })
        })
        .collect()
}

fn latest_by_source(rows: &[Value]) -> Vec<Value> {
    let mut latest = HashMap::new();
    for row in rows {
        let source_id = row
            .get("source_id")
            .or_else(|| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if !source_id.is_empty() {
            latest.insert(source_id.to_string(), row.clone());
        }
    }
    latest.into_values().collect()
}

fn build_recent_primary_sources(policy_rows: &[Value]) -> Vec<Value> {
    latest_by_source(policy_rows)
        .into_iter()
        .filter_map(|row| {
            let source_type = row.get("source_type").and_then(Value::as_str).unwrap_or("").to_lowercase();
            if matches!(source_type.as_str(), "githubrepo" | "synthetic" | "opposingviewpoint") {
                return None;
            }
            let deep = row.get("deep").or_else(|| row.get("data")).cloned().unwrap_or_else(|| json!({}));
            let tags = deep.get("relevance_tags").and_then(Value::as_array).cloned().unwrap_or_default();
            let policy_readiness = row
                .get("policy_readiness")
                .or_else(|| deep.get("policy_readiness"))
                .and_then(Value::as_str)
                .unwrap_or("reference_only");
            let implementation_brief = deep.get("implementation_brief");
            let disposition = if policy_readiness == "policy_ready" && implementation_brief.is_some() {
                "promote_now"
            } else {
                "reference_only"
            };
            let (domain, subsystem_targets) = classify_domain(&tags, "", "scholarly_source");
            Some(json!({
                "source_id": row.get("source_id").cloned().unwrap_or(Value::Null),
                "source_kind": if source_type.contains("scholarly") { "scholarly_source" } else if source_type.is_empty() { "primary_source" } else { source_type.as_str() },
                "title": deep.get("title").cloned().or_else(|| row.get("title").cloned()).or_else(|| row.get("source_id").cloned()).unwrap_or(Value::Null),
                "url": row.get("source_url").cloned().or_else(|| row.get("url").cloned()).unwrap_or(Value::Null),
                "domain": domain,
                "subsystem_targets": subsystem_targets,
                "policy_readiness": policy_readiness,
                "policy_confidence": deep.get("confidence").cloned().unwrap_or(Value::Null),
                "integration_lane": "research_absorption",
                "integration_state": "reference_corpus",
                "activation_status": "none",
                "disposition": disposition,
                "rationale": rationale_for(disposition, policy_readiness),
                "next_action": if disposition == "reference_only" { json!("Run opposition harvest and raise policy readiness before implementation promotion.") } else { json!("Generate implementation tasks from policy-ready source.") },
                "system_surfaces": ["data/athena/policy_readiness.jsonl", "data/athena/books/"],
                "absorption_pattern": if implementation_brief.is_some() { "scholarly_brief" } else { "scholarly_reference" },
            }))
        })
        .collect()
}

fn extract_title_from_human_ref(
    path_value: Option<&str>,
    fallback: &str,
    root: &std::path::Path,
) -> String {
    let Some(path_value) = path_value else {
        return fallback.to_string();
    };
    let path = root.join(path_value);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return fallback.to_string();
    };
    for line in raw.lines() {
        if let Some((_, title)) = line.split_once("**Title**:") {
            let title = title.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    fallback.to_string()
}

fn build_research_backlog_promotions(
    github_repo_integration: &Value,
    seen_source_ids: &std::collections::HashSet<String>,
    root: &std::path::Path,
) -> Vec<Value> {
    github_repo_integration
        .get("research_backlog_top")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let source_id = row.get("source_id").and_then(Value::as_str).unwrap_or("").trim();
            if source_id.is_empty() || seen_source_ids.contains(source_id) {
                return None;
            }
            let confidence = row.get("confidence").and_then(Value::as_f64).unwrap_or(0.0);
            let triad_passed = row.get("triad_passed").and_then(Value::as_bool).unwrap_or(false);
            let status = row.get("status").and_then(Value::as_str).unwrap_or("");
            if status != "deep" || !triad_passed || confidence < 0.75 {
                return None;
            }
            let url = row.get("url").and_then(Value::as_str);
            let title = extract_title_from_human_ref(row.get("human_ref").and_then(Value::as_str), source_id, root);
            if title.starts_with("OPPOSING_VIEWPOINT") {
                return None;
            }
            let (domain, subsystem_targets, absorption_pattern) = classify_repo_absorption(&title, url);
            Some(json!({
                "source_id": source_id,
                "source_kind": "research_backlog",
                "title": title,
                "url": url,
                "domain": domain,
                "subsystem_targets": subsystem_targets,
                "policy_readiness": "policy_ready",
                "policy_confidence": confidence,
                "integration_lane": "research_backlog_promotion",
                "integration_state": "backlog_ready",
                "activation_status": "none",
                "disposition": "promote_now",
                "rationale": "Research backlog source is already deep, triad-passed, and high-confidence; it should emit implementation or integration work instead of remaining passive evidence.",
                "next_action": "Promote this backlog source into bounded subsystem work through the source absorption emitter.",
                "system_surfaces": row.get("system_surfaces").cloned().unwrap_or_else(|| json!(["core/state/github_repo_integration.json", "core/state/source_absorption_pipeline.json"])),
                "absorption_pattern": absorption_pattern,
            }))
        })
        .collect()
}

fn latest_deep_book(source_id: &str, root: &std::path::Path) -> Value {
    let path = root
        .join("data/athena/books")
        .join(format!("{source_id}.jsonl"));
    read_jsonl_objects_local(&path)
        .into_iter()
        .rfind(|row| row.get("stage").and_then(Value::as_str) == Some("deep"))
        .unwrap_or_else(|| json!({}))
}

fn fallback_repo_brief(title: &str, url: Option<&str>, pattern: &str) -> Value {
    match pattern {
        "ecosystem_catalog" => json!({
            "method_summary": "Curated ecosystem map for agent frameworks and tool candidates",
            "implementation_implications": [
                "Extract the highest-signal projects from the catalog into bounded sovereign candidate records.",
                "Separate framework references from directly productizable runtimes so promotion pressure stays disciplined.",
            ],
            "risks": ["List repos can inflate backlog volume unless they are deduplicated and scored before promotion."],
            "source_url": url.unwrap_or(title),
        }),
        "community_signal_map" => json!({
            "method_summary": "External community map for operator, builder, and market signals",
            "implementation_implications": [
                "Track community surfaces as signal sources rather than treating them as direct runtime dependencies.",
                "Route useful community intelligence into HERMES and boardroom observation policies.",
            ],
            "risks": ["Community lists drift quickly and can create noisy intake if not bounded by source quality controls."],
            "source_url": url.unwrap_or(title),
        }),
        "research_workflow_pattern" => json!({
            "method_summary": "Research workflow orchestration pattern for autonomous evidence loops",
            "implementation_implications": [
                "Translate the workflow into APOLLO-compatible stages instead of mirroring the upstream project wholesale.",
                "Bind research-loop outputs back into ATHENA books and PROMETHEUS prioritization surfaces.",
            ],
            "risks": ["Unbounded autonomous research loops can consume resources without clear promotion gates."],
            "source_url": url.unwrap_or(title),
        }),
        "search_runtime" => json!({
            "method_summary": "Self-hosted search aggregation runtime for governed retrieval",
            "implementation_implications": [
                "Define a bounded search-runtime contract before treating the source as a default retrieval backend.",
                "Keep retrieval policy and package posture explicit so search is observable and reversible.",
            ],
            "risks": ["Search backends can broaden external exposure if privacy and retention posture are not explicit."],
            "source_url": url.unwrap_or(title),
        }),
        _ => json!({
            "method_summary": "Repo-backed implementation candidate requiring bounded sovereign mapping",
            "implementation_implications": [
                "Translate the source into a bounded contract, workflow, or package posture instead of copying the upstream repo directly.",
            ],
            "risks": ["Digesting a repo is not the same as productizing it; bounded state surfaces must be defined before activation."],
            "source_url": url.unwrap_or(title),
        }),
    }
}

fn source_brief(
    source_id: &str,
    title: &str,
    url: Option<&str>,
    pattern: &str,
    root: &std::path::Path,
) -> Value {
    let deep = latest_deep_book(source_id, root);
    let data = deep.get("data").cloned().unwrap_or_else(|| json!({}));
    if let Some(brief) = data.get("implementation_brief").cloned() {
        if brief.is_object() {
            return brief;
        }
    }
    fallback_repo_brief(title, url, pattern)
}

fn downstream_task_templates(source_id: &str, title: &str, pattern: &str) -> Value {
    let rows = match pattern {
        "bounded_ingest_runtime" => vec![
            json!({"emitter_owner":"athena","owner":"athena","slug":"native_runtime_receipts","title":format!("Absorption follow-through for {source_id}: validate native runtime receipts for {title}"),"priority":"high","notes":"Use the absorbed source to harden ATHENA receipts, artifact capture, and bounded native runtime verification."}),
            json!({"emitter_owner":"prometheus","owner":"manwe","slug":"promotion_gate_policy","title":format!("Absorption follow-through for {source_id}: codify promotion-gate routing policy for {title}"),"priority":"high","notes":"Translate the absorbed source into explicit route and promotion-gate policy instead of relying on informal preference."}),
        ],
        "ecosystem_catalog" => vec![
            json!({"emitter_owner":"athena","owner":"athena","slug":"candidate_extraction","title":format!("Absorption follow-through for {source_id}: extract ranked candidates from {title}"),"priority":"high","notes":"Convert the absorbed catalog into deduplicated ATHENA candidate records with bounded relevance tags and evidence links."}),
            json!({"emitter_owner":"prometheus","owner":"prometheus","slug":"portfolio_ranking","title":format!("Absorption follow-through for {source_id}: rank promotion portfolio from {title}"),"priority":"high","notes":"Use the absorbed catalog to define a promote-now/signal/reference portfolio rather than leaving the list as passive evidence."}),
        ],
        "community_signal_map" => vec![
            json!({"emitter_owner":"athena","owner":"athena","slug":"community_signal_curation","title":format!("Absorption follow-through for {source_id}: curate community signals from {title}"),"priority":"medium","notes":"Turn the absorbed community map into bounded ATHENA signal records rather than broad unfiltered community backlog."}),
            json!({"emitter_owner":"prometheus","owner":"hermes","slug":"community_intel_policy","title":format!("Absorption follow-through for {source_id}: define community-intelligence intake policy for {title}"),"priority":"medium","notes":"Bind community-source monitoring to HERMES and boardroom policies instead of treating external communities as direct dependencies."}),
        ],
        "research_workflow_pattern" => vec![
            json!({"emitter_owner":"athena","owner":"athena","slug":"workflow_pattern_brief","title":format!("Absorption follow-through for {source_id}: map workflow primitives from {title}"),"priority":"high","notes":"Extract reusable research-loop primitives into ATHENA evidence and implementation briefs."}),
            json!({"emitter_owner":"prometheus","owner":"apollo","slug":"workflow_contract","title":format!("Absorption follow-through for {source_id}: emit workflow contract from {title}"),"priority":"high","notes":"Translate the absorbed workflow into bounded APOLLO execution stages and governor-visible checkpoints."}),
        ],
        "search_runtime" => vec![
            json!({"emitter_owner":"athena","owner":"athena","slug":"retrieval_adapter_contract","title":format!("Absorption follow-through for {source_id}: define retrieval adapter contract for {title}"),"priority":"high","notes":"Use the absorbed source to define a governed ATHENA retrieval adapter contract with explicit privacy and retention posture."}),
            json!({"emitter_owner":"prometheus","owner":"prometheus","slug":"search_runtime_posture","title":format!("Absorption follow-through for {source_id}: define package posture for {title}"),"priority":"high","notes":"Create the bounded package/runtime posture for the absorbed search runtime before any activation decision."}),
        ],
        _ => vec![
            json!({"emitter_owner":"athena","owner":"athena","slug":"implementation_mapping","title":format!("Absorption follow-through for {source_id}: map implementation deltas from {title}"),"priority":"high","notes":"Extract bounded implementation deltas from the absorbed source into ATHENA books and contracts."}),
            json!({"emitter_owner":"prometheus","owner":"prometheus","slug":"productization_mapping","title":format!("Absorption follow-through for {source_id}: define productization mapping for {title}"),"priority":"high","notes":"Convert the absorbed source into an explicit sovereign productization path instead of leaving it as passive evidence."}),
        ],
    };
    Value::Array(rows)
}

fn queued_absorption_tasks(latest: &HashMap<String, Value>, source_id: &str) -> Value {
    let mut rows = latest
        .values()
        .filter_map(|row| {
            let meta = row.get("meta")?;
            if row.get("status").and_then(Value::as_str) != Some("queued")
                || meta.get("origin").and_then(Value::as_str) != Some("source_absorption_pipeline")
                || meta.get("source_id").and_then(Value::as_str) != Some(source_id)
            {
                return None;
            }
            Some(json!({
                "task_id": row.get("id").cloned().unwrap_or(Value::Null),
                "owner": row.get("owner").cloned().unwrap_or(Value::Null),
                "title": row.get("title").cloned().unwrap_or(Value::Null),
                "queued_at_utc": row.get("queued_at_utc").cloned().unwrap_or(Value::Null),
            }))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        row.get("queued_at_utc")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    });
    Value::Array(rows)
}

fn ecosystem_candidate_rank(row: &Value) -> (i32, i64) {
    let activation = row
        .get("package_enablement")
        .and_then(|v| v.get("activation_status"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let confidence = row
        .get("athena")
        .and_then(|v| v.get("confidence"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let activation_rank = match activation {
        "active_in_system" => 0,
        "governed_on_demand" => 1,
        "active_signal" => 2,
        "planned" => 3,
        _ => 4,
    };
    (activation_rank, -(confidence * 1000.0) as i64)
}

fn select_tasks(tasks: &[Value], status: &str) -> Vec<Value> {
    let mut filtered = tasks
        .iter()
        .filter(|task| task.get("status").and_then(Value::as_str) == Some(status))
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|a, b| {
        b.get("queued_at_utc")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("queued_at_utc").and_then(Value::as_str).unwrap_or(""))
    });
    filtered
}

fn collect_human_corpus_root(
    workspace_root: &std::path::Path,
    root_name: &str,
    base: &std::path::Path,
    max_entries: usize,
) -> (Value, Vec<Value>) {
    if !base.exists() {
        return (
            json!({
                "root_id": root_name,
                "path": base.to_string_lossy(),
                "present": false,
                "files_total": 0,
                "markdown_total": 0,
                "high_priority_total": 0,
            }),
            Vec::new(),
        );
    }
    let mut files = walk_files(base);
    files.sort_by(|a, b| {
        let a_size = a.metadata().map(|m| m.len()).unwrap_or(0);
        let b_size = b.metadata().map(|m| m.len()).unwrap_or(0);
        (std::cmp::Reverse(a_size), a.to_string_lossy().as_ref())
            .cmp(&(std::cmp::Reverse(b_size), b.to_string_lossy().as_ref()))
    });

    let mut entries = Vec::new();
    let mut markdown_total = 0usize;
    let mut high_priority_total = 0usize;
    for path in files.iter().take(max_entries) {
        let class = human_corpus_file_class(path);
        let priority = infer_human_corpus_priority(path, root_name);
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false)
        {
            markdown_total += 1;
        }
        if priority == "high" {
            high_priority_total += 1;
        }
        let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
        let relative_path = path
            .strip_prefix(base)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        entries.push(json!({
            "path": path.to_string_lossy(),
            "canonical_path": canonicalize_human_corpus_path(workspace_root, path),
            "relative_path": relative_path,
            "class": class,
            "priority": priority,
            "size_bytes": size_bytes,
            "root_id": root_name,
        }));
    }

    (
        json!({
            "root_id": root_name,
            "path": base.to_string_lossy(),
            "canonical_path": canonicalize_human_corpus_path(workspace_root, base),
            "present": true,
            "files_total": files.len(),
            "markdown_total": markdown_total,
            "high_priority_total": high_priority_total,
        }),
        entries,
    )
}

fn walk_files(base: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut stack = vec![base.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else if entry_path.is_file() {
                files.push(entry_path);
            }
        }
    }
    files
}

fn canonicalize_human_corpus_path(
    workspace_root: &std::path::Path,
    path: &std::path::Path,
) -> String {
    let resolved_workspace = workspace_root.canonicalize().ok();
    let resolved_path = path.canonicalize().ok();
    if let (Some(root), Some(resolved)) = (resolved_workspace, resolved_path) {
        if let Ok(relative) = resolved.strip_prefix(root) {
            let mut parts = relative
                .iter()
                .map(|part| part.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            if parts.len() >= 2 && parts[0] == "human" && parts[1] == "Notes" {
                parts[1] = "notes".to_string();
            }
            return parts.join("/");
        }
    }
    path.to_string_lossy().to_string()
}

fn human_corpus_file_class(path: &std::path::Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
        .unwrap_or_default();
    match ext.as_str() {
        ".md" | ".txt" | ".json" | ".jsonl" | ".yaml" | ".yml" | ".toml" => "text",
        ".docx" | ".pdf" => "document",
        ".zip" => "archive",
        _ => "other",
    }
}

fn infer_human_corpus_priority(path: &std::path::Path, root_name: &str) -> &'static str {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let full = path.to_string_lossy().to_ascii_lowercase();
    if root_name == "human_notes"
        && (name.ends_with("_spec.docx") || name.contains("spec") || name.contains("gdd"))
    {
        return "high";
    }
    if name.contains("master_operations_plan") || name.contains("the_soterion") {
        return "high";
    }
    if full.contains("architecture") || name.contains("contract") || name.contains("framework") {
        return "high";
    }
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
    {
        return "medium";
    }
    "deferred_extract"
}

fn top_human_corpus_candidates(entries: &[Value], limit: usize) -> Vec<Value> {
    let mut scored = entries.to_vec();
    scored.sort_by(|a, b| {
        let a_priority = match a.get("priority").and_then(Value::as_str) {
            Some("high") => 0,
            Some("medium") => 1,
            _ => 2,
        };
        let b_priority = match b.get("priority").and_then(Value::as_str) {
            Some("high") => 0,
            Some("medium") => 1,
            _ => 2,
        };
        let a_class = match a.get("class").and_then(Value::as_str) {
            Some("text") => 0,
            Some("document") => 1,
            _ => 2,
        };
        let b_class = match b.get("class").and_then(Value::as_str) {
            Some("text") => 0,
            Some("document") => 1,
            _ => 2,
        };
        let a_size = a.get("size_bytes").and_then(Value::as_u64).unwrap_or(0);
        let b_size = b.get("size_bytes").and_then(Value::as_u64).unwrap_or(0);
        (
            a_priority,
            a_class,
            std::cmp::Reverse(a_size),
            a.get("path").and_then(Value::as_str).unwrap_or(""),
        )
            .cmp(&(
                b_priority,
                b_class,
                std::cmp::Reverse(b_size),
                b.get("path").and_then(Value::as_str).unwrap_or(""),
            ))
    });
    scored.into_iter().take(limit).collect()
}

fn workflow_step(
    step_id: &str,
    name: &str,
    action: &str,
    payload: Value,
    deps: Vec<&str>,
) -> Value {
    json!({
        "id": step_id,
        "name": name,
        "action": action,
        "payload": payload,
        "dependencies": deps,
        "retry_count": 0,
        "max_retries": 1,
    })
}

fn readiness_label(
    executor_ready: bool,
    async_ready: bool,
    bounded_contracts: i64,
    operator_load: i64,
) -> &'static str {
    if executor_ready && async_ready && bounded_contracts >= 3 && operator_load == 0 {
        "ready"
    } else if executor_ready && bounded_contracts >= 2 {
        "guided"
    } else {
        "planned"
    }
}

#[allow(clippy::too_many_arguments)]
fn domain_row(
    domain_id: &str,
    mission: &str,
    primary_intake: &str,
    primary_executor: &str,
    deliberation_owner: &str,
    contracts: Value,
    signals: Value,
    default_provider_profile: &str,
    async_allowed: bool,
    human_override_points: Value,
    routing_targets: Vec<Value>,
    readiness: &str,
) -> Value {
    json!({
        "domain_id": domain_id,
        "mission": mission,
        "primary_intake": primary_intake,
        "primary_executor": primary_executor,
        "deliberation_owner": deliberation_owner,
        "readiness": readiness,
        "routing_policy": {
            "async_allowed": async_allowed,
            "default_provider_profile": default_provider_profile,
            "routing_targets": routing_targets,
            "human_override_points": human_override_points,
        },
        "contracts": contracts,
        "signals": signals,
    })
}

fn filter_targets(available_targets: &[String], preferred: &[&str]) -> Vec<Value> {
    preferred
        .iter()
        .filter(|target| {
            available_targets
                .iter()
                .any(|available| available == **target)
        })
        .map(|target| Value::String((*target).to_string()))
        .collect()
}

fn profile_or<'a>(profiles: &'a [String], wanted: &'a str, fallback: &'a str) -> &'a str {
    if profiles.iter().any(|profile| profile == wanted) {
        wanted
    } else {
        fallback
    }
}
