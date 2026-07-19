#![cfg(feature = "full-cli")]
use super::*;

pub(crate) fn build_operations_briefing(
    root: &std::path::Path,
) -> anyhow::Result<serde_json::Value> {
    let core_root = root.join("core");
    if core_root.join("realm/boot.toml").exists() {
        let _ = crate::prometheus::CoreAutonomyProfile::load(&core_root);
    }

    let autopilot_state = read_json_or_default(
        &root.join("data/ceo/autopilot.state.json"),
        json!({"error": "autopilot_state_unavailable"}),
    );
    let queue_observability = queue_observability_snapshot();
    let provider_routing_posture = build_provider_routing_posture(root);
    let chronos_runtime = read_json_or_default(
        &root.join("core/state/chronos_runtime.json"),
        json!({"status": "unknown", "source": "core/state/chronos_runtime.json"}),
    );
    let plutus_runtime = read_json_or_default(
        &root.join("core/state/plutus_runtime.json"),
        json!({"status": "unknown", "source": "core/state/plutus_runtime.json"}),
    );
    let mnemosyne_continuity = read_json_or_default(
        &root.join("core/state/mnemosyne_continuity.json"),
        json!({"status": "unknown", "source": "core/state/mnemosyne_continuity.json"}),
    );
    let warden_guardhouse = read_json_or_default(
        &root.join("core/state/warden_guardhouse.json"),
        json!({"status": "unknown", "source": "core/state/warden_guardhouse.json"}),
    );
    let athena_runtime = read_json_or_default(
        &root.join("core/state/athena_runtime.json"),
        json!({"status": "unknown", "source": "core/state/athena_runtime.json"}),
    );
    let runtime_admission_receipts = read_json_or_default(
        &root.join("core/state/runtime_admission_receipts.json"),
        json!({"status": "unknown", "source": "core/state/runtime_admission_receipts.json"}),
    );
    let runtime_admission_recovery = read_json_or_default(
        &root.join("core/state/runtime_admission_recovery.json"),
        json!({"status": "unknown", "source": "core/state/runtime_admission_recovery.json"}),
    );

    Ok(build_operations_briefing_from_runtime_inputs(
        &autopilot_state,
        &queue_observability,
        &provider_routing_posture,
        &chronos_runtime,
        &plutus_runtime,
        &mnemosyne_continuity,
        &warden_guardhouse,
        &athena_runtime,
        &runtime_admission_receipts,
        &runtime_admission_recovery,
    ))
}

#[cfg(test)]
pub(crate) fn build_operations_briefing_from_inputs(
    autopilot_state: &serde_json::Value,
    queue_observability: &serde_json::Value,
    provider_routing_posture: &serde_json::Value,
) -> serde_json::Value {
    build_operations_briefing_from_runtime_inputs(
        autopilot_state,
        queue_observability,
        provider_routing_posture,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
    )
}

