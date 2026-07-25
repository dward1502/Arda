#![cfg(feature = "full-cli")]
use super::super::*;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(crate) fn handle(command: UtilityCommands) -> anyhow::Result<()> {
    let value = match command {
        UtilityCommands::OperatorRuntimeStatus => operator_runtime_status()?,
        UtilityCommands::ProfessionalizationAuditCloseout { audit_dir } => {
            professionalization_audit_closeout(&workspace_root().join(audit_dir))?
        }
        UtilityCommands::CreateCrateSpawnBlueprint {
            crate_name,
            realm,
            output_root,
            force,
            productizable,
        } => create_crate_spawn_blueprint(&crate_name, &realm, &output_root, force, productizable)?,
        UtilityCommands::RepairHadesStores { apply, report_path } => {
            repair_hades_stores(apply, &report_path)?
        }
        UtilityCommands::StampSoterionSigils { apply } => stamp_soterion_sigils(apply)?,
        UtilityCommands::ScraplingFetch { url, filter, query } => {
            scrapling_fetch(&url, &filter, query.as_deref())?
        }
        UtilityCommands::HermesAgentEdgeBridgeListTargets { config, targets } => {
            hermes_agent_edge_bridge_list_targets(config.as_deref(), &targets)?
        }
        UtilityCommands::HermesAgentEdgeBridgePreflight {
            node,
            config,
            targets,
            dry_run,
        } => hermes_agent_edge_bridge_preflight(&node, config.as_deref(), &targets, dry_run)?,
        UtilityCommands::HermesAgentEdgeBridgeProbe {
            node,
            config,
            targets,
            dry_run,
        } => hermes_agent_edge_bridge_probe(&node, config.as_deref(), &targets, dry_run)?,
        UtilityCommands::HermesAgentEdgeBridgeDispatch {
            node,
            prompt,
            config,
            targets,
            toolsets,
            provider,
            model,
            cwd,
            query_memory,
            dry_run,
        } => hermes_agent_edge_bridge_dispatch(
            &node,
            &prompt,
            config.as_deref(),
            &targets,
            toolsets.as_deref(),
            provider.as_deref(),
            model.as_deref(),
            cwd.as_deref(),
            query_memory,
            dry_run,
        )?,
        UtilityCommands::HermesAgentGatewayReceipt {
            task_id,
            background_task_id,
            platform,
            channel,
            status,
            summary,
            verification,
            changed_files,
            blockers,
            next_action,
            dry_run,
        } => hermes_agent_gateway_receipt(
            &task_id,
            background_task_id.as_deref(),
            &platform,
            &channel,
            &status,
            &summary,
            &verification,
            &changed_files,
            &blockers,
            next_action.as_deref(),
            dry_run,
        )?,
        UtilityCommands::HermesAgentGatewayActivationCheck => {
            hermes_agent_gateway_activation_check()?
        }
        UtilityCommands::RemoteConfidence => remote_confidence_snapshot()?,
        UtilityCommands::RemoteConfidencePublish => remote_confidence_snapshot_publish()?,
        UtilityCommands::SafeLocalWorkCyclePreflight => safe_local_work_cycle_preflight()?,
        UtilityCommands::PermissionScopeRefresh {
            profile,
            scope,
            ttl_hours,
            reason,
            dry_run,
        } => permission_scope_refresh(&profile, &scope, ttl_hours, &reason, dry_run)?,
        UtilityCommands::AgentConversationAppend {
            conversation_id,
            topic,
            speaker_agent,
            seat,
            message_class,
            actionability,
            risk_lane,
            summary,
            related_plan,
            related_task,
            related_scout_request,
            confidence,
            source_links,
            receipt_links,
            dry_run,
        } => agent_conversation_append(
            &conversation_id,
            &topic,
            &speaker_agent,
            &seat,
            &message_class,
            &actionability,
            &risk_lane,
            &summary,
            related_plan.as_deref(),
            related_task.as_deref(),
            related_scout_request.as_deref(),
            confidence.as_deref(),
            &source_links,
            &receipt_links,
            dry_run,
        )?,
        UtilityCommands::ScoutRequestAppend {
            scout_request_id,
            requester_agent,
            question,
            desired_output_type,
            allowed_sources,
            risk_lane,
            status,
            target_plan,
            target_task,
            expires_at_utc,
            staleness_policy,
            notes,
            dry_run,
        } => scout_request_append(
            &scout_request_id,
            &requester_agent,
            &question,
            &desired_output_type,
            &allowed_sources,
            &risk_lane,
            &status,
            &target_plan,
            target_task.as_deref(),
            expires_at_utc.as_deref(),
            staleness_policy.as_deref(),
            notes.as_deref(),
            dry_run,
        )?,
        UtilityCommands::ScoutFindingAppend {
            scout_finding_id,
            scout_request_id,
            source_agent,
            title,
            summary,
            source_policy,
            status,
            risk_lane,
            confidence,
            source_links,
            recommended_follow_up_tasks,
            receipt_links,
            dry_run,
        } => scout_finding_append(
            &scout_finding_id,
            &scout_request_id,
            &source_agent,
            &title,
            &summary,
            &source_policy,
            &status,
            &risk_lane,
            confidence.as_deref(),
            &source_links,
            &recommended_follow_up_tasks,
            &receipt_links,
            dry_run,
        )?,
        UtilityCommands::ScoutRuntimeRefresh => scout_runtime_refresh()?,
        UtilityCommands::RemoteConfidenceProducerProof => remote_confidence_producer_proof()?,
        UtilityCommands::TaskPivot {
            title,
            id,
            owner,
            priority,
            status,
            result,
            notes,
            origin,
            scope,
            meta,
            glyph,
            sigil,
            queued_at_utc,
            completed_at_utc,
            dry_run,
        } => task_pivot(
            &title,
            id.as_deref(),
            &owner,
            &priority,
            &status,
            result.as_deref(),
            notes.as_deref(),
            origin.as_deref(),
            scope.as_deref(),
            &meta,
            &glyph,
            sigil.as_deref(),
            queued_at_utc.as_deref(),
            completed_at_utc.as_deref(),
            dry_run,
        )?,
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn permission_scope_refresh(
    profile: &str,
    scope: &str,
    ttl_hours: u64,
    reason: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let scope = scope.trim().to_ascii_lowercase();
    anyhow::ensure!(
        scope == "network",
        "permission scope refresh only supports network; destructive scope requires explicit approval workflow"
    );
    anyhow::ensure!(
        (1..=8).contains(&ttl_hours),
        "ttl_hours must be between 1 and 8"
    );
    anyhow::ensure!(
        !reason.trim().is_empty(),
        "reason is required for permission scope refresh"
    );

    let root = workspace_root();
    let path = root.join("core/state/permission_profiles.json");
    let mut state = read_json(&path);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::hours(ttl_hours as i64);
    let profile_value = state
        .get_mut("profiles")
        .and_then(Value::as_object_mut)
        .and_then(|profiles| profiles.get_mut(profile))
        .ok_or_else(|| anyhow::anyhow!("permission profile '{profile}' not found"))?;
    let scope_value = profile_value
        .get_mut("scopes")
        .and_then(Value::as_object_mut)
        .and_then(|scopes| scopes.get_mut(&scope))
        .ok_or_else(|| anyhow::anyhow!("permission profile '{profile}' has no '{scope}' scope"))?;

    let previous_expires_at = scope_value
        .get("expires_at_utc")
        .and_then(Value::as_str)
        .unwrap_or("missing")
        .to_string();
    scope_value["allowed"] = json!(true);
    scope_value["expires_at_utc"] =
        json!(expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    profile_value["updated_at_utc"] = json!(now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    profile_value["last_scope_refresh"] = json!({
        "scope": scope,
        "reason": reason,
        "ttl_hours": ttl_hours,
        "refreshed_at_utc": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "expires_at_utc": expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "previous_expires_at_utc": previous_expires_at,
    });

    let receipt = json!({
        "contract": "arda.permission_scope_refresh.v1",
        "profile": profile,
        "scope": scope,
        "ttl_hours": ttl_hours,
        "reason": reason,
        "dry_run": dry_run,
        "refreshed_at_utc": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "expires_at_utc": expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "previous_expires_at_utc": previous_expires_at,
        "path": path.display().to_string(),
        "mutation_policy": "bounded_network_scope_refresh_only",
    });

    if !dry_run {
        write_json(&path, &state)?;
        append_jsonl(
            &root.join("data/warden/permission_profile_audit.jsonl"),
            &receipt,
        )?;
    }

    Ok(receipt)
}

fn fetch_json(url: &str, payload: Option<&Value>) -> Value {
    let mut command = Command::new("curl");
    command.arg("-fsS").arg(url);
    if let Some(payload) = payload {
        command.args(["-H", "Content-Type: application/json", "-X", "POST"]);
        command
            .arg("-d")
            .arg(serde_json::to_string(payload).unwrap_or_default());
    }
    command
        .output()
        .ok()
        .and_then(|output| serde_json::from_slice(&output.stdout).ok())
        .unwrap_or_else(|| json!({}))
}

fn parse_utc(value: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(&value.replace('Z', "+00:00"))
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn decay_lane_fitness(raw: &Value) -> Value {
    let Some(lanes) = raw.get("lanes").and_then(Value::as_object) else {
        return json!({});
    };
    let now = Utc::now();
    let mut out = serde_json::Map::new();
    for (lane, providers) in lanes {
        let Some(providers) = providers.as_object() else {
            continue;
        };
        let mut lane_out = serde_json::Map::new();
        for (provider_id, state) in providers {
            let Some(last_result) = state
                .get("last_result_utc")
                .and_then(Value::as_str)
                .and_then(parse_utc)
            else {
                continue;
            };
            let age_hours = (now - last_result).num_seconds().max(0) as f64 / 3600.0;
            if age_hours >= 72.0 {
                continue;
            }
            let decay_factor = 0.5f64.powf(age_hours / 12.0f64.max(0.25));
            let success_count = (state
                .get("success_count")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                * decay_factor)
                .round() as i64;
            let failure_count = (state
                .get("failure_count")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                * decay_factor)
                .round() as i64;
            if success_count == 0 && failure_count == 0 {
                continue;
            }
            let mut next = state.as_object().cloned().unwrap_or_default();
            next.insert("success_count".to_string(), Value::from(success_count));
            next.insert("failure_count".to_string(), Value::from(failure_count));
            lane_out.insert(provider_id.clone(), Value::Object(next));
        }
        if !lane_out.is_empty() {
            out.insert(lane.clone(), Value::Object(lane_out));
        }
    }
    Value::Object(out)
}

fn soft_lane_cap(provider_id: &str, lane: &str) -> i64 {
    match lane {
        "execution" => match provider_id {
            "edge_hub_3080" => 3,
            "edge_worker" => 2,
            "edge_laptop" => 1,
            "edge_guardhouse" => 0,
            _ => 1,
        },
        "background" => match provider_id {
            "edge_guardhouse" => 2,
            "edge_laptop" => 2,
            "edge_worker" => 1,
            "edge_hub_3080" => 1,
            _ => 1,
        },
        _ => match provider_id {
            "edge_worker" => 3,
            "edge_hub_3080" => 2,
            "edge_laptop" => 2,
            "edge_guardhouse" => 1,
            _ => 2,
        },
    }
}

fn scrapling_fetch(url: &str, filter: &str, query: Option<&str>) -> anyhow::Result<Value> {
    let result = scrapling_fetch_markdown(url, filter, query)?;
    Ok(serde_json::to_value(result)?)
}

fn parse_toml_json(path: &Path) -> anyhow::Result<Value> {
    let raw = fs::read_to_string(path)?;
    let value: toml::Value = raw.parse()?;
    Ok(serde_json::to_value(value)?)
}

fn read_jsonl_records(path: &Path) -> anyhow::Result<Vec<Value>> {
    let raw = fs::read_to_string(path)?;
    raw.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str::<Value>(line).map_err(|err| {
                anyhow::anyhow!(
                    "failed to parse JSONL record {} in {}: {err}",
                    index + 1,
                    path.display()
                )
            })
        })
        .collect()
}

fn count_by_status(rows: &[Value]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(status).or_insert(0) += 1;
    }
    counts
}

fn rows_with_status(rows: &[Value], statuses: &[&str]) -> Vec<Value> {
    rows.iter()
        .filter(|row| {
            row.get("status")
                .and_then(Value::as_str)
                .map(|status| statuses.contains(&status))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn professionalization_audit_closeout(audit_dir: &Path) -> anyhow::Result<Value> {
    let findings = read_jsonl_records(&audit_dir.join("findings.jsonl"))?;
    let remediation = read_jsonl_records(&audit_dir.join("remediation-backlog.jsonl"))?;
    let hardening = read_jsonl_records(&audit_dir.join("phase8-hardening-backlog.jsonl"))?;
    let closeout_packet = audit_dir.join("PHASE7_CLOSEOUT_PACKET.md");
    let phase7_validation = audit_dir.join("phase7-validation.txt");
    let phase8_validation = audit_dir.join("phase8-validation.txt");

    let finding_status_counts = count_by_status(&findings);
    let remediation_status_counts = count_by_status(&remediation);
    let hardening_status_counts = count_by_status(&hardening);
    let unresolved_findings = rows_with_status(&findings, &["open", "queued", "in_progress"]);
    let unresolved_remediation = rows_with_status(&remediation, &["open", "queued", "in_progress"]);
    let unresolved_hardening = rows_with_status(&hardening, &["queued", "in_progress", "blocked"]);
    let queued_hardening = rows_with_status(&hardening, &["queued"]);
    let completed_hardening = rows_with_status(&hardening, &["completed"]);

    let phase7_closed = closeout_packet.exists()
        && phase7_validation.exists()
        && unresolved_findings.is_empty()
        && unresolved_remediation.is_empty();

    Ok(json!({
        "schema_version": "arda.professionalization-audit-closeout.v1",
        "generated_at_utc": now_utc(),
        "mode": "read_only_summary",
        "side_effect_policy": {
            "writes_generated_state": false,
            "refreshes_runtime_state": false,
            "reads_audit_ledgers_only": true
        },
        "audit_dir": audit_dir.display().to_string(),
        "evidence_boundary": {
            "kind": "audit_ledger_summary_not_live_runtime_status",
            "statement": "This command reports committed audit evidence and hardening ledger state; it does not verify current service liveness or refresh generated runtime files.",
            "source_files": [
                "findings.jsonl",
                "remediation-backlog.jsonl",
                "phase8-hardening-backlog.jsonl",
                "PHASE7_CLOSEOUT_PACKET.md",
                "phase7-validation.txt",
                "phase8-validation.txt"
            ]
        },
        "phase7": {
            "closeout_status": if phase7_closed { "closed_with_packet" } else { "needs_review" },
            "closeout_packet_present": closeout_packet.exists(),
            "validation_receipt_present": phase7_validation.exists(),
            "findings": {
                "total": findings.len(),
                "by_status": finding_status_counts,
                "unresolved_count": unresolved_findings.len()
            },
            "remediation_backlog": {
                "total": remediation.len(),
                "by_status": remediation_status_counts,
                "unresolved_count": unresolved_remediation.len()
            }
        },
        "phase8": {
            "status": if unresolved_hardening.is_empty() { "complete" } else { "hardening_active" },
            "total_items": hardening.len(),
            "by_status": hardening_status_counts,
            "completed_count": completed_hardening.len(),
            "queued_count": queued_hardening.len(),
            "unresolved_count": unresolved_hardening.len(),
            "queued_items": queued_hardening,
            "unresolved_items": unresolved_hardening,
            "validation_receipt_present": phase8_validation.exists()
        },
        "next_action": queued_hardening.first().map(|row| json!({
            "id": row.get("id").and_then(Value::as_str).unwrap_or("unknown"),
            "title": row.get("title").and_then(Value::as_str).unwrap_or("unknown"),
            "owner": row.get("owner").and_then(Value::as_str).unwrap_or("unknown"),
            "scope": row.get("scope").and_then(Value::as_str).unwrap_or("unknown")
        }))
    }))
}

fn bridge_default_config(root: &Path, requested: Option<&str>) -> anyhow::Result<PathBuf> {
    if let Some(requested) = requested {
        return Ok(root.join(requested));
    }
    let config = root.join("config/hermes_agent_bridge.toml");
    if config.exists() {
        return Ok(config);
    }
    let example = root.join("config/hermes_agent_bridge.example.toml");
    if example.exists() {
        return Ok(example);
    }
    anyhow::bail!("bridge config not found in config/hermes_agent_bridge.toml or example");
}

fn load_bridge_targets(path: &Path) -> anyhow::Result<BTreeMap<String, Value>> {
    let payload = parse_toml_json(path)?;
    let mut out = BTreeMap::new();
    for row in payload
        .get("node")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            out.insert(id.to_string(), row.clone());
        }
    }
    Ok(out)
}

fn load_bridge_config(path: &Path) -> anyhow::Result<Value> {
    parse_toml_json(path)
}

fn object_map(value: &Value) -> serde_json::Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn bridge_provider_arg(raw: Option<&str>, has_base_url: bool) -> Option<String> {
    let trimmed = raw.unwrap_or_default().trim();
    if trimmed.is_empty() {
        return has_base_url.then(|| "custom".to_string());
    }
    let normalized = trimmed.to_ascii_lowercase();
    if has_base_url
        && matches!(
            normalized.as_str(),
            "auto" | "inherit" | "upstream" | "default" | "*"
        )
    {
        return Some("custom".to_string());
    }
    Some(trimmed.to_string())
}

fn bridge_toolsets_arg(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "inherit" | "upstream" | "default" | "auto" | "*" => None,
        _ => Some(trimmed.to_string()),
    }
}

fn bridge_model_arg(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "auto" | "inherit" | "upstream" | "default" | "*"
    ) {
        return Some("auto".to_string());
    }
    // Harmonic-Hermes is a useful local sovereign model, but upstream Hermes Agent has
    // already proven unreliable for tool-calling continuations when pinned to it directly.
    // Degrade stale bridge pins back to Manwe-owned auto routing instead of preserving a
    // non-agentic execution path that freezes after tool output.
    if normalized == "harmonic-hermes-9b-q6_k" || normalized == "mesh_local/harmonic-hermes-9b-q6_k"
    {
        return Some("auto".to_string());
    }
    Some(trimmed.to_string())
}

fn bridge_node_overrides<'a>(bridge: &'a Value, node_id: &str) -> Option<&'a Value> {
    bridge
        .get("node")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(node_id))
}

