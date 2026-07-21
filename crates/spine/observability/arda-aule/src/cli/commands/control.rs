#![cfg(feature = "full-cli")]
use super::super::*;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn handle(command: ControlCommands) -> anyhow::Result<()> {
    let value = match command {
        ControlCommands::SyncOperatorProfile {
            profile_path,
            runtime_out,
            env_out,
            apply_runtime,
            apply_env,
            force_env,
            apply_control_policy,
        } => sync_operator_profile(
            &profile_path,
            &runtime_out,
            &env_out,
            apply_runtime,
            apply_env,
            force_env,
            apply_control_policy,
        )?,
        ControlCommands::LaunchPreflight {
            report_path,
            governance_path,
            budget_path,
            pressure_path,
            topology_path,
            degraded_agents,
            enforce_swap,
            enforce_exit,
        } => launch_preflight(
            &report_path,
            &governance_path,
            &budget_path,
            &pressure_path,
            &topology_path,
            &degraded_agents,
            enforce_swap,
            enforce_exit,
        )?,
        ControlCommands::ApplyOpencodeRouteGovernor {
            model_control_path,
            routes_path,
            state_path,
        } => apply_opencode_route_governor(&model_control_path, &routes_path, &state_path)?,
        ControlCommands::ApplyRuntimeRecoveryRouteGovernor {
            recovery_path,
            manwe_router_path,
            route_matrix_path,
            state_path,
        } => apply_runtime_recovery_route_governor(
            &recovery_path,
            &manwe_router_path,
            &route_matrix_path,
            &state_path,
        )?,
        ControlCommands::RunRuntimeAdmissionRecoveryExecutor {
            recovery_path,
            out_path,
            timeout_seconds,
        } => run_runtime_admission_recovery_executor(&recovery_path, &out_path, timeout_seconds)?,
        ControlCommands::SyncOutputAccounting {
            topology_path,
            state_path,
            ledger_path,
            mirror_root,
        } => sync_output_accounting(&topology_path, &state_path, &ledger_path, &mirror_root)?,
        ControlCommands::PruneRuntimeBuildCache { out_path } => {
            prune_runtime_build_cache(&out_path)?
        }
        ControlCommands::OrganizeHadesBackups {
            hades_root,
            out_path,
        } => organize_hades_backups(&hades_root, &out_path)?,
        ControlCommands::ReconcileEscalations {
            escalations_path,
            autonomy_runtime_path,
            pressure_report_path,
        } => reconcile_escalations(
            &escalations_path,
            &autonomy_runtime_path,
            &pressure_report_path,
        )?,
        ControlCommands::ApproveHumanAugmentation {
            decision_class,
            command_signature,
            approvers,
            evidence,
            expires_at_utc,
            note,
            status,
            approvals_path,
            runtime_out,
        } => approve_human_augmentation(
            &decision_class,
            command_signature.as_deref(),
            &approvers,
            &evidence,
            expires_at_utc.as_deref(),
            note.as_deref(),
            &status,
            &approvals_path,
            &runtime_out,
        )?,
        ControlCommands::RecordCeoCouncilSession {
            objective,
            ceo_identity,
            cto_identity,
            cto_mode,
            ingress,
            channel_ref,
            loop_class,
            decision_class,
            triad_required,
            participants,
            proposals,
            objections,
            synthesis,
            outcome_status,
            human_escalated,
            validators_invoked,
            memory_lanes,
            memory_writes,
            promoted_private_memory,
            sessions_path,
            runtime_out,
        } => record_ceo_council_session(
            &objective,
            &ceo_identity,
            &cto_identity,
            &cto_mode,
            &ingress,
            channel_ref.as_deref(),
            &loop_class,
            &decision_class,
            triad_required,
            &participants,
            &proposals,
            &objections,
            synthesis.as_deref(),
            &outcome_status,
            human_escalated,
            &validators_invoked,
            &memory_lanes,
            &memory_writes,
            promoted_private_memory,
            &sessions_path,
            &runtime_out,
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
    Utc::now().to_rfc3339()
}

fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_text(path: &Path, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn default_human_augmentation_approval_state() -> Value {
    json!({
        "schema_version": "arda.human-augmentation-approval.v1",
        "updated_at_utc": now_utc(),
        "approvals": []
    })
}

fn default_ceo_council_state() -> Value {
    json!({
        "schema_version": "arda.ceo-council-sessions.v1",
        "updated_at_utc": now_utc(),
        "sessions": []
    })
}

fn build_ceo_council_runtime_snapshot(
    ruleset: &Value,
    sessions_state: &Value,
    sessions_path: &Path,
    ruleset_path: &Path,
) -> Value {
    let policy = ruleset.get("policy").cloned().unwrap_or_else(|| json!({}));
    let human_augmentation = policy
        .get("human_augmentation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let sessions = sessions_state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total_sessions = sessions.len();
    let triad_sessions = sessions
        .iter()
        .filter(|row| row.get("triad_required").and_then(Value::as_bool) == Some(true))
        .count();
    let lightweight_sessions = sessions
        .iter()
        .filter(|row| row.get("loop_class").and_then(Value::as_str) == Some("lightweight"))
        .count();
    let human_escalations = sessions
        .iter()
        .filter(|row| row.get("human_escalated").and_then(Value::as_bool) == Some(true))
        .count();
    let promoted_private_memory = sessions
        .iter()
        .filter(|row| row.get("promoted_private_memory").and_then(Value::as_bool) == Some(true))
        .count();

    let mut validator_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut lane_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut memory_write_counts = std::collections::BTreeMap::<String, usize>::new();
    for session in &sessions {
        if let Some(validators) = session.get("validators_invoked").and_then(Value::as_array) {
            for validator in validators.iter().filter_map(Value::as_str) {
                *validator_counts.entry(validator.to_string()).or_insert(0) += 1;
            }
        }
        if let Some(lanes) = session.get("memory_lanes").and_then(Value::as_array) {
            for lane in lanes.iter().filter_map(Value::as_str) {
                *lane_counts.entry(lane.to_string()).or_insert(0) += 1;
            }
        }
        if let Some(writes) = session
            .get("memory_write_intents")
            .and_then(Value::as_array)
        {
            for lane in writes
                .iter()
                .filter_map(|row| row.get("lane").and_then(Value::as_str))
            {
                *memory_write_counts.entry(lane.to_string()).or_insert(0) += 1;
            }
        }
    }

    json!({
        "schema_version": "arda.ceo-council-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "control.record-ceo-council-session",
        "policy": {
            "human_augmentation": human_augmentation,
            "memory_lanes": [
                {
                    "id": "human_sovereign",
                    "retention": "durable",
                    "promotion_required": false,
                    "can_authorize_irreversible_action": true
                },
                {
                    "id": "ceo_private_working",
                    "retention": "expiring",
                    "promotion_required": true,
                    "can_authorize_irreversible_action": false
                },
                {
                    "id": "shared_executive",
                    "retention": "durable",
                    "promotion_required": false,
                    "can_authorize_irreversible_action": false
                },
                {
                    "id": "institutional",
                    "retention": "durable",
                    "promotion_required": true,
                    "can_authorize_irreversible_action": false
                },
                {
                    "id": "episodic",
                    "retention": "expiring",
                    "promotion_required": false,
                    "can_authorize_irreversible_action": false
                }
            ]
        },
        "sessions": sessions,
        "summary": {
            "total_sessions": total_sessions,
            "triad_sessions": triad_sessions,
            "lightweight_sessions": lightweight_sessions,
            "human_escalations": human_escalations,
            "promoted_private_memory_total": promoted_private_memory,
            "validator_invocation_counts": validator_counts,
            "memory_lane_usage": lane_counts,
            "memory_write_counts": memory_write_counts
        },
        "paths": {
            "sessions": sessions_path.display().to_string(),
            "active_ruleset": ruleset_path.display().to_string()
        },
        "arda_hints": {
            "primary_panel": "ceo_council_runtime",
            "boardroom_section": "executive_council",
            "highlight_escalations": human_escalations > 0
        }
    })
}

fn build_human_augmentation_runtime_snapshot(
    ruleset: &Value,
    approvals_state: &Value,
    approvals_path: &Path,
    ruleset_path: &Path,
) -> Value {
    let policy = ruleset.get("policy").cloned().unwrap_or_else(|| json!({}));
    let human_augmentation = policy
        .get("human_augmentation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let approvals = approvals_state
        .get("approvals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let pending_total = approvals
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("pending"))
        .count();
    let approved_total = approvals
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("approved"))
        .count();
    json!({
        "schema_version": "arda.human-augmentation-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "control.approve-human-augmentation",
        "policy": human_augmentation,
        "approvals": approvals,
        "summary": {
            "pending_total": pending_total,
            "approved_total": approved_total
        },
        "paths": {
            "approvals": approvals_path.display().to_string(),
            "active_ruleset": ruleset_path.display().to_string()
        },
        "arda_hints": {
            "primary_panel": "human_augmentation_runtime",
            "boardroom_section": "governance_guardhouse",
            "highlight_pending_approvals": pending_total > 0
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn approve_human_augmentation(
    decision_class: &str,
    command_signature: Option<&str>,
    approvers: &[String],
    evidence: &[String],
    expires_at_utc: Option<&str>,
    note: Option<&str>,
    status: &str,
    approvals_path: &str,
    runtime_out: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let approvals_path = root.join(approvals_path);
    let runtime_out = root.join(runtime_out);
    let ruleset_path = root.join("core/state/active_ruleset.json");

    let mut approvals_state = read_json(&approvals_path);
    if approvals_state.as_object().is_none() {
        approvals_state = default_human_augmentation_approval_state();
    }
    if approvals_state
        .get("approvals")
        .and_then(Value::as_array)
        .is_none()
    {
        approvals_state["approvals"] = json!([]);
    }
    approvals_state["schema_version"] = json!("arda.human-augmentation-approval.v1");
    approvals_state["updated_at_utc"] = json!(now_utc());

    let approval_id = format!(
        "ha_{}_{}",
        decision_class.replace(['/', ' '], "_"),
        Utc::now().timestamp()
    );
    let approval = json!({
        "approval_id": approval_id,
        "decision_class": decision_class,
        "command_signature": command_signature,
        "approvers": approvers,
        "evidence": evidence,
        "status": status,
        "note": note,
        "approved_at_utc": now_utc(),
        "expires_at_utc": expires_at_utc
    });

    if let Some(approvals) = approvals_state
        .get_mut("approvals")
        .and_then(Value::as_array_mut)
    {
        approvals.push(approval.clone());
    } else {
        anyhow::bail!("approvals state missing approvals array");
    }
    write_json(&approvals_path, &approvals_state)?;

    let ruleset = read_json(&ruleset_path);
    let runtime = build_human_augmentation_runtime_snapshot(
        &ruleset,
        &approvals_state,
        &approvals_path,
        &ruleset_path,
    );
    write_json(&runtime_out, &runtime)?;

    Ok(json!({
        "ts_utc": now_utc(),
        "authority": "approve_human_augmentation",
        "approval": approval,
        "runtime_out": runtime_out.display().to_string()
    }))
}

#[allow(clippy::too_many_arguments)]
fn record_ceo_council_session(
    objective: &str,
    ceo_identity: &str,
    cto_identity: &str,
    cto_mode: &str,
    ingress: &str,
    channel_ref: Option<&str>,
    loop_class: &str,
    decision_class: &str,
    triad_required: bool,
    participants: &[String],
    proposals: &[String],
    objections: &[String],
    synthesis: Option<&str>,
    outcome_status: &str,
    human_escalated: bool,
    validators_invoked: &[String],
    memory_lanes: &[String],
    memory_writes: &[String],
    promoted_private_memory: bool,
    sessions_path: &str,
    runtime_out: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let sessions_path = root.join(sessions_path);
    let runtime_out = root.join(runtime_out);
    let ruleset_path = root.join("core/state/active_ruleset.json");

    let mut sessions_state = read_json(&sessions_path);
    if sessions_state.as_object().is_none() {
        sessions_state = default_ceo_council_state();
    }
    if sessions_state
        .get("sessions")
        .and_then(Value::as_array)
        .is_none()
    {
        sessions_state["sessions"] = json!([]);
    }
    sessions_state["schema_version"] = json!("arda.ceo-council-sessions.v1");
    sessions_state["updated_at_utc"] = json!(now_utc());

    let session_id = format!("council_{}", Utc::now().timestamp());
    let memory_write_intents: Vec<Value> = memory_writes
        .iter()
        .enumerate()
        .map(|(idx, content)| {
            let lane = memory_lanes
                .get(idx)
                .cloned()
                .unwrap_or_else(|| "episodic".to_string());
            let retention = if lane == "ceo_private_working" || lane == "episodic" {
                "expiring"
            } else {
                "durable"
            };
            let promotion_required = lane == "ceo_private_working" || lane == "institutional";
            json!({
                "intent_id": format!("{}_mw_{}", session_id, idx + 1),
                "lane": lane,
                "content": content,
                "retention_class": retention,
                "promotion_required": promotion_required,
                "approved": !promotion_required || promoted_private_memory
            })
        })
        .collect();

    let session = json!({
        "session_id": session_id,
        "objective": objective,
        "ceo_identity": ceo_identity,
        "cto_identity": cto_identity,
        "cto_mode": cto_mode,
        "ingress": ingress,
        "channel_ref": channel_ref,
        "loop_class": loop_class,
        "decision_class": decision_class,
        "triad_required": triad_required,
        "participants": participants,
        "proposals": proposals,
        "objections": objections,
        "synthesis": synthesis,
        "validators_invoked": validators_invoked,
        "memory_lanes": memory_lanes,
        "memory_write_intents": memory_write_intents,
        "promoted_private_memory": promoted_private_memory,
        "human_escalated": human_escalated,
        "outcome_status": outcome_status,
        "created_at_utc": now_utc(),
        "closed_at_utc": now_utc()
    });

    if let Some(sessions) = sessions_state
        .get_mut("sessions")
        .and_then(Value::as_array_mut)
    {
        sessions.push(session.clone());
    } else {
        anyhow::bail!("CEO council state missing sessions array");
    }
    write_json(&sessions_path, &sessions_state)?;

    let ruleset = read_json(&ruleset_path);
    let runtime = build_ceo_council_runtime_snapshot(
        &ruleset,
        &sessions_state,
        &sessions_path,
        &ruleset_path,
    );
    write_json(&runtime_out, &runtime)?;

    Ok(json!({
        "ts_utc": now_utc(),
        "authority": "record_ceo_council_session",
        "session": session,
        "runtime_out": runtime_out.display().to_string()
    }))
}

fn append_jsonl(path: &Path, row: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    handle.write_all(serde_json::to_string(row)?.as_bytes())?;
    handle.write_all(b"\n")?;
    Ok(())
}

fn runtime_dir() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_RUNTIME_SOCKET_DIR") {
        let p = PathBuf::from(path);
        if p.ends_with("arda") {
            return p;
        }
        return p.join("arda");
    }
    if let Ok(path) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(path);
        if p.ends_with("arda") {
            return p;
        }
        return p.join("arda");
    }
    PathBuf::from(format!("/run/user/{}/arda", nixless_uid()))
}

fn now_utc_seconds() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn relative_label(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

fn safe_slug(path: &str) -> String {
    path.replace(['/', '\\'], "__").replace(':', "_")
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "__pycache__" | "node_modules")
    )
}

fn glob_to_regex(pattern: &str) -> anyhow::Result<regex::Regex> {
    let mut out = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.push('$');
    Ok(regex::Regex::new(&out)?)
}

fn matches_any(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        glob_to_regex(pattern)
            .map(|regex| regex.is_match(path))
            .unwrap_or(false)
    })
}

fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return path.metadata().map(|meta| meta.len()).unwrap_or(0);
    }
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate.is_dir() {
                stack.push(candidate);
            } else if candidate.is_file() {
                total += candidate.metadata().map(|meta| meta.len()).unwrap_or(0);
            }
        }
    }
    total
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let output = Command::new("sha256sum").arg(path).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "sha256sum failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string())
}

