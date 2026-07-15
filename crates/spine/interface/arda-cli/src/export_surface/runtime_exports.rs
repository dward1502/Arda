use anyhow::Result;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::process::Command;

use super::*;

pub(crate) fn export_runtime_admission_receipts_impl() -> Result<Value> {
    let root = workspace_root();
    let in_path = root.join("data/prometheus/runtime_admission_shed_receipts.jsonl");
    let out_path = root.join("core/state/runtime_admission_receipts.json");
    let rows = read_jsonl_objects_local(&in_path);
    let mut label_counts = BTreeMap::new();
    let mut mode_counts = BTreeMap::new();
    let mut pressure_status_counts = BTreeMap::new();
    let mut latest = None::<String>;
    let mut local_joule_pressure_events = 0usize;

    for row in &rows {
        if let Some(label) = row.get("label").and_then(Value::as_str) {
            *label_counts.entry(label.to_string()).or_insert(0usize) += 1;
        }
        if let Some(mode) = row.get("mode").and_then(Value::as_str) {
            *mode_counts.entry(mode.to_string()).or_insert(0usize) += 1;
        }
        if let Some(pressure) = row.get("pressure").and_then(Value::as_object) {
            if pressure
                .get("local_joule_pressure")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                local_joule_pressure_events += 1;
            }
            if let Some(status) = pressure.get("pressure_status").and_then(Value::as_str) {
                if !status.is_empty() {
                    *pressure_status_counts
                        .entry(status.to_string())
                        .or_insert(0usize) += 1;
                }
            }
        }
        if let Some(ts) = row.get("ts_utc").and_then(Value::as_str) {
            if latest.as_deref().is_none_or(|current| ts > current) {
                latest = Some(ts.to_string());
            }
        }
    }

    let recent_limit = rows.len().saturating_sub(50);
    let payload = json!({
        "schema_version": "annunimas.runtime-admission-receipts.v1",
        "generated_at_utc": now_utc(),
        "authority": "runtime_admission_receipts",
        "summary": {
            "shed_events_total": rows.len(),
            "labels_total": label_counts.len(),
            "modes_total": mode_counts.len(),
            "local_joule_pressure_events_total": local_joule_pressure_events,
            "latest_shed_at_utc": latest,
        },
        "counts_by_label": label_counts,
        "counts_by_mode": mode_counts,
        "counts_by_pressure_status": pressure_status_counts,
        "recent_receipts": rows.into_iter().skip(recent_limit).collect::<Vec<_>>(),
        "source_surfaces": {
            "receipts_jsonl": rel(&in_path, &root),
            "runtime_budget_policy": "core/state/runtime_budget_policy.json",
            "runtime_admission_pressure": "core/state/runtime_admission_pressure.json",
        },
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_runtime_budget_policy_impl() -> Result<Value> {
    let root = workspace_root();
    let config_path = root.join("config/runtime_governor_budget.toml");
    let charon_router_path = root.join("core/state/charon_router.json");
    let charon_providers_path = root.join("config/charon.providers.toml");
    let plutus_status_path = root.join("data/plutus/runtime_status.json");
    let out_path = root.join("core/state/runtime_budget_policy.json");

    let cfg = read_toml_or(&config_path, toml::Value::Table(Default::default()));
    let charon = read_json_or(&charon_router_path, json!({}));
    let charon_provider_cfg = read_toml_or(
        &charon_providers_path,
        toml::Value::Table(Default::default()),
    );
    let plutus = read_json_or(&plutus_status_path, json!({}));

    let user_plan = toml_table(&cfg, "user_plan");
    let provider_cfg = toml_table(&cfg, "providers");
    let routing_load_shed = toml_table(&cfg, "routing_load_shed");
    let contract = toml_table(&cfg, "contract");
    let configured_provider_rows = charon_provider_cfg
        .get("provider")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let provider_rows = charon
        .get("provider_pressure")
        .and_then(|value| value.get("providers"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let pressure_by_id = provider_rows
        .into_iter()
        .filter_map(|row| {
            let id = row.get("id")?.as_str()?.to_string();
            Some((id, row))
        })
        .collect::<BTreeMap<_, _>>();
    let economics = plutus
        .get("economics")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let joulework = plutus
        .get("joulework")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut providers = Vec::new();
    let mut pressure_total = 0usize;
    let mut seen = BTreeSet::new();

    let mut append_provider = |provider_id: &str,
                               row: Option<&toml::Value>,
                               pressure: Option<&Value>| {
        let limits = provider_cfg
            .and_then(|table| table.get(provider_id))
            .and_then(toml::Value::as_table);
        let requests_used_day = pressure
            .and_then(|value| value.get("requests_used_day"))
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        let requests_per_day = pressure
            .and_then(|value| value.get("requests_per_day"))
            .and_then(Value::as_i64)
            .or_else(|| {
                row.and_then(|value| value.get("requests_per_day"))
                    .and_then(toml::Value::as_integer)
            });
        let monthly_soft = limits
            .and_then(|table| table.get("monthly_requests_soft_cap"))
            .and_then(toml::Value::as_float)
            .or_else(|| {
                limits
                    .and_then(|table| table.get("monthly_requests_soft_cap"))
                    .and_then(toml::Value::as_integer)
                    .map(|v| v as f64)
            });
        let monthly_hard = limits
            .and_then(|table| table.get("monthly_requests_hard_cap"))
            .and_then(toml::Value::as_float)
            .or_else(|| {
                limits
                    .and_then(|table| table.get("monthly_requests_hard_cap"))
                    .and_then(toml::Value::as_integer)
                    .map(|v| v as f64)
            });
        let inferred_monthly_used = requests_used_day * 30;
        let monthly_soft_pct = pct(inferred_monthly_used as f64, monthly_soft);
        let monthly_hard_pct = pct(inferred_monthly_used as f64, monthly_hard);
        let over_soft = monthly_soft_pct.is_some_and(|value| value >= 100.0);
        let over_hard = monthly_hard_pct.is_some_and(|value| value >= 100.0);
        if over_soft || over_hard {
            pressure_total += 1;
        }
        providers.push(json!({
            "provider_id": provider_id,
            "enabled": pressure.and_then(|value| value.get("enabled")).cloned().or_else(|| row.and_then(|value| value.get("enabled")).and_then(|v| v.as_bool()).map(Value::from)).unwrap_or(Value::Null),
            "healthy": pressure.and_then(|value| value.get("healthy")).cloned().or_else(|| row.and_then(|value| value.get("healthy")).and_then(|v| v.as_bool()).map(Value::from)).unwrap_or(Value::Null),
            "requests_used_day": requests_used_day,
            "requests_per_day": requests_per_day,
            "daily_request_usage_percent": requests_per_day.map(|value| value as f64).and_then(|cap| pct(requests_used_day as f64, Some(cap))),
            "monthly_requests_soft_cap": monthly_soft,
            "monthly_requests_hard_cap": monthly_hard,
            "monthly_requests_inferred_used": inferred_monthly_used,
            "monthly_soft_cap_usage_percent": monthly_soft_pct,
            "monthly_hard_cap_usage_percent": monthly_hard_pct,
            "in_cooldown": pressure.and_then(|value| value.get("in_cooldown")).cloned().unwrap_or(Value::Null),
            "budget_pressure": if over_hard { "hard_cap_exceeded" } else if over_soft { "soft_cap_exceeded" } else { "normal" },
        }));
    };

    for row in &configured_provider_rows {
        let Some(provider_id) = row.get("id").and_then(toml::Value::as_str) else {
            continue;
        };
        seen.insert(provider_id.to_string());
        append_provider(provider_id, Some(row), pressure_by_id.get(provider_id));
    }

    for (provider_id, pressure) in &pressure_by_id {
        if seen.contains(provider_id) {
            continue;
        }
        append_provider(provider_id, None, Some(pressure));
    }

    let total_spend = economics
        .get("total_spend")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let budget_remaining = economics
        .get("budget_remaining")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let budget_usage_percent = economics
        .get("budget_usage_percent")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let monthly_soft_spend = user_plan
        .and_then(|table| table.get("monthly_spend_usd_soft_cap"))
        .and_then(toml::Value::as_float)
        .or_else(|| {
            user_plan
                .and_then(|table| table.get("monthly_spend_usd_soft_cap"))
                .and_then(toml::Value::as_integer)
                .map(|v| v as f64)
        })
        .unwrap_or(0.0);
    let monthly_hard_spend = user_plan
        .and_then(|table| table.get("monthly_spend_usd_hard_cap"))
        .and_then(toml::Value::as_float)
        .or_else(|| {
            user_plan
                .and_then(|table| table.get("monthly_spend_usd_hard_cap"))
                .and_then(toml::Value::as_integer)
                .map(|v| v as f64)
        })
        .unwrap_or(0.0);
    let inferred_monthly_spend = total_spend * 30.0;
    let local_joule_total = joulework
        .get("total")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let local_joule_soft = user_plan
        .and_then(|table| table.get("local_joulework_daily_soft_cap"))
        .and_then(toml::Value::as_float)
        .or_else(|| {
            user_plan
                .and_then(|table| table.get("local_joulework_daily_soft_cap"))
                .and_then(toml::Value::as_integer)
                .map(|v| v as f64)
        })
        .unwrap_or(0.0);
    let period_start = parse_utc_datetime(joulework.get("period_start").and_then(Value::as_str));
    let period_end = parse_utc_datetime(joulework.get("period_end").and_then(Value::as_str));
    let period_span_seconds = period_start
        .zip(period_end)
        .map(|(start, end)| (end - start).num_milliseconds().max(0) as f64 / 1000.0)
        .unwrap_or(0.0);
    let local_joule_window_valid = !(local_joule_total > 100.0 && period_span_seconds < 300.0);
    let local_joule_effective = if local_joule_window_valid {
        local_joule_total
    } else {
        0.0
    };
    let local_joule_pct = pct(local_joule_effective, Some(local_joule_soft));
    let monthly_soft_pressure_pct = pct(inferred_monthly_spend, Some(monthly_soft_spend));
    let monthly_hard_pressure_pct = pct(inferred_monthly_spend, Some(monthly_hard_spend));

    let payload = json!({
        "schema_version": contract.and_then(|table| table.get("schema_version")).and_then(toml::Value::as_str).unwrap_or("annunimas.runtime-governor-budget.v1"),
        "generated_at_utc": now_utc(),
        "authority": contract.and_then(|table| table.get("authority")).and_then(toml::Value::as_str).unwrap_or("runtime_governor_budget_policy"),
        "doctrine": {
            "provider_budget_pressure_guides_routing": true,
            "user_plan_budget_pressure_guides_operator_actions": true,
            "monthly_limits_are_policy_inputs_even_when_only_daily_usage_is_observed": true,
            "local_and_edge_budget_pressure_must_be_visible_without_ui_coupling": true,
        },
        "user_plan_budget": {
            "daily_spend_usd_soft_cap": user_plan.and_then(|table| table.get("daily_spend_usd_soft_cap")).cloned().unwrap_or(toml::Value::String(String::new())),
            "daily_spend_usd_observed": total_spend,
            "daily_spend_usd_usage_percent": budget_usage_percent,
            "daily_spend_usd_remaining": budget_remaining,
            "monthly_spend_usd_soft_cap": monthly_soft_spend,
            "monthly_spend_usd_hard_cap": monthly_hard_spend,
            "monthly_spend_usd_inferred_used": ((inferred_monthly_spend * 10000.0).round() / 10000.0),
            "monthly_spend_soft_cap_usage_percent": monthly_soft_pressure_pct,
            "monthly_spend_hard_cap_usage_percent": monthly_hard_pressure_pct,
            "local_joulework_daily_soft_cap": local_joule_soft,
            "local_joulework_observed": local_joule_effective,
            "local_joulework_observed_cumulative": local_joule_total,
            "local_joulework_usage_percent": local_joule_pct,
            "local_joulework_window_valid": local_joule_window_valid,
            "local_joulework_window_span_seconds": ((period_span_seconds * 1000.0).round() / 1000.0),
            "edge_model_storage_gib_soft_cap": user_plan.and_then(|table| table.get("edge_model_storage_gib_soft_cap")).cloned().unwrap_or(toml::Value::String(String::new())),
        },
        "routing_load_shed": {
            "reasoning_minute_pressure_soft": routing_load_shed.and_then(|table| table.get("reasoning_minute_pressure_soft")).cloned().unwrap_or(toml::Value::String(String::new())),
            "reasoning_minute_pressure_hard": routing_load_shed.and_then(|table| table.get("reasoning_minute_pressure_hard")).cloned().unwrap_or(toml::Value::String(String::new())),
            "reasoning_day_pressure_soft": routing_load_shed.and_then(|table| table.get("reasoning_day_pressure_soft")).cloned().unwrap_or(toml::Value::String(String::new())),
            "reasoning_latency_ms_soft": routing_load_shed.and_then(|table| table.get("reasoning_latency_ms_soft")).cloned().unwrap_or(toml::Value::String(String::new())),
            "reasoning_latency_ms_hard": routing_load_shed.and_then(|table| table.get("reasoning_latency_ms_hard")).cloned().unwrap_or(toml::Value::String(String::new())),
            "prefer_gateway_when_local_joule_pressure": routing_load_shed.and_then(|table| table.get("prefer_gateway_when_local_joule_pressure")).and_then(toml::Value::as_bool).unwrap_or(true),
            "prefer_gateway_when_provider_budget_pressure": routing_load_shed.and_then(|table| table.get("prefer_gateway_when_provider_budget_pressure")).and_then(toml::Value::as_bool).unwrap_or(true),
        },
        "provider_budgets": providers,
        "summary": {
            "providers_tracked_total": providers.len(),
            "provider_budget_pressure_total": pressure_total,
            "daily_spend_pressure": budget_usage_percent >= 100.0,
            "monthly_spend_soft_pressure": monthly_soft_pressure_pct.is_some_and(|value| value >= 100.0),
            "monthly_spend_hard_pressure": monthly_hard_pressure_pct.is_some_and(|value| value >= 100.0),
            "local_joule_pressure": local_joule_window_valid && local_joule_pct.is_some_and(|value| value >= 80.0),
        },
        "source_surfaces": {
            "budget_config": "config/runtime_governor_budget.toml",
            "charon_router": "core/state/charon_router.json",
            "plutus_runtime": "data/plutus/runtime_status.json",
        },
        "source_validity": {
            "local_joulework_window_valid": local_joule_window_valid,
            "local_joulework_validity_reason": if local_joule_window_valid {
                "rolling_window_available"
            } else {
                "restored_plutus_aggregate_window_too_short_for_daily_budget"
            },
        },
    });

    write_pretty_json(&out_path, &payload)?;
    export_runtime_admission_pressure_snapshot(&root, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn export_runtime_admission_pressure_snapshot(
    root: &std::path::Path,
    budget: &Value,
) -> Result<()> {
    let out_path = root.join("core/state/runtime_admission_pressure.json");
    let audit = read_json_or(&root.join("core/metrics/audit_latest.json"), json!({}));
    let budget_summary = budget.get("summary").cloned().unwrap_or_else(|| json!({}));
    let user_plan_budget = budget
        .get("user_plan_budget")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let storage_pressure = audit
        .get("storage_pressure")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut violations = Vec::new();
    if budget_summary
        .get("monthly_spend_hard_pressure")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        violations.push(json!({
            "id": "monthly_spend_hard_pressure",
            "severity": "critical",
            "message": "inferred monthly spend exceeded the configured hard cap",
            "actual": user_plan_budget.get("monthly_spend_hard_cap_usage_percent").cloned().unwrap_or(Value::Null),
            "threshold": 100.0,
        }));
    }
    if budget_summary
        .get("monthly_spend_soft_pressure")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        violations.push(json!({
            "id": "monthly_spend_soft_pressure",
            "severity": "warning",
            "message": "inferred monthly spend exceeded the configured soft cap",
            "actual": user_plan_budget.get("monthly_spend_soft_cap_usage_percent").cloned().unwrap_or(Value::Null),
            "threshold": 100.0,
        }));
    }
    if budget_summary
        .get("local_joule_pressure")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        violations.push(json!({
            "id": "local_joule_pressure",
            "severity": "warning",
            "message": "local JouleWork usage is above the runtime admission soft pressure threshold",
            "actual": user_plan_budget.get("local_joulework_usage_percent").cloned().unwrap_or(Value::Null),
            "threshold": 80.0,
        }));
    }
    if storage_pressure
        .get("oversize_present")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        violations.push(json!({
            "id": "storage_oversize_files",
            "severity": "warning",
            "message": "audit storage scan found files at or above the oversize threshold",
            "actual": storage_pressure.get("oversize_files_gte_100mb").cloned().unwrap_or(Value::Null),
            "threshold": 0,
        }));
    }

    let status = if violations.iter().any(|violation| {
        violation
            .get("severity")
            .and_then(Value::as_str)
            .is_some_and(|severity| severity.eq_ignore_ascii_case("critical"))
    }) {
        "critical"
    } else if violations.is_empty() {
        "ok"
    } else {
        "attention_required"
    };

    let payload = json!({
        "schema_version": "annunimas.runtime-admission-pressure.v1",
        "generated_at_utc": now_utc(),
        "authority": "runtime_admission_pressure_projection",
        "status": status,
        "rationale": {
            "first_class_runtime_surface": true,
            "supersedes_legacy_pressure_guard_path": "data/prometheus/pressure_guard_last.json",
            "reason": "the legacy pressure guard producer lives only in archived scripts; Chronos and admission control now consume this live core/state projection generated by runtime budget export"
        },
        "observed": {
            "budget_summary": budget_summary,
            "user_plan_budget": user_plan_budget,
            "storage_pressure": storage_pressure,
        },
        "violations": violations,
        "source_surfaces": {
            "runtime_budget_policy": "core/state/runtime_budget_policy.json",
            "audit_latest": "core/metrics/audit_latest.json"
        },
    });
    write_pretty_json(&out_path, &payload)
}

pub(crate) fn export_runtime_admission_recovery_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/runtime_admission_recovery.json");
    let receipts = read_json_or(
        &root.join("core/state/runtime_admission_receipts.json"),
        json!({}),
    );
    let budget = read_json_or(
        &root.join("core/state/runtime_budget_policy.json"),
        json!({}),
    );
    let pressure = read_json_or(
        &root.join("core/state/runtime_admission_pressure.json"),
        json!({}),
    );

    let recent = receipts
        .get("recent_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut latest_by_label = BTreeMap::new();
    let mut counts = BTreeMap::new();
    let recovery_window_minutes = 60i64;
    let cutoff = Utc::now() - chrono::Duration::minutes(recovery_window_minutes);
    let mut stale_shed_total = 0usize;
    for row in recent {
        if row.get("event").and_then(Value::as_str) != Some("shed") {
            continue;
        }
        let fresh = parse_utc_datetime(row.get("ts_utc").and_then(Value::as_str))
            .is_some_and(|ts| ts >= cutoff);
        if !fresh {
            stale_shed_total += 1;
            continue;
        }
        let Some(label) = row
            .get("label")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        if label.is_empty() {
            continue;
        }
        latest_by_label.insert(label.clone(), row);
        *counts.entry(label).or_insert(0usize) += 1;
    }

    let pressure_status = pressure.get("status").cloned().unwrap_or(Value::Null);
    let budget_summary = budget.get("summary").cloned().unwrap_or_else(|| json!({}));
    let local_joule_pressure = budget_summary
        .get("local_joule_pressure")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut recovery_items = Vec::new();
    let mut kind_counts = BTreeMap::new();
    for (label, row) in latest_by_label {
        let policy = runtime_recovery_policy(&label);
        let kind = policy
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("observe");
        *kind_counts.entry(kind.to_string()).or_insert(0usize) += 1;
        recovery_items.push(json!({
            "recovery_id": format!("runtime_recovery_{label}"),
            "label": label,
            "owner": policy.get("owner").cloned().unwrap_or(Value::Null),
            "kind": kind,
            "title": policy.get("title").cloned().unwrap_or(Value::Null),
            "recommended_action": policy.get("recommended_action").cloned().unwrap_or(Value::Null),
            "retry_surface": policy.get("retry_surface").cloned().unwrap_or(Value::Null),
            "recent_shed_total": counts.get(&label).copied().unwrap_or(0),
            "latest_shed_at_utc": row.get("ts_utc").cloned().unwrap_or(Value::Null),
            "pressure_snapshot": row.get("pressure").cloned().unwrap_or_else(|| json!({})),
            "writes_through": policy.get("writes_through").cloned().unwrap_or_else(|| json!([])),
            "autonomous_candidate": matches!(kind, "route_shift" | "reroute_retry" | "deferred_retry"),
        }));
    }

    let payload = json!({
        "schema_version": "annunimas.runtime-admission-recovery.v1",
        "generated_at_utc": now_utc(),
        "authority": "runtime_admission_recovery_policy",
        "doctrine": {
            "shed_receipts_must_project_into_recovery_actions": true,
            "recovery_prefers_reroute_before_human_escalation": true,
            "maintenance_work_should_defer_under_pressure": true,
            "ui_optional": true,
            "agent_consumable": true,
        },
        "source_surfaces": {
            "runtime_admission_receipts": "core/state/runtime_admission_receipts.json",
            "runtime_budget_policy": "core/state/runtime_budget_policy.json",
            "runtime_admission_pressure": "core/state/runtime_admission_pressure.json",
        },
        "summary": {
            "recovery_actions_total": recovery_items.len(),
            "route_shift_total": kind_counts.get("route_shift").copied().unwrap_or(0),
            "reroute_retry_total": kind_counts.get("reroute_retry").copied().unwrap_or(0),
            "deferred_retry_total": kind_counts.get("deferred_retry").copied().unwrap_or(0),
            "control_plane_defer_total": kind_counts.get("control_plane_defer").copied().unwrap_or(0),
            "steady_state": recovery_items.is_empty(),
            "pressure_guard_status": pressure_status,
            "local_joule_pressure": local_joule_pressure,
            "active_recovery_window_minutes": recovery_window_minutes,
            "stale_shed_receipts_ignored_total": stale_shed_total,
        },
        "recovery_actions": recovery_items,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

fn runtime_recovery_policy(label: &str) -> Value {
    if label.starts_with("athena_") {
        if label.contains("http") {
            return json!({
                "kind": "route_shift",
                "owner": "charon",
                "title": "Shift burst ATHENA demand off the local lane",
                "recommended_action": "prefer_backbone_or_gateway",
                "writes_through": [
                    "config/model_route_matrix.toml",
                    "core/state/charon_router.json",
                    "core/state/runtime_budget_policy.json",
                ],
                "retry_surface": "cargo run --quiet -- athena deep-process --limit 25 --retry-failed",
            });
        }
        return json!({
            "kind": "deferred_retry",
            "owner": "athena",
            "title": "Retry deferred ATHENA deep work when pressure clears",
            "recommended_action": "retry_when_pressure_ok",
            "writes_through": [
                "core/state/runtime_admission_receipts.json",
                "core/state/runtime_budget_policy.json",
                "core/state/athena_digest_pipeline.json",
            ],
            "retry_surface": "cargo run --quiet -- athena deep-process --limit 25 --retry-failed",
        });
    }
    if label.starts_with("hermes_") {
        return json!({
            "kind": "reroute_retry",
            "owner": "hermes",
            "title": "Retry or reroute deferred Hermes outbound work",
            "recommended_action": "retry_outbound_then_reroute",
            "writes_through": [
                "core/state/runtime_admission_receipts.json",
                "core/state/charon_router.json",
                "core/state/model_control_surface.json",
            ],
            "retry_surface": "cargo run --quiet -- hermes retry-outbound --limit 100",
        });
    }
    if label.starts_with("hades_") {
        return json!({
            "kind": "deferred_retry",
            "owner": "hades",
            "title": "Defer HADES maintenance until pressure drops",
            "recommended_action": "retry_maintenance_when_pressure_ok",
            "writes_through": [
                "core/state/runtime_admission_receipts.json",
                "core/state/runtime_budget_policy.json",
                "data/hades/hades_log.jsonl",
            ],
            "retry_surface": "cargo run --quiet -- hades sweep --type manual",
        });
    }
    if label.starts_with("charon_") {
        return json!({
            "kind": "route_shift",
            "owner": "charon",
            "title": "Reduce local routing pressure and shift to backbone lanes",
            "recommended_action": "prefer_backbone_or_gateway",
            "writes_through": [
                "config/model_route_matrix.toml",
                "core/state/charon_router.json",
                "core/state/model_control_surface.json",
            ],
            "retry_surface": Value::Null,
        });
    }
    if label.starts_with("prometheus_") {
        return json!({
            "kind": "control_plane_defer",
            "owner": "prometheus",
            "title": "Hold low-priority control-plane work until pressure recovers",
            "recommended_action": "defer_noncritical_control_plane_work",
            "writes_through": [
                "core/state/runtime_budget_policy.json",
                "core/state/runtime_topology.json",
            ],
            "retry_surface": Value::Null,
        });
    }
    json!({
        "kind": "observe",
        "owner": "prometheus",
        "title": format!("Review shed activity for {label}"),
        "recommended_action": "inspect_runtime_receipts",
        "writes_through": ["core/state/runtime_admission_receipts.json"],
        "retry_surface": Value::Null,
    })
}

pub(crate) fn export_memory_governor_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/memory_governor.json");
    let continuity = read_json_or(
        &root.join("core/state/mnemosyne_continuity.json"),
        json!({}),
    );
    let contract = read_json_or(
        &root.join("core/state/agent_continuity_contract.json"),
        json!({}),
    );
    let stats_root = read_json_or(
        &root.join("core/metrics/by_crate/mnemosyne/stats.json"),
        json!({}),
    );
    let stats = continuity
        .get("stats")
        .and_then(|value| value.get("status").or(Some(value)))
        .cloned()
        .filter(|value| value.is_object())
        .unwrap_or(stats_root);
    let health = continuity
        .get("health")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let recent_counts = continuity
        .get("recent_activity")
        .and_then(|value| value.get("counts"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let continuity_meta = continuity
        .get("continuity")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let recent_memory_count = recent_counts
        .get("recent_memory_count")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let high_significance_memories = recent_counts
        .get("high_significance_memories")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let noise_events = recent_counts
        .get("noise_events")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let consolidation_age_hours = continuity_meta
        .get("consolidation_age_hours")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| {
            let last = continuity_meta
                .get("last_consolidation_utc")
                .and_then(Value::as_str)
                .or_else(|| stats.get("last_consolidation_utc").and_then(Value::as_str));
            parse_utc_datetime(last)
                .map(|dt| ((Utc::now() - dt).num_seconds().max(0)) / 3600)
                .unwrap_or(999)
        });
    let consolidation_stale = continuity_meta
        .get("consolidation_stale")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let continuity_drought = health
        .get("continuity_drought")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let noise_dominant = health
        .get("noise_dominant")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let continuity_pressure = health
        .get("continuity_pressure")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let mut actions = Vec::new();
    if consolidation_stale {
        actions.push("run_mnemosyne_consolidate");
    }
    if continuity_drought {
        actions.push("increase_checkpoint_capture");
    }
    if noise_dominant {
        actions.push("reduce_noise_and_raise_signal_threshold");
    }
    if recent_memory_count < 3 {
        actions.push("promote_retrieval_worthy_events");
    }

    let payload = json!({
        "schema_version": "annunimas.memory-governor.v1",
        "generated_at_utc": now_utc(),
        "authority": "mnemosyne_continuity + agent_continuity_contract",
        "summary": {
            "continuity_pressure": continuity_pressure,
            "recommended_actions_total": actions.len(),
            "continuity_capabilities_total": contract
                .get("summary")
                .and_then(|value| value.get("continuity_capabilities_total"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            "recent_memory_count": recent_memory_count,
            "high_significance_memories": high_significance_memories,
            "noise_events": noise_events,
        },
        "signals": {
            "continuity_drought": continuity_drought,
            "consolidation_stale": consolidation_stale,
            "noise_dominant": noise_dominant,
            "consolidation_age_hours": consolidation_age_hours,
            "recent_memory_count": recent_memory_count,
            "high_significance_memories": high_significance_memories,
            "noise_events": noise_events,
            "malformed_episodic_records": stats.get("malformed_episodic_records").and_then(Value::as_i64).unwrap_or(0),
        },
        "recommended_actions": actions,
        "doctrine": {
            "memory_must_drive_retrieval_not_prompt_bloat": true,
            "continuity_requires_fresh_checkpointing": true,
            "consolidation_should_not_go_stale_during_active_system_work": true,
        },
        "operator_message": health.get("recommended_action").and_then(Value::as_str).unwrap_or("memory posture healthy"),
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_metrics_delta_impl() -> Result<Value> {
    let root = workspace_root();
    let history_root = root.join("core/metrics/history");
    let manifest_path = root.join("core/metrics/manifest.json");
    let github_integration_path = root.join("core/state/github_repo_integration.json");
    let out_path = root.join("core/state/metrics_delta.json");

    let manifest = read_json_or(&manifest_path, json!({}));
    let current_snapshot_id = manifest
        .get("snapshot_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut history_dirs = if history_root.exists() {
        std::fs::read_dir(&history_root)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry.file_type().ok().filter(|ft| ft.is_dir())?;
                Some(entry.path())
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    history_dirs.sort();
    let baseline_snapshot_id = current_snapshot_id.as_ref().and_then(|current| {
        history_dirs
            .iter()
            .find(|path| path.file_name().and_then(|n| n.to_str()) != Some(current.as_str()))
            .or_else(|| history_dirs.first())
            .and_then(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
            })
    });

    if current_snapshot_id.is_none() || baseline_snapshot_id.is_none() {
        let payload = json!({
            "schema_version": "annunimas.metrics-delta.v1",
            "generated_at_utc": now_utc(),
            "authority": "metrics_delta_export",
            "status": "insufficient_history",
            "baseline_snapshot_id": baseline_snapshot_id,
            "current_snapshot_id": current_snapshot_id,
        });
        write_pretty_json(&out_path, &payload)?;
        return Ok(json!({ "out": rel(&out_path, &root) }));
    }

    let (Some(baseline_snapshot_id), Some(current_snapshot_id)) =
        (baseline_snapshot_id, current_snapshot_id)
    else {
        unreachable!("snapshot ids are checked before metrics delta export");
    };
    let load_snapshot = |snapshot_id: &str, relative_path: &str| {
        read_json_or(
            &history_root.join(snapshot_id).join(relative_path),
            json!({}),
        )
    };

    let athena_baseline = load_snapshot(&baseline_snapshot_id, "athena/status.json");
    let athena_current = load_snapshot(&current_snapshot_id, "athena/status.json");
    let hades_baseline = load_snapshot(&baseline_snapshot_id, "hades/status.json");
    let hades_current = load_snapshot(&current_snapshot_id, "hades/status.json");
    let hermes_baseline = load_snapshot(&baseline_snapshot_id, "hermes/status.json");
    let hermes_current = load_snapshot(&current_snapshot_id, "hermes/status.json");
    let package_baseline = load_snapshot(&baseline_snapshot_id, "prometheus/package_health.json");
    let package_current = load_snapshot(&current_snapshot_id, "prometheus/package_health.json");
    let github_integration = read_json_or(&github_integration_path, json!({}));

    let payload = json!({
        "schema_version": "annunimas.metrics-delta.v1",
        "generated_at_utc": now_utc(),
        "authority": "metrics_delta_export",
        "baseline_snapshot_id": baseline_snapshot_id,
        "current_snapshot_id": current_snapshot_id,
        "baseline_history_path": format!("core/metrics/history/{baseline_snapshot_id}"),
        "current_history_path": format!("core/metrics/history/{current_snapshot_id}"),
        "headline": {
            "athena_ingest_growth": metric_delta(&athena_baseline, &athena_current, &["ingest_success_total"]),
            "athena_digest_growth": metric_delta(&athena_baseline, &athena_current, &["digest_events"]),
            "athena_queue_relief": metric_delta(&athena_baseline, &athena_current, &["deep_queue_depth"]),
            "hades_pending_relief": metric_delta(&hades_baseline, &hades_current, &["pending_actions"]),
            "hermes_inbound_growth": metric_delta(&hermes_baseline, &hermes_current, &["messages_today", "inbound"]),
        },
        "systems": {
            "athena": {
                "metrics": [
                    metric_delta(&athena_baseline, &athena_current, &["books_count"]),
                    metric_delta(&athena_baseline, &athena_current, &["deduplicated_ingests_total"]),
                    metric_delta(&athena_baseline, &athena_current, &["deep_graph_events"]),
                    metric_delta(&athena_baseline, &athena_current, &["deep_queue_depth"]),
                    metric_delta(&athena_baseline, &athena_current, &["digest_events"]),
                    metric_delta(&athena_baseline, &athena_current, &["ingest_success_total"]),
                    metric_delta(&athena_baseline, &athena_current, &["policy_ready_count"]),
                    metric_delta(&athena_baseline, &athena_current, &["primary_policy_ready_count"]),
                    metric_delta(&athena_baseline, &athena_current, &["reference_only_count"]),
                ]
            },
            "hades": {
                "metrics": [
                    metric_delta(&hades_baseline, &hades_current, &["orphans_active"]),
                    metric_delta(&hades_baseline, &hades_current, &["pending_actions"]),
                ]
            },
            "hermes": {
                "metrics": [
                    metric_delta(&hermes_baseline, &hermes_current, &["messages_today", "inbound"]),
                    metric_delta(&hermes_baseline, &hermes_current, &["messages_today", "escalated_to_prometheus"]),
                    {
                        "metric": "providers_online",
                        "baseline": hermes_baseline.get("providers_online").cloned().unwrap_or(Value::Null),
                        "current": hermes_current.get("providers_online").cloned().unwrap_or(Value::Null),
                    },
                    {
                        "metric": "providers_offline",
                        "baseline": hermes_baseline.get("providers_offline").cloned().unwrap_or(Value::Null),
                        "current": hermes_current.get("providers_offline").cloned().unwrap_or(Value::Null),
                    },
                ]
            },
            "package_health": {
                "metrics": [
                    metric_delta(&package_baseline, &package_current, &["summary", "observed_with_version"]),
                    {
                        "metric": "critical_version_blind",
                        "baseline": package_baseline.get("critical_version_blind").cloned().unwrap_or(Value::Null),
                        "current": package_current.get("critical_version_blind").cloned().unwrap_or(Value::Null),
                    },
                ]
            },
        },
        "github_corpus": {
            "source": "core/state/github_repo_integration.json",
            "summary": github_integration.get("summary").cloned().unwrap_or_else(|| json!({})),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_edge_model_rollout_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/edge_model_rollout.json");
    let previous = read_json_or(&out_path, json!({}));
    let targets = [
        EdgeRolloutTarget {
            target_id: "node-ser9-worker",
            ssh_target: "citadel@100.103.125.88",
            log_path: "~/annunimas-model-logs/seed_bluefin.log",
            runtime_kind: "ramalama",
            models: &["Qwen3-8B-Q4_K_M.gguf", "Qwen3-8B-Q5_K_M.gguf"],
            store_root: Some("~/.local/share/ramalama/store/huggingface/unsloth/Qwen3-8B-GGUF"),
        },
        EdgeRolloutTarget {
            target_id: "node-backbone-server-01",
            ssh_target: "annunimasserver@100.118.123.88",
            log_path: "~/annunimas-model-logs/seed_backbone.log",
            runtime_kind: "ramalama",
            models: &[
                "Qwen3-8B-Q4_K_M.gguf",
                "Devstral-Small-2507-Q4_K_M.gguf",
                "Qwen3-30B-A3B-Q4_K_M.gguf",
            ],
            store_root: Some("~/.local/share/ramalama/store/huggingface/unsloth"),
        },
        EdgeRolloutTarget {
            target_id: "node-pi5-warden",
            ssh_target: "numenor@100.110.85.37",
            log_path: "~/annunimas-model-logs/seed_warden.log",
            runtime_kind: "ollama",
            models: &["qwen2.5-coder:3b"],
            store_root: None,
        },
    ];

    let targets = targets.iter().map(check_rollout_target).collect::<Vec<_>>();
    if targets
        .iter()
        .all(|target| target.get("probe_status").and_then(Value::as_str) == Some("probe_blocked"))
        && previous.get("targets").and_then(Value::as_array).is_some()
    {
        let mut payload = previous;
        payload["generated_at_utc"] = json!(now_utc());
        payload["authority"] = json!("edge_model_rollout_probe_cached_fallback");
        payload["probe_status"] = json!("cached_fallback");
        payload["last_probe_error"] = json!("ssh probe blocked by sandbox restrictions");
        write_pretty_json(&out_path, &payload)?;
        return Ok(json!({ "out": rel(&out_path, &root) }));
    }

    let payload = json!({
        "schema_version": "annunimas.edge-model-rollout.v1",
        "generated_at_utc": now_utc(),
        "authority": "edge_model_rollout_probe",
        "probe_status": "ok",
        "summary": {
            "targets_total": targets.len(),
            "targets_complete_total": targets.iter().filter(|t| t.get("models_completed") == t.get("models_total")).count(),
            "targets_active_total": targets.iter().filter(|t| t.get("active_pull").and_then(Value::as_bool) == Some(true)).count(),
            "targets_probe_blocked_total": targets.iter().filter(|t| t.get("probe_status").and_then(Value::as_str) == Some("probe_blocked")).count(),
        },
        "targets": targets,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_runtime_governor_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/runtime_governor_contract.json");
    let model_control = read_json_or(
        &root.join("core/state/model_control_surface.json"),
        json!({}),
    );
    let charon_router = read_json_or(&root.join("core/state/charon_router.json"), json!({}));
    let fleet_runtime = read_json_or(&root.join("core/state/fleet_runtime.json"), json!({}));
    let fleet_control = read_json_or(
        &root.join("core/metrics/by_crate/prometheus/fleet_control.json"),
        json!({}),
    );
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let operator_actions = read_json_or(&root.join("core/state/operator_actions.json"), json!({}));
    let manifest = read_json_or(&root.join("core/metrics/manifest.json"), json!({}));
    let runtime_budget_policy = read_json_or(
        &root.join("core/state/runtime_budget_policy.json"),
        json!({}),
    );
    let edge_model_rollout =
        read_json_or(&root.join("core/state/edge_model_rollout.json"), json!({}));
    let edge_endpoint_verification = read_json_or(
        &root.join("core/state/edge_endpoint_verification.json"),
        json!({}),
    );
    let opencode_route_governor = read_json_or(
        &root.join("core/state/opencode_route_governor.json"),
        json!({}),
    );

    let pressure_rows: HashMap<String, Value> = charon_router
        .get("provider_pressure")
        .and_then(|value| value.get("providers"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let id = row.get("id").and_then(Value::as_str)?.to_string();
            Some((id, row.clone()))
        })
        .collect();
    let providers = model_control
        .get("charon_providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.get("id").and_then(Value::as_str).is_some())
        .map(|row| {
            summarize_provider(
                row,
                pressure_rows.get(row.get("id").and_then(Value::as_str).unwrap_or("")),
            )
        })
        .collect::<Vec<_>>();
    let provider_limits = json!({
        "providers_with_daily_limits": providers.iter().filter(|p| p.get("requests_per_day").is_some()).count(),
        "providers_with_minute_limits": providers.iter().filter(|p| p.get("requests_per_minute").is_some()).count(),
        "cooldown_active_total": providers.iter().filter(|p| p.get("in_cooldown").and_then(Value::as_bool) == Some(true)).count(),
    });

    let observed_rows = fleet_control
        .get("fleet_nodes_full")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let configured_targets = fleet_runtime
        .get("inventory")
        .and_then(|value| value.get("configured_targets"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let fleet_nodes = configured_targets
        .iter()
        .filter(|row| row.is_object())
        .map(|row| summarize_node(row, &observed_rows))
        .collect::<Vec<_>>();

    let enabled_tools = package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.is_object())
        .map(|row| {
            json!({
                "tool": row.get("tool").cloned().unwrap_or(Value::Null),
                "activation_status": row.get("activation_status").cloned().unwrap_or(Value::Null),
                "integration_state": row.get("integration_state").cloned().unwrap_or(Value::Null),
                "next_action": row.get("next_action").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let rollout_targets = edge_model_rollout
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.is_object())
        .map(summarize_rollout_target)
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "annunimas.runtime-governor-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "agnostic_runtime_governor_projection",
        "purpose": {
            "agent_kind": "control_plane_subagent",
            "mission": "Monitor usage, uptime, downtime, provider budgets, and role posture across sovereign runtime surfaces.",
            "ui_optional": true,
            "consumer_agnostic": true,
        },
        "doctrine": {
            "finds_existing_connections_before_change": true,
            "reads_sovereign_state_before_ui": true,
            "can_recommend_role_switches_but_must_write_through_authority_surfaces": true,
            "provider_limits_and_user_plan_budgets_are_first_class_inputs": true,
            "arda_hud_not_required": true,
            "soterion_trace_required_for_meaningful_runtime_changes": true,
            "joulework_budget_review_required_before_expensive_shift": true,
        },
        "input_surfaces": {
            "model_control_surface": "core/state/model_control_surface.json",
            "opencode_route_governor": "core/state/opencode_route_governor.json",
            "charon_router": "core/state/charon_router.json",
            "runtime_budget_policy": "core/state/runtime_budget_policy.json",
            "fleet_runtime": "core/state/fleet_runtime.json",
            "fleet_control": "core/metrics/by_crate/prometheus/fleet_control.json",
            "package_enablement": "core/state/package_enablement.json",
            "operator_actions": "core/state/operator_actions.json",
            "edge_model_rollout": "core/state/edge_model_rollout.json",
            "edge_endpoint_verification": "core/state/edge_endpoint_verification.json",
            "metrics_manifest": "core/metrics/manifest.json",
            "soterion_joulework_enforcement": "core/state/soterion_joulework_enforcement.json",
        },
        "capability_lanes": {
            "provider_budget_tracking": {
                "owner": "charon",
                "surfaces": ["core/state/charon_router.json", "core/state/model_control_surface.json"],
                "summary": provider_limits,
                "providers": providers,
            },
            "fleet_uptime_and_downtime": {
                "owner": "warden",
                "surfaces": ["core/metrics/by_crate/prometheus/fleet_control.json", "core/state/fleet_runtime.json"],
                "fleet_status": fleet_control.get("status").cloned().unwrap_or(Value::Null),
                "network": fleet_control.get("network").cloned().unwrap_or(Value::Null),
                "nodes": fleet_nodes,
            },
            "runtime_role_posture": {
                "owner": "hermes_charon",
                "surfaces": ["core/state/fleet_runtime.json", "core/state/model_control_surface.json", "core/state/opencode_route_governor.json"],
                "placement": model_control.get("placement").cloned().unwrap_or(Value::Null),
                "runtime_defaults": model_control.get("defaults").cloned().unwrap_or(Value::Null),
                "opencode_route_governor": {
                    "applied_total": opencode_route_governor.get("summary").and_then(|v| v.get("applied_total")).cloned().unwrap_or(Value::Null),
                    "manual_override_total": opencode_route_governor.get("summary").and_then(|v| v.get("manual_override_total")).cloned().unwrap_or(Value::Null),
                    "changed": opencode_route_governor.get("summary").and_then(|v| v.get("changed")).cloned().unwrap_or(Value::Null),
                },
                "nodes": fleet_nodes.iter().map(|n| json!({
                    "target_id": n.get("target_id").cloned().unwrap_or(Value::Null),
                    "role": n.get("role").cloned().unwrap_or(Value::Null),
                    "node_class": n.get("node_class").cloned().unwrap_or(Value::Null),
                    "online": n.get("online").cloned().unwrap_or(Value::Null),
                })).collect::<Vec<_>>(),
            },
            "tool_activation_and_health": {
                "owner": "prometheus",
                "surfaces": ["core/state/package_enablement.json", "core/state/operator_actions.json"],
                "summary": package_enablement.get("summary").cloned().unwrap_or(Value::Null),
                "tools": enabled_tools,
                "human_needed_total": operator_actions.get("summary").and_then(|v| v.get("human_needed_total")).cloned().unwrap_or(Value::Null),
            },
            "user_and_provider_budget_pressure": {
                "owner": "plutus_charon",
                "surfaces": ["core/state/runtime_budget_policy.json", "core/state/charon_router.json", "data/plutus/runtime_status.json"],
                "summary": runtime_budget_policy.get("summary").cloned().unwrap_or(Value::Null),
                "user_plan_budget": runtime_budget_policy.get("user_plan_budget").cloned().unwrap_or(Value::Null),
                "provider_budgets": runtime_budget_policy.get("provider_budgets").cloned().unwrap_or(Value::Null),
            },
            "edge_model_rollout_and_readiness": {
                "owner": "charon",
                "surfaces": ["core/state/edge_model_rollout.json", "core/state/fleet_runtime.json"],
                "summary": edge_model_rollout.get("summary").cloned().unwrap_or(Value::Null),
                "targets": rollout_targets,
            },
            "edge_endpoint_verification": {
                "owner": "charon",
                "surfaces": ["core/state/edge_endpoint_verification.json", "config/charon.providers.toml"],
                "summary": edge_endpoint_verification.get("summary").cloned().unwrap_or(Value::Null),
                "targets": edge_endpoint_verification.get("targets").cloned().unwrap_or(Value::Null),
            },
        },
        "action_contracts": [
            {
                "action": "monitor_provider_budgets",
                "writes_through": ["core/state/charon_router.json"],
                "description": "Observe daily/minute request limits, used counters, cooldowns, and provider health."
            },
            {
                "action": "monitor_fleet_uptime",
                "writes_through": ["core/metrics/by_crate/prometheus/fleet_control.json"],
                "description": "Track online/offline posture and recent usage across configured nodes."
            },
            {
                "action": "propose_role_switches",
                "writes_through": ["config/fleet.toml", "core/edge/targets.toml", "core/state/model_control_surface.json"],
                "description": "Recommend changes in node duty or routing posture, but only through sovereign config and projection surfaces."
            },
            {
                "action": "surface_plan_budget_pressure",
                "writes_through": ["core/state/operator_actions.json", "core/state/runtime_governor_contract.json", "core/state/runtime_budget_policy.json"],
                "description": "Expose user-plan and provider-plan pressure to downstream agents and UIs without hard-coding a frontend."
            },
            {
                "action": "monitor_edge_model_rollout",
                "writes_through": ["core/state/edge_model_rollout.json", "core/state/runtime_governor_contract.json"],
                "description": "Track model artifact rollout on edge and backbone nodes so routing and local serving can activate from machine truth."
            },
            {
                "action": "verify_edge_endpoints",
                "writes_through": ["core/state/edge_endpoint_verification.json", "config/charon.providers.toml", "core/state/charon_router.json"],
                "description": "Probe live node-local inference endpoints and detect contract drift between configured provider URLs and observed serving ports."
            },
        ],
        "ui_ingestion_contract": {
            "display_ready": true,
            "editable_surfaces": [
                "config/fleet.toml",
                "core/edge/targets.toml",
                "config/charon.providers.toml",
                "config/model_route_matrix.toml",
            ],
            "read_only_state_surfaces": [
                "core/state/runtime_governor_contract.json",
                "core/state/runtime_budget_policy.json",
                "core/state/edge_model_rollout.json",
                "core/state/edge_endpoint_verification.json",
                "core/state/charon_router.json",
                "core/metrics/by_crate/prometheus/fleet_control.json",
                "core/state/package_enablement.json",
            ],
        },
        "latest_snapshot": {
            "metrics_snapshot_id": manifest.get("snapshot_id").cloned().unwrap_or(Value::Null),
            "metrics_generated_at_utc": manifest.get("generated_at_utc").cloned().unwrap_or(Value::Null),
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_search_runtime_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/search_runtime_contract.json");
    let portfolio = read_json_or(
        &root.join("core/state/source_absorption_portfolio.json"),
        json!({}),
    );
    let runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    )
    .get("surfaces")
    .and_then(|v| v.get("search_runtime"))
    .cloned()
    .unwrap_or_else(|| json!({}));
    let source = portfolio
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("source_id").and_then(Value::as_str) == Some("src_86fa4360"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let brief = source
        .get("implementation_brief")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let runtime_status = runtime
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("not_running");
    let ready = runtime
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (activation_status, integration_state) = if runtime_status == "running" && ready {
        ("active_in_system", "bounded_optional_service")
    } else if matches!(runtime_status, "not_running" | "docker_unavailable") {
        ("governed_on_demand", "contract_ready_not_activated")
    } else {
        ("attention_required", "service_unhealthy")
    };

    let payload = json!({
        "schema_version": "annunimas.search-runtime-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_portfolio",
        "campaign": {
            "name": "Search Runtime Absorption",
            "owner": "athena_prometheus",
            "source_id": "src_86fa4360",
            "mission": "Define the governed retrieval adapter and package posture for self-hosted search backends before any activation decision.",
        },
        "source": {
            "title": source.get("title").cloned().unwrap_or(Value::Null),
            "url": source.get("url").cloned().unwrap_or(Value::Null),
            "implementation_brief": brief,
        },
        "athena_retrieval_adapter": {
            "mode": "governed_on_demand",
            "adapter_boundary": "retrieval happens behind ATHENA-owned query and evidence surfaces",
            "required_controls": [
                "explicit query provenance",
                "retention and privacy posture",
                "receipt capture for retrieved result sets",
            ],
            "runtime_base_url": runtime.get("base_url").cloned().unwrap_or(Value::Null),
        },
        "package_posture": {
            "activation_status": activation_status,
            "integration_state": integration_state,
            "runtime_mode": "optional_self_hosted_search",
            "promotion_rule": "do not activate until privacy, indexing, and operational cost posture are bounded",
            "runtime_env_contract": [
                "ANNUNIMAS_SEARCH_RUNTIME_URL",
                "ANNUNIMAS_SEARCH_RUNTIME_HOST",
                "ANNUNIMAS_SEARCH_RUNTIME_PORT",
                "ANNUNIMAS_SEARCH_RUNTIME_IMAGE",
                "ANNUNIMAS_SEARCH_RUNTIME_CONTAINER_NAME",
            ],
            "service_probe": runtime,
        },
        "governor_notes": {
            "why_not_default": [
                "search backends broaden network and privacy exposure",
                "operational indexing and retention costs need explicit budgeting",
                "retrieval quality does not justify default activation without a bounded contract",
            ],
            "acceptable_uses": [
                "operator-approved retrieval augmentation",
                "ATHENA evidence harvest with receipts",
                "bounded internal search experiments",
            ],
            "launcher": "scripts/runtime/searxng_service.sh",
        },
        "summary": {
            "activation_status": activation_status,
            "adapter_mode": "governed_on_demand",
            "required_controls_total": 3,
            "service_status": runtime_status,
            "ready": ready,
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root), "source_id": "src_86fa4360" }))
}

pub(crate) fn export_scrapling_runtime_contract_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/scrapling_runtime_contract.json");
    let runtime_env = read_env_assignments(&root.join("config/runtime.env.example"));
    let shared_env = read_env_assignments(&root.join("config/.env.example"));
    let package_enablement =
        read_json_or(&root.join("core/state/package_enablement.json"), json!({}));
    let package_runtime = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    );
    let scrapling_book_path = root.join("data/athena/books/src_df11630e.jsonl");
    let crawl_receipts = read_jsonl_objects_local(&root.join("data/athena/crawl_receipts.jsonl"))
        .into_iter()
        .filter(|row| {
            row.get("source_id").and_then(Value::as_str) == Some("src_df11630e")
                && row.get("success").and_then(Value::as_bool).unwrap_or(false)
        })
        .collect::<Vec<_>>();

    let configured_order = runtime_env
        .get("ANNUNIMAS_ATHENA_CRAWL_PROVIDER")
        .cloned()
        .or_else(|| shared_env.get("ANNUNIMAS_ATHENA_CRAWL_PROVIDER").cloned())
        .unwrap_or_else(|| "crawl4ai,scrapling".to_string());
    let crawl_profile = runtime_env
        .get("ANNUNIMAS_ATHENA_CRAWL_PROFILE")
        .cloned()
        .or_else(|| shared_env.get("ANNUNIMAS_ATHENA_CRAWL_PROFILE").cloned())
        .unwrap_or_else(|| "production".to_string());
    let order = configured_order
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let fetch_command = "annunimas-cli utility scrapling-fetch".to_string();
    let runtime_mode = runtime_env
        .get("ANNUNIMAS_SCRAPLING_RUNTIME_MODE")
        .cloned()
        .or_else(|| shared_env.get("ANNUNIMAS_SCRAPLING_RUNTIME_MODE").cloned())
        .unwrap_or_else(|| "shim_allowed".to_string());

    let shim_backed = true;
    let scrapling_runtime = shell_surface("scripts/runtime/scrapling_runtime.sh", "status", &root);
    let runtime_surfaces = package_runtime
        .get("surfaces")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let package_row = package_enablement
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("crawl4ai"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let crawl_runtime = runtime_surfaces
        .get("crawl4ai")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let native_runtime_bounded = scrapling_runtime
        .get("fail_closed_native_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && root.join("scripts/runtime/scrapling_runtime.sh").exists();
    let provider_policy_aligned = order.first().map(|v| v.as_str()) == Some("crawl4ai");
    let shared_receipts_ready = !crawl_receipts.is_empty();
    let promotion_gates_passed = [
        native_runtime_bounded,
        provider_policy_aligned,
        shared_receipts_ready,
    ]
    .into_iter()
    .filter(|gate| *gate)
    .count();

    let payload = json!({
        "schema_version": "annunimas.scrapling-runtime-contract.v1",
        "generated_at_utc": now_utc(),
        "authority": "athena_ingest_policy + runtime_env + source_books",
        "campaign": {
            "name": "Scrapling bounded runtime promotion",
            "owner": "athena",
            "source_id": "src_df11630e",
            "mission": "Promote Scrapling from repo-digested preferred future into a bounded sovereign runtime contract without displacing the live crawl4ai ingest lane prematurely.",
        },
        "doctrine": {
            "live_primary_provider": "crawl4ai",
            "preferred_future_provider": "scrapling",
            "switch_primary_before_contract": false,
            "scrapling_must_be_bounded_before_primary_promotion": true,
            "provider_policy_must_be_machine_readable": true,
        },
        "current_posture": {
            "configured_provider_order": order,
            "configured_primary": order.first().cloned(),
            "crawl_profile": crawl_profile,
            "live_primary_status": {
                "tool": "crawl4ai",
                "activation_status": package_row.get("activation_status").cloned().unwrap_or(Value::Null),
                "runtime_status": crawl_runtime.get("status").cloned().unwrap_or(Value::Null),
                "runtime_ok": crawl_runtime.get("ok").cloned().unwrap_or(Value::Null),
            },
            "scrapling_fetch_mode": if shim_backed { "shim_backed" } else { "native_module" },
            "scrapling_fetch_entrypoint": {
                "command": fetch_command,
                "kind": "cli_command",
            },
            "scrapling_runtime_mode": runtime_mode,
            "implementation_state": if provider_policy_aligned { "bounded_native_contract_ready" } else { "policy_drift_detected" },
            "source_book_present": scrapling_book_path.exists(),
            "runtime_probe": scrapling_runtime,
        },
        "runtime_contract": {
            "entrypoint": {
                "kind": "cli_command",
                "command": "annunimas-cli utility scrapling-fetch",
            },
            "runtime_probe": "scripts/runtime/scrapling_runtime.sh",
            "required_env": [
                "ANNUNIMAS_ATHENA_CRAWL_PROVIDER",
                "ANNUNIMAS_ATHENA_CRAWL_PROFILE",
                "ANNUNIMAS_SCRAPLING_RUNTIME_MODE",
            ],
            "fallback_env": ["ANNUNIMAS_CRAWL4AI_URL"],
            "bounded_requirements": [
                "Provider order must keep crawl4ai first until Scrapling promotion gates are cleared.",
                "Scrapling invocation must remain behind the ATHENA crawl surface rather than ad hoc shell use.",
                "The fetch path must explicitly declare whether native Scrapling or shim mode is serving the request.",
                "Receipts and markdown artifacts must land in ATHENA crawl surfaces shared with crawl4ai.",
            ],
        },
        "provider_selection_policy": {
            "production_default_order": ["crawl4ai", "scrapling"],
            "research_override_order": ["scrapling", "crawl4ai"],
            "selection_rules": [
                "Default to crawl4ai for continuously available ingest while Scrapling remains shim-backed.",
                "Allow Scrapling-first ordering only for targeted verification or bounded local experiments.",
                "Never switch the sovereign default by changing env templates without updating this contract and ATHENA integration plan.",
            ],
        },
        "receipt_evidence": {
            "shared_receipts_required": true,
            "successful_receipts_total": crawl_receipts.len(),
            "latest_successful_receipts": crawl_receipts.iter().rev().take(3).cloned().collect::<Vec<_>>().into_iter().rev().map(|row| json!({
                "captured_at_utc": row.get("captured_at_utc").cloned().unwrap_or(Value::Null),
                "task_context": row.get("task_context").cloned().unwrap_or(Value::Null),
                "artifact_path": row.get("artifact_path").cloned().unwrap_or(Value::Null),
                "crawl_service_url": row.get("crawl_service_url").cloned().unwrap_or(Value::Null),
                "markdown_bytes": row.get("markdown_bytes").cloned().unwrap_or(Value::Null),
            })).collect::<Vec<_>>(),
            "artifact_surface": "data/athena/crawl_receipts.jsonl",
        },
        "promotion_gates": [
            {
                "gate": "native_runtime_bounded",
                "required": true,
                "status": if native_runtime_bounded { "passed" } else { "pending" },
                "evidence": "annunimas-cli utility scrapling-fetch + scripts/runtime/scrapling_runtime.sh",
            },
            {
                "gate": "provider_policy_aligned",
                "required": true,
                "status": if provider_policy_aligned { "passed" } else { "failed" },
                "evidence": "config/runtime.env.example + config/.env.example",
            },
            {
                "gate": "shared_receipts_and_markdown_artifacts",
                "required": true,
                "status": if shared_receipts_ready { "passed" } else { "pending" },
                "evidence": "data/athena/crawl_receipts.jsonl + data/athena/books/src_df11630e.jsonl",
            },
        ],
        "next_actions": [
            "Replace shim-backed Scrapling mode with a bounded native runtime contract or declare shim mode as intentionally non-promotable.",
            "Keep crawl4ai first in production provider order until native runtime and promotion gates pass.",
            "Bind any future primary-provider change to this contract, `core/state/athena_integration_plan.json`, and runtime env templates in the same change set.",
        ],
        "summary": {
            "configured_primary": order.first().cloned(),
            "crawl_profile": crawl_profile,
            "shim_backed": shim_backed,
            "runtime_mode": runtime_mode,
            "successful_receipts_total": crawl_receipts.len(),
            "promotion_gates_total": 3,
            "promotion_gates_passed": promotion_gates_passed,
        },
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({
        "out": rel(&out_path, &root),
        "configured_primary": payload.get("summary").and_then(|v| v.get("configured_primary")).cloned().unwrap_or(Value::Null),
    }))
}

fn pct(used: f64, cap: Option<f64>) -> Option<f64> {
    let cap = cap?;
    if cap <= 0.0 {
        return None;
    }
    Some(((used / cap) * 100.0 * 100.0).round() / 100.0)
}

fn parse_utc_datetime(value: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    let value = value?;
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn toml_table<'a>(
    value: &'a toml::Value,
    key: &str,
) -> Option<&'a toml::map::Map<String, toml::Value>> {
    value.get(key)?.as_table()
}

fn first_value<'a>(data: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut value = data;
    for key in path {
        value = value.get(*key)?;
    }
    Some(value)
}

fn metric_delta(baseline: &Value, current: &Value, path: &[&str]) -> Value {
    let baseline_value = first_value(baseline, path).cloned().unwrap_or(Value::Null);
    let current_value = first_value(current, path).cloned().unwrap_or(Value::Null);
    let delta = match (baseline_value.as_f64(), current_value.as_f64()) {
        (Some(base), Some(now)) => Value::from(now - base),
        _ => Value::Null,
    };
    json!({
        "metric": path.join("."),
        "baseline": baseline_value,
        "current": current_value,
        "delta": delta,
    })
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

fn read_env_assignments(path: &std::path::Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#') && line.contains('='))
                .filter_map(|line| {
                    line.split_once('=')
                        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

struct EdgeRolloutTarget {
    target_id: &'static str,
    ssh_target: &'static str,
    log_path: &'static str,
    runtime_kind: &'static str,
    models: &'static [&'static str],
    store_root: Option<&'static str>,
}

fn ssh_probe(target: &str, remote: &str) -> (Option<i32>, String, String) {
    match Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=4",
            target,
            remote,
        ])
        .output()
    {
        Ok(output) => (
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ),
        Err(err) => (None, String::new(), err.to_string()),
    }
}

fn check_rollout_target(cfg: &EdgeRolloutTarget) -> Value {
    let mut model_checks = Vec::new();
    let mut completed = 0usize;
    let mut blocked = false;
    for model in cfg.models {
        let remote = if cfg.runtime_kind == "ollama" {
            format!("ollama list 2>/dev/null | awk 'NR>1 {{print $1}}' | grep -Fx '{model}' | head -n 1")
        } else {
            format!(
                "find {} -name '{model}' 2>/dev/null | head -n 1",
                cfg.store_root.unwrap_or("")
            )
        };
        let partial_remote = if cfg.runtime_kind == "ollama" {
            "true".to_string()
        } else {
            format!(
                "find {} -path '*/blobs/*.partial' 2>/dev/null | xargs -r ls -ln 2>/dev/null | awk '{{print $5\" \"$NF}}' | head -n 1",
                cfg.store_root.unwrap_or("")
            )
        };
        let (code, out, err) = ssh_probe(cfg.ssh_target, &remote);
        if err.contains("Operation not permitted") {
            blocked = true;
        }
        let present = code == Some(0) && !out.trim().is_empty();
        if present {
            completed += 1;
        }
        let mut partial_bytes = None::<i64>;
        let mut partial_path = None::<String>;
        if !present && cfg.runtime_kind != "ollama" {
            let (partial_code, partial_out, partial_err) =
                ssh_probe(cfg.ssh_target, &partial_remote);
            if partial_err.contains("Operation not permitted") {
                blocked = true;
            }
            if partial_code == Some(0) && !partial_out.trim().is_empty() {
                let first = partial_out.lines().next().unwrap_or("");
                if let Some((bytes, path)) = first.split_once(' ') {
                    partial_bytes = bytes.parse::<i64>().ok();
                    partial_path = Some(path.to_string());
                }
            }
        }
        model_checks.push(json!({
            "model_artifact": model,
            "present": present,
            "path": if out.trim().is_empty() { Value::Null } else { json!(out.trim()) },
            "partial_path": partial_path,
            "partial_bytes": partial_bytes,
            "error": if err.trim().is_empty() { Value::Null } else { json!(err.trim()) },
        }));
    }

    let (_, out, _) = ssh_probe(cfg.ssh_target, "pgrep -af 'ramalama pull|seed_' || true");
    let mut active_pull = out.lines().any(|line| {
        line.contains("ramalama pull") || line.contains("bash -lc set -euo pipefail; for model")
    });
    if cfg.runtime_kind == "ollama" {
        let (_, out, _) = ssh_probe(
            cfg.ssh_target,
            "pgrep -af 'ollama pull|ollama serve' || true",
        );
        active_pull = active_pull || out.lines().any(|line| line.contains("ollama pull"));
    }
    let (_, log_out, log_err) = ssh_probe(
        cfg.ssh_target,
        &format!("tail -n 20 {} 2>/dev/null || true", cfg.log_path),
    );
    if log_err.contains("Operation not permitted") {
        blocked = true;
    }

    json!({
        "target_id": cfg.target_id,
        "ssh_target": cfg.ssh_target,
        "probe_status": if blocked { "probe_blocked" } else { "ok" },
        "models_total": cfg.models.len(),
        "models_completed": completed,
        "active_pull": active_pull,
        "completion_percent": if cfg.models.is_empty() { 0.0 } else { ((completed as f64 / cfg.models.len() as f64) * 100.0 * 100.0).round() / 100.0 },
        "models": model_checks,
        "log_tail": log_out.lines().map(ToOwned::to_owned).collect::<Vec<_>>(),
        "log_error": if log_err.trim().is_empty() { Value::Null } else { json!(log_err.trim()) },
    })
}

fn summarize_provider(provider: &Value, pressure: Option<&Value>) -> Value {
    let pressure = pressure.cloned().unwrap_or_else(|| json!({}));
    json!({
        "provider_id": provider.get("id").cloned().unwrap_or(Value::Null),
        "enabled": provider.get("enabled").cloned().unwrap_or(Value::Null),
        "healthy": provider.get("healthy").cloned().unwrap_or(Value::Null),
        "base_url": provider.get("base_url").cloned().unwrap_or(Value::Null),
        "requests_per_day": pressure.get("requests_per_day").cloned().unwrap_or(Value::Null),
        "requests_used_day": pressure.get("requests_used_day").cloned().unwrap_or(Value::Null),
        "requests_per_minute": pressure.get("requests_per_minute").cloned().unwrap_or(Value::Null),
        "requests_used_minute": pressure.get("requests_used_minute").cloned().unwrap_or(Value::Null),
        "cooldown_until_utc": pressure.get("cooldown_until_utc").cloned().unwrap_or(Value::Null),
        "in_cooldown": pressure.get("in_cooldown").cloned().unwrap_or(Value::Null),
        "models": provider.get("models").cloned().unwrap_or_else(|| json!([])),
    })
}

fn summarize_node(configured: &Value, observed_rows: &[Value]) -> Value {
    let tailscale_ip = configured.get("tailscale_ip").and_then(Value::as_str);
    let observed = observed_rows.iter().find(|row| {
        tailscale_ip.is_some_and(|tailscale_ip| {
            row.get("tailscale_ips")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .any(|ip| ip == tailscale_ip)
        })
    });
    json!({
        "target_id": configured.get("id").cloned().unwrap_or(Value::Null),
        "role": configured.get("role").cloned().unwrap_or(Value::Null),
        "node_class": configured.get("node_class").cloned().unwrap_or(Value::Null),
        "canonical_hostname": configured.get("hostname").cloned().unwrap_or(Value::Null),
        "tailscale_ip": configured.get("tailscale_ip").cloned().unwrap_or(Value::Null),
        "enrollment_status": configured.get("enrollment_status").cloned().unwrap_or(Value::Null),
        "online": observed.and_then(|row| row.get("online")).and_then(Value::as_bool).unwrap_or(false),
        "observed_hostname": observed.and_then(|row| row.get("hostname")).cloned().unwrap_or(Value::Null),
        "observed_dns_name": observed.and_then(|row| row.get("dns_name")).cloned().unwrap_or(Value::Null),
        "usage": observed.and_then(|row| row.get("usage")).cloned().unwrap_or_else(|| json!({})),
    })
}

fn summarize_rollout_target(target: &Value) -> Value {
    let models_total = target
        .get("models_total")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let models_completed = target
        .get("models_completed")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let active_pull = target
        .get("active_pull")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if models_total > 0 && models_completed >= models_total {
        if active_pull {
            "complete_active"
        } else {
            "complete"
        }
    } else if active_pull {
        "in_progress"
    } else if target
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|model| {
            model.get("error").is_some() && !model.get("error").unwrap_or(&Value::Null).is_null()
        })
    {
        "attention_required"
    } else {
        "pending"
    };
    json!({
        "target_id": target.get("target_id").cloned().unwrap_or(Value::Null),
        "status": status,
        "models_total": models_total,
        "models_completed": models_completed,
        "completion_percent": target.get("completion_percent").cloned().unwrap_or(Value::Null),
        "active_pull": active_pull,
        "artifacts_present": target
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|model| model.get("present").and_then(Value::as_bool) == Some(true))
            .filter_map(|model| model.get("model_artifact").cloned())
            .collect::<Vec<_>>(),
    })
}