pub(crate) fn build_operations_briefing_from_runtime_inputs(
    autopilot_state: &serde_json::Value,
    queue_observability: &serde_json::Value,
    provider_routing_posture: &serde_json::Value,
    chronos_runtime: &serde_json::Value,
    plutus_runtime: &serde_json::Value,
    mnemosyne_continuity: &serde_json::Value,
    warden_guardhouse: &serde_json::Value,
    athena_runtime: &serde_json::Value,
    runtime_admission_receipts: &serde_json::Value,
    runtime_admission_recovery: &serde_json::Value,
) -> serde_json::Value {
    let pending_tasks = autopilot_state
        .get("queue")
        .and_then(|queue| queue.get("pending"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let effective_pending_tasks = queue_observability
        .get("summary")
        .and_then(|summary| summary.get("total_known_work_items"))
        .or_else(|| {
            queue_observability
                .get("summary")
                .and_then(|summary| summary.get("total_active_internal_tasks"))
        })
        .and_then(|value| value.as_u64())
        .unwrap_or(pending_tasks);
    let oldest_pending_secs = autopilot_state
        .get("queue")
        .and_then(|queue| queue.get("aging_oldest_pending_secs"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let effective_oldest_pending_secs = if effective_pending_tasks > 0 {
        oldest_pending_secs
    } else {
        0.0
    };
    let oldest_pending_hours = (oldest_pending_secs / 36.0).round() / 100.0;
    let alerts = autopilot_state
        .get("dashboard")
        .and_then(|dashboard| dashboard.get("alerts"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    if effective_pending_tasks > 0 {
                        return true;
                    }
                    item.get("source").and_then(|value| value.as_str()) != Some("queue")
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .map(serde_json::Value::Array)
        .unwrap_or_else(|| json!([]));
    let services = autopilot_state
        .get("services")
        .unwrap_or(&serde_json::Value::Null);
    let degraded_services = services
        .get("services")
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| {
                    let score_degraded = entry
                        .get("score")
                        .and_then(|value| value.as_f64())
                        .map(|score| score < 0.85)
                        .unwrap_or(false);
                    score_degraded
                })
                .map(|entry| {
                    json!({
                        "unit": entry.get("unit").and_then(|value| value.as_str()).unwrap_or("unknown"),
                        "active": entry.get("active").and_then(|value| value.as_str()).unwrap_or("unknown"),
                        "sub": entry.get("sub").and_then(|value| value.as_str()).unwrap_or("unknown"),
                        "note": entry.get("note").and_then(|value| value.as_str()).unwrap_or(""),
                        "score": entry.get("score").and_then(|value| value.as_f64()).unwrap_or(0.0),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let recommended_next_bounded_action = recommended_operations_action(
        effective_pending_tasks,
        effective_oldest_pending_secs,
        services
            .get("failed")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        services
            .get("degraded")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
    );

    json!({
        "contract": "arda.operations_briefing.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "latest_cycle_summary": {
            "timestamp": autopilot_state.get("timestamp").and_then(|value| value.as_str()).unwrap_or("unknown"),
            "pending_tasks": pending_tasks,
            "effective_pending_tasks": effective_pending_tasks,
            "recent_completions_24h": autopilot_state.get("queue").and_then(|queue| queue.get("recent_completions_24h")).and_then(|value| value.as_u64()).unwrap_or(0),
            "recent_failures_24h": autopilot_state.get("queue").and_then(|queue| queue.get("recent_failures_24h")).and_then(|value| value.as_u64()).unwrap_or(0),
            "completion_rate_24h": autopilot_state.get("queue").and_then(|queue| queue.get("completion_rate_24h")).and_then(|value| value.as_f64()).unwrap_or(0.0),
        },
        "alerts": alerts,
        "task_aging": {
            "oldest_pending_secs": oldest_pending_secs,
            "oldest_pending_hours": oldest_pending_hours,
            "over_24h": effective_oldest_pending_secs >= 86_400.0,
            "raw_over_24h": oldest_pending_secs >= 86_400.0,
        },
        "service_degradation": {
            "healthy_count": services.get("healthy").and_then(|value| value.as_u64()).unwrap_or(0),
            "degraded_count": services.get("degraded").and_then(|value| value.as_u64()).unwrap_or(0),
            "failed_count": services.get("failed").and_then(|value| value.as_u64()).unwrap_or(0),
            "overall_score": services.get("overall_score").and_then(|value| value.as_f64()).unwrap_or(0.0),
            "degraded_services": degraded_services,
        },
        "queue_posture": queue_observability,
        "provider_routing_posture": provider_routing_posture,
        "chronos_temporal_baseline": summarize_chronos_runtime(chronos_runtime),
        "plutus_economics_governance_joulework": summarize_plutus_runtime(plutus_runtime),
        "mnemosyne_memory_continuity": summarize_mnemosyne_continuity(mnemosyne_continuity),
        "warden_informant_fleet_posture": summarize_warden_guardhouse(warden_guardhouse),
        "athena_human_ingestion_policy_readiness": summarize_athena_runtime(athena_runtime),
        "runtime_admission_pressure_bacon_lite": summarize_runtime_admission(runtime_admission_receipts, runtime_admission_recovery, plutus_runtime),
        "recommended_next_bounded_action": recommended_next_bounded_action,
        "source_files": {
            "autopilot_state": "data/ceo/autopilot.state.json",
            "charon_router": "core/state/charon_router.json",
            "charon_route_history": "data/charon_route_smoke_history.jsonl",
            "chronos_runtime": "core/state/chronos_runtime.json",
            "plutus_runtime": "core/state/plutus_runtime.json",
            "mnemosyne_continuity": "core/state/mnemosyne_continuity.json",
            "warden_guardhouse": "core/state/warden_guardhouse.json",
            "athena_runtime": "core/state/athena_runtime.json",
            "runtime_admission_receipts": "core/state/runtime_admission_receipts.json",
            "runtime_admission_recovery": "core/state/runtime_admission_recovery.json",
            "mutated": false,
            "mutation_policy": "read_only_briefing_no_queue_or_receipt_rewrite"
        }
    })
}

pub(crate) fn format_operations_briefing_text(briefing: &serde_json::Value) -> String {
    let latest = briefing
        .get("latest_cycle_summary")
        .unwrap_or(&serde_json::Value::Null);
    let task_aging = briefing
        .get("task_aging")
        .unwrap_or(&serde_json::Value::Null);
    let services = briefing
        .get("service_degradation")
        .unwrap_or(&serde_json::Value::Null);
    let provider = briefing
        .get("provider_routing_posture")
        .unwrap_or(&serde_json::Value::Null);
    let chronos = briefing
        .get("chronos_temporal_baseline")
        .unwrap_or(&serde_json::Value::Null);
    let plutus = briefing
        .get("plutus_economics_governance_joulework")
        .unwrap_or(&serde_json::Value::Null);
    let mnemosyne = briefing
        .get("mnemosyne_memory_continuity")
        .unwrap_or(&serde_json::Value::Null);
    let warden = briefing
        .get("warden_informant_fleet_posture")
        .unwrap_or(&serde_json::Value::Null);
    let athena = briefing
        .get("athena_human_ingestion_policy_readiness")
        .unwrap_or(&serde_json::Value::Null);
    let admission = briefing
        .get("runtime_admission_pressure_bacon_lite")
        .unwrap_or(&serde_json::Value::Null);
    let source_files = briefing
        .get("source_files")
        .unwrap_or(&serde_json::Value::Null);
    let alerts = briefing
        .get("alerts")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let degraded_services = services
        .get("degraded_services")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lines = vec![
        "arda Operations Briefing".to_string(),
        format!(
            "Latest cycle: {}",
            latest
                .get("timestamp")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
        ),
        format!(
            "Queue: {} pending, oldest {:.2}h, completions_24h={}, failures_24h={}, completion_rate_24h={:.2}",
            latest
                .get("pending_tasks")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            task_aging
                .get("oldest_pending_hours")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0),
            latest
                .get("recent_completions_24h")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            latest
                .get("recent_failures_24h")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            latest
                .get("completion_rate_24h")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0),
        ),
        format!("Alerts: {}", alerts.len()),
    ];

    if alerts.is_empty() {
        lines.push("- none".to_string());
    } else {
        for alert in alerts.iter().take(5) {
            lines.push(format!(
                "- {} [{}]: {}",
                alert
                    .get("severity")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Info"),
                alert
                    .get("source")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                alert
                    .get("message")
                    .and_then(|value| value.as_str())
                    .unwrap_or("no message")
            ));
        }
        if alerts.len() > 5 {
            lines.push(format!("- ... {} more alert(s)", alerts.len() - 5));
        }
    }

    lines.extend([
        format!(
            "Services: {} healthy, {} degraded, {} failed, overall_score={:.4}",
            services
                .get("healthy_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            services
                .get("degraded_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            services
                .get("failed_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            services
                .get("overall_score")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0),
        ),
        "Degraded services:".to_string(),
    ]);

    if degraded_services.is_empty() {
        lines.push("- none".to_string());
    } else {
        for service in degraded_services.iter().take(5) {
            lines.push(format!(
                "- {}: {}/{} score={} note={}",
                service
                    .get("unit")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                service
                    .get("active")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                service
                    .get("sub")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                service
                    .get("score")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                service
                    .get("note")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
            ));
        }
        if degraded_services.len() > 5 {
            lines.push(format!(
                "- ... {} more degraded service(s)",
                degraded_services.len() - 5
            ));
        }
    }

    lines.extend([
        format!(
            "Provider/routing: {} (active_provider={})",
            provider
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            provider
                .get("active_provider")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
        ),
        format!(
            "Chronos: status={} feeds={}/{} stale={} next={}",
            chronos.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
            chronos.get("present_count").and_then(|value| value.as_u64()).unwrap_or(0),
            chronos.get("feed_count").and_then(|value| value.as_u64()).unwrap_or(0),
            chronos.get("stale_count").and_then(|value| value.as_u64()).unwrap_or(0),
            join_json_string_array(chronos.get("next_integration_steps"), 3, "none"),
        ),
        format!(
            "Plutus: budget_remaining={:.2} usage={:.2}% governance_records={} bacon_lite_passed={} joulework_total={:.2}",
            plutus.get("budget_remaining").and_then(|value| value.as_f64()).unwrap_or(0.0),
            plutus.get("budget_usage_percent").and_then(|value| value.as_f64()).unwrap_or(0.0),
            plutus.get("governance_records_total").and_then(|value| value.as_u64()).unwrap_or(0),
            plutus.get("bacon_lite_passed_total").and_then(|value| value.as_u64()).unwrap_or(0),
            plutus.get("joulework_total").and_then(|value| value.as_f64()).unwrap_or(0.0),
        ),
        format!(
            "Mnemosyne: pressure={} consolidation_stale={} recommended_action={}",
            mnemosyne.get("continuity_pressure").and_then(|value| value.as_str()).unwrap_or("unknown"),
            mnemosyne.get("consolidation_stale").and_then(|value| value.as_bool()).unwrap_or(false),
            mnemosyne.get("recommended_action").and_then(|value| value.as_str()).unwrap_or("unknown"),
        ),
        format!(
            "Warden: fleet_status={} active_peers={}/{} attention_required={} raw={} repeated_repair_noise={} next={}",
            warden.get("fleet_status").and_then(|value| value.as_str()).unwrap_or("unknown"),
            warden.get("active_peers").and_then(|value| value.as_u64()).unwrap_or(0),
            warden.get("peers_total").and_then(|value| value.as_u64()).unwrap_or(0),
            warden.get("attention_required_events").and_then(|value| value.as_u64()).unwrap_or(0),
            warden.get("raw_attention_required_events").and_then(|value| value.as_u64()).unwrap_or(0),
            warden.get("repeated_repair_noise").and_then(|value| value.as_u64()).unwrap_or(0),
            join_json_string_array(warden.get("next_actions"), 3, "none"),
        ),
        format!(
            "Athena: sources_recent={} policy_ready_recent={} reference_only_recent={} task_receipts={}",
            athena.get("sources_recent").and_then(|value| value.as_u64()).unwrap_or(0),
            athena.get("policy_ready_recent").and_then(|value| value.as_u64()).unwrap_or(0),
            athena.get("reference_only_recent").and_then(|value| value.as_u64()).unwrap_or(0),
            athena.get("task_emission_receipts_total").and_then(|value| value.as_u64()).unwrap_or(0),
        ),
        format!(
            "Runtime admission/Bacon-lite: latest_shed_at={} steady_state={} latest_bacon_lite_passed={}",
            admission.get("latest_shed_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
            admission.get("steady_state").and_then(|value| value.as_bool()).unwrap_or(false),
            admission.get("latest_bacon_lite_passed").and_then(|value| value.as_bool()).unwrap_or(false),
        ),
        format!(
            "Next bounded action: {}",
            briefing
                .get("recommended_next_bounded_action")
                .and_then(|value| value.as_str())
                .unwrap_or("continue periodic read-only operations monitoring")
        ),
        format!(
            "Mutation policy: {}; mutated={}",
            source_files
                .get("mutation_policy")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            source_files
                .get("mutated")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        ),
        format!(
            "Sources: autopilot_state={}, charon_route_history={}",
            source_files
                .get("autopilot_state")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            source_files
                .get("charon_route_history")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
        ),
    ]);

    lines.join("\n")
}

fn build_provider_routing_posture(root: &std::path::Path) -> serde_json::Value {
    let router = read_json_or_default(&root.join("core/state/charon_router.json"), json!({}));
    let pressure = router
        .get("provider_pressure")
        .unwrap_or(&serde_json::Value::Null);
    let providers = pressure
        .get("providers")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let local_fallback = pressure
        .get("local_fallback")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let providers_total = providers.len() as u64;
    let enabled_total = providers
        .iter()
        .filter(|provider| {
            provider
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .count() as u64;
    let enabled_healthy_total = providers
        .iter()
        .filter(|provider| {
            provider
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
                && provider
                    .get("healthy")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
        })
        .count() as u64;
    let degraded_enabled = providers
        .iter()
        .filter(|provider| {
            provider
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
                && !provider
                    .get("healthy")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
        })
        .filter_map(|provider| provider.get("id").and_then(|value| value.as_str()))
        .collect::<Vec<_>>();
    let blocked_enabled = providers
        .iter()
        .filter(|provider| {
            provider
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
                && provider
                    .get("operational_blocked")
                    .or_else(|| provider.get("blocked"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
        })
        .filter_map(|provider| provider.get("id").and_then(|value| value.as_str()))
        .collect::<Vec<_>>();
    let available_enabled_total = providers
        .iter()
        .filter(|provider| {
            provider
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
                && provider
                    .get("healthy")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
                && !provider
                    .get("operational_blocked")
                    .or_else(|| provider.get("blocked"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
        })
        .count() as u64;
    let cooldowns = pressure
        .get("cooldowns")
        .and_then(|value| value.as_array())
        .map(|items| items.len() as u64)
        .unwrap_or(0);
    let active_provider = router
        .get("recent_events")
        .and_then(|value| value.as_array())
        .and_then(|events| {
            events.iter().rev().find_map(|event| {
                event
                    .get("payload")
                    .and_then(|payload| payload.get("provider_id"))
                    .and_then(|value| value.as_str())
            })
        })
        .unwrap_or("unknown");
    let status = if available_enabled_total == 0 {
        "unavailable"
    } else if !degraded_enabled.is_empty() {
        "degraded"
    } else if cooldowns > 0 {
        "healthy_with_cooldowns"
    } else {
        "healthy"
    };

    json!({
        "status": status,
        "source_of_truth": "core/state/charon_router.json",
        "source_preference": "prefer_charon_router_projection_over_static_config",
        "generated_at_utc": router.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "routing_defaults": router.get("routing_defaults").cloned().unwrap_or_else(|| json!({})),
        "active_provider": active_provider,
        "providers_total": providers_total,
        "enabled_total": enabled_total,
        "enabled_healthy_total": enabled_healthy_total,
        "available_enabled_total": available_enabled_total,
        "degraded_enabled": degraded_enabled,
        "blocked_enabled": blocked_enabled,
        "cooldowns": cooldowns,
        "local_fallback": {
            "id": local_fallback.get("id").and_then(|value| value.as_str()).unwrap_or("local_fallback"),
            "healthy": local_fallback.get("healthy").and_then(|value| value.as_bool()).unwrap_or(false),
            "base_url": local_fallback.get("base_url").and_then(|value| value.as_str()).unwrap_or("unknown")
        }
    })
}

fn summarize_chronos_runtime(runtime: &serde_json::Value) -> serde_json::Value {
    let summary = runtime
        .get("feed_summary")
        .unwrap_or(&serde_json::Value::Null);
    let audit_runner = runtime
        .get("audit_runner")
        .unwrap_or(&serde_json::Value::Null);
    json!({
        "source": "core/state/chronos_runtime.json",
        "generated_at_utc": runtime.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "status": runtime.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "feed_count": summary.get("feed_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "present_count": summary.get("present_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "stale_count": summary.get("stale_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "invalid_json_count": summary.get("invalid_json_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "max_age_seconds": summary.get("max_age_seconds").and_then(|value| value.as_u64()).unwrap_or(0),
        "audit_ready_task_count": audit_runner.get("ready_task_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "audit_receipt_count": audit_runner.get("receipt_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "configured_audit_classes": audit_runner.get("configured_audit_classes").cloned().unwrap_or_else(|| json!([])),
        "next_integration_steps": runtime.get("next_integration_steps").cloned().unwrap_or_else(|| json!([])),
    })
}

fn summarize_plutus_runtime(runtime: &serde_json::Value) -> serde_json::Value {
    let body = runtime.get("runtime").unwrap_or(runtime);
    let economics = body.get("economics").unwrap_or(&serde_json::Value::Null);
    let governance = body.get("governance").unwrap_or(&serde_json::Value::Null);
    let joulework = body.get("joulework").unwrap_or(&serde_json::Value::Null);
    json!({
        "source": "core/state/plutus_runtime.json",
        "generated_at_utc": runtime.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "daily_budget": economics.get("daily_budget").and_then(|value| value.as_f64()).unwrap_or(0.0),
        "budget_remaining": economics.get("budget_remaining").and_then(|value| value.as_f64()).unwrap_or(0.0),
        "budget_usage_percent": economics.get("budget_usage_percent").and_then(|value| value.as_f64()).unwrap_or(0.0),
        "total_spend": economics.get("total_spend").and_then(|value| value.as_f64()).unwrap_or(0.0),
        "governance_records_total": governance.get("records_total").and_then(|value| value.as_u64()).unwrap_or(0),
        "triad_passed_total": governance.get("triad_passed_total").and_then(|value| value.as_u64()).unwrap_or(0),
        "bacon_lite_passed_total": governance.get("bacon_lite_passed_total").and_then(|value| value.as_u64()).unwrap_or(0),
        "joulework_total": joulework.get("total_joulework").or_else(|| joulework.get("total")).and_then(|value| value.as_f64()).unwrap_or(0.0),
    })
}

fn summarize_mnemosyne_continuity(runtime: &serde_json::Value) -> serde_json::Value {
    let continuity = runtime
        .get("continuity")
        .unwrap_or(&serde_json::Value::Null);
    let health = runtime.get("health").unwrap_or(&serde_json::Value::Null);
    let counts = runtime
        .get("recent_activity")
        .and_then(|value| value.get("counts"))
        .unwrap_or(&serde_json::Value::Null);
    json!({
        "source": "core/state/mnemosyne_continuity.json",
        "generated_at_utc": runtime.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "chain_head_present": continuity.get("chain_head_present").and_then(|value| value.as_bool()).unwrap_or(false),
        "consolidation_age_hours": continuity.get("consolidation_age_hours").and_then(|value| value.as_u64()).unwrap_or(0),
        "consolidation_stale": continuity.get("consolidation_stale").and_then(|value| value.as_bool()).unwrap_or(false),
        "continuity_pressure": health.get("continuity_pressure").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "recommended_action": health.get("recommended_action").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "recent_memory_count": counts.get("recent_memory_count").and_then(|value| value.as_u64()).unwrap_or(0),
        "obsidian_entries": counts.get("obsidian_entries").and_then(|value| value.as_u64()).unwrap_or(0),
        "noise_events": counts.get("noise_events").and_then(|value| value.as_u64()).unwrap_or(0),
    })
}

fn summarize_warden_guardhouse(runtime: &serde_json::Value) -> serde_json::Value {
    let fleet = runtime
        .get("health")
        .and_then(|value| value.get("fleet_control"))
        .unwrap_or(&serde_json::Value::Null);
    let summary = fleet.get("summary").unwrap_or(fleet);
    let network = summary.get("network").unwrap_or(summary);
    let queue = runtime.get("queue").unwrap_or(&serde_json::Value::Null);
    let status_counts = queue
        .get("effective_status_counts")
        .or_else(|| queue.get("status_counts"))
        .unwrap_or(&serde_json::Value::Null);
    let repair_pressure = queue
        .get("repair_pressure")
        .unwrap_or(&serde_json::Value::Null);
    let fleet_cleanup = runtime
        .get("health")
        .and_then(|value| value.get("fleet_cleanup"))
        .unwrap_or(&serde_json::Value::Null);
    json!({
        "source": "core/state/warden_guardhouse.json",
        "generated_at_utc": runtime.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "duties": runtime.get("duties").cloned().unwrap_or_else(|| json!([])),
        "edge_role": runtime.get("edge_role").cloned().unwrap_or_else(|| json!({})),
        "fleet_status": summary.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "active_peers": network.get("active_peers").and_then(|value| value.as_u64()).unwrap_or(0),
        "peers_total": network.get("peers_total").or_else(|| network.get("peers_discovered")).or_else(|| network.get("peers_discovered_total")).and_then(|value| value.as_u64()).unwrap_or(0),
        "attention_required_events": status_counts.get("attention_required").and_then(|value| value.as_u64()).unwrap_or(0),
        "raw_attention_required_events": queue.get("status_counts").and_then(|counts| counts.get("attention_required")).and_then(|value| value.as_u64()).unwrap_or(0),
        "repeated_repair_noise": repair_pressure.get("repeated_repair_noise").and_then(|value| value.as_u64()).unwrap_or(0),
        "unique_repair_files": repair_pressure.get("unique_repair_files").and_then(|value| value.as_u64()).unwrap_or(0),
        "fleet_cleanup_status": fleet_cleanup.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "fleet_stale_candidates_total": fleet_cleanup.get("stale_candidates_total").and_then(|value| value.as_u64()).unwrap_or(0),
        "fleet_cleanup_safe_review_candidates_total": fleet_cleanup.get("safe_review_candidates_total").and_then(|value| value.as_u64()).unwrap_or(0),
        "fleet_cleanup_safe_action": fleet_cleanup.get("safe_action").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "source_counts": queue.get("source_counts").cloned().unwrap_or_else(|| json!({})),
        "next_actions": summary.get("next_actions").or_else(|| fleet.get("next_actions")).cloned().unwrap_or_else(|| json!([])),
    })
}

fn summarize_athena_runtime(runtime: &serde_json::Value) -> serde_json::Value {
    let counts = runtime
        .get("knowledge")
        .and_then(|value| value.get("counts"))
        .unwrap_or(&serde_json::Value::Null);
    let task_emission = runtime
        .get("task_emission")
        .unwrap_or(&serde_json::Value::Null);
    json!({
        "source": "core/state/athena_runtime.json",
        "generated_at_utc": runtime.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "sources_recent": counts.get("sources_recent").and_then(|value| value.as_u64()).unwrap_or(0),
        "digest_recent": counts.get("digest_recent").and_then(|value| value.as_u64()).unwrap_or(0),
        "deep_graph_recent": counts.get("deep_graph_recent").and_then(|value| value.as_u64()).unwrap_or(0),
        "deep_queue_recent": counts.get("deep_queue_recent").and_then(|value| value.as_u64()).unwrap_or(0),
        "policy_ready_recent": counts.get("policy_ready_recent").and_then(|value| value.as_u64()).unwrap_or(0),
        "reference_only_recent": counts.get("reference_only_recent").and_then(|value| value.as_u64()).unwrap_or(0),
        "task_emission_receipts_total": task_emission.get("receipts_total").and_then(|value| value.as_u64()).unwrap_or(0),
    })
}

fn summarize_runtime_admission(
    receipts: &serde_json::Value,
    recovery: &serde_json::Value,
    plutus_runtime: &serde_json::Value,
) -> serde_json::Value {
    let latest_receipt = receipts
        .get("recent_receipts")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items
                .iter()
                .rev()
                .find(|item| item.get("event").and_then(|value| value.as_str()) == Some("shed"))
        });
    let latest_governance = plutus_runtime
        .get("runtime")
        .and_then(|value| value.get("governance"))
        .and_then(|value| value.get("recent_records"))
        .and_then(|value| value.as_array())
        .and_then(|items| items.first());
    json!({
        "source_receipts": "core/state/runtime_admission_receipts.json",
        "source_recovery": "core/state/runtime_admission_recovery.json",
        "generated_at_utc": receipts.get("generated_at_utc").and_then(|value| value.as_str()).unwrap_or("unknown"),
        "labels_total": receipts.get("counts_by_label").and_then(|value| value.as_object()).map(|object| object.len() as u64).unwrap_or(0),
        "pressure_counts": receipts.get("counts_by_pressure_status").cloned().unwrap_or_else(|| json!({})),
        "latest_shed_at_utc": latest_receipt.and_then(|value| value.get("ts_utc")).and_then(|value| value.as_str()).unwrap_or("unknown"),
        "recovery_actions_total": recovery.get("summary").and_then(|value| value.get("recovery_actions_total")).and_then(|value| value.as_u64()).unwrap_or(0),
        "steady_state": recovery.get("summary").and_then(|value| value.get("steady_state")).and_then(|value| value.as_bool()).unwrap_or(false),
        "latest_bacon_lite_passed": latest_governance.and_then(|value| value.get("bacon_lite")).and_then(|value| value.get("passed")).and_then(|value| value.as_bool()).unwrap_or(false),
        "latest_bacon_lite_confidence": latest_governance.and_then(|value| value.get("bacon_lite")).and_then(|value| value.get("confidence")).and_then(|value| value.as_f64()).unwrap_or(0.0),
    })
}

fn join_json_string_array(
    value: Option<&serde_json::Value>,
    limit: usize,
    fallback: &str,
) -> String {
    let joined = value
        .and_then(|items| items.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .take(limit)
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();
    if joined.is_empty() {
        fallback.to_string()
    } else {
        joined
    }
}

fn recommended_operations_action(
    pending_tasks: u64,
    oldest_pending_secs: f64,
    failed_services: u64,
    degraded_services: u64,
) -> &'static str {
    if failed_services > 0 {
        "restore failed services before expanding automation"
    } else if pending_tasks > 0 || oldest_pending_secs >= 86_400.0 {
        "triage queued/aging tasks before expanding automation"
    } else if degraded_services > 0 {
        "review degraded services and timer posture"
    } else {
        "continue periodic read-only operations monitoring"
    }
}

pub(crate) async fn build_ops_dashboard(core_root: &str) -> anyhow::Result<serde_json::Value> {
    if std::env::var("ARDA_OPS_DASHBOARD_LIVE")
        .ok()
        .as_deref()
        != Some("1")
    {
        let queue_observability = queue_observability_snapshot();
        let ruleset = load_active_ruleset();
        let system_control = load_system_control_state();
        let package_health = read_json_or_default(
            std::path::Path::new("data/prometheus/package_health_last.json"),
            json!({}),
        );
        let storage_pressure = read_json_or_default(
            std::path::Path::new("data/prometheus/compaction_last.json"),
            json!({}),
        );
        let flywheel_packet_runtime = read_json_or_default(
            std::path::Path::new("core/state/flywheel_packet_runtime.json"),
            json!({}),
        );
        return Ok(json!({
            "generated_at_utc": Utc::now().to_rfc3339(),
            "dashboard_mode": "state_first_nonblocking",
            "live_service_collection": false,
            "live_service_collection_hint": "set ARDA_OPS_DASHBOARD_LIVE=1 for the legacy live service sweep",
            "queue_observability": queue_observability,
            "flywheel_packet_runtime": flywheel_packet_runtime,
            "runtime_surface": runtime_surface(),
            "active_ruleset": ruleset,
            "system_control": system_control,
            "package_observation": package_health,
            "storage_observation": storage_pressure,
        }));
    }
    let status_timeout = std::time::Duration::from_secs(1);
    let prometheus = PrometheusService::from_core(core_root)?;
    let charon = CharonService::from_default_or_fallback()?;
    let mnemosyne = MnemosyneService::from_default_or_fallback()?;
    let hades = HadesService::from_default_or_fallback()?;
    let hermes = HermesService::from_default_or_fallback()?;
    let apollo = ApolloService::from_default_or_workspace_fallback()?;
    let plutus = PlutusService::from_default_or_workspace_fallback()?;
    let oracle = OracleService::from_default_or_workspace_fallback()?;

    let prometheus_status_value = match prometheus.status() {
        Ok(status) => serde_json::to_value(&status)?,
        Err(err) => json!({
            "ok": false,
            "error": err.to_string(),
            "status_source": "prometheus_status_unavailable",
        }),
    };
    let athena_status_value = read_json_or_default(
        std::path::Path::new("core/state/athena_runtime.json"),
        json!({"ok": false, "status_source": "athena_runtime_unavailable"}),
    );
    let charon_status_value = match tokio::time::timeout(status_timeout, charon.status()).await {
        Ok(Ok(status)) => serde_json::to_value(&status)?,
        Ok(Err(err)) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "charon_status_unavailable"})
        }
        Err(_) => {
            json!({"ok": false, "error": "timed out", "status_source": "charon_status_timeout"})
        }
    };
    let charon_providers = tokio::time::timeout(status_timeout, charon.providers())
        .await
        .unwrap_or_default();
    let mnemosyne_status_value = match mnemosyne.status() {
        Ok(status) => serde_json::to_value(&status)?,
        Err(err) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "mnemosyne_status_unavailable"})
        }
    };
    let hades_status_value = match hades.status() {
        Ok(status) => serde_json::to_value(&status)?,
        Err(err) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "hades_status_unavailable"})
        }
    };
    let hermes_status_value = match tokio::time::timeout(status_timeout, hermes.status()).await {
        Ok(Ok(status)) => serde_json::to_value(&status)?,
        Ok(Err(err)) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "hermes_status_unavailable"})
        }
        Err(_) => {
            json!({"ok": false, "error": "timed out", "status_source": "hermes_status_timeout"})
        }
    };
    let apollo_status_value = match tokio::time::timeout(status_timeout, apollo.status()).await {
        Ok(Ok(status)) => serde_json::to_value(&status)?,
        Ok(Err(err)) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "apollo_status_unavailable"})
        }
        Err(_) => {
            json!({"ok": false, "error": "timed out", "status_source": "apollo_status_timeout"})
        }
    };
    let plutus_status_value = match tokio::time::timeout(status_timeout, plutus.status()).await {
        Ok(Ok(status)) => serde_json::to_value(&status)?,
        Ok(Err(err)) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "plutus_status_unavailable"})
        }
        Err(_) => {
            json!({"ok": false, "error": "timed out", "status_source": "plutus_status_timeout"})
        }
    };
    let oracle_status_value = match tokio::time::timeout(status_timeout, oracle.status()).await {
        Ok(Ok(status)) => serde_json::to_value(&status)?,
        Ok(Err(err)) => {
            json!({"ok": false, "error": err.to_string(), "status_source": "oracle_status_unavailable"})
        }
        Err(_) => {
            json!({"ok": false, "error": "timed out", "status_source": "oracle_status_timeout"})
        }
    };
    let home = home_root();
    let var_disk_pct = disk_usage_percent(home.to_string_lossy().as_ref());
    let tmp_disk_pct = disk_usage_percent("/tmp");
    let ruleset = load_active_ruleset();
    let system_control = load_system_control_state();
    let package_health = read_json_or_default(
        std::path::Path::new("data/prometheus/package_health_last.json"),
        json!({}),
    );
    let storage_pressure = read_json_or_default(
        std::path::Path::new("data/prometheus/compaction_last.json"),
        json!({}),
    );
    let flywheel_packet_runtime = read_json_or_default(
        std::path::Path::new("core/state/flywheel_packet_runtime.json"),
        json!({}),
    );
    let governance_observation = build_governance_observation(
        &prometheus_status_value,
        &hermes_status_value,
        &charon_status_value,
        &hades_status_value,
        &athena_status_value,
        &plutus_status_value,
        &mnemosyne_status_value,
        var_disk_pct,
        &ruleset,
        &system_control,
    );
    let queue_observability = queue_observability_snapshot();

    let provider_budgets = charon_providers
        .iter()
        .map(|p| {
            let day_remaining = p
                .requests_per_day
                .map(|max| max.saturating_sub(p.requests_used_day));
            let minute_remaining = p
                .requests_per_minute
                .map(|max| max.saturating_sub(p.requests_used_minute));
            json!({
                "provider_id": p.id,
                "healthy": p.healthy,
                "in_cooldown": p.in_cooldown,
                "day_remaining": day_remaining,
                "minute_remaining": minute_remaining,
                "consecutive_failures": p.consecutive_failures,
                "error_count": p.error_count,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "generated_at_utc": Utc::now().to_rfc3339(),
        "prometheus": prometheus_status_value,
        "athena": athena_status_value,
        "charon": {
            "status": charon_status_value,
            "provider_failure_budgets": provider_budgets,
        },
        "mnemosyne": mnemosyne_status_value,
        "hades": hades_status_value,
        "hermes": hermes_status_value,
        "apollo": apollo_status_value,
        "plutus": plutus_status_value,
        "oracle": oracle_status_value,
        "risk": {
            "hades_queue_depth": hades.queue(10_000)?.len(),
            "charon_degraded": charon_providers.iter().filter(|p| p.consecutive_failures >= 2 || p.error_count >= 5).count(),
            "charon_in_cooldown": charon_providers.iter().filter(|p| p.in_cooldown).count(),
            "hermes_outbound_queue_depth": hermes_status_value.get("queue_depth").and_then(|v| v.as_u64()).unwrap_or(0),
            "disk_var_used_pct": var_disk_pct,
            "disk_tmp_used_pct": tmp_disk_pct,
        },
        "governance_observation": governance_observation,
        "queue_observability": queue_observability,
        "flywheel_packet_runtime": flywheel_packet_runtime,
        "runtime_surface": runtime_surface(),
        "active_ruleset": ruleset,
        "system_control": system_control,
        "package_observation": package_health,
        "storage_observation": storage_pressure,
    }))
}