#[derive(Default)]
struct MirrorStats {
    files: usize,
    bytes: u64,
    skipped_files: usize,
    skipped_bytes: u64,
    compressed_files: usize,
    observed_source_bytes: u64,
}

fn copy_tree(
    src: &Path,
    dest: &Path,
    exclude_globs: &[String],
    compress_globs: &[String],
) -> anyhow::Result<MirrorStats> {
    let mut stats = MirrorStats::default();
    if !src.exists() {
        return Ok(stats);
    }
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    let mut stack = vec![src.to_path_buf()];
    while let Some(current) = stack.pop() {
        let rel = current.strip_prefix(src).unwrap_or(Path::new(""));
        let target_dir = dest.join(rel);
        fs::create_dir_all(&target_dir)?;
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate.is_dir() {
                if should_skip_dir(&candidate) {
                    continue;
                }
                stack.push(candidate);
                continue;
            }
            let rel_file = candidate
                .strip_prefix(src)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .replace('\\', "/");
            let source_bytes = candidate.metadata().map(|meta| meta.len()).unwrap_or(0);
            stats.observed_source_bytes += source_bytes;
            if matches_any(&rel_file, exclude_globs) {
                stats.skipped_files += 1;
                stats.skipped_bytes += source_bytes;
                continue;
            }
            if matches_any(&rel_file, compress_globs) {
                let dest_file = target_dir.join(format!(
                    "{}.gz",
                    candidate
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("compressed")
                ));
                let output = Command::new("gzip").arg("-c").arg(&candidate).output()?;
                if !output.status.success() {
                    stats.skipped_files += 1;
                    stats.skipped_bytes += source_bytes;
                    continue;
                }
                fs::write(&dest_file, output.stdout)?;
                stats.bytes += dest_file.metadata().map(|meta| meta.len()).unwrap_or(0);
                stats.files += 1;
                stats.compressed_files += 1;
                continue;
            }
            let dest_file = target_dir.join(
                candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file"),
            );
            if let Some(parent) = dest_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&candidate, &dest_file)?;
            stats.files += 1;
            stats.bytes += source_bytes;
        }
    }
    Ok(stats)
}

