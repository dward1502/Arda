#![cfg(feature = "full-cli")]
use super::super::*;
use regex::Regex;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) async fn handle(command: PipelineCommands) -> anyhow::Result<()> {
    let value = match command {
        PipelineCommands::ProjectTaskExecutor => project_task_executor()?,
        PipelineCommands::FlywheelPacketReadiness => flywheel_packet_readiness()?,
        PipelineCommands::FlywheelDispatch { task_id, write } => {
            flywheel_dispatch(task_id.as_deref(), write)?
        }
        PipelineCommands::FlywheelReviewReceipt {
            task_id,
            dispatch_receipt,
            changed_files,
            verification,
            diff_review,
            recommendation,
            notes,
            write,
        } => flywheel_review_receipt_command(
            task_id,
            dispatch_receipt.as_deref(),
            changed_files,
            verification,
            diff_review,
            recommendation,
            notes.as_deref(),
            write,
        )?,
        PipelineCommands::EmitAsyncUserIntakeTasks => emit_async_user_intake_tasks()?,
        PipelineCommands::RunAsyncUserIntakeExecutor => run_async_user_intake_executor()?,
        PipelineCommands::EmitHumanCorpusDigestTasks => emit_human_corpus_digest_tasks()?,
        PipelineCommands::ReconcileHumanCorpusDigestTasks => reconcile_human_corpus_digest_tasks()?,
        PipelineCommands::EmitSourceAbsorptionTasks => emit_source_absorption_tasks()?,
        PipelineCommands::RunSourceAbsorptionExecutor => run_source_absorption_executor()?,
        PipelineCommands::ReconcileSourceAbsorptionDownstream => {
            reconcile_source_absorption_downstream()?
        }
        PipelineCommands::PlatformOsMigrationExecutor => platform_os_migration_executor()?,
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

fn read_jsonl_objects(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
                .filter(|value| value.is_object())
                .collect()
        })
        .unwrap_or_default()
}

fn latest_tasks(path: &Path) -> BTreeMap<String, Value> {
    let mut latest = BTreeMap::new();
    for row in read_jsonl_objects(path) {
        let Some(task_id) = row.get("id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if !task_id.is_empty() {
            latest.insert(task_id.to_string(), row);
        }
    }
    latest
}

fn ensure_parent(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn ensure_file(path: &Path) -> anyhow::Result<()> {
    ensure_parent(path)?;
    if !path.exists() {
        fs::write(path, "")?;
    }
    Ok(())
}

fn write_json_pretty(path: &Path, value: &Value) -> anyhow::Result<()> {
    ensure_parent(path)?;
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> anyhow::Result<()> {
    ensure_file(path)?;
    let mut handle = OpenOptions::new().append(true).open(path)?;
    handle.write_all(serde_json::to_string(row)?.as_bytes())?;
    handle.write_all(b"\n")?;
    Ok(())
}

fn stable_task_id(parts: &[&str], prefix: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(parts.join(":").as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{prefix}{}", &digest[..12])
}

fn split_csv_field(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn task_terminal_success(task: Option<&Value>) -> bool {
    let Some(task) = task else {
        return false;
    };
    let status = task
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let result = task
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or_default();
    matches!(
        status,
        "completed" | "executed_verified" | "operator_approved_executed"
    ) && !matches!(
        result,
        "failed" | "blocked" | "cancelled" | "superseded_by_later_completion"
    )
}

fn is_flywheel_packet(task: &Value) -> bool {
    task.get("meta")
        .and_then(|meta| meta.get("origin"))
        .and_then(Value::as_str)
        == Some("flywheel_plan_packet")
}

fn flywheel_readiness_executor_rule_applies(task: &Value) -> bool {
    let meta = task.get("meta").unwrap_or(&Value::Null);
    let field = |name: &str| meta.get(name).and_then(Value::as_str).unwrap_or_default();
    field("origin") == "flywheel_plan_packet"
        && field("packet_id") == "F2"
        && field("risk") == "safe-local"
        && field("harness") == "arda-cli"
        && field("acceptance") == "read_only_packet_readiness_projection"
        && field("receipt_surface") == "core/state/flywheel_packet_runtime.json"
}

fn is_public_repo_skeleton_task(task: &Value) -> bool {
    let meta = task.get("meta").unwrap_or(&Value::Null);
    let field = |name: &str| meta.get(name).and_then(Value::as_str).unwrap_or_default();
    let title = task
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    field("origin") == "session_pivot"
        && title.contains("public repo")
        && (title.contains("skeleton") || title.contains("plan") || title.contains("review"))
        && (field("risk") == "safe-local"
            || field("risk") == "safe-public"
            || field("risk").is_empty())
        && (field("harness") == "arda-cli" || field("harness").is_empty())
        && (field("acceptance") == "public_repository_skeleton_creation"
            || field("acceptance").is_empty())
}
fn is_platform_os_migration_task(task: &Value) -> bool {
    let meta = task.get("meta").unwrap_or(&Value::Null);
    let field = |name: &str| meta.get(name).and_then(Value::as_str).unwrap_or_default();
    let title = task
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();

    (field("origin") == "session_pivot"
        || field("origin") == "core/projects/Plans/PLATFORM_OS.md"
        || field("origin") == "docs/operations/first-light-bootstrap-and-packaging-plan.md")
        && (title.contains("S1/G1")
            || title.contains("S1/G3")
            || title.contains("S2/G1")
            || title.contains("S3/G1")
            || title.contains("S4/G1"))
        && (field("risk") == "medium-structural"
            || field("risk") == "high-structural"
            || field("risk").is_empty())
        && (field("harness") == "arda-cli" || field("harness").is_empty())
        && (field("acceptance") == "platform_os_migration_operation"
            || field("acceptance").is_empty())
}
fn is_readiness_audit_task(task: &Value) -> bool {
    let meta = task.get("meta").unwrap_or(&Value::Null);
    let field = |name: &str| meta.get(name).and_then(Value::as_str).unwrap_or_default();
    let title = task
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();

    field("origin") == "docs/operations/first-light-bootstrap-and-packaging-plan.md"
        && title.contains("readiness audit")
        && (field("risk") == "safe-validation"
            || field("risk") == "medium-validation"
            || field("risk").is_empty())
        && (field("harness") == "arda-cli" || field("harness").is_empty())
        && (field("acceptance") == "system_readiness_audit" || field("acceptance").is_empty())
}
fn is_queue_management_task(task: &Value) -> bool {
    let meta = task.get("meta").unwrap_or(&Value::Null);
    let field = |name: &str| meta.get(name).and_then(Value::as_str).unwrap_or_default();
    let title = task
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    field("origin") == "queue_sequencer"
        && title.contains("queue")
        && (field("risk") == "safe-meta" || field("risk").is_empty())
        && (field("harness") == "arda-cli" || field("harness").is_empty())
        && (field("acceptance") == "queue_system_operation" || field("acceptance").is_empty())
}
fn is_agent_loop_contract_task(task: &Value) -> bool {
    let meta = task.get("meta").unwrap_or(&Value::Null);
    let field = |name: &str| meta.get(name).and_then(Value::as_str).unwrap_or_default();

    field("origin") == "session_plan_closeout"
        && (field("risk") == "safe-local" || field("risk").is_empty())
        && (field("harness") == "arda-cli" || field("harness").is_empty())
        && (field("acceptance") == "loop_contract_execution" || field("acceptance").is_empty())
}

fn classify_flywheel_packet(task: &Value, latest: &BTreeMap<String, Value>, root: &Path) -> Value {
    let task_id = task.get("id").and_then(Value::as_str).unwrap_or_default();
    let title = task.get("title").cloned().unwrap_or(Value::Null);
    let owner = task.get("owner").cloned().unwrap_or(Value::Null);
    let status = task
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
    let meta_obj = meta.as_object();
    let harness = meta
        .get("harness")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let plan = meta.get("plan").and_then(Value::as_str).unwrap_or_default();
    let depends_on = split_csv_field(meta.get("depends_on"));
    let mut missing_fields = Vec::new();
    for field in [
        "origin",
        "scope",
        "plan",
        "packet_id",
        "depends_on",
        "risk",
        "harness",
        "acceptance",
        "verify",
        "receipt_surface",
    ] {
        let missing = meta_obj
            .and_then(|obj| obj.get(field))
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty() && field != "depends_on");
        if missing {
            missing_fields.push(field.to_string());
        }
    }
    if harness == "hermes-agent-manwe" {
        let target_missing = meta
            .get("target_node")
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty());
        if target_missing {
            missing_fields.push("target_node".to_string());
        }
    }
    if !plan.is_empty() && !root.join(plan).exists() {
        missing_fields.push("plan_path_exists".to_string());
    }

    let dependency_status = depends_on
        .iter()
        .map(|dep| {
            let dep_task = latest.get(dep);
            json!({
                "id": dep,
                "present": dep_task.is_some(),
                "status": dep_task.and_then(|task| task.get("status")).cloned().unwrap_or(Value::Null),
                "result": dep_task.and_then(|task| task.get("result")).cloned().unwrap_or(Value::Null),
                "terminal_success": task_terminal_success(dep_task),
            })
        })
        .collect::<Vec<_>>();
    let dependency_blocked = dependency_status
        .iter()
        .any(|row| row.get("terminal_success").and_then(Value::as_bool) != Some(true));
    let risk = meta.get("risk").and_then(Value::as_str).unwrap_or_default();
    let human_approval_status = meta
        .get("human_approval")
        .and_then(|approval| approval.get("status"))
        .and_then(Value::as_str)
        .or_else(|| {
            meta.get("approval")
                .and_then(|approval| approval.get("status"))
                .and_then(Value::as_str)
        });
    let human_approved = matches!(human_approval_status, Some("approved"));
    let human_gated = !risk.is_empty() && risk != "safe-local" && !human_approved;
    let readiness = if matches!(
        status,
        "completed" | "cancelled" | "blocked" | "executed_verified" | "operator_approved_executed"
    ) {
        status.to_string()
    } else if !missing_fields.is_empty() {
        "missing_fields".to_string()
    } else if dependency_blocked {
        "dependency_blocked".to_string()
    } else if human_gated {
        "human_gated".to_string()
    } else {
        "ready".to_string()
    };

    json!({
        "task_id": task_id,
        "title": title,
        "owner": owner,
        "status": status,
        "readiness": readiness,
        "missing_fields": missing_fields,
        "dependencies": dependency_status,
        "plan": plan,
        "packet_id": meta.get("packet_id").cloned().unwrap_or(Value::Null),
        "scope": meta.get("scope").cloned().unwrap_or(Value::Null),
        "risk": risk,
        "human_approved": human_approved,
        "harness": harness,
        "target_node": meta.get("target_node").cloned().unwrap_or(Value::Null),
        "receipt_surface": meta.get("receipt_surface").cloned().unwrap_or(Value::Null),
    })
}

fn flywheel_owner_allowed(owner: &str) -> bool {
    matches!(
        owner,
        "prometheus" | "hades" | "apollo" | "hermes" | "manwe"
    )
}

fn flywheel_prompt(task: &Value, plan_text: &str) -> String {
    let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
    let plan_excerpt = plan_text.chars().take(12000).collect::<String>();
    format!(
        "You are executing an arda Flywheel work packet.\n\n\
Task ID: {}\n\
Title: {}\n\
Owner: {}\n\
Packet: {}\n\
Risk: {}\n\
Expected files: {}\n\
Acceptance: {}\n\
Verification: {}\n\
Receipt surface: {}\n\n\
Instructions:\n\
- Work only within the safe-local scope described by the packet.\n\
- Do not use credentials, deploy, spend money, restart services, publish externally, or perform destructive actions.\n\
- Keep changes scoped to expected files unless the repository proves a nearby file is required.\n\
- Run or state the verification commands before closeout.\n\
- Do not mark the task complete; completion requires a separate review receipt.\n\n\
Plan excerpt:\n{}\n",
        task.get("id").and_then(Value::as_str).unwrap_or_default(),
        task.get("title").and_then(Value::as_str).unwrap_or_default(),
        task.get("owner").and_then(Value::as_str).unwrap_or_default(),
        meta.get("packet_id").and_then(Value::as_str).unwrap_or_default(),
        meta.get("risk").and_then(Value::as_str).unwrap_or_default(),
        meta.get("expected_files")
            .and_then(Value::as_str)
            .unwrap_or("unspecified"),
        meta.get("acceptance").and_then(Value::as_str).unwrap_or_default(),
        meta.get("verify").and_then(Value::as_str).unwrap_or_default(),
        meta.get("receipt_surface")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        plan_excerpt
    )
}

fn sha1_12(input: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest[..12].to_string()
}

fn flywheel_dispatch_receipt(
    task: &Value,
    prompt_digest: &str,
    dry_run: bool,
    preflight: &Value,
    dispatch: &Value,
) -> Value {
    let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
    json!({
        "schema_version": "arda.flywheel.dispatch_receipt.v1",
        "ts_utc": now_utc(),
        "task_id": task.get("id").cloned().unwrap_or(Value::Null),
        "plan": meta.get("plan").cloned().unwrap_or(Value::Null),
        "packet_id": meta.get("packet_id").cloned().unwrap_or(Value::Null),
        "target_node": meta.get("target_node").cloned().unwrap_or(Value::Null),
        "harness": meta.get("harness").cloned().unwrap_or(Value::Null),
        "risk": meta.get("risk").cloned().unwrap_or(Value::Null),
        "manwe_route_intent": "hermes-agent bridge target uses configured Manwe OpenAI-compatible base_url/model routing",
        "prompt_sha1_12": prompt_digest,
        "dry_run": dry_run,
        "preflight": preflight,
        "dispatch": dispatch,
        "receipt_writer": "arda-cli pipeline flywheel-dispatch",
    })
}

fn flywheel_review_receipt(
    task: &Value,
    dispatch_receipt: Option<&str>,
    changed_files: &[String],
    verification: &[String],
    diff_review: &str,
    recommendation: &str,
    notes: Option<&str>,
    dry_run: bool,
) -> Value {
    let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
    json!({
        "schema_version": "arda.flywheel.review_receipt.v1",
        "ts_utc": now_utc(),
        "task_id": task.get("id").cloned().unwrap_or(Value::Null),
        "plan": meta.get("plan").cloned().unwrap_or(Value::Null),
        "packet_id": meta.get("packet_id").cloned().unwrap_or(Value::Null),
        "dispatch_receipt": dispatch_receipt,
        "changed_files": changed_files,
        "verification": verification,
        "diff_review": diff_review,
        "completion_recommendation": recommendation,
        "notes": notes,
        "dry_run": dry_run,
        "receipt_writer": "arda-cli pipeline flywheel-review-receipt",
    })
}

fn task_status_is_open_or_done(task: &Value) -> bool {
    matches!(
        task.get("status").and_then(Value::as_str),
        Some("queued" | "in_progress" | "completed" | "blocked")
    )
}

fn extract_json_payload(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({});
    }
    for line in trimmed.lines() {
        let stripped = line.trim_start();
        if stripped.starts_with('{') || stripped.starts_with('[') {
            if let Ok(value) = serde_json::from_str::<Value>(stripped) {
                return value;
            }
        }
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| json!({}))
}

#[derive(Clone)]
struct CommandEvent {
    display: Vec<String>,
    program: PathBuf,
    args: Vec<String>,
}

fn cli_command(args: &[&str]) -> anyhow::Result<CommandEvent> {
    Ok(CommandEvent {
        display: std::iter::once("arda-cli".to_string())
            .chain(args.iter().map(|arg| (*arg).to_string()))
            .collect(),
        program: env::current_exe()?,
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
    })
}

fn run_command_event(event: &CommandEvent, root: &Path) -> Value {
    let proc = Command::new(&event.program)
        .args(&event.args)
        .current_dir(root)
        .output();
    match proc {
        Ok(output) => json!({
            "cmd": event.display,
            "exit_code": output.status.code().unwrap_or(1),
            "stdout": String::from_utf8_lossy(&output.stdout).trim().chars().take(2000).collect::<String>(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim().chars().take(2000).collect::<String>(),
        }),
        Err(err) => json!({
            "cmd": event.display,
            "exit_code": 1,
            "stdout": "",
            "stderr": err.to_string(),
        }),
    }
}

fn community_policy_for(row: &Value, payload: &Value) -> Option<Value> {
    let source = row
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let channel = row
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or_default();
    payload
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|candidate| {
            candidate.get("provider").and_then(Value::as_str) == Some(source)
                && candidate
                    .get("channels")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .any(|entry| entry.as_str() == Some(channel))
        })
        .cloned()
}

fn confidence_ladder_for(url: &str, intake: &Value, brief: &Value) -> Value {
    let recent = intake
        .get("known_recent_sources")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for row in brief
        .get("comparison_set")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let title = row.get("title").and_then(Value::as_str).unwrap_or_default();
        if !title.is_empty() && url.trim_end_matches('/') == title.trim_end_matches('/') {
            let source_id = row
                .get("source_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let known = recent.get(&source_id).cloned().unwrap_or_else(|| json!({}));
            let route_to = intake
                .get("ladder")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|level| {
                    level.get("level").and_then(Value::as_str)
                        == known.get("status").and_then(Value::as_str)
                })
                .and_then(|level| level.get("route_to").and_then(Value::as_str))
                .unwrap_or("athena_digest_first");
            return json!({
                "source_id": source_id,
                "status": known.get("status").and_then(Value::as_str).unwrap_or("unknown_new_source"),
                "route_to": route_to,
                "reason": known.get("reason").cloned().unwrap_or(Value::Null),
            });
        }
    }
    json!({
        "source_id": Value::Null,
        "status": "unknown_new_source",
        "route_to": "athena_digest_first",
        "reason": "New external source without prior comparison posture; digest before promotion.",
    })
}