pub(crate) fn queue_observability_snapshot() -> serde_json::Value {
    let active_daily = count_jsonl_status("core/queue/queue.jsonl", "status", "queued");
    let active_projects =
        count_jsonl_latest_status("core/projects/tasks/queue.jsonl", "id", "status", "queued");
    let backlog_queued =
        count_jsonl_status("core/projects/backlog_post_v01.jsonl", "status", "queued");
    let backlog_open = count_jsonl_status_any(
        "core/projects/backlog_post_v01.jsonl",
        "status",
        &["queued", "backlog", "pending", "in_progress"],
    );
    let backlog_total = count_jsonl_lines("core/projects/backlog_post_v01.jsonl");
    let hades_action_queue = hades_action_queue_observability(
        "data/hades/action_queue.jsonl",
        "data/hades/action_queue_closeouts.jsonl",
    );
    let hades_pending = hades_action_queue.pending_records;
    let athena_pending = count_athena_pending_latest("data/athena/deep_queue.jsonl");
    let hermes_outbound = count_jsonl_latest_status(
        "data/hermes/outbound_queue.jsonl",
        "message_id",
        "status",
        "queued",
    ) + count_jsonl_latest_status(
        "data/hermes/outbound_queue.jsonl",
        "message_id",
        "status",
        "pending",
    );

    let total_active_internal =
        hades_pending + athena_pending + hermes_outbound + active_daily + active_projects;
    let total_known_work_items = total_active_internal + backlog_open;

    json!({
        "generated_at_utc": Utc::now().to_rfc3339(),
        "summary": {
            "total_active_internal_tasks": total_active_internal,
            "total_known_work_items": total_known_work_items,
            "project_task_queue_queued": active_projects,
            "legacy_daily_queue_queued": active_daily,
            "projects_queue_queued": active_projects,
            "backlog_open": backlog_open,
            "backlog_queued": backlog_queued,
            "backlog_total_records": backlog_total
        },
        "breakdown": {
            "project_task_queue": {
                "path": "core/projects/tasks/queue.jsonl",
                "queued": active_projects,
                "canonical": true
            },
            "legacy_daily_queue": {
                "path": "core/queue/queue.jsonl",
                "queued": active_daily,
                "canonical": false,
                "compatibility_lane": true
            },
            "projects_queue": {
                "path": "core/projects/tasks/queue.jsonl",
                "queued": active_projects,
                "canonical": true,
                "alias_of": "project_task_queue"
            },
            "projects_backlog": {
                "path": "core/projects/backlog_post_v01.jsonl",
                "queued": backlog_queued,
                "open": backlog_open,
                "total_records": backlog_total
            },
            "hades_action_queue": {
                "path": "data/hades/action_queue.jsonl",
                "pending_records": hades_action_queue.pending_records,
                "historical_records": hades_action_queue.historical_records,
                "closed_records": hades_action_queue.closed_records,
                "closeout_contract": hades_action_queue.closeout_contract,
                "closeout_ledger": {
                    "path": hades_action_queue.closeout_ledger_path,
                    "records_total": hades_action_queue.closeout_records_total,
                    "source_queue_mutated": false,
                    "source_queue_mutation_policy": "append_only_closeout_ledger_no_source_queue_rewrite"
                }
            },
            "athena_deep_queue": {
                "path": "data/athena/deep_queue.jsonl",
                "pending_deep": athena_pending
            },
            "hermes_outbound_queue": {
                "path": "data/hermes/outbound_queue.jsonl",
                "pending_records": hermes_outbound
            }
        }
    })
}