fn snapshot_manifest(root: &Path, src: &Path, dest: &Path) -> anyhow::Result<Value> {
    let mut entries = Vec::new();
    if src.exists() {
        let mut stack = vec![src.to_path_buf()];
        while let Some(current) = stack.pop() {
            let Ok(dir_entries) = fs::read_dir(&current) else {
                continue;
            };
            for entry in dir_entries.flatten() {
                let candidate = entry.path();
                if candidate.is_dir() {
                    if should_skip_dir(&candidate) {
                        continue;
                    }
                    stack.push(candidate);
                } else if candidate.is_file() {
                    let Ok(meta) = candidate.metadata() else {
                        continue;
                    };
                    let modified_at = meta
                        .modified()
                        .ok()
                        .map(chrono::DateTime::<Utc>::from)
                        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
                        .unwrap_or_else(now_utc_seconds);
                    entries.push(json!({
                        "path": relative_label(root, &candidate),
                        "bytes": meta.len(),
                        "modified_at_utc": modified_at,
                    }));
                }
            }
        }
    }
    entries.sort_by(|left, right| {
        left.get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .cmp(
                right
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
    });
    let total_bytes = entries
        .iter()
        .filter_map(|entry| entry.get("bytes").and_then(Value::as_u64))
        .sum::<u64>();
    let manifest = json!({
        "generated_at_utc": now_utc_seconds(),
        "source_path": relative_label(root, src),
        "entry_count": entries.len(),
        "sample": entries.iter().take(200).cloned().collect::<Vec<_>>(),
        "total_bytes": total_bytes,
    });
    write_json(dest, &manifest)?;
    Ok(manifest)
}

fn sync_output_accounting(
    topology_path: &str,
    state_path: &str,
    ledger_path: &str,
    mirror_root: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let topology_path = root.join(topology_path);
    let state_path = root.join(state_path);
    let ledger_path = root.join(ledger_path);
    let mirror_root = root.join(mirror_root);
    let topology = read_json(&topology_path);
    if topology == json!({}) {
        anyhow::bail!("missing topology file: {}", topology_path.display());
    }
    fs::create_dir_all(&mirror_root)?;
    let generated_at = now_utc_seconds();
    let mut candidate_results = Vec::new();
    let mut total_mirrored_files = 0usize;
    let mut total_mirrored_bytes = 0u64;
    let mut total_observed_source_bytes = 0u64;
    let mut total_estimated_joulework = 0.0f64;

    for candidate in topology
        .get("long_term_accounting_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let source_rel = candidate
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let recommended_action = candidate
            .get("recommended_action")
            .and_then(Value::as_str)
            .unwrap_or("mirror_tree")
            .to_string();
        let exclude_globs = candidate
            .get("exclude_globs")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let compress_globs = candidate
            .get("compress_globs")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let source_path = root.join(&source_rel);
        let mirror_dir = mirror_root.join(safe_slug(&source_rel));
        let estimated_joulework = candidate
            .get("estimated_joulework")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        total_estimated_joulework += estimated_joulework;
        let mut result = json!({
            "path": source_rel,
            "reason": candidate.get("reason").cloned().unwrap_or(Value::String(String::new())),
            "recommended_action": recommended_action,
            "priority": candidate.get("priority").cloned().unwrap_or(Value::String("unknown".to_string())),
            "estimated_joulework": estimated_joulework,
            "status": "missing",
            "source_exists": source_path.exists(),
        });
        if !exclude_globs.is_empty() {
            result["exclude_globs"] = json!(exclude_globs);
        }
        if !compress_globs.is_empty() {
            result["compress_globs"] = json!(compress_globs);
        }

        if result
            .get("source_exists")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if recommended_action == "snapshot_manifest" {
                let manifest_path = mirror_dir.join("manifest.json");
                let manifest = snapshot_manifest(&root, &source_path, &manifest_path)?;
                let manifest_size = manifest_path.metadata().map(|meta| meta.len()).unwrap_or(0);
                result["status"] = Value::String("snapshotted".to_string());
                result["mirror_path"] = Value::String(relative_label(&root, &manifest_path));
                result["mirrored_files"] = Value::from(1u64);
                result["mirrored_bytes"] = Value::from(manifest_size);
                result["observed_source_bytes"] = manifest
                    .get("total_bytes")
                    .cloned()
                    .unwrap_or(Value::from(0u64));
                result["content_sha256"] = Value::String(sha256_file(&manifest_path)?);
                total_mirrored_files += 1;
                total_mirrored_bytes += manifest_size;
                total_observed_source_bytes += manifest
                    .get("total_bytes")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
            } else {
                let stats = copy_tree(&source_path, &mirror_dir, &exclude_globs, &compress_globs)?;
                let manifest_path = mirror_dir.join("_mirror_manifest.json");
                write_json(
                    &manifest_path,
                    &json!({
                        "generated_at_utc": generated_at,
                        "source_path": relative_label(&root, &source_path),
                        "mirror_path": relative_label(&root, &mirror_dir),
                        "files": stats.files,
                        "bytes": stats.bytes,
                        "compressed_files": stats.compressed_files,
                        "skipped_files": stats.skipped_files,
                        "skipped_bytes": stats.skipped_bytes,
                        "observed_source_bytes": stats.observed_source_bytes,
                    }),
                )?;
                result["status"] = Value::String("mirrored".to_string());
                result["mirror_path"] = Value::String(relative_label(&root, &mirror_dir));
                result["mirrored_files"] = Value::from(stats.files as u64);
                result["mirrored_bytes"] = Value::from(stats.bytes);
                result["observed_source_bytes"] = Value::from(stats.observed_source_bytes);
                result["compressed_files"] = Value::from(stats.compressed_files as u64);
                result["skipped_files"] = Value::from(stats.skipped_files as u64);
                result["skipped_bytes"] = Value::from(stats.skipped_bytes);
                result["content_sha256"] = Value::String(sha256_file(&manifest_path)?);
                total_mirrored_files += stats.files;
                total_mirrored_bytes += stats.bytes;
                total_observed_source_bytes += stats.observed_source_bytes;
            }
        }

        candidate_results.push(result);
    }

    let projection = json!({
        "schema_version": "arda.core.state.v1",
        "generated_at_utc": generated_at,
        "authority": "output_accounting_projection",
        "mode": "mirror_only_non_destructive",
        "mirror_root": relative_label(&root, &mirror_root),
        "runtime_guards": {
            "moves_runtime_authority": false,
            "mutates_operational_paths": false,
            "requires_manual_relocation": true,
        },
        "candidates": candidate_results,
        "summary": {
            "candidate_total": candidate_results.len(),
            "mirrored_files_total": total_mirrored_files,
            "mirrored_bytes_total": total_mirrored_bytes,
            "observed_source_bytes_total": total_observed_source_bytes,
            "estimated_joulework_total": total_estimated_joulework,
        },
    });
    write_json(&state_path, &projection)?;
    append_jsonl(
        &ledger_path,
        &json!({
            "ts": generated_at,
            "status": "ok",
            "candidate_total": projection["summary"]["candidate_total"],
            "mirrored_files_total": projection["summary"]["mirrored_files_total"],
            "mirrored_bytes_total": projection["summary"]["mirrored_bytes_total"],
            "state_path": relative_label(&root, &state_path),
        }),
    )?;
    Ok(json!({
        "out": state_path.display().to_string(),
        "candidate_total": projection["summary"]["candidate_total"],
    }))
}