fn emit_async_user_intake_tasks() -> anyhow::Result<Value> {
    let root = workspace_root();
    let messages_path = root.join("data/hermes/messages.jsonl");
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/async_user_intake_queue.json");
    let community_sources = read_json(&root.join("core/state/hermes_community_sources.json"));
    let intake_ladder = read_json(&root.join("core/state/intake_confidence_ladder.json"));
    let external_brief = read_json(&root.join("core/state/external_absorption_brief.json"));
    let latest = &mut latest_tasks(&queue_path);
    let url_re = Regex::new(r#"https?://[^\s<>()`"']+"#)?;
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for row in read_jsonl_objects(&messages_path) {
        if row.get("direction").and_then(Value::as_str) != Some("inbound") {
            continue;
        }
        let content = row
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        for capture in url_re.find_iter(&content) {
            let url = capture
                .as_str()
                .trim_end_matches(&['.', ',', ')'][..])
                .to_string();
            if !seen.insert(url.clone()) {
                continue;
            }
            candidates.push(json!({
                "url": url,
                "source": row.get("source").cloned().unwrap_or(Value::Null),
                "sender": row.get("sender").cloned().unwrap_or(Value::Null),
                "channel": row.get("channel").cloned().unwrap_or(Value::Null),
                "received_at_utc": row.get("received_at_utc").cloned().unwrap_or(Value::Null),
                "classification": row.get("classification").cloned().unwrap_or_else(|| json!({})),
                "community_policy": community_policy_for(&row, &community_sources).unwrap_or_else(|| json!({})),
                "intake_ladder": confidence_ladder_for(&url, &intake_ladder, &external_brief),
                "content_preview": content.chars().take(240).collect::<String>(),
            }));
        }
    }
    candidates.sort_by(|a, b| {
        b.get("received_at_utc")
            .and_then(Value::as_str)
            .cmp(&a.get("received_at_utc").and_then(Value::as_str))
    });

    let mut emitted = Vec::new();
    let mut already_open = Vec::new();
    for candidate in &candidates {
        let url = candidate
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        let task_id = stable_task_id(&[url], "tsk_async_intake_");
        if let Some(existing) = latest.get(&task_id) {
            if task_status_is_open_or_done(existing) {
                already_open.push(json!({
                    "task_id": task_id,
                    "url": url,
                    "status": existing.get("status").cloned().unwrap_or(Value::Null),
                }));
                continue;
            }
        }
        let community_policy = candidate
            .get("community_policy")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let intake = candidate
            .get("intake_ladder")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let task = json!({
            "id": task_id,
            "title": format!("Async intake for inbound source {}", url),
            "owner": "athena",
            "priority": "high",
            "status": "queued",
            "queued_at_utc": now_utc(),
            "notes": format!(
                "Auto-emitted from HERMES inbound async intake. Source `{}` sender `{}` dropped `{}` during conversation; process it in ATHENA background without blocking foreground chat.",
                candidate.get("source").and_then(Value::as_str).unwrap_or_default(),
                candidate.get("sender").and_then(Value::as_str).unwrap_or_default(),
                url
            ),
            "glyphs": ["↝", "◈"],
            "meta": {
                "origin": "async_user_intake",
                "scope": "inbound_link_handoff",
                "url": url,
                "source": candidate.get("source").cloned().unwrap_or(Value::Null),
                "sender": candidate.get("sender").cloned().unwrap_or(Value::Null),
                "channel": candidate.get("channel").cloned().unwrap_or(Value::Null),
                "received_at_utc": candidate.get("received_at_utc").cloned().unwrap_or(Value::Null),
                "community_signal_class": community_policy.get("signal_class").cloned().unwrap_or(Value::Null),
                "community_route_to": community_policy.get("route_to").cloned().unwrap_or(Value::Null),
                "intake_confidence_status": intake.get("status").cloned().unwrap_or(Value::Null),
                "intake_confidence_route": intake.get("route_to").cloned().unwrap_or(Value::Null),
                "comparison_source_id": intake.get("source_id").cloned().unwrap_or(Value::Null),
            }
        });
        append_jsonl(&queue_path, &task)?;
        latest.insert(task_id.clone(), task.clone());
        emitted.push(task);
    }

    let payload = json!({
        "schema_version": "arda.async-user-intake-queue.v1",
        "generated_at_utc": now_utc(),
        "authority": "hermes_messages + project_task_queue",
        "summary": {
            "candidates_total": candidates.len(),
            "emitted_total": emitted.len(),
            "already_open_total": already_open.len(),
        },
        "candidates": candidates.into_iter().take(20).collect::<Vec<_>>(),
        "emitted_tasks": emitted,
        "already_open": already_open,
    });
    write_json_pretty(&out, &payload)?;
    Ok(
        json!({"out": "core/state/async_user_intake_queue.json", "emitted_total": payload["summary"]["emitted_total"]}),
    )
}

fn run_async_user_intake_executor() -> anyhow::Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/async_user_intake_runtime.json");
    let ladder = read_json(&root.join("core/state/intake_confidence_ladder.json"));
    let ladder_routes = ladder
        .get("ladder")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some((
                item.get("level")?.as_str()?.to_string(),
                item.get("route_to")?.as_str()?.to_string(),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut latest = latest_tasks(&queue_path);
    let queued = latest
        .values()
        .filter(|task| {
            task.get("status").and_then(Value::as_str) == Some("queued")
                && task
                    .get("meta")
                    .and_then(|meta| meta.get("origin"))
                    .and_then(Value::as_str)
                    == Some("async_user_intake")
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut processed = Vec::new();
    let mut failed = Vec::new();
    let mut route_summary = BTreeMap::<String, usize>::new();

    for task in queued.iter() {
        let task_id = task.get("id").and_then(Value::as_str).unwrap_or_default();
        if latest
            .get(task_id)
            .and_then(|row| row.get("status"))
            .and_then(Value::as_str)
            != Some("queued")
        {
            continue;
        }
        let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
        let url = meta.get("url").and_then(Value::as_str).unwrap_or_default();
        if url.is_empty() {
            failed.push(json!({"task_id": task_id, "reason": "missing_url"}));
            continue;
        }
        let intake_status = meta
            .get("intake_confidence_status")
            .and_then(Value::as_str)
            .unwrap_or("unknown_new_source")
            .to_string();
        let intake_route = meta
            .get("intake_confidence_route")
            .and_then(Value::as_str)
            .filter(|route| !route.is_empty())
            .map(str::to_string)
            .or_else(|| ladder_routes.get(&intake_status).cloned())
            .unwrap_or_else(|| "athena_digest_first".to_string());

        let ingest = run_command_event(
            &cli_command(&[
                "athena",
                "ingest",
                url,
                "--submitted-by",
                "async_user_intake",
                "--task-context",
                "async inbound link handoff",
            ])?,
            &root,
        );
        let ingest_json = extract_json_payload(
            ingest
                .get("stdout")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        let source_id = ingest_json
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if ingest.get("exit_code").and_then(Value::as_i64) != Some(0) || source_id.is_empty() {
            failed.push(json!({
                "task_id": task_id,
                "url": url,
                "stage": "ingest",
                "stderr": ingest.get("stderr").cloned().unwrap_or(Value::Null),
            }));
            continue;
        }

        let deep = run_command_event(
            &cli_command(&[
                "athena",
                "deep",
                &source_id,
                "--reason",
                "async inbound link handoff",
            ])?,
            &root,
        );
        let deep_json = extract_json_payload(
            deep.get("stdout")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        let policy_readiness = deep_json
            .get("policy_readiness")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                deep_json
                    .get("deep")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.get("policy_readiness"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        if deep.get("exit_code").and_then(Value::as_i64) != Some(0) {
            failed.push(json!({
                "task_id": task_id,
                "url": url,
                "source_id": source_id,
                "stage": "deep",
                "stderr": deep.get("stderr").cloned().unwrap_or(Value::Null),
            }));
            continue;
        }

        let planning_skip_reason = if matches!(
            intake_route.as_str(),
            "compare_and_extract"
                | "productize_operator_surface"
                | "athena_ingestion_policy"
                | "manwe_model_strategy"
                | "fetch_retry_or_hold"
        ) {
            Some(format!("route:{}", intake_route))
        } else if matches!(
            policy_readiness.as_deref(),
            Some("reference_only" | "reject" | "rejected" | "observed_only")
        ) {
            Some(format!(
                "policy:{}",
                policy_readiness.clone().unwrap_or_default()
            ))
        } else {
            None
        };
        let planning = if planning_skip_reason.is_some() {
            json!({
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
            })
        } else {
            run_command_event(
                &cli_command(&[
                    "athena",
                    "generate-planning-tasks",
                    &source_id,
                    "--limit",
                    "8",
                ])?,
                &root,
            )
        };
        let planning_json = extract_json_payload(
            planning
                .get("stdout")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );

        let mut completion = task.clone();
        completion["status"] = json!("completed");
        completion["result"] = json!("completed");
        completion["completed_at_utc"] = json!(now_utc());
        completion["notes"] = if let Some(reason) = &planning_skip_reason {
            json!(format!(
                "Completed by async user intake executor. ATHENA ingested `{}` as `{}`, deepened it, and held follow-on planning under `{}` / `{}` ({}).",
                url, source_id, intake_status, intake_route, reason
            ))
        } else {
            json!(format!(
                "Completed by async user intake executor. ATHENA ingested `{}` as `{}`, deepened it, and generated planning tasks from the resulting evidence.",
                url, source_id
            ))
        };
        append_jsonl(&queue_path, &completion)?;
        latest.insert(task_id.to_string(), completion);
        *route_summary.entry(intake_route.clone()).or_insert(0) += 1;
        processed.push(json!({
            "task_id": task_id,
            "url": url,
            "source_id": source_id,
            "intake_confidence_status": intake_status,
            "intake_confidence_route": intake_route,
            "ingest": {"exit_code": ingest.get("exit_code").cloned().unwrap_or(Value::Null)},
            "deep": {
                "exit_code": deep.get("exit_code").cloned().unwrap_or(Value::Null),
                "policy_readiness": policy_readiness,
            },
            "planning": {
                "exit_code": planning.get("exit_code").cloned().unwrap_or(Value::Null),
                "skipped": planning_skip_reason.is_some(),
                "skip_reason": planning_skip_reason,
                "queued_tasks": planning_json.get("queued_tasks").cloned().unwrap_or(Value::Null),
                "stderr": planning.get("stderr").and_then(Value::as_str).unwrap_or_default().chars().take(300).collect::<String>(),
            },
        }));
    }

    let payload = json!({
        "schema_version": "arda.async-user-intake-runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "project_task_queue + athena_cli_handoff",
        "summary": {
            "queued_total": queued.len(),
            "processed_total": processed.len(),
            "failed_total": failed.len(),
        },
        "route_summary": route_summary,
        "processed": processed,
        "failed": failed,
    });
    write_json_pretty(&out, &payload)?;
    Ok(
        json!({"out": "core/state/async_user_intake_runtime.json", "processed_total": payload["summary"]["processed_total"]}),
    )
}

fn emit_human_corpus_digest_tasks() -> anyhow::Result<Value> {
    let root = workspace_root();
    let plan_path = root.join("core/state/human_corpus_digest_plan.json");
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/human_corpus_digest_tasks.json");
    let plan = read_json(&plan_path);
    let groups = plan
        .get("plan_groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let latest = &mut latest_tasks(&queue_path);
    let mut emitted = Vec::new();
    let mut already_open = Vec::new();

    for group in groups.iter() {
        let group_id = group
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if group_id.is_empty() {
            continue;
        }
        let task_id = stable_task_id(&[group_id], "tsk_humancorpus_");
        if let Some(existing) = latest.get(&task_id) {
            if task_status_is_open_or_done(existing) {
                already_open.push(json!({
                    "task_id": task_id,
                    "group_id": group_id,
                    "status": existing.get("status").cloned().unwrap_or(Value::Null),
                }));
                continue;
            }
        }
        let lane = group
            .get("lane")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let sources = group
            .get("sources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|row| row.get("source_id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        let task = json!({
            "id": task_id,
            "title": group.get("task_title").cloned().unwrap_or(Value::Null),
            "owner": group.get("owner").cloned().unwrap_or(Value::Null),
            "priority": if matches!(lane, "suite_spec" | "architecture") { "high" } else { "medium" },
            "status": "queued",
            "queued_at_utc": now_utc(),
            "notes": "Auto-emitted from the human corpus digest plan. Convert the grouped human/Numenor evidence into a bounded extraction brief or sovereign contract, not direct implementation doctrine.",
            "glyphs": ["↝", "◈"],
            "meta": {
                "origin": "human_corpus_digest_plan",
                "scope": "grouped_extraction",
                "group_id": group_id,
                "lane": group.get("lane").cloned().unwrap_or(Value::Null),
                "plan_id": group_id,
                "plan_surface": "core/state/human_corpus_digest_plan.json",
                "plan_type": "human_corpus_digest_group",
                "executor_rule_id": "human_corpus_digest_handoff",
                "receipt_surface": "core/state/athena_digest_pipeline.json",
                "source_ids": sources,
            }
        });
        append_jsonl(&queue_path, &task)?;
        latest.insert(task_id.clone(), task.clone());
        emitted.push(task);
    }

    let payload = json!({
        "schema_version": "arda.human-corpus-digest-tasks.v1",
        "generated_at_utc": now_utc(),
        "authority": "human_corpus_digest_plan + project_task_queue",
        "summary": {
            "groups_total": groups.len(),
            "emitted_total": emitted.len(),
            "already_open_total": already_open.len(),
        },
        "emitted_tasks": emitted,
        "already_open": already_open,
    });
    write_json_pretty(&out, &payload)?;
    Ok(
        json!({"out": "core/state/human_corpus_digest_tasks.json", "emitted_total": payload["summary"]["emitted_total"]}),
    )
}

fn reconcile_human_corpus_digest_tasks() -> anyhow::Result<Value> {
    let root = workspace_root();
    let registry = read_json(&root.join("core/state/human_corpus_extraction_registry.json"));
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/human_corpus_digest_reconciliation.json");
    let latest = &mut latest_tasks(&queue_path);
    let mut completed = Vec::new();

    for group in registry
        .get("groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let group_id = group
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if group_id.is_empty() {
            continue;
        }
        let task_id = stable_task_id(&[group_id], "tsk_humancorpus_");
        let Some(current) = latest.get(&task_id).cloned() else {
            continue;
        };
        if current.get("status").and_then(Value::as_str) != Some("queued") {
            continue;
        }
        let completion = json!({
            "id": task_id,
            "title": current.get("title").cloned().unwrap_or(Value::Null),
            "owner": current.get("owner").cloned().unwrap_or(Value::Null),
            "priority": current.get("priority").cloned().unwrap_or(Value::Null),
            "status": "completed",
            "queued_at_utc": current.get("queued_at_utc").cloned().unwrap_or(Value::Null),
            "completed_at_utc": now_utc(),
            "result": "completed",
            "notes": format!(
                "Closed after bounded extraction surface was emitted at {}. This wave formalized the source group into contract/crate candidates without claiming direct implementation authority.",
                group.get("group_path").and_then(Value::as_str).unwrap_or_default()
            ),
            "glyphs": current.get("glyphs").cloned().unwrap_or_else(|| json!(["↝", "◈"])),
            "meta": {
                "origin": current.get("meta").and_then(|meta| meta.get("origin")).cloned().unwrap_or(Value::Null),
                "scope": current.get("meta").and_then(|meta| meta.get("scope")).cloned().unwrap_or(Value::Null),
                "group_id": current.get("meta").and_then(|meta| meta.get("group_id")).cloned().unwrap_or(Value::Null),
                "lane": current.get("meta").and_then(|meta| meta.get("lane")).cloned().unwrap_or(Value::Null),
                "plan_id": current.get("meta").and_then(|meta| meta.get("plan_id")).cloned().unwrap_or(Value::Null),
                "plan_surface": current.get("meta").and_then(|meta| meta.get("plan_surface")).cloned().unwrap_or(Value::Null),
                "plan_type": current.get("meta").and_then(|meta| meta.get("plan_type")).cloned().unwrap_or(Value::Null),
                "executor_rule_id": current.get("meta").and_then(|meta| meta.get("executor_rule_id")).cloned().unwrap_or(Value::Null),
                "receipt_surface": "core/state/athena_digest_pipeline.json",
                "source_ids": current.get("meta").and_then(|meta| meta.get("source_ids")).cloned().unwrap_or_else(|| json!([])),
                "group_contract_surface": group.get("group_path").cloned().unwrap_or(Value::Null),
                "execution_stage": "contract_ready",
            }
        });
        append_jsonl(&queue_path, &completion)?;
        latest.insert(task_id.clone(), completion.clone());
        completed.push(json!({
            "task_id": task_id,
            "group_id": group_id,
            "group_path": group.get("group_path").cloned().unwrap_or(Value::Null),
        }));
    }

    let payload = json!({
        "schema_version": "arda.human-corpus-digest-reconciliation.v1",
        "generated_at_utc": now_utc(),
        "authority": "human_corpus_extraction_registry + project_task_queue",
        "summary": {"completed_total": completed.len()},
        "completed": completed,
    });
    write_json_pretty(&out, &payload)?;
    Ok(
        json!({"out": "core/state/human_corpus_digest_reconciliation.json", "completed_total": payload["summary"]["completed_total"]}),
    )
}

fn emit_source_absorption_tasks() -> anyhow::Result<Value> {
    let root = workspace_root();
    let pipeline = read_json(&root.join("core/state/source_absorption_pipeline.json"));
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/source_absorption_execution.json");
    let latest = &mut latest_tasks(&queue_path);
    let candidates = pipeline
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut emitted = Vec::new();
    let mut already_open = Vec::new();
    let mut skipped = Vec::new();

    for candidate in &candidates {
        if candidate.get("disposition").and_then(Value::as_str) != Some("promote_now") {
            continue;
        }
        let source_id = candidate
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let title = candidate
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(source_id);
        let targets = candidate
            .get("subsystem_targets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if source_id.is_empty() || targets.is_empty() {
            skipped.push(json!({
                "source_id": source_id,
                "title": title,
                "reason": "missing_source_or_subsystem_targets",
            }));
            continue;
        }
        for subsystem in targets.iter().take(2) {
            let subsystem_name = subsystem.as_str().unwrap_or_default();
            let task_id = stable_task_id(&[source_id, subsystem_name], "tsk_absorb_");
            if let Some(existing) = latest.get(&task_id) {
                if task_status_is_open_or_done(existing) {
                    already_open.push(json!({
                        "task_id": task_id,
                        "source_id": source_id,
                        "subsystem": subsystem_name,
                        "status": existing.get("status").cloned().unwrap_or(Value::Null),
                    }));
                    continue;
                }
            }
            let task = json!({
                "id": task_id,
                "title": format!("Absorb source {} into {}: {}", source_id, subsystem_name, title),
                "owner": subsystem_name,
                "priority": "high",
                "status": "queued",
                "queued_at_utc": now_utc(),
                "notes": format!("Auto-emitted from the source absorption pipeline. Promote `{}` into the `{}` subsystem using the source disposition, rationale, and next action already classified in sovereign state.", title, subsystem_name),
                "glyphs": ["↝", "◈"],
                "meta": {
                    "origin": "source_absorption_pipeline",
                    "scope": "auto_emission",
                    "source_id": source_id,
                    "domain": candidate.get("domain").cloned().unwrap_or(Value::Null),
                    "disposition": candidate.get("disposition").cloned().unwrap_or(Value::Null),
                    "subsystem": subsystem_name,
                }
            });
            append_jsonl(&queue_path, &task)?;
            latest.insert(task_id.clone(), task.clone());
            emitted.push(task);
        }
    }

    let promote_now_total = candidates
        .iter()
        .filter(|row| row.get("disposition").and_then(Value::as_str) == Some("promote_now"))
        .count();
    let payload = json!({
        "schema_version": "arda.source-absorption-execution.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_pipeline + project_task_queue",
        "source_surface": "core/state/source_absorption_pipeline.json",
        "summary": {
            "promote_now_candidates_total": promote_now_total,
            "emitted_total": emitted.len(),
            "already_open_total": already_open.len(),
            "skipped_total": skipped.len(),
        },
        "emitted_tasks": emitted,
        "already_open": already_open,
        "skipped": skipped,
    });
    write_json_pretty(&out, &payload)?;
    Ok(
        json!({"out": "core/state/source_absorption_execution.json", "emitted_total": payload["summary"]["emitted_total"]}),
    )
}

fn run_source_absorption_executor() -> anyhow::Result<Value> {
    let root = workspace_root();
    let portfolio = read_json(&root.join("core/state/source_absorption_portfolio.json"));
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/source_absorption_autopilot.json");
    let latest = &mut latest_tasks(&queue_path);
    let mut emitted = Vec::new();
    let mut completed = Vec::new();
    let mut skipped = Vec::new();
    let mut cancelled = Vec::new();

    for source in portfolio
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let source_id = source
            .get("source_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let title = source
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(source_id);
        let brief = source
            .get("implementation_brief")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let implications = brief
            .get("implementation_implications")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let templates = source
            .get("downstream_task_templates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let pattern = source
            .get("absorption_pattern")
            .cloned()
            .unwrap_or(Value::Null);
        let source_latest = latest.clone();
        let absorption_tasks = source_latest
            .values()
            .filter(|row| {
                row.get("meta")
                    .and_then(|meta| meta.get("origin"))
                    .and_then(Value::as_str)
                    == Some("source_absorption_pipeline")
                    && row
                        .get("meta")
                        .and_then(|meta| meta.get("source_id"))
                        .and_then(Value::as_str)
                        == Some(source_id)
                    && matches!(
                        row.get("status").and_then(Value::as_str),
                        Some("queued" | "completed")
                    )
            })
            .cloned()
            .collect::<Vec<_>>();
        let stale = source_latest
            .values()
            .filter(|row| {
                row.get("meta")
                    .and_then(|meta| meta.get("origin"))
                    .and_then(Value::as_str)
                    == Some("source_absorption_executor")
                    && row
                        .get("meta")
                        .and_then(|meta| meta.get("source_id"))
                        .and_then(Value::as_str)
                        == Some(source_id)
                    && row.get("status").and_then(Value::as_str) == Some("queued")
                    && row
                        .get("meta")
                        .and_then(|meta| meta.get("absorption_pattern"))
                        != Some(&pattern)
            })
            .cloned()
            .collect::<Vec<_>>();
        for row in stale {
            let cancellation = json!({
                "id": row.get("id").cloned().unwrap_or(Value::Null),
                "title": row.get("title").cloned().unwrap_or(Value::Null),
                "owner": row.get("owner").cloned().unwrap_or(Value::Null),
                "priority": row.get("priority").cloned().unwrap_or(Value::Null),
                "status": "cancelled",
                "queued_at_utc": row.get("queued_at_utc").cloned().unwrap_or(Value::Null),
                "completed_at_utc": now_utc(),
                "result": "cancelled",
                "notes": format!(
                    "Cancelled by source absorption executor reconciliation. Queued downstream task reflected absorption pattern `{}`, but current source portfolio requires `{}` for `{}`.",
                    row.get("meta").and_then(|meta| meta.get("absorption_pattern")).and_then(Value::as_str).unwrap_or_default(),
                    pattern.as_str().unwrap_or_default(),
                    title
                ),
                "glyphs": row.get("glyphs").cloned().unwrap_or_else(|| json!(["↝", "◈"])),
                "meta": row.get("meta").cloned().unwrap_or_else(|| json!({})),
            });
            append_jsonl(&queue_path, &cancellation)?;
            if let Some(id) = cancellation.get("id").and_then(Value::as_str) {
                latest.insert(id.to_string(), cancellation.clone());
            }
            cancelled.push(json!({
                "task_id": cancellation.get("id").cloned().unwrap_or(Value::Null),
                "owner": cancellation.get("owner").cloned().unwrap_or(Value::Null),
                "title": cancellation.get("title").cloned().unwrap_or(Value::Null),
            }));
        }

        for anchor in absorption_tasks {
            let task_id = anchor.get("id").and_then(Value::as_str).unwrap_or_default();
            let Some(task) = latest.get(task_id).cloned() else {
                continue;
            };
            if !matches!(
                task.get("status").and_then(Value::as_str),
                Some("queued" | "completed")
            ) {
                continue;
            }
            let emitter_owner = task
                .get("owner")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut source_emitted = 0usize;
            let mut source_skipped = 0usize;
            for template in &templates {
                if template.get("emitter_owner").and_then(Value::as_str) != Some(emitter_owner) {
                    continue;
                }
                let downstream_id = stable_task_id(
                    &[
                        source_id,
                        template
                            .get("owner")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        template
                            .get("slug")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    ],
                    "tsk_absorb_exec_",
                );
                let existing_open = latest
                    .get(&downstream_id)
                    .map(task_status_is_open_or_done)
                    .unwrap_or(false);
                if existing_open {
                    source_skipped += 1;
                    skipped.push(json!({
                        "task_id": downstream_id,
                        "status": latest.get(&downstream_id).and_then(|row| row.get("status")).cloned().unwrap_or(Value::Null),
                        "reason": "already_present",
                    }));
                    continue;
                }
                let implication_summary = implications
                    .iter()
                    .take(2)
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ");
                let downstream = json!({
                    "id": downstream_id,
                    "title": template.get("title").cloned().unwrap_or(Value::Null),
                    "owner": template.get("owner").cloned().unwrap_or(Value::Null),
                    "priority": template.get("priority").cloned().unwrap_or_else(|| json!("high")),
                    "status": "queued",
                    "queued_at_utc": now_utc(),
                    "notes": format!(
                        "{} Source absorption anchor: `{}`. Implications: {}",
                        template.get("notes").and_then(Value::as_str).unwrap_or_default(),
                        title,
                        if implication_summary.is_empty() { "bounded implementation follow-through required." } else { &implication_summary }
                    ),
                    "glyphs": ["↝", "◈"],
                    "meta": {
                        "origin": "source_absorption_executor",
                        "scope": "downstream_emission",
                        "source_id": source_id,
                        "absorption_pattern": pattern.clone(),
                        "source_absorption_anchor_task": task_id,
                        "emitter_owner": emitter_owner,
                    }
                });
                append_jsonl(&queue_path, &downstream)?;
                latest.insert(downstream_id.clone(), downstream.clone());
                emitted.push(downstream);
                source_emitted += 1;
            }

            if task.get("status").and_then(Value::as_str) == Some("queued") {
                let completion = json!({
                    "id": task_id,
                    "title": task.get("title").cloned().unwrap_or(Value::Null),
                    "owner": task.get("owner").cloned().unwrap_or(Value::Null),
                    "priority": task.get("priority").cloned().unwrap_or(Value::Null),
                    "status": "completed",
                    "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
                    "completed_at_utc": now_utc(),
                    "result": "completed",
                    "notes": format!(
                        "Completed by source absorption executor using `core/state/source_absorption_portfolio.json`. Emitted {} downstream task(s) and skipped {} already-open task(s) for absorbed source `{}`.",
                        source_emitted, source_skipped, title
                    ),
                    "glyphs": task.get("glyphs").cloned().unwrap_or_else(|| json!(["↝", "◈"])),
                    "meta": task.get("meta").cloned().unwrap_or_else(|| json!({})),
                });
                append_jsonl(&queue_path, &completion)?;
                latest.insert(task_id.to_string(), completion.clone());
                completed.push(json!({
                    "task_id": task_id,
                    "owner": completion.get("owner").cloned().unwrap_or(Value::Null),
                    "title": completion.get("title").cloned().unwrap_or(Value::Null),
                }));
            }
        }
    }

    let remaining = latest
        .values()
        .filter(|row| {
            row.get("status").and_then(Value::as_str) == Some("queued")
                && row
                    .get("meta")
                    .and_then(|meta| meta.get("origin"))
                    .and_then(Value::as_str)
                    == Some("source_absorption_pipeline")
        })
        .count();
    let payload = json!({
        "schema_version": "arda.source-absorption-autopilot.v1",
        "generated_at_utc": now_utc(),
        "authority": "source_absorption_portfolio + project_task_queue",
        "summary": {
            "completed_absorption_tasks_total": completed.len(),
            "downstream_emitted_total": emitted.len(),
            "downstream_skipped_total": skipped.len(),
            "downstream_cancelled_total": cancelled.len(),
            "remaining_absorption_queue_total": remaining,
        },
        "completed_absorption_tasks": completed,
        "downstream_emitted": emitted,
        "downstream_skipped": skipped,
        "downstream_cancelled": cancelled,
    });
    write_json_pretty(&out, &payload)?;
    Ok(json!({
        "out": "core/state/source_absorption_autopilot.json",
        "completed_total": payload["summary"]["completed_absorption_tasks_total"],
        "downstream_emitted_total": payload["summary"]["downstream_emitted_total"]
    }))
}

fn reconcile_source_absorption_downstream() -> anyhow::Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/source_absorption_downstream_reconciliation.json");
    let evidence = json!({
        "crawl4ai_runtime_contract": read_json(&root.join("core/state/crawl4ai_runtime_contract.json")),
        "scrapling_runtime_contract": read_json(&root.join("core/state/scrapling_runtime_contract.json")),
        "source_ecosystem_registry": read_json(&root.join("core/state/source_ecosystem_registry.json")),
        "community_signal_intake": read_json(&root.join("core/state/community_signal_intake.json")),
        "research_workflow_contract": read_json(&root.join("core/state/research_workflow_contract.json")),
        "search_runtime_contract": read_json(&root.join("core/state/search_runtime_contract.json")),
    });
    let latest = latest_tasks(&queue_path);
    let mut completed = Vec::new();
    let mut skipped = Vec::new();

    for task in latest.values() {
        if task.get("status").and_then(Value::as_str) != Some("queued")
            || task
                .get("meta")
                .and_then(|meta| meta.get("origin"))
                .and_then(Value::as_str)
                != Some("source_absorption_executor")
        {
            continue;
        }
        let source_id = task
            .get("meta")
            .and_then(|meta| meta.get("source_id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let (ok, surfaces): (bool, Vec<&str>) = match source_id {
            "src_d46d1480" => {
                let contract = &evidence["crawl4ai_runtime_contract"];
                let summary = contract
                    .get("summary")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let posture = contract
                    .get("operating_posture")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                (
                    summary
                        .get("active_in_system")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        && summary
                            .get("runtime_ok")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        && posture
                            .get("live_primary_designated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        && posture
                            .get("service_running")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        && posture
                            .get("service_ready")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    vec!["core/state/crawl4ai_runtime_contract.json"],
                )
            }
            "src_df11630e" => {
                let contract = &evidence["scrapling_runtime_contract"];
                let summary = contract
                    .get("summary")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                (
                    summary
                        .get("successful_receipts_total")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                        > 0
                        && summary
                            .get("promotion_gates_passed")
                            .and_then(Value::as_i64)
                            .unwrap_or(0)
                            >= 2,
                    vec!["core/state/scrapling_runtime_contract.json"],
                )
            }
            "src_33fa61b2" | "src_ca2f031e" => {
                let sources = evidence["source_ecosystem_registry"]
                    .get("sources")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                (
                    sources
                        .iter()
                        .filter_map(|row| row.get("source_id").and_then(Value::as_str))
                        .any(|id| id == source_id),
                    vec!["core/state/source_ecosystem_registry.json"],
                )
            }
            "src_dc355aed" => (
                evidence["community_signal_intake"]
                    .get("campaign")
                    .and_then(|row| row.get("source_id"))
                    .and_then(Value::as_str)
                    == Some(source_id),
                vec!["core/state/community_signal_intake.json"],
            ),
            "src_f226959a" => {
                let workflow = &evidence["research_workflow_contract"];
                (
                    workflow
                        .get("campaign")
                        .and_then(|row| row.get("source_id"))
                        .and_then(Value::as_str)
                        == Some(source_id)
                        && workflow
                            .get("summary")
                            .and_then(|row| row.get("apollo_stages_total"))
                            .and_then(Value::as_i64)
                            .unwrap_or(0)
                            > 0,
                    vec!["core/state/research_workflow_contract.json"],
                )
            }
            "src_86fa4360" => {
                let search = &evidence["search_runtime_contract"];
                (
                    search
                        .get("campaign")
                        .and_then(|row| row.get("source_id"))
                        .and_then(Value::as_str)
                        == Some(source_id)
                        && search
                            .get("summary")
                            .and_then(|row| row.get("activation_status"))
                            .and_then(Value::as_str)
                            == Some("planned"),
                    vec!["core/state/search_runtime_contract.json"],
                )
            }
            _ => (false, vec![]),
        };
        if !ok {
            skipped.push(json!({"task_id": task.get("id").cloned().unwrap_or(Value::Null), "reason": "missing_evidence_surface"}));
            continue;
        }
        let completion = json!({
            "id": task.get("id").cloned().unwrap_or(Value::Null),
            "title": task.get("title").cloned().unwrap_or(Value::Null),
            "owner": task.get("owner").cloned().unwrap_or(Value::Null),
            "priority": task.get("priority").cloned().unwrap_or(Value::Null),
            "status": "completed",
            "queued_at_utc": task.get("queued_at_utc").cloned().unwrap_or(Value::Null),
            "completed_at_utc": now_utc(),
            "result": "completed",
            "notes": format!(
                "Completed by source absorption downstream reconciliation. The required sovereign evidence is now materialized in {}.",
                surfaces.iter().map(|surface| format!("`{surface}`")).collect::<Vec<_>>().join(", ")
            ),
            "glyphs": task.get("glyphs").cloned().unwrap_or_else(|| json!(["↝", "◈"])),
            "meta": task.get("meta").cloned().unwrap_or_else(|| json!({})),
        });
        append_jsonl(&queue_path, &completion)?;
        completed.push(json!({
            "task_id": completion.get("id").cloned().unwrap_or(Value::Null),
            "owner": completion.get("owner").cloned().unwrap_or(Value::Null),
            "source_id": source_id,
            "surfaces": surfaces,
        }));
    }

    let payload = json!({
        "schema_version": "arda.source-absorption-downstream-reconciliation.v1",
        "generated_at_utc": now_utc(),
        "authority": "project_task_queue + downstream_absorption_contracts",
        "summary": {
            "completed_total": completed.len(),
            "skipped_total": skipped.len(),
        },
        "completed": completed,
        "skipped": skipped,
    });
    write_json_pretty(&out, &payload)?;
    Ok(
        json!({"out": "core/state/source_absorption_downstream_reconciliation.json", "completed_total": payload["summary"]["completed_total"]}),
    )
}

fn platform_os_workspace_members(root: &Path) -> anyhow::Result<Vec<String>> {
    let manifest_path = root.join("Cargo.toml");
    let raw = fs::read_to_string(&manifest_path)?;
    let manifest = raw.parse::<toml::Value>()?;
    let members = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .filter_map(|member| {
            Path::new(member)
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    Ok(members)
}

fn string_array_field(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn platform_os_boundary_review_payload(root: &Path) -> anyhow::Result<Value> {
    let audit_path = root.join("core/state/platform_os_core_manifest_audit.json");
    let audit = read_json(&audit_path);
    let workspace_members = platform_os_workspace_members(root)?;
    let frozen_core_members = string_array_field(&audit, "frozen_core_members");
    let workspace_set = workspace_members.iter().cloned().collect::<BTreeSet<_>>();
    let frozen_set = frozen_core_members.iter().cloned().collect::<BTreeSet<_>>();
    let missing_frozen_core_members = frozen_set
        .difference(&workspace_set)
        .cloned()
        .collect::<Vec<_>>();
    let non_core_workspace_members = workspace_set
        .difference(&frozen_set)
        .cloned()
        .collect::<Vec<_>>();
    let status = if missing_frozen_core_members.is_empty() {
        "boundary_review_required_for_non_core_members"
    } else {
        "blocked_missing_frozen_core_members"
    };

    Ok(json!({
        "schema_version": "arda.platform_os_boundary_review.v1",
        "generated_at_utc": now_utc(),
        "task_id": "tsk_20260619_s1_g3_freeze_workspace_cargo_toml_and_add_bounda",
        "authority": "S1/G3 Platform OS workspace boundary review from Cargo.toml and S1/G1 core manifest audit",
        "status": status,
        "mutation_policy": "read_only_boundary_review_no_workspace_rewrite",
        "review_gate": {
            "new_workspace_member_requires_boundary_review": true,
            "new_workspace_member_requires_classification": true,
            "new_workspace_member_requires_plan_or_receipt": true,
            "destructive_workspace_member_removal_allowed": false,
            "staged_member_extraction_deferred_to_follow_on_tasks": true
        },
        "workspace_member_count": workspace_members.len(),
        "frozen_core_count": frozen_core_members.len(),
        "workspace_members": workspace_members,
        "frozen_core_members": frozen_core_members,
        "missing_frozen_core_members": missing_frozen_core_members,
        "non_core_workspace_members": non_core_workspace_members,
        "evidence_surfaces": [
            "Cargo.toml",
            "core/state/platform_os_core_manifest_audit.json",
            "docs/plans/platform-os-core-manifest-audit.md",
            "docs/plans/platform-os-schema-freeze-audit.md",
            "docs/plans/platform-os-tenant-staged-feature-separation.md"
        ],
        "non_actions": [
            "did_not_remove_workspace_members",
            "did_not_move_crates",
            "did_not_publish_or_create_external_repositories",
            "did_not_rewrite_root_cargo_manifest"
        ],
        "closeout_basis": "Boundary review gate is materialized as machine-readable evidence; physical extraction remains deferred to explicit S2/S3 tasks."
    }))
}

fn platform_os_migration_executor() -> anyhow::Result<Value> {
    let root = workspace_root();
    let out = root.join("core/state/platform_os_boundary_review.json");
    let payload = platform_os_boundary_review_payload(&root)?;
    write_json_pretty(&out, &payload)?;
    Ok(json!({
        "out": "core/state/platform_os_boundary_review.json",
        "status": payload["status"],
        "workspace_member_count": payload["workspace_member_count"],
        "frozen_core_count": payload["frozen_core_count"],
        "non_core_workspace_members_total": payload["non_core_workspace_members"].as_array().map(|items| items.len()).unwrap_or_default(),
        "mutation_policy": payload["mutation_policy"]
    }))
}

fn project_task_executor() -> anyhow::Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/project_task_executor.json");
    let latest = latest_tasks(&queue_path);
    let queued = latest
        .values()
        .filter(|task| {
            let status = task
                .get("status")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("queued");
            status == "queued" || status == "pending"
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut eligible = Vec::new();
    let mut ineligible = Vec::new();
    let mut processed_rules = BTreeSet::new();
    let mut runs = Vec::new();

    for task in queued.iter() {
        let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
        let origin = meta
            .get("origin")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let owner = task
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let (rule_id, description, owner_allowlist, commands): (&str, &str, &[&str], Vec<CommandEvent>) =
            match origin {
                "async_user_intake" => (
                    "async_user_intake_handoff",
                    "Inbound links hand off into ATHENA background ingest and planning.",
                    &["athena"],
                    vec![
                        cli_command(&["export", "async-user-intake-contract"])?,
                        cli_command(&["pipeline", "emit-async-user-intake-tasks"])?,
                        cli_command(&["pipeline", "run-async-user-intake-executor"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "human_corpus_digest_plan" => (
                    "human_corpus_digest_handoff",
                    "Human corpus digestion binds grouped evidence into bounded contract surfaces and emits shared receipts.",
                    &["athena", "prometheus", "core"],
                    vec![
                        cli_command(&["export", "human-corpus-digest-plan"])?,
                        cli_command(&["pipeline", "emit-human-corpus-digest-tasks"])?,
                        cli_command(&["export", "human-corpus-extraction-registry"])?,
                        cli_command(&["pipeline", "reconcile-human-corpus-digest-tasks"])?,
                        cli_command(&["export", "athena-digest-pipeline"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "source_absorption_pipeline" => (
                    "absorption_pipeline_autopilot",
                    "Absorption tasks promote into portfolio and downstream executor surfaces.",
                    &["athena", "prometheus", "manwe", "hermes", "apollo"],
                    vec![
                        cli_command(&["export", "source-absorption-pipeline"])?,
                        cli_command(&["export", "source-absorption-portfolio"])?,
                        cli_command(&["pipeline", "emit-source-absorption-tasks"])?,
                        cli_command(&["pipeline", "run-source-absorption-executor"])?,
                        cli_command(&["export", "source-absorption-executor"])?,
                        cli_command(&["export", "athena-digest-pipeline"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "source_absorption_executor" => (
                    "absorption_downstream_reconcile",
                    "Downstream absorption tasks close against sovereign contract surfaces.",
                    &["athena", "prometheus", "manwe", "hermes", "apollo"],
                    vec![
                        cli_command(&["export", "scrapling-runtime-contract"])?,
                        cli_command(&["export", "source-ecosystem-registry"])?,
                        cli_command(&["export", "community-signal-intake"])?,
                        cli_command(&["export", "research-workflow-contract"])?,
                        cli_command(&["export", "search-runtime-contract"])?,
                        cli_command(&["pipeline", "reconcile-source-absorption-downstream"])?,
                        cli_command(&["export", "athena-digest-pipeline"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "flywheel_plan_packet" if flywheel_readiness_executor_rule_applies(task) => (
                    "flywheel_packet_readiness_projection",
                    "Flywheel selector/readiness packets refresh the read-only packet runtime projection.",
                    &["prometheus"],
                    vec![cli_command(&["pipeline", "flywheel-packet-readiness"])?],
                ),
                "session_pivot" if is_public_repo_skeleton_task(task) => (
                    "public_repo_skeleton_executor",
                    "Public repository skeleton tasks verify repository-template plan surfaces and close same-id queue records externally.",
                    &["hades", "warden", "chronos", "prometheus"],
                    vec![
                        cli_command(&["export", "autonomy-task-truth"])?,
                        cli_command(&["export", "plan-index"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "session_pivot" => (
                    "session_pivot_handoff_executor",
                    "Strategic session pivot tasks are executed by Prometheus within bounded executor runtime.",
                    &["prometheus"],
                    vec![
                        cli_command(&["export", "autonomy-task-truth"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "platform_os_migration" if is_platform_os_migration_task(task) => (
                    "platform_os_migration_executor",
                    "Platform OS migration tasks handle structural changes and extractions.",
                    &["prometheus", "hades", "warden"],
                    vec![
                        cli_command(&["export", "autonomy-task-truth"])?,
                        cli_command(&["pipeline", "platform-os-migration-executor"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "readiness_audit" if is_readiness_audit_task(task) => (
                    "readiness_audit_executor",
                    "Readiness audit tasks perform system validation and testing.",
                    &["prometheus", "warden", "chronos"],
                    vec![
                        cli_command(&["export", "autonomy-task-truth"])?,
                        cli_command(&["pipeline", "readiness-audit-executor"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "queue_sequencer" if is_queue_management_task(task) => (
                    "queue_management_executor",
                    "Queue management tasks handle queue refresh and task selection.",
                    &["prometheus", "hades"],
                    vec![
                        cli_command(&["export", "autonomy-task-truth"])?,
                        cli_command(&["export", "queue-hygiene"])?,
                        cli_command(&["export", "queue-active"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                "session_plan_closeout" if is_agent_loop_contract_task(task) => (
                    "agent_loop_contract_executor",
                    "Agent loop contract tasks validate loop contract surfaces before downstream execution.",
                    &["chronos", "prometheus", "hades"],
                    vec![
                        cli_command(&["export", "autonomy-task-truth"])?,
                        cli_command(&["export", "autonomy-resume"])?,
                    ],
                ),
                _ => {
                    ineligible.push(json!({
                        "task_id": task.get("id").cloned().unwrap_or(Value::Null),
                        "owner": task.get("owner").cloned().unwrap_or(Value::Null),
                        "title": task.get("title").cloned().unwrap_or(Value::Null),
                        "reason": "no_bounded_executor_rule",
                    }));
                    continue;
                }
            };
        if !owner_allowlist.contains(&owner) {
            ineligible.push(json!({
                "task_id": task.get("id").cloned().unwrap_or(Value::Null),
                "owner": owner,
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "reason": "no_bounded_executor_rule",
            }));
            continue;
        }
        eligible.push(json!({
            "task_id": task.get("id").cloned().unwrap_or(Value::Null),
            "owner": task.get("owner").cloned().unwrap_or(Value::Null),
            "title": task.get("title").cloned().unwrap_or(Value::Null),
            "rule_id": rule_id,
        }));
        if !processed_rules.insert(rule_id.to_string()) {
            continue;
        }
        let mut events = Vec::new();
        let mut ok = true;
        for command in commands {
            let event = run_command_event(&command, &root);
            let event_ok = event.get("exit_code").and_then(Value::as_i64) == Some(0);
            events.push(event);
            if !event_ok {
                ok = false;
                break;
            }
        }
        runs.push(json!({
            "rule_id": rule_id,
            "ok": ok,
            "description": description,
            "events": events,
        }));
    }

    let payload = json!({
        "schema_version": "arda.project-task-executor.v1",
        "generated_at_utc": now_utc(),
        "authority": "project_task_queue + bounded_executor_rules",
        "doctrine": {
            "only_explicit_queue_classes_auto_run": true,
            "strategic_session_pivots_remain_human_directed": true,
            "executor_runs_rules_once_per_cycle": true,
            "autopilot_may_consume_bounded_work_before_prompting_humans": true,
        },
        "summary": {
            "queued_total": queued.len(),
            "eligible_total": eligible.len(),
            "ineligible_total": ineligible.len(),
            "rules_ran_total": runs.len(),
            "rules_succeeded_total": runs.iter().filter(|row| row.get("ok").and_then(Value::as_bool) == Some(true)).count(),
        },
        "eligible_tasks": eligible,
        "ineligible_tasks": ineligible,
        "runs": runs,
    });
    write_json_pretty(&out, &payload)?;
    Ok(json!({
        "out": "core/state/project_task_executor.json",
        "eligible_total": payload["summary"]["eligible_total"],
        "rules_ran_total": payload["summary"]["rules_ran_total"]
    }))
}

fn flywheel_packet_readiness() -> anyhow::Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let out = root.join("core/state/flywheel_packet_runtime.json");
    let latest = latest_tasks(&queue_path);
    let mut packets = latest
        .values()
        .filter(|task| is_flywheel_packet(task))
        .map(|task| classify_flywheel_packet(task, &latest, &root))
        .collect::<Vec<_>>();
    packets.sort_by(|left, right| {
        left.get("packet_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .cmp(
                right
                    .get("packet_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
            .then_with(|| {
                left.get("task_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .cmp(
                        right
                            .get("task_id")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    )
            })
    });

    let mut readiness_counts = BTreeMap::<String, usize>::new();
    for packet in packets.iter() {
        let readiness = packet
            .get("readiness")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *readiness_counts.entry(readiness).or_insert(0) += 1;
    }
    let ready_total = readiness_counts.get("ready").copied().unwrap_or(0);
    let blocked_total = packets
        .iter()
        .filter(|packet| {
            matches!(
                packet.get("readiness").and_then(Value::as_str),
                Some("dependency_blocked" | "missing_fields" | "human_gated")
            )
        })
        .count();
    let payload = json!({
        "schema_version": "arda.flywheel.packet_runtime.v1",
        "generated_at_utc": now_utc(),
        "authority": "core/projects/tasks/queue.jsonl + docs/contracts/flywheel-work-packet-contract.md",
        "source_queue": "core/projects/tasks/queue.jsonl",
        "contract": "docs/contracts/flywheel-work-packet-contract.md",
        "doctrine": {
            "read_only_projection": true,
            "task_queue_mutation": false,
            "dispatch": false,
            "completion": false,
        },
        "summary": {
            "packet_total": packets.len(),
            "ready_total": ready_total,
            "blocked_total": blocked_total,
            "readiness_counts": readiness_counts,
        },
        "packets": packets,
    });
    write_json_pretty(&out, &payload)?;
    Ok(json!({
        "out": "core/state/flywheel_packet_runtime.json",
        "packet_total": payload["summary"]["packet_total"],
        "ready_total": payload["summary"]["ready_total"],
        "blocked_total": payload["summary"]["blocked_total"],
    }))
}

fn flywheel_dispatch(task_id: Option<&str>, write: bool) -> anyhow::Result<Value> {
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let latest = latest_tasks(&queue_path);
    let mut candidates = latest
        .values()
        .filter(|task| is_flywheel_packet(task))
        .filter(|task| {
            task_id.is_none_or(|selected| task.get("id").and_then(Value::as_str) == Some(selected))
        })
        .map(|task| {
            let classified = classify_flywheel_packet(task, &latest, &root);
            (task.clone(), classified)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.1
            .get("packet_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .cmp(
                right
                    .1
                    .get("packet_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
    });
    let Some((task, classified)) = candidates.into_iter().find(|(_, classified)| {
        classified.get("readiness").and_then(Value::as_str) == Some("ready")
    }) else {
        anyhow::bail!("no ready Flywheel packet matched the selection");
    };
    let meta = task.get("meta").cloned().unwrap_or_else(|| json!({}));
    if meta.get("harness").and_then(Value::as_str) != Some("hermes-agent-manwe") {
        anyhow::bail!(
            "selected Flywheel packet is ready but not Hermes-dispatchable: harness={}",
            meta.get("harness")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
    }
    if meta.get("risk").and_then(Value::as_str) != Some("safe-local") {
        anyhow::bail!("selected Flywheel packet is not safe-local");
    }
    let owner = task
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !flywheel_owner_allowed(owner) {
        anyhow::bail!("selected Flywheel packet owner '{owner}' is not dispatch-allowlisted");
    }
    let target_node = meta
        .get("target_node")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("selected Flywheel packet has no target_node"))?;
    let plan_path = meta
        .get("plan")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("selected Flywheel packet has no plan"))?;
    let plan_text = fs::read_to_string(root.join(plan_path))?;
    let prompt = flywheel_prompt(&task, &plan_text);
    let prompt_digest = sha1_12(&prompt);
    let dry_run = !write;

    let mut preflight_args = vec![
        "utility",
        "hermes-agent-edge-bridge-preflight",
        "--node",
        target_node,
    ];
    if dry_run {
        preflight_args.push("--dry-run");
    }
    let preflight = run_command_event(&cli_command(&preflight_args)?, &root);
    let preflight_ok = dry_run || preflight.get("exit_code").and_then(Value::as_i64) == Some(0);
    if !preflight_ok {
        anyhow::bail!("Hermes bridge preflight failed: {}", preflight);
    }

    let mut dispatch_args = vec![
        "utility",
        "hermes-agent-edge-bridge-dispatch",
        "--node",
        target_node,
        "--prompt",
        &prompt,
    ];
    if dry_run {
        dispatch_args.push("--dry-run");
    }
    let dispatch = run_command_event(&cli_command(&dispatch_args)?, &root);
    let dispatch_ok = dry_run || dispatch.get("exit_code").and_then(Value::as_i64) == Some(0);
    if !dispatch_ok {
        anyhow::bail!("Hermes bridge dispatch failed: {}", dispatch);
    }

    let receipt = flywheel_dispatch_receipt(&task, &prompt_digest, dry_run, &preflight, &dispatch);
    let receipt_path = root.join("data/hermes/flywheel_dispatch_receipts.jsonl");
    let receipt_written = if write {
        append_jsonl(&receipt_path, &receipt)?;
        true
    } else {
        false
    };

    Ok(json!({
        "schema_version": "arda.flywheel.dispatch_report.v1",
        "task_id": task.get("id").cloned().unwrap_or(Value::Null),
        "packet": classified,
        "dry_run": dry_run,
        "write": write,
        "receipt_written": receipt_written,
        "receipt_path": "data/hermes/flywheel_dispatch_receipts.jsonl",
        "prompt_sha1_12": prompt_digest,
        "preflight_exit_code": preflight.get("exit_code").cloned().unwrap_or(Value::Null),
        "dispatch_exit_code": dispatch.get("exit_code").cloned().unwrap_or(Value::Null),
        "receipt_preview": if write { Value::Null } else { receipt },
    }))
}

#[allow(clippy::too_many_arguments)]
fn flywheel_review_receipt_command(
    task_id: String,
    dispatch_receipt: Option<&str>,
    changed_files: Vec<String>,
    verification: Vec<String>,
    diff_review: String,
    recommendation: String,
    notes: Option<&str>,
    write: bool,
) -> anyhow::Result<Value> {
    if changed_files.is_empty() {
        anyhow::bail!("at least one --changed-file is required");
    }
    if verification.is_empty() {
        anyhow::bail!("at least one --verification is required");
    }
    if diff_review.trim().is_empty() {
        anyhow::bail!("--diff-review must not be empty");
    }
    let root = workspace_root();
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let latest = latest_tasks(&queue_path);
    let task = latest
        .get(&task_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("task not found in latest queue state: {task_id}"))?;
    if !is_flywheel_packet(&task) {
        anyhow::bail!("task is not a flywheel_plan_packet: {task_id}");
    }
    let dry_run = !write;
    let receipt = flywheel_review_receipt(
        &task,
        dispatch_receipt,
        &changed_files,
        &verification,
        &diff_review,
        &recommendation,
        notes,
        dry_run,
    );
    let receipt_path = root.join("data/hades/flywheel_review_receipts.jsonl");
    let receipt_written = if write {
        append_jsonl(&receipt_path, &receipt)?;
        true
    } else {
        false
    };
    Ok(json!({
        "schema_version": "arda.flywheel.review_report.v1",
        "task_id": task_id,
        "dry_run": dry_run,
        "write": write,
        "receipt_written": receipt_written,
        "receipt_path": "data/hades/flywheel_review_receipts.jsonl",
        "receipt_preview": if write { Value::Null } else { receipt },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "arda-cli-flywheel-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        path
    }

    fn task(id: &str, status: &str, meta: Value) -> Value {
        json!({
            "id": id,
            "title": id,
            "owner": "prometheus",
            "priority": "high",
            "status": status,
            "queued_at_utc": "2026-05-27T00:00:00Z",
            "meta": meta,
        })
    }

    fn packet_meta(packet_id: &str, depends_on: &str) -> Value {
        json!({
            "origin": "flywheel_plan_packet",
            "scope": "flywheel_automation",
            "plan": "docs/plans/example.md",
            "packet_id": packet_id,
            "depends_on": depends_on,
            "risk": "safe-local",
            "harness": "arda-cli",
            "acceptance": "acceptance",
            "verify": "verify",
            "receipt_surface": "core/state/flywheel_packet_runtime.json",
        })
    }

    #[test]
    fn platform_os_migration_rule_matches_s1g3_plan_origin() {
        let task = json!({
            "id": "tsk_20260619_s1_g3_freeze_workspace_cargo_toml_and_add_bounda",
            "title": "S1/G3: freeze workspace Cargo.toml and add boundary review gate for new crates",
            "owner": "prometheus",
            "priority": "high",
            "status": "queued",
            "queued_at_utc": "2026-06-19T07:59:53Z",
            "meta": {
                "origin": "core/projects/Plans/PLATFORM_OS.md",
                "scope": "platform_os"
            }
        });

        assert!(is_platform_os_migration_task(&task));
    }

    #[test]
    fn platform_os_boundary_review_payload_is_read_only_gate() {
        let dir = temp_root("platform-os-boundary");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[workspace]
members = [
    "crates/arda-core",
    "crates/arda-cli",
    "crates/arda-human",
]
"#,
        )
        .expect("manifest");
        let audit_path = dir.join("core/state/platform_os_core_manifest_audit.json");
        ensure_parent(&audit_path).expect("audit parent");
        fs::write(
            &audit_path,
            serde_json::to_string(&json!({
                "frozen_core_members": ["arda-core", "arda-cli"]
            }))
            .expect("audit json"),
        )
        .expect("audit");

        let payload = platform_os_boundary_review_payload(&dir).expect("payload");

        assert_eq!(
            payload["schema_version"],
            "arda.platform_os_boundary_review.v1"
        );
        assert_eq!(
            payload["mutation_policy"],
            "read_only_boundary_review_no_workspace_rewrite"
        );
        assert_eq!(
            payload["review_gate"]["new_workspace_member_requires_boundary_review"],
            true
        );
        assert_eq!(
            payload["review_gate"]["destructive_workspace_member_removal_allowed"],
            false
        );
        assert_eq!(payload["workspace_member_count"], 3);
        assert_eq!(payload["frozen_core_count"], 2);
        assert_eq!(
            payload["missing_frozen_core_members"]
                .as_array()
                .expect("missing")
                .len(),
            0
        );
        assert!(payload["non_core_workspace_members"]
            .as_array()
            .expect("non-core")
            .iter()
            .any(|member| member.as_str() == Some("arda-human")));
        assert!(payload["non_actions"]
            .as_array()
            .expect("non-actions")
            .iter()
            .any(|action| action.as_str() == Some("did_not_rewrite_root_cargo_manifest")));
    }

    #[test]
    fn flywheel_classifies_ready_packet_when_dependency_completed() {
        let dir = temp_root("ready");
        let plan_path = dir.join("docs/plans/example.md");
        ensure_parent(&plan_path).expect("parent");
        fs::write(&plan_path, "# Example\n").expect("plan");
        let dep = task("dep", "completed", json!({"origin": "session_pivot"}));
        let packet = task("packet", "queued", packet_meta("F2", "dep"));
        let latest = BTreeMap::from([
            ("dep".to_string(), dep),
            ("packet".to_string(), packet.clone()),
        ]);

        let classified = classify_flywheel_packet(&packet, &latest, &dir);

        assert_eq!(classified["readiness"], "ready");
        assert_eq!(classified["dependencies"][0]["terminal_success"], true);
    }

    #[test]
    fn flywheel_readiness_executor_rule_is_narrow() {
        let selector = task(
            "selector",
            "queued",
            json!({
                "origin": "flywheel_plan_packet",
                "scope": "flywheel_automation",
                "plan": "docs/plans/example.md",
                "packet_id": "F2",
                "depends_on": "dep",
                "risk": "safe-local",
                "harness": "arda-cli",
                "acceptance": "read_only_packet_readiness_projection",
                "verify": "cargo_test_arda_cli_flywheel_and_project_task_executor",
                "receipt_surface": "core/state/flywheel_packet_runtime.json",
            }),
        );
        let mut dispatch_meta = packet_meta("F3", "selector");
        dispatch_meta["harness"] = json!("hermes-agent-manwe");
        dispatch_meta["target_node"] = json!("node-backbone-server");
        dispatch_meta["acceptance"] = json!("bounded_hermes_dispatch_rule_and_receipts");
        dispatch_meta["receipt_surface"] = json!("data/hermes/flywheel_dispatch_receipts.jsonl");
        let dispatch = task("dispatch", "queued", dispatch_meta);

        assert!(flywheel_readiness_executor_rule_applies(&selector));
        assert!(!flywheel_readiness_executor_rule_applies(&dispatch));
    }

    #[test]
    fn flywheel_classifies_dependency_blocked_packet() {
        let dir = temp_root("blocked");
        let plan_path = dir.join("docs/plans/example.md");
        ensure_parent(&plan_path).expect("parent");
        fs::write(&plan_path, "# Example\n").expect("plan");
        let packet = task("packet", "queued", packet_meta("F2", "missing_dep"));
        let latest = BTreeMap::from([("packet".to_string(), packet.clone())]);

        let classified = classify_flywheel_packet(&packet, &latest, &dir);

        assert_eq!(classified["readiness"], "dependency_blocked");
        assert_eq!(classified["dependencies"][0]["present"], false);
    }

    #[test]
    fn flywheel_classifies_human_gated_packet_as_ready_after_approval() {
        let dir = temp_root("human-approved");
        let plan_path = dir.join("docs/plans/example.md");
        ensure_parent(&plan_path).expect("parent");
        fs::write(&plan_path, "# Example\n").expect("plan");
        let mut meta = packet_meta("F2", "");
        meta["risk"] = json!("human-gated");
        meta["human_approval"] = json!({
            "status": "approved",
            "approved_by": "operator",
            "approved_at_utc": "2026-05-31T00:00:00Z",
        });
        let packet = task("packet", "queued", meta);
        let latest = BTreeMap::from([("packet".to_string(), packet.clone())]);

        let classified = classify_flywheel_packet(&packet, &latest, &dir);

        assert_eq!(classified["readiness"], "ready");
        assert_eq!(classified["human_approved"], true);
    }

    #[test]
    fn flywheel_hermes_packet_requires_target_node() {
        let dir = temp_root("target");
        let plan_path = dir.join("docs/plans/example.md");
        ensure_parent(&plan_path).expect("parent");
        fs::write(&plan_path, "# Example\n").expect("plan");
        let mut meta = packet_meta("F3", "");
        meta["harness"] = json!("hermes-agent-manwe");
        let packet = task("packet", "queued", meta);
        let latest = BTreeMap::from([("packet".to_string(), packet.clone())]);

        let classified = classify_flywheel_packet(&packet, &latest, &dir);

        assert_eq!(classified["readiness"], "missing_fields");
        assert!(classified["missing_fields"]
            .as_array()
            .expect("missing")
            .iter()
            .any(|field| field.as_str() == Some("target_node")));
    }

    #[test]
    fn flywheel_prompt_contains_packet_boundaries() {
        let mut meta = packet_meta("F3", "");
        meta["harness"] = json!("hermes-agent-manwe");
        meta["target_node"] = json!("node-backbone-server");
        meta["expected_files"] = json!("crates/arda-cli/src/commands/pipeline.rs");
        let packet = task("packet", "queued", meta);

        let prompt = flywheel_prompt(&packet, "# Plan\n\nImplement safely.");

        assert!(prompt.contains("Task ID: packet"));
        assert!(prompt.contains("Packet: F3"));
        assert!(prompt.contains("Do not mark the task complete"));
        assert!(prompt.contains("Plan excerpt:"));
    }

    #[test]
    fn flywheel_dispatch_receipt_preserves_dry_run_and_digest() {
        let mut meta = packet_meta("F3", "");
        meta["harness"] = json!("hermes-agent-manwe");
        meta["target_node"] = json!("node-backbone-server");
        let packet = task("packet", "queued", meta);
        let preflight = json!({"exit_code": 0});
        let dispatch = json!({"exit_code": 0});

        let receipt =
            flywheel_dispatch_receipt(&packet, "abc123def456", true, &preflight, &dispatch);

        assert_eq!(
            receipt["schema_version"],
            "arda.flywheel.dispatch_receipt.v1"
        );
        assert_eq!(receipt["task_id"], "packet");
        assert_eq!(receipt["target_node"], "node-backbone-server");
        assert_eq!(receipt["prompt_sha1_12"], "abc123def456");
        assert_eq!(receipt["dry_run"], true);
    }

    #[test]
    fn flywheel_review_receipt_records_evidence_fields() {
        let packet = task("packet", "queued", packet_meta("F4", "dep"));
        let changed_files = vec!["crates/arda-cli/src/commands/pipeline.rs".to_string()];
        let verification = vec!["cargo test -p arda-cli flywheel --all-targets".to_string()];

        let receipt = flywheel_review_receipt(
            &packet,
            Some("data/hermes/flywheel_dispatch_receipts.jsonl:1"),
            &changed_files,
            &verification,
            "diff reviewed; scoped to pipeline command",
            "complete",
            Some("no follow-up required"),
            true,
        );

        assert_eq!(
            receipt["schema_version"],
            "arda.flywheel.review_receipt.v1"
        );
        assert_eq!(receipt["task_id"], "packet");
        assert_eq!(
            receipt["changed_files"][0],
            "crates/arda-cli/src/commands/pipeline.rs"
        );
        assert_eq!(receipt["completion_recommendation"], "complete");
        assert_eq!(receipt["dry_run"], true);
    }
}