pub(crate) fn persist_queue_observability(snapshot: &serde_json::Value) {
    let root = std::path::Path::new("core/metrics/by_crate/prometheus");
    if let Err(err) = fs::create_dir_all(root) {
        tracing::warn!(error = %err, "failed to create prometheus metrics directory");
        return;
    }
    let latest_path = root.join("queue_observability.json");
    if let Err(err) = fs::write(
        &latest_path,
        match serde_json::to_string_pretty(snapshot) {
            Ok(s) => s + "\n",
            Err(err) => {
                tracing::warn!(error = %err, "failed to serialize queue observability");
                return;
            }
        },
    ) {
        tracing::warn!(error = %err, "failed to write queue observability latest file");
    }
}

#[derive(Debug, Clone)]
struct HadesActionQueueObservability {
    historical_records: usize,
    pending_records: usize,
    closed_records: usize,
    closeout_records_total: usize,
    closeout_ledger_path: String,
    closeout_contract: String,
}

fn hades_action_queue_observability(
    source_queue_path: &str,
    closeout_ledger_path: &str,
) -> HadesActionQueueObservability {
    let source_records = read_jsonl_values(source_queue_path);
    let source_task_ids = source_records
        .iter()
        .filter_map(|record| record.get("task_id").and_then(|value| value.as_str()))
        .map(ToOwned::to_owned)
        .collect::<std::collections::HashSet<String>>();
    let closeout_records = read_jsonl_values(closeout_ledger_path);
    let closed_task_ids = closeout_records
        .iter()
        .filter(|record| {
            record.get("contract").and_then(|value| value.as_str())
                == Some("arda.hades.action_queue_closeout.v1")
                && record.get("source_queue").and_then(|value| value.as_str())
                    == Some(source_queue_path)
                && record
                    .get("source_queue_mutated")
                    .and_then(|value| value.as_bool())
                    == Some(false)
        })
        .filter_map(|record| record.get("task_id").and_then(|value| value.as_str()))
        .filter(|task_id| source_task_ids.contains(*task_id))
        .map(ToOwned::to_owned)
        .collect::<std::collections::HashSet<String>>();

    let historical_records = source_records.len();
    let closed_records = closed_task_ids.len();
    HadesActionQueueObservability {
        historical_records,
        pending_records: historical_records.saturating_sub(closed_records),
        closed_records,
        closeout_records_total: closeout_records.len(),
        closeout_ledger_path: closeout_ledger_path.to_string(),
        closeout_contract: "arda.hades.action_queue_closeout.v1".to_string(),
    }
}

