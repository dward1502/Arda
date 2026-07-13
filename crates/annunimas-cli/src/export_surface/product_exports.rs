use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::*;

pub(crate) fn export_crate_spawn_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/crate_spawn_contract.json");
    let alignment = read_json_or(&root.join("core/state/openfang_alignment.json"), json!({}));
    let patterns = alignment
        .get("pattern_extraction")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let payload = json!({
        "schema_version": "annunimas.crate-spawn-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "openfang_alignment_plus_sovereign_spawn_blueprint",
        "source_alignment": {
            "source_id": alignment.get("source_id").cloned().unwrap_or(Value::Null),
            "source_url": alignment.get("source_url").cloned().unwrap_or(Value::Null),
            "pattern": patterns.get("hands_to_spawnable_crates").cloned().unwrap_or_else(|| json!({})),
        },
        "scaffold_flow": {
            "command": "annunimas-cli utility create-crate-spawn-blueprint",
            "target_root_default": "crates",
            "required_files": [
                "Cargo.toml",
                "src/lib.rs",
                "src/contract.rs",
                "src/service.rs",
                "README.md",
                "tests/contract_smoke.rs",
            ],
            "required_contracts": [
                "task_ledger_linked",
                "state_export_defined",
                "soterion_trace_defined",
                "arda_visibility_defined",
                "productizable_boundary_declared",
                "memory_checkpoint_expected",
            ],
        },
        "defaults": {
            "realm": "operations",
            "productizable": true,
            "runtime_mode": "local-sovereign",
            "metrics_export": "core/state/<crate>.json",
            "workspace_dependencies_required": [
                "annunimas-core",
                "annunimas-governance",
            ],
        },
        "governance_validators": {
            "triad_required": true,
            "bacon_lite_required": true,
            "joulework_required": true,
            "love_equation_required": true,
            "soterion_trace_required": true,
        },
        "continuity_requirements": {
            "mnemosyne_checkpoint_required": true,
            "task_pivot_linkage_required": true,
            "arda_visibility_required": true,
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_opencode_project_runtime_impl() -> Result<Value> {
    let root = workspace_root();
    let config_out_path = root.join(".opencode/oh-my-opencode.json");
    let state_out_path = root.join("core/state/opencode_project_runtime.json");
    let surface = read_json_or(
        &root.join("core/state/model_control_surface.json"),
        json!({}),
    );
    let providers = provider_map(&surface);
    let advisor = surface
        .get("routing_advisor")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let opencode_agents = advisor
        .get("opencode_agents")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let local_light = "ollama/qwen2.5-coder-3b";
    let local_medium = "ollama/qwen2.5-coder-7b";
    let local_heavy = "ollama/qwen2.5-coder-14b-q5km";
    let cloud_reasoning =
        pick_cloud_reasoning(&providers).unwrap_or_else(|| "openrouter/auto".to_string());
    let cloud_code = "openrouter/auto".to_string();

    let prefers_cloud = |agent_id: &str| -> bool {
        opencode_agents
            .get(agent_id)
            .and_then(|v| v.get("recommended_provider"))
            .and_then(Value::as_str)
            .map(|provider_id| matches!(provider_id, "openrouter" | "litellm_gateway"))
            .unwrap_or(false)
    };
    let load_shed_active = |agent_id: &str| -> bool {
        opencode_agents
            .get(agent_id)
            .and_then(|v| v.get("load_shed_active"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };

    let reasoning_primary = if prefers_cloud("plan") || load_shed_active("plan") {
        cloud_reasoning.clone()
    } else {
        local_medium.to_string()
    };
    let code_primary = if prefers_cloud("explore") || prefers_cloud("plan") {
        cloud_code.clone()
    } else {
        local_medium.to_string()
    };

    let config = json!({
        "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json",
        "project_name": "Annunimas",
        "model_fallback": true,
        "tmux": {
            "enabled": false
        },
        "background_task": {
            "defaultConcurrency": 1,
            "providerConcurrency": {
                "openrouter": 1,
                "google": 1,
                "ollama": 1,
                "openai": 1,
            },
            "modelConcurrency": {
                local_heavy: 1,
                local_medium: 1,
                local_light: 2,
                cloud_reasoning.clone(): 1,
                cloud_code.clone(): 1,
            },
            "staleTimeoutMs": 120000,
            "messageStalenessTimeoutMs": 180000,
            "syncPollTimeoutMs": 120000,
        },
        "disabled_hooks": [
            "agent-usage-reminder",
            "todo-continuation-enforcer",
            "unstable-agent-babysitter",
        ],
        "sisyphus_agent": {
            "planner_enabled": false,
            "replace_plan": false,
        },
        "agents": {
            "sisyphus": {
                "model": code_primary.clone(),
                "fallback_models": build_fallbacks(&[Some(cloud_code.clone()), Some(local_heavy.to_string()), Some(local_medium.to_string()), Some(local_light.to_string())]),
            },
            "hephaestus": {
                "model": code_primary.clone(),
                "fallback_models": build_fallbacks(&[Some(cloud_code.clone()), Some(local_medium.to_string()), Some(local_light.to_string())]),
            },
            "prometheus": {
                "model": reasoning_primary.clone(),
                "fallback_models": build_fallbacks(&[Some(cloud_reasoning.clone()), Some(local_medium.to_string()), Some(local_light.to_string())]),
            },
            "oracle": {
                "model": reasoning_primary.clone(),
                "fallback_models": build_fallbacks(&[Some(cloud_reasoning.clone()), Some(local_medium.to_string()), Some(local_light.to_string())]),
            },
            "librarian": {
                "model": local_light,
                "fallback_models": build_fallbacks(&[Some(local_medium.to_string()), Some(cloud_reasoning.clone())]),
            },
            "explore": {
                "model": local_light,
                "fallback_models": build_fallbacks(&[Some(local_medium.to_string()), Some(cloud_code.clone())]),
            },
        },
        "categories": {
            "ultrabrain": {
                "model": cloud_reasoning.clone(),
                "fallback_models": build_fallbacks(&[Some(cloud_code.clone()), Some(local_heavy.to_string()), Some(local_medium.to_string())]),
            },
            "deep": {
                "model": reasoning_primary.clone(),
                "fallback_models": build_fallbacks(&[Some(local_medium.to_string()), Some(cloud_reasoning.clone()), Some(local_light.to_string())]),
            },
            "quick": {
                "model": local_light,
                "fallback_models": build_fallbacks(&[Some(local_medium.to_string())]),
            },
            "unspecified-high": {
                "model": reasoning_primary.clone(),
                "fallback_models": build_fallbacks(&[Some(cloud_reasoning.clone()), Some(local_medium.to_string()), Some(local_light.to_string())]),
            },
            "unspecified-low": {
                "model": local_light,
                "fallback_models": build_fallbacks(&[Some(local_medium.to_string())]),
            },
            "writing": {
                "model": local_light,
                "fallback_models": build_fallbacks(&[Some(local_medium.to_string()), Some(cloud_reasoning.clone())]),
            },
        },
    });

    let runtime_state = json!({
        "schema_version": "annunimas.opencode-project-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "model_control_surface + bounded_opencode_project_runtime",
        "source_surfaces": {
            "model_control_surface": "core/state/model_control_surface.json",
            "project_config": ".opencode/oh-my-opencode.json",
        },
        "summary": {
            "reasoning_primary": reasoning_primary,
            "code_primary": code_primary,
            "cloud_reasoning_available": providers.get("openrouter").map(available).unwrap_or(false)
                || providers.get("google").map(available).unwrap_or(false),
        },
        "routing_inputs": {
            "recommended_origin": advisor.get("recommended_origin").cloned().unwrap_or(Value::Null),
            "opencode_agents": opencode_agents,
        },
        "generated_config": config,
    });

    write_pretty_json(&config_out_path, &config)?;
    write_pretty_json(&state_out_path, &runtime_state)?;
    Ok(json!({
        "ok": true,
        "config": rel(&config_out_path, &root),
        "state": rel(&state_out_path, &root),
    }))
}

pub(crate) fn export_source_lesson_embodiment_registry_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_lesson_embodiment_registry.json");
    let portfolio = read_json_or(
        &root.join("core/state/source_absorption_portfolio.json"),
        json!({}),
    );
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let tasks = latest_tasks(&root);
    let policy_latest = latest_rows(
        &read_jsonl_objects(&root.join("data/athena/policy_readiness.jsonl")),
        "source_id",
    );

    let candidate_sources = portfolio
        .get("sources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.get("disposition").and_then(Value::as_str) == Some("promote_now"))
        .collect::<Vec<_>>();
    let package_candidates = package_scope_rows(&package_enablement);
    let scholarly_candidates = scholarly_scope_rows(&policy_latest)
        .into_iter()
        .filter(|row| row.get("source_id").and_then(Value::as_str) == Some("src_4ecbcb57"))
        .collect::<Vec<_>>();

    let package_rows = package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row.get("source_id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let runtime_surfaces = package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let contract_summaries = contract_summaries(&root);

    let mut merged_sources = BTreeMap::new();
    for source in package_candidates
        .into_iter()
        .chain(candidate_sources.clone())
        .chain(scholarly_candidates)
    {
        let Some(source_id) = source.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        merged_sources
            .entry(source_id.to_string())
            .or_insert(source);
    }

    let mut sources = Vec::new();
    for source in merged_sources.into_values() {
        let Some(source_id) = source.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        let task_status = source_latest_task_status(source_id, &tasks);
        let (embodiment_status, status_reason) = source_status(
            &root,
            source_id,
            source
                .get("disposition")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &package_rows,
            &runtime_surfaces,
            &contract_summaries,
            task_status.as_deref(),
        );
        sources.push(json!({
            "source_id": source_id,
            "title": source.get("title").cloned().unwrap_or(Value::Null),
            "url": source.get("url").cloned().unwrap_or(Value::Null),
            "source_kind": source.get("source_kind").cloned().unwrap_or_else(|| json!("github")),
            "domain": source.get("domain").cloned().unwrap_or(Value::Null),
            "disposition": source.get("disposition").cloned().unwrap_or(Value::Null),
            "absorption_pattern": source.get("absorption_pattern").cloned().unwrap_or(Value::Null),
            "embodiment_status": embodiment_status,
            "status_reason": status_reason,
            "task_status": task_status,
            "artifacts": source_artifacts(source_id),
            "lessons": build_lesson_rows(&source, &embodiment_status, &status_reason, task_status.as_deref()),
            "policy_gate": source.get("policy_gate").cloned().unwrap_or_else(|| json!({})),
        }));
    }

    let mut by_status = BTreeMap::new();
    let mut by_domain = BTreeMap::new();
    let mut unembodied_sources_total = 0usize;
    for row in &sources {
        let status = row
            .get("embodiment_status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let domain = row
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *by_status.entry(status.clone()).or_insert(0usize) += 1;
        *by_domain.entry(domain).or_insert(0usize) += 1;
        if matches!(
            status.as_str(),
            "contract_only" | "evidence_only" | "queued_for_embodiment"
        ) {
            unembodied_sources_total += 1;
        }
    }
    let top_unembodied = sources
        .iter()
        .filter(|row| {
            matches!(
                row.get("embodiment_status").and_then(Value::as_str),
                Some("contract_only" | "evidence_only" | "queued_for_embodiment")
            )
        })
        .take(10)
        .map(|row| {
            json!({
                "source_id": row.get("source_id").cloned().unwrap_or(Value::Null),
                "title": row.get("title").cloned().unwrap_or(Value::Null),
                "embodiment_status": row.get("embodiment_status").cloned().unwrap_or(Value::Null),
                "status_reason": row.get("status_reason").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "annunimas.source-lesson-embodiment-registry.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_policy_readiness + source_absorption_portfolio + runtime_contracts",
        "mission": {
            "goal": "Track whether strong ATHENA source lessons have been embodied into sovereign code, runtime, or contract surfaces.",
            "anti_drift_rule": "Strong-source implementation ideas must resolve into embodied_active, embodied_governed, contract_only, queued_for_embodiment, evidence_only, or rejected states.",
        },
        "summary": {
            "sources_total": sources.len(),
            "lesson_total": sources.iter().map(|row| row.get("lessons").and_then(Value::as_array).map(Vec::len).unwrap_or(0)).sum::<usize>(),
            "by_status": by_status,
            "by_domain": by_domain,
            "unembodied_sources_total": unembodied_sources_total,
        },
        "top_unembodied_sources": top_unembodied,
        "sources": sources,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "sources_total": payload.get("summary").and_then(|v| v.get("sources_total")).cloned().unwrap_or(Value::Null),
    }))
}

pub(crate) fn export_source_lesson_embodiment_backlog_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/source_lesson_embodiment_backlog.json");
    let registry = read_json_or(
        &root.join("core/state/source_lesson_embodiment_registry.json"),
        json!({}),
    );
    let rows = backlog_rows(&registry);
    let payload = json!({
        "schema_version": "annunimas.source-lesson-embodiment-backlog.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_lesson_embodiment_registry",
        "summary": {
            "backlog_total": rows.len(),
            "critical_embodiment_frontiers_total": rows.iter().filter(|row| row.get("priority").and_then(Value::as_i64).unwrap_or(0) >= 90).count(),
        },
        "backlog": rows,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "backlog_total": payload.get("summary").and_then(|v| v.get("backlog_total")).cloned().unwrap_or(Value::Null),
    }))
}

fn provider_map(surface: &Value) -> BTreeMap<String, Value> {
    surface
        .get("charon_providers")
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

fn normalize_model(provider_id: &str, model_id: Option<&str>) -> Option<String> {
    let model_id = model_id?;
    if matches!(
        provider_id,
        "openrouter" | "google" | "openai" | "anthropic" | "ollama"
    ) {
        if model_id.starts_with(&format!("{provider_id}/")) {
            Some(model_id.to_string())
        } else {
            Some(format!("{provider_id}/{model_id}"))
        }
    } else {
        Some(model_id.to_string())
    }
}

fn first_model(provider: &Value, capable_task: Option<&str>, prefer_free: bool) -> Option<String> {
    let mut candidates = Vec::new();
    for row in provider
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let model_id = row.get("id").and_then(Value::as_str)?;
        let capable_tasks = row.get("capable_tasks").and_then(Value::as_array);
        if let Some(task) = capable_task {
            if let Some(capable_tasks) = capable_tasks {
                if !capable_tasks
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|value| value == task)
                {
                    continue;
                }
            }
        }
        candidates.push(model_id.to_string());
    }
    if prefer_free {
        if let Some(model_id) = candidates
            .iter()
            .find(|model_id| model_id.contains(":free"))
        {
            return Some(model_id.clone());
        }
    }
    candidates.into_iter().next().or_else(|| {
        provider
            .get("default_model")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn available(provider: &Value) -> bool {
    provider
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && provider
            .get("available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && provider
            .get("healthy")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn pick_cloud_reasoning(providers: &BTreeMap<String, Value>) -> Option<String> {
    let openrouter = providers.get("openrouter");
    if let Some(provider) = openrouter {
        if available(provider) {
            if let Some(model_id) = first_model(provider, Some("reasoning"), true) {
                if let Some(normalized) = normalize_model("openrouter", Some(&model_id)) {
                    return Some(normalized);
                }
            }
        }
    }
    let google = providers.get("google");
    if let Some(provider) = google {
        if available(provider) {
            if let Some(model_id) = first_model(provider, Some("research"), false) {
                if let Some(normalized) = normalize_model("google", Some(&model_id)) {
                    return Some(normalized);
                }
            }
        }
    }
    None
}

fn build_fallbacks(models: &[Option<String>]) -> Vec<String> {
    let mut out = Vec::new();
    for model in models.iter().flatten() {
        if !out.contains(model) {
            out.push(model.clone());
        }
    }
    out
}

fn latest_rows(rows: &[Value], key: &str) -> BTreeMap<String, Value> {
    let mut latest = BTreeMap::new();
    for row in rows {
        let Some(row_key) = row.get(key).and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if !row_key.is_empty() {
            latest.insert(row_key.to_string(), row.clone());
        }
    }
    latest
}

fn latest_tasks(root: &std::path::Path) -> BTreeMap<String, Value> {
    latest_rows(
        &read_jsonl_objects(&root.join("core/projects/tasks/queue.jsonl")),
        "id",
    )
}

fn source_latest_task_status(source_id: &str, tasks: &BTreeMap<String, Value>) -> Option<String> {
    let mut statuses = Vec::new();
    for row in tasks.values() {
        if row
            .get("meta")
            .and_then(|v| v.get("source_id"))
            .and_then(Value::as_str)
            == Some(source_id)
        {
            statuses.push(
                row.get("status")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            );
        }
    }
    if statuses.iter().any(|value| value == "queued") {
        Some("queued".to_string())
    } else if statuses.iter().any(|value| value == "blocked") {
        Some("blocked".to_string())
    } else if statuses.iter().any(|value| value == "in_progress") {
        Some("in_progress".to_string())
    } else if statuses.iter().any(|value| value == "completed") {
        Some("completed".to_string())
    } else {
        None
    }
}

fn scholarly_scope_rows(policy_rows: &BTreeMap<String, Value>) -> Vec<Value> {
    let mut rows = Vec::new();
    for row in policy_rows.values() {
        let deep = row.get("deep").cloned().unwrap_or_else(|| json!({}));
        let brief = deep
            .get("implementation_brief")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if brief == json!({}) {
            continue;
        }
        if !row
            .get("source_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains("scholarly")
        {
            continue;
        }
        rows.push(json!({
            "source_id": row.get("source_id").cloned().unwrap_or(Value::Null),
            "title": row.get("title").cloned().unwrap_or(Value::Null),
            "url": row.get("url").cloned().unwrap_or(Value::Null),
            "domain": "research",
            "disposition": row.get("policy_readiness").cloned().unwrap_or_else(|| json!("reference_only")),
            "absorption_pattern": "scholarly_brief",
            "implementation_brief": brief,
            "policy_gate": deep.get("policy_gate").cloned().unwrap_or_else(|| json!({})),
            "source_kind": "scholarly",
        }));
    }
    rows
}

fn package_scope_rows(package_enablement: &Value) -> Vec<Value> {
    let mut rows = Vec::new();
    for row in package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let source_id = row
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if source_id.is_empty() {
            continue;
        }
        let policy_readiness = row
            .get("policy_readiness")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let activation_status = row
            .get("activation_status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if policy_readiness != "policy_ready"
            && !matches!(
                activation_status,
                "active_in_system" | "governed_on_demand" | "active_signal"
            )
        {
            continue;
        }
        let tool = row
            .get("tool")
            .and_then(Value::as_str)
            .or_else(|| row.get("repo").and_then(Value::as_str))
            .unwrap_or(source_id);
        let lane = row
            .get("integration_lane")
            .and_then(Value::as_str)
            .unwrap_or("tool_runtime");
        let status = if activation_status.is_empty() {
            "contract_only"
        } else {
            activation_status
        };
        let implications = if status == "active_in_system" {
            vec![format!(
                "Operate `{tool}` as an active sovereign lane for `{lane}` work."
            )]
        } else if status == "active_signal" {
            vec![format!(
                "Use `{tool}` as a live decision/routing signal without turning it into the primary runtime."
            )]
        } else {
            vec![format!(
                "Keep `{tool}` bounded as governed-on-demand capability for `{lane}` work."
            )]
        };
        rows.push(json!({
            "source_id": source_id,
            "title": row.get("repo_url").cloned().or_else(|| row.get("repo").cloned()).unwrap_or_else(|| json!(tool)),
            "url": row.get("repo_url").cloned().unwrap_or(Value::Null),
            "domain": "development",
            "disposition": if status == "active_in_system" { json!("adopted") } else { json!("adapted") },
            "absorption_pattern": "registry_linked_tool",
            "implementation_brief": {
                "method_summary": format!("Registry-linked sovereign tool lane for `{tool}`"),
                "implementation_implications": implications,
                "risks": [
                    "Tool activation must remain bounded by explicit env, launcher, and governance surfaces."
                ],
                "source_url": row.get("repo_url").cloned().unwrap_or(Value::Null),
            },
            "source_kind": "registry_tool",
        }));
    }
    rows
}

fn source_artifacts(source_id: &str) -> Value {
    let mapping = match source_id {
        "src_df11630e" => vec![
            "core/state/scrapling_runtime_contract.json",
            "annunimas-cli utility scrapling-fetch",
            "crates/annunimas-athena/src/ingest.rs",
        ],
        "src_d46d1480" => vec![
            "core/state/crawl4ai_runtime_contract.json",
            "scripts/runtime/crawl4ai_service.sh",
            "core/state/package_runtime_activation.json",
        ],
        "src_dab89283" => vec![
            "core/state/litellm_routing_contract.json",
            "scripts/litellm_proxy.sh",
            "core/state/model_control_surface.json",
        ],
        "src_234088bc" => vec![
            "core/state/llmfit_routing_contract.json",
            "core/state/model_control_surface.json",
            "core/state/runtime_budget_policy.json",
        ],
        "src_bfd43480" => vec![
            "core/state/nanoclaw_productization_contract.json",
            "scripts/runtime/nanoclaw_runtime.sh",
            "core/state/package_runtime_activation.json",
        ],
        "src_ba31bde2" => vec![
            "core/state/playwright_mcp_productization_contract.json",
            "scripts/runtime/playwright_mcp_bridge.sh",
            "core/state/package_runtime_activation.json",
        ],
        "src_33fa61b2" | "src_ca2f031e" => vec![
            "core/state/source_ecosystem_registry.json",
            "core/state/source_ecosystem_operationalization.json",
            "core/state/source_absorption_portfolio.json",
        ],
        "src_dc355aed" => vec![
            "core/state/community_signal_intake.json",
            "core/state/communication_adapter_contract.json",
        ],
        "src_f226959a" => vec![
            "core/state/research_workflow_contract.json",
            "core/state/source_absorption_portfolio.json",
        ],
        "src_86fa4360" => vec![
            "core/state/search_runtime_contract.json",
            "scripts/runtime/searxng_service.sh",
            "core/state/package_runtime_activation.json",
        ],
        "src_4ecbcb57" => vec![
            "data/athena/books/src_4ecbcb57.jsonl",
            "core/state/socratic_validator_contract.json",
            "core/state/multi_domain_routing_contract.json",
        ],
        _ => Vec::new(),
    };
    json!(mapping)
}

fn contract_summaries(root: &std::path::Path) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    for (key, path) in [
        (
            "scrapling_runtime_contract",
            "core/state/scrapling_runtime_contract.json",
        ),
        (
            "crawl4ai_runtime_contract",
            "core/state/crawl4ai_runtime_contract.json",
        ),
        (
            "litellm_routing_contract",
            "core/state/litellm_routing_contract.json",
        ),
        (
            "llmfit_routing_contract",
            "core/state/llmfit_routing_contract.json",
        ),
        (
            "nanoclaw_productization_contract",
            "core/state/nanoclaw_productization_contract.json",
        ),
        (
            "playwright_mcp_productization_contract",
            "core/state/playwright_mcp_productization_contract.json",
        ),
        (
            "search_runtime_contract",
            "core/state/search_runtime_contract.json",
        ),
        (
            "research_workflow_contract",
            "core/state/research_workflow_contract.json",
        ),
        (
            "community_signal_intake",
            "core/state/community_signal_intake.json",
        ),
        (
            "source_ecosystem_registry",
            "core/state/source_ecosystem_registry.json",
        ),
    ] {
        map.insert(
            key.to_string(),
            read_json_or(&root.join(path), json!({}))
                .get("summary")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
    }
    map
}

fn source_status(
    root: &std::path::Path,
    source_id: &str,
    disposition: &str,
    package_rows: &BTreeMap<String, Value>,
    _runtime_surfaces: &serde_json::Map<String, Value>,
    contract_summaries: &BTreeMap<String, Value>,
    task_status: Option<&str>,
) -> (Value, Value) {
    let package_row = package_rows
        .get(source_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let activation = package_row
        .get("activation_status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if activation == "active_in_system" {
        return (
            json!("embodied_active"),
            json!("Runtime or routing lane is active in system."),
        );
    }
    if matches!(activation, "governed_on_demand" | "active_signal") {
        return (
            json!("embodied_governed"),
            json!("Lesson is embodied behind bounded, non-default runtime or routing controls."),
        );
    }
    match source_id {
        "src_df11630e" => {
            let summary = contract_summaries
                .get("scrapling_runtime_contract")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let passed = summary
                .get("promotion_gates_passed")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let total = summary
                .get("promotion_gates_total")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if total > 0 && passed >= total {
                return (
                    json!("embodied_governed"),
                    json!("Scrapling now has a bounded native-runtime contract and remains governed behind crawl4ai-first provider policy."),
                );
            }
            return (
                json!("contract_only"),
                json!(format!(
                    "Scrapling path exists, but promotion gates are only {passed}/{total}."
                )),
            );
        }
        "src_d46d1480" => {
            let crawl4ai = contract_summaries
                .get("crawl4ai_runtime_contract")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if crawl4ai
                .get("active_in_system")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || crawl4ai
                    .get("runtime_ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                return (
                    json!("embodied_active"),
                    json!("crawl4ai is embodied as the active sovereign ingest runtime."),
                );
            }
            return (
                json!("contract_only"),
                json!("crawl4ai contract exists, but active runtime posture is not confirmed."),
            );
        }
        "src_bfd43480" => {
            let nanoclaw = contract_summaries
                .get("nanoclaw_productization_contract")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if nanoclaw
                .get("runtime_ready")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || nanoclaw
                    .get("tailscale_ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                || nanoclaw
                    .get("lanes_total")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    > 0
            {
                return (
                    json!("embodied_governed"),
                    json!("NanoClaw is embodied as a bounded edge/runtime contract rather than passive evidence."),
                );
            }
            return (
                json!("contract_only"),
                json!(
                    "NanoClaw contract exists, but bounded runtime posture is not yet confirmed."
                ),
            );
        }
        "src_86fa4360" => {
            let search = contract_summaries
                .get("search_runtime_contract")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if matches!(
                search.get("activation_status").and_then(Value::as_str),
                Some("active_in_system" | "governed_on_demand")
            ) {
                if search.get("service_status").and_then(Value::as_str) == Some("running") {
                    return (
                        json!("embodied_governed"),
                        json!("Optional SearXNG runtime is live under bounded retrieval policy rather than serving as a default backend."),
                    );
                }
                return (
                    json!("embodied_governed"),
                    json!("Optional SearXNG runtime is embodied as governed-on-demand retrieval rather than a default resident backend."),
                );
            }
            return (
                json!("contract_only"),
                json!(format!(
                    "Search runtime is bounded as `{}` with service status `{}`.",
                    search
                        .get("activation_status")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    search
                        .get("service_status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                )),
            );
        }
        "src_33fa61b2" | "src_ca2f031e" | "src_dc355aed" | "src_f226959a" => {
            if matches!(source_id, "src_33fa61b2" | "src_ca2f031e")
                && root
                    .join("core/state/source_ecosystem_operationalization.json")
                    .exists()
            {
                return (
                    json!("embodied_governed"),
                    json!("Catalog lessons are now operationalized into recurring candidate extraction and portfolio ranking lanes."),
                );
            }
            if source_id == "src_dc355aed"
                && root
                    .join("core/state/hermes_community_sources.json")
                    .exists()
            {
                return (
                    json!("embodied_governed"),
                    json!("Community signal map is now bound into explicit HERMES intake source policy."),
                );
            }
            if source_id == "src_f226959a"
                && root
                    .join("core/state/apollo_research_workflow_runtime.json")
                    .exists()
            {
                return (
                    json!("embodied_governed"),
                    json!("Autoresearch workflow is now embodied as executable APOLLO workflow stage state."),
                );
            }
            return (
                json!("contract_only"),
                json!("Lesson is embodied as a sovereign policy/registry/workflow contract, not a live runtime."),
            );
        }
        _ => {}
    }

    if disposition == "reference_only" {
        if task_status == Some("queued") {
            return (
                json!("queued_for_embodiment"),
                json!("Evidence remains below policy gate but follow-on work is still queued."),
            );
        }
        return (
            json!("evidence_only"),
            json!("Source remains evidence-only because policy readiness is not sufficient for embodiment."),
        );
    }
    if matches!(task_status, Some("queued" | "blocked" | "in_progress")) {
        return (
            json!("queued_for_embodiment"),
            json!(format!(
                "Lesson promotion is still `{}` in the project queue.",
                task_status.unwrap_or_default()
            )),
        );
    }
    (
        json!("contract_only"),
        json!(
            "Source has implementation implications captured, but embodiment remains contractual."
        ),
    )
}

fn build_lesson_rows(
    source: &Value,
    source_status_id: &Value,
    status_reason: &Value,
    task_status: Option<&str>,
) -> Value {
    let brief = source
        .get("implementation_brief")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let implications = brief
        .get("implementation_implications")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::new();
    for (idx, implication) in implications.into_iter().enumerate() {
        let mut lesson_status = source_status_id.clone();
        if source_status_id == &json!("evidence_only") {
            lesson_status = json!("not_embodied");
        }
        rows.push(json!({
            "lesson_id": format!("{}:lesson_{}", source.get("source_id").and_then(Value::as_str).unwrap_or_default(), idx + 1),
            "statement": implication,
            "status": lesson_status,
            "task_status": task_status,
            "method_summary": brief.get("method_summary").cloned().unwrap_or(Value::Null),
            "risks": brief.get("risks").cloned().unwrap_or_else(|| json!([])),
            "status_reason": status_reason,
        }));
    }
    json!(rows)
}

fn backlog_rows(registry: &Value) -> Vec<Value> {
    let priorities = BTreeMap::from([
        (
            "src_df11630e",
            (
                100,
                "athena_charon",
                "Close Scrapling native-runtime promotion gate",
                "Replace shim-backed posture with native package/runtime proof and promote only after the final gate passes.",
            ),
        ),
        (
            "src_f226959a",
            (
                95,
                "apollo_athena",
                "Embody autoresearch workflow as executable APOLLO stages",
                "Turn the research workflow contract into reusable APOLLO execution primitives with ATHENA outputs and checkpoints.",
            ),
        ),
        (
            "src_dc355aed",
            (
                90,
                "hermes",
                "Bind community signal map into HERMES intake sources",
                "Translate community signal classes into real HERMES source policies and observable intake channels.",
            ),
        ),
        (
            "src_86fa4360",
            (
                85,
                "athena_prometheus",
                "Activate optional SearXNG search runtime under bounded policy",
                "Start the optional service when operator/runtime access permits and connect it to governed ATHENA retrieval receipts.",
            ),
        ),
        (
            "src_33fa61b2",
            (
                75,
                "prometheus",
                "Operationalize awesome-llm-agents ecosystem ranking",
                "Turn the registry catalog into promotion scoring and recurring candidate extraction instead of static reference.",
            ),
        ),
        (
            "src_ca2f031e",
            (
                74,
                "prometheus",
                "Operationalize awesome-ai-agents ecosystem ranking",
                "Turn the second ecosystem catalog into promotion scoring and recurring candidate extraction instead of static reference.",
            ),
        ),
    ]);

    let mut rows = Vec::new();
    for source in registry
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if !matches!(
            source.get("embodiment_status").and_then(Value::as_str),
            Some("contract_only" | "queued_for_embodiment")
        ) {
            continue;
        }
        let Some(source_id) = source.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        let Some((priority, owner, task, next_move)) = priorities.get(source_id) else {
            continue;
        };
        rows.push(json!({
            "source_id": source_id,
            "title": source.get("title").cloned().unwrap_or(Value::Null),
            "embodiment_status": source.get("embodiment_status").cloned().unwrap_or(Value::Null),
            "status_reason": source.get("status_reason").cloned().unwrap_or(Value::Null),
            "priority": priority,
            "owner": owner,
            "task": task,
            "next_move": next_move,
            "artifacts": source.get("artifacts").cloned().unwrap_or_else(|| json!([])),
            "lessons_total": source.get("lessons").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        }));
    }
    rows.sort_by(|a, b| {
        b.get("priority")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .cmp(&a.get("priority").and_then(Value::as_i64).unwrap_or(0))
    });
    rows
}