fn bridge_target_for_node(
    node_id: &str,
    targets: &BTreeMap<String, Value>,
    overrides: Option<&Value>,
) -> Value {
    let target_id = overrides
        .and_then(|row| row.get("target_id"))
        .and_then(Value::as_str)
        .unwrap_or(node_id);
    targets
        .get(target_id)
        .or_else(|| targets.get(node_id))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn merge_bridge_node(node_id: &str, targets: &BTreeMap<String, Value>, bridge: &Value) -> Value {
    let override_value = bridge_node_overrides(bridge, node_id);
    let target = bridge_target_for_node(node_id, targets, override_value);
    let defaults = bridge.get("defaults").cloned().unwrap_or_else(|| json!({}));
    let overrides = override_value.cloned().unwrap_or_else(|| json!({}));
    let mut merged = object_map(&defaults);
    merged.extend(object_map(&target));
    merged.extend(object_map(&overrides));
    merged.insert("id".to_string(), Value::from(node_id));
    if !merged.contains_key("target_id") {
        if let Some(target_id) = target.get("id").and_then(Value::as_str) {
            merged.insert("target_id".to_string(), Value::from(target_id));
        }
    }
    if !merged.contains_key("host") {
        let host = target
            .get("tailscale_ip")
            .and_then(Value::as_str)
            .or_else(|| target.get("hostname").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        merged.insert("host".to_string(), Value::from(host));
    }
    if !merged.contains_key("ssh_user") {
        let ssh_user = target
            .get("ssh_user")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "arda".to_string());
        merged.insert("ssh_user".to_string(), Value::from(ssh_user));
    }
    merged
        .entry("transport".to_string())
        .or_insert_with(|| Value::from("ssh"));
    let has_base_url = merged
        .get("base_url")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    if let Some(provider) =
        bridge_provider_arg(merged.get("provider").and_then(Value::as_str), has_base_url)
    {
        merged.insert("provider".to_string(), Value::from(provider));
    }
    Value::Object(merged)
}

fn bridge_ssh_target(node: &Value) -> anyhow::Result<String> {
    let user = json_string(node, "ssh_user").unwrap_or_default();
    let host = json_string(node, "host").unwrap_or_default();
    if host.is_empty() {
        anyhow::bail!(
            "node {} has no host/tailscale_ip configured",
            json_string(node, "id").unwrap_or_else(|| "unknown".to_string())
        );
    }
    Ok(if user.is_empty() {
        host
    } else {
        format!("{user}@{host}")
    })
}

fn bridge_remote_cwd_expr(node: &Value) -> String {
    match json_string(node, "remote_cwd")
        .unwrap_or_else(|| "~".to_string())
        .trim()
    {
        "" | "~" => "\"$HOME\"".to_string(),
        other => shell_escape(other),
    }
}

fn build_bridge_remote_command(node: &Value, prompt: &str) -> String {
    let hermes_bin = json_string(node, "hermes_bin").unwrap_or_else(|| "hermes".to_string());
    let bridge_mode = json_string(node, "bridge_mode").unwrap_or_default();
    let base_url = json_string(node, "base_url");
    let provider =
        bridge_provider_arg(json_string(node, "provider").as_deref(), base_url.is_some());
    let api_key = json_string(node, "api_key");
    let mut env_parts = Vec::new();
    let mut parts = if bridge_mode == "python_cli" {
        vec![
            shell_escape(&hermes_bin),
            "cli.py".to_string(),
            "--query".to_string(),
            shell_escape(prompt),
            "--quiet".to_string(),
        ]
    } else {
        vec![
            shell_escape(&hermes_bin),
            "chat".to_string(),
            "-q".to_string(),
            shell_escape(prompt),
        ]
    };
    if let Some(provider) = provider.as_deref() {
        if bridge_mode == "python_cli" {
            if !provider.eq_ignore_ascii_case("custom") {
                parts.push("--provider".to_string());
                parts.push(shell_escape(provider));
            }
        } else if provider.eq_ignore_ascii_case("custom") {
            env_parts.push(format!(
                "HERMES_INFERENCE_PROVIDER={}",
                shell_escape(provider)
            ));
            parts.push("--provider".to_string());
            parts.push(shell_escape(provider));
        } else {
            parts.push("--provider".to_string());
            parts.push(shell_escape(provider));
        }
    }
    if let Some(base_url) = base_url.as_deref() {
        if bridge_mode == "python_cli" {
            parts.push("--base_url".to_string());
            parts.push(shell_escape(base_url));
        } else {
            env_parts.push(format!("OPENAI_BASE_URL={}", shell_escape(base_url)));
        }
    }
    if let Some(api_key) = api_key.as_deref() {
        if bridge_mode == "python_cli" {
            parts.push("--api_key".to_string());
            parts.push(shell_escape(api_key));
        } else {
            env_parts.push(format!("OPENAI_API_KEY={}", shell_escape(api_key)));
        }
    }
    if let Some(model) = json_string(node, "model").and_then(|raw| bridge_model_arg(&raw)) {
        parts.push("--model".to_string());
        parts.push(shell_escape(&model));
    }
    if let Some(toolsets) = json_string(node, "toolsets") {
        if let Some(toolsets) = bridge_toolsets_arg(&toolsets) {
            parts.push("--toolsets".to_string());
            parts.push(shell_escape(&toolsets));
        }
    }
    let command = parts.join(" ");
    if env_parts.is_empty() {
        format!("cd {} && {}", bridge_remote_cwd_expr(node), command)
    } else {
        format!(
            "cd {} && {} {}",
            bridge_remote_cwd_expr(node),
            env_parts.join(" "),
            command
        )
    }
}

fn build_bridge_memory_command(node: &Value) -> String {
    let query = "\n    Summarize my recent work, current focus, and any relevant context from the past 48 hours.\n    Include my identity state and mission focus.\n    Keep it under 500 words.\n    ";
    build_bridge_remote_command(node, query)
}

fn build_bridge_preflight_command(node: &Value) -> String {
    let hermes_bin = json_string(node, "hermes_bin").unwrap_or_else(|| "hermes".to_string());
    let bridge_mode = json_string(node, "bridge_mode").unwrap_or_default();
    if bridge_mode == "python_cli" {
        format!(
            "cd {} && test -f cli.py && test -x {} && {} cli.py --help >/dev/null 2>&1 && printf 'ready\n'",
            bridge_remote_cwd_expr(node),
            shell_escape(&hermes_bin),
            shell_escape(&hermes_bin)
        )
    } else {
        format!(
            "cd {} && command -v {} && {} --help >/dev/null 2>&1 && printf 'ready\n'",
            bridge_remote_cwd_expr(node),
            shell_escape(&hermes_bin),
            shell_escape(&hermes_bin)
        )
    }
}

fn wrap_bridge_remote_command(remote: &str) -> String {
    format!(
        "env -i HOME=\"$HOME\" PATH=$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin TERM=dumb COLUMNS=120 LINES=40 PYTHONUNBUFFERED=1 /bin/bash --noprofile --norc -lc {}",
        shell_escape(remote)
    )
}

fn shell_escape(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

fn append_bridge_ledger(result: &Value) -> anyhow::Result<()> {
    let ledger = workspace_root().join("data/hermes/hermes_agent_bridge.jsonl");
    if let Some(parent) = ledger.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut handle = OpenOptions::new().create(true).append(true).open(ledger)?;
    writeln!(handle, "{}", serde_json::to_string(result)?)?;
    Ok(())
}

fn run_bridge_ssh(node: &Value, remote: &str, dry_run: bool) -> anyhow::Result<Value> {
    let wrapped_remote = wrap_bridge_remote_command(remote);
    let ssh_target = bridge_ssh_target(node)?;
    let timeout_seconds = node
        .get("timeout_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(180)
        .max(1) as u64;
    let connect_timeout_seconds = timeout_seconds.clamp(1, 30);
    let cmd = vec![
        "ssh".to_string(),
        "-F".to_string(),
        "/dev/null".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        format!("ConnectTimeout={connect_timeout_seconds}"),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        ssh_target.clone(),
        wrapped_remote.clone(),
    ];
    let mut result = json!({
        "ts_utc": now_utc(),
        "node_id": json_string(node, "id").unwrap_or_default(),
        "transport": "ssh",
        "ssh_target": ssh_target,
        "remote_command": remote,
        "wrapped_remote_command": wrapped_remote,
        "command": cmd,
        "dry_run": dry_run,
        "timeout_seconds": timeout_seconds as i64,
        "connect_timeout_seconds": connect_timeout_seconds as i64,
    });
    if dry_run {
        return Ok(result);
    }
    let started = Instant::now();
    let mut child = Command::new("ssh")
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let timeout = Duration::from_secs(timeout_seconds);
    let mut timed_out = false;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let proc = child.wait_with_output()?;
    result["returncode"] =
        Value::from(
            proc.status
                .code()
                .unwrap_or(if timed_out { 124 } else { 1 }),
        );
    result["stdout"] = Value::from(String::from_utf8_lossy(&proc.stdout).to_string());
    result["stderr"] = Value::from(String::from_utf8_lossy(&proc.stderr).to_string());
    result["ok"] = Value::from(proc.status.success() && !timed_out);
    result["timed_out"] = Value::from(timed_out);
    result["timeout_seconds"] = Value::from(timeout_seconds as i64);
    result["elapsed_ms"] = Value::from(started.elapsed().as_millis() as u64);
    append_bridge_ledger(&result)?;
    Ok(result)
}

fn load_bridge_node(
    config: Option<&str>,
    targets: &str,
    node: &str,
) -> anyhow::Result<(Value, PathBuf, PathBuf)> {
    let root = workspace_root();
    let targets_path = root.join(targets);
    let config_path = bridge_default_config(&root, config)?;
    let target_rows = load_bridge_targets(&targets_path)?;
    let bridge = load_bridge_config(&config_path)?;
    Ok((
        merge_bridge_node(node, &target_rows, &bridge),
        config_path,
        targets_path,
    ))
}

fn hermes_agent_edge_bridge_list_targets(
    config: Option<&str>,
    targets: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let targets_path = root.join(targets);
    let config_path = bridge_default_config(&root, config)?;
    let target_rows = load_bridge_targets(&targets_path)?;
    let bridge = load_bridge_config(&config_path)?;
    let mut node_ids = target_rows.keys().cloned().collect::<BTreeSet<_>>();
    if let Some(config_nodes) = bridge.get("node").and_then(Value::as_object) {
        node_ids.extend(config_nodes.keys().cloned());
    }
    let rows = node_ids
        .iter()
        .map(|node_id| {
            let node = merge_bridge_node(node_id, &target_rows, &bridge);
            json!({
                "id": node_id,
                "target_id": node.get("target_id").cloned().unwrap_or(Value::Null),
                "host": node.get("host").cloned().unwrap_or(Value::Null),
                "ssh_user": node.get("ssh_user").cloned().unwrap_or(Value::Null),
                "transport": node.get("transport").cloned().unwrap_or(Value::Null),
                "enabled": node.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                "toolsets": node.get("toolsets").cloned().unwrap_or(Value::Null),
                "provider": node.get("provider").cloned().unwrap_or(Value::Null),
                "model": node.get("model").cloned().unwrap_or(Value::Null),
                "purpose": node.get("purpose").cloned().or_else(|| node.get("notes").cloned()).unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "config_path": config_path,
        "targets_path": targets_path,
        "rows": rows,
    }))
}

fn hermes_agent_edge_bridge_preflight(
    node: &str,
    config: Option<&str>,
    targets: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let (node, config_path, targets_path) = load_bridge_node(config, targets, node)?;
    let payload = run_bridge_ssh(&node, &build_bridge_preflight_command(&node), dry_run)?;
    Ok(json!({
        "config_path": config_path,
        "targets_path": targets_path,
        "result": payload,
    }))
}

fn hermes_agent_edge_bridge_probe(
    node: &str,
    config: Option<&str>,
    targets: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let (node, config_path, targets_path) = load_bridge_node(config, targets, node)?;
    let prompt = "Reply with a one-line health acknowledgement and the active model.";
    let payload = run_bridge_ssh(&node, &build_bridge_remote_command(&node, prompt), dry_run)?;
    Ok(json!({
        "config_path": config_path,
        "targets_path": targets_path,
        "result": payload,
    }))
}

#[allow(clippy::too_many_arguments)]
fn hermes_agent_edge_bridge_dispatch(
    node: &str,
    prompt: &str,
    config: Option<&str>,
    targets: &str,
    toolsets: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    cwd: Option<&str>,
    query_memory: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let (mut node, config_path, targets_path) = load_bridge_node(config, targets, node)?;
    if let Some(toolsets) = toolsets {
        node["toolsets"] = Value::from(toolsets);
    }
    if let Some(provider) = provider {
        node["provider"] = Value::from(provider);
    }
    if let Some(model) = model.and_then(bridge_model_arg) {
        node["model"] = Value::from(model);
    }
    if let Some(cwd) = cwd {
        node["remote_cwd"] = Value::from(cwd);
    }
    let memory_payload = if query_memory {
        Some(run_bridge_ssh(
            &node,
            &build_bridge_memory_command(&node),
            dry_run,
        )?)
    } else {
        None
    };
    let payload = run_bridge_ssh(&node, &build_bridge_remote_command(&node, prompt), dry_run)?;
    Ok(json!({
        "config_path": config_path,
        "targets_path": targets_path,
        "memory_result": memory_payload,
        "result": payload,
    }))
}

#[allow(clippy::too_many_arguments)]
fn hermes_agent_gateway_receipt(
    task_id: &str,
    background_task_id: Option<&str>,
    platform: &str,
    channel: &str,
    status: &str,
    summary: &str,
    verification: &[String],
    changed_files: &[String],
    blockers: &[String],
    next_action: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let normalized_status = status.trim().to_ascii_lowercase();
    let gateway_receipt = build_hermes_agent_gateway_receipt(
        task_id,
        background_task_id,
        platform,
        channel,
        &normalized_status,
        summary,
        verification,
        changed_files,
        blockers,
        next_action,
    );
    if dry_run {
        return Ok(gateway_receipt);
    }

    append_jsonl(
        &workspace_root().join("data/hermes/hermes_agent_gateway_receipts.jsonl"),
        &gateway_receipt,
    )?;
    let service = HermesService::new(workspace_root().join("data/hermes"))?;
    let mut adapted_summary = format!(
        "Hermes Agent gateway result from {}/{}: {}",
        platform.trim(),
        channel.trim(),
        summary.trim()
    );
    if let Some(background_task_id) = background_task_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        adapted_summary = format!("{adapted_summary} (background task `{background_task_id}`)");
    }
    let verified =
        normalized_status == "completed" && blockers.is_empty() && !verification.is_empty();
    let risk = if normalized_status == "completed" && blockers.is_empty() {
        arda_orome::types::CommsEventRisk::Low
    } else {
        arda_orome::types::CommsEventRisk::Medium
    };
    let packet = service.record_subagent_completion_packet(
        task_id,
        "hermes_agent_gateway",
        &adapted_summary,
        verification.to_vec(),
        changed_files.to_vec(),
        blockers.to_vec(),
        risk,
        next_action.unwrap_or(
            "Review Hermes Agent gateway result and decide whether to close or continue the task.",
        ),
        verified,
    )?;
    Ok(json!({
        "schema_version": "arda.hermes_agent_gateway_receipt_adapter.v1",
        "mode": "write",
        "gateway_receipt_path": "data/hermes/hermes_agent_gateway_receipts.jsonl",
        "subagent_receipt_path": "data/hermes/messages.jsonl",
        "gateway_receipt": gateway_receipt,
        "subagent_completion": packet,
    }))
}

#[allow(clippy::too_many_arguments)]
fn build_hermes_agent_gateway_receipt(
    task_id: &str,
    background_task_id: Option<&str>,
    platform: &str,
    channel: &str,
    status: &str,
    summary: &str,
    verification: &[String],
    changed_files: &[String],
    blockers: &[String],
    next_action: Option<&str>,
) -> Value {
    json!({
        "schema_version": "arda.hermes_agent_gateway_background_result.v1",
        "receipt_id": format!("hag_{}", Utc::now().timestamp_nanos_opt().unwrap_or_default()),
        "task_ref": if task_id.starts_with("task:") { task_id.to_string() } else { format!("task:{task_id}") },
        "source": "hermes_agent_gateway",
        "platform": platform.trim(),
        "semantic_channel": channel.trim(),
        "background_task_id": background_task_id.map(str::trim).filter(|value| !value.is_empty()),
        "status": status,
        "summary": summary.trim(),
        "verification": verification,
        "changed_files": changed_files,
        "blockers": blockers,
        "next_action": next_action.unwrap_or("Review Hermes Agent gateway result and decide whether to close or continue the task."),
        "policy_boundary": {
            "arda_records_authority": true,
            "gateway_result_is_not_approval": true,
            "requires_review_when_unverified_or_blocked": true
        },
        "created_at_utc": now_utc(),
    })
}

fn append_jsonl(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut handle = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(handle, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn optional_json_string(value: Option<&str>) -> Value {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn validate_contract_enum(field: &str, value: &str, allowed: &[&str]) -> anyhow::Result<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if allowed.iter().any(|allowed| *allowed == normalized) {
        Ok(normalized)
    } else {
        anyhow::bail!(
            "{field} `{normalized}` is not allowed; expected one of {}",
            allowed.join(", ")
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn agent_conversation_append(
    conversation_id: &str,
    topic: &str,
    speaker_agent: &str,
    seat: &str,
    message_class: &str,
    actionability: &str,
    risk_lane: &str,
    summary: &str,
    related_plan: Option<&str>,
    related_task: Option<&str>,
    related_scout_request: Option<&str>,
    confidence: Option<&str>,
    source_links: &[String],
    receipt_links: &[String],
    dry_run: bool,
) -> anyhow::Result<Value> {
    let record = build_agent_conversation_record(
        conversation_id,
        topic,
        speaker_agent,
        seat,
        message_class,
        actionability,
        risk_lane,
        summary,
        related_plan,
        related_task,
        related_scout_request,
        confidence,
        source_links,
        receipt_links,
    )?;
    let root = workspace_root();
    let path = root.join("data/council/agent_conversations.jsonl");
    if dry_run {
        return Ok(json!({
            "schema_version": "arda.council.agent_conversation_append.v1",
            "mode": "dry_run",
            "record_path": path,
            "record": record,
            "side_effect_policy": local_ledger_side_effect_policy()
        }));
    }

    let projection_report = write_agent_conversation_ledger_and_projection(
        &root,
        conversation_id,
        topic,
        speaker_agent,
        seat,
        message_class,
        actionability,
        risk_lane,
        summary,
        related_plan,
        related_task,
        related_scout_request,
        confidence,
        source_links,
        receipt_links,
    )?;
    Ok(json!({
        "schema_version": "arda.council.agent_conversation_append.v1",
        "mode": "write",
        "record_path": path,
        "runtime_path": projection_report["runtime_path"].clone(),
        "record": projection_report["record"].clone(),
        "runtime_projection": projection_report["runtime_projection"].clone(),
        "side_effect_policy": local_ledger_side_effect_policy()
    }))
}

#[allow(clippy::too_many_arguments)]
fn write_agent_conversation_ledger_and_projection(
    root: &Path,
    conversation_id: &str,
    topic: &str,
    speaker_agent: &str,
    seat: &str,
    message_class: &str,
    actionability: &str,
    risk_lane: &str,
    summary: &str,
    related_plan: Option<&str>,
    related_task: Option<&str>,
    related_scout_request: Option<&str>,
    confidence: Option<&str>,
    source_links: &[String],
    receipt_links: &[String],
) -> anyhow::Result<Value> {
    let record = build_agent_conversation_record(
        conversation_id,
        topic,
        speaker_agent,
        seat,
        message_class,
        actionability,
        risk_lane,
        summary,
        related_plan,
        related_task,
        related_scout_request,
        confidence,
        source_links,
        receipt_links,
    )?;
    let record_path = root.join("data/council/agent_conversations.jsonl");
    let runtime_path = root.join("core/state/scout_runtime.json");
    append_jsonl(&record_path, &record)?;
    let runtime = build_scout_runtime_projection_with_record(root, None, None);
    write_json(&runtime_path, &runtime)?;

    Ok(json!({
        "schema_version": "arda.council.agent_conversation_projection_write.v1",
        "mode": "local_ledger_projection_write",
        "record_path": record_path,
        "runtime_path": runtime_path,
        "record": record,
        "runtime_projection": runtime,
        "side_effect_policy": local_ledger_side_effect_policy()
    }))
}

#[allow(clippy::too_many_arguments)]
fn build_agent_conversation_record(
    conversation_id: &str,
    topic: &str,
    speaker_agent: &str,
    seat: &str,
    message_class: &str,
    actionability: &str,
    risk_lane: &str,
    summary: &str,
    related_plan: Option<&str>,
    related_task: Option<&str>,
    related_scout_request: Option<&str>,
    confidence: Option<&str>,
    source_links: &[String],
    receipt_links: &[String],
) -> anyhow::Result<Value> {
    Ok(json!({
        "schema_version": "arda.council.agent_conversation.v1",
        "conversation_id": conversation_id.trim(),
        "ts_utc": now_utc(),
        "topic": topic.trim(),
        "speaker_agent": speaker_agent.trim(),
        "seat": seat.trim(),
        "message_class": validate_contract_enum("message_class", message_class, &["observation", "proposal", "objection", "decision", "receipt", "question"] )?,
        "actionability": validate_contract_enum("actionability", actionability, &["informational", "proposal", "gated_action", "completed_evidence"] )?,
        "risk_lane": validate_contract_enum("risk_lane", risk_lane, &["read_only", "safe_local", "human_gated", "external"] )?,
        "summary": summary.trim(),
        "related_plan": optional_json_string(related_plan),
        "related_task": optional_json_string(related_task),
        "related_scout_request": optional_json_string(related_scout_request),
        "confidence": optional_json_string(confidence),
        "source_links": source_links,
        "receipt_links": receipt_links,
        "policy_boundary": {
            "conversation_is_not_execution_approval": true,
            "external_messages_sent": false,
            "queue_mutated": false
        }
    }))
}

#[allow(clippy::too_many_arguments)]
fn scout_request_append(
    scout_request_id: &str,
    requester_agent: &str,
    question: &str,
    desired_output_type: &str,
    allowed_sources: &str,
    risk_lane: &str,
    status: &str,
    target_plan: &str,
    target_task: Option<&str>,
    expires_at_utc: Option<&str>,
    staleness_policy: Option<&str>,
    notes: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let record = build_scout_request_record(
        scout_request_id,
        requester_agent,
        question,
        desired_output_type,
        allowed_sources,
        risk_lane,
        status,
        target_plan,
        target_task,
        expires_at_utc,
        staleness_policy,
        notes,
    )?;
    let root = workspace_root();
    let path = root.join("data/athena/scout_requests.jsonl");
    let runtime = build_scout_runtime_projection_with_record(&root, Some(&record), None);
    if !dry_run {
        append_jsonl(&path, &record)?;
        write_json(&root.join("core/state/scout_runtime.json"), &runtime)?;
    }
    Ok(json!({
        "schema_version": "arda.athena.scout_request_append.v1",
        "mode": if dry_run { "dry_run" } else { "write" },
        "record_path": path,
        "runtime_path": root.join("core/state/scout_runtime.json"),
        "record": record,
        "runtime_projection": runtime,
        "side_effect_policy": local_ledger_side_effect_policy()
    }))
}

#[allow(clippy::too_many_arguments)]
fn build_scout_request_record(
    scout_request_id: &str,
    requester_agent: &str,
    question: &str,
    desired_output_type: &str,
    allowed_sources: &str,
    risk_lane: &str,
    status: &str,
    target_plan: &str,
    target_task: Option<&str>,
    expires_at_utc: Option<&str>,
    staleness_policy: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<Value> {
    Ok(json!({
        "schema_version": "arda.athena.scout_request.v1",
        "scout_request_id": scout_request_id.trim(),
        "ts_utc": now_utc(),
        "requester_agent": requester_agent.trim(),
        "question": question.trim(),
        "desired_output_type": validate_contract_enum("desired_output_type", desired_output_type, &["design_brief", "implementation_notes", "source_digest", "comparison", "risk_review"] )?,
        "allowed_sources": validate_contract_enum("allowed_sources", allowed_sources, &["local_only", "local_plus_docs", "repo_allowed", "web_allowed"] )?,
        "risk_lane": validate_contract_enum("risk_lane", risk_lane, &["read_only", "safe_local", "human_gated", "external"] )?,
        "status": validate_contract_enum("status", status, &["requested", "in_progress", "satisfied", "cancelled", "stale"] )?,
        "target_plan": target_plan.trim(),
        "target_task": optional_json_string(target_task),
        "expires_at_utc": optional_json_string(expires_at_utc),
        "staleness_policy": optional_json_string(staleness_policy),
        "notes": optional_json_string(notes),
        "policy_boundary": {
            "request_is_not_task_queue_write": true,
            "external_messages_sent": false,
            "queue_mutated": false
        }
    }))
}

#[allow(clippy::too_many_arguments)]
fn scout_finding_append(
    scout_finding_id: &str,
    scout_request_id: &str,
    source_agent: &str,
    title: &str,
    summary: &str,
    source_policy: &str,
    status: &str,
    risk_lane: &str,
    confidence: Option<&str>,
    source_links: &[String],
    recommended_follow_up_tasks: &[String],
    receipt_links: &[String],
    dry_run: bool,
) -> anyhow::Result<Value> {
    let record = build_scout_finding_record(
        scout_finding_id,
        scout_request_id,
        source_agent,
        title,
        summary,
        source_policy,
        status,
        risk_lane,
        confidence,
        source_links,
        recommended_follow_up_tasks,
        receipt_links,
    )?;
    let root = workspace_root();
    let path = root.join("data/athena/scout_findings.jsonl");
    let runtime = build_scout_runtime_projection_with_record(&root, None, Some(&record));
    if !dry_run {
        append_jsonl(&path, &record)?;
        write_json(&root.join("core/state/scout_runtime.json"), &runtime)?;
    }
    Ok(json!({
        "schema_version": "arda.athena.scout_finding_append.v1",
        "mode": if dry_run { "dry_run" } else { "write" },
        "record_path": path,
        "runtime_path": root.join("core/state/scout_runtime.json"),
        "record": record,
        "runtime_projection": runtime,
        "side_effect_policy": local_ledger_side_effect_policy()
    }))
}

#[allow(clippy::too_many_arguments)]
fn build_scout_finding_record(
    scout_finding_id: &str,
    scout_request_id: &str,
    source_agent: &str,
    title: &str,
    summary: &str,
    source_policy: &str,
    status: &str,
    risk_lane: &str,
    confidence: Option<&str>,
    source_links: &[String],
    recommended_follow_up_tasks: &[String],
    receipt_links: &[String],
) -> anyhow::Result<Value> {
    Ok(json!({
        "schema_version": "arda.athena.scout_finding.v1",
        "scout_finding_id": scout_finding_id.trim(),
        "scout_request_id": scout_request_id.trim(),
        "ts_utc": now_utc(),
        "source_agent": source_agent.trim(),
        "title": title.trim(),
        "summary": summary.trim(),
        "source_policy": validate_contract_enum("source_policy", source_policy, &["local_only", "local_plus_docs", "repo_allowed", "web_allowed"] )?,
        "status": validate_contract_enum("status", status, &["found", "partial", "blocked", "stale"] )?,
        "risk_lane": validate_contract_enum("risk_lane", risk_lane, &["read_only", "safe_local", "human_gated", "external"] )?,
        "confidence": optional_json_string(confidence),
        "source_links": source_links,
        "recommended_follow_up_tasks": recommended_follow_up_tasks,
        "receipt_links": receipt_links,
        "policy_boundary": {
            "finding_is_not_task_queue_write": true,
            "external_messages_sent": false,
            "queue_mutated": false
        }
    }))
}

fn scout_runtime_refresh() -> anyhow::Result<Value> {
    let root = workspace_root();
    let runtime = build_scout_runtime_projection_with_record(&root, None, None);
    let path = root.join("core/state/scout_runtime.json");
    write_json(&path, &runtime)?;
    Ok(json!({
        "schema_version": "arda.athena.scout_runtime_refresh.v1",
        "mode": "write",
        "runtime_path": path,
        "runtime_projection": runtime,
        "side_effect_policy": local_ledger_side_effect_policy()
    }))
}

fn remote_confidence_producer_proof() -> anyhow::Result<Value> {
    write_remote_confidence_producer_proof(&workspace_root())
}

fn write_remote_confidence_producer_proof(root: &Path) -> anyhow::Result<Value> {
    let ts = now_utc();
    let id_suffix = ts
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .take(14)
        .collect::<String>();
    let plan = "docs/plans/2026-05-30-autonomous-remote-confidence-console-plan.md";
    let task = "remote_confidence_runtime_producer_wiring";
    let conversation_id = format!("conv_remote_confidence_runtime_{id_suffix}");
    let scout_request_id = format!("scout_remote_confidence_runtime_{id_suffix}");
    let scout_finding_id = format!("finding_remote_confidence_runtime_{id_suffix}");
    let source_links = vec![
        plan.to_string(),
        "docs/contracts/arda-conversation-scout-ledger-contract.md".to_string(),
        "docs/plans/2026-05-30-hermes-discord-gateway-unification-plan.md".to_string(),
    ];
    let receipt_links = vec![
        "data/hermes/discord_operating_room_interactions.jsonl".to_string(),
        "data/hermes/hermes_agent_gateway_receipts.jsonl".to_string(),
    ];

    let conversation = build_agent_conversation_record(
        &conversation_id,
        "Remote confidence runtime producer wiring",
        "prometheus",
        "planning",
        "receipt",
        "completed_evidence",
        "safe_local",
        "Runtime producer proof appended local conversation and scout evidence for ARDA without granting execution authority.",
        Some(plan),
        Some(task),
        Some(&scout_request_id),
        Some("0.91"),
        &source_links,
        &receipt_links,
    )?;
    let request = build_scout_request_record(
        &scout_request_id,
        "prometheus",
        "Verify ARDA remote-confidence producer wiring can consume local conversation and scout evidence ledgers.",
        "implementation_notes",
        "repo_allowed",
        "safe_local",
        "satisfied",
        plan,
        Some(task),
        None,
        Some("refresh when remote-confidence producer contracts change"),
        Some("Generated by remote-confidence-producer-proof; local evidence only."),
    )?;
    let finding = build_scout_finding_record(
        &scout_finding_id,
        &scout_request_id,
        "athena",
        "Remote-confidence producer wiring evidence",
        "The producer proof appends council conversation, scout request, and scout finding records, then refreshes the scout runtime projection consumed by ARDA.",
        "repo_allowed",
        "found",
        "safe_local",
        Some("0.91"),
        &source_links,
        &[],
        &receipt_links,
    )?;

    let conversation_path = root.join("data/council/agent_conversations.jsonl");
    let request_path = root.join("data/athena/scout_requests.jsonl");
    let finding_path = root.join("data/athena/scout_findings.jsonl");
    let runtime_path = root.join("core/state/scout_runtime.json");
    let report_path = root.join("data/prometheus/remote_confidence_producer_proof.json");

    append_jsonl(&conversation_path, &conversation)?;
    append_jsonl(&request_path, &request)?;
    append_jsonl(&finding_path, &finding)?;
    let runtime = build_scout_runtime_projection_with_record(root, None, None);
    write_json(&runtime_path, &runtime)?;

    let report = json!({
        "schema_version": "arda.remote_confidence_producer_proof.v1",
        "mode": "local_runtime_producer_wiring_proof",
        "generated_at_utc": ts,
        "records": {
            "conversation": conversation,
            "scout_request": request,
            "scout_finding": finding
        },
        "paths": {
            "conversation_ledger": conversation_path,
            "scout_request_ledger": request_path,
            "scout_finding_ledger": finding_path,
            "runtime_projection": runtime_path,
            "report": report_path
        },
        "runtime_projection": runtime,
        "side_effect_policy": local_ledger_side_effect_policy()
    });
    write_json(&report_path, &report)?;
    Ok(report)
}

fn build_scout_runtime_projection_with_record(
    root: &Path,
    appended_request: Option<&Value>,
    appended_finding: Option<&Value>,
) -> Value {
    let request_path = root.join("data/athena/scout_requests.jsonl");
    let finding_path = root.join("data/athena/scout_findings.jsonl");
    let conversation_path = root.join("data/council/agent_conversations.jsonl");
    let mut requests = read_jsonl_records_or_empty(&request_path);
    let mut findings = read_jsonl_records_or_empty(&finding_path);
    let conversations = read_jsonl_records_or_empty(&conversation_path);
    if let Some(record) = appended_request {
        requests.push(record.clone());
    }
    if let Some(record) = appended_finding {
        findings.push(record.clone());
    }
    let open_request_count = requests
        .iter()
        .filter(|row| {
            matches!(
                row.get("status").and_then(Value::as_str),
                Some("requested") | Some("in_progress")
            )
        })
        .count();
    let stale_count = requests
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("stale"))
        .count()
        + findings
            .iter()
            .filter(|row| row.get("status").and_then(Value::as_str) == Some("stale"))
            .count();
    json!({
        "schema_version": "arda.athena.scout_runtime.v1",
        "generated_at_utc": now_utc(),
        "source_ledgers": {
            "requests": "data/athena/scout_requests.jsonl",
            "findings": "data/athena/scout_findings.jsonl",
            "conversations": "data/council/agent_conversations.jsonl"
        },
        "summary": {
            "request_count": requests.len(),
            "open_request_count": open_request_count,
            "finding_count": findings.len(),
            "conversation_count": conversations.len(),
            "stale_count": stale_count
        },
        "latest_request": requests.last().cloned().unwrap_or_else(|| json!({})),
        "latest_finding": findings.last().cloned().unwrap_or_else(|| json!({})),
        "latest_conversation": conversations.last().cloned().unwrap_or_else(|| json!({})),
        "side_effect_policy": {
            "projection_only": true,
            "external_messages_sent": false,
            "queue_mutated": false,
            "service_restart": false,
            "credential_change": false
        }
    })
}

fn local_ledger_side_effect_policy() -> Value {
    json!({
        "writes_local_ledger_or_projection": true,
        "external_messages_sent": false,
        "queue_mutated": false,
        "service_restart": false,
        "credential_change": false,
        "destructive_operations": false,
        "mutates_task_status": false,
        "live_gateway_credentials_required": false,
        "live_discord_validation": "human_gated_separate"
    })
}

fn hermes_agent_gateway_activation_check() -> anyhow::Result<Value> {
    let root = workspace_root();
    let hermes_binary = command_output("command", &["-v", "hermes"]);
    let gateway_status = command_output("hermes", &["gateway", "status"]);
    let env = [
        "ARDA_DISCORD_WORK_STREAM_CHANNEL_ID",
        "ARDA_OPERATOR_DISCORD_USER_ID",
        "DISCORD_ALLOWED_USERS",
        "DISCORD_ALLOW_ALL_USERS",
        "GATEWAY_ALLOW_ALL_USERS",
    ]
    .into_iter()
    .map(|key| {
        (
            key.to_string(),
            std::env::var(key)
                .ok()
                .map(|value| redact_env_value(key, &value))
                .unwrap_or_else(|| "missing".to_string()),
        )
    })
    .collect::<BTreeMap<_, _>>();
    Ok(build_hermes_agent_gateway_activation_check(
        &root,
        hermes_binary,
        gateway_status,
        env,
    ))
}

fn remote_confidence_snapshot() -> anyhow::Result<Value> {
    let root = workspace_root();
    let hermes_binary = command_output("command", &["-v", "hermes"]);
    let gateway_status = command_output("hermes", &["gateway", "status"]);
    let env = [
        "ARDA_DISCORD_WORK_STREAM_CHANNEL_ID",
        "ARDA_OPERATOR_DISCORD_USER_ID",
        "DISCORD_ALLOWED_USERS",
        "DISCORD_ALLOW_ALL_USERS",
        "GATEWAY_ALLOW_ALL_USERS",
    ]
    .into_iter()
    .map(|key| {
        (
            key.to_string(),
            std::env::var(key)
                .ok()
                .map(|value| redact_env_value(key, &value))
                .unwrap_or_else(|| "missing".to_string()),
        )
    })
    .collect::<BTreeMap<_, _>>();

    Ok(build_remote_confidence_snapshot(
        &root,
        hermes_binary,
        gateway_status,
        env,
    ))
}

fn remote_confidence_snapshot_publish() -> anyhow::Result<Value> {
    let root = workspace_root();
    let hermes_binary = command_output("command", &["-v", "hermes"]);
    let gateway_status = command_output("hermes", &["gateway", "status"]);
    let env = [
        "ARDA_DISCORD_WORK_STREAM_CHANNEL_ID",
        "ARDA_OPERATOR_DISCORD_USER_ID",
        "DISCORD_ALLOWED_USERS",
        "DISCORD_ALLOW_ALL_USERS",
        "GATEWAY_ALLOW_ALL_USERS",
    ]
    .into_iter()
    .map(|key| {
        (
            key.to_string(),
            std::env::var(key)
                .ok()
                .map(|value| redact_env_value(key, &value))
                .unwrap_or_else(|| "missing".to_string()),
        )
    })
    .collect::<BTreeMap<_, _>>();

    publish_remote_confidence_snapshot(&root, hermes_binary, gateway_status, env)
}

fn publish_remote_confidence_snapshot(
    root: &Path,
    hermes_binary: Value,
    gateway_status: Value,
    env: BTreeMap<String, String>,
) -> anyhow::Result<Value> {
    let target_state_path = root.join("core/state/remote_confidence_snapshot.json");
    let mut snapshot = build_remote_confidence_snapshot(root, hermes_binary, gateway_status, env);
    if let Some(object) = snapshot.as_object_mut() {
        object.insert("mode".to_string(), json!("local_runtime_published"));
        object.insert(
            "publisher".to_string(),
            json!({
                "schema_version": "arda.remote_confidence_publisher.v1",
                "writes_only_local_state_file": true,
                "target_state_path": target_state_path,
                "external_messages_sent": false,
                "live_discord_validation": "human_gated_separate"
            }),
        );
    }
    if let Some(policy) = snapshot
        .get_mut("side_effect_policy")
        .and_then(Value::as_object_mut)
    {
        policy.insert("read_only".to_string(), json!(false));
        policy.insert("writes_generated_state".to_string(), json!(true));
        policy.insert("external_messages_sent".to_string(), json!(false));
        policy.insert("service_restart".to_string(), json!(false));
        policy.insert("credential_change".to_string(), json!(false));
    }
    if let Some(arda_hud) = snapshot.get_mut("arda_hud").and_then(Value::as_object_mut) {
        arda_hud.insert(
            "projection_mode".to_string(),
            json!("local_runtime_state_file"),
        );
        arda_hud.insert("target_state_path".to_string(), json!(target_state_path));
    }

    write_json(&target_state_path, &snapshot)?;
    let bytes_written = fs::metadata(&target_state_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    Ok(json!({
        "schema_version": "arda.remote_confidence_publisher.v1",
        "checked_at_utc": now_utc(),
        "status": "published",
        "writes": {
            "generated_state": true,
            "target_state_path": target_state_path,
            "only_intended_local_state_file": true,
            "bytes_written": bytes_written
        },
        "side_effect_policy": {
            "read_only": false,
            "service_restart": false,
            "credential_change": false,
            "external_messages_sent": false,
            "live_discord_validation": "human_gated_separate"
        },
        "snapshot_schema_version": snapshot.get("schema_version").cloned().unwrap_or_else(|| json!(null)),
        "snapshot_overall_status": snapshot.get("overall_status").cloned().unwrap_or_else(|| json!("unknown")),
        "arda_hud": snapshot.get("arda_hud").cloned().unwrap_or_else(|| json!({}))
    }))
}

fn read_jsonl_records_or_empty(path: &Path) -> Vec<Value> {
    read_jsonl_records(path).unwrap_or_else(|_| Vec::new())
}

fn safe_local_work_cycle_preflight() -> anyhow::Result<Value> {
    let root = workspace_root();
    write_safe_local_work_cycle_preflight(&root)
}

fn write_safe_local_work_cycle_preflight(root: &Path) -> anyhow::Result<Value> {
    let report_path = root.join("data/prometheus/safe_local_work_cycle_preflight.json");
    let report = build_safe_local_work_cycle_preflight(root, &report_path);
    write_json(&report_path, &report)?;
    let bytes_written = fs::metadata(&report_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Ok(json!({
        "schema_version": "arda.safe_local_work_cycle_preflight.v1",
        "checked_at_utc": now_utc(),
        "status": "report_written",
        "report_path": report_path,
        "bytes_written": bytes_written,
        "side_effect_policy": {
            "read_only_intake": true,
            "writes_local_report": true,
            "mutates_task_status": false,
            "external_messages_sent": false,
            "service_restart": false,
            "credential_change": false,
            "destructive_operations": false,
            "live_discord_validation": "human_gated_separate"
        }
    }))
}

fn build_safe_local_work_cycle_preflight(root: &Path, report_path: &Path) -> Value {
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let tasks = latest_task_records(&queue_path);
    let candidate_statuses = [
        "queued",
        "open",
        "ready",
        "pending",
        "in_progress",
        "human_gated",
        "requires_human",
        "awaiting_human",
        "needs_human",
        "blocked",
    ];
    let candidates = tasks
        .iter()
        .filter(|row| status_in(row, &candidate_statuses))
        .map(classify_work_cycle_candidate)
        .collect::<Vec<_>>();
    let safe_local_count = candidates
        .iter()
        .filter(|row| {
            row.get("safe_local_eligible")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let human_gated_count = candidates
        .iter()
        .filter(|row| {
            row.get("required_human_gate")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();

    json!({
        "schema_version": "arda.safe_local_work_cycle_preflight.v1",
        "checked_at_utc": now_utc(),
        "mode": "safe_local_preflight_report",
        "summary": "Classifies candidate work packets without mutating task status or taking external actions.",
        "report_path": report_path,
        "canonical_sources": {
            "task_queue": queue_path,
            "remote_confidence_snapshot": root.join("core/state/remote_confidence_snapshot.json")
        },
        "side_effect_policy": {
            "read_only_intake": true,
            "writes_local_report": true,
            "mutates_task_status": false,
            "external_messages_sent": false,
            "service_restart": false,
            "credential_change": false,
            "destructive_operations": false,
            "live_discord_validation": "human_gated_separate"
        },
        "candidate_summary": {
            "total_count": candidates.len(),
            "safe_local_count": safe_local_count,
            "human_gated_count": human_gated_count
        },
        "candidates": candidates,
        "arda_hud": {
            "future_projection_path": report_path,
            "projection_mode": "local_report_file",
            "new_rail_required": false,
            "forks_autonomy_logic": false
        }
    })
}

fn classify_work_cycle_candidate(row: &Value) -> Value {
    let status = row
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    let action_class = task_action_class(row);
    let human_status = status_in(
        row,
        &[
            "human_gated",
            "requires_human",
            "awaiting_human",
            "needs_human",
            "blocked",
        ],
    ) || row
        .get("human_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_gate_action = matches!(
        action_class.as_str(),
        "external_publication"
            | "external_message"
            | "credential_change"
            | "service_runtime_mutation"
            | "destructive_operation"
            | "money_or_purchase"
    );
    let required_human_gate = human_status || human_gate_action;
    let safe_local_eligible = !required_human_gate
        && matches!(
            action_class.as_str(),
            "safe_local_read_write" | "read_only" | "docs" | "tests" | "code_local"
        );

    let mut blockers = Vec::new();
    if human_status {
        blockers.push("task status or human_required flag requires human gate".to_string());
    }
    if human_gate_action {
        blockers.push(format!(
            "action class {action_class} is outside safe-local autonomous scope"
        ));
    }
    if !safe_local_eligible && blockers.is_empty() {
        blockers.push(format!(
            "action class {action_class} is not recognized as safe-local"
        ));
    }

    json!({
        "id": row.get("id").or_else(|| row.get("task_id")).cloned().unwrap_or_else(|| json!("unknown")),
        "title": row.get("title").or_else(|| row.get("summary")).cloned().unwrap_or_else(|| json!("untitled")),
        "status": status,
        "priority": row.get("priority").cloned().unwrap_or_else(|| json!("unknown")),
        "owner": row.get("owner").cloned().unwrap_or_else(|| json!("unknown")),
        "action_class": action_class,
        "safe_local_eligible": safe_local_eligible,
        "required_human_gate": required_human_gate,
        "blockers": blockers,
        "preflight_decision": if safe_local_eligible { "safe_local_candidate" } else { "human_gated_or_blocked" }
    })
}

fn task_action_class(row: &Value) -> String {
    for key in ["action_class", "risk", "scope_class"] {
        if let Some(value) = row.get(key).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return normalize_action_class(value);
            }
        }
    }
    if let Some(meta) = row.get("meta") {
        for key in ["action_class", "risk", "scope_class"] {
            if let Some(value) = meta.get(key).and_then(Value::as_str) {
                if !value.trim().is_empty() {
                    return normalize_action_class(value);
                }
            }
        }
    }
    let haystack = [
        row.get("title").and_then(Value::as_str).unwrap_or(""),
        row.get("summary").and_then(Value::as_str).unwrap_or(""),
        row.get("notes").and_then(Value::as_str).unwrap_or(""),
    ]
    .join(" ")
    .to_ascii_lowercase();
    if haystack.contains("credential") || haystack.contains("secret") || haystack.contains("token")
    {
        "credential_change".to_string()
    } else if haystack.contains("restart")
        || haystack.contains("systemctl")
        || haystack.contains("service")
    {
        "service_runtime_mutation".to_string()
    } else if haystack.contains("discord")
        || haystack.contains("external message")
        || haystack.contains("publish")
    {
        "external_message".to_string()
    } else if haystack.contains("delete")
        || haystack.contains("destroy")
        || haystack.contains("remove")
    {
        "destructive_operation".to_string()
    } else if haystack.contains("doc") || haystack.contains("plan") {
        "docs".to_string()
    } else if haystack.contains("test") || haystack.contains("contract") {
        "tests".to_string()
    } else {
        "safe_local_read_write".to_string()
    }
}

fn normalize_action_class(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "safe_local" | "safe_local_review_only" | "safe_local_readwrite" => {
            "safe_local_read_write".to_string()
        }
        "external" | "live_discord" | "discord" => "external_message".to_string(),
        "service_mutation" | "runtime_mutation" | "restart" => {
            "service_runtime_mutation".to_string()
        }
        other => other.to_string(),
    }
}

fn latest_jsonl_record(path: &Path) -> Value {
    read_jsonl_records_or_empty(path)
        .into_iter()
        .last()
        .unwrap_or_else(|| json!({}))
}

fn latest_jsonl_records(path: &Path, limit: usize) -> Vec<Value> {
    let rows = read_jsonl_records_or_empty(path);
    rows.into_iter()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn latest_task_records(path: &Path) -> Vec<Value> {
    let mut latest = BTreeMap::<String, Value>::new();
    for row in read_jsonl_records_or_empty(path) {
        let Some(id) = row
            .get("id")
            .or_else(|| row.get("task_id"))
            .and_then(Value::as_str)
            .map(str::trim)
        else {
            continue;
        };
        if !id.is_empty() {
            latest.insert(id.to_string(), row);
        }
    }
    latest.into_values().collect()
}

fn status_in(row: &Value, statuses: &[&str]) -> bool {
    row.get("status")
        .and_then(Value::as_str)
        .map(|status| {
            statuses
                .iter()
                .any(|candidate| status.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

fn compact_task(row: &Value) -> Value {
    json!({
        "id": row.get("id").or_else(|| row.get("task_id")).cloned().unwrap_or_else(|| json!("unknown")),
        "title": row.get("title").or_else(|| row.get("summary")).cloned().unwrap_or_else(|| json!("untitled")),
        "status": row.get("status").cloned().unwrap_or_else(|| json!("unknown")),
        "priority": row.get("priority").cloned().unwrap_or_else(|| json!("unknown")),
        "owner": row.get("owner").cloned().unwrap_or_else(|| json!("unknown")),
    })
}

fn build_remote_confidence_snapshot(
    root: &Path,
    hermes_binary: Value,
    gateway_status: Value,
    env: BTreeMap<String, String>,
) -> Value {
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let autonomy_path = root.join("core/control/autonomy/state.json");
    let gateway_state_path = if root == workspace_root() {
        PathBuf::from("/var/home/mythos/.hermes/gateway_state.json")
    } else {
        root.join(".hermes/gateway_state.json")
    };
    let latest_packet_path = root.join("data/flywheel/latest_packet.json");
    let readiness_path = root.join("core/state/flywheel_packet_readiness.json");
    let scout_requests_path = root.join("data/athena/scout_requests.jsonl");
    let scout_findings_path = root.join("data/athena/scout_findings.jsonl");
    let council_decisions_path = root.join("data/prometheus/council_decisions.jsonl");
    let gateway_receipts_path = root.join("data/hermes/hermes_agent_gateway_receipts.jsonl");

    let tasks = latest_task_records(&queue_path);
    let open_statuses = ["queued", "open", "ready", "in_progress", "pending"];
    let human_gate_statuses = [
        "human_gated",
        "requires_human",
        "awaiting_human",
        "needs_human",
        "blocked",
    ];
    let open_tasks = tasks
        .iter()
        .filter(|row| status_in(row, &open_statuses))
        .map(compact_task)
        .collect::<Vec<_>>();
    let human_required_gates = tasks
        .iter()
        .filter(|row| {
            status_in(row, &human_gate_statuses)
                || row
                    .get("human_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .map(compact_task)
        .collect::<Vec<_>>();
    let completed_count = tasks
        .iter()
        .filter(|row| status_in(row, &["completed", "done", "closed"]))
        .count();

    let gateway_state = read_json(&gateway_state_path);
    let gateway_status_running = gateway_status
        .get("stdout")
        .and_then(Value::as_str)
        .map(hermes_gateway_status_is_running)
        .unwrap_or(false)
        && gateway_status
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let gateway_state_running = gateway_state
        .get("gateway_state")
        .and_then(Value::as_str)
        .map(|state| state.eq_ignore_ascii_case("running"))
        .unwrap_or(false);
    let gateway_discord_connected = gateway_state
        .get("platforms")
        .and_then(|platforms| platforms.get("discord"))
        .and_then(|discord| discord.get("state"))
        .and_then(Value::as_str)
        .map(|state| state.eq_ignore_ascii_case("connected"))
        .unwrap_or(false);
    let gateway_running = gateway_status_running || gateway_state_running;
    let unsafe_allow_all = env
        .iter()
        .filter(|(key, value)| {
            key.contains("ALLOW_ALL") && matches!(value.as_str(), "true" | "1" | "yes")
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let scout_requests = read_jsonl_records_or_empty(&scout_requests_path);
    let scout_findings = read_jsonl_records_or_empty(&scout_findings_path);
    let open_scout_requests = scout_requests
        .iter()
        .filter(|row| status_in(row, &open_statuses))
        .count();

    let mut attention = Vec::new();
    if !gateway_running {
        attention.push("Hermes Agent gateway is not reporting running".to_string());
    }
    if !unsafe_allow_all.is_empty() {
        attention.push(format!(
            "unsafe allow-all gateway env values are enabled: {}",
            unsafe_allow_all.join(",")
        ));
    }
    if !human_required_gates.is_empty() {
        attention.push(format!(
            "{} task(s) require human attention",
            human_required_gates.len()
        ));
    }

    json!({
        "schema_version": "arda.remote_confidence_snapshot.v1",
        "checked_at_utc": now_utc(),
        "mode": "read_only",
        "overall_status": if attention.is_empty() { "nominal" } else { "attention_required" },
        "summary": if attention.is_empty() { "System is inspectable remotely with no human gates visible in the read-only snapshot." } else { "System is inspectable remotely, but attention gates are present." },
        "side_effect_policy": {
            "read_only": true,
            "service_restart": false,
            "credential_change": false,
            "writes_generated_state": false,
            "external_messages_sent": false
        },
        "primary_consoles": ["ARDA HUD", "Hermes Agent CLI/TUI"],
        "arda_hud": {
            "target_state_path": root.join("core/state/remote_confidence_snapshot.json"),
            "projection_mode": "read_only_cli_output_until_runtime_publisher_exists",
            "primary_console": true
        },
        "discord": {
            "role": "remote_confidence_surface",
            "primary_console": false,
            "runtime_required": false,
            "expected_command_path": "/gateway action:remote_confidence or utility remote-confidence",
            "connected": gateway_discord_connected
        },
        "gateway": {
            "running": gateway_running,
            "status_running": gateway_status_running,
            "state_running": gateway_state_running,
            "discord_connected": gateway_discord_connected,
            "hermes_binary": hermes_binary,
            "gateway_status": gateway_status,
            "gateway_state_path": gateway_state_path,
            "gateway_state_present": gateway_state_path.exists()
        },
        "autonomy": {
            "state_path": autonomy_path,
            "state_present": autonomy_path.exists(),
            "mode": read_json(&autonomy_path).get("mode").cloned().unwrap_or_else(|| json!("unknown")),
            "holds": read_json(&autonomy_path).get("holds").cloned().unwrap_or_else(|| json!([]))
        },
        "flywheel": {
            "latest_packet_path": latest_packet_path,
            "latest_packet": read_json(&latest_packet_path),
            "readiness_path": readiness_path,
            "readiness": read_json(&readiness_path)
        },
        "tasks": {
            "queue_path": queue_path,
            "total_count": tasks.len(),
            "open_count": open_tasks.len(),
            "human_gated_count": human_required_gates.len(),
            "completed_count": completed_count,
            "open": open_tasks.iter().take(10).cloned().collect::<Vec<_>>()
        },
        "scout_queue": {
            "request_path": scout_requests_path,
            "finding_path": scout_findings_path,
            "request_count": scout_requests.len(),
            "open_request_count": open_scout_requests,
            "finding_count": scout_findings.len(),
            "latest_request": scout_requests.last().cloned().unwrap_or_else(|| json!({})),
            "latest_finding": scout_findings.last().cloned().unwrap_or_else(|| json!({}))
        },
        "last_council_decision": latest_jsonl_record(&council_decisions_path),
        "last_completion_receipts": latest_jsonl_records(&gateway_receipts_path, 3),
        "human_required_gates": human_required_gates,
        "attention": attention,
        "env": env,
    })
}

fn command_output(command: &str, args: &[&str]) -> Value {
    match Command::new(command).args(args).output() {
        Ok(output) => json!({
            "ok": output.status.success(),
            "returncode": output.status.code().unwrap_or(1),
            "stdout": String::from_utf8_lossy(&output.stdout).trim(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim(),
        }),
        Err(err) => json!({
            "ok": false,
            "returncode": 127,
            "stdout": "",
            "stderr": err.to_string(),
        }),
    }
}

fn redact_env_value(key: &str, value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "empty".to_string();
    }
    if key.contains("ALLOW_ALL") {
        return trimmed.to_ascii_lowercase();
    }
    "set_redacted".to_string()
}

fn build_hermes_agent_gateway_activation_check(
    root: &Path,
    hermes_binary: Value,
    gateway_status: Value,
    env: BTreeMap<String, String>,
) -> Value {
    let plan_path = root.join("docs/plans/2026-05-30-hermes-discord-gateway-unification-plan.md");
    let runbook_path = root.join("docs/operations/hermes-agent-discord-gateway-runbook.md");
    let template_path = root.join("config/hermes_agent_gateway_arda.example.yaml");
    let semantic_channel_path = root.join("crates/arda-hermes/src/service/semantic_channel.rs");
    let serenity_bot_path = root.join("crates/arda-hermes/src/serenity_bot.rs");
    let arda_env_path = root.join("config/.env");
    let hermes_env_path = if root == workspace_root() {
        PathBuf::from("/var/home/mythos/.hermes/.env")
    } else {
        root.join(".hermes/.env")
    };
    let gateway_state_path = if root == workspace_root() {
        PathBuf::from("/var/home/mythos/.hermes/gateway_state.json")
    } else {
        root.join(".hermes/gateway_state.json")
    };
    let arda_env = read_env_file(&arda_env_path);
    let hermes_env = read_env_file(&hermes_env_path);
    let gateway_state = read_json(&gateway_state_path);
    let work_stream_candidate = first_present_env(
        &env,
        &arda_env,
        &hermes_env,
        &[
            "ARDA_DISCORD_WORK_STREAM_CHANNEL_ID",
            "DISCORD_CHANNEL_WORK_STREAM",
            "DISCORD_CHANNEL_TASKS",
            "DISCORD_FREE_RESPONSE_CHANNELS",
            "DISCORD_ALLOWED_CHANNELS",
        ],
    );
    let operator_candidate = first_present_env(
        &env,
        &arda_env,
        &hermes_env,
        &[
            "ARDA_OPERATOR_DISCORD_USER_ID",
            "DISCORD_ALLOWED_USERS",
            "DISCORD_ADMIN_USERS",
        ],
    );
    let template = fs::read_to_string(&template_path).unwrap_or_default();
    let semantic_source = fs::read_to_string(&semantic_channel_path).unwrap_or_default();
    let serenity_source = fs::read_to_string(&serenity_bot_path).unwrap_or_default();
    let work_stream_registered = semantic_source.contains("\"work-stream\"")
        && semantic_source.contains("\"workstream\"")
        && semantic_source.contains("\"work\"")
        && semantic_source.contains("\"tasks\"");
    let discord_gateway_surface_ready = serenity_source.contains("CreateCommand::new(\"gateway\")")
        && serenity_source.contains("\"activation_check\"")
        && serenity_source.contains("\"remote_confidence\"")
        && serenity_source.contains("\"record_receipt\"")
        && serenity_source.contains("arda.hermes_agent_gateway_background_result.v1")
        && serenity_source.contains("gateway_result_is_not_approval");
    let operating_room_commands = [
        "/plans",
        "/tasks",
        "/task",
        "/review",
        "/continue",
        "/council",
        "/gateway",
    ];
    let operating_room_surface_ready = [
        "plans", "tasks", "task", "review", "continue", "council", "gateway",
    ]
    .into_iter()
    .all(|command| serenity_source.contains(&format!("CreateCommand::new(\"{command}\")")))
        && serenity_source.contains("arda.hermes.work_stream_continuation_request.v1")
        && serenity_source.contains("arda.hermes.discord_operating_room_interaction.v1")
        && !serenity_source.contains("CreateCommand::new(\"citadel\")")
        && !serenity_source.contains("CreateCommand::new(\"citadel_");
    let channel_plan = json!({
        "source_path": semantic_channel_path,
        "has_work_stream": work_stream_registered,
        "env_key": "DISCORD_CHANNEL_WORK_STREAM",
        "fallback": "tasks",
        "read_only_source_check": true,
    });

    let plan_ready = plan_path.exists() && runbook_path.exists() && template_path.exists();
    let receipt_adapter_ready = true;
    let hermes_available = hermes_binary
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let gateway_status_running = gateway_status
        .get("stdout")
        .and_then(Value::as_str)
        .map(hermes_gateway_status_is_running)
        .unwrap_or(false)
        && gateway_status
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let gateway_state_running = gateway_state
        .get("gateway_state")
        .and_then(Value::as_str)
        .map(|state| state.eq_ignore_ascii_case("running"))
        .unwrap_or(false);
    let gateway_discord_connected = gateway_state
        .get("platforms")
        .and_then(|platforms| platforms.get("discord"))
        .and_then(|discord| discord.get("state"))
        .and_then(Value::as_str)
        .map(|state| state.eq_ignore_ascii_case("connected"))
        .unwrap_or(false);
    let gateway_running =
        gateway_status_running || (gateway_state_running && gateway_discord_connected);
    let unsafe_allow_all = env
        .iter()
        .filter(|(key, value)| {
            key.contains("ALLOW_ALL") && matches!(value.as_str(), "true" | "1" | "yes")
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let exact_env_missing = [
        "ARDA_DISCORD_WORK_STREAM_CHANNEL_ID",
        "ARDA_OPERATOR_DISCORD_USER_ID",
    ]
    .into_iter()
    .filter(|key| env.get(*key).map(String::as_str) == Some("missing"))
    .collect::<Vec<_>>();
    let required_live_mapping_missing = [
        ("work_stream_channel", work_stream_candidate.is_none()),
        ("operator_user", operator_candidate.is_none()),
    ]
    .into_iter()
    .filter_map(|(name, missing)| missing.then_some(name))
    .collect::<Vec<_>>();
    let template_has_policy = template.contains("allow_from")
        && template.contains("allow_admin_from")
        && template.contains("background_process_notifications: result")
        && template.contains("hermes-agent-gateway-receipt");
    let mut blockers = Vec::new();
    if !plan_ready {
        blockers.push("missing canonical plan, runbook, or gateway template".to_string());
    }
    if !work_stream_registered {
        blockers.push("work-stream is not present in the HERMES semantic channel plan".to_string());
    }
    if !discord_gateway_surface_ready {
        blockers.push(
            "Discord /gateway activation_check and record_receipt surfaces are not ready"
                .to_string(),
        );
    }
    if !operating_room_surface_ready {
        blockers.push(
            "Discord operating-room command surface is incomplete or still advertises CITADEL"
                .to_string(),
        );
    }
    if !hermes_available {
        blockers.push("Hermes Agent CLI is not available on PATH".to_string());
    }
    if !template_has_policy {
        blockers.push(
            "gateway template is missing allowlist/admin/progress/receipt policy".to_string(),
        );
    }
    if !unsafe_allow_all.is_empty() {
        blockers.push(format!(
            "unsafe allow-all gateway env values are enabled: {}",
            unsafe_allow_all.join(",")
        ));
    }
    if !required_live_mapping_missing.is_empty() {
        blockers.push(format!(
            "live activation mappings are not set: {}",
            required_live_mapping_missing.join(",")
        ));
    }
    if !gateway_running {
        blockers.push(
            "Hermes Agent gateway is not currently running; live smoke tests remain pending"
                .to_string(),
        );
    }

    let safe_local_ready = plan_ready
        && work_stream_registered
        && discord_gateway_surface_ready
        && operating_room_surface_ready
        && receipt_adapter_ready
        && template_has_policy
        && hermes_available
        && unsafe_allow_all.is_empty();

    json!({
        "schema_version": "arda.hermes_agent_gateway_activation_check.v1",
        "mode": "read_only",
        "checked_at_utc": now_utc(),
        "safe_local_ready": safe_local_ready,
        "live_ready": blockers.is_empty(),
        "status": if blockers.is_empty() { "ready_for_live_gateway" } else if safe_local_ready { "safe_local_ready_live_human_gates_pending" } else { "not_ready" },
        "artifacts": {
            "plan": plan_path,
            "runbook": runbook_path,
            "template": template_path,
            "semantic_channel_source": semantic_channel_path,
            "discord_bot_source": serenity_bot_path,
            "arda_env": arda_env_path,
            "hermes_env": hermes_env_path,
            "gateway_state": gateway_state_path,
            "plan_exists": plan_path.exists(),
            "runbook_exists": runbook_path.exists(),
            "template_exists": template_path.exists(),
            "semantic_channel_source_exists": semantic_channel_path.exists(),
            "discord_bot_source_exists": serenity_bot_path.exists(),
            "arda_env_exists": arda_env_path.exists(),
            "hermes_env_exists": hermes_env_path.exists(),
            "gateway_state_exists": gateway_state_path.exists(),
        },
        "hermes_agent": {
            "binary": hermes_binary,
            "gateway_status": gateway_status,
            "gateway_state": gateway_state,
            "gateway_status_running": gateway_status_running,
            "gateway_state_running": gateway_state_running,
            "gateway_discord_connected": gateway_discord_connected,
            "gateway_running": gateway_running,
        },
        "arda_hermes": {
            "channel_plan": channel_plan,
            "receipt_adapter": {
                "command": "cargo run -p arda-cli -- utility hermes-agent-gateway-receipt",
                "ready": receipt_adapter_ready,
                "default_mode": "dry_run"
            },
            "discord_gateway_surface": {
                "source_path": serenity_bot_path,
                "ready": discord_gateway_surface_ready,
                "commands": [
                    "/gateway action:activation_check",
                    "/gateway action:remote_confidence",
                    "/gateway action:record_receipt"
                ],
                "receipt_policy": "gateway_result_is_not_approval"
            },
            "operating_room_surface": {
                "source_path": serenity_bot_path,
                "ready": operating_room_surface_ready,
                "commands": operating_room_commands,
                "continuation_policy": "continue_records_context_without_execution_approval",
                "interaction_receipts": "data/hermes/discord_operating_room_interactions.jsonl",
                "citadel_commands_retired": !serenity_source.contains("CreateCommand::new(\"citadel\")")
                    && !serenity_source.contains("CreateCommand::new(\"citadel_")
            }
        },
        "env": env,
        "live_mapping_candidates": {
            "exact_arda_vars_missing": exact_env_missing,
            "work_stream_channel": work_stream_candidate,
            "operator_user": operator_candidate,
        },
        "human_gates": [
            "confirm live Discord channel IDs and operator user ID",
            "copy allowlist/admin tier values into the active Hermes Agent runtime config",
            "start or restart the Hermes Agent gateway service",
            "run live Discord /whoami, /platform list, /plans, /tasks, /continue, work-stream chat, and one harmless /background smoke test",
            "adapt the smoke-test result with hermes-agent-gateway-receipt --dry-run=false"
        ],
        "blockers": blockers,
    })
}

fn hermes_gateway_status_is_running(stdout: &str) -> bool {
    let normalized = stdout.to_ascii_lowercase();
    if normalized.contains("inactive")
        || normalized.contains("dead")
        || normalized.contains("stopped")
        || normalized.contains("service is stopped")
        || normalized.contains("not running")
    {
        return false;
    }
    normalized.contains("active: active")
        || normalized.contains("active (running)")
        || normalized.contains("service is running")
}

fn read_env_file(path: &Path) -> BTreeMap<String, String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let (key, value) = trimmed.split_once('=')?;
            Some((
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            ))
        })
        .collect()
}

fn first_present_env(
    process_env: &BTreeMap<String, String>,
    arda_env: &BTreeMap<String, String>,
    hermes_env: &BTreeMap<String, String>,
    keys: &[&str],
) -> Option<Value> {
    for key in keys {
        if let Some(value) = process_env
            .get(*key)
            .filter(|value| !matches!(value.as_str(), "missing" | "empty"))
        {
            return Some(json!({
                "key": key,
                "source": "process_env",
                "value": redact_candidate_value(value),
            }));
        }
        if let Some(value) = arda_env.get(*key).filter(|value| !value.trim().is_empty()) {
            return Some(json!({
                "key": key,
                "source": "config/.env",
                "value": redact_candidate_value(value),
            }));
        }
        if let Some(value) = hermes_env
            .get(*key)
            .filter(|value| !value.trim().is_empty())
        {
            return Some(json!({
                "key": key,
                "source": "~/.hermes/.env",
                "value": redact_candidate_value(value),
            }));
        }
    }
    None
}

fn redact_candidate_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("false") || trimmed.eq_ignore_ascii_case("true") {
        return trimmed.to_ascii_lowercase();
    }
    if trimmed.contains(',') {
        return format!("set_redacted_list_{}", trimmed.split(',').count());
    }
    "set_redacted".to_string()
}

fn default_manwe_control_url() -> String {
    std::env::var("ARDA_MANWE_CONTROL_URL")
        .unwrap_or_else(|_| format!("http://{}:{}", "127.0.0.1", 5110))
}

fn manwe_control_endpoint(path: &str) -> String {
    format!(
        "{}/{}",
        default_manwe_control_url().trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn operator_runtime_status() -> anyhow::Result<Value> {
    let root = workspace_root();
    let edge = read_json(&root.join("core/state/edge_endpoint_verification.json"));
    let lane_fitness = read_json(&root.join("data/manwe/lane_fitness.json"));
    let manwe_status = fetch_json(&manwe_control_endpoint("status"), None);
    let providers = fetch_json(&manwe_control_endpoint("providers"), None);
    let live_targets = edge
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            row.get("has_live_endpoint")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let intentional_offline_targets = edge
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            row.get("intentional_offline")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !row
                    .get("has_live_endpoint")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let unexpected_offline_targets = edge
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            !row.get("intentional_offline")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !row
                    .get("has_live_endpoint")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let routable_local = providers
        .get("providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            row.get("enabled").and_then(Value::as_bool).unwrap_or(false)
                && row.get("healthy").and_then(Value::as_bool).unwrap_or(false)
                && row
                    .get("has_api_key")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                && row
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .starts_with("edge_")
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut lane_routes = serde_json::Map::new();
    for (lane, payload) in [
        (
            "interactive",
            json!({"agent_id":"arda_hud_probe","task_type":"chat","priority":"normal","messages":[{"role":"user","content":"hello"}],"options":{}}),
        ),
        (
            "execution",
            json!({"agent_id":"arda_hud_probe","task_type":"code","priority":"high","messages":[{"role":"user","content":"write rust"}],"options":{}}),
        ),
        (
            "background",
            json!({"agent_id":"arda_hud_probe","task_type":"background","priority":"low","messages":[{"role":"user","content":"sweep logs"}],"options":{}}),
        ),
    ] {
        let route = fetch_json(&manwe_control_endpoint("route"), Some(&payload))
            .get("decision")
            .cloned()
            .unwrap_or_else(|| json!({"error":"route_unavailable"}));
        lane_routes.insert(lane.to_string(), route);
    }
    let lane_headroom = ["interactive", "execution", "background"]
        .into_iter()
        .map(|lane| {
            let rows = routable_local
                .iter()
                .map(|row| {
                    let provider_id = row.get("id").and_then(Value::as_str).unwrap_or_default();
                    let active_connections = row
                        .get("active_connections")
                        .and_then(Value::as_i64)
                        .unwrap_or(0);
                    let cap = soft_lane_cap(provider_id, lane).max(1);
                    (
                        provider_id.to_string(),
                        json!((((cap - active_connections).max(0) as f64) / cap as f64)),
                    )
                })
                .collect::<serde_json::Map<String, Value>>();
            (lane.to_string(), Value::Object(rows))
        })
        .collect::<serde_json::Map<String, Value>>();
    let payload = json!({
        "generated_at_utc": now_utc(),
        "authority": "operator_runtime_status",
        "manwe": manwe_status.get("status").cloned().unwrap_or_else(|| json!({})),
        "fleet": edge.get("summary").cloned().unwrap_or_else(|| json!({})),
        "summary": {
            "manwe_http_ok": manwe_status.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "fleet_live_llm_nodes_total": live_targets.len(),
            "fleet_routable_local_providers_total": routable_local.len(),
            "unexpected_offline_total": edge.get("summary").and_then(|v| v.get("targets_unexpected_offline_total")).cloned().unwrap_or(Value::from(0)),
        },
        "lane_headroom": Value::Object(lane_headroom),
        "lane_routes": Value::Object(lane_routes),
        "lane_fitness": decay_lane_fitness(&lane_fitness),
        "intentional_offline_targets": intentional_offline_targets,
        "unexpected_offline_targets": unexpected_offline_targets,
        "live_targets": live_targets,
        "routable_providers": routable_local,
    });
    let out = root.join("core/state/operator_runtime_status.json");
    write_json(&out, &payload)?;
    Ok(payload)
}

fn create_crate_spawn_blueprint(
    crate_name: &str,
    realm: &str,
    output_root: &str,
    force: bool,
    productizable: bool,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let crate_root = root.join(output_root).join(crate_name);
    let write_file = |path: &Path, content: String| -> anyhow::Result<()> {
        if path.exists() && !force {
            anyhow::bail!("refusing to overwrite existing file: {}", path.display());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    };
    let core_path = pathdiff::diff_paths(root.join("crates/arda-core"), &crate_root)
        .unwrap_or_else(|| PathBuf::from("../../arda-core"));
    let governance_path = pathdiff::diff_paths(root.join("crates/arda-governance"), &crate_root)
        .unwrap_or_else(|| PathBuf::from("../../arda-governance"));
    let type_name = crate_name
        .replace('-', "_")
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<String>();
    write_file(
        &crate_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\narda-core = {{ path = \"{}\" }}\narda-governance = {{ path = \"{}\" }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\nchrono = {{ version = \"0.4\", features = [\"serde\"] }}\n\n[dev-dependencies]\nserde_json = \"1\"\n",
            core_path.display(),
            governance_path.display()
        ),
    )?;
    write_file(
        &crate_root.join("src/lib.rs"),
        format!(
            "// sigil: ANKH\npub mod contract;\npub mod service;\n\npub fn crate_identity() -> &'static str {{\n    \"{crate_name}\"\n}}\n"
        ),
    )?;
    write_file(
        &crate_root.join("src/contract.rs"),
        format!(
            "// sigil: ANKH\nuse serde::Serialize;\n\n#[derive(Debug, Clone, Serialize)]\npub struct {type_name}Contract {{\n    pub crate_name: &'static str,\n    pub realm: &'static str,\n    pub productizable: bool,\n    pub state_export_path: &'static str,\n}}\n\npub fn contract() -> {type_name}Contract {{\n    {type_name}Contract {{\n        crate_name: \"{crate_name}\",\n        realm: \"{realm}\",\n        productizable: {productizable},\n        state_export_path: \"core/state/{crate_name}.json\",\n    }}\n}}\n"
        ),
    )?;
    write_file(
        &crate_root.join("src/service.rs"),
        format!(
            "// sigil: ANKH\nuse crate::contract::contract;\nuse serde::Serialize;\n\n#[derive(Debug, Clone, Serialize)]\npub struct {type_name}Status {{\n    pub crate_name: &'static str,\n    pub realm: &'static str,\n    pub productizable: bool,\n    pub state_export_path: &'static str,\n    pub governance_ready: bool,\n}}\n\npub fn status() -> {type_name}Status {{\n    let base = contract();\n    {type_name}Status {{\n        crate_name: base.crate_name,\n        realm: base.realm,\n        productizable: base.productizable,\n        state_export_path: base.state_export_path,\n        governance_ready: true,\n    }}\n}}\n"
        ),
    )?;
    write_file(
        &crate_root.join("README.md"),
        format!(
            "# {crate_name}\n\nSpawned from the arda sovereign crate blueprint.\n\n- Realm: `{realm}`\n- Productizable: `{}`\n- Required exports: `core/state/{crate_name}.json`\n",
            if productizable { "true" } else { "false" }
        ),
    )?;
    write_file(
        &crate_root.join("tests/contract_smoke.rs"),
        format!(
            "use {}::contract::contract;\n\n#[test]\nfn sovereign_baseline_contract_is_present() {{\n    let base = contract();\n    assert_eq!(base.state_export_path, \"core/state/{crate_name}.json\");\n}}\n",
            crate_name.replace('-', "_")
        ),
    )?;
    Ok(json!({
        "schema_version": "arda.crate-spawn-receipt.v1",
        "generated_at_utc": now_utc(),
        "crate_name": crate_name,
        "realm": realm,
        "productizable": productizable,
        "output_root": crate_root.display().to_string(),
        "files": [
            crate_root.join("Cargo.toml").display().to_string(),
            crate_root.join("src/lib.rs").display().to_string(),
            crate_root.join("src/contract.rs").display().to_string(),
            crate_root.join("src/service.rs").display().to_string(),
            crate_root.join("README.md").display().to_string(),
            crate_root.join("tests/contract_smoke.rs").display().to_string(),
        ],
    }))
}

fn stamp_soterion_sigils(apply: bool) -> anyhow::Result<Value> {
    let root = workspace_root();
    let roots = [
        "core", "docs", "crates", "scripts", "config", "tests", "apps",
    ];
    let skip_dirs = [
        ".git",
        "target",
        "node_modules",
        "dist",
        "build",
        ".next",
        "out",
        "data",
    ];
    let code_ext = [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".c", ".h", ".cpp", ".cc", ".go", ".java", ".kt",
        ".swift",
    ];
    let hash_ext = [
        ".sh", ".rb", ".pl", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf", ".env", ".txt",
        ".md",
    ];
    let json_ext = [".json"];
    let skip_ext = [
        ".jsonl", ".png", ".ico", ".icns", ".zip", ".tar", ".gz", ".jpg", ".jpeg", ".webp",
    ];
    let mut changed = Vec::new();
    let mut scanned = 0u64;
    let mut skipped = 0u64;
    for root_name in roots {
        let root_path = root.join(root_name);
        if !root_path.exists() {
            continue;
        }
        let mut stack = vec![root_path];
        while let Some(current) = stack.pop() {
            for entry in fs::read_dir(&current)?.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.components().any(|part| {
                        skip_dirs.contains(&part.as_os_str().to_str().unwrap_or_default())
                    }) {
                        continue;
                    }
                    stack.push(path);
                    continue;
                }
                let rel = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                if rel.contains("core/metrics/history/")
                    || rel.contains("core/metrics/by_crate/")
                    || rel.ends_with("core/metrics/audit_latest.json")
                {
                    skipped += 1;
                    continue;
                }
                let suffix = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| format!(".{value}"))
                    .unwrap_or_default();
                if skip_ext.contains(&suffix.as_str()) {
                    skipped += 1;
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else {
                    skipped += 1;
                    continue;
                };
                scanned += 1;
                if text
                    .lines()
                    .take(120)
                    .any(|line| line.to_ascii_lowercase().contains("sigil:"))
                {
                    continue;
                }
                let sigil = if rel.contains("/hades/") || rel.contains("destructive") {
                    "COIN"
                } else if rel.starts_with("docs/") || rel.starts_with("config/") {
                    "SCROLL"
                } else if rel.starts_with("scripts/") {
                    "ANKH"
                } else {
                    "REPAIR"
                };
                let updated = if json_ext.contains(&suffix.as_str()) {
                    let Ok(mut payload) = serde_json::from_str::<Value>(&text) else {
                        continue;
                    };
                    let Some(object) = payload.as_object_mut() else {
                        continue;
                    };
                    object.insert("sigil".to_string(), Value::String(sigil.to_string()));
                    serde_json::to_string_pretty(&payload)? + "\n"
                } else if code_ext.contains(&suffix.as_str()) {
                    format!("// sigil: {sigil}\n{text}")
                } else if hash_ext.contains(&suffix.as_str())
                    || matches!(
                        path.file_name().and_then(|value| value.to_str()),
                        Some(".env" | ".env.example" | ".env.generated")
                    )
                {
                    if text.starts_with("#!") {
                        let nl = text.find('\n').unwrap_or(text.len());
                        let tail = if nl < text.len() { &text[nl + 1..] } else { "" };
                        format!(
                            "{}# sigil: {sigil}\n{tail}",
                            &text[..=nl.min(text.len() - 1)]
                        )
                    } else {
                        format!("# sigil: {sigil}\n{text}")
                    }
                } else {
                    continue;
                };
                changed.push(rel);
                if apply {
                    fs::write(&path, updated)?;
                }
            }
        }
    }
    Ok(json!({
        "apply": apply,
        "scanned_text_files": scanned,
        "changed_files": changed.len(),
        "skipped_files": skipped,
        "sample_changed": changed.into_iter().take(50).collect::<Vec<_>>(),
    }))
}

fn parse_objects(line: &str) -> (Vec<Value>, u64) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return (Vec::new(), 0);
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return if value.is_object() {
            (vec![value], 0)
        } else {
            (Vec::new(), 0)
        };
    }
    let mut out = Vec::new();
    let mut bad_fragments = 0u64;
    let stream = serde_json::Deserializer::from_str(trimmed).into_iter::<Value>();
    for item in stream {
        match item {
            Ok(value) if value.is_object() => out.push(value),
            Ok(_) => bad_fragments += 1,
            Err(_) => {
                bad_fragments += 1;
                break;
            }
        }
    }
    (out, bad_fragments)
}

fn keep_record(record: &Value, role: &str) -> bool {
    if role == "action_queue"
        && record.get("action").and_then(Value::as_str) == Some("investigate_orphan")
    {
        return record
            .get("file")
            .and_then(Value::as_str)
            .map(Path::new)
            .map(Path::exists)
            .unwrap_or(false);
    }
    true
}

fn repair_hades_stores(apply: bool, report_path: &str) -> anyhow::Result<Value> {
    let root = workspace_root();
    let plan = [
        (
            "data/hades/hades_log.jsonl",
            "hades_log",
            std::env::var("ARDA_HADES_LOG_MAX_KEEP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100_000usize),
        ),
        (
            "data/hades/warden_queue.jsonl",
            "warden_queue",
            std::env::var("ARDA_HADES_WARDEN_QUEUE_MAX_KEEP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(250_000usize),
        ),
        (
            "data/hades/action_queue.jsonl",
            "action_queue",
            std::env::var("ARDA_HADES_QUEUE_MAX_KEEP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(200_000usize),
        ),
    ];
    let mut stores = Vec::new();
    for (rel, role, max_keep) in plan {
        let path = root.join(rel);
        let mut kept = VecDeque::with_capacity(max_keep);
        let mut total_lines = 0u64;
        let mut valid_objects = 0u64;
        let mut dropped_bad_fragments = 0u64;
        let mut dropped_filters = 0u64;
        let mut dropped_by_max_keep = 0u64;
        if path.exists() {
            for raw in fs::read_to_string(&path)?.lines() {
                total_lines += 1;
                let (objects, bad) = parse_objects(raw);
                dropped_bad_fragments += bad;
                for record in objects {
                    valid_objects += 1;
                    if !keep_record(&record, role) {
                        dropped_filters += 1;
                        continue;
                    }
                    if kept.len() >= max_keep {
                        kept.pop_front();
                        dropped_by_max_keep += 1;
                    }
                    kept.push_back(record);
                }
            }
            if apply {
                let mut content = String::new();
                for row in &kept {
                    content.push_str(&serde_json::to_string(row)?);
                    content.push('\n');
                }
                fs::write(&path, content)?;
            }
        }
        stores.push(json!({
            "path": path.display().to_string(),
            "role": role,
            "exists": path.exists(),
            "total_lines": total_lines,
            "valid_objects": valid_objects,
            "kept_objects": kept.len(),
            "dropped_bad_fragments": dropped_bad_fragments,
            "dropped_non_object": 0,
            "dropped_filters": dropped_filters,
            "dropped_by_max_keep": dropped_by_max_keep,
        }));
    }
    let report = json!({
        "ts_utc": now_utc(),
        "apply": apply,
        "stores": stores,
    });
    let report_path = root.join(report_path);
    write_json(&report_path, &report)?;
    Ok(json!({"report": report_path.display().to_string(), "apply": apply}))
}

fn candidate_cli_bins() -> Vec<PathBuf> {
    let build_root = PathBuf::from(
        std::env::var("ARDA_BUILD_CACHE_ROOT")
            .or_else(|_| std::env::var("ARDA_RUNTIME_BUILD_ROOT"))
            .unwrap_or_else(|_| "/tmp/arda-build".to_string()),
    );
    vec![
        build_root.join("target/debug/arda-cli"),
        workspace_root().join("target/debug/arda-cli"),
    ]
}

fn emit_memory_checkpoint(task: &Value) {
    let status = task
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("queued");
    let title = task
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let owner = task
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or("prometheus")
        .trim();
    let notes = task
        .get("notes")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let mut content = format!("Task ledger update: {title}");
    if !notes.is_empty() {
        content.push_str(". ");
        content.push_str(notes);
    }
    let event_type = match status {
        "completed" => "task_completed",
        "queued" | "in_progress" => "task_delegated",
        "blocked" => "task_failed",
        "deferred" => "interruption_captured",
        _ => "decision_completed",
    };
    let mut cmd = candidate_cli_bins()
        .into_iter()
        .find(|candidate| candidate.exists())
        .map(|candidate| {
            let mut command = Command::new(candidate);
            command.arg("mnemosyne").arg("encode").arg(&content).args([
                "--event-type",
                event_type,
                "--informant-id",
                "task_pivot",
                "--crate-name",
                "prometheus",
                "--confidence",
                "0.92",
                "--tag",
                "task_pivot",
                "--tag",
                "checkpoint",
                "--tag",
                owner,
                "--tag",
                status,
            ]);
            command
        })
        .unwrap_or_else(|| {
            let mut command = Command::new("cargo");
            command
                .arg("run")
                .arg("--quiet")
                .arg("--")
                .arg("mnemosyne")
                .arg("encode")
                .arg(&content)
                .args([
                    "--event-type",
                    event_type,
                    "--informant-id",
                    "task_pivot",
                    "--crate-name",
                    "prometheus",
                    "--confidence",
                    "0.92",
                    "--tag",
                    "task_pivot",
                    "--tag",
                    "checkpoint",
                    "--tag",
                    owner,
                    "--tag",
                    status,
                ]);
            command.current_dir(workspace_root());
            command
        });
    let _ = cmd.stdout(Stdio::null()).stderr(Stdio::null()).status();
}

#[allow(clippy::too_many_arguments)]
fn task_pivot(
    title: &str,
    id: Option<&str>,
    owner: &str,
    priority: &str,
    status: &str,
    result: Option<&str>,
    notes: Option<&str>,
    origin: Option<&str>,
    scope: Option<&str>,
    meta: &[String],
    glyph: &[String],
    sigil: Option<&str>,
    queued_at_utc: Option<&str>,
    completed_at_utc: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let slug_regex = Regex::new(r"[^a-z0-9]+")?;
    let ts = now_utc();
    let slug = slug_regex
        .replace_all(&title.to_ascii_lowercase(), "_")
        .trim_matches('_')
        .chars()
        .take(48)
        .collect::<String>();
    let task_id = id.map(ToOwned::to_owned).unwrap_or_else(|| {
        format!(
            "tsk_{}_{}",
            &ts[..10].replace('-', ""),
            if slug.is_empty() {
                "task".to_string()
            } else {
                slug
            }
        )
    });
    let terminal_status = matches!(status, "completed" | "cancelled" | "blocked");
    if queue_path.exists() {
        for line in fs::read_to_string(&queue_path)?.lines() {
            let id_exists = serde_json::from_str::<Value>(line).ok().and_then(|value| {
                value
                    .get("id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }) == Some(task_id.clone());
            if id_exists && !terminal_status {
                anyhow::bail!("task id already exists: {task_id}");
            }
        }
    }
    let glyph_aliases = std::collections::HashMap::from([
        ("SOVEREIGN", "∇"),
        ("JOULEWORK", "⚡"),
        ("LOVE", "♥"),
        ("VERIFY", "◈"),
        ("TRANSITION", "↝"),
    ]);
    let resolved_glyphs = glyph
        .iter()
        .filter_map(|item| {
            let token = item.trim();
            if token.is_empty() {
                None
            } else {
                Some(
                    glyph_aliases
                        .get(&token.to_ascii_uppercase().as_str())
                        .copied()
                        .unwrap_or(token)
                        .to_string(),
                )
            }
        })
        .collect::<Vec<_>>();
    let mut meta_map = serde_json::Map::new();
    for item in meta {
        let Some((key, value)) = item.split_once('=') else {
            anyhow::bail!("invalid --meta value '{item}'; expected key=value");
        };
        let key = key.trim();
        if key.is_empty() {
            anyhow::bail!("invalid --meta value '{item}'; empty key");
        }
        meta_map.insert(key.to_string(), Value::String(value.trim().to_string()));
    }
    if let Some(origin) = origin {
        meta_map.insert("origin".to_string(), Value::String(origin.to_string()));
    }
    if let Some(scope) = scope {
        meta_map.insert("scope".to_string(), Value::String(scope.to_string()));
    }
    let mut task = serde_json::Map::new();
    task.insert("id".to_string(), Value::String(task_id));
    task.insert("title".to_string(), Value::String(title.to_string()));
    task.insert("owner".to_string(), Value::String(owner.to_string()));
    task.insert("priority".to_string(), Value::String(priority.to_string()));
    task.insert("status".to_string(), Value::String(status.to_string()));
    task.insert(
        "queued_at_utc".to_string(),
        Value::String(queued_at_utc.unwrap_or(&ts).to_string()),
    );
    if let Some(notes) = notes {
        task.insert("notes".to_string(), Value::String(notes.to_string()));
    }
    if let Some(result) = result {
        task.insert("result".to_string(), Value::String(result.to_string()));
    }
    if status == "completed" {
        task.insert(
            "completed_at_utc".to_string(),
            Value::String(completed_at_utc.unwrap_or(&ts).to_string()),
        );
    }
    if !resolved_glyphs.is_empty() {
        task.insert(
            "glyphs".to_string(),
            Value::Array(resolved_glyphs.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(sigil) = sigil {
        task.insert("sigil".to_string(), Value::String(sigil.to_string()));
    }
    if !meta_map.is_empty() {
        task.insert("meta".to_string(), Value::Object(meta_map));
    }
    let value = Value::Object(task);
    if dry_run {
        return Ok(value);
    }
    if let Some(parent) = queue_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut handle = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&queue_path)?;
    handle.write_all(serde_json::to_string(&value)?.as_bytes())?;
    handle.write_all(b"\n")?;
    emit_memory_checkpoint(&value);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        append_jsonl, bridge_model_arg, bridge_provider_arg, bridge_toolsets_arg,
        build_agent_conversation_record, build_bridge_remote_command,
        build_hermes_agent_gateway_activation_check, build_hermes_agent_gateway_receipt,
        build_remote_confidence_snapshot, build_scout_finding_record, build_scout_request_record,
        build_scout_runtime_projection_with_record, hermes_gateway_status_is_running,
        merge_bridge_node, professionalization_audit_closeout, publish_remote_confidence_snapshot,
        run_bridge_ssh, write_agent_conversation_ledger_and_projection, write_json,
        write_remote_confidence_producer_proof, write_safe_local_work_cycle_preflight,
    };
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_audit_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("arda-{name}-{nanos}"))
    }

    fn write_jsonl(path: &Path, rows: &[serde_json::Value]) {
        let content = rows
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .expect("serialize jsonl")
            .join("\n");
        fs::write(path, format!("{content}\n")).expect("write jsonl");
    }

    fn snapshot_tree(path: &Path) -> BTreeSet<(PathBuf, u64)> {
        let mut files = BTreeSet::new();
        let mut stack = vec![path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).expect("read test directory") {
                let entry = entry.expect("read test directory entry");
                let entry_path = entry.path();
                let metadata = entry.metadata().expect("read test metadata");
                if metadata.is_dir() {
                    stack.push(entry_path);
                } else {
                    let relative = entry_path
                        .strip_prefix(path)
                        .expect("strip test directory prefix")
                        .to_path_buf();
                    files.insert((relative, metadata.len()));
                }
            }
        }
        files
    }

    fn seed_audit_closeout_fixture() -> PathBuf {
        let dir = unique_test_audit_dir("audit-closeout");
        fs::create_dir_all(&dir).expect("create audit fixture");
        write_jsonl(
            &dir.join("findings.jsonl"),
            &[json!({"id":"F1","status":"resolved","title":"resolved finding"})],
        );
        write_jsonl(
            &dir.join("remediation-backlog.jsonl"),
            &[json!({"id":"R1","status":"completed","title":"completed remediation"})],
        );
        write_jsonl(
            &dir.join("phase8-hardening-backlog.jsonl"),
            &[
                json!({"id":"P8-HARD-001","status":"completed","title":"validator","owner":"hades","scope":"audit"}),
                json!({"id":"P8-HARD-004","status":"queued","title":"Add packaged audit closeout command","owner":"arda-cli","scope":"arda-cli audit/status surface"}),
                json!({"id":"P8-HARD-005","status":"in_progress","title":"project pending","owner":"prometheus","scope":"audit"}),
            ],
        );
        fs::write(dir.join("PHASE7_CLOSEOUT_PACKET.md"), "phase 7 closed\n")
            .expect("write closeout packet");
        fs::write(dir.join("phase7-validation.txt"), "phase 7 validation\n")
            .expect("write phase7 validation");
        fs::write(dir.join("phase8-validation.txt"), "phase 8 validation\n")
            .expect("write phase8 validation");
        dir
    }

    #[test]
    fn professionalization_audit_closeout_reports_closeout_and_hardening_state() {
        let dir = seed_audit_closeout_fixture();
        let summary = professionalization_audit_closeout(&dir).expect("summarize audit closeout");

        assert_eq!(
            summary["schema_version"],
            "arda.professionalization-audit-closeout.v1"
        );
        assert_eq!(summary["mode"], "read_only_summary");
        assert_eq!(summary["phase7"]["closeout_status"], "closed_with_packet");
        assert_eq!(summary["phase7"]["findings"]["unresolved_count"], 0);
        assert_eq!(summary["phase8"]["status"], "hardening_active");
        assert_eq!(summary["phase8"]["completed_count"], 1);
        assert_eq!(summary["phase8"]["queued_count"], 1);
        assert_eq!(summary["phase8"]["unresolved_count"], 2);
        assert_eq!(summary["next_action"]["id"], "P8-HARD-004");
        assert_eq!(
            summary["evidence_boundary"]["kind"],
            "audit_ledger_summary_not_live_runtime_status"
        );

        fs::remove_dir_all(dir).expect("remove audit fixture");
    }

    #[test]
    fn hermes_agent_gateway_receipt_keeps_arda_policy_boundary() {
        let receipt = build_hermes_agent_gateway_receipt(
            "tsk_gateway_probe",
            Some("bg_123"),
            "discord",
            "work-stream",
            "completed",
            "background task finished",
            &["cargo test -p arda-hermes".to_string()],
            &["crates/arda-hermes/src/serenity_bot.rs".to_string()],
            &[],
            None,
        );

        assert_eq!(
            receipt["schema_version"],
            "arda.hermes_agent_gateway_background_result.v1"
        );
        assert_eq!(receipt["task_ref"], "task:tsk_gateway_probe");
        assert_eq!(receipt["source"], "hermes_agent_gateway");
        assert_eq!(receipt["platform"], "discord");
        assert_eq!(receipt["semantic_channel"], "work-stream");
        assert_eq!(receipt["background_task_id"], "bg_123");
        assert_eq!(receipt["policy_boundary"]["arda_records_authority"], true);
        assert_eq!(
            receipt["policy_boundary"]["gateway_result_is_not_approval"],
            true
        );
    }

    #[test]
    fn hermes_agent_gateway_activation_check_reports_human_gated_live_state() {
        let dir = unique_test_audit_dir("gateway-activation-check");
        fs::create_dir_all(dir.join("docs/plans")).expect("create plan dir");
        fs::create_dir_all(dir.join("docs/operations")).expect("create operations dir");
        fs::create_dir_all(dir.join("config")).expect("create config dir");
        fs::create_dir_all(dir.join("crates/arda-hermes/src/service"))
            .expect("create semantic channel dir");
        fs::create_dir_all(dir.join("crates/arda-hermes/src")).expect("create hermes src dir");
        fs::write(
            dir.join("docs/plans/2026-05-30-hermes-discord-gateway-unification-plan.md"),
            "plan\n",
        )
        .expect("write plan");
        fs::write(
            dir.join("docs/operations/hermes-agent-discord-gateway-runbook.md"),
            "runbook\n",
        )
        .expect("write runbook");
        fs::write(
            dir.join("config/hermes_agent_gateway_arda.example.yaml"),
            "allow_from:\nallow_admin_from:\nbackground_process_notifications: result\nhermes-agent-gateway-receipt\n",
        )
        .expect("write template");
        fs::write(
            dir.join("crates/arda-hermes/src/service/semantic_channel.rs"),
            r#""work-stream" "workstream" "work" "tasks""#,
        )
        .expect("write semantic channel source");
        fs::write(
            dir.join("crates/arda-hermes/src/serenity_bot.rs"),
            r#"CreateCommand::new("plans") CreateCommand::new("tasks") CreateCommand::new("task") CreateCommand::new("review") CreateCommand::new("continue") CreateCommand::new("council") CreateCommand::new("gateway") "activation_check" "remote_confidence" "record_receipt" arda.hermes_agent_gateway_background_result.v1 gateway_result_is_not_approval arda.hermes.work_stream_continuation_request.v1 arda.hermes.discord_operating_room_interaction.v1"#,
        )
        .expect("write serenity source");
        fs::write(
            dir.join("config/.env"),
            "DISCORD_CHANNEL_TASKS=1472529224911945770\nDISCORD_ALLOWED_USERS=442042210536521752\nDISCORD_ALLOW_ALL_USERS=false\n",
        )
        .expect("write env");
        let env = BTreeMap::from([
            (
                "ARDA_DISCORD_WORK_STREAM_CHANNEL_ID".to_string(),
                "missing".to_string(),
            ),
            (
                "ARDA_OPERATOR_DISCORD_USER_ID".to_string(),
                "missing".to_string(),
            ),
            ("DISCORD_ALLOW_ALL_USERS".to_string(), "false".to_string()),
            ("GATEWAY_ALLOW_ALL_USERS".to_string(), "false".to_string()),
        ]);
        let check = build_hermes_agent_gateway_activation_check(
            &dir,
            json!({"ok": true, "stdout": "/tmp/hermes", "stderr": ""}),
            json!({"ok": true, "stdout": "User gateway service is stopped", "stderr": ""}),
            env,
        );

        assert_eq!(
            check["schema_version"],
            "arda.hermes_agent_gateway_activation_check.v1"
        );
        assert_eq!(check["mode"], "read_only");
        assert_eq!(check["status"], "safe_local_ready_live_human_gates_pending");
        assert_eq!(check["safe_local_ready"], true);
        assert_eq!(check["live_ready"], false);
        assert_eq!(
            check["arda_hermes"]["receipt_adapter"]["default_mode"],
            "dry_run"
        );
        assert_eq!(
            check["arda_hermes"]["discord_gateway_surface"]["ready"],
            true
        );
        assert_eq!(
            check["arda_hermes"]["operating_room_surface"]["ready"],
            true
        );
        assert_eq!(
            check["live_mapping_candidates"]["work_stream_channel"]["source"],
            "config/.env"
        );
        assert_eq!(
            check["live_mapping_candidates"]["operator_user"]["source"],
            "config/.env"
        );
        assert!(check["blockers"]
            .as_array()
            .expect("blockers array")
            .iter()
            .any(|value| value
                .as_str()
                .unwrap_or_default()
                .contains("Hermes Agent gateway is not currently running")));

        fs::remove_dir_all(dir).expect("remove gateway activation fixture");
    }

    #[test]
    fn hermes_gateway_status_parser_does_not_treat_inactive_as_running() {
        assert!(!hermes_gateway_status_is_running(
            "Active: inactive (dead)\nUser gateway service is stopped\nSystemd linger is enabled"
        ));
        assert!(hermes_gateway_status_is_running(
            "Active: active (running) since Sat 2026-05-30"
        ));
    }

    #[test]
    fn remote_confidence_snapshot_is_read_only_and_summarizes_attention_gates() {
        let dir = unique_test_audit_dir("remote-confidence");
        fs::create_dir_all(dir.join("core/projects/tasks")).expect("create tasks dir");
        fs::create_dir_all(dir.join("data/hermes")).expect("create hermes dir");
        fs::create_dir_all(dir.join("data/flywheel")).expect("create flywheel dir");
        fs::create_dir_all(dir.join("data/prometheus")).expect("create prometheus dir");
        fs::create_dir_all(dir.join("core/control/autonomy")).expect("create autonomy dir");
        write_jsonl(
            &dir.join("core/projects/tasks/queue.jsonl"),
            &[
                json!({"id":"tsk_open","title":"Open safe-local work","status":"queued","priority":"high","owner":"prometheus"}),
                json!({"id":"tsk_gate","title":"Needs human approval","status":"human_gated","priority":"critical","owner":"warden"}),
                json!({"id":"tsk_done","title":"Done","status":"completed","priority":"normal","owner":"hermes"}),
                json!({"id":"tsk_superseded","title":"Old pending row","status":"pending","priority":"normal","owner":"hades"}),
                json!({"id":"tsk_superseded","title":"Old pending row","status":"completed","priority":"normal","owner":"hades"}),
            ],
        );
        write_jsonl(
            &dir.join("data/hermes/hermes_agent_gateway_receipts.jsonl"),
            &[
                json!({"receipt_id":"hag_1","status":"completed","summary":"Gateway receipt complete"}),
            ],
        );
        write_jsonl(
            &dir.join("data/prometheus/council_decisions.jsonl"),
            &[
                json!({"decision_id":"council_1","status":"approved","summary":"Proceed with safe-local work"}),
            ],
        );
        fs::write(
            dir.join("data/flywheel/latest_packet.json"),
            serde_json::to_string_pretty(
                &json!({"packet_id":"flywheel_1","status":"ready","eligible_task_count":3}),
            )
            .expect("serialize packet"),
        )
        .expect("write flywheel packet");
        fs::write(
            dir.join("core/control/autonomy/state.json"),
            serde_json::to_string_pretty(
                &json!({"mode":"safe_local","holds":["requires_human_for_external_side_effects"]}),
            )
            .expect("serialize autonomy state"),
        )
        .expect("write autonomy state");
        let before = snapshot_tree(&dir);

        let snapshot = build_remote_confidence_snapshot(
            &dir,
            json!({"ok": true, "stdout": "/tmp/hermes", "stderr": ""}),
            json!({"ok": true, "stdout": "Active: active (running) since Sat 2026-05-30", "stderr": ""}),
            BTreeMap::new(),
        );
        let after = snapshot_tree(&dir);

        assert_eq!(before, after);
        assert_eq!(
            snapshot["schema_version"],
            "arda.remote_confidence_snapshot.v1"
        );
        assert_eq!(snapshot["mode"], "read_only");
        assert_eq!(snapshot["overall_status"], "attention_required");
        assert_eq!(snapshot["gateway"]["running"], true);
        assert_eq!(snapshot["discord"]["role"], "remote_confidence_surface");
        assert_eq!(snapshot["primary_consoles"][0], "ARDA HUD");
        let expected_arda_target = dir
            .join("core/state/remote_confidence_snapshot.json")
            .to_string_lossy()
            .to_string();
        assert_eq!(
            snapshot["arda_hud"]["target_state_path"].as_str(),
            Some(expected_arda_target.as_str())
        );
        assert_eq!(snapshot["autonomy"]["mode"], "safe_local");
        assert_eq!(snapshot["tasks"]["open_count"], 1);
        assert_eq!(snapshot["tasks"]["human_gated_count"], 1);
        assert_eq!(snapshot["human_required_gates"][0]["id"], "tsk_gate");
        assert_eq!(
            snapshot["flywheel"]["latest_packet"]["packet_id"],
            "flywheel_1"
        );
        assert_eq!(
            snapshot["last_council_decision"]["decision_id"],
            "council_1"
        );
        assert_eq!(
            snapshot["last_completion_receipts"][0]["receipt_id"],
            "hag_1"
        );

        fs::remove_dir_all(dir).expect("remove remote confidence fixture");
    }

    #[test]
    fn remote_confidence_publisher_writes_only_local_arda_state_snapshot() {
        let dir = unique_test_audit_dir("remote-confidence-publisher");
        fs::create_dir_all(dir.join("core/projects/tasks")).expect("create tasks dir");
        fs::create_dir_all(dir.join("core/state")).expect("create state dir");
        fs::create_dir_all(dir.join("data/hermes")).expect("create hermes dir");
        write_jsonl(
            &dir.join("core/projects/tasks/queue.jsonl"),
            &[
                json!({"id":"tsk_safe_local","title":"Safe local work","status":"queued","priority":"high","owner":"prometheus"}),
            ],
        );
        fs::write(
            dir.join("core/state/existing_runtime_state.json"),
            "{\"untouched\":true}\n",
        )
        .expect("write unrelated state fixture");
        let before = snapshot_tree(&dir);
        let target = dir.join("core/state/remote_confidence_snapshot.json");

        let report = publish_remote_confidence_snapshot(
            &dir,
            json!({"ok": true, "stdout": "/tmp/hermes", "stderr": ""}),
            json!({"ok": true, "stdout": "Active: active (running) since Sat 2026-05-30", "stderr": ""}),
            BTreeMap::new(),
        )
        .expect("publish local remote confidence snapshot");
        let after = snapshot_tree(&dir);
        let added_or_changed = after.difference(&before).cloned().collect::<BTreeSet<_>>();

        assert_eq!(
            added_or_changed,
            BTreeSet::from([(
                PathBuf::from("core/state/remote_confidence_snapshot.json"),
                fs::metadata(&target).expect("target metadata").len()
            )])
        );
        assert_eq!(
            report["schema_version"],
            "arda.remote_confidence_publisher.v1"
        );
        assert_eq!(report["writes"]["generated_state"], true);
        assert_eq!(
            report["writes"]["target_state_path"].as_str(),
            Some(target.to_string_lossy().as_ref())
        );
        assert_eq!(
            report["side_effect_policy"]["external_messages_sent"],
            false
        );
        assert_eq!(report["side_effect_policy"]["service_restart"], false);
        assert_eq!(report["side_effect_policy"]["credential_change"], false);
        let published: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&target).expect("read published snapshot"))
                .expect("parse published snapshot");
        assert_eq!(
            published["schema_version"],
            "arda.remote_confidence_snapshot.v1"
        );
        assert_eq!(published["mode"], "local_runtime_published");
        assert_eq!(
            published["side_effect_policy"]["external_messages_sent"],
            false
        );
        assert_eq!(
            published["side_effect_policy"]["writes_generated_state"],
            true
        );
        assert_eq!(
            published["arda_hud"]["projection_mode"],
            "local_runtime_state_file"
        );

        fs::remove_dir_all(dir).expect("remove remote confidence publisher fixture");
    }

    #[test]
    fn safe_local_work_cycle_preflight_writes_only_report_and_does_not_mutate_queue() {
        let dir = unique_test_audit_dir("safe-local-work-cycle-preflight");
        fs::create_dir_all(dir.join("core/projects/tasks")).expect("create tasks dir");
        fs::create_dir_all(dir.join("data/prometheus")).expect("create prometheus dir");
        fs::create_dir_all(dir.join("core/state")).expect("create state dir");
        write_jsonl(
            &dir.join("core/projects/tasks/queue.jsonl"),
            &[
                json!({"id":"tsk_safe","title":"Safe local docs and tests packet","status":"queued","priority":"high","owner":"prometheus","meta":{"action_class":"safe_local_read_write"}}),
                json!({"id":"tsk_discord","title":"Run live Discord validation","status":"queued","priority":"high","owner":"hermes","meta":{"action_class":"external_message"}}),
                json!({"id":"tsk_restart","title":"Restart Manwe service","status":"queued","priority":"critical","owner":"manwe","meta":{"action_class":"service_runtime_mutation"}}),
                json!({"id":"tsk_done","title":"Completed packet","status":"completed","priority":"normal","owner":"hades","meta":{"action_class":"safe_local_read_write"}}),
                json!({"id":"tsk_superseded","title":"Superseded pending packet","status":"pending","priority":"normal","owner":"hades","meta":{"action_class":"safe_local_read_write"}}),
                json!({"id":"tsk_superseded","title":"Superseded pending packet","status":"completed","priority":"normal","owner":"hades","meta":{"action_class":"safe_local_read_write"}}),
            ],
        );
        fs::write(
            dir.join("core/state/remote_confidence_snapshot.json"),
            "{\"schema_version\":\"arda.remote_confidence_snapshot.v1\"}\n",
        )
        .expect("write remote confidence fixture");
        let queue_before = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue before");
        let before = snapshot_tree(&dir);
        let report_path = dir.join("data/prometheus/safe_local_work_cycle_preflight.json");

        let receipt = write_safe_local_work_cycle_preflight(&dir).expect("write preflight report");
        let queue_after = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue after");
        let after = snapshot_tree(&dir);
        let added_or_changed = after.difference(&before).cloned().collect::<BTreeSet<_>>();

        assert_eq!(queue_before, queue_after);
        assert_eq!(
            added_or_changed,
            BTreeSet::from([(
                PathBuf::from("data/prometheus/safe_local_work_cycle_preflight.json"),
                fs::metadata(&report_path).expect("report metadata").len()
            )])
        );
        assert_eq!(
            receipt["schema_version"],
            "arda.safe_local_work_cycle_preflight.v1"
        );
        assert_eq!(receipt["status"], "report_written");
        assert_eq!(receipt["side_effect_policy"]["mutates_task_status"], false);
        assert_eq!(
            receipt["side_effect_policy"]["external_messages_sent"],
            false
        );
        assert_eq!(receipt["side_effect_policy"]["service_restart"], false);
        assert_eq!(receipt["side_effect_policy"]["credential_change"], false);
        assert_eq!(
            receipt["side_effect_policy"]["destructive_operations"],
            false
        );

        let report: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&report_path).expect("read preflight report"))
                .expect("parse preflight report");
        assert_eq!(
            report["schema_version"],
            "arda.safe_local_work_cycle_preflight.v1"
        );
        assert_eq!(report["mode"], "safe_local_preflight_report");
        assert_eq!(report["side_effect_policy"]["writes_local_report"], true);
        assert_eq!(report["side_effect_policy"]["mutates_task_status"], false);
        assert_eq!(
            report["side_effect_policy"]["external_messages_sent"],
            false
        );
        assert_eq!(
            report["side_effect_policy"]["live_discord_validation"],
            "human_gated_separate"
        );
        assert_eq!(report["candidate_summary"]["total_count"], 3);
        assert_eq!(report["candidate_summary"]["safe_local_count"], 1);
        assert_eq!(report["candidate_summary"]["human_gated_count"], 2);
        assert_eq!(report["arda_hud"]["projection_mode"], "local_report_file");
        assert_eq!(report["arda_hud"]["new_rail_required"], false);

        let candidates = report["candidates"].as_array().expect("candidate array");
        let safe = candidates
            .iter()
            .find(|row| row["id"] == "tsk_safe")
            .expect("safe candidate");
        assert_eq!(safe["action_class"], "safe_local_read_write");
        assert_eq!(safe["safe_local_eligible"], true);
        assert_eq!(safe["required_human_gate"], false);
        let discord = candidates
            .iter()
            .find(|row| row["id"] == "tsk_discord")
            .expect("discord candidate");
        assert_eq!(discord["safe_local_eligible"], false);
        assert_eq!(discord["required_human_gate"], true);
        assert_eq!(discord["action_class"], "external_message");
        let restart = candidates
            .iter()
            .find(|row| row["id"] == "tsk_restart")
            .expect("restart candidate");
        assert_eq!(restart["safe_local_eligible"], false);
        assert_eq!(restart["required_human_gate"], true);
        assert_eq!(restart["action_class"], "service_runtime_mutation");

        fs::remove_dir_all(dir).expect("remove safe-local preflight fixture");
    }

    #[test]
    fn conversation_and_scout_ledgers_are_local_only_and_projectable() {
        let dir = unique_test_audit_dir("conversation-scout-ledgers");
        fs::create_dir_all(dir.join("core/projects/tasks")).expect("create tasks dir");
        fs::create_dir_all(dir.join("data/council")).expect("create council dir");
        fs::create_dir_all(dir.join("data/athena")).expect("create athena dir");
        fs::create_dir_all(dir.join("core/state")).expect("create state dir");
        fs::write(
            dir.join("core/projects/tasks/queue.jsonl"),
            "{\"id\":\"existing\",\"status\":\"queued\"}\n",
        )
        .expect("write queue fixture");
        let queue_before = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue before");

        let conversation = build_agent_conversation_record(
            "conv_a3_a4",
            "A3/A4 producer proof",
            "prometheus",
            "planning",
            "proposal",
            "proposal",
            "safe_local",
            "Append local council evidence for ARDA without approving execution.",
            Some("docs/plans/2026-05-30-autonomous-remote-confidence-console-plan.md"),
            Some("tsk_a3_a4_producer"),
            Some("scout_a4_contract"),
            Some("0.88"),
            &["docs/contracts/arda-conversation-scout-ledger-contract.md".to_string()],
            &[],
        )
        .expect("build conversation");
        append_jsonl(
            &dir.join("data/council/agent_conversations.jsonl"),
            &conversation,
        )
        .expect("append conversation");

        let request = build_scout_request_record(
            "scout_a4_contract",
            "prometheus",
            "What local producer commands are needed for ARDA scout projection?",
            "implementation_notes",
            "repo_allowed",
            "safe_local",
            "requested",
            "docs/plans/2026-05-30-autonomous-remote-confidence-console-plan.md",
            Some("tsk_a3_a4_producer"),
            None,
            Some("refresh after producer implementation"),
            Some("local-only test fixture"),
        )
        .expect("build request");
        append_jsonl(&dir.join("data/athena/scout_requests.jsonl"), &request)
            .expect("append request");
        let finding = build_scout_finding_record(
            "finding_a4_contract",
            "scout_a4_contract",
            "athena",
            "A4 producer command path",
            "Utility append commands can feed ARDA's existing command-console readers.",
            "repo_allowed",
            "found",
            "safe_local",
            Some("0.9"),
            &["docs/contracts/arda-conversation-scout-ledger-contract.md".to_string()],
            &["Add CLI append commands and scout runtime refresh".to_string()],
            &[],
        )
        .expect("build finding");
        append_jsonl(&dir.join("data/athena/scout_findings.jsonl"), &finding)
            .expect("append finding");

        let runtime = build_scout_runtime_projection_with_record(&dir, None, None);
        write_json(&dir.join("core/state/scout_runtime.json"), &runtime).expect("write runtime");

        let queue_after = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue after");
        assert_eq!(queue_before, queue_after);
        assert_eq!(
            conversation["schema_version"],
            "arda.council.agent_conversation.v1"
        );
        assert_eq!(conversation["policy_boundary"]["queue_mutated"], false);
        assert_eq!(request["schema_version"], "arda.athena.scout_request.v1");
        assert_eq!(
            request["policy_boundary"]["request_is_not_task_queue_write"],
            true
        );
        assert_eq!(finding["schema_version"], "arda.athena.scout_finding.v1");
        assert_eq!(
            finding["policy_boundary"]["finding_is_not_task_queue_write"],
            true
        );
        assert_eq!(runtime["schema_version"], "arda.athena.scout_runtime.v1");
        assert_eq!(runtime["summary"]["request_count"], 1);
        assert_eq!(runtime["summary"]["open_request_count"], 1);
        assert_eq!(runtime["summary"]["finding_count"], 1);
        assert_eq!(
            runtime["side_effect_policy"]["external_messages_sent"],
            false
        );

        fs::remove_dir_all(dir).expect("remove conversation/scout fixture");
    }

    #[test]
    fn scout_runtime_projection_exposes_conversation_evidence_without_external_side_effects() {
        let dir = unique_test_audit_dir("conversation-runtime-projection");
        fs::create_dir_all(dir.join("core/projects/tasks")).expect("create tasks dir");
        fs::create_dir_all(dir.join("data/council")).expect("create council dir");
        fs::create_dir_all(dir.join("data/athena")).expect("create athena dir");
        fs::create_dir_all(dir.join("core/state")).expect("create state dir");
        fs::write(
            dir.join("core/projects/tasks/queue.jsonl"),
            "{\"id\":\"existing\",\"status\":\"queued\"}\n",
        )
        .expect("write queue fixture");
        let queue_before = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue before");
        let before = snapshot_tree(&dir);
        let runtime_path = dir.join("core/state/scout_runtime.json");

        let report = write_agent_conversation_ledger_and_projection(
            &dir,
            "conv_runtime_surface",
            "Conversation evidence projection",
            "prometheus",
            "planning",
            "observation",
            "informational",
            "safe_local",
            "Council conversation evidence should appear in local runtime projection.",
            Some("docs/plans/conversation-runtime-projection.md"),
            Some("tsk_runtime_projection"),
            Some("scout_runtime_projection"),
            Some("0.82"),
            &["data/council/agent_conversations.jsonl".to_string()],
            &[],
        )
        .expect("write conversation evidence projection");

        let queue_after = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue after");
        let after = snapshot_tree(&dir);
        let added_or_changed = after.difference(&before).cloned().collect::<BTreeSet<_>>();

        assert_eq!(queue_before, queue_after);
        assert_eq!(
            added_or_changed,
            BTreeSet::from([
                (
                    PathBuf::from("core/state/scout_runtime.json"),
                    fs::metadata(&runtime_path).expect("runtime metadata").len()
                ),
                (
                    PathBuf::from("data/council/agent_conversations.jsonl"),
                    fs::metadata(dir.join("data/council/agent_conversations.jsonl"))
                        .expect("conversation ledger metadata")
                        .len()
                )
            ])
        );
        assert_eq!(
            report["schema_version"],
            "arda.council.agent_conversation_projection_write.v1"
        );
        assert_eq!(report["mode"], "local_ledger_projection_write");
        assert_eq!(
            report["record_path"].as_str(),
            Some(
                dir.join("data/council/agent_conversations.jsonl")
                    .to_string_lossy()
                    .as_ref()
            )
        );
        assert_eq!(
            report["runtime_path"].as_str(),
            Some(runtime_path.to_string_lossy().as_ref())
        );
        assert_eq!(report["record"]["conversation_id"], "conv_runtime_surface");
        assert_eq!(
            report["runtime_projection"]["summary"]["conversation_count"],
            1
        );
        assert_eq!(
            report["runtime_projection"]["latest_conversation"]["conversation_id"],
            "conv_runtime_surface"
        );
        assert_eq!(
            report["side_effect_policy"]["external_messages_sent"],
            false
        );
        assert_eq!(report["side_effect_policy"]["service_restart"], false);
        assert_eq!(report["side_effect_policy"]["credential_change"], false);
        assert_eq!(report["side_effect_policy"]["queue_mutated"], false);
        assert_eq!(
            report["side_effect_policy"]["live_gateway_credentials_required"],
            false
        );

        let runtime: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&runtime_path).expect("read runtime projection"),
        )
        .expect("parse runtime projection");
        assert_eq!(
            runtime["latest_conversation"]["conversation_id"],
            "conv_runtime_surface"
        );

        fs::remove_dir_all(dir).expect("remove conversation runtime fixture");
    }

    #[test]
    fn remote_confidence_producer_proof_wires_conversation_and_scout_ledgers() {
        let dir = unique_test_audit_dir("remote-confidence-producer-proof");
        fs::create_dir_all(dir.join("core/projects/tasks")).expect("create tasks dir");
        fs::create_dir_all(dir.join("data/council")).expect("create council dir");
        fs::create_dir_all(dir.join("data/athena")).expect("create athena dir");
        fs::create_dir_all(dir.join("data/prometheus")).expect("create prometheus dir");
        fs::create_dir_all(dir.join("core/state")).expect("create state dir");
        fs::write(
            dir.join("core/projects/tasks/queue.jsonl"),
            "{\"id\":\"existing\",\"status\":\"queued\"}\n",
        )
        .expect("write queue fixture");
        let queue_before = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue before");
        let before = snapshot_tree(&dir);

        let report = write_remote_confidence_producer_proof(&dir)
            .expect("write remote confidence producer proof");

        let queue_after = fs::read_to_string(dir.join("core/projects/tasks/queue.jsonl"))
            .expect("read queue after");
        let after = snapshot_tree(&dir);
        let added_or_changed = after.difference(&before).cloned().collect::<BTreeSet<_>>();

        assert_eq!(queue_before, queue_after);
        assert_eq!(
            added_or_changed,
            BTreeSet::from([
                (
                    PathBuf::from("core/state/scout_runtime.json"),
                    fs::metadata(dir.join("core/state/scout_runtime.json"))
                        .expect("runtime metadata")
                        .len()
                ),
                (
                    PathBuf::from("data/athena/scout_findings.jsonl"),
                    fs::metadata(dir.join("data/athena/scout_findings.jsonl"))
                        .expect("finding metadata")
                        .len()
                ),
                (
                    PathBuf::from("data/athena/scout_requests.jsonl"),
                    fs::metadata(dir.join("data/athena/scout_requests.jsonl"))
                        .expect("request metadata")
                        .len()
                ),
                (
                    PathBuf::from("data/council/agent_conversations.jsonl"),
                    fs::metadata(dir.join("data/council/agent_conversations.jsonl"))
                        .expect("conversation metadata")
                        .len()
                ),
                (
                    PathBuf::from("data/prometheus/remote_confidence_producer_proof.json"),
                    fs::metadata(dir.join("data/prometheus/remote_confidence_producer_proof.json"))
                        .expect("report metadata")
                        .len()
                )
            ])
        );
        assert_eq!(
            report["schema_version"],
            "arda.remote_confidence_producer_proof.v1"
        );
        assert_eq!(report["mode"], "local_runtime_producer_wiring_proof");
        assert_eq!(
            report["records"]["conversation"]["schema_version"],
            "arda.council.agent_conversation.v1"
        );
        assert_eq!(
            report["records"]["scout_request"]["schema_version"],
            "arda.athena.scout_request.v1"
        );
        assert_eq!(report["records"]["scout_request"]["status"], "satisfied");
        assert_eq!(
            report["records"]["scout_finding"]["schema_version"],
            "arda.athena.scout_finding.v1"
        );
        assert_eq!(
            report["runtime_projection"]["summary"]["conversation_count"],
            1
        );
        assert_eq!(report["runtime_projection"]["summary"]["request_count"], 1);
        assert_eq!(report["runtime_projection"]["summary"]["finding_count"], 1);
        assert_eq!(
            report["side_effect_policy"]["external_messages_sent"],
            false
        );
        assert_eq!(report["side_effect_policy"]["queue_mutated"], false);
        assert_eq!(report["side_effect_policy"]["service_restart"], false);
        assert_eq!(report["side_effect_policy"]["credential_change"], false);

        fs::remove_dir_all(dir).expect("remove remote confidence producer fixture");
    }

    #[test]
    fn professionalization_audit_closeout_is_read_only() {
        let dir = seed_audit_closeout_fixture();
        let before = snapshot_tree(&dir);
        let summary = professionalization_audit_closeout(&dir).expect("summarize audit closeout");
        let after = snapshot_tree(&dir);

        assert_eq!(before, after);
        assert_eq!(
            summary["side_effect_policy"]["writes_generated_state"],
            false
        );
        assert_eq!(
            summary["side_effect_policy"]["refreshes_runtime_state"],
            false
        );
        assert_eq!(
            summary["side_effect_policy"]["reads_audit_ledgers_only"],
            true
        );

        fs::remove_dir_all(dir).expect("remove audit fixture");
    }

    #[test]
    fn bridge_toolsets_omit_when_inheriting_upstream_defaults() {
        assert_eq!(bridge_toolsets_arg(""), None);
        assert_eq!(bridge_toolsets_arg("inherit"), None);
        assert_eq!(bridge_toolsets_arg("auto"), None);
        assert_eq!(bridge_toolsets_arg("*"), None);
    }

    #[test]
    fn bridge_toolsets_pass_through_explicit_values_unchanged() {
        assert_eq!(
            bridge_toolsets_arg("search_files,read_file,patch,process,terminal,write_file"),
            Some("search_files,read_file,patch,process,terminal,write_file".to_string())
        );
        assert_eq!(bridge_toolsets_arg("all"), Some("all".to_string()));
    }

    #[test]
    fn bridge_model_degrades_stale_harmonic_hermes_pins_to_auto() {
        assert_eq!(
            bridge_model_arg("Harmonic-Hermes-9B-Q6_K"),
            Some("auto".to_string())
        );
        assert_eq!(
            bridge_model_arg("mesh_local/Harmonic-Hermes-9B-Q6_K"),
            Some("auto".to_string())
        );
    }

    #[test]
    fn bridge_model_preserves_other_explicit_models() {
        assert_eq!(bridge_model_arg("auto"), Some("auto".to_string()));
        assert_eq!(
            bridge_model_arg("openrouter/auto"),
            Some("openrouter/auto".to_string())
        );
        assert_eq!(
            bridge_model_arg("Qwen_Qwen3.5-4B-Q6_K"),
            Some("Qwen_Qwen3.5-4B-Q6_K".to_string())
        );
    }

    #[test]
    fn bridge_provider_normalizes_auto_to_custom_when_base_url_is_present() {
        assert_eq!(
            bridge_provider_arg(Some("auto"), true),
            Some("custom".to_string())
        );
        assert_eq!(
            bridge_provider_arg(Some("inherit"), true),
            Some("custom".to_string())
        );
        assert_eq!(bridge_provider_arg(None, true), Some("custom".to_string()));
    }

    #[test]
    fn bridge_provider_preserves_explicit_non_custom_provider_without_base_url() {
        assert_eq!(
            bridge_provider_arg(Some("openrouter"), false),
            Some("openrouter".to_string())
        );
        assert_eq!(
            bridge_provider_arg(Some("auto"), false),
            Some("auto".to_string())
        );
    }

    #[test]
    fn bridge_python_cli_avoids_auto_provider_when_pinned_to_manwe_base_url() {
        let node = json!({
            "hermes_bin": "/tmp/venv/bin/python",
            "bridge_mode": "python_cli",
            "provider": "auto",
            "base_url": "http://100.78.138.113:5110/v1",
            "remote_cwd": "/tmp/hermes-agent",
            "model": "auto"
        });
        let command = build_bridge_remote_command(&node, "ping");
        assert!(command.contains("--base_url 'http://100.78.138.113:5110/v1'"));
        assert!(!command.contains("--provider 'auto'"));
        assert!(!command.contains("--provider 'custom'"));
    }

    #[test]
    fn bridge_chat_mode_exports_custom_provider_for_manwe_base_url() {
        let node = json!({
            "hermes_bin": "hermes",
            "provider": "auto",
            "base_url": "http://100.78.138.113:5110/v1",
            "remote_cwd": "~",
            "model": "auto"
        });
        let command = build_bridge_remote_command(&node, "ping");
        assert!(command.contains("HERMES_INFERENCE_PROVIDER='custom'"));
        assert!(command.contains("--provider 'custom'"));
        assert!(command.contains("OPENAI_BASE_URL='http://100.78.138.113:5110/v1'"));
    }

    #[test]
    fn bridge_alias_target_id_inherits_current_edge_target_connection_fields() {
        let targets = BTreeMap::from([(
            "node-backbone-server-01".to_string(),
            json!({
                "id": "node-backbone-server-01",
                "hostname": "beelink",
                "tailscale_ip": "100.118.123.88",
                "ssh_user": "ardaserver",
                "llm_runtime": "multi_gpu_sovereign_backbone"
            }),
        )]);
        let bridge = json!({
            "defaults": {"transport": "ssh", "provider": "custom", "base_url": "http://127.0.0.1:5110/v1"},
            "node": {
                "node-backbone-server": {
                    "enabled": true,
                    "target_id": "node-backbone-server-01",
                    "remote_cwd": "/var/home/ardaserver/.hermes/hermes-agent"
                }
            }
        });

        let node = merge_bridge_node("node-backbone-server", &targets, &bridge);

        assert_eq!(node["id"], "node-backbone-server");
        assert_eq!(node["target_id"], "node-backbone-server-01");
        assert_eq!(node["host"], "100.118.123.88");
        assert_eq!(node["ssh_user"], "ardaserver");
        assert_eq!(node["enabled"], true);
    }

    #[test]
    fn bridge_dry_run_command_includes_bounded_ssh_options() {
        let node = json!({
            "id": "node-test",
            "host": "100.64.0.1",
            "ssh_user": "arda",
            "timeout_seconds": 7
        });
        let result = run_bridge_ssh(&node, "printf 'ready\\n'", true).expect("dry-run bridge");
        let command = result["command"].as_array().expect("command array");
        let command_text = command
            .iter()
            .filter_map(|part| part.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(command_text.contains("ConnectTimeout=7"));
        assert!(command_text.contains("BatchMode=yes"));
        assert_eq!(result["dry_run"], true);
        assert_eq!(result["timeout_seconds"], 7);
    }
}