fn read_jsonl_values(path: &str) -> Vec<serde_json::Value> {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            serde_json::from_str::<serde_json::Value>(line).ok()
        })
        .collect()
}

fn count_jsonl_lines(path: &str) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn count_jsonl_status(path: &str, key: &str, expected: &str) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            serde_json::from_str::<serde_json::Value>(line).ok()
        })
        .filter(|v| v.get(key).and_then(|x| x.as_str()) == Some(expected))
        .count()
}

fn count_jsonl_latest_status(path: &str, id_key: &str, status_key: &str, expected: &str) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let mut latest = std::collections::HashMap::<String, String>::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(id) = v.get(id_key).and_then(|x| x.as_str()) else {
            continue;
        };
        let Some(status) = v.get(status_key).and_then(|x| x.as_str()) else {
            continue;
        };
        latest.insert(id.to_string(), status.to_string());
    }
    latest
        .values()
        .filter(|status| status.as_str() == expected)
        .count()
}

fn count_jsonl_status_any(path: &str, key: &str, expected: &[&str]) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            serde_json::from_str::<serde_json::Value>(line).ok()
        })
        .filter(|v| {
            let value = v.get(key).and_then(|x| x.as_str());
            expected.iter().any(|s| Some(*s) == value)
        })
        .count()
}