fn prune_runtime_build_cache(out_path: &str) -> anyhow::Result<Value> {
    let root = workspace_root();
    let build_root = std::env::var("ARDA_BUILD_CACHE_ROOT")
        .or_else(|_| std::env::var("ARDA_RUNTIME_BUILD_ROOT"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/arda-build"));
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| build_root.join("target"));
    let tmp_dir = std::env::var("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| build_root.join("tmp"));
    let max_age_hours = std::env::var("ARDA_BUILD_CACHE_MAX_AGE_HOURS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(24u64);
    let max_bytes = std::env::var("ARDA_BUILD_CACHE_MAX_BYTES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4 * 1024 * 1024 * 1024u64);
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_hours * 3600))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let mut removed_bytes = 0u64;
    let mut removed_paths = Vec::new();

    if tmp_dir.exists()
        && tmp_dir
            .metadata()
            .and_then(|meta| meta.modified())
            .map(|modified| modified < cutoff)
            .unwrap_or(false)
    {
        removed_bytes += dir_size(&tmp_dir);
        fs::remove_dir_all(&tmp_dir).ok();
        removed_paths.push(tmp_dir.display().to_string());
    }

    let pressure_candidates = vec![
        target_dir.join("debug/incremental"),
        target_dir.join("debug/.fingerprint"),
        target_dir.join("debug/build"),
        target_dir.join("debug/deps"),
        target_dir.join("debug/examples"),
    ];

    let mut observed_bytes = dir_size(&build_root);
    let mut target_bytes = dir_size(&target_dir);
    if observed_bytes > max_bytes {
        for candidate in &pressure_candidates {
            if candidate.exists() {
                removed_bytes += dir_size(candidate);
                if candidate.is_dir() {
                    fs::remove_dir_all(candidate).ok();
                } else {
                    fs::remove_file(candidate).ok();
                }
                removed_paths.push(candidate.display().to_string());
            }
        }
        observed_bytes = dir_size(&build_root);
        target_bytes = dir_size(&target_dir);
    }

    let payload = json!({
        "schema_version": "arda.runtime-build-cache.v1",
        "generated_at_utc": now_utc_seconds(),
        "authority": "runtime_build_cache_compactor",
        "build_root": build_root.display().to_string(),
        "target_dir": target_dir.display().to_string(),
        "tmp_dir": tmp_dir.display().to_string(),
        "retention": {
            "max_age_hours": max_age_hours,
            "max_bytes": max_bytes,
        },
        "observed_bytes": observed_bytes,
        "target_bytes": target_bytes,
        "removed_bytes": removed_bytes,
        "removed_paths": removed_paths,
        "pressure_candidates": pressure_candidates.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "soterion_sigils": ["∇", "⚡", "◈", "↝"],
        "soterion_sigils_binary": {
            "command": "111000",
            "joulework": "101010",
            "truth": "110001",
            "transition": "100111",
        },
        "status": if observed_bytes <= max_bytes { "ok" } else { "pressure_remaining" },
    });
    let out_path = root.join(out_path);
    write_json(&out_path, &payload)?;
    Ok(json!({
        "out": out_path.display().to_string(),
        "status": payload["status"].clone(),
    }))
}

fn organize_hades_backups(hades_root: &str, out_path: &str) -> anyhow::Result<Value> {
    let root = workspace_root();
    let hades_root = root.join(hades_root);
    let archive_root = std::env::var("ARDA_HADES_ARCHIVE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| hades_root.join("archive/backups"));
    fs::create_dir_all(&archive_root)?;
    let mut moved = Vec::new();
    if hades_root.exists() {
        for entry in fs::read_dir(&hades_root)?.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.contains(".bak.") || !path.is_file() {
                continue;
            }
            let stem = name.split(".bak.").next().unwrap_or("backup");
            let target_dir = archive_root.join(stem);
            fs::create_dir_all(&target_dir)?;
            let target = target_dir.join(name);
            if target.exists() {
                continue;
            }
            fs::rename(&path, &target)?;
            moved.push(json!({
                "from": path.display().to_string(),
                "to": target.display().to_string(),
            }));
        }
    }
    let active_hades_files = fs::read_dir(&hades_root)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file()
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .contains(".bak.")
            {
                Some(relative_label(&hades_root, &path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let manwe_root = root.join("data/manwe");
    let active_manwe_files = fs::read_dir(&manwe_root)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() {
                Some(relative_label(&manwe_root, &path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let backup_files_total = if archive_root.exists() {
        let mut total = 0usize;
        let mut stack = vec![archive_root.clone()];
        while let Some(current) = stack.pop() {
            let Ok(entries) = fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.flatten() {
                let candidate = entry.path();
                if candidate.is_dir() {
                    stack.push(candidate);
                } else if candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .contains(".bak.")
                {
                    total += 1;
                }
            }
        }
        total
    } else {
        0
    };
    let payload = json!({
        "schema_version": "arda.hades-manwe-layout.v1",
        "generated_at_utc": now_utc_seconds(),
        "authority": "bounded_backup_organization",
        "hades": {
            "active_root_files": active_hades_files,
            "backup_files_total": backup_files_total,
            "backup_archive_root": relative_label(&root, &archive_root),
        },
        "manwe": {
            "active_root_files": active_manwe_files,
        },
        "moves_this_run": moved,
    });
    let out_path = root.join(out_path);
    write_json(&out_path, &payload)?;
    Ok(json!({
        "out": out_path.display().to_string(),
        "moves": payload.get("moves_this_run").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
    }))
}

fn dedupe_key(row: &Value) -> Option<String> {
    let reason = row
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if reason == "policy_guard.denied" {
        let command = row
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();
        return Some(format!("{reason}|{command}"));
    }
    if reason.starts_with("core_pressure_guard.") {
        return Some(reason.to_string());
    }
    None
}

fn pressure_reason_active(reason: &str, pressure_report: &Value) -> bool {
    let observed = pressure_report.get("observed").unwrap_or(&Value::Null);
    let thresholds = pressure_report.get("thresholds").unwrap_or(&Value::Null);
    match reason {
        "core_pressure_guard.total_known_work" => {
            observed
                .get("total_known_work_items")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > thresholds
                    .get("max_total_known_work")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
        }
        "core_pressure_guard.projects_queue_queued" => {
            observed
                .get("projects_queue_queued")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > thresholds
                    .get("max_projects_queue_queued")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
        }
        "core_pressure_guard.oversize_files" => {
            observed
                .get("oversize_files_gte_100mb")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > thresholds
                    .get("max_oversize_count")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
        }
        "core_pressure_guard.disk_used_pct" => {
            observed
                .get("disk_used_pct")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= thresholds
                    .get("max_disk_used_pct")
                    .and_then(Value::as_i64)
                    .unwrap_or(100)
        }
        "core_pressure_guard.invalid_json" => {
            observed
                .get("invalid_json_artifacts")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > thresholds
                    .get("max_invalid_json")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
        }
        _ => true,
    }
}

fn escalation_still_active(row: &Value, autonomy_runtime: &Value, pressure_report: &Value) -> bool {
    let reason = row
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if reason == "policy_guard.denied" {
        return autonomy_runtime
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("normal")
            .eq_ignore_ascii_case("degraded");
    }
    if reason.starts_with("core_pressure_guard.") {
        return pressure_reason_active(reason, pressure_report);
    }
    true
}

fn reconcile_escalations(
    escalations_path: &str,
    autonomy_runtime_path: &str,
    pressure_report_path: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let escalations_path = root.join(escalations_path);
    if !escalations_path.exists() {
        return Ok(json!({"status":"missing","path": escalations_path.display().to_string()}));
    }
    let autonomy_runtime = read_json(&root.join(autonomy_runtime_path));
    let pressure_report = read_json(&root.join(pressure_report_path));
    let mut latest_by_id = std::collections::BTreeMap::new();
    for line in fs::read_to_string(&escalations_path)?.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if let Some(escalation_id) = row.get("escalation_id").and_then(Value::as_str) {
            latest_by_id.insert(escalation_id.to_string(), row);
        }
    }
    let mut pending_groups: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();
    for row in latest_by_id.into_values() {
        if row.get("status").and_then(Value::as_str) != Some("pending") {
            continue;
        }
        if let Some(key) = dedupe_key(&row) {
            pending_groups.entry(key).or_default().push(row);
        }
    }
    let ts = now_utc_seconds();
    let mut resolved_rows = Vec::new();
    let mut dedupe_groups = serde_json::Map::new();
    for (key, group) in pending_groups {
        if group.len() > 1 {
            dedupe_groups.insert(key.clone(), Value::from(group.len() as u64));
        }
        let (duplicates_to_resolve, note) = if group.len() <= 1 {
            let row = &group[0];
            if escalation_still_active(row, &autonomy_runtime, &pressure_report) {
                continue;
            }
            (
                group,
                format!("escalation condition cleared by current runtime for key {key}"),
            )
        } else {
            let mut group = group;
            group.sort_by(|left, right| {
                left.get("ts")
                    .or_else(|| left.get("ts_utc"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .cmp(
                        right
                            .get("ts")
                            .or_else(|| right.get("ts_utc"))
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    )
            });
            let latest = group.last().cloned().unwrap_or_else(|| json!({}));
            if escalation_still_active(&latest, &autonomy_runtime, &pressure_report) {
                (
                    group[..group.len() - 1].to_vec(),
                    format!("duplicate escalation reconciled by key {key}"),
                )
            } else {
                (
                    group,
                    format!("escalation condition cleared by current runtime for key {key}"),
                )
            }
        };
        for duplicate in duplicates_to_resolve {
            resolved_rows.push(json!({
                "escalation_id": duplicate.get("escalation_id").cloned().unwrap_or(Value::Null),
                "ts": ts,
                "task_id": duplicate.get("task_id").cloned().unwrap_or(Value::String("reconcile".to_string())),
                "status": "resolved",
                "reason": duplicate.get("reason").cloned().unwrap_or(Value::String("reconciled".to_string())),
                "confidence": duplicate.get("confidence").cloned().unwrap_or(Value::from(1.0)),
                "note": note,
            }));
        }
    }
    for row in &resolved_rows {
        append_jsonl(&escalations_path, row)?;
    }
    Ok(json!({
        "status": "ok",
        "resolved_duplicates": resolved_rows.len(),
        "autonomy_mode": autonomy_runtime.get("mode").cloned().unwrap_or(Value::String("unknown".to_string())),
        "dedupe_groups": Value::Object(dedupe_groups),
    }))
}

fn nixless_uid() -> u32 {
    std::env::var("UID")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1000)
}

fn normalized_agents(raw: &str) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for token in raw.replace(',', " ").split_whitespace() {
        let value = token.trim();
        if value.is_empty() || !seen.insert(value.to_string()) {
            continue;
        }
        ordered.push(value.to_string());
    }
    ordered
}

fn extract_agent_block_ranges(toml: &str) -> Vec<(String, usize, usize)> {
    let mut markers = Vec::new();
    for (offset, _) in toml.match_indices("[agents.") {
        let after = &toml[offset + "[agents.".len()..];
        if let Some(end) = after.find(']') {
            let agent_id = after[..end].trim().to_string();
            if !agent_id.is_empty() {
                markers.push((agent_id, offset));
            }
        }
    }
    let mut ranges = Vec::new();
    for (idx, (agent_id, start)) in markers.iter().enumerate() {
        let end = markers
            .get(idx + 1)
            .map(|(_, next_start)| *next_start)
            .unwrap_or_else(|| toml.len());
        ranges.push((agent_id.clone(), *start, end));
    }
    ranges
}

fn get_agent_block_bounds(toml: &str, agent_id: &str) -> Option<(usize, usize)> {
    extract_agent_block_ranges(toml)
        .into_iter()
        .find_map(|(id, start, end)| {
            if id == agent_id {
                Some((start, end))
            } else {
                None
            }
        })
}

fn read_agent_field(toml: &str, agent_id: &str, field: &str) -> Option<String> {
    let (start, end) = get_agent_block_bounds(toml, agent_id)?;
    let block = &toml[start..end];
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(field) {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim();
                if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    return Some(value[1..value.len() - 1].to_string());
                }
            }
        }
    }
    None
}

fn update_agent_field(toml: &str, agent_id: &str, field: &str, value: &str) -> String {
    let Some((start, end)) = get_agent_block_bounds(toml, agent_id) else {
        return toml.to_string();
    };
    let block = &toml[start..end];
    let mut lines = block
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let mut replaced = false;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&format!("{field} ")) || trimmed.starts_with(&format!("{field}=")) {
            let indent_len = line.len() - trimmed.len();
            let indent = " ".repeat(indent_len);
            *line = format!("{indent}{field} = \"{value}\"");
            replaced = true;
            break;
        }
    }
    if !replaced {
        while matches!(lines.last(), Some(last) if last.trim().is_empty()) {
            lines.pop();
        }
        lines.push(format!("{field} = \"{value}\""));
    }
    let mut updated = lines.join("\n");
    if block.ends_with('\n') {
        updated.push('\n');
    }
    format!("{}{}{}", &toml[..start], updated, &toml[end..])
}

fn swap_entries() -> Vec<String> {
    let path = Path::new("/proc/swaps");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn apply_opencode_route_governor(
    model_control_path: &str,
    routes_path: &str,
    state_path: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let model_control_path = root.join(model_control_path);
    let routes_path = root.join(routes_path);
    let state_path = root.join(state_path);

    let model_control = read_json(&model_control_path);
    let recommendations = model_control
        .get("routing_advisor")
        .and_then(|v| v.get("opencode_agents"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let previous_state = read_json(&state_path);
    let previous_agents = previous_state
        .get("agents")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut routes_toml = fs::read_to_string(&routes_path).unwrap_or_default();

    let mut agents_out = serde_json::Map::new();
    let mut applied_total = 0usize;
    let mut skipped_override_total = 0usize;
    let mut changed = false;

    for (agent_id, recommendation) in recommendations {
        let Some(recommendation) = recommendation.as_object() else {
            continue;
        };
        let current_provider = read_agent_field(&routes_toml, &agent_id, "provider")
            .unwrap_or_else(|| "auto".to_string());
        let current_profile =
            read_agent_field(&routes_toml, &agent_id, "model_profile").unwrap_or_default();
        let recommended_provider = recommendation
            .get("recommended_provider")
            .and_then(Value::as_str)
            .unwrap_or("auto")
            .to_string();
        let recommended_profile = recommendation
            .get("recommended_model_profile")
            .and_then(Value::as_str)
            .unwrap_or(&current_profile)
            .to_string();
        let previous_agent = previous_agents
            .get(&agent_id)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let previously_applied_provider = previous_agent
            .get("applied_provider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let previously_applied_profile = previous_agent
            .get("applied_model_profile")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let under_governor_control = current_provider == "auto"
            || (Some(current_provider.as_str()) == previously_applied_provider.as_deref()
                && Some(current_profile.as_str()) == previously_applied_profile.as_deref());

        let mut effective_provider = current_provider.clone();
        let mut effective_profile = current_profile.clone();
        let (status, reason) = if !under_governor_control
            && (current_provider != recommended_provider || current_profile != recommended_profile)
        {
            skipped_override_total += 1;
            (
                "manual_override_respected",
                "Current route differs from governor-managed lane; leaving explicit override in place.",
            )
        } else if current_provider != recommended_provider || current_profile != recommended_profile
        {
            routes_toml =
                update_agent_field(&routes_toml, &agent_id, "provider", &recommended_provider);
            routes_toml = update_agent_field(
                &routes_toml,
                &agent_id,
                "model_profile",
                &recommended_profile,
            );
            effective_provider = recommended_provider.clone();
            effective_profile = recommended_profile.clone();
            applied_total += 1;
            changed = true;
            (
                "applied",
                "Applied bounded health recommendation into sovereign OpenCode route contract.",
            )
        } else {
            ("unchanged", "Already aligned with recommendation.")
        };

        agents_out.insert(
            agent_id,
            json!({
                "task_type": recommendation.get("task_type").cloned().unwrap_or(Value::Null),
                "current_provider": effective_provider,
                "current_model_profile": effective_profile,
                "recommended_provider": recommended_provider,
                "recommended_model_profile": recommended_profile,
                "applied_provider": if matches!(status, "applied" | "unchanged") {
                    Value::String(effective_provider.clone())
                } else {
                    previously_applied_provider.map(Value::String).unwrap_or(Value::Null)
                },
                "applied_model_profile": if matches!(status, "applied" | "unchanged") {
                    Value::String(effective_profile.clone())
                } else {
                    previously_applied_profile.map(Value::String).unwrap_or(Value::Null)
                },
                "governor_controlled": under_governor_control,
                "status": status,
                "reason": reason,
            }),
        );
    }

    if changed {
        write_text(&routes_path, &routes_toml)?;
    }

    let payload = json!({
        "schema_version": "arda.opencode-route-governor.v1",
        "generated_at_utc": now_utc(),
        "authority": "model_control_surface + opencode_route_contract",
        "doctrine": {
            "bounded_auto_apply": true,
            "manual_non_auto_overrides_are_respected": true,
            "writes_flow_through_sovereign_route_contract": true,
            "arda_is_observer_and_override_surface_not_primary_authority": true,
        },
        "source_surfaces": {
            "model_control_surface": "core/state/model_control_surface.json",
            "route_contract": "config/opencode_agent_routes.toml",
        },
        "summary": {
            "agents_total": agents_out.len(),
            "applied_total": applied_total,
            "manual_override_total": skipped_override_total,
            "changed": changed,
        },
        "agents": agents_out,
    });
    write_json(&state_path, &payload)?;
    Ok(json!({
        "ok": true,
        "changed": changed,
        "state": state_path.to_string_lossy(),
    }))
}

fn apply_runtime_recovery_route_governor(
    recovery_path: &str,
    manwe_router_path: &str,
    route_matrix_path: &str,
    state_path: &str,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let recovery_path = root.join(recovery_path);
    let manwe_router_path = root.join(manwe_router_path);
    let route_matrix_path = root.join(route_matrix_path);
    let state_path = root.join(state_path);

    let recovery = read_json(&recovery_path);
    let manwe_router = read_json(&manwe_router_path);
    let previous = read_json(&state_path);
    let route_matrix = fs::read_to_string(&route_matrix_path).unwrap_or_default();

    let actions = recovery
        .get("recovery_actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let route_shift_active = actions
        .iter()
        .any(|action| action.get("kind").and_then(Value::as_str) == Some("route_shift"));
    let desired_origin = if route_shift_active {
        choose_runtime_recovery_origin(&manwe_router)
    } else {
        "auto".to_string()
    };

    let previous_origin = previous
        .get("applied_origin")
        .and_then(Value::as_str)
        .unwrap_or("auto")
        .to_string();
    let current_origin = route_matrix
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("default_inference_origin")
                .and_then(|rest| rest.trim().strip_prefix('='))
                .map(|value| value.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "auto".to_string());
    let governor_controlled = current_origin == "auto" || current_origin == previous_origin;
    let mut changed = false;
    let (status, reason, applied_origin, current_origin_out) = if !governor_controlled
        && current_origin != desired_origin
    {
        (
            "manual_override_respected",
            "Default route origin differs from previous governor state; explicit manual override respected.",
            previous_origin.clone(),
            current_origin.clone(),
        )
    } else if current_origin != desired_origin {
        let updated = update_route_matrix_origins(&route_matrix, &desired_origin);
        write_text(&route_matrix_path, &updated)?;
        changed = true;
        (
            "applied",
            "Applied bounded runtime recovery route shift into sovereign route matrix.",
            desired_origin.clone(),
            desired_origin.clone(),
        )
    } else {
        (
            "unchanged",
            "Route matrix already aligned with desired recovery posture.",
            desired_origin.clone(),
            current_origin.clone(),
        )
    };

    let payload = json!({
        "schema_version": "arda.runtime-recovery-route-governor.v1",
        "generated_at_utc": now_utc(),
        "authority": "runtime_admission_recovery + sovereign_route_matrix",
        "source_surfaces": {
            "runtime_admission_recovery": "core/state/runtime_admission_recovery.json",
            "manwe_router": "core/state/manwe_router.json",
            "route_matrix": "config/model_route_matrix.toml",
        },
        "summary": {
            "route_shift_active": route_shift_active,
            "changed": changed,
        },
        "current_origin": current_origin_out,
        "desired_origin": desired_origin,
        "applied_origin": applied_origin,
        "governor_controlled": governor_controlled,
        "status": status,
        "reason": reason,
    });
    write_json(&state_path, &payload)?;
    Ok(json!({
        "ok": true,
        "changed": changed,
        "state": state_path.to_string_lossy(),
    }))
}

fn choose_runtime_recovery_origin(manwe_router: &Value) -> String {
    let providers = manwe_router
        .get("provider_pressure")
        .and_then(|value| value.get("providers"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), row.clone()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let edge_backbone = providers.get("edge_backbone");
    let litellm_gateway = providers.get("litellm_gateway");
    let openrouter = providers.get("openrouter");
    if provider_ready(edge_backbone) {
        "edge".to_string()
    } else if provider_ready(litellm_gateway) || provider_ready(openrouter) {
        "cloud".to_string()
    } else {
        "edge".to_string()
    }
}

fn provider_ready(provider: Option<&Value>) -> bool {
    provider
        .map(|row| {
            row.get("enabled").and_then(Value::as_bool).unwrap_or(false)
                && row.get("healthy").and_then(Value::as_bool).unwrap_or(false)
        })
        .unwrap_or(false)
}

fn update_route_matrix_origins(raw: &str, desired_origin: &str) -> String {
    let mut lines = Vec::new();
    let mut current_section = String::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed.to_string();
            lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with("default_inference_origin") {
            let indent = " ".repeat(line.len() - line.trim_start().len());
            lines.push(format!(
                "{indent}default_inference_origin = \"{desired_origin}\""
            ));
            continue;
        }
        if matches!(
            current_section.as_str(),
            "[tiers.code]" | "[tiers.research]" | "[tiers.chat]"
        ) && trimmed.starts_with("primary_origin")
        {
            let indent = " ".repeat(line.len() - line.trim_start().len());
            lines.push(format!("{indent}primary_origin = \"{desired_origin}\""));
            continue;
        }
        lines.push(line.to_string());
    }
    let mut updated = lines.join("\n");
    if raw.ends_with('\n') {
        updated.push('\n');
    }
    updated
}

fn command_timeout_seconds(override_value: Option<u64>) -> u64 {
    override_value
        .or_else(|| {
            std::env::var("ARDA_RUNTIME_RECOVERY_CMD_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(20)
}

fn command_env() -> std::collections::BTreeMap<String, String> {
    let mut merged = std::env::vars().collect::<std::collections::BTreeMap<_, _>>();
    merged
        .entry("ARDA_WORKSPACE_ROOT".to_string())
        .or_insert_with(|| workspace_root().to_string_lossy().to_string());
    merged
}

fn run_bounded_command(cmd: &[&str], timeout_seconds: u64) -> Value {
    if cmd.is_empty() {
        return json!({
            "cmd": [],
            "exit_code": 0,
            "ok": true,
            "timed_out": false,
            "timeout_seconds": timeout_seconds,
            "stdout": "",
            "stderr": "",
        });
    }
    let mut command = Command::new("timeout");
    command.arg(timeout_seconds.to_string()).args(cmd);
    let output = command
        .current_dir(workspace_root())
        .envs(command_env())
        .output();
    match output {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(1);
            json!({
                "cmd": cmd,
                "exit_code": exit_code,
                "ok": output.status.success(),
                "timed_out": exit_code == 124,
                "timeout_seconds": timeout_seconds,
                "stdout": String::from_utf8_lossy(&output.stdout).trim().chars().take(4000).collect::<String>(),
                "stderr": String::from_utf8_lossy(&output.stderr).trim().chars().take(4000).collect::<String>(),
            })
        }
        Err(error) => json!({
            "cmd": cmd,
            "exit_code": 1,
            "ok": false,
            "timed_out": false,
            "timeout_seconds": timeout_seconds,
            "stdout": "",
            "stderr": error.to_string(),
        }),
    }
}

fn pressure_ok(summary: &Value) -> bool {
    summary
        .get("pressure_guard_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        == "ok"
        && !summary
            .get("local_joule_pressure")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn run_runtime_admission_recovery_executor(
    recovery_path: &str,
    out_path: &str,
    timeout_seconds: Option<u64>,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let recovery_path = root.join(recovery_path);
    let out_path = root.join(out_path);
    let recovery = read_json(&recovery_path);
    let summary = recovery
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let actions = recovery
        .get("recovery_actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let timeout_seconds = command_timeout_seconds(timeout_seconds);

    let mut runs = Vec::new();
    let mut route_shift_present = false;
    for action in &actions {
        let kind = action.get("kind").and_then(Value::as_str).unwrap_or("");
        let label = action
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let title = action.get("title").and_then(Value::as_str).unwrap_or(label);

        if kind == "route_shift" {
            route_shift_present = true;
            continue;
        }

        if kind == "reroute_retry" {
            let events = vec![
                run_bounded_command(
                    &[
                        "cargo",
                        "run",
                        "--quiet",
                        "--",
                        "hermes",
                        "retry-outbound",
                        "--limit",
                        "100",
                    ],
                    timeout_seconds,
                ),
                run_bounded_command(
                    &[
                        "cargo",
                        "run",
                        "--quiet",
                        "--",
                        "hermes",
                        "retry-reroute-dlq",
                        "--limit",
                        "100",
                    ],
                    timeout_seconds,
                ),
            ];
            let ok = events
                .iter()
                .all(|event| event.get("ok").and_then(Value::as_bool) == Some(true));
            runs.push(json!({
                "label": label,
                "title": title,
                "kind": kind,
                "status": if ok { "executed" } else { "failed" },
                "events": events,
            }));
        } else if kind == "deferred_retry" {
            if !pressure_ok(&summary) {
                runs.push(json!({
                    "label": label,
                    "title": title,
                    "kind": kind,
                    "status": "skipped",
                    "reason": "pressure_not_clear_for_deferred_retry",
                }));
                continue;
            }
            let event = if label.starts_with("athena_") {
                run_bounded_command(
                    &[
                        "cargo",
                        "run",
                        "--quiet",
                        "--",
                        "athena",
                        "deep-process",
                        "--limit",
                        "25",
                        "--retry-failed",
                    ],
                    timeout_seconds,
                )
            } else if label.starts_with("hades_") {
                run_bounded_command(
                    &[
                        "cargo",
                        "run",
                        "--quiet",
                        "--",
                        "hades",
                        "sweep",
                        "--sweep-type",
                        "manual",
                    ],
                    timeout_seconds,
                )
            } else {
                json!({
                    "cmd": [],
                    "exit_code": 0,
                    "ok": true,
                    "timed_out": false,
                    "timeout_seconds": timeout_seconds,
                    "stdout": "",
                    "stderr": "",
                })
            };
            runs.push(json!({
                "label": label,
                "title": title,
                "kind": kind,
                "status": if event.get("ok").and_then(Value::as_bool) == Some(true) { "executed" } else { "failed" },
                "events": [event],
            }));
        }
    }

    if route_shift_present {
        let route_event = run_bounded_command(
            &[
                "cargo",
                "run",
                "--quiet",
                "--",
                "control",
                "apply-runtime-recovery-route-governor",
            ],
            timeout_seconds,
        );
        if route_event.get("ok").and_then(Value::as_bool) == Some(true) {
            let sync_event = run_bounded_command(
                &["bash", "scripts/control/sync_model_control_surface.sh"],
                timeout_seconds,
            );
            let opencode_event = run_bounded_command(
                &[
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "control",
                    "apply-opencode-route-governor",
                ],
                timeout_seconds,
            );
            let remote_contract_event = run_bounded_command(
                &[
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "export",
                    "remote-operator-contract",
                ],
                timeout_seconds,
            );
            let ok = sync_event.get("ok").and_then(Value::as_bool) == Some(true)
                && opencode_event.get("ok").and_then(Value::as_bool) == Some(true)
                && remote_contract_event.get("ok").and_then(Value::as_bool) == Some(true);
            runs.push(json!({
                "label": "runtime_route_shift",
                "title": "Apply runtime route shift governor",
                "kind": "route_shift",
                "status": if ok { "executed" } else { "failed" },
                "events": [route_event, sync_event, opencode_event, remote_contract_event],
            }));
        } else {
            runs.push(json!({
                "label": "runtime_route_shift",
                "title": "Apply runtime route shift governor",
                "kind": "route_shift",
                "status": "failed",
                "events": [route_event],
            }));
        }
    }

    let payload = json!({
        "schema_version": "arda.runtime-admission-recovery-executor.v1",
        "generated_at_utc": now_utc(),
        "authority": "runtime_admission_recovery + bounded_executor",
        "summary": {
            "recovery_actions_total": actions.len(),
            "executed_total": runs.iter().filter(|run| run.get("status").and_then(Value::as_str) == Some("executed")).count(),
            "skipped_total": runs.iter().filter(|run| run.get("status").and_then(Value::as_str) == Some("skipped")).count(),
            "failed_total": runs.iter().filter(|run| run.get("status").and_then(Value::as_str) == Some("failed")).count(),
            "pressure_guard_status": summary.get("pressure_guard_status").cloned().unwrap_or(Value::Null),
            "local_joule_pressure": summary.get("local_joule_pressure").cloned().unwrap_or(Value::Null),
        },
        "runs": runs,
    });
    write_json(&out_path, &payload)?;
    Ok(json!({
        "out": out_path.to_string_lossy(),
        "executed_total": payload["summary"]["executed_total"],
    }))
}

fn sync_operator_profile(
    profile_path: &str,
    runtime_out: &str,
    env_out: &str,
    apply_runtime: bool,
    apply_env: bool,
    force_env: bool,
    apply_control_policy: bool,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let profile_path = root.join(profile_path);
    let runtime_out = root.join(runtime_out);
    let env_out = root.join(env_out);
    let profile_label = profile_path
        .strip_prefix(&root)
        .unwrap_or(&profile_path)
        .to_string_lossy()
        .to_string();
    let profile = read_json(&profile_path);
    let paths = profile.get("paths").cloned().unwrap_or_else(|| json!({}));
    let routing = profile.get("routing").cloned().unwrap_or_else(|| json!({}));
    let retention = profile
        .get("retention")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let pressure = profile
        .get("pressure")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let handoff = profile.get("handoff").cloned().unwrap_or_else(|| json!({}));
    let autonomy = profile
        .get("autonomy")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let integrations = profile
        .get("integrations")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_dir = runtime_dir().to_string_lossy().to_string();
    let replace_runtime = |value: Option<&str>| {
        value
            .unwrap_or_default()
            .replace("{runtime_dir}", &runtime_dir)
    };
    let hades_watch_paths = paths
        .get("hades_watch_paths")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("core"), json!("docs"), json!("config")]);
    let required_live_sockets = autonomy
        .get("required_live_sockets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut runtime = vec![
        (
            "ARDA_TASK_QUEUE_PATH".to_string(),
            paths
                .get("task_queue")
                .and_then(Value::as_str)
                .unwrap_or("core/projects/tasks/queue.jsonl")
                .to_string(),
        ),
        (
            "ARDA_PROJECT_TASK_QUEUE_PATH".to_string(),
            paths
                .get("project_task_queue")
                .and_then(Value::as_str)
                .unwrap_or("core/projects/tasks/queue.jsonl")
                .to_string(),
        ),
        (
            "ARDA_DAILY_QUEUE_PATH".to_string(),
            paths
                .get("daily_queue")
                .and_then(Value::as_str)
                .unwrap_or("core/queue/queue.jsonl")
                .to_string(),
        ),
        (
            "ARDA_WARDEN_QUEUE_PATH".to_string(),
            paths
                .get("warden_queue")
                .and_then(Value::as_str)
                .unwrap_or("data/warden/informant_queue.jsonl")
                .to_string(),
        ),
        (
            "ARDA_HADES_WATCH_PATHS".to_string(),
            hades_watch_paths
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(":"),
        ),
        (
            "ARDA_ROUTE_PRIVACY_DEFAULT".to_string(),
            routing
                .get("privacy_default")
                .and_then(Value::as_str)
                .unwrap_or("public")
                .to_string(),
        ),
        (
            "ARDA_ROUTE_COST_DEFAULT".to_string(),
            routing
                .get("cost_default")
                .and_then(Value::as_str)
                .unwrap_or("balanced")
                .to_string(),
        ),
        (
            "ARDA_ROUTE_QUALITY_DEFAULT".to_string(),
            routing
                .get("quality_default")
                .and_then(Value::as_str)
                .unwrap_or("balanced")
                .to_string(),
        ),
        (
            "ARDA_ROUTE_ORIGIN_DEFAULT".to_string(),
            routing
                .get("origin_default")
                .and_then(Value::as_str)
                .unwrap_or("auto")
                .to_string(),
        ),
        (
            "ARDA_RETENTION_DAYS_HADES_QUEUE".to_string(),
            retention
                .get("hades_queue_days")
                .and_then(Value::as_i64)
                .unwrap_or(14)
                .to_string(),
        ),
        (
            "ARDA_RETENTION_DAYS_HADES_LOG".to_string(),
            retention
                .get("hades_log_days")
                .and_then(Value::as_i64)
                .unwrap_or(30)
                .to_string(),
        ),
        (
            "ARDA_RETENTION_DAYS_HADES_JOULEWORK".to_string(),
            retention
                .get("hades_joulework_days")
                .and_then(Value::as_i64)
                .unwrap_or(30)
                .to_string(),
        ),
        (
            "ARDA_RETENTION_DAYS_HERMES_MESSAGES".to_string(),
            retention
                .get("hermes_messages_days")
                .and_then(Value::as_i64)
                .unwrap_or(30)
                .to_string(),
        ),
        (
            "ARDA_RETENTION_DAYS_HERMES_QUEUE".to_string(),
            retention
                .get("hermes_queue_days")
                .and_then(Value::as_i64)
                .unwrap_or(14)
                .to_string(),
        ),
        (
            "ARDA_RETENTION_DAYS_MNEMOSYNE_COMPACT".to_string(),
            retention
                .get("mnemosyne_compact_days")
                .and_then(Value::as_i64)
                .unwrap_or(180)
                .to_string(),
        ),
        (
            "ARDA_PRESSURE_MAX_TOTAL_KNOWN_WORK".to_string(),
            pressure
                .get("max_total_known_work")
                .and_then(Value::as_i64)
                .unwrap_or(120)
                .to_string(),
        ),
        (
            "ARDA_PRESSURE_MAX_PROJECTS_QUEUE_QUEUED".to_string(),
            pressure
                .get("max_projects_queue_queued")
                .and_then(Value::as_i64)
                .unwrap_or(20)
                .to_string(),
        ),
        (
            "ARDA_PRESSURE_MAX_OVERSIZE_COUNT".to_string(),
            pressure
                .get("max_oversize_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .to_string(),
        ),
        (
            "ARDA_PRESSURE_MAX_DISK_USED_PCT".to_string(),
            pressure
                .get("max_disk_used_pct")
                .and_then(Value::as_i64)
                .unwrap_or(92)
                .to_string(),
        ),
        (
            "ARDA_PRESSURE_MAX_INVALID_JSON".to_string(),
            pressure
                .get("max_invalid_json")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .to_string(),
        ),
        (
            "ARDA_PRESSURE_REQUIRE_COVERAGE_OK".to_string(),
            pressure
                .get("require_coverage_ok")
                .and_then(Value::as_bool)
                .unwrap_or(true)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_LOOKBACK_HOURS".to_string(),
            handoff
                .get("lookback_hours")
                .and_then(Value::as_i64)
                .unwrap_or(24)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_MAX_P95_MS".to_string(),
            handoff
                .get("max_p95_ms")
                .and_then(Value::as_i64)
                .unwrap_or(2500)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_MIN_SUCCESS_RATE".to_string(),
            handoff
                .get("min_success_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.95)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_MAX_DEFER_RATE".to_string(),
            handoff
                .get("max_defer_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.2)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_MAX_DUPLICATE_RATE".to_string(),
            handoff
                .get("max_duplicate_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.25)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_MAX_STUCK_INTENTS".to_string(),
            handoff
                .get("max_stuck_intents")
                .and_then(Value::as_i64)
                .unwrap_or(5)
                .to_string(),
        ),
        (
            "ARDA_HANDOFF_STUCK_MINUTES".to_string(),
            handoff
                .get("stuck_minutes")
                .and_then(Value::as_i64)
                .unwrap_or(30)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_MIN_ACK_SUCCESS_RATE".to_string(),
            autonomy
                .get("min_ack_success_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.95)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_MAX_P95_LATENCY_MS".to_string(),
            autonomy
                .get("max_p95_latency_ms")
                .and_then(Value::as_i64)
                .unwrap_or(2500)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_MAX_QUEUE_STALE_MINUTES".to_string(),
            autonomy
                .get("max_queue_stale_minutes")
                .and_then(Value::as_i64)
                .unwrap_or(30)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_MIN_SAFETY_SCORE".to_string(),
            autonomy
                .get("min_safety_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.65)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_REQUIRE_ATHENA_LOOKUP_BEFORE_ACTION".to_string(),
            autonomy
                .get("require_athena_lookup_before_action")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_MAX_ATHENA_LOOKUP_AGE_MINUTES".to_string(),
            autonomy
                .get("max_athena_lookup_age_minutes")
                .and_then(Value::as_i64)
                .unwrap_or(60)
                .to_string(),
        ),
        (
            "ARDA_AUTONOMY_REQUIRED_LIVE_SOCKETS".to_string(),
            required_live_sockets
                .iter()
                .filter_map(Value::as_str)
                .map(|v| replace_runtime(Some(v)))
                .collect::<Vec<_>>()
                .join(":"),
        ),
        (
            "ARDA_AUTONOMY_MAX_MISSING_REQUIRED_SOCKETS".to_string(),
            autonomy
                .get("max_missing_required_sockets")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .to_string(),
        ),
    ];
    if let Some(latency) = routing.get("latency_sla_ms").and_then(Value::as_i64) {
        runtime.push((
            "ARDA_ROUTE_LATENCY_SLA_MS".to_string(),
            latency.to_string(),
        ));
    }
    let mut runtime_lines = vec![format!("# Generated from {}", profile_label)];
    for (key, value) in &runtime {
        runtime_lines.push(format!("{key}={value}"));
    }
    write_text(&runtime_out, &(runtime_lines.join("\n") + "\n"))?;

    let secret_keys = integrations
        .get("secret_keys")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut env_lines = vec![format!("# Generated template from {}", profile_label)];
    for key in &secret_keys {
        if let Some(key) = key.as_str() {
            env_lines.push(format!("{key}="));
        }
    }
    write_text(&env_out, &(env_lines.join("\n") + "\n"))?;

    if apply_runtime {
        write_text(
            &root.join("config/runtime.env"),
            &fs::read_to_string(&runtime_out)?,
        )?;
    }
    let mut env_copy_message = Value::Null;
    if apply_env {
        let target = root.join("config/.env");
        if force_env || !target.exists() {
            write_text(&target, &fs::read_to_string(&env_out)?)?;
        } else {
            env_copy_message = json!("config/.env exists; set --force-env to overwrite");
        }
    }
    let mut control_policy_applied = false;
    if apply_control_policy {
        let script = root.join("scripts/control/export_control_plane_policy.sh");
        if script.exists() {
            let status = Command::new("bash").arg(script).current_dir(&root).status();
            control_policy_applied = matches!(status, Ok(s) if s.success());
        }
    }

    Ok(json!({
        "profile": profile.get("profile").and_then(Value::as_str).unwrap_or("unknown"),
        "profile_path": profile_label,
        "runtime_out": runtime_out.strip_prefix(&root).unwrap_or(&runtime_out).to_string_lossy(),
        "env_out": env_out.strip_prefix(&root).unwrap_or(&env_out).to_string_lossy(),
        "runtime_keys": runtime.len(),
        "secret_keys": secret_keys.len(),
        "control_policy_applied": control_policy_applied,
        "env_copy_message": env_copy_message,
    }))
}

#[allow(clippy::too_many_arguments)]
fn launch_preflight(
    report_path: &str,
    governance_path: &str,
    budget_path: &str,
    pressure_path: &str,
    topology_path: &str,
    degraded_agents: &str,
    enforce_swap: bool,
    enforce_exit: bool,
) -> anyhow::Result<Value> {
    let root = workspace_root();
    let report_path = root.join(report_path);
    let governance = read_json(&root.join(governance_path));
    let budget = read_json(&root.join(budget_path));
    let pressure = read_json(&root.join(pressure_path));
    let topology = read_json(&root.join(topology_path));
    let full_managed_agents = topology
        .get("local_control_plane")
        .and_then(|v| v.get("managed_agents"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "prometheus".to_string(),
                "manwe".to_string(),
                "hermes".to_string(),
                "hades".to_string(),
                "athena".to_string(),
                "mnemosyne".to_string(),
            ]
        });
    let degraded_agents = {
        let parsed = normalized_agents(degraded_agents);
        if parsed.is_empty() {
            vec!["prometheus".into(), "manwe".into(), "hermes".into()]
        } else {
            parsed
        }
    };
    let signals = governance
        .get("signals")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let goal = governance.get("goal").cloned().unwrap_or_else(|| json!({}));
    let thresholds = governance
        .get("control")
        .and_then(|v| v.get("thresholds"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let validators = governance
        .get("control")
        .and_then(|v| v.get("validators"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let human_augmentation = governance
        .get("control")
        .and_then(|v| v.get("human_augmentation"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let avg_joulework = signals
        .get("avg_joulework")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let avg_love_eq = signals
        .get("avg_love_eq")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let triad_pass_rate = signals
        .get("triad_pass_rate")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let autonomy_ready = goal
        .get("autonomy_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let triad_required_pass_rate = goal
        .get("triad_required_pass_rate")
        .and_then(Value::as_f64)
        .unwrap_or(0.45);
    let joulework_min = thresholds
        .get("joulework_min")
        .and_then(Value::as_f64)
        .unwrap_or(0.45);
    let love_equation_min = thresholds
        .get("love_equation_min")
        .and_then(Value::as_f64)
        .unwrap_or(0.45);
    let budget_summary = budget.get("summary").cloned().unwrap_or_else(|| json!({}));
    let user_plan_budget = budget
        .get("user_plan_budget")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let local_joule_pressure = budget_summary
        .get("local_joule_pressure")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let local_joule_usage_percent = user_plan_budget
        .get("local_joulework_usage_percent")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let pressure_status = pressure
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let pressure_violations = pressure
        .get("violations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let critical_pressure = pressure_violations
        .iter()
        .filter(|item| {
            item.get("severity")
                .and_then(Value::as_str)
                .unwrap_or("")
                .eq_ignore_ascii_case("critical")
        })
        .cloned()
        .collect::<Vec<_>>();
    let swaps = swap_entries();
    let swap_ok = !swaps.is_empty();
    let mut status = "ok".to_string();
    let mut lane = "full_local_control_plane".to_string();
    let mut allowed_agents = full_managed_agents.clone();
    let mut reasons = Vec::<Value>::new();

    if pressure_status == "error" {
        status = "block".into();
        lane = "blocked".into();
        allowed_agents.clear();
        reasons.push(json!({
            "id": "pressure_guard_unavailable",
            "severity": "critical",
            "message": format!("pressure guard receipt missing or unreadable at {}", root.join(pressure_path).display()),
        }));
    }
    if !critical_pressure.is_empty() {
        status = "block".into();
        lane = "blocked".into();
        allowed_agents.clear();
        reasons.extend(critical_pressure.iter().map(|item| {
            json!({
                "id": item.get("id").cloned().unwrap_or_else(|| json!("pressure_violation")),
                "severity": "critical",
                "message": item.get("message").cloned().unwrap_or_else(|| json!("critical pressure violation")),
                "actual": item.get("actual").cloned().unwrap_or(Value::Null),
                "threshold": item.get("threshold").cloned().unwrap_or(Value::Null),
            })
        }));
    }
    if enforce_swap && !swap_ok {
        status = "block".into();
        lane = "blocked".into();
        allowed_agents.clear();
        reasons.push(json!({
            "id": "swap_missing",
            "severity": "critical",
            "message": "no active swap detected; launch is blocked to avoid preventable memory crashes",
        }));
    }
    for (flag, id, message, actual, threshold) in [
        (
            !autonomy_ready,
            "autonomy_not_ready",
            "governance runtime is not autonomy-ready".to_string(),
            Value::Null,
            Value::Null,
        ),
        (
            triad_pass_rate < triad_required_pass_rate,
            "triad_below_threshold",
            "triad pass rate below required threshold".to_string(),
            json!(triad_pass_rate),
            json!(triad_required_pass_rate),
        ),
        (
            avg_love_eq < love_equation_min,
            "love_equation_below_threshold",
            "love equation below launch threshold".to_string(),
            json!(avg_love_eq),
            json!(love_equation_min),
        ),
        (
            avg_joulework < joulework_min,
            "joulework_below_threshold",
            "joulework efficiency below launch threshold".to_string(),
            json!(avg_joulework),
            json!(joulework_min),
        ),
    ] {
        if flag {
            status = "block".into();
            lane = "blocked".into();
            allowed_agents.clear();
            reasons.push(json!({
                "id": id,
                "severity": "critical",
                "message": message,
                "actual": actual,
                "threshold": threshold,
            }));
        }
    }
    if status != "block"
        && (local_joule_pressure
            || local_joule_usage_percent >= 100.0
            || pressure_status == "alert")
    {
        status = "degraded".into();
        lane = "control_plane_only".into();
        allowed_agents = degraded_agents.clone();
        if local_joule_pressure || local_joule_usage_percent >= 100.0 {
            reasons.push(json!({
                "id": "local_joule_pressure",
                "severity": "warn",
                "message": "local joule budget is under pressure; restrict launches to bounded control-plane agents",
                "actual": local_joule_usage_percent,
                "threshold": 100.0,
            }));
        }
        for item in &pressure_violations {
            if item
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("")
                .eq_ignore_ascii_case("warn")
            {
                reasons.push(json!({
                    "id": item.get("id").cloned().unwrap_or_else(|| json!("pressure_warn")),
                    "severity": "warn",
                    "message": item.get("message").cloned().unwrap_or_else(|| json!("pressure guard warning")),
                    "actual": item.get("actual").cloned().unwrap_or(Value::Null),
                    "threshold": item.get("threshold").cloned().unwrap_or(Value::Null),
                }));
            }
        }
    }
    let payload = json!({
        "ts_utc": now_utc(),
        "authority": "launch_preflight",
        "schema_version": "arda.launch-preflight.v1",
        "status": status,
        "lane": lane,
        "allowed_agents": allowed_agents,
        "degraded_agents": degraded_agents,
        "full_managed_agents": full_managed_agents,
        "signals": {
            "avg_joulework": avg_joulework,
            "avg_love_eq": avg_love_eq,
            "triad_pass_rate": triad_pass_rate,
            "autonomy_ready": autonomy_ready,
            "local_joule_pressure": local_joule_pressure,
            "local_joulework_usage_percent": local_joule_usage_percent,
            "pressure_guard_status": pressure_status,
            "swap_ok": swap_ok,
            "swap_entries": swaps,
        },
        "thresholds": {
            "joulework_min": joulework_min,
            "love_equation_min": love_equation_min,
            "triad_required_pass_rate": triad_required_pass_rate,
        },
        "validators": validators,
        "human_augmentation": human_augmentation,
        "sources": {
            "governance_runtime": root.join(governance_path).display().to_string(),
            "runtime_budget_policy": root.join(budget_path).display().to_string(),
            "pressure_guard": root.join(pressure_path).display().to_string(),
            "runtime_topology": root.join(topology_path).display().to_string(),
        },
        "reasons": reasons,
        "summary": {
            "message": match status.as_str() {
                "ok" => "launch permitted",
                "degraded" => "launch degraded to bounded control-plane lane",
                _ => "launch blocked by sovereign preflight",
            }
        }
    });
    write_text(
        &report_path,
        &(serde_json::to_string_pretty(&payload)? + "\n"),
    )?;
    if enforce_exit && payload.get("status").and_then(Value::as_str) == Some("block") {
        std::process::exit(2);
    }
    Ok(payload)
}