fn count_athena_pending_latest(path: &str) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let mut latest = std::collections::HashMap::<String, String>::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(source_id) = v.get("source_id").and_then(|x| x.as_str()) else {
            continue;
        };
        let status = v
            .get("status")
            .and_then(|x| x.as_str())
            .unwrap_or("pending_deep")
            .to_string();
        latest.insert(source_id.to_string(), status);
    }
    latest
        .values()
        .filter(|status| status.as_str() == "pending_deep")
        .count()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_governance_observation(
    prometheus_status: &serde_json::Value,
    hermes_status: &serde_json::Value,
    charon_status: &serde_json::Value,
    hades_status: &serde_json::Value,
    athena_status: &serde_json::Value,
    plutus_status: &serde_json::Value,
    mnemosyne_status: &serde_json::Value,
    disk_var_used_pct: Option<u8>,
    ruleset: &serde_json::Value,
    system_control: &serde_json::Value,
) -> serde_json::Value {
    let hermes_triad = hermes_status
        .get("messages_today")
        .and_then(|v| v.get("triad_pass_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let hermes_inbound = hermes_status
        .get("messages_today")
        .and_then(|v| v.get("inbound"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let hermes_joule = hermes_status
        .get("messages_today")
        .and_then(|v| v.get("avg_joulework"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let hermes_love = hermes_status
        .get("messages_today")
        .and_then(|v| v.get("avg_love_eq"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let plutus_joulework = plutus_status.get("joulework");
    let plutus_joule = plutus_joulework
        .and_then(|v| v.get("total"))
        .and_then(|v| v.as_f64())
        .map(|v| (v / 10.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let plutus_joulework_measurement = plutus_joulework
        .and_then(|v| v.get("measurement_metadata"))
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "observed_total": 0.0,
                "default_fallback_total": 0.0,
                "average_confidence": 0.0,
                "autonomy_truth_warning": false,
                "default_fallback_autonomy_truth": false,
                "source": "missing_plutus_measurement_metadata"
            })
        });
    let plutus_love = plutus_status
        .get("love_equation")
        .and_then(|v| {
            v.get("top_relationships")
                .or_else(|| v.get("relationships"))
        })
        .and_then(|v| v.as_array())
        .and_then(|values| {
            if values.is_empty() {
                None
            } else {
                let sum = values
                    .iter()
                    .filter_map(|value| {
                        value
                            .get("score")
                            .or_else(|| value.get("value"))
                            .and_then(|v| v.as_f64())
                    })
                    .sum::<f64>();
                Some((sum / values.len() as f64).clamp(0.0, 1.0))
            }
        })
        .unwrap_or(0.0);
    let retinue_score = prometheus_status
        .get("retinue_game_theory_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    let provider_total = charon_status
        .get("providers_ready")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            charon_status
                .get("providers_enabled")
                .and_then(|v| v.as_u64())
        })
        .or_else(|| {
            charon_status
                .get("providers_total")
                .and_then(|v| v.as_u64())
        })
        .unwrap_or(0) as f64;
    let provider_healthy = charon_status
        .get("providers_healthy")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as f64;
    let provider_health = if provider_total > 0.0 {
        (provider_healthy / provider_total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let hades_pending = hades_status
        .get("pending_actions")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as f64;
    let athena_deep_queue = athena_status
        .get("deep_queue_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as f64;
    let queue_pressure =
        ((hades_pending / 2000.0).min(1.0) * 0.7) + ((athena_deep_queue / 500.0).min(1.0) * 0.3);
    let queue_health = (1.0 - queue_pressure).clamp(0.0, 1.0);
    let memory_ok = mnemosyne_status
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let hades_joule_efficiency = read_latest_hades_joule_efficiency().unwrap_or(0.0);
    let (bacon_triad_rate, bacon_confidence, bacon_samples) = read_bacon_lite_recent(400);
    let ambient = read_recent_mnemosyne_signal_averages(400);
    let triad_pass_rate = if bacon_samples > 0 && hermes_inbound == 0 {
        bacon_triad_rate.clamp(0.0, 1.0)
    } else if bacon_samples > 0 {
        let blended = ((hermes_triad * 0.6) + (bacon_triad_rate * 0.4)).clamp(0.0, 1.0);
        blended.max((bacon_triad_rate * 0.7).clamp(0.0, 1.0))
    } else {
        hermes_triad
    };
    let direct_joule_samples = [
        (hermes_joule, hermes_joule > 0.0, 0.40),
        (plutus_joule, plutus_joule > 0.0, 0.25),
        (ambient.avg_joulework, ambient.samples > 0, 0.20),
        (hades_joule_efficiency, hades_joule_efficiency > 0.0, 0.15),
    ];
    let mut joule_numer = 0.0;
    let mut joule_denom = 0.0;
    for (value, present, weight) in direct_joule_samples {
        if !present {
            continue;
        }
        joule_numer += value * weight;
        joule_denom += weight;
    }
    let joule_signal = if joule_denom > 0.0 {
        (joule_numer / joule_denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let love_signal = if hermes_love > 0.0 {
        ((hermes_love * 0.60) + (plutus_love * 0.15) + (ambient.avg_love_eq * 0.25)).clamp(0.0, 1.0)
    } else if ambient.samples > 0 && plutus_love <= 0.0 {
        ambient.avg_love_eq.clamp(0.0, 1.0)
    } else if plutus_love > 0.0 || ambient.samples > 0 {
        ((plutus_love * 0.35) + (ambient.avg_love_eq * 0.65)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let mut coverage_checks = 0u32;
    let mut coverage_hits = 0u32;
    let checks = [
        !prometheus_status.is_null(),
        !hermes_status.is_null(),
        !charon_status.is_null(),
        !hades_status.is_null(),
        !athena_status.is_null(),
        memory_ok,
    ];
    for check in checks {
        coverage_checks += 1;
        if check {
            coverage_hits += 1;
        }
    }
    let coverage = if coverage_checks > 0 {
        coverage_hits as f64 / coverage_checks as f64
    } else {
        0.0
    };

    let retinue_norm = (retinue_score / 100.0).clamp(0.0, 1.0);
    let disk_health = disk_var_used_pct
        .map(|pct| (1.0 - (pct as f64 / 100.0)).clamp(0.0, 1.0))
        .unwrap_or(0.5);
    let autonomy_threshold = ruleset
        .get("policy")
        .and_then(|v| v.get("autonomy_score_threshold"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.65)
        .clamp(0.4, 0.95);
    let triad_required_pass_rate = ruleset
        .get("policy")
        .and_then(|v| v.get("triad_required_pass_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.45)
        .clamp(0.2, 0.95);
    let strict_gate = ruleset
        .get("policy")
        .and_then(|v| v.get("gate_strict"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let triad_block_on_fail = system_control
        .get("governance_gates")
        .and_then(|v| v.get("classes"))
        .and_then(|v| v.get("dispatch"))
        .and_then(|v| v.get("block_on_triad_fail"))
        .or_else(|| {
            system_control
                .get("governance_gates")
                .and_then(|v| v.get("defaults"))
                .and_then(|v| v.get("block_on_triad_fail"))
        })
        .or_else(|| {
            ruleset
                .get("policy")
                .and_then(|v| v.get("block_on_triad_fail"))
        })
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let triad_fail_policy_label = if triad_block_on_fail {
        "triad_fail_blocks_dispatch_by_policy"
    } else {
        "triad_fail_recorded_non_blocking_permissive_default"
    };
    let thresholds = system_control
        .get("governance")
        .and_then(|v| v.get("thresholds"))
        .cloned()
        .unwrap_or_else(|| default_signal_thresholds("arda_totality"));
    let weights = system_control
        .get("governance")
        .and_then(|v| v.get("weights"))
        .cloned()
        .unwrap_or_else(|| default_governance_weights("arda_totality"));
    let weight = |key: &str, default: f64| {
        weights
            .get(key)
            .and_then(|v| v.as_f64())
            .unwrap_or(default)
            .max(0.0)
    };
    let weight_sum = [
        weight("triad_pass_rate", 0.22),
        weight("joulework", 0.22),
        weight("love_equation", 0.16),
        weight("bacon_lite_confidence", 0.10),
        weight("retinue_game_theory", 0.14),
        weight("provider_health", 0.08),
        weight("queue_health", 0.04),
        weight("observation_coverage", 0.02),
        weight("disk_health", 0.02),
    ]
    .iter()
    .sum::<f64>()
    .max(1.0);
    let bacon_signal = bacon_confidence.clamp(0.0, 1.0);
    let autonomy_observation_score = ((triad_pass_rate * weight("triad_pass_rate", 0.22))
        + (joule_signal * weight("joulework", 0.22))
        + (love_signal * weight("love_equation", 0.16))
        + (bacon_signal * weight("bacon_lite_confidence", 0.10))
        + (retinue_norm * weight("retinue_game_theory", 0.14))
        + (provider_health * weight("provider_health", 0.08))
        + (queue_health * weight("queue_health", 0.04))
        + (coverage * weight("observation_coverage", 0.02))
        + (disk_health * weight("disk_health", 0.02)))
        / weight_sum;

    let mut attention = Vec::new();
    let joule_min = thresholds
        .get("joulework_min")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.45)
        .clamp(0.0, 1.0);
    let love_min = thresholds
        .get("love_equation_min")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.45)
        .clamp(0.0, 1.0);
    let provider_min = thresholds
        .get("provider_health_min")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.40)
        .clamp(0.0, 1.0);
    let queue_min = thresholds
        .get("queue_health_min")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.40)
        .clamp(0.0, 1.0);
    let coverage_min = thresholds
        .get("observation_coverage_min")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.80)
        .clamp(0.0, 1.0);
    if triad_pass_rate < triad_required_pass_rate {
        attention.push("triad_pass_rate_low".to_string());
    }
    if joule_signal < joule_min {
        attention.push("joule_signal_low".to_string());
    }
    if love_signal < love_min {
        attention.push("love_alignment_low".to_string());
    }
    if provider_health < provider_min {
        attention.push("provider_health_low".to_string());
    }
    if queue_health < queue_min {
        attention.push("queue_pressure_high".to_string());
    }
    if coverage < coverage_min {
        attention.push("observation_coverage_low".to_string());
    }
    if system_control
        .get("governance")
        .and_then(|v| v.get("always_on"))
        .and_then(|v| v.get("joulework_required"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
        && joule_signal <= 0.0
    {
        attention.push("joulework_missing".to_string());
    }
    if system_control
        .get("governance")
        .and_then(|v| v.get("always_on"))
        .and_then(|v| v.get("love_equation_influence"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
        && love_signal <= 0.0
    {
        attention.push("love_equation_missing".to_string());
    }
    if strict_gate && provider_health < 0.55 {
        attention.push("strict_gate_provider_health_block".to_string());
    }
    if strict_gate && queue_health < 0.55 {
        attention.push("strict_gate_queue_health_block".to_string());
    }

    let triad_philosopher = prometheus_status.get("triad_philosopher").cloned();
    let triad_philosopher_evidence = prometheus_status
        .get("triad_philosopher_evidence")
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .filter(|entry| entry.starts_with("triad_philosopher:"))
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut observation = json!({
        "generated_at_utc": Utc::now().to_rfc3339(),
        "source": "prometheus+hermes+charon+hades+athena+mnemosyne+bacon_lite",
        "signals": {
            "triad_pass_rate": triad_pass_rate,
            "avg_joulework": joule_signal,
            "avg_love_eq": love_signal,
            "retinue_game_theory_score": retinue_score,
            "provider_health": provider_health,
            "queue_health": queue_health,
            "observation_coverage": coverage,
            "disk_health": disk_health,
            "autonomy_observation_score": autonomy_observation_score,
            "bacon_lite_recent_triad_rate": bacon_triad_rate,
            "bacon_lite_recent_confidence": bacon_confidence,
            "bacon_lite_recent_samples": bacon_samples,
            "hades_joule_efficiency": hades_joule_efficiency
        },
        "governance_metadata": {
            "joulework": {
                "semantic": "source_aware_joulework_signal",
                "signal_key": "avg_joulework",
                "measurement_metadata": plutus_joulework_measurement
            },
            "love_equation": {
                "semantic": "task_value_proxy",
                "source": "impact_reach_energy_time_proxy_not_canonical_love_dynamics",
                "signal_key": "avg_love_eq"
            },
            "game_theory": {
                "selection_policy_kind": "HistoricalWeightedHeuristic",
                "selection_policy_label": "historical_weighted_heuristic_not_autonomous_consensus",
                "fallback_policy_label": "fallback_not_autonomous_consensus",
                "autonomous_consensus": false,
                "signal_key": "retinue_game_theory_score"
            },
            "triad": {
                "block_on_triad_fail": triad_block_on_fail,
                "fail_policy_label": triad_fail_policy_label,
                "shipped_default": "permissive_record_and_proceed"
            }
        },
        "control": {
            "weights": weights,
            "thresholds": thresholds,
            "always_on": system_control.get("governance").and_then(|v| v.get("always_on")).cloned().unwrap_or_else(|| json!({})),
            "validators": system_control.get("governance").and_then(|v| v.get("validators")).cloned().unwrap_or_else(|| json!({})),
            "human_augmentation": system_control.get("governance").and_then(|v| v.get("human_augmentation")).cloned().unwrap_or_else(|| json!({})),
            "signal_sources": {
                "hermes_avg_joulework": hermes_joule,
                "hermes_avg_love_eq": hermes_love,
                "plutus_joulework": plutus_joule,
                "plutus_love_eq": plutus_love,
                "mnemosyne_avg_joulework": ambient.avg_joulework,
                "mnemosyne_avg_love_eq": ambient.avg_love_eq,
                "mnemosyne_samples": ambient.samples,
            }
        },
        "goal": {
            "ceo_observation_goal": "autonomy_and_full_system_observability",
            "autonomy_ready": autonomy_observation_score >= autonomy_threshold && coverage >= coverage_min,
            "autonomy_threshold": autonomy_threshold,
            "triad_required_pass_rate": triad_required_pass_rate,
            "strict_gate": strict_gate,
            "validator_mode": ruleset
                .get("policy")
                .and_then(|v| v.get("validators"))
                .and_then(|v| v.get("core"))
                .and_then(|v| v.get("philosopher_triad"))
                .and_then(|v| v.get("mode"))
                .cloned()
                .unwrap_or_else(|| serde_json::json!("consensus_2_of_3")),
            "attention_required": attention,
            "active_ruleset": ruleset.get("active_ruleset").cloned().unwrap_or(serde_json::json!("arda_totality"))
        }
    });

    if let Some(goal) = observation
        .get_mut("goal")
        .and_then(|value| value.as_object_mut())
    {
        if let Some(verdict) = triad_philosopher {
            goal.insert("triad_philosopher".to_string(), verdict);
        }
        if !triad_philosopher_evidence.is_empty() {
            goal.insert(
                "triad_philosopher_evidence".to_string(),
                serde_json::json!(triad_philosopher_evidence),
            );
        }
    }

    observation
}

pub(crate) fn persist_governance_observation(snapshot: &serde_json::Value) {
    let root = std::path::Path::new("core/metrics/by_crate/governance");
    if let Err(err) = fs::create_dir_all(root) {
        tracing::warn!(error = %err, "failed to create governance metrics directory");
        return;
    }
    let latest_path = root.join("signals.json");
    if let Err(err) = fs::write(
        &latest_path,
        match serde_json::to_string_pretty(snapshot) {
            Ok(s) => s + "\n",
            Err(err) => {
                tracing::warn!(error = %err, "failed to serialize governance observation");
                return;
            }
        },
    ) {
        tracing::warn!(error = %err, "failed to write governance signals latest file");
        return;
    }

    let history_path = root.join("signals_history.jsonl");
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(err) = writeln!(file, "{}", snapshot) {
                tracing::warn!(error = %err, "failed to append governance signals history");
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to open governance signals history");
        }
    }
}

fn read_bacon_lite_recent(limit: usize) -> (f64, f64, usize) {
    let path = std::path::Path::new("data/governance/bacon_lite.jsonl");
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return (0.0, 0.0, 0),
    };
    let mut total = 0usize;
    let mut triad_passed = 0usize;
    let mut confidence_sum = 0.0;
    for line in content.lines().rev().take(limit.max(1)) {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        total += 1;
        if value
            .get("triad_passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            triad_passed += 1;
        }
        confidence_sum += value
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
    }
    if total == 0 {
        return (0.0, 0.0, 0);
    }
    (
        triad_passed as f64 / total as f64,
        confidence_sum / total as f64,
        total,
    )
}

fn read_latest_hades_joule_efficiency() -> Option<f64> {
    let path = std::path::Path::new("data/hades/joulework.jsonl");
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let estimated = value.get("estimated_joules").and_then(|v| v.as_f64())?;
        let baseline = value.get("baseline_joules").and_then(|v| v.as_f64())?;
        if estimated <= 0.0 {
            return Some(1.0);
        }
        return Some((baseline / estimated).clamp(0.0, 1.0));
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct AmbientSignalAverages {
    avg_joulework: f64,
    avg_love_eq: f64,
    samples: usize,
}

fn read_recent_mnemosyne_signal_averages(limit: usize) -> AmbientSignalAverages {
    let root = std::path::Path::new("data/mnemosyne");
    let mut files = Vec::new();
    collect_jsonl_files(&root.join("episodic"), &mut files);
    collect_jsonl_files(&root.join("episodic_compact"), &mut files);
    files.sort_by(|a, b| {
        let a_key = fs::metadata(a)
            .and_then(|m| m.modified())
            .ok()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let b_key = fs::metadata(b)
            .and_then(|m| m.modified())
            .ok()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        b_key.cmp(&a_key)
    });

    let mut samples = 0usize;
    let mut joule_sum = 0.0f64;
    let mut love_sum = 0.0f64;
    for path in files.into_iter().take(limit.max(1)) {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        for line in content.lines().rev() {
            if samples >= limit {
                break;
            }
            if line.trim().is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let joule = value
                .get("joulework")
                .or_else(|| value.get("significance").and_then(|v| v.get("joulework")))
                .and_then(|v| v.as_f64());
            let love = value
                .get("love_eq")
                .or_else(|| value.get("significance").and_then(|v| v.get("love_eq")))
                .and_then(|v| v.as_f64());
            if joule.is_none() && love.is_none() {
                continue;
            }
            joule_sum += joule.unwrap_or(0.0).clamp(0.0, 1.0);
            love_sum += love.unwrap_or(0.0).clamp(0.0, 1.0);
            samples += 1;
        }
        if samples >= limit {
            break;
        }
    }
    if samples == 0 {
        return AmbientSignalAverages {
            avg_joulework: 0.0,
            avg_love_eq: 0.0,
            samples: 0,
        };
    }
    AmbientSignalAverages {
        avg_joulework: (joule_sum / samples as f64).clamp(0.0, 1.0),
        avg_love_eq: (love_sum / samples as f64).clamp(0.0, 1.0),
        samples,
    }
}

fn collect_jsonl_files(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, out);
            continue;
        }
        if path.extension().and_then(|v| v.to_str()) == Some("jsonl") {
            out.push(path);
        }
    }
}

pub(crate) fn disk_usage_percent(path: &str) -> Option<u8> {
    let output = Command::new("df").arg("-P").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let line = stdout.lines().nth(1)?;
    let used = line.split_whitespace().nth(4)?;
    used.trim_end_matches('%').parse::<u8>().ok()
}

pub(crate) fn read_json_or_default(
    path: &std::path::Path,
    default: serde_json::Value,
) -> serde_json::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> anyhow::Result<std::path::PathBuf> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(std::env::temp_dir().join(format!("arda-cli-{name}-{stamp}")))
    }

    #[test]
    fn operations_briefing_surfaces_compact_periodic_summary_contract() {
        let briefing = build_operations_briefing_from_inputs(
            &serde_json::json!({
                "timestamp": "2026-05-30T01:40:20Z",
                "queue": {
                    "pending": 8,
                    "aging_oldest_pending_secs": 196319,
                    "recent_completions_24h": 30,
                    "recent_failures_24h": 0,
                    "completion_rate_24h": 1.0
                },
                "services": {
                    "healthy": 25,
                    "degraded": 1,
                    "failed": 0,
                    "overall_score": 0.9288,
                    "services": [
                        {"unit": "arda-ceo-autopilot-supervised.service", "active": "activating", "sub": "start", "note": "activating (start)", "score": 0.6},
                        {"unit": "arda-charon.service", "active": "active", "sub": "running", "note": "running", "score": 1.0}
                    ]
                },
                "dashboard": {
                    "alerts": [{"severity": "Warning", "message": "oldest pending task is over 24h old", "source": "queue"}]
                }
            }),
            &serde_json::json!({
                "summary": {"total_active_internal_tasks": 14},
                "breakdown": {"hades_action_queue": {"pending_records": 14}}
            }),
            &serde_json::json!({"status": "healthy", "active_provider": "local"}),
        );

        assert_eq!(briefing["contract"], "arda.operations_briefing.v1");
        assert_eq!(briefing["latest_cycle_summary"]["pending_tasks"], 8);
        assert_eq!(
            briefing["alerts"][0]["message"],
            "oldest pending task is over 24h old"
        );
        assert_eq!(briefing["task_aging"]["oldest_pending_hours"], 54.53);
        assert_eq!(briefing["service_degradation"]["degraded_count"], 1);
        assert_eq!(
            briefing["service_degradation"]["degraded_services"][0]["unit"],
            "arda-ceo-autopilot-supervised.service"
        );
        assert_eq!(briefing["provider_routing_posture"]["status"], "healthy");
        assert_eq!(
            briefing["recommended_next_bounded_action"].as_str(),
            Some("triage queued/aging tasks before expanding automation")
        );
        assert_eq!(briefing["source_files"]["mutated"], false);
    }

    #[test]
    fn operations_briefing_text_readout_is_operator_facing_and_non_mutating() {
        let briefing = build_operations_briefing_from_inputs(
            &serde_json::json!({
                "timestamp": "2026-05-30T01:40:20Z",
                "queue": {
                    "pending": 8,
                    "aging_oldest_pending_secs": 196319,
                    "recent_completions_24h": 30,
                    "recent_failures_24h": 0,
                    "completion_rate_24h": 1.0
                },
                "services": {
                    "healthy": 25,
                    "degraded": 1,
                    "failed": 0,
                    "overall_score": 0.9288,
                    "services": [
                        {"unit": "arda-ceo-autopilot-supervised.service", "active": "activating", "sub": "start", "note": "activating (start)", "score": 0.6}
                    ]
                },
                "dashboard": {
                    "alerts": [{"severity": "Warning", "message": "oldest pending task is over 24h old", "source": "queue"}]
                }
            }),
            &serde_json::json!({
                "summary": {"total_active_internal_tasks": 14},
                "breakdown": {"hades_action_queue": {"pending_records": 0}}
            }),
            &serde_json::json!({"status": "healthy", "active_provider": "local"}),
        );

        let readout = format_operations_briefing_text(&briefing);

        assert!(readout.contains("Arda Operations Briefing"));
        assert!(readout.contains("Latest cycle: 2026-05-30T01:40:20Z"));
        assert!(readout.contains("Queue: 8 pending, oldest 54.53h"));
        assert!(readout.contains("Alerts: 1"));
        assert!(readout.contains("Warning [queue]: oldest pending task is over 24h old"));
        assert!(readout.contains("Services: 25 healthy, 1 degraded, 0 failed"));
        assert!(readout
            .contains("- arda-ceo-autopilot-supervised.service: activating/start score=0.6"));
        assert!(readout.contains("Provider/routing: healthy (active_provider=local)"));
        assert!(readout.contains(
            "Next bounded action: triage queued/aging tasks before expanding automation"
        ));
        assert!(readout.contains(
            "Mutation policy: read_only_briefing_no_queue_or_receipt_rewrite; mutated=false"
        ));
    }

    #[test]
    fn hades_action_queue_folds_append_only_closeout_ledger() -> anyhow::Result<()> {
        let temp_root = temp_root("hades-action-queue-fold")?;
        let source_queue_path = temp_root.join("data/hades/action_queue.jsonl");
        let closeout_ledger_path = temp_root.join(
            "audit/OPERATIONS_QUEUE_REVIEW_2026-05-30/hades_action_queue_closeout_ledger.jsonl",
        );
        if let Some(parent) = source_queue_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = closeout_ledger_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let source_queue = source_queue_path.to_string_lossy().to_string();
        let closeout_ledger = closeout_ledger_path.to_string_lossy().to_string();
        fs::write(
            &source_queue_path,
            "{\"task_id\":\"hds_closed\",\"status\":\"queued\"}\n{\"task_id\":\"hds_open\",\"status\":\"queued\"}\n",
        )?;
        fs::write(
            &closeout_ledger_path,
            format!(
                "{{\"contract\":\"arda.hades.action_queue_closeout.v1\",\"source_queue\":{source_queue_json},\"task_id\":\"hds_closed\",\"source_queue_mutated\":false,\"status\":\"completed_remediated\"}}\n{{\"contract\":\"arda.hades.action_queue_closeout.v1\",\"source_queue\":{source_queue_json},\"task_id\":\"hds_unrelated\",\"source_queue_mutated\":false,\"status\":\"completed_remediated\"}}\n",
                source_queue_json = serde_json::to_string(&source_queue)?,
            ),
        )?;

        let observation = hades_action_queue_observability(&source_queue, &closeout_ledger);

        assert_eq!(observation.historical_records, 2);
        assert_eq!(observation.closed_records, 1);
        assert_eq!(observation.pending_records, 1);
        assert_eq!(observation.closeout_records_total, 2);
        assert_eq!(
            observation.closeout_contract,
            "arda.hades.action_queue_closeout.v1"
        );

        let _ = fs::remove_dir_all(temp_root);
        Ok(())
    }

    #[test]
    fn governance_observation_surfaces_compact_triad_philosopher_evidence() {
        let observation = build_governance_observation(
            &serde_json::json!({
                "retinue_game_theory_score": 77.0,
                "triad_philosopher": {"action": "hold", "alignment_score": 0.42},
                "triad_philosopher_evidence": [
                    "triad_philosopher:hold:0.42",
                    "non_triad:evidence"
                ]
            }),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
            &serde_json::json!({}),
            &serde_json::json!({}),
        );

        assert_eq!(
            observation
                .get("goal")
                .and_then(|goal| goal.get("triad_philosopher"))
                .and_then(|verdict| verdict.get("action"))
                .and_then(|action| action.as_str()),
            Some("hold")
        );
        assert_eq!(
            observation
                .get("goal")
                .and_then(|goal| goal.get("triad_philosopher_evidence")),
            Some(&serde_json::json!(["triad_philosopher:hold:0.42"]))
        );
    }

    #[test]
    fn governance_observation_surfaces_operator_metadata_without_overclaiming() {
        let observation = build_governance_observation(
            &serde_json::json!({"retinue_game_theory_score": 77.0}),
            &serde_json::json!({"messages_today": {"avg_love_eq": 0.51}}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
            &serde_json::json!({}),
            &serde_json::json!({
                "governance_gates": {
                    "classes": {
                        "dispatch": {"block_on_triad_fail": true}
                    }
                }
            }),
        );

        let metadata = observation
            .get("governance_metadata")
            .expect("governance metadata should be operator-facing");

        assert_eq!(
            metadata
                .get("love_equation")
                .and_then(|value| value.get("semantic"))
                .and_then(|value| value.as_str()),
            Some("task_value_proxy")
        );
        assert_eq!(
            metadata
                .get("love_equation")
                .and_then(|value| value.get("source"))
                .and_then(|value| value.as_str()),
            Some("impact_reach_energy_time_proxy_not_canonical_love_dynamics")
        );
        assert_eq!(
            metadata
                .get("game_theory")
                .and_then(|value| value.get("selection_policy_label"))
                .and_then(|value| value.as_str()),
            Some("historical_weighted_heuristic_not_autonomous_consensus")
        );
        assert_eq!(
            metadata
                .get("game_theory")
                .and_then(|value| value.get("autonomous_consensus"))
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            metadata
                .get("triad")
                .and_then(|value| value.get("block_on_triad_fail"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            metadata
                .get("triad")
                .and_then(|value| value.get("fail_policy_label"))
                .and_then(|value| value.as_str()),
            Some("triad_fail_blocks_dispatch_by_policy")
        );
    }
}
