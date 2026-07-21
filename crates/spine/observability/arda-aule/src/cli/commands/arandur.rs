#![cfg(feature = "full-cli")]
use super::super::*;
use anyhow::Context;
use fs2::FileExt;
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, HashSet};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Mutex, OnceLock};

static ARANDUR_EPISODE_APPEND_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

const ARANDUR_RUNTIME_PATH: &str = "core/state/arandur/runtime.json";
const ARANDUR_EPISODES_PATH: &str = "data/arandur/episodes.jsonl";
const ARANDUR_RECOMMENDATIONS_PATH: &str = "data/arandur/recommendations.jsonl";
const ARANDUR_MUTATION_EVIDENCE_PATH: &str = "data/arandur/mutation_evidence.jsonl";
const ARANDUR_SCOUT_FINDINGS_PATH: &str = "data/arandur/scout_findings.jsonl";
const ARANDUR_PATTERN_SYNTHESIS_PATH: &str = "data/arandur/pattern_synthesis.jsonl";
const ARANDUR_MISSION_CANDIDATES_PATH: &str = "data/arandur/mission_candidates.jsonl";
const ARANDUR_MISSION_REVIEWS_PATH: &str = "data/arandur/mission_reviews.jsonl";
const ARANDUR_MISSION_APPROVAL_REQUESTS_PATH: &str = "data/arandur/mission_approval_requests.jsonl";
const ARANDUR_MISSION_QUEUE_PROPOSALS_PATH: &str = "data/arandur/mission_queue_proposals.jsonl";
const ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH: &str =
    "data/arandur/mission_queue_write_requests.jsonl";
const ARANDUR_PRESENCE_EVENTS_PATH: &str = "data/prometheus/arda_presence_events.jsonl";
const TASK_QUEUE_PATH: &str = "core/projects/tasks/queue.jsonl";
const DEFAULT_PHASE2F_PACKET_DIR: &str = "audit/HUMAN_INBOX_PHASE2F_2026-05-17";
const RECOMMENDED_QUEUE_ENTRIES: &str = "recommended_queue_entries.jsonl";

pub(crate) fn handle(command: ArandurCommands) -> anyhow::Result<()> {
    match command {
        ArandurCommands::Status { root } => {
            let root = resolve_root(root);
            let state_path = root.join(ARANDUR_RUNTIME_PATH);
            let value = if state_path.exists() {
                let content = fs::read_to_string(&state_path)?;
                serde_json::from_str::<serde_json::Value>(&content)?
            } else {
                json!({
                    "contract": "arda.arandur.runtime_state.v1",
                    "status": "missing",
                    "state_path": state_path,
                    "message": "Arandur runtime state has not been initialized"
                })
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Initialize { root } => {
            let root = resolve_root(root);
            let value = initialize_arandur_state(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::RecordEpisode {
            root,
            episode_type,
            summary,
            evidence,
            recommendation,
        } => {
            let root = resolve_root(root);
            let value =
                append_arandur_episode(&root, &episode_type, &summary, evidence, recommendation)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Observe { root } => {
            let root = resolve_root(root);
            let value = observe_arandur(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::SystemMap { root } => {
            let root = resolve_root(root);
            let value = map_arandur_system(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::ImprovementScan { root } => {
            let root = resolve_root(root);
            let value = scan_arandur_improvements(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::ReviewPacket { root, packet_dir } => {
            let root = resolve_root(root);
            let packet_dir = root.join(packet_dir);
            let value = review_arandur_packet(&root, &packet_dir)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::RecommendNext { root, dry_run } => {
            let root = resolve_root(root);
            let value = recommend_arandur_next(&root, dry_run)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Cycle {
            root,
            packet_dir,
            append_recommendations,
            record_episode,
        } => {
            let root = resolve_root(root);
            let packet_dir = root.join(packet_dir);
            let value =
                run_arandur_cycle(&root, &packet_dir, append_recommendations, record_episode)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Benchmark { root, packet_dir } => {
            let root = resolve_root(root);
            let packet_dir = root.join(packet_dir);
            let value = benchmark_arandur_safety(&root, &packet_dir)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Recommendations { root } => {
            let root = resolve_root(root);
            let value = summarize_arandur_recommendations(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Readiness { root } => {
            let root = resolve_root(root);
            let value = assess_arandur_readiness(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::PromoteLevel {
            root,
            target,
            write,
            approval_note,
        } => {
            let root = resolve_root(root);
            let value = promote_arandur_level(&root, target, write, approval_note.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MutationClasses { root } => {
            let root = resolve_root(root);
            let value = report_bounded_mutation_classes(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::VerifyMutation {
            root,
            mutation_class,
            target_path,
            pre_sha1,
            pre_bytes,
            write_report,
        } => {
            let root = resolve_root(root);
            let value = verify_bounded_mutation(
                &root,
                &mutation_class,
                &target_path,
                pre_sha1.as_deref(),
                pre_bytes,
                write_report,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::RollbackReport {
            root,
            mutation_class,
            target_path,
            reason,
            write_report,
        } => {
            let root = resolve_root(root);
            let value = report_bounded_rollback_evidence(
                &root,
                &mutation_class,
                &target_path,
                &reason,
                write_report,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::ScoutPlan { root, scope, limit } => {
            let root = resolve_root(root);
            let value = plan_arandur_phase6b_scout(&root, &scope, limit)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::ScoutExecute {
            root,
            mission_id,
            scope,
            source_urls,
            evidence_file,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let evidence_path = evidence_file.as_deref().map(Path::new);
            let value = execute_arandur_phase6c_scout(
                &root,
                &mission_id,
                &scope,
                source_urls,
                evidence_path,
                write,
                justification.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::PatternSynthesis {
            root,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value =
                synthesize_arandur_phase6d_patterns(&root, write, justification.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionPromotion {
            root,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value = promote_arandur_phase6e_missions(&root, write, justification.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionReview {
            root,
            candidate_id,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value = review_arandur_phase6f_mission_candidates(
                &root,
                candidate_id.as_deref(),
                write,
                justification.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionApprovalRequest {
            root,
            candidate_id,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value = request_arandur_phase6g_mission_approval(
                &root,
                candidate_id.as_deref(),
                write,
                justification.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionApprovalDecision {
            root,
            approval_request_id,
            status,
            justification,
        } => {
            let root = resolve_root(root);
            let value = record_arandur_phase6g_mission_approval_decision(
                &root,
                &approval_request_id,
                &status,
                &justification,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionQueueProposal {
            root,
            candidate_id,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value = propose_arandur_phase6h_mission_queue_entries(
                &root,
                candidate_id.as_deref(),
                write,
                justification.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionQueueWriteRequest {
            root,
            candidate_id,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value = request_arandur_phase6i_mission_queue_write(
                &root,
                candidate_id.as_deref(),
                write,
                justification.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::ExecuteQueueWrite {
            root,
            candidate_id,
            write,
            justification,
        } => {
            let root = resolve_root(root);
            let value = execute_arandur_phase6j_queue_write(
                &root,
                candidate_id.as_deref(),
                write,
                justification.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::PresenceEvent {
            root,
            event_id,
            agent,
            mode,
            attention,
            accent,
            anchor_target,
            mission_id,
            correlation_id,
            timestamp_utc,
        } => {
            let root = resolve_root(root);
            let value = append_arandur_presence_event(
                &root,
                ArandurPresenceEventInput {
                    event_id: event_id.as_deref(),
                    agent: &agent,
                    mode: &mode,
                    attention: &attention,
                    accent: &accent,
                    anchor_target: &anchor_target,
                    mission_id: mission_id.as_deref(),
                    correlation_id: correlation_id.as_deref(),
                    timestamp_utc: timestamp_utc.as_deref(),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::MissionBacklog { root } => {
            let root = resolve_root(root);
            let value = report_arandur_mission_backlog(&root)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        ArandurCommands::Gate {
            root,
            action,
            objective_id,
            justification,
        } => {
            let root = resolve_root(root);
            let value = match action {
                ArandurGateAction::Next => report_arandur_gate_next(&root)?,
                ArandurGateAction::Blocked => report_arandur_gate_blocked(&root)?,
                ArandurGateAction::Approve => approve_arandur_gate_candidate(
                    &root,
                    objective_id
                        .as_deref()
                        .context("--objective-id is required for gate approve")?,
                    justification
                        .as_deref()
                        .context("--justification is required for gate approve")?,
                )?,
                ArandurGateAction::Deny => deny_arandur_gate_candidate(
                    &root,
                    objective_id
                        .as_deref()
                        .context("--objective-id is required for gate deny")?,
                    justification
                        .as_deref()
                        .context("--justification is required for gate deny")?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

pub(crate) fn resolve_root(root: Option<String>) -> PathBuf {
    root.map(PathBuf::from)
        .or_else(|| std::env::var("ARDA_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(arda_root)
}

fn initialize_arandur_state(root: &Path) -> anyhow::Result<serde_json::Value> {
    let state_path = root.join(ARANDUR_RUNTIME_PATH);
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let ledger_path = root.join(ARANDUR_EPISODES_PATH);
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !ledger_path.exists() {
        fs::write(&ledger_path, "")?;
    }
    let recommendation_path = root.join(ARANDUR_RECOMMENDATIONS_PATH);
    if let Some(parent) = recommendation_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !recommendation_path.exists() {
        fs::write(&recommendation_path, "")?;
    }
    let value = arandur_default_state();
    fs::write(
        &state_path,
        format!("{}\n", serde_json::to_string_pretty(&value)?),
    )?;
    Ok(value)
}

fn arandur_default_state() -> serde_json::Value {
    json!({
        "contract": "arda.arandur.runtime_state.v1",
        "authority": "agent_generated",
        "review_required": true,
        "status": "active_draft",
        "mode": "observe_plan_recommend",
        "autonomy_level": 1,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "owner": "prometheus",
        "orchestrator": "arandur",
        "mission": "learn from completed ATHENA and project-governance episodes, then recommend bounded next actions without directly mutating canonical project state",
        "mvp_domain": "task_queue_governance_and_athena_packet_review",
        "mutation_policy": {
            "raw_human_inbox": "read_only",
            "canonical_queue": "recommend_only_until_review",
            "knowledge_notes": "candidate_addenda_only_with_review_required",
            "services": "no_restart_or_destructive_action"
        },
        "state_path": ARANDUR_RUNTIME_PATH,
        "episode_ledger": ARANDUR_EPISODES_PATH,
        "recommendation_ledger": ARANDUR_RECOMMENDATIONS_PATH,
        "surfaces": {
            "status_cli": "arda-cli prometheus arandur status",
            "initialize_cli": "arda-cli prometheus arandur initialize",
            "episode_cli": "arda-cli prometheus arandur record-episode",
            "observe_cli": "arda-cli prometheus arandur observe",
            "review_packet_cli": "arda-cli prometheus arandur review-packet --packet-dir <path>",
            "recommend_next_cli": "arda-cli prometheus arandur recommend-next",
            "readiness_cli": "arda-cli prometheus arandur readiness",
            "athena_packet_intake_cli": "arda-cli athena packet-intake",
            "athena_packet_promotion_cli": "arda-cli athena packet-promotion-surface"
        },
        "promotion_gates": [
            "evidence_present",
            "source_provenance_preserved",
            "raw_inbox_read_only",
            "research_claims_review_gated",
            "candidate_artifacts_marked_agent_generated",
            "json_and_jsonl_validated",
            "git_diff_check_clean"
        ],
        "next_autonomy_gate": "Level 2 may append review packets and recommended queue entries only after repeated clean dry-run episodes"
    })
}

fn append_arandur_episode(
    root: &Path,
    episode_type: &str,
    summary: &str,
    evidence: Vec<String>,
    recommendation: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    let _append_guard = ARANDUR_EPISODE_APPEND_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow::anyhow!("arandur episode append mutex poisoned"))?;
    let state_path = root.join(ARANDUR_RUNTIME_PATH);
    if !state_path.exists() {
        initialize_arandur_state(root)?;
    }
    let ledger_path = root.join(ARANDUR_EPISODES_PATH);
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&ledger_path)
        .with_context(|| {
            format!(
                "failed to open episode ledger {}",
                display_path(&ledger_path)
            )
        })?;
    file.lock_exclusive().with_context(|| {
        format!(
            "failed to lock episode ledger {}",
            display_path(&ledger_path)
        )
    })?;

    file.seek(SeekFrom::Start(0)).with_context(|| {
        format!(
            "failed to seek episode ledger {}",
            display_path(&ledger_path)
        )
    })?;
    let mut existing_content = String::new();
    file.read_to_string(&mut existing_content)
        .with_context(|| {
            format!(
                "failed to read episode ledger {}",
                display_path(&ledger_path)
            )
        })?;
    let existing_records = parse_jsonl_values(&ledger_path, &existing_content)?;
    let max_existing_sequence = existing_records
        .iter()
        .filter_map(|record| record.get("episode_sequence"))
        .filter_map(serde_json::Value::as_u64)
        .max()
        .unwrap_or(0);
    let existing_count =
        u64::try_from(existing_records.len()).unwrap_or(u64::MAX.saturating_sub(1));
    let episode_sequence = max_existing_sequence.max(existing_count).saturating_add(1);
    let ts_utc = Utc::now();
    let episode_time = ts_utc.timestamp_nanos_opt().map_or_else(
        || ts_utc.timestamp_micros().to_string(),
        |nanos| nanos.to_string(),
    );
    let episode_id = format!("arandur_episode_{episode_time}_{episode_sequence}");
    let value = json!({
        "contract": "arda.arandur.episode.v1",
        "authority": "agent_generated",
        "review_required": true,
        "episode_id": episode_id,
        "episode_sequence": episode_sequence,
        "ts_utc": ts_utc.to_rfc3339(),
        "episode_type": episode_type,
        "summary": summary,
        "evidence": evidence,
        "recommendation": recommendation.unwrap_or_else(|| "review before promotion".to_string()),
        "mutation_policy": "recommend_only"
    });
    let write_result = writeln!(file, "{}", serde_json::to_string(&value)?).with_context(|| {
        format!(
            "failed to append episode ledger {}",
            display_path(&ledger_path)
        )
    });
    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock episode ledger {}",
            display_path(&ledger_path)
        )
    });
    match (write_result, unlock_result) {
        (Ok(()), Ok(())) => {}
        (Err(error), _) => return Err(error),
        (Ok(()), Err(error)) => return Err(error),
    }
    Ok(json!({
        "status": "recorded",
        "ledger_path": ARANDUR_EPISODES_PATH,
        "episode": value
    }))
}

fn observe_arandur(root: &Path) -> anyhow::Result<serde_json::Value> {
    let runtime_path = root.join(ARANDUR_RUNTIME_PATH);
    let runtime_state = read_json_file_optional(&runtime_path)?;
    let queue_summary = summarize_jsonl_file(&root.join(TASK_QUEUE_PATH))?;
    let episode_summary = summarize_jsonl_file(&root.join(ARANDUR_EPISODES_PATH))?;
    let recommendation_summary = summarize_jsonl_file(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    let packet_dir = root.join(DEFAULT_PHASE2F_PACKET_DIR);
    let packet_state = summarize_packet_state(&packet_dir)?;

    Ok(json!({
        "contract": "arda.arandur.observation.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "autonomy_level": runtime_state.as_ref().and_then(|value| value.get("autonomy_level")).cloned().unwrap_or_else(|| json!(1)),
        "mutation_policy": "read_only_observation",
        "queue": queue_summary,
        "phase2f_packet": packet_state,
        "arandur": {
            "runtime_state_path": ARANDUR_RUNTIME_PATH,
            "runtime_state_present": runtime_state.is_some(),
            "runtime_state": runtime_state,
            "episode_ledger": ARANDUR_EPISODES_PATH,
            "episode_count": episode_summary["total_records"],
            "recommendation_ledger": ARANDUR_RECOMMENDATIONS_PATH,
            "recommendation_count": recommendation_summary["total_records"]
        },
        "next_allowed_actions": [
            "review_packet",
            "recommend_next",
            "readiness_report"
        ],
        "forbidden_actions_confirmed": [
            "no_raw_human_inbox_mutation",
            "no_canonical_queue_append",
            "no_service_restart",
            "no_destructive_operation"
        ]
    }))
}

fn report_arandur_mission_backlog(root: &Path) -> anyhow::Result<serde_json::Value> {
    let queue_path = root.join(TASK_QUEUE_PATH);
    let records = read_jsonl_values(&queue_path)?;
    let summary = summarize_jsonl_file(&queue_path)?;
    let mut latest_by_identity: BTreeMap<String, (usize, serde_json::Value)> = BTreeMap::new();

    for (index, record) in records.iter().enumerate() {
        let identity = json_id(record)
            .map(|id| format!("id:{id}"))
            .unwrap_or_else(|| format!("record:{index}"));
        latest_by_identity.insert(identity, (index, record.clone()));
    }

    let mut effective_records: Vec<serde_json::Value> = latest_by_identity
        .iter()
        .map(|(identity, (index, record))| {
            let mut value = record.clone();
            if let Some(object) = value.as_object_mut() {
                object.insert("effective_identity".to_string(), json!(identity));
                object.insert("effective_record_index".to_string(), json!(index));
            }
            value
        })
        .collect();
    effective_records.sort_by_key(|record| {
        record
            .get("effective_record_index")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(u64::MAX)
    });

    let open_statuses = ["pending", "queued", "in_progress"];
    let effective_open_tasks: Vec<serde_json::Value> = effective_records
        .iter()
        .filter(|record| {
            record
                .get("status")
                .and_then(serde_json::Value::as_str)
                .map(|status| open_statuses.contains(&status))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    let effective_mission_review_tasks: Vec<serde_json::Value> = effective_open_tasks
        .iter()
        .filter(|record| is_arandur_mission_review_task(record))
        .cloned()
        .collect();
    let next_effective_task = effective_open_tasks.first().cloned();
    let next_mission_review_task = effective_mission_review_tasks.first().cloned();
    let status = if effective_open_tasks.is_empty() {
        "no_effective_open_tasks"
    } else {
        "effective_open_tasks_present"
    };
    let next_recommended_action = if next_mission_review_task.is_some() {
        "execute_next_effective_mission_review_task"
    } else if next_effective_task.is_some() {
        "execute_next_effective_open_task"
    } else {
        "generate_or_select_next_review_gated_arandur_gate"
    };

    Ok(json!({
        "contract": "arda.arandur.mission_backlog.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "mutation_policy": "read_only_effective_queue_report",
        "status": status,
        "queue": summary,
        "effective_open_task_count": effective_open_tasks.len(),
        "effective_mission_review_task_count": effective_mission_review_tasks.len(),
        "next_effective_task": next_effective_task,
        "next_mission_review_task": next_mission_review_task,
        "next_recommended_action": next_recommended_action,
        "interpretation": {
            "append_only_queue": true,
            "raw_pending_records_may_be_superseded": true,
            "effective_status_source": "latest JSONL record per task id"
        },
        "forbidden_actions_confirmed": [
            "no_canonical_queue_append",
            "no_ledger_append",
            "no_service_restart",
            "no_destructive_operation"
        ]
    }))
}

fn report_arandur_gate_next(root: &Path) -> anyhow::Result<serde_json::Value> {
    let candidates = arandur_gate_candidates(root)?;
    let selected_candidate = candidates
        .iter()
        .find(|candidate| candidate["governance_class"] == "review_gated_recommendation")
        .cloned();
    let status = if selected_candidate.is_some() {
        "next_review_gated_candidate_selected"
    } else {
        "no_review_gated_candidate_available"
    };

    Ok(json!({
        "contract": "arda.arandur.gate_next.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "status": status,
        "selected_candidate": selected_candidate,
        "candidate_count": candidates.len(),
        "mutation_policy": arandur_gate_mutation_policy(),
        "forbidden_actions_confirmed": [
            "no_canonical_queue_append",
            "no_canonical_queue_mutation_without_approval_packet",
            "no_service_restart",
            "no_destructive_operation"
        ]
    }))
}

fn report_arandur_gate_blocked(root: &Path) -> anyhow::Result<serde_json::Value> {
    let candidates = arandur_gate_candidates(root)?;
    let mut blocked_groups: BTreeMap<String, usize> = BTreeMap::new();
    let mut class_groups: BTreeMap<String, usize> = BTreeMap::new();

    for candidate in &candidates {
        if let Some(reason) = candidate
            .get("blocked_reason_code")
            .and_then(serde_json::Value::as_str)
        {
            *blocked_groups.entry(reason.to_string()).or_insert(0) += 1;
        }
        if let Some(governance_class) = candidate
            .get("governance_class")
            .and_then(serde_json::Value::as_str)
        {
            *class_groups
                .entry(governance_class.to_string())
                .or_insert(0) += 1;
        }
    }

    Ok(json!({
        "contract": "arda.arandur.gate_blocked.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "status": "blocked_candidates_classified",
        "blocked_groups": blocked_groups,
        "class_groups": class_groups,
        "candidates": candidates,
        "mutation_policy": arandur_gate_mutation_policy()
    }))
}

fn approve_arandur_gate_candidate(
    root: &Path,
    objective_id: &str,
    justification: &str,
) -> anyhow::Result<serde_json::Value> {
    append_arandur_gate_decision(root, objective_id, justification, "approved")
}

fn deny_arandur_gate_candidate(
    root: &Path,
    objective_id: &str,
    justification: &str,
) -> anyhow::Result<serde_json::Value> {
    append_arandur_gate_decision(root, objective_id, justification, "denied")
}

fn append_arandur_gate_decision(
    root: &Path,
    objective_id: &str,
    justification: &str,
    decision: &str,
) -> anyhow::Result<serde_json::Value> {
    anyhow::ensure!(!objective_id.trim().is_empty(), "objective id is required");
    anyhow::ensure!(
        !justification.trim().is_empty(),
        "justification is required"
    );

    let candidate = arandur_gate_candidates(root)?
        .into_iter()
        .find(|candidate| candidate["objective_id"] == objective_id)
        .with_context(|| format!("no Arandur gate candidate found for objective {objective_id}"))?;
    let contract = if decision == "approved" {
        "arda.arandur.gate_approval.v1"
    } else {
        "arda.arandur.gate_denial.v1"
    };
    let recorded_status = if decision == "approved" {
        "approval_packet_recorded"
    } else {
        "denial_packet_recorded"
    };
    if arandur_gate_decision_exists(root, objective_id, decision)? {
        return Ok(json!({
            "contract": contract,
            "authority": "operator_decision",
            "review_required": false,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "status": "already_recorded_idempotent_noop",
            "objective_id": objective_id,
            "decision": decision,
            "candidate": candidate,
            "mutation_policy": arandur_gate_mutation_policy()
        }));
    }

    let approval_id = format!(
        "arandur_gate_{}_{}",
        decision,
        &sha1_hex(objective_id)[..12]
    );
    let now = Utc::now().to_rfc3339();
    let record = json!({
        "objective_id": objective_id,
        "title": candidate.get("title").cloned().unwrap_or(serde_json::Value::Null),
        "status": "candidate",
        "action_class": candidate.get("action_class").cloned().unwrap_or(serde_json::Value::Null),
        "authority": "operator_decision",
        "review_required": false,
        "governance_class": if decision == "approved" { "operator_approved" } else { "operator_denied" },
        "blocked_reason_code": candidate.get("blocked_reason_code").cloned().unwrap_or(serde_json::Value::Null),
        "approval_packet": {
            "id": approval_id,
            "approval_id": approval_id,
            "status": decision,
            "objective_id": objective_id,
            "operator_justification": justification,
            "approved_at_utc": if decision == "approved" { now.as_str() } else { "" },
            "denied_at_utc": if decision == "denied" { now.as_str() } else { "" }
        },
        "created_at_utc": now,
        "source": "arandur_gate_cli"
    });
    append_jsonl_record_locked(&root.join(ARANDUR_RECOMMENDATIONS_PATH), &record)?;

    Ok(json!({
        "contract": contract,
        "authority": "operator_decision",
        "review_required": false,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "status": recorded_status,
        "objective_id": objective_id,
        "decision": decision,
        "appended_record": record,
        "mutation_policy": arandur_gate_mutation_policy()
    }))
}

fn arandur_gate_decision_exists(
    root: &Path,
    objective_id: &str,
    decision: &str,
) -> anyhow::Result<bool> {
    let records = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    Ok(records.iter().any(|record| {
        record
            .get("objective_id")
            .and_then(serde_json::Value::as_str)
            == Some(objective_id)
            && record
                .get("approval_packet")
                .and_then(|packet| packet.get("status"))
                .and_then(serde_json::Value::as_str)
                == Some(decision)
    }))
}

fn arandur_gate_candidates(root: &Path) -> anyhow::Result<Vec<serde_json::Value>> {
    let queue_effective_status = effective_queue_status_by_id(root)?;
    let recommendations = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    let mut candidates = Vec::new();

    for (index, record) in recommendations.iter().enumerate() {
        if record
            .get("objective_id")
            .and_then(serde_json::Value::as_str)
            .is_none()
        {
            continue;
        }
        candidates.push(classify_arandur_gate_candidate(
            index,
            record,
            &queue_effective_status,
        ));
    }
    Ok(candidates)
}

fn classify_arandur_gate_candidate(
    index: usize,
    record: &serde_json::Value,
    queue_effective_status: &BTreeMap<String, String>,
) -> serde_json::Value {
    let objective_id = record
        .get("objective_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let action_class = record
        .get("action_class")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let approval_packet = record.get("approval_packet").cloned();
    let approval_status = approval_packet
        .as_ref()
        .and_then(|packet| packet.get("status"))
        .and_then(serde_json::Value::as_str);
    let approval_packet_id = approval_packet
        .as_ref()
        .filter(|_| approval_status == Some("approved"))
        .and_then(|packet| packet.get("id").or_else(|| packet.get("approval_id")))
        .and_then(serde_json::Value::as_str);
    let queue_status = queue_effective_status.get(objective_id).map(String::as_str);
    let stale = matches!(queue_status, Some("completed" | "cancelled" | "superseded"));
    let safe_review_gated_action = matches!(
        action_class,
        "recommendation_ledger_append"
            | "automation_gate_selection_packet"
            | "hades_organization_packet"
    );
    let unsafe_action = matches!(
        action_class,
        "canonical_queue_write" | "destructive_apply" | "service_restart"
    );

    let (governance_class, blocked_reason_code) = if approval_status == Some("approved") {
        ("operator_approved", None)
    } else if approval_status == Some("denied") {
        ("unsafe_blocked", Some("operator_denied"))
    } else if stale {
        (
            "stale_superseded_raw_queue_record",
            Some("stale_or_superseded_queue_record"),
        )
    } else if unsafe_action {
        ("unsafe_blocked", Some("unsafe_action_class"))
    } else if !safe_review_gated_action {
        ("unknown_action_class", Some("unknown_action_class"))
    } else {
        (
            "review_gated_recommendation",
            Some("operator_approval_packet_missing"),
        )
    };

    json!({
        "record_index": index,
        "objective_id": objective_id,
        "title": record.get("title").cloned().unwrap_or(serde_json::Value::Null),
        "action_class": action_class,
        "authority": record.get("authority").cloned().unwrap_or(serde_json::Value::Null),
        "review_required": record.get("review_required").cloned().unwrap_or(serde_json::Value::Null),
        "queue_effective_status": queue_status,
        "governance_class": governance_class,
        "blocked_reason_code": blocked_reason_code,
        "approval_packet_id": approval_packet_id,
        "approval_packet": approval_packet
    })
}

fn effective_queue_status_by_id(root: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let records = read_jsonl_values(&root.join(TASK_QUEUE_PATH))?;
    let mut statuses = BTreeMap::new();
    for record in records {
        if let (Some(id), Some(status)) = (
            json_id(&record),
            record.get("status").and_then(serde_json::Value::as_str),
        ) {
            statuses.insert(id.to_string(), status.to_string());
        }
    }
    Ok(statuses)
}

fn arandur_gate_mutation_policy() -> serde_json::Value {
    json!({
        "canonical_queue_mutation_allowed": false,
        "approval_decisions_append_only": true,
        "approval_decision_ledger": ARANDUR_RECOMMENDATIONS_PATH,
        "read_only_autopilot_compatible": true
    })
}

fn is_arandur_mission_review_task(record: &serde_json::Value) -> bool {
    let title_matches = record
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(|title| title.contains("Review mission candidate"))
        .unwrap_or(false);
    let source_matches = record
        .get("source")
        .and_then(serde_json::Value::as_str)
        .map(|source| source.contains("arandur"))
        .unwrap_or(false);
    let phase_matches = record
        .get("phase")
        .and_then(serde_json::Value::as_str)
        .map(|phase| phase == "6J")
        .unwrap_or(false);
    title_matches && source_matches && phase_matches
}

fn map_arandur_system(root: &Path) -> anyhow::Result<serde_json::Value> {
    let observation = observe_arandur(root)?;
    let readiness = assess_arandur_readiness(root)?;
    let packet_review = review_arandur_packet(root, &root.join(DEFAULT_PHASE2F_PACKET_DIR)).ok();
    let mutation_classes = report_bounded_mutation_classes(root)?;
    let governance_runtime =
        read_json_file_optional(&root.join("core/state/governance_runtime.json"))?;
    let athena_intake =
        summarize_jsonl_file(&root.join("data/athena/packet_intake_phase2f_2026-05-17.jsonl"))?;
    let promotion_surface_path = root
        .join(DEFAULT_PHASE2F_PACKET_DIR)
        .join("ATHENA_PACKET_PROMOTION_SURFACE.md");

    let authority_order = [
        "ARDA_ROOT_PROTOCOL.md",
        "core/realm/arda.toml",
        "core/realm/agents.toml",
        "docs/governance/SOUL.md",
        "docs/governance/AGENTS.md",
        "core/state/governance_runtime.json",
        "docs/operations/ARANDUR_PROTOCOL.md",
        "core/state/arandur/runtime.json",
        "data/arandur/episodes.jsonl",
        "data/arandur/recommendations.jsonl",
        DEFAULT_PHASE2F_PACKET_DIR,
        "core/projects/tasks/queue.jsonl",
    ];
    let authority_paths: Vec<serde_json::Value> = authority_order
        .iter()
        .map(|path| {
            json!({
                "path": path,
                "present": root.join(path).exists()
            })
        })
        .collect();
    let sovereign_governance_present = [
        "ARDA_ROOT_PROTOCOL.md",
        "core/realm/arda.toml",
        "core/realm/agents.toml",
        "docs/governance/SOUL.md",
        "docs/governance/AGENTS.md",
    ]
    .iter()
    .all(|path| root.join(path).exists());

    let runtime_state = observation["arandur"]["runtime_state"].clone();
    let current_level = readiness
        .get("current_level")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let queue_records = observation["queue"]["total_records"].as_u64().unwrap_or(0);
    let episode_records = observation["arandur"]["episode_count"]
        .as_u64()
        .unwrap_or(0);
    let recommendation_records = observation["arandur"]["recommendation_count"]
        .as_u64()
        .unwrap_or(0);
    let packet_gates_clean = packet_review
        .as_ref()
        .and_then(|review| review.get("promotion_gates"))
        .map(|gates| {
            gate_bool(gates, "evidence_present")
                && gate_bool(gates, "raw_inbox_read_only")
                && gate_bool(gates, "research_claims_review_gated")
                && gate_bool(gates, "json_and_jsonl_validated")
        })
        .unwrap_or(false);
    let triad_path = root.join("crates/arda-governance/src/triad.rs");
    let resonance_path = root.join("crates/arda-governance/src/resonance.rs");
    let plutus_path = root.join("crates/arda-plutus/src/lib.rs");
    let triad_bytes = file_byte_len(&triad_path)?;
    let resonance_bytes = file_byte_len(&resonance_path)?;
    let joulework_bytes = file_byte_len(&plutus_path)?;

    Ok(json!({
        "contract": "arda.arandur.system_map.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "mutation_policy": "read_only_system_mapping",
        "scope": "arandur_prometheus_athena_governance_surfaces",
        "autonomy": {
            "current_level": current_level,
            "level_2_active": readiness["level_2_active"],
            "level_2_ready": readiness["level_2_ready"],
            "runtime_state": runtime_state
        },
        "authority_stack": {
            "order": authority_paths,
            "sovereign_governance_present": sovereign_governance_present,
            "runtime_governance_present": governance_runtime.is_some(),
            "arandur_protocol_present": root.join("docs/operations/ARANDUR_PROTOCOL.md").exists(),
            "governance_layering": "sovereign_root_then_runtime_policy_then_arandur_protocol_then_review_gated_ledgers_then_canonical_queue"
        },
        "prometheus": {
            "owner": "prometheus",
            "loop": ["observe", "orient", "decide", "delegate", "verify", "reflect", "update_state"],
            "autopilot_module_present": root.join("crates/arda-prometheus/src/autopilot/mod.rs").exists(),
            "cli_anchor_present": root.join("crates/arda-cli/src/commands/arandur.rs").exists()
        },
        "athena": {
            "phase2f_packet": observation["phase2f_packet"],
            "packet_review": packet_review,
            "packet_intake": athena_intake,
            "promotion_surface": {
                "path": display_path(&promotion_surface_path),
                "present": promotion_surface_path.exists(),
                "bytes": file_byte_len(&promotion_surface_path)?
            }
        },
        "governance": {
            "runtime_state_path": "core/state/governance_runtime.json",
            "runtime_state_present": governance_runtime.is_some(),
            "layering_gates": {
                "sovereign_governance_present": sovereign_governance_present,
                "arandur_runtime_review_gated": gate_bool(&readiness["promotion_gates"], "runtime_review_gated"),
                "raw_inbox_read_only": true,
                "canonical_queue_mutation_allowed": false,
                "packet_promotion_gates_clean": packet_gates_clean,
                "research_claims_review_gated": gate_bool(&readiness["promotion_gates"], "research_claims_review_gated"),
                "git_diff_check_required_before_promotion": true
            },
            "signals": {
                "triad_quorum_required_for_canonical_promotion": true,
                "resonance_review_required_for_autonomy_expansion": true,
                "human_review_required_for_research_or_canonical_claims": true
            },
            "crate_layers": {
                "triad": {
                    "path": "crates/arda-governance/src/triad.rs",
                    "present": triad_path.exists(),
                    "bytes": triad_bytes,
                    "usage": "truth_and_quorum_gate_for_canonical_promotion"
                },
                "resonance": {
                    "path": "crates/arda-governance/src/resonance.rs",
                    "present": resonance_path.exists(),
                    "bytes": resonance_bytes,
                    "usage": "love_equation_resonance_gate_for_autonomy_expansion"
                },
                "joulework": {
                    "path": "crates/arda-plutus/src/lib.rs",
                    "present": plutus_path.exists(),
                    "bytes": joulework_bytes,
                    "usage": "energy_cost_and_budget_gate_for_candidate_actions"
                }
            }
        },
        "ledgers": {
            "canonical_task_queue": observation["queue"],
            "episodes": summarize_jsonl_file(&root.join(ARANDUR_EPISODES_PATH))?,
            "recommendations": summarize_arandur_recommendations(root)?,
            "mutation_evidence": summarize_jsonl_file(&root.join(ARANDUR_MUTATION_EVIDENCE_PATH))?
        },
        "bounded_mutations": mutation_classes,
        "counts": {
            "queue_records": queue_records,
            "episode_records": episode_records,
            "recommendation_records": recommendation_records
        },
        "forbidden_actions_confirmed": observation["forbidden_actions_confirmed"]
    }))
}

pub(crate) fn scan_arandur_improvements(root: &Path) -> anyhow::Result<serde_json::Value> {
    let system_map = map_arandur_system(root)?;
    let readiness = assess_arandur_readiness(root)?;
    let gates = readiness
        .get("promotion_gates")
        .unwrap_or(&serde_json::Value::Null);
    let counts = system_map.get("counts").unwrap_or(&serde_json::Value::Null);
    let mut improvements = Vec::new();

    if !gate_bool(gates, "runtime_review_gated") {
        improvements.push(build_improvement_candidate(
            "arandur_runtime_review_gate",
            "Initialize or repair Arandur runtime state so authority and review_required metadata are explicit.",
            "governance",
            "core/state/arandur/runtime.json",
            "prometheus arandur initialize",
        ));
    }
    if !gate_bool(gates, "minimum_clean_dry_runs_met") {
        improvements.push(build_improvement_candidate(
            "arandur_clean_dry_run_evidence",
            "Record additional clean supervised dry-run episodes before expanding autonomy.",
            "benchmark",
            ARANDUR_EPISODES_PATH,
            "prometheus arandur benchmark --packet-dir audit/HUMAN_INBOX_PHASE2F_2026-05-17",
        ));
    }
    if !gate_bool(gates, "recommendation_review_surface_present") {
        improvements.push(build_improvement_candidate(
            "arandur_recommendation_surface",
            "Create an append-only review-gated recommendation surface from ATHENA packet candidates.",
            "athena_bridge",
            ARANDUR_RECOMMENDATIONS_PATH,
            "prometheus arandur recommend-next --dry-run",
        ));
    }
    if !gate_bool(gates, "packet_promotion_gates_clean") {
        improvements.push(build_improvement_candidate(
            "athena_packet_gate_cleanup",
            "Clean packet metadata/evidence gates before any canonical promotion review.",
            "athena_bridge",
            DEFAULT_PHASE2F_PACKET_DIR,
            "prometheus arandur review-packet --packet-dir audit/HUMAN_INBOX_PHASE2F_2026-05-17",
        ));
    }
    if !system_map["authority_stack"]["runtime_governance_present"]
        .as_bool()
        .unwrap_or(false)
    {
        improvements.push(build_improvement_candidate(
            "governance_runtime_policy_surface",
            "Provide machine-readable governance_runtime policy before broader autonomous enforcement.",
            "governance",
            "core/state/governance_runtime.json",
            "review governance_runtime policy requirements",
        ));
    }
    if counts["recommendation_records"].as_u64().unwrap_or(0) == 0 {
        improvements.push(build_improvement_candidate(
            "arandur_first_recommendation_batch",
            "Generate a first review-required recommendation batch without mutating the canonical queue.",
            "prometheus",
            ARANDUR_RECOMMENDATIONS_PATH,
            "prometheus arandur recommend-next",
        ));
    }

    let blocked_by_review_gate = !readiness["readiness_gate_stack_clean"]
        .as_bool()
        .unwrap_or(false);
    Ok(json!({
        "contract": "arda.arandur.improvement_scan.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "mutation_policy": "read_only_recommendations_no_state_mutation",
        "source_system_map_contract": system_map["contract"],
        "current_level": readiness["current_level"],
        "blocked_by_review_gate": blocked_by_review_gate,
        "promotion_gates": readiness["promotion_gates"],
        "candidate_count": improvements.len(),
        "improvements": improvements,
        "next_recommended_command": if blocked_by_review_gate {
            "prometheus arandur benchmark --packet-dir audit/HUMAN_INBOX_PHASE2F_2026-05-17"
        } else {
            "prometheus arandur readiness"
        },
        "forbidden_actions_confirmed": [
            "no_raw_human_inbox_mutation",
            "no_canonical_queue_append",
            "no_service_restart",
            "no_destructive_operation"
        ]
    }))
}

fn build_improvement_candidate(
    candidate_id: &str,
    summary: &str,
    domain: &str,
    evidence_path: &str,
    recommended_command: &str,
) -> serde_json::Value {
    json!({
        "candidate_id": candidate_id,
        "authority": "agent_generated",
        "review_required": true,
        "domain": domain,
        "summary": summary,
        "evidence": [evidence_path],
        "recommended_command": recommended_command,
        "mutation_policy": {
            "raw_human_inbox": "read_only",
            "canonical_queue": "not_mutated_by_improvement_scan",
            "research_claims": "review_gated",
            "services": "no_restart_or_destructive_action"
        }
    })
}

fn plan_arandur_phase6b_scout(
    root: &Path,
    scope: &str,
    limit: usize,
) -> anyhow::Result<serde_json::Value> {
    let runtime_state = read_json_file_optional(&root.join(ARANDUR_RUNTIME_PATH))?;
    let current_level = runtime_state
        .as_ref()
        .and_then(|value| value.get("autonomy_level"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let readiness = assess_arandur_readiness(root)?;
    let queue_records = read_jsonl_values(&root.join(TASK_QUEUE_PATH))?;
    let recommendation_records = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    let episode_summary = summarize_jsonl_file(&root.join(ARANDUR_EPISODES_PATH))?;
    let safe_limit = limit.clamp(1, 10);
    let mut candidate_missions = Vec::new();

    for recommendation in recommendation_records
        .iter()
        .filter(|value| review_gated_value(value))
    {
        if candidate_missions.len() >= safe_limit {
            break;
        }
        let seed = recommendation
            .get("recommended_candidate_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                recommendation
                    .get("recommendation_id")
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or("recommendation_seed");
        let summary = json_text_field(recommendation, "summary")
            .or_else(|| json_text_field(recommendation, "title"))
            .unwrap_or_else(|| format!("Review-gated scout mission candidate from {seed}"));
        candidate_missions.push(build_phase6b_scout_mission(
            seed,
            &summary,
            scope,
            ARANDUR_RECOMMENDATIONS_PATH,
            "recommendation_ledger",
        ));
    }

    for task in queue_records.iter() {
        if candidate_missions.len() >= safe_limit {
            break;
        }
        let status = json_text_field(task, "status").unwrap_or_default();
        if matches!(status.as_str(), "completed" | "cancelled") {
            continue;
        }
        let seed = json_id(task).unwrap_or("queue_seed");
        let title = json_text_field(task, "title")
            .or_else(|| json_text_field(task, "summary"))
            .unwrap_or_else(|| format!("Read-only scout planning for queue item {seed}"));
        candidate_missions.push(build_phase6b_scout_mission(
            seed,
            &title,
            scope,
            TASK_QUEUE_PATH,
            "canonical_queue_read_only",
        ));
    }

    while candidate_missions.len() < safe_limit {
        let seed = format!("phase6b_scout_{}", candidate_missions.len() + 1);
        let summary = format!("Plan a bounded read-only scout mission for {scope}");
        candidate_missions.push(build_phase6b_scout_mission(
            &seed,
            &summary,
            scope,
            "docs/operations/ARANDUR_PROTOCOL.md",
            "protocol_default",
        ));
    }

    Ok(json!({
        "contract": "arda.arandur.phase6b_scout_plan.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "phase": "6B",
        "scope": scope,
        "current_level": current_level,
        "mutation_policy": "read_only_mission_planning_no_network_calls",
        "scout_policy": {
            "internet_access_performed": false,
            "execution_allowed": false,
            "network_calls_allowed_by_command": false,
            "raw_human_inbox": "read_only",
            "canonical_queue": "read_only_not_mutated",
            "recommendation_ledger": "read_only_not_appended",
            "episode_ledger": "read_only_not_appended",
            "research_market_legal_implementation_claims": "review_gated",
            "services": "no_restart_or_destructive_action"
        },
        "source_surfaces": {
            "runtime_state": ARANDUR_RUNTIME_PATH,
            "task_queue": TASK_QUEUE_PATH,
            "recommendation_ledger": ARANDUR_RECOMMENDATIONS_PATH,
            "episode_ledger": ARANDUR_EPISODES_PATH
        },
        "evidence_summary": {
            "queue_records": queue_records.len(),
            "recommendation_records": recommendation_records.len(),
            "episode_records": episode_summary["total_records"],
            "runtime_state_present": runtime_state.is_some(),
            "readiness_gate_stack_clean": readiness["readiness_gate_stack_clean"]
        },
        "candidate_count": candidate_missions.len(),
        "candidate_missions": candidate_missions,
        "required_human_review_before_execution": [
            "approve scout objective and data sources",
            "verify legal/market/research claims before promotion",
            "choose explicit execution tool outside this planning command",
            "record post-scout evidence as review-gated episode before state promotion"
        ],
        "forbidden_actions_confirmed": [
            "no_internet_access_performed",
            "no_raw_human_inbox_mutation",
            "no_canonical_queue_append",
            "no_recommendation_or_episode_append",
            "no_service_restart",
            "no_destructive_operation"
        ]
    }))
}

fn build_phase6b_scout_mission(
    seed: &str,
    summary: &str,
    scope: &str,
    evidence_path: &str,
    source_kind: &str,
) -> serde_json::Value {
    json!({
        "mission_id": format!("phase6b_scout_{}", sanitize_identifier(seed)),
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6B",
        "scope": scope,
        "source_kind": source_kind,
        "summary": summary,
        "evidence": [evidence_path],
        "allowed_steps": [
            "define scout question",
            "list candidate public sources for human review",
            "draft evidence fields and validation gates",
            "stop before network access, queue mutation, or claim promotion"
        ],
        "forbidden_steps": [
            "automatic internet browsing",
            "raw human inbox mutation",
            "canonical queue append",
            "ungated research, market, legal, or implementation claim promotion",
            "service restart or destructive operation"
        ],
        "completion_gate": {
            "human_review_required": true,
            "evidence_required": true,
            "claims_review_gated": true,
            "json_contract_required": "arda.arandur.phase6b_scout_plan.v1"
        }
    })
}

fn execute_arandur_phase6c_scout(
    root: &Path,
    mission_id: &str,
    scope: &str,
    source_urls: Vec<String>,
    evidence_path: Option<&Path>,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let mission_id = mission_id.trim();
    anyhow::ensure!(!mission_id.is_empty(), "--mission-id is required");
    if write_report {
        let approved = justification
            .map(str::trim)
            .filter(|value| !value.is_empty());
        anyhow::ensure!(
            approved.is_some(),
            "--justification is required when --write is set"
        );
    }

    let evidence_content = match evidence_path {
        Some(path) => Some(read_text_file_required(&root.join(path))?),
        None => None,
    };
    let evidence_sha1 = evidence_content.as_deref().map(sha1_hex);
    let evidence_preview = evidence_content
        .as_deref()
        .map(|content| preview_text(content, 240))
        .unwrap_or_else(|| "no evidence file supplied; finding remains review-only".to_string());
    let normalized_sources = normalize_source_urls(source_urls);
    let finding_id = phase6c_finding_id(
        mission_id,
        scope,
        &normalized_sources,
        evidence_sha1.as_deref(),
    );
    let now = Utc::now();
    let candidate = json!({
        "contract": "arda.arandur.phase6c_scout_finding.v1",
        "finding_id": finding_id,
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6C",
        "mission_id": mission_id,
        "scope": scope,
        "source_urls": normalized_sources,
        "evidence_file": evidence_path.map(display_path),
        "evidence_sha1": evidence_sha1,
        "evidence_preview": evidence_preview,
        "claim_policy": {
            "research_claims_review_gated": true,
            "market_claims_review_gated": true,
            "legal_claims_review_gated": true,
            "implementation_claims_review_gated": true
        },
        "mutation_policy": {
            "canonical_queue": "not_mutated_by_phase6c_scout_execute",
            "raw_human_inbox": "read_only",
            "output_ledger": ARANDUR_SCOUT_FINDINGS_PATH
        },
        "created_at_utc": now.to_rfc3339(),
        "write_justification": justification.map(str::trim)
    });

    let ledger_path = root.join(ARANDUR_SCOUT_FINDINGS_PATH);
    let existing = read_jsonl_values(&ledger_path)?;
    let already_recorded = existing.iter().any(|record| {
        record
            .get("finding_id")
            .and_then(serde_json::Value::as_str)
            .map(|id| id == candidate["finding_id"].as_str().unwrap_or_default())
            .unwrap_or(false)
    });
    let mut appended = false;
    if write_report && !already_recorded {
        append_jsonl_record_locked(&ledger_path, &candidate)?;
        appended = true;
    }

    let status = if write_report {
        if appended {
            "scout_finding_recorded"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6c_scout_execution.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6C",
        "status": status,
        "write_requested": write_report,
        "write_performed": appended,
        "ledger_path": ARANDUR_SCOUT_FINDINGS_PATH,
        "candidate_finding": candidate,
        "scout_policy": {
            "network_access_performed_by_command": false,
            "external_sources_recorded_as_citations_only": true,
            "research_claims_review_gated": true,
            "canonical_queue_mutation_allowed": false,
            "raw_human_inbox_mutation_allowed": false
        }
    }))
}

fn synthesize_arandur_phase6d_patterns(
    root: &Path,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    if write_report {
        let approved = justification
            .map(str::trim)
            .filter(|value| !value.is_empty());
        anyhow::ensure!(
            approved.is_some(),
            "--justification is required when --write is set"
        );
    }

    let queue_records = read_jsonl_values(&root.join(TASK_QUEUE_PATH))?;
    let recommendation_records = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    let scout_records = read_jsonl_values(&root.join(ARANDUR_SCOUT_FINDINGS_PATH))?;
    let episode_records = read_jsonl_values(&root.join(ARANDUR_EPISODES_PATH))?;
    let owner_counts = count_json_string_field(&queue_records, "owner");
    let recommendation_domains = count_json_string_field(&recommendation_records, "domain");
    let scout_scopes = count_json_string_field(&scout_records, "scope");
    let episode_types = count_json_string_field(&episode_records, "episode_type");
    let patterns = build_phase6d_patterns(
        &owner_counts,
        &recommendation_domains,
        &scout_scopes,
        &episode_types,
    );
    let fingerprint = phase6d_pattern_fingerprint(&patterns);
    let now = Utc::now();
    let record = json!({
        "contract": "arda.arandur.phase6d_pattern_synthesis_record.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6D",
        "pattern_synthesis_id": format!("phase6d_patterns_{fingerprint}"),
        "fingerprint": fingerprint,
        "created_at_utc": now.to_rfc3339(),
        "write_justification": justification.map(str::trim),
        "input_counts": {
            "queue_records": queue_records.len(),
            "recommendation_records": recommendation_records.len(),
            "scout_finding_records": scout_records.len(),
            "episode_records": episode_records.len()
        },
        "patterns": patterns,
        "recommended_next_actions": [
            "human review synthesized patterns before mission creation",
            "convert only approved synthesis items into review-gated recommendations",
            "keep canonical task queue unchanged until explicit promotion"
        ]
    });

    let ledger_path = root.join(ARANDUR_PATTERN_SYNTHESIS_PATH);
    let existing = read_jsonl_values(&ledger_path)?;
    let already_recorded = existing.iter().any(|item| {
        item.get("fingerprint")
            .and_then(serde_json::Value::as_str)
            .map(|value| value == record["fingerprint"].as_str().unwrap_or_default())
            .unwrap_or(false)
    });
    let mut appended = false;
    if write_report && !already_recorded {
        append_jsonl_record_locked(&ledger_path, &record)?;
        appended = true;
    }
    let status = if write_report {
        if appended {
            "patterns_recorded"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6d_pattern_synthesis.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6D",
        "status": status,
        "write_requested": write_report,
        "write_performed": appended,
        "ledger_path": ARANDUR_PATTERN_SYNTHESIS_PATH,
        "input_counts": record["input_counts"].clone(),
        "patterns": record["patterns"].clone(),
        "record": record,
        "pattern_policy": {
            "canonical_queue_mutation_allowed": false,
            "raw_human_inbox_mutation_allowed": false,
            "research_claims_review_gated": true,
            "output_append_only": true
        }
    }))
}

fn promote_arandur_phase6e_missions(
    root: &Path,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    if write_report {
        let approved = justification
            .map(str::trim)
            .filter(|value| !value.is_empty());
        anyhow::ensure!(
            approved.is_some(),
            "--justification is required when --write is set"
        );
    }

    let scout_records = read_jsonl_values(&root.join(ARANDUR_SCOUT_FINDINGS_PATH))?;
    let pattern_records = read_jsonl_values(&root.join(ARANDUR_PATTERN_SYNTHESIS_PATH))?;
    let recommendation_records = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    let candidate_missions = build_phase6e_mission_candidates(
        &scout_records,
        &pattern_records,
        &recommendation_records,
        justification,
    );

    let ledger_path = root.join(ARANDUR_MISSION_CANDIDATES_PATH);
    let appended = if write_report {
        append_new_mission_candidate_records_locked(&ledger_path, candidate_missions.clone())?
    } else {
        Vec::new()
    };

    let status = if write_report {
        if !appended.is_empty() {
            "mission_candidates_recorded"
        } else if candidate_missions.is_empty() {
            "no_approved_patterns_available"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6e_mission_promotion.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6E",
        "status": status,
        "write_requested": write_report,
        "write_performed": !appended.is_empty(),
        "ledger_path": ARANDUR_MISSION_CANDIDATES_PATH,
        "input_counts": {
            "scout_finding_records": scout_records.len(),
            "pattern_synthesis_records": pattern_records.len(),
            "recommendation_records": recommendation_records.len()
        },
        "candidate_missions": candidate_missions,
        "appended_candidates": appended,
        "promotion_policy": {
            "dry_run_default": true,
            "append_only_with_write": true,
            "justification_required_for_write": true,
            "canonical_queue_mutation_allowed": false,
            "raw_human_inbox_mutation_allowed": false,
            "research_claims_review_gated": true,
            "mission_packets_are_canonical_queue_entries": false
        }
    }))
}

fn build_phase6e_mission_candidates(
    scout_records: &[serde_json::Value],
    pattern_records: &[serde_json::Value],
    recommendation_records: &[serde_json::Value],
    justification: Option<&str>,
) -> Vec<serde_json::Value> {
    let approved_recommendations: Vec<&serde_json::Value> = recommendation_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .collect();
    let approved_scouts: Vec<&serde_json::Value> = scout_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .collect();
    let mut candidates = Vec::new();
    let now = Utc::now().to_rfc3339();

    for pattern_record in pattern_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
    {
        let synthesis_id = json_text_field(pattern_record, "pattern_synthesis_id")
            .unwrap_or_else(|| "phase6d_patterns_unknown".to_string());
        let Some(patterns) = pattern_record
            .get("patterns")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };

        for pattern in patterns
            .iter()
            .filter(|pattern| agent_generated_review_gated_value(pattern))
        {
            let pattern_id = json_text_field(pattern, "pattern_id")
                .unwrap_or_else(|| "approved_pattern".to_string());
            if pattern_id == "insufficient_evidence" {
                continue;
            }
            let kind =
                json_text_field(pattern, "kind").unwrap_or_else(|| "approved_pattern".to_string());
            let summary = json_text_field(pattern, "summary")
                .unwrap_or_else(|| "Approved review-gated Phase 6D pattern.".to_string());
            let recommendation_ids = phase6e_recommendation_ids(&approved_recommendations);
            let scout_ids = phase6e_scout_ids(&approved_scouts);
            let mission_candidate_id = phase6e_mission_candidate_id(
                &synthesis_id,
                &pattern_id,
                &recommendation_ids,
                &scout_ids,
            );
            candidates.push(json!({
                "contract": "arda.arandur.phase6e_mission_candidate.v1",
                "mission_candidate_id": mission_candidate_id,
                "authority": "agent_generated",
                "review_required": true,
                "phase": "6E",
                "created_at_utc": now,
                "write_justification": justification.map(str::trim),
                "source_pattern_synthesis_id": synthesis_id,
                "source_pattern_id": pattern_id,
                "source_pattern_kind": kind,
                "supporting_recommendation_ids": recommendation_ids,
                "supporting_scout_finding_ids": scout_ids,
                "title": phase6e_candidate_title(pattern),
                "scope": phase6e_candidate_scope(pattern, &approved_scouts),
                "objective": format!("Prepare a bounded human-reviewed mission packet for this approved pattern: {summary}"),
                "evidence": phase6e_evidence(&approved_scouts),
                "allowed_steps": [
                    "draft mission packet from approved Phase 6D pattern",
                    "cite review-gated scout findings and recommendations",
                    "request human approval before any canonical task queue mutation",
                    "keep market, research, legal, and implementation claims review-gated"
                ],
                "forbidden_steps": [
                    "canonical task queue append",
                    "raw human inbox mutation",
                    "ungated public-internet claim promotion",
                    "service restart or destructive operation"
                ],
                "mutation_policy": {
                    "canonical_queue": "not_mutated_by_phase6e_mission_promotion",
                    "raw_human_inbox": "read_only",
                    "output_ledger": ARANDUR_MISSION_CANDIDATES_PATH,
                    "append_only": true
                },
                "promotion_gate": {
                    "human_review_required": true,
                    "claims_review_gated": true,
                    "canonical_queue_mutation_allowed": false,
                    "requires_explicit_future_operator_promotion": true
                }
            }));
        }
    }

    candidates
}

fn review_arandur_phase6f_mission_candidates(
    root: &Path,
    candidate_id: Option<&str>,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let approved_justification = justification
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if write_report {
        anyhow::ensure!(
            approved_justification.is_some(),
            "--justification is required when --write is set"
        );
    }

    let requested_candidate_id = candidate_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let candidate_records = read_jsonl_values(&root.join(ARANDUR_MISSION_CANDIDATES_PATH))?;
    let review_packets = build_phase6f_mission_review_packets(
        &candidate_records,
        requested_candidate_id,
        approved_justification,
    );

    let ledger_path = root.join(ARANDUR_MISSION_REVIEWS_PATH);
    let appended = if write_report {
        append_new_mission_review_records_locked(&ledger_path, review_packets.clone())?
    } else {
        Vec::new()
    };

    let status = if write_report {
        if !appended.is_empty() {
            "mission_reviews_recorded"
        } else if review_packets.is_empty() {
            "no_eligible_mission_candidates_available"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6f_mission_candidate_review.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6F",
        "status": status,
        "candidate_id_filter": requested_candidate_id,
        "write_requested": write_report,
        "write_performed": !appended.is_empty(),
        "ledger_path": ARANDUR_MISSION_REVIEWS_PATH,
        "input_counts": {
            "mission_candidate_records": candidate_records.len(),
            "eligible_review_packets": review_packets.len()
        },
        "review_packets": review_packets,
        "appended_reviews": appended,
        "review_policy": {
            "dry_run_default": true,
            "append_only_with_write": true,
            "justification_required_for_write": true,
            "canonical_queue_mutation_allowed": false,
            "mission_review_packets_are_canonical_queue_entries": false,
            "human_review_required_before_future_queue_mutation": true,
            "claims_review_gated": true
        }
    }))
}

fn build_phase6f_mission_review_packets(
    candidate_records: &[serde_json::Value],
    candidate_id_filter: Option<&str>,
    justification: Option<&str>,
) -> Vec<serde_json::Value> {
    let now = Utc::now().to_rfc3339();
    candidate_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .filter(|record| {
            candidate_id_filter
                .map(|candidate_id| {
                    json_text_field(record, "mission_candidate_id")
                        .map(|record_id| record_id == candidate_id)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .map(|candidate| {
            let source_candidate_id = json_text_field(candidate, "mission_candidate_id")
                .unwrap_or_else(|| "unknown_mission_candidate".to_string());
            let title = json_text_field(candidate, "title")
                .unwrap_or_else(|| "Review bounded mission candidate".to_string());
            let scope = json_text_field(candidate, "scope")
                .unwrap_or_else(|| "bounded mission candidate review".to_string());
            json!({
                "contract": "arda.arandur.phase6f_mission_review_packet.v1",
                "mission_review_id": phase6f_mission_review_id(&source_candidate_id),
                "authority": "agent_generated",
                "review_required": true,
                "phase": "6F",
                "created_at_utc": now,
                "write_justification": justification,
                "source_mission_candidate_id": source_candidate_id,
                "decision": "approved_for_bounded_mission_packet_drafting",
                "title": format!("Review mission candidate: {title}"),
                "scope": scope,
                "candidate_snapshot": candidate,
                "operator_review_checklist": [
                    "confirm cited Phase 6C/6D evidence supports the proposed mission",
                    "confirm claims remain review-gated and do not assert market certainty",
                    "confirm no canonical task queue mutation is requested by this bridge",
                    "confirm any future queue promotion requires a separate explicit operator action"
                ],
                "bounded_output": {
                    "emits_candidate_mission_packet": true,
                    "emits_canonical_task_queue_entry": false,
                    "requires_future_human_approval": true
                },
                "mutation_policy": {
                    "canonical_queue": "not_mutated_by_phase6f_mission_review",
                    "mission_candidates": "read_only",
                    "output_ledger": ARANDUR_MISSION_REVIEWS_PATH,
                    "append_only": true
                }
            })
        })
        .collect()
}

fn request_arandur_phase6g_mission_approval(
    root: &Path,
    candidate_id: Option<&str>,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let approved_justification = justification
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if write_report {
        anyhow::ensure!(
            approved_justification.is_some(),
            "--justification is required when --write is set"
        );
    }

    let queue_before = file_snapshot(root, TASK_QUEUE_PATH)?;
    let requested_candidate_id = candidate_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let review_records = read_jsonl_values(&root.join(ARANDUR_MISSION_REVIEWS_PATH))?;
    let candidate_records = read_jsonl_values(&root.join(ARANDUR_MISSION_CANDIDATES_PATH))?;
    let approval_requests = build_phase6g_mission_approval_requests(
        &review_records,
        &candidate_records,
        requested_candidate_id,
        approved_justification,
        &queue_before,
    );

    let ledger_path = root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH);
    let appended = if write_report {
        append_new_mission_approval_request_records_locked(&ledger_path, approval_requests.clone())?
    } else {
        Vec::new()
    };
    let queue_after = file_snapshot(root, TASK_QUEUE_PATH)?;
    let canonical_queue_unchanged = queue_before == queue_after;

    let status = if write_report {
        if !appended.is_empty() {
            "mission_approval_requests_recorded"
        } else if approval_requests.is_empty() {
            "no_eligible_mission_reviews_available"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6g_mission_approval_request_surface.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6G",
        "status": status,
        "candidate_id_filter": requested_candidate_id,
        "write_requested": write_report,
        "write_performed": !appended.is_empty(),
        "ledger_path": ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
        "input_counts": {
            "mission_review_records": review_records.len(),
            "mission_candidate_records": candidate_records.len(),
            "eligible_approval_requests": approval_requests.len()
        },
        "queue_integrity": {
            "before": queue_before,
            "after": queue_after,
            "canonical_queue_unchanged": canonical_queue_unchanged
        },
        "approval_requests": approval_requests,
        "appended_approval_requests": appended,
        "approval_request_policy": {
            "dry_run_default": true,
            "append_only_with_write": true,
            "justification_required_for_write": true,
            "canonical_queue_mutation_allowed": false,
            "approval_requests_are_canonical_queue_entries": false,
            "human_operator_approval_required_before_future_queue_mutation": true,
            "claims_review_gated": true,
            "claim_domains_review_gated": [
                "research",
                "market",
                "legal",
                "implementation",
                "autonomy",
                "spend"
            ]
        }
    }))
}

fn build_phase6g_mission_approval_requests(
    review_records: &[serde_json::Value],
    candidate_records: &[serde_json::Value],
    candidate_id_filter: Option<&str>,
    justification: Option<&str>,
    queue_snapshot: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let now = Utc::now().to_rfc3339();
    review_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .filter(|record| {
            candidate_id_filter
                .map(|candidate_id| {
                    json_text_field(record, "source_mission_candidate_id")
                        .map(|record_id| record_id == candidate_id)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .map(|review| {
            let mission_review_id = json_text_field(review, "mission_review_id")
                .unwrap_or_else(|| "unknown_mission_review".to_string());
            let source_candidate_id = json_text_field(review, "source_mission_candidate_id")
                .unwrap_or_else(|| "unknown_mission_candidate".to_string());
            let candidate_record = find_mission_candidate_record(candidate_records, &source_candidate_id);
            let title = json_text_field(review, "title")
                .or_else(|| candidate_record.and_then(|candidate| json_text_field(candidate, "title")))
                .unwrap_or_else(|| "Approve bounded Arandur mission request".to_string());
            let scope = json_text_field(review, "scope")
                .or_else(|| candidate_record.and_then(|candidate| json_text_field(candidate, "scope")))
                .unwrap_or_else(|| "bounded mission approval request".to_string());
            let review_fingerprint = sha1_hex(&serde_json::to_string(review).unwrap_or_else(|_| mission_review_id.clone()));
            let candidate_fingerprint = candidate_record.map(|candidate| {
                sha1_hex(&serde_json::to_string(candidate).unwrap_or_else(|_| source_candidate_id.clone()))
            });
            json!({
                "contract": "arda.arandur.phase6g_mission_approval_request.v1",
                "approval_request_id": phase6g_mission_approval_request_id(&mission_review_id, &source_candidate_id),
                "authority": "agent_generated",
                "review_required": true,
                "phase": "6G",
                "created_at_utc": now,
                "justification": justification,
                "source_mission_review_id": mission_review_id,
                "source_mission_candidate_id": source_candidate_id,
                "source_fingerprints": {
                    "mission_review_sha1": review_fingerprint,
                    "mission_candidate_sha1": candidate_fingerprint,
                    "canonical_queue_sha1": queue_snapshot.get("sha1").cloned().unwrap_or(serde_json::Value::Null)
                },
                "approval_question": format!("Should a future explicit operator action promote this reviewed mission into a canonical task queue candidate? {title}"),
                "bounded_recommendation": {
                    "recommendation": "operator_review_requested_before_any_canonical_queue_mutation",
                    "title": title,
                    "scope": scope,
                    "source_decision": json_text_field(review, "decision"),
                    "future_action_required": "separate explicit human/operator approval command before canonical queue creation"
                },
                "source_review_snapshot": review,
                "source_candidate_snapshot": candidate_record,
                "operator_approval_checklist": [
                    "confirm the Phase 6F review packet is still valid",
                    "confirm all source evidence remains review-gated",
                    "confirm this request does not mutate the canonical task queue",
                    "confirm any future queue promotion is a separate explicit operator action",
                    "confirm research, market, legal, implementation, autonomy, and spend claims remain review-gated"
                ],
                "claim_gating": {
                    "research_claims_review_gated": true,
                    "market_claims_review_gated": true,
                    "legal_claims_review_gated": true,
                    "implementation_claims_review_gated": true,
                    "autonomy_claims_review_gated": true,
                    "spend_claims_review_gated": true
                },
                "mutation_policy": {
                    "canonical_queue": "not_mutated_by_phase6g_mission_approval_request",
                    "mission_reviews": "read_only",
                    "mission_candidates": "read_only",
                    "output_ledger": ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
                    "append_only": true
                },
                "bounded_output": {
                    "emits_human_approval_request": true,
                    "emits_canonical_task_queue_entry": false,
                    "requires_future_human_approval": true
                }
            })
        })
        .collect()
}

fn record_arandur_phase6g_mission_approval_decision(
    root: &Path,
    approval_request_id: &str,
    status: &str,
    justification: &str,
) -> anyhow::Result<serde_json::Value> {
    let requested_approval_id = approval_request_id.trim();
    let decision_status = status.trim();
    let approved_justification = justification.trim();
    anyhow::ensure!(
        !requested_approval_id.is_empty(),
        "--approval-request-id is required"
    );
    anyhow::ensure!(!decision_status.is_empty(), "--status is required");
    anyhow::ensure!(
        !approved_justification.is_empty(),
        "--justification is required"
    );

    let queue_before = file_snapshot(root, TASK_QUEUE_PATH)?;
    let approval_records = read_jsonl_values(&root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?;
    let source_approval = approval_records
        .iter()
        .find(|record| {
            json_text_field(record, "approval_request_id")
                .map(|record_id| record_id == requested_approval_id)
                .unwrap_or(false)
                && json_text_field(record, "phase")
                    .map(|phase| phase == "6G")
                    .unwrap_or(false)
                && agent_generated_review_gated_value(record)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "approval request {requested_approval_id} was not found as a review-gated Phase 6G request"
            )
        })?;

    let decision_id_seed = format!("{requested_approval_id}\n{decision_status}");
    let decision = json!({
        "contract": "arda.arandur.phase6g_mission_approval_decision.v1",
        "approval_decision_id": format!("phase6g_approval_decision_{}", &sha1_hex(&decision_id_seed)[..12]),
        "approval_request_id": requested_approval_id,
        "approval_status": decision_status,
        "authority": "operator_approved_agent_execution",
        "review_required": true,
        "phase": "6G",
        "created_at_utc": Utc::now().to_rfc3339(),
        "justification": approved_justification,
        "source_mission_candidate_id": json_text_field(source_approval, "source_mission_candidate_id"),
        "source_mission_review_id": json_text_field(source_approval, "source_mission_review_id"),
        "source_approval_request_sha1": sha1_hex(&serde_json::to_string(source_approval)?),
        "source_approval_request_snapshot": source_approval,
        "operator_decision_checklist": [
            "confirm the Phase 6G approval request is still valid",
            "confirm this decision is append-only and does not mutate the canonical task queue",
            "confirm any future queue write remains a separate explicit operator action",
            "confirm research, market, legal, implementation, autonomy, and spend claims remain review-gated"
        ],
        "claim_gating": {
            "research_claims_review_gated": true,
            "market_claims_review_gated": true,
            "legal_claims_review_gated": true,
            "implementation_claims_review_gated": true,
            "autonomy_claims_review_gated": true,
            "spend_claims_review_gated": true
        },
        "mutation_policy": {
            "canonical_queue": "not_mutated_by_phase6g_mission_approval_decision",
            "mission_approval_requests": "append_only_decision_record",
            "output_ledger": ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "append_only": true
        },
        "bounded_output": {
            "emits_operator_approval_decision": true,
            "emits_canonical_task_queue_entry": false,
            "requires_future_queue_write_request": true
        }
    });

    let ledger_path = root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH);
    let appended = append_new_records_by_id_locked(
        &ledger_path,
        vec![decision.clone()],
        "mission approval decision",
        "approval_decision_id",
    )?;
    let queue_after = file_snapshot(root, TASK_QUEUE_PATH)?;

    Ok(json!({
        "contract": "arda.arandur.phase6g_mission_approval_decision_surface.v1",
        "authority": "operator_approved_agent_execution",
        "review_required": true,
        "phase": "6G",
        "status": if appended.is_empty() { "already_recorded_idempotent_noop" } else { "mission_approval_decision_recorded" },
        "approval_request_id": requested_approval_id,
        "approval_status": decision_status,
        "write_performed": !appended.is_empty(),
        "ledger_path": ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
        "queue_integrity": {
            "before": queue_before,
            "after": queue_after,
            "canonical_queue_unchanged": queue_before == queue_after
        },
        "approval_decision": decision,
        "appended_approval_decisions": appended,
        "approval_decision_policy": {
            "append_only_with_write": true,
            "justification_required": true,
            "canonical_queue_mutation_allowed": false,
            "deduplicate_by": "approval_decision_id",
            "claims_review_gated": true
        }
    }))
}

fn propose_arandur_phase6h_mission_queue_entries(
    root: &Path,
    candidate_id: Option<&str>,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let approved_justification = justification
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if write_report {
        anyhow::ensure!(
            approved_justification.is_some(),
            "--justification is required when --write is set"
        );
    }

    let queue_before = file_snapshot(root, TASK_QUEUE_PATH)?;
    let requested_candidate_id = candidate_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let approval_records = read_jsonl_values(&root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?;
    let queue_proposals = build_phase6h_mission_queue_proposals(
        &approval_records,
        requested_candidate_id,
        approved_justification,
        &queue_before,
    );

    let ledger_path = root.join(ARANDUR_MISSION_QUEUE_PROPOSALS_PATH);
    let appended = if write_report {
        append_new_records_by_id_locked(
            &ledger_path,
            queue_proposals.clone(),
            "mission queue proposal",
            "queue_proposal_id",
        )?
    } else {
        Vec::new()
    };
    let queue_after = file_snapshot(root, TASK_QUEUE_PATH)?;
    let canonical_queue_unchanged = queue_before == queue_after;

    let status = if write_report {
        if !appended.is_empty() {
            "mission_queue_proposals_recorded"
        } else if queue_proposals.is_empty() {
            "no_eligible_mission_approval_requests_available"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6h_mission_queue_proposal_surface.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6H",
        "status": status,
        "candidate_id_filter": requested_candidate_id,
        "write_requested": write_report,
        "write_performed": !appended.is_empty(),
        "ledger_path": ARANDUR_MISSION_QUEUE_PROPOSALS_PATH,
        "input_counts": {
            "mission_approval_request_records": approval_records.len(),
            "eligible_queue_proposals": queue_proposals.len()
        },
        "queue_integrity": {
            "before": queue_before,
            "after": queue_after,
            "canonical_queue_unchanged": canonical_queue_unchanged
        },
        "queue_proposals": queue_proposals,
        "appended_queue_proposals": appended,
        "queue_proposal_policy": {
            "dry_run_default": true,
            "append_only_with_write": true,
            "justification_required_for_write": true,
            "canonical_queue_mutation_allowed": false,
            "queue_proposals_are_canonical_queue_entries": false,
            "human_operator_review_required_before_future_queue_mutation": true,
            "claims_review_gated": true
        }
    }))
}

fn build_phase6h_mission_queue_proposals(
    approval_records: &[serde_json::Value],
    candidate_id_filter: Option<&str>,
    justification: Option<&str>,
    queue_snapshot: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let now = Utc::now().to_rfc3339();
    approval_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .filter(|record| {
            candidate_id_filter
                .map(|candidate_id| {
                    json_text_field(record, "source_mission_candidate_id")
                        .map(|record_id| record_id == candidate_id)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .map(|approval| {
            let approval_request_id = json_text_field(approval, "approval_request_id")
                .unwrap_or_else(|| "unknown_approval_request".to_string());
            let source_candidate_id = json_text_field(approval, "source_mission_candidate_id")
                .unwrap_or_else(|| "unknown_mission_candidate".to_string());
            let bounded_recommendation = approval.get("bounded_recommendation");
            let title = bounded_recommendation
                .and_then(|value| json_text_field(value, "title"))
                .or_else(|| json_text_field(approval, "approval_question"))
                .unwrap_or_else(|| "Bounded Arandur mission queue proposal".to_string());
            let scope = bounded_recommendation
                .and_then(|value| json_text_field(value, "scope"))
                .unwrap_or_else(|| "bounded mission queue proposal".to_string());
            let approval_fingerprint = sha1_hex(
                &serde_json::to_string(approval).unwrap_or_else(|_| approval_request_id.clone()),
            );
            json!({
                "contract": "arda.arandur.phase6h_mission_queue_proposal.v1",
                "queue_proposal_id": phase6h_mission_queue_proposal_id(&approval_request_id, &source_candidate_id),
                "authority": "agent_generated",
                "review_required": true,
                "phase": "6H",
                "created_at_utc": now,
                "justification": justification,
                "source_approval_request_id": approval_request_id,
                "source_mission_candidate_id": source_candidate_id,
                "source_fingerprints": {
                    "mission_approval_request_sha1": approval_fingerprint,
                    "canonical_queue_sha1": queue_snapshot.get("sha1").cloned().unwrap_or(serde_json::Value::Null)
                },
                "proposed_queue_entry": {
                    "title": title,
                    "scope": scope,
                    "authority": "agent_generated",
                    "review_required": true,
                    "source": "arandur_phase6h_bounded_queue_proposal",
                    "requires_separate_operator_queue_write": true,
                    "not_canonical_until_explicit_future_approval": true
                },
                "source_approval_snapshot": approval,
                "operator_review_checklist": [
                    "confirm the Phase 6G approval request is explicitly approved by a human/operator",
                    "confirm this Phase 6H proposal still does not mutate the canonical task queue",
                    "confirm all cited claims remain review-gated",
                    "confirm any future canonical queue append uses a separate explicit operator action"
                ],
                "claim_gating": {
                    "research_claims_review_gated": true,
                    "market_claims_review_gated": true,
                    "legal_claims_review_gated": true,
                    "implementation_claims_review_gated": true,
                    "autonomy_claims_review_gated": true,
                    "spend_claims_review_gated": true
                },
                "mutation_policy": {
                    "canonical_queue": "not_mutated_by_phase6h_mission_queue_proposal",
                    "mission_approval_requests": "read_only",
                    "output_ledger": ARANDUR_MISSION_QUEUE_PROPOSALS_PATH,
                    "append_only": true
                },
                "bounded_output": {
                    "emits_candidate_queue_proposal": true,
                    "emits_canonical_task_queue_entry": false,
                    "requires_future_human_approval": true
                }
            })
        })
        .collect()
}

fn phase6h_mission_queue_proposal_id(
    approval_request_id: &str,
    mission_candidate_id: &str,
) -> String {
    let seed = format!("{approval_request_id}\n{mission_candidate_id}");
    format!("phase6h_queue_proposal_{}", &sha1_hex(&seed)[..12])
}

fn request_arandur_phase6i_mission_queue_write(
    root: &Path,
    candidate_id: Option<&str>,
    write_report: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let approved_justification = justification
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if write_report {
        anyhow::ensure!(
            approved_justification.is_some(),
            "--justification is required when --write is set"
        );
    }

    let queue_before = file_snapshot(root, TASK_QUEUE_PATH)?;
    let requested_candidate_id = candidate_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let proposal_records = read_jsonl_values(&root.join(ARANDUR_MISSION_QUEUE_PROPOSALS_PATH))?;
    let queue_write_requests = build_phase6i_mission_queue_write_requests(
        &proposal_records,
        requested_candidate_id,
        approved_justification,
        &queue_before,
    );

    let ledger_path = root.join(ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH);
    let appended = if write_report {
        append_new_records_by_id_locked(
            &ledger_path,
            queue_write_requests.clone(),
            "mission queue write request",
            "queue_write_request_id",
        )?
    } else {
        Vec::new()
    };
    let queue_after = file_snapshot(root, TASK_QUEUE_PATH)?;
    let canonical_queue_unchanged = queue_before == queue_after;

    let status = if write_report {
        if !appended.is_empty() {
            "mission_queue_write_requests_recorded"
        } else if queue_write_requests.is_empty() {
            "no_eligible_mission_queue_proposals_available"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6i_mission_queue_write_request_surface.v1",
        "authority": "agent_generated",
        "review_required": true,
        "phase": "6I",
        "status": status,
        "candidate_id_filter": requested_candidate_id,
        "write_requested": write_report,
        "write_performed": !appended.is_empty(),
        "ledger_path": ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH,
        "input_counts": {
            "mission_queue_proposal_records": proposal_records.len(),
            "eligible_queue_write_requests": queue_write_requests.len()
        },
        "queue_integrity": {
            "before": queue_before,
            "after": queue_after,
            "canonical_queue_unchanged": canonical_queue_unchanged
        },
        "queue_write_requests": queue_write_requests,
        "appended_queue_write_requests": appended,
        "queue_write_request_policy": {
            "dry_run_default": true,
            "append_only_with_write": true,
            "justification_required_for_write": true,
            "canonical_queue_mutation_allowed": false,
            "queue_write_requests_are_canonical_queue_entries": false,
            "requires_separate_future_canonical_queue_write": true,
            "claims_review_gated": true
        }
    }))
}

fn build_phase6i_mission_queue_write_requests(
    proposal_records: &[serde_json::Value],
    candidate_id_filter: Option<&str>,
    justification: Option<&str>,
    queue_snapshot: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let now = Utc::now().to_rfc3339();
    proposal_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .filter(|record| {
            record
                .get("bounded_output")
                .and_then(|value| value.get("emits_canonical_task_queue_entry"))
                .and_then(serde_json::Value::as_bool)
                .map(|emits| !emits)
                .unwrap_or(false)
        })
        .filter(|record| {
            candidate_id_filter
                .map(|candidate_id| {
                    json_text_field(record, "source_mission_candidate_id")
                        .map(|record_id| record_id == candidate_id)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .map(|proposal| {
            let queue_proposal_id = json_text_field(proposal, "queue_proposal_id")
                .unwrap_or_else(|| "unknown_queue_proposal".to_string());
            let source_candidate_id = json_text_field(proposal, "source_mission_candidate_id")
                .unwrap_or_else(|| "unknown_mission_candidate".to_string());
            let source_approval_request_id = json_text_field(proposal, "source_approval_request_id")
                .unwrap_or_else(|| "unknown_approval_request".to_string());
            let proposed_queue_entry = proposal
                .get("proposed_queue_entry")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let proposal_fingerprint = sha1_hex(
                &serde_json::to_string(proposal).unwrap_or_else(|_| queue_proposal_id.clone()),
            );
            json!({
                "contract": "arda.arandur.phase6i_mission_queue_write_request.v1",
                "queue_write_request_id": phase6i_mission_queue_write_request_id(
                    &queue_proposal_id,
                    &source_candidate_id,
                    &source_approval_request_id,
                ),
                "authority": "agent_generated",
                "review_required": true,
                "phase": "6I",
                "write_pending": true,
                "created_at_utc": now,
                "justification": justification,
                "source_queue_proposal_id": queue_proposal_id,
                "source_approval_request_id": source_approval_request_id,
                "source_mission_candidate_id": source_candidate_id,
                "source_fingerprints": {
                    "mission_queue_proposal_sha1": proposal_fingerprint,
                    "canonical_queue_sha1": queue_snapshot.get("sha1").cloned().unwrap_or(serde_json::Value::Null)
                },
                "requested_queue_entry": proposed_queue_entry,
                "source_queue_proposal_snapshot": proposal,
                "operator_write_checklist": [
                    "confirm the Phase 6H queue proposal is explicitly approved by a human/operator",
                    "confirm this Phase 6I request still does not mutate the canonical task queue",
                    "confirm all requested queue-entry fields are bounded and review-gated",
                    "confirm any future canonical queue append uses a separate explicit operator action"
                ],
                "claim_gating": {
                    "research_claims_review_gated": true,
                    "market_claims_review_gated": true,
                    "legal_claims_review_gated": true,
                    "implementation_claims_review_gated": true,
                    "autonomy_claims_review_gated": true,
                    "spend_claims_review_gated": true
                },
                "mutation_policy": {
                    "canonical_queue": "not_mutated_by_phase6i_mission_queue_write_request",
                    "mission_queue_proposals": "read_only",
                    "output_ledger": ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH,
                    "append_only": true
                },
                "bounded_output": {
                    "emits_queue_write_request": true,
                    "emits_canonical_task_queue_entry": false,
                    "requires_future_human_approval": true,
                    "requires_separate_future_canonical_queue_write": true
                }
            })
        })
        .collect()
}

fn phase6i_mission_queue_write_request_id(
    queue_proposal_id: &str,
    mission_candidate_id: &str,
    source_approval_request_id: &str,
) -> String {
    let seed = format!("{queue_proposal_id}\n{mission_candidate_id}\n{source_approval_request_id}");
    format!("phase6i_queue_write_request_{}", &sha1_hex(&seed)[..12])
}

fn execute_arandur_phase6j_queue_write(
    root: &Path,
    candidate_id: Option<&str>,
    write_queue: bool,
    justification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let approved_justification = justification
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if write_queue {
        anyhow::ensure!(
            approved_justification.is_some(),
            "--justification is required when --write is set"
        );
    }

    let requested_candidate_id = candidate_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let queue_before = file_snapshot(root, TASK_QUEUE_PATH)?;
    let write_request_records =
        read_jsonl_values(&root.join(ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH))?;
    let approval_records = read_jsonl_values(&root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?;
    let eligible_write_requests =
        eligible_phase6j_queue_write_requests(&write_request_records, requested_candidate_id);
    validate_phase6j_approved_requests(&eligible_write_requests, &approval_records)?;
    let canonical_entries = build_phase6j_canonical_queue_entries(
        &eligible_write_requests,
        approved_justification,
        &queue_before,
    )?;

    let queue_path = root.join(TASK_QUEUE_PATH);
    let appended = if write_queue {
        append_new_records_by_id_locked(
            &queue_path,
            canonical_entries.clone(),
            "canonical task queue",
            "id",
        )?
    } else {
        Vec::new()
    };
    let queue_after = file_snapshot(root, TASK_QUEUE_PATH)?;
    let canonical_queue_mutated = queue_before != queue_after;

    let status = if write_queue {
        if !appended.is_empty() {
            "canonical_queue_entries_appended"
        } else if canonical_entries.is_empty() {
            "no_eligible_approved_queue_write_requests_available"
        } else {
            "already_recorded_idempotent_noop"
        }
    } else {
        "dry_run_no_mutation"
    };

    Ok(json!({
        "contract": "arda.arandur.phase6j_canonical_queue_write_surface.v1",
        "authority": "operator_approved_agent_execution",
        "review_required": true,
        "phase": "6J",
        "status": status,
        "candidate_id_filter": requested_candidate_id,
        "write_requested": write_queue,
        "write_performed": !appended.is_empty(),
        "canonical_queue_path": TASK_QUEUE_PATH,
        "roadmap_anchor": {
            "parent_plan": "docs/plans/arda-autonomy-readiness-and-human-ingestion-plan.md",
            "parent_gate": "Gate 3.4 Queue Staging and Approval",
            "protocol": "docs/operations/ARANDUR_PROTOCOL.md",
            "purpose": "execute explicitly approved queue write requests without creating ad-hoc phase drift"
        },
        "input_counts": {
            "mission_queue_write_request_records": write_request_records.len(),
            "mission_approval_request_records": approval_records.len(),
            "eligible_approved_queue_write_requests": eligible_write_requests.len(),
            "canonical_entries_prepared": canonical_entries.len()
        },
        "queue_integrity": {
            "before": queue_before,
            "after": queue_after,
            "canonical_queue_mutated": canonical_queue_mutated,
            "before_after_sha_verified": true
        },
        "queue_write_requests": eligible_write_requests,
        "canonical_queue_entries": canonical_entries,
        "appended_canonical_queue_entries": appended,
        "queue_write_policy": {
            "dry_run_default": true,
            "append_only_with_write": true,
            "justification_required_for_write": true,
            "approval_status_required": "approved",
            "write_pending_required": true,
            "deduplicate_by": "id",
            "file_locking_required": true,
            "canonical_queue_mutation_allowed": write_queue,
            "claims_review_gated": true
        }
    }))
}

fn eligible_phase6j_queue_write_requests(
    write_request_records: &[serde_json::Value],
    candidate_id_filter: Option<&str>,
) -> Vec<serde_json::Value> {
    write_request_records
        .iter()
        .filter(|record| agent_generated_review_gated_value(record))
        .filter(|record| {
            record
                .get("write_pending")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .filter(|record| {
            candidate_id_filter
                .map(|candidate_id| {
                    json_text_field(record, "source_mission_candidate_id")
                        .map(|record_id| record_id == candidate_id)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

fn validate_phase6j_approved_requests(
    write_requests: &[serde_json::Value],
    approval_records: &[serde_json::Value],
) -> anyhow::Result<()> {
    for request in write_requests {
        let queue_write_request_id = json_text_field(request, "queue_write_request_id")
            .unwrap_or_else(|| "unknown_queue_write_request".to_string());
        let approval_request_id = phase6j_source_approval_request_id(request).ok_or_else(|| {
            anyhow::anyhow!(
                "queue write request {queue_write_request_id} lacks source_approval_request_id for approved approval record validation"
            )
        })?;
        let approved = approval_records.iter().any(|approval| {
            json_text_field(approval, "approval_request_id")
                .map(|record_id| record_id == approval_request_id)
                .unwrap_or(false)
                && json_text_field(approval, "approval_status")
                    .map(|status| status == "approved")
                    .unwrap_or(false)
                && review_gated_value(approval)
        });
        anyhow::ensure!(
            approved,
            "queue write request {queue_write_request_id} requires matching approved approval record {approval_request_id} before canonical queue mutation"
        );
    }
    Ok(())
}

fn build_phase6j_canonical_queue_entries(
    write_requests: &[serde_json::Value],
    justification: Option<&str>,
    queue_snapshot: &serde_json::Value,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let now = Utc::now().to_rfc3339();
    let mut entries = Vec::new();
    for request in write_requests {
        let queue_write_request_id = json_text_field(request, "queue_write_request_id")
            .unwrap_or_else(|| "unknown_queue_write_request".to_string());
        let source_candidate_id = json_text_field(request, "source_mission_candidate_id")
            .unwrap_or_else(|| "unknown_mission_candidate".to_string());
        let source_approval_request_id = phase6j_source_approval_request_id(request)
            .unwrap_or_else(|| "unknown_approval_request".to_string());
        let requested_entry = request
            .get("requested_queue_entry")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "queue write request {queue_write_request_id} lacks requested_queue_entry object"
                )
            })?;
        let mut entry: serde_json::Map<String, serde_json::Value> = requested_entry.clone();
        if !entry.contains_key("id") {
            entry.insert(
                "id".to_string(),
                json!(phase6j_canonical_queue_entry_id(
                    &queue_write_request_id,
                    &source_candidate_id
                )),
            );
        }
        if !entry.contains_key("status") {
            entry.insert("status".to_string(), json!("pending"));
        }
        entry.insert(
            "authority".to_string(),
            json!("operator_approved_agent_execution"),
        );
        entry.insert("review_required".to_string(), json!(true));
        entry.insert(
            "source".to_string(),
            json!("arandur_phase6j_canonical_queue_write"),
        );
        entry.insert("phase".to_string(), json!("6J"));
        entry.insert("created_at_utc".to_string(), json!(now));
        entry.insert("justification".to_string(), json!(justification));
        entry.insert(
            "source_queue_write_request_id".to_string(),
            json!(queue_write_request_id),
        );
        entry.insert(
            "source_approval_request_id".to_string(),
            json!(source_approval_request_id),
        );
        entry.insert(
            "source_mission_candidate_id".to_string(),
            json!(source_candidate_id),
        );
        entry.insert(
            "source_queue_write_request_sha1".to_string(),
            json!(sha1_hex(&serde_json::to_string(request)?)),
        );
        entry.insert(
            "canonical_queue_before_sha1".to_string(),
            queue_snapshot
                .get("sha1")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );
        entries.push(serde_json::Value::Object(entry));
    }
    Ok(entries)
}

fn phase6j_source_approval_request_id(request: &serde_json::Value) -> Option<String> {
    json_text_field(request, "source_approval_request_id").or_else(|| {
        request
            .get("source_queue_proposal_snapshot")
            .and_then(|snapshot| json_text_field(snapshot, "source_approval_request_id"))
    })
}

fn phase6j_canonical_queue_entry_id(
    queue_write_request_id: &str,
    mission_candidate_id: &str,
) -> String {
    let seed = format!("{queue_write_request_id}\n{mission_candidate_id}");
    format!("task_phase6j_{}", &sha1_hex(&seed)[..12])
}

fn find_mission_candidate_record<'a>(
    candidate_records: &'a [serde_json::Value],
    source_candidate_id: &str,
) -> Option<&'a serde_json::Value> {
    candidate_records.iter().find(|candidate| {
        json_text_field(candidate, "mission_candidate_id")
            .map(|candidate_id| candidate_id == source_candidate_id)
            .unwrap_or(false)
    })
}

fn append_new_mission_approval_request_records_locked(
    ledger_path: &Path,
    pending_records: Vec<serde_json::Value>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    append_new_records_by_id_locked(
        ledger_path,
        pending_records,
        "approval request",
        "approval_request_id",
    )
}

struct ArandurPresenceEventInput<'a> {
    event_id: Option<&'a str>,
    agent: &'a str,
    mode: &'a str,
    attention: &'a str,
    accent: &'a str,
    anchor_target: &'a str,
    mission_id: Option<&'a str>,
    correlation_id: Option<&'a str>,
    timestamp_utc: Option<&'a str>,
}

fn append_arandur_presence_event(
    root: &Path,
    input: ArandurPresenceEventInput<'_>,
) -> anyhow::Result<serde_json::Value> {
    let ArandurPresenceEventInput {
        event_id,
        agent,
        mode,
        attention,
        accent,
        anchor_target,
        mission_id,
        correlation_id,
        timestamp_utc,
    } = input;
    validate_presence_token(
        "agent",
        agent,
        &["arandur", "prometheus", "athena", "manwe", "citadel"],
    )?;
    validate_presence_token(
        "mode",
        mode,
        &[
            "observing",
            "advising",
            "coordinating",
            "escalating",
            "executing",
            "offline",
        ],
    )?;
    validate_presence_token(
        "attention",
        attention,
        &["idle", "focused", "elevated", "critical"],
    )?;
    validate_presence_token(
        "accent",
        accent,
        &["cyan", "gold", "amber", "red", "violet", "green"],
    )?;
    validate_presence_anchor(anchor_target)?;

    let timestamp = match timestamp_utc {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .with_context(|| format!("timestamp_utc must be RFC3339: {value}"))?
            .with_timezone(&Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        None => Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };

    let correlation = correlation_id.unwrap_or("presence-event");
    let id = event_id.map(ToString::to_string).unwrap_or_else(|| {
        deterministic_presence_event_id(agent, mode, attention, &timestamp, correlation)
    });

    let record = json!({
        "id": id,
        "schema": "arda.arda.presence_event.v1",
        "kind": "presence.agent_state",
        "domain": "agent",
        "timestamp_utc": timestamp,
        "entity": {
            "agent": agent,
            "mission_id": mission_id,
        },
        "scene": {
            "presence": {
                "attention": attention,
                "mode": mode,
                "accent": accent,
                "anchor_target": anchor_target,
            }
        },
        "metrics": {},
        "trace": {
            "source": "arda-cli.prometheus.arandur.presence-event",
            "correlation_id": correlation,
        }
    });

    let ledger_path = root.join(ARANDUR_PRESENCE_EVENTS_PATH);
    let appended = append_new_records_by_id_locked(
        &ledger_path,
        vec![record.clone()],
        "arda presence events",
        "id",
    )?;
    Ok(json!({
        "contract": "arda.arandur.presence_event_writer.v1",
        "path": display_path(&ledger_path),
        "appended": appended.len(),
        "duplicates_ignored": if appended.is_empty() { 1 } else { 0 },
        "event": record,
    }))
}

fn deterministic_presence_event_id(
    agent: &str,
    mode: &str,
    attention: &str,
    timestamp: &str,
    correlation_id: &str,
) -> String {
    let mut hasher = Sha1::new();
    hasher.update(agent.as_bytes());
    hasher.update(b"\0");
    hasher.update(mode.as_bytes());
    hasher.update(b"\0");
    hasher.update(attention.as_bytes());
    hasher.update(b"\0");
    hasher.update(timestamp.as_bytes());
    hasher.update(b"\0");
    hasher.update(correlation_id.as_bytes());
    let digest = hasher.finalize();
    format!("presence_{}", hex_prefix(&digest, 24))
}

fn validate_presence_token(name: &str, value: &str, allowed: &[&str]) -> anyhow::Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        anyhow::bail!(
            "invalid {name} '{value}'; allowed values: {}",
            allowed.join(", ")
        )
    }
}

fn validate_presence_anchor(anchor_target: &str) -> anyhow::Result<()> {
    let valid = anchor_target
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
    if valid && !anchor_target.is_empty() && anchor_target.len() <= 96 {
        Ok(())
    } else {
        anyhow::bail!(
            "invalid anchor_target '{anchor_target}'; use a non-empty named scene anchor containing only [A-Za-z0-9_.-] and <= 96 chars"
        )
    }
}

fn append_new_records_by_id_locked(
    ledger_path: &Path,
    pending_records: Vec<serde_json::Value>,
    ledger_label: &str,
    id_field: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    if pending_records.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(ledger_path)
        .with_context(|| {
            format!(
                "failed to open {ledger_label} ledger {}",
                display_path(ledger_path)
            )
        })?;
    file.lock_exclusive().with_context(|| {
        format!(
            "failed to lock {ledger_label} ledger {}",
            display_path(ledger_path)
        )
    })?;

    let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
        let existing_records = read_jsonl_values(ledger_path)?;
        let mut existing_ids: HashSet<String> = existing_records
            .iter()
            .filter_map(|value| value.get(id_field))
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect();
        let mut appended = Vec::new();
        for record in pending_records {
            let Some(record_id) = record
                .get(id_field)
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
            else {
                continue;
            };
            if existing_ids.contains(&record_id) {
                continue;
            }
            writeln!(file, "{}", serde_json::to_string(&record)?)?;
            existing_ids.insert(record_id);
            appended.push(record);
        }
        Ok(appended)
    })();

    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock {ledger_label} ledger {}",
            display_path(ledger_path)
        )
    });
    match (result, unlock_result) {
        (Ok(appended), Ok(())) => Ok(appended),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn append_new_mission_review_records_locked(
    ledger_path: &Path,
    pending_records: Vec<serde_json::Value>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    if pending_records.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(ledger_path)
        .with_context(|| {
            format!(
                "failed to open mission review ledger {}",
                display_path(ledger_path)
            )
        })?;
    file.lock_exclusive().with_context(|| {
        format!(
            "failed to lock mission review ledger {}",
            display_path(ledger_path)
        )
    })?;

    let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
        let existing_records = read_jsonl_values(ledger_path)?;
        let mut existing_ids = jsonl_mission_review_ids(&existing_records);
        let mut appended = Vec::new();
        for record in pending_records {
            let Some(review_id) = record
                .get("mission_review_id")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
            else {
                continue;
            };
            if existing_ids.contains(&review_id) {
                continue;
            }
            writeln!(file, "{}", serde_json::to_string(&record)?)?;
            existing_ids.insert(review_id);
            appended.push(record);
        }
        Ok(appended)
    })();

    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock mission review ledger {}",
            display_path(ledger_path)
        )
    });
    match (result, unlock_result) {
        (Ok(appended), Ok(())) => Ok(appended),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn append_new_mission_candidate_records_locked(
    ledger_path: &Path,
    pending_records: Vec<serde_json::Value>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    if pending_records.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(ledger_path)
        .with_context(|| {
            format!(
                "failed to open mission candidate ledger {}",
                display_path(ledger_path)
            )
        })?;
    file.lock_exclusive().with_context(|| {
        format!(
            "failed to lock mission candidate ledger {}",
            display_path(ledger_path)
        )
    })?;

    let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
        let existing_records = read_jsonl_values(ledger_path)?;
        let mut existing_ids = jsonl_mission_candidate_ids(&existing_records);
        let mut appended = Vec::new();
        for record in pending_records {
            let Some(candidate_id) = record
                .get("mission_candidate_id")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
            else {
                continue;
            };
            if existing_ids.contains(&candidate_id) {
                continue;
            }
            writeln!(file, "{}", serde_json::to_string(&record)?)?;
            existing_ids.insert(candidate_id);
            appended.push(record);
        }
        Ok(appended)
    })();

    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock mission candidate ledger {}",
            display_path(ledger_path)
        )
    });
    match (result, unlock_result) {
        (Ok(appended), Ok(())) => Ok(appended),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn append_jsonl_record_locked(
    ledger_path: &Path,
    record: &serde_json::Value,
) -> anyhow::Result<()> {
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(ledger_path)
        .with_context(|| format!("failed to open JSONL ledger {}", display_path(ledger_path)))?;
    file.lock_exclusive()
        .with_context(|| format!("failed to lock JSONL ledger {}", display_path(ledger_path)))?;
    let result = writeln!(file, "{}", serde_json::to_string(record)?).map_err(anyhow::Error::from);
    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock JSONL ledger {}",
            display_path(ledger_path)
        )
    });
    match (result, unlock_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
    }
}

fn read_text_file_required(path: &Path) -> anyhow::Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("failed to read evidence file {}", display_path(path)))
}

fn normalize_source_urls(source_urls: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = source_urls
        .into_iter()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn phase6c_finding_id(
    mission_id: &str,
    scope: &str,
    source_urls: &[String],
    evidence_sha1: Option<&str>,
) -> String {
    let mut hasher = Sha1::new();
    hasher.update(mission_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(scope.as_bytes());
    hasher.update(b"\n");
    for url in source_urls {
        hasher.update(url.as_bytes());
        hasher.update(b"\n");
    }
    if let Some(sha1) = evidence_sha1 {
        hasher.update(sha1.as_bytes());
    }
    let digest = hasher.finalize();
    format!("phase6c_scout_{}", hex_prefix(&digest[..], 12))
}

fn phase6d_pattern_fingerprint(patterns: &[serde_json::Value]) -> String {
    let serialized = serde_json::to_string(patterns).unwrap_or_else(|_| "[]".to_string());
    sha1_hex(&serialized)[..12].to_string()
}

fn phase6f_mission_review_id(mission_candidate_id: &str) -> String {
    format!("phase6f_review_{}", &sha1_hex(mission_candidate_id)[..12])
}

fn phase6g_mission_approval_request_id(
    mission_review_id: &str,
    mission_candidate_id: &str,
) -> String {
    let seed = format!("{mission_review_id}\n{mission_candidate_id}");
    format!("phase6g_approval_{}", &sha1_hex(&seed)[..12])
}

fn sha1_hex(content: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    hex_prefix(&digest[..], digest.len())
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(chars)
        .map(|nibble| char::from_digit(u32::from(nibble), 16).unwrap_or('0'))
        .collect()
}

fn preview_text(content: &str, max_chars: usize) -> String {
    let mut preview: String = content.chars().take(max_chars).collect();
    if content.chars().count() > max_chars {
        preview.push('…');
    }
    preview
}

fn count_json_string_field(records: &[serde_json::Value], field: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for value in records {
        if let Some(text) = value
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            *counts.entry(text.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn build_phase6d_patterns(
    owner_counts: &BTreeMap<String, usize>,
    recommendation_domains: &BTreeMap<String, usize>,
    scout_scopes: &BTreeMap<String, usize>,
    episode_types: &BTreeMap<String, usize>,
) -> Vec<serde_json::Value> {
    let mut patterns = Vec::new();
    if let Some((owner, count)) = max_count(owner_counts) {
        patterns.push(json!({
            "pattern_id": format!("queue_owner_focus_{}", sanitize_identifier(owner)),
            "kind": "queue_owner_focus",
            "summary": format!("Task queue currently concentrates around owner `{owner}` ({count} records)."),
            "evidence_count": count,
            "authority": "agent_generated",
            "review_required": true
        }));
    }
    if let Some((domain, count)) = max_count(recommendation_domains) {
        patterns.push(json!({
            "pattern_id": format!("recommendation_domain_{}", sanitize_identifier(domain)),
            "kind": "recommendation_domain_focus",
            "summary": format!("Recommendation ledger has repeated `{domain}` domain candidates ({count} records)."),
            "evidence_count": count,
            "authority": "agent_generated",
            "review_required": true
        }));
    }
    if let Some((scope, count)) = max_count(scout_scopes) {
        patterns.push(json!({
            "pattern_id": format!("scout_scope_{}", sanitize_identifier(scope)),
            "kind": "scout_scope_focus",
            "summary": format!("Scout findings repeatedly target `{scope}` ({count} records)."),
            "evidence_count": count,
            "authority": "agent_generated",
            "review_required": true
        }));
    }
    if let Some((episode_type, count)) = max_count(episode_types) {
        patterns.push(json!({
            "pattern_id": format!("episode_type_{}", sanitize_identifier(episode_type)),
            "kind": "episode_type_focus",
            "summary": format!("Episode ledger contains `{episode_type}` learning events ({count} records)."),
            "evidence_count": count,
            "authority": "agent_generated",
            "review_required": true
        }));
    }
    if patterns.is_empty() {
        patterns.push(json!({
            "pattern_id": "insufficient_evidence",
            "kind": "insufficient_evidence",
            "summary": "No repeated Arandur ledger patterns are available yet; gather review-gated recommendations, scout findings, or episodes first.",
            "evidence_count": 0,
            "authority": "agent_generated",
            "review_required": true
        }));
    }
    patterns
}

fn max_count(counts: &BTreeMap<String, usize>) -> Option<(&str, usize)> {
    counts
        .iter()
        .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
        .map(|(key, count)| (key.as_str(), *count))
}

fn json_text_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn sanitize_identifier(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "mission".to_string()
    } else {
        trimmed.to_string()
    }
}

fn run_arandur_cycle(
    root: &Path,
    packet_dir: &Path,
    append_recommendations: bool,
    record_episode: bool,
) -> anyhow::Result<serde_json::Value> {
    let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH)).unwrap_or_default();
    let observation = observe_arandur(root)?;
    let packet_review = review_arandur_packet(root, packet_dir)?;
    let recommendation_batch =
        recommend_arandur_next_from_packet(root, packet_dir, !append_recommendations)?;
    let readiness = assess_arandur_readiness(root)?;
    let queue_after = fs::read_to_string(root.join(TASK_QUEUE_PATH)).unwrap_or_default();
    let canonical_queue_unchanged = queue_before == queue_after;

    let episode = if record_episode {
        Some(append_arandur_episode(
            root,
            "arandur_supervised_cycle",
            "Ran bounded Arandur observe/orient/decide/verify/reflect cycle without canonical queue or raw inbox mutation",
            vec![
                ARANDUR_RUNTIME_PATH.to_string(),
                ARANDUR_EPISODES_PATH.to_string(),
                ARANDUR_RECOMMENDATIONS_PATH.to_string(),
                display_path(packet_dir),
            ],
            Some(
                "Review cycle output before any canonical queue or knowledge promotion".to_string(),
            ),
        )?)
    } else {
        None
    };

    Ok(json!({
        "contract": "arda.arandur.supervised_cycle.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "cycle_status": if canonical_queue_unchanged { "completed_review_gated" } else { "blocked_canonical_queue_changed" },
        "packet_dir": display_path(packet_dir),
        "append_recommendations": append_recommendations,
        "record_episode": record_episode,
        "mutation_policy": {
            "raw_human_inbox": "read_only",
            "canonical_queue": "not_mutated_by_arandur_cycle",
            "recommendation_ledger": if append_recommendations { "append_only_review_required" } else { "dry_run_no_mutation" },
            "episode_ledger": if record_episode { "append_only_review_required" } else { "not_mutated" },
            "services": "no_restart_or_destructive_action"
        },
        "loop": {
            "observe": observation,
            "orient": packet_review,
            "decide": recommendation_batch,
            "delegate": {
                "status": "not_delegated",
                "reason": "Arandur Level 1/2 cycle only emits review-required surfaces"
            },
            "verify": {
                "readiness": readiness,
                "canonical_queue_unchanged": canonical_queue_unchanged,
                "raw_inbox_read_only": true
            },
            "reflect": {
                "episode_recorded": episode.is_some(),
                "episode": episode
            },
            "update_state": {
                "runtime_state_mutated": false,
                "recommendation_ledger_appended": append_recommendations,
                "episode_ledger_appended": record_episode
            }
        },
        "forbidden_actions_confirmed": [
            "no_raw_human_inbox_mutation",
            "no_canonical_queue_append",
            "no_service_restart",
            "no_destructive_operation",
            "research_claims_remain_review_gated"
        ]
    }))
}

fn benchmark_arandur_safety(root: &Path, packet_dir: &Path) -> anyhow::Result<serde_json::Value> {
    let queue_before = read_text_optional(&root.join(TASK_QUEUE_PATH))?;
    let episodes_before = read_text_optional(&root.join(ARANDUR_EPISODES_PATH))?;
    let recommendations_before = read_text_optional(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;

    let cycle = run_arandur_cycle(root, packet_dir, false, false)?;

    let queue_after = read_text_optional(&root.join(TASK_QUEUE_PATH))?;
    let episodes_after = read_text_optional(&root.join(ARANDUR_EPISODES_PATH))?;
    let recommendations_after = read_text_optional(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;

    let canonical_queue_unchanged = queue_before == queue_after;
    let episode_ledger_unchanged = episodes_before == episodes_after;
    let recommendation_ledger_unchanged = recommendations_before == recommendations_after;
    let raw_inbox_read_only = cycle
        .get("loop")
        .and_then(|loop_value| loop_value.get("verify"))
        .and_then(|verify| verify.get("raw_inbox_read_only"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let read_only_safety_passed = canonical_queue_unchanged
        && episode_ledger_unchanged
        && recommendation_ledger_unchanged
        && raw_inbox_read_only;

    Ok(json!({
        "contract": "arda.arandur.safety_benchmark.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "benchmark_status": if read_only_safety_passed {
            "passed_read_only_safety"
        } else {
            "failed_read_only_safety"
        },
        "packet_dir": display_path(packet_dir),
        "mutation_policy": "read_only_benchmark_no_ledgers_or_queue_mutation",
        "mutation_checks": {
            "canonical_queue_unchanged": canonical_queue_unchanged,
            "episode_ledger_unchanged": episode_ledger_unchanged,
            "recommendation_ledger_unchanged": recommendation_ledger_unchanged,
            "raw_inbox_read_only": raw_inbox_read_only,
            "services_restarted": false,
            "destructive_operations": false
        },
        "gates": {
            "read_only_cycle_completed": cycle["cycle_status"] == "completed_review_gated",
            "recommendation_append_disabled": cycle["append_recommendations"] == false,
            "episode_append_disabled": cycle["record_episode"] == false,
            "canonical_queue_mutation_allowed": false,
            "research_claims_review_gated": true
        },
        "loop": {
            "cycle": cycle
        },
        "recommendation": if read_only_safety_passed {
            "benchmark_passed_continue_level_2_append_only_review_gated_operation"
        } else {
            "benchmark_failed_hold_autonomy_and_review_mutation_checks"
        }
    }))
}

fn report_bounded_mutation_classes(root: &Path) -> anyhow::Result<serde_json::Value> {
    let runtime_state = read_json_file_optional(&root.join(ARANDUR_RUNTIME_PATH))?;
    let current_level = runtime_state
        .as_ref()
        .and_then(|value| value.get("autonomy_level"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    Ok(json!({
        "contract": "arda.arandur.bounded_mutation_classes.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "current_level": current_level,
        "evidence_ledger": ARANDUR_MUTATION_EVIDENCE_PATH,
        "mutation_policy": "contract_only_no_mutation",
        "classes": bounded_mutation_classes(),
        "global_forbidden_actions": [
            "raw_human_inbox_mutation",
            "canonical_queue_append_from_packet_promotion",
            "service_restart",
            "destructive_cleanup",
            "ungated_research_market_legal_or_implementation_claim_promotion"
        ],
        "required_verification": {
            "pre_snapshot_required": true,
            "post_snapshot_required": true,
            "path_must_match_class": true,
            "authority_must_be_agent_generated": true,
            "review_required_must_be_true": true,
            "rollback_report_required_for_failed_postcheck": true
        }
    }))
}

fn bounded_mutation_classes() -> Vec<serde_json::Value> {
    vec![
        json!({
            "class_id": "recommendation_ledger_append",
            "level": 2,
            "allowed": true,
            "path": ARANDUR_RECOMMENDATIONS_PATH,
            "operation": "append_jsonl_record_only",
            "record_requirements": {
                "authority": "agent_generated",
                "review_required": true,
                "research_claims": "review_gated"
            },
            "precheck": ["valid_jsonl_before", "target_path_matches_class"],
            "postcheck": ["valid_jsonl_after", "pre_content_is_prefix_of_post_content", "new_records_review_gated"],
            "rollback": "append_compensating_review_required_rollback_report_do_not_rewrite_ledger"
        }),
        json!({
            "class_id": "episode_ledger_append",
            "level": 2,
            "allowed": true,
            "path": ARANDUR_EPISODES_PATH,
            "operation": "append_jsonl_record_only",
            "record_requirements": {
                "authority": "agent_generated",
                "review_required": true,
                "mutation_policy": "recommend_only_or_review_gated"
            },
            "precheck": ["valid_jsonl_before", "target_path_matches_class"],
            "postcheck": ["valid_jsonl_after", "pre_content_is_prefix_of_post_content", "new_records_review_gated"],
            "rollback": "append_compensating_review_required_rollback_report_do_not_rewrite_ledger"
        }),
        json!({
            "class_id": "runtime_level_promotion",
            "level": 2,
            "allowed": "explicit_human_approval_only",
            "path": ARANDUR_RUNTIME_PATH,
            "operation": "single_json_object_update_only_after_readiness_gates",
            "record_requirements": {
                "authority": "agent_generated",
                "review_required": true,
                "approval_note": "required"
            },
            "precheck": ["readiness_level_2_ready", "approval_note_present", "valid_json_before"],
            "postcheck": ["valid_json_after", "autonomy_level_is_2", "forbidden_mutation_flags_false"],
            "rollback": "restore_prior_runtime_state_from_operator_reviewed_snapshot"
        }),
        json!({
            "class_id": "mutation_evidence_append",
            "level": 2,
            "allowed": true,
            "path": ARANDUR_MUTATION_EVIDENCE_PATH,
            "operation": "append_jsonl_evidence_record_only",
            "record_requirements": {
                "authority": "agent_generated",
                "review_required": true,
                "evidence_type": "verification_or_rollback_report"
            },
            "precheck": ["valid_jsonl_before_or_absent", "target_path_matches_class"],
            "postcheck": ["valid_jsonl_after", "evidence_record_review_gated"],
            "rollback": "append_followup_evidence_correction_no_rewrite"
        }),
    ]
}

fn verify_bounded_mutation(
    root: &Path,
    mutation_class: &str,
    target_path: &str,
    pre_sha1: Option<&str>,
    pre_bytes: Option<u64>,
    write_report: bool,
) -> anyhow::Result<serde_json::Value> {
    let class = bounded_mutation_class(mutation_class)
        .ok_or_else(|| anyhow::anyhow!("unknown Arandur mutation class: {mutation_class}"))?;
    let expected_path = class
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let normalized_target = target_path.trim_start_matches("./");
    let target_matches_class = normalized_target == expected_path;
    let post_snapshot = file_snapshot(root, normalized_target)?;
    let post_sha1 = post_snapshot
        .get("sha1")
        .and_then(serde_json::Value::as_str);
    let post_bytes = post_snapshot
        .get("bytes")
        .and_then(serde_json::Value::as_u64);
    let pre_snapshot_provided = pre_sha1.is_some() && pre_bytes.is_some();
    let append_only_size_check = pre_bytes
        .and_then(|expected| post_bytes.map(|actual| actual >= expected))
        .unwrap_or(true);
    let unchanged_or_append_growth = match (pre_sha1, pre_bytes, post_sha1, post_bytes) {
        (Some(before_sha1), Some(before_bytes), Some(after_sha1), Some(after_bytes)) => {
            if after_bytes == before_bytes {
                after_sha1 == before_sha1
            } else {
                after_bytes > before_bytes
            }
        }
        _ => true,
    };
    let json_valid = validate_path_shape(root, normalized_target)?;
    let verification_passed = target_matches_class
        && pre_snapshot_provided
        && append_only_size_check
        && unchanged_or_append_growth
        && json_valid;
    let report = json!({
        "contract": "arda.arandur.bounded_mutation_verification.v1",
        "authority": "agent_generated",
        "review_required": true,
        "evidence_type": "post_mutation_verification",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "mutation_class": mutation_class,
        "class_contract": class,
        "target_path": normalized_target,
        "write_report": write_report,
        "report_ledger": ARANDUR_MUTATION_EVIDENCE_PATH,
        "pre_snapshot_expectation": {
            "sha1": pre_sha1,
            "bytes": pre_bytes
        },
        "post_snapshot": post_snapshot,
        "checks": {
            "target_matches_class": target_matches_class,
            "pre_snapshot_provided": pre_snapshot_provided,
            "append_only_size_check": append_only_size_check,
            "unchanged_or_append_growth": unchanged_or_append_growth,
            "json_or_jsonl_valid": json_valid,
            "authority_agent_generated": true,
            "review_required": true
        },
        "verification_status": if verification_passed { "passed" } else { "failed_requires_rollback_report" },
        "mutation_policy": "verification_report_only_no_target_mutation"
    });
    maybe_append_mutation_evidence(root, &report, write_report)
}

fn report_bounded_rollback_evidence(
    root: &Path,
    mutation_class: &str,
    target_path: &str,
    reason: &str,
    write_report: bool,
) -> anyhow::Result<serde_json::Value> {
    let class = bounded_mutation_class(mutation_class)
        .ok_or_else(|| anyhow::anyhow!("unknown Arandur mutation class: {mutation_class}"))?;
    let normalized_target = target_path.trim_start_matches("./");
    let snapshot = file_snapshot(root, normalized_target)?;
    let report = json!({
        "contract": "arda.arandur.rollback_evidence.v1",
        "authority": "agent_generated",
        "review_required": true,
        "evidence_type": "rollback_report",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "mutation_class": mutation_class,
        "class_contract": class,
        "target_path": normalized_target,
        "reason": reason,
        "write_report": write_report,
        "report_ledger": ARANDUR_MUTATION_EVIDENCE_PATH,
        "current_snapshot": snapshot,
        "rollback_policy": {
            "automatic_rollback_performed": false,
            "target_mutated_by_command": false,
            "operator_review_required": true,
            "recommended_action": "review evidence, restore from reviewed snapshot if needed, then append follow-up verification evidence"
        },
        "forbidden_actions_confirmed": [
            "no_raw_human_inbox_mutation",
            "no_canonical_queue_append",
            "no_service_restart",
            "no_destructive_operation"
        ],
        "rollback_status": "reported_review_required"
    });
    maybe_append_mutation_evidence(root, &report, write_report)
}

fn bounded_mutation_class(mutation_class: &str) -> Option<serde_json::Value> {
    bounded_mutation_classes().into_iter().find(|class| {
        class
            .get("class_id")
            .and_then(serde_json::Value::as_str)
            .map(|class_id| class_id == mutation_class)
            .unwrap_or(false)
    })
}

fn file_snapshot(root: &Path, relative_path: &str) -> anyhow::Result<serde_json::Value> {
    let path = root.join(relative_path);
    if !path.exists() {
        return Ok(json!({
            "path": relative_path,
            "present": false,
            "bytes": 0,
            "sha1": null
        }));
    }
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read file snapshot {}", display_path(&path)))?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let sha1 = format!("{:x}", hasher.finalize());
    Ok(json!({
        "path": relative_path,
        "present": true,
        "bytes": u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        "sha1": sha1
    }))
}

fn validate_path_shape(root: &Path, relative_path: &str) -> anyhow::Result<bool> {
    let path = root.join(relative_path);
    if !path.exists() {
        return Ok(true);
    }
    if relative_path.ends_with(".jsonl") {
        read_jsonl_values(&path)?;
        Ok(true)
    } else if relative_path.ends_with(".json") {
        read_json_file_optional(&path)?;
        Ok(true)
    } else {
        Ok(true)
    }
}

fn maybe_append_mutation_evidence(
    root: &Path,
    report: &serde_json::Value,
    write_report: bool,
) -> anyhow::Result<serde_json::Value> {
    if !write_report {
        return Ok(report.clone());
    }
    let ledger_path = root.join(ARANDUR_MUTATION_EVIDENCE_PATH);
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&ledger_path)
        .with_context(|| {
            format!(
                "failed to open mutation evidence ledger {}",
                display_path(&ledger_path)
            )
        })?;
    file.lock_exclusive().with_context(|| {
        format!(
            "failed to lock mutation evidence ledger {}",
            display_path(&ledger_path)
        )
    })?;
    let write_result = writeln!(file, "{}", serde_json::to_string(report)?).with_context(|| {
        format!(
            "failed to append mutation evidence ledger {}",
            display_path(&ledger_path)
        )
    });
    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock mutation evidence ledger {}",
            display_path(&ledger_path)
        )
    });
    match (write_result, unlock_result) {
        (Ok(()), Ok(())) => {}
        (Err(error), _) => return Err(error),
        (Ok(()), Err(error)) => return Err(error),
    }
    let mut value = report.clone();
    if let Some(object) = value.as_object_mut() {
        object.insert("ledger_appended".to_string(), json!(true));
    }
    Ok(value)
}

fn review_arandur_packet(root: &Path, packet_dir: &Path) -> anyhow::Result<serde_json::Value> {
    let required_files = [
        "README.md",
        "CORE_TASK_SYNTHESIS.md",
        "PROMOTION_MATRIX.md",
        RECOMMENDED_QUEUE_ENTRIES,
    ];
    let mut file_status = BTreeMap::new();
    for file_name in required_files {
        let path = packet_dir.join(file_name);
        file_status.insert(
            file_name.to_string(),
            json!({
                "present": path.exists(),
                "bytes": file_byte_len(&path)?,
            }),
        );
    }

    let readme = read_text_optional(&packet_dir.join("README.md"))?;
    let synthesis = read_text_optional(&packet_dir.join("CORE_TASK_SYNTHESIS.md"))?;
    let matrix = read_text_optional(&packet_dir.join("PROMOTION_MATRIX.md"))?;
    let recommended_path = packet_dir.join(RECOMMENDED_QUEUE_ENTRIES);
    let recommended_summary = summarize_jsonl_file(&recommended_path)?;
    let recommended_records = read_jsonl_values(&recommended_path)?;
    let queue_ids = jsonl_ids(&read_jsonl_values(&root.join(TASK_QUEUE_PATH))?);
    let existing_count = recommended_records
        .iter()
        .filter_map(json_id)
        .filter(|id| queue_ids.contains(*id))
        .count();
    let missing_source_count = recommended_records
        .iter()
        .flat_map(json_sources)
        .filter(|source| !root.join(source).exists())
        .count();

    let evidence_present = file_status.values().all(|status| {
        status
            .get("present")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });
    let candidate_artifacts_marked = contains_review_metadata(&readme)
        && contains_review_metadata(&synthesis)
        && matrix
            .as_deref()
            .map(|content| content.contains("review-gated") || content.contains("review_required"))
            .unwrap_or(false);
    let research_claims_review_gated = matrix
        .as_deref()
        .map(|content| {
            content.to_ascii_lowercase().contains("research")
                && content.to_ascii_lowercase().contains("review")
        })
        .unwrap_or(false);

    Ok(json!({
        "contract": "arda.arandur.packet_review.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "packet_dir": display_path(packet_dir),
        "mutation_policy": "read_only_packet_assessment",
        "files": file_status,
        "recommended_entries": {
            "path": display_path(&recommended_path),
            "total_records": recommended_summary["total_records"],
            "valid_jsonl": recommended_summary["valid_jsonl"],
            "already_in_queue": existing_count,
            "not_in_queue": recommended_records.len().saturating_sub(existing_count),
            "missing_source_paths": missing_source_count
        },
        "promotion_gates": {
            "evidence_present": evidence_present,
            "raw_inbox_read_only": true,
            "source_provenance_preserved": missing_source_count == 0,
            "research_claims_review_gated": research_claims_review_gated,
            "candidate_artifacts_marked_agent_generated": candidate_artifacts_marked,
            "json_and_jsonl_validated": recommended_summary["valid_jsonl"],
            "canonical_queue_unchanged_by_command": true,
            "git_diff_check_required_before_promotion": true
        },
        "assessment": if evidence_present && candidate_artifacts_marked && research_claims_review_gated { "review_ready_candidate_packet" } else { "blocked_pending_packet_cleanup" }
    }))
}

pub(crate) fn recommend_arandur_next(
    root: &Path,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    let packet_dir = root.join(DEFAULT_PHASE2F_PACKET_DIR);
    recommend_arandur_next_from_packet(root, &packet_dir, dry_run)
}

fn recommend_arandur_next_from_packet(
    root: &Path,
    packet_dir: &Path,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    let recommended_path = packet_dir.join(RECOMMENDED_QUEUE_ENTRIES);
    let recommended_records = read_jsonl_values(&recommended_path)?;
    let recommended_record_count = recommended_records.len();
    let queue_ids = jsonl_ids(&read_jsonl_values(&root.join(TASK_QUEUE_PATH))?);
    let ledger_path = root.join(ARANDUR_RECOMMENDATIONS_PATH);

    let ts_utc = Utc::now();
    let mut candidates = Vec::new();
    let mut pending_records = Vec::new();
    for candidate in recommended_records {
        let Some(candidate_id) = json_id(&candidate).map(ToString::to_string) else {
            continue;
        };
        if queue_ids.contains(&candidate_id) {
            continue;
        }
        let record = build_recommendation_record(packet_dir, &ts_utc, &candidate_id, candidate);
        if dry_run {
            candidates.push(record);
        } else {
            pending_records.push(record);
        }
    }

    let appended = if dry_run {
        Vec::new()
    } else {
        append_new_recommendation_records_locked(&ledger_path, pending_records)?
    };

    Ok(json!({
        "contract": "arda.arandur.recommendation_batch.v1",
        "authority": "agent_generated",
        "review_required": true,
        "status": if dry_run { "dry_run_no_mutation" } else { "recommendations_recorded" },
        "generated_at_utc": Utc::now().to_rfc3339(),
        "ledger_path": ARANDUR_RECOMMENDATIONS_PATH,
        "source_packet": display_path(packet_dir),
        "dry_run": dry_run,
        "append_only": true,
        "appended_count": appended.len(),
        "candidate_append_count": candidates.len(),
        "skipped_existing_or_queued_count": recommended_record_count.saturating_sub(appended.len()).saturating_sub(candidates.len()),
        "recommendations": if dry_run { candidates } else { appended }
    }))
}

fn build_recommendation_record(
    packet_dir: &Path,
    ts_utc: &chrono::DateTime<Utc>,
    candidate_id: &str,
    candidate: serde_json::Value,
) -> serde_json::Value {
    json!({
        "contract": "arda.arandur.recommendation.v1",
        "authority": "agent_generated",
        "review_required": true,
        "recommendation_id": format!("arandur_rec_{}_{}", ts_utc.format("%Y%m%dT%H%M%SZ"), candidate_id),
        "ts_utc": ts_utc.to_rfc3339(),
        "source": "athena_phase2f_recommended_queue_entries",
        "source_packet": display_path(packet_dir),
        "recommended_candidate_id": candidate_id,
        "recommended_action": "review_candidate_for_possible_canonical_queue_promotion",
        "mutation_policy": {
            "raw_human_inbox": "read_only",
            "canonical_queue": "not_mutated_by_arandur_recommend_next",
            "research_claims": "review_gated"
        },
        "candidate": candidate
    })
}

fn append_new_recommendation_records_locked(
    ledger_path: &Path,
    pending_records: Vec<serde_json::Value>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    if pending_records.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(parent) = ledger_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(ledger_path)
        .with_context(|| {
            format!(
                "failed to open recommendation ledger {}",
                display_path(ledger_path)
            )
        })?;
    file.lock_exclusive().with_context(|| {
        format!(
            "failed to lock recommendation ledger {}",
            display_path(ledger_path)
        )
    })?;

    let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
        let existing_records = read_jsonl_values(ledger_path)?;
        let mut existing_ids = jsonl_recommended_candidate_ids(&existing_records);
        let mut appended = Vec::new();
        for record in pending_records {
            let Some(candidate_id) = record
                .get("recommended_candidate_id")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
            else {
                continue;
            };
            if existing_ids.contains(&candidate_id) {
                continue;
            }
            writeln!(file, "{}", serde_json::to_string(&record)?)?;
            existing_ids.insert(candidate_id);
            appended.push(record);
        }
        Ok(appended)
    })();

    let unlock_result = file.unlock().with_context(|| {
        format!(
            "failed to unlock recommendation ledger {}",
            display_path(ledger_path)
        )
    });
    match (result, unlock_result) {
        (Ok(appended), Ok(())) => Ok(appended),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

pub(crate) fn summarize_arandur_recommendations(root: &Path) -> anyhow::Result<serde_json::Value> {
    let ledger_path = root.join(ARANDUR_RECOMMENDATIONS_PATH);
    let records = read_jsonl_values(&ledger_path)?;
    let mut by_candidate_id: BTreeMap<String, usize> = BTreeMap::new();
    let mut pending_review = 0usize;
    let all_review_gated = !records.is_empty()
        && records.iter().all(|value| {
            let review_gated = review_gated_value(value);
            let agent_generated = value
                .get("authority")
                .and_then(serde_json::Value::as_str)
                .map(|authority| authority == "agent_generated")
                .unwrap_or(false);
            if review_gated {
                pending_review += 1;
            }
            review_gated && agent_generated
        });

    for candidate_id in records
        .iter()
        .filter_map(|value| value.get("recommended_candidate_id"))
        .filter_map(serde_json::Value::as_str)
    {
        *by_candidate_id.entry(candidate_id.to_string()).or_insert(0) += 1;
    }

    let duplicate_recommended_candidate_ids: Vec<String> = by_candidate_id
        .iter()
        .filter_map(|(candidate_id, count)| {
            if *count > 1 {
                Some(candidate_id.clone())
            } else {
                None
            }
        })
        .collect();
    let duplicate_absent = duplicate_recommended_candidate_ids.is_empty();
    let review_surface_present = ledger_path.exists();

    Ok(json!({
        "contract": "arda.arandur.recommendation_ledger_summary.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "ledger_path": ARANDUR_RECOMMENDATIONS_PATH,
        "present": review_surface_present,
        "valid_jsonl": true,
        "total_records": records.len(),
        "pending_review_count": pending_review,
        "unique_recommended_candidate_count": by_candidate_id.len(),
        "duplicate_recommended_candidate_ids": duplicate_recommended_candidate_ids,
        "promotion_gates": {
            "recommendation_ledger_valid": true,
            "recommendation_review_surface_present": review_surface_present,
            "recommendations_review_required": all_review_gated,
            "duplicate_recommendations_absent": duplicate_absent,
            "append_only_review_required_records": all_review_gated && duplicate_absent
        }
    }))
}

fn assess_arandur_readiness(root: &Path) -> anyhow::Result<serde_json::Value> {
    let runtime_state = read_json_file_optional(&root.join(ARANDUR_RUNTIME_PATH))?;
    let episode_records = read_jsonl_values(&root.join(ARANDUR_EPISODES_PATH))?;
    let recommendation_records = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
    let current_level = runtime_state
        .as_ref()
        .and_then(|value| value.get("autonomy_level"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let runtime_review_gated = runtime_state
        .as_ref()
        .map(review_gated_value)
        .unwrap_or(false);
    let episodes_review_required =
        !episode_records.is_empty() && episode_records.iter().all(review_gated_value);
    let recommendations_review_required = !recommendation_records.is_empty()
        && recommendation_records.iter().all(|value| {
            review_gated_value(value)
                && value
                    .get("authority")
                    .and_then(serde_json::Value::as_str)
                    .map(|authority| authority == "agent_generated")
                    .unwrap_or(false)
        });
    let packet_review = review_arandur_packet(root, &root.join(DEFAULT_PHASE2F_PACKET_DIR)).ok();
    let mission_backlog = report_arandur_mission_backlog(root)?;
    let next_recommended_action = mission_backlog
        .get("next_recommended_action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("generate_or_select_next_review_gated_arandur_gate");
    let recommendation_summary = summarize_arandur_recommendations(root)?;
    let recommendation_ledger_valid = recommendation_summary["promotion_gates"]
        ["recommendation_ledger_valid"]
        .as_bool()
        .unwrap_or(false);
    let duplicate_recommendations_absent = recommendation_summary["promotion_gates"]
        ["duplicate_recommendations_absent"]
        .as_bool()
        .unwrap_or(false);
    let recommendation_review_surface_present = recommendation_summary["promotion_gates"]
        ["recommendation_review_surface_present"]
        .as_bool()
        .unwrap_or(false);
    let packet_gates_clean = packet_review
        .as_ref()
        .and_then(|review| review.get("promotion_gates"))
        .map(|gates| {
            gate_bool(gates, "evidence_present")
                && gate_bool(gates, "raw_inbox_read_only")
                && gate_bool(gates, "research_claims_review_gated")
                && gate_bool(gates, "candidate_artifacts_marked_agent_generated")
                && gate_bool(gates, "json_and_jsonl_validated")
        })
        .unwrap_or(true);
    let clean_dry_run_count = episode_records.len();
    let gate_stack_clean = runtime_review_gated
        && clean_dry_run_count >= 2
        && recommendations_review_required
        && recommendation_ledger_valid
        && duplicate_recommendations_absent
        && recommendation_review_surface_present
        && packet_gates_clean;
    let level_2_active = current_level >= 2;
    let level_2_ready = current_level == 1 && gate_stack_clean;

    Ok(json!({
        "contract": "arda.arandur.readiness.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "current_level": current_level,
        "target_level": 2,
        "level_2_ready": level_2_ready,
        "level_2_active": level_2_active,
        "readiness_gate_stack_clean": gate_stack_clean,
        "mutation_policy": "report_only_no_approval_or_mutation",
        "evidence": {
            "runtime_state": ARANDUR_RUNTIME_PATH,
            "episode_ledger": ARANDUR_EPISODES_PATH,
            "recommendation_ledger": ARANDUR_RECOMMENDATIONS_PATH,
            "packet_review": DEFAULT_PHASE2F_PACKET_DIR,
            "mission_backlog": TASK_QUEUE_PATH
        },
        "promotion_gates": {
            "runtime_review_gated": runtime_review_gated,
            "clean_dry_run_episode_count": clean_dry_run_count,
            "minimum_clean_dry_runs_met": clean_dry_run_count >= 2,
            "episodes_review_required": episodes_review_required,
            "recommendations_review_required": recommendations_review_required,
            "recommendation_ledger_valid": recommendation_ledger_valid,
            "duplicate_recommendations_absent": duplicate_recommendations_absent,
            "recommendation_review_surface_present": recommendation_review_surface_present,
            "packet_promotion_gates_clean": packet_gates_clean,
            "raw_inbox_read_only": true,
            "canonical_queue_mutation_allowed": false,
            "research_claims_review_gated": packet_review
                .as_ref()
                .and_then(|review| review.get("promotion_gates"))
                .map(|gates| gate_bool(gates, "research_claims_review_gated"))
                .unwrap_or(true),
            "git_diff_check_required_before_promotion": true
        },
        "mission_backlog": mission_backlog,
        "next_recommended_action": next_recommended_action,
        "recommendation": if level_2_active {
            "remain_level_2_append_only_review_gated_candidate_surfaces"
        } else if level_2_ready {
            "eligible_for_explicit_human_review_to_enable_level_2_append_only_candidate_surfaces"
        } else {
            "remain_level_1_observe_plan_recommend_until_gates_are_clean"
        }
    }))
}

fn promote_arandur_level(
    root: &Path,
    target_level: u8,
    write: bool,
    approval_note: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    if target_level != 2 {
        anyhow::bail!("Arandur promotion currently supports only target level 2");
    }

    let readiness = assess_arandur_readiness(root)?;
    let level_2_ready = readiness
        .get("level_2_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let current_level = readiness
        .get("current_level")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let already_at_target = current_level >= i64::from(target_level);
    let status = if already_at_target {
        "already_at_or_above_target_level"
    } else if level_2_ready {
        if write {
            "promoted_to_level_2"
        } else {
            "dry_run_ready_for_explicit_write"
        }
    } else {
        "blocked_by_readiness_gates"
    };

    if write && already_at_target {
        return Ok(json!({
            "contract": "arda.arandur.level_promotion.v1",
            "authority": "agent_generated",
            "review_required": true,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "target_level": target_level,
            "write": write,
            "status": status,
            "runtime_mutated": false,
            "runtime_path": ARANDUR_RUNTIME_PATH,
            "readiness": readiness,
            "mutation_policy": "no_op_already_at_or_above_target_level"
        }));
    }

    if write && !level_2_ready {
        return Ok(json!({
            "contract": "arda.arandur.level_promotion.v1",
            "authority": "agent_generated",
            "review_required": true,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "target_level": target_level,
            "write": write,
            "status": status,
            "runtime_mutated": false,
            "readiness": readiness,
            "mutation_policy": "blocked_no_runtime_mutation"
        }));
    }

    if write {
        let note = approval_note
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("--approval-note is required when --write is set"))?;
        let runtime_path = root.join(ARANDUR_RUNTIME_PATH);
        let mut runtime =
            read_json_file_optional(&runtime_path)?.unwrap_or_else(arandur_default_state);
        let runtime_object = runtime.as_object_mut().ok_or_else(|| {
            anyhow::anyhow!("Arandur runtime state must be a JSON object before Level 2 promotion")
        })?;
        runtime_object.insert("autonomy_level".to_string(), json!(2));
        runtime_object.insert("status".to_string(), json!("active_review_gated_level_2"));
        runtime_object.insert(
            "mode".to_string(),
            json!("append_only_review_gated_candidate_surfaces"),
        );
        runtime_object.insert("authority".to_string(), json!("agent_generated"));
        runtime_object.insert("review_required".to_string(), json!(true));
        runtime_object.insert(
            "promoted_at_utc".to_string(),
            json!(Utc::now().to_rfc3339()),
        );
        runtime_object.insert("promotion_approval_note".to_string(), json!(note));
        runtime_object.insert(
            "level_2".to_string(),
            json!({
                "scope": "append_only_review_gated_candidate_surfaces",
                "recommendation_append_allowed": true,
                "episode_append_allowed": true,
                "canonical_queue_mutation_allowed": false,
                "raw_human_inbox_mutation_allowed": false,
                "service_restart_allowed": false,
                "research_claims_review_gated": true
            }),
        );
        runtime_object.insert("last_promotion_readiness".to_string(), readiness.clone());
        if let Some(parent) = runtime_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &runtime_path,
            format!("{}\n", serde_json::to_string_pretty(&runtime)?),
        )?;
    }

    Ok(json!({
        "contract": "arda.arandur.level_promotion.v1",
        "authority": "agent_generated",
        "review_required": true,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "target_level": target_level,
        "write": write,
        "status": status,
        "runtime_mutated": write && level_2_ready,
        "runtime_path": ARANDUR_RUNTIME_PATH,
        "approval_note_present": approval_note
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
        "readiness": readiness,
        "mutation_policy": if write {
            "runtime_state_only_no_queue_or_raw_inbox_mutation"
        } else {
            "dry_run_no_mutation"
        }
    }))
}

fn summarize_packet_state(packet_dir: &Path) -> anyhow::Result<serde_json::Value> {
    Ok(json!({
        "packet_dir": display_path(packet_dir),
        "present": packet_dir.exists(),
        "readme_present": packet_dir.join("README.md").exists(),
        "core_task_synthesis_present": packet_dir.join("CORE_TASK_SYNTHESIS.md").exists(),
        "promotion_matrix_present": packet_dir.join("PROMOTION_MATRIX.md").exists(),
        "recommended_queue_entries": summarize_jsonl_file(&packet_dir.join(RECOMMENDED_QUEUE_ENTRIES))?
    }))
}

fn summarize_jsonl_file(path: &Path) -> anyhow::Result<serde_json::Value> {
    let records = read_jsonl_values(path)?;
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut latest_status_by_identity: BTreeMap<String, String> = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        let status = record
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *by_status.entry(status.clone()).or_insert(0) += 1;

        let identity = json_id(record)
            .map(|id| format!("id:{id}"))
            .unwrap_or_else(|| format!("record:{index}"));
        latest_status_by_identity.insert(identity, status);
    }

    let mut effective_by_status: BTreeMap<String, usize> = BTreeMap::new();
    for status in latest_status_by_identity.values() {
        *effective_by_status.entry(status.clone()).or_insert(0) += 1;
    }
    let effective_total_records = latest_status_by_identity.len();
    let superseded_record_count = records.len().saturating_sub(effective_total_records);

    Ok(json!({
        "path": display_path(path),
        "present": path.exists(),
        "valid_jsonl": true,
        "total_records": records.len(),
        "status_counts": by_status,
        "effective_total_records": effective_total_records,
        "effective_status_counts": effective_by_status,
        "superseded_record_count": superseded_record_count
    }))
}

fn read_json_file_optional(path: &Path) -> anyhow::Result<Option<serde_json::Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file {}", display_path(path)))?;
    let value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON file {}", display_path(path)))?;
    Ok(Some(value))
}

fn read_text_optional(path: &Path) -> anyhow::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read text file {}", display_path(path)))?;
    Ok(Some(content))
}

fn read_jsonl_values(path: &Path) -> anyhow::Result<Vec<serde_json::Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSONL file {}", display_path(path)))?;
    parse_jsonl_values(path, &content)
}

fn parse_jsonl_values(path: &Path, content: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut values = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str::<serde_json::Value>(line).with_context(|| {
            format!(
                "failed to parse JSONL record {} in {}",
                index + 1,
                display_path(path)
            )
        })?;
        values.push(value);
    }
    Ok(values)
}

fn jsonl_ids(records: &[serde_json::Value]) -> HashSet<String> {
    records
        .iter()
        .filter_map(json_id)
        .map(ToString::to_string)
        .collect()
}

fn jsonl_recommended_candidate_ids(records: &[serde_json::Value]) -> HashSet<String> {
    records
        .iter()
        .filter_map(|value| value.get("recommended_candidate_id"))
        .filter_map(serde_json::Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn jsonl_mission_candidate_ids(records: &[serde_json::Value]) -> HashSet<String> {
    records
        .iter()
        .filter_map(|value| value.get("mission_candidate_id"))
        .filter_map(serde_json::Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn jsonl_mission_review_ids(records: &[serde_json::Value]) -> HashSet<String> {
    records
        .iter()
        .filter_map(|value| value.get("mission_review_id"))
        .filter_map(serde_json::Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn agent_generated_review_gated_value(value: &serde_json::Value) -> bool {
    review_gated_value(value)
        && value
            .get("authority")
            .and_then(serde_json::Value::as_str)
            .map(|authority| authority == "agent_generated")
            .unwrap_or(false)
}

fn phase6e_recommendation_ids(records: &[&serde_json::Value]) -> Vec<String> {
    let mut ids: Vec<String> = records
        .iter()
        .filter_map(|record| {
            json_text_field(record, "recommendation_id")
                .or_else(|| json_text_field(record, "recommended_candidate_id"))
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

fn phase6e_scout_ids(records: &[&serde_json::Value]) -> Vec<String> {
    let mut ids: Vec<String> = records
        .iter()
        .filter_map(|record| json_text_field(record, "finding_id"))
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

fn phase6e_mission_candidate_id(
    synthesis_id: &str,
    pattern_id: &str,
    recommendation_ids: &[String],
    scout_ids: &[String],
) -> String {
    let seed = json!({
        "synthesis_id": synthesis_id,
        "pattern_id": pattern_id,
        "recommendation_ids": recommendation_ids,
        "scout_ids": scout_ids
    });
    let serialized = serde_json::to_string(&seed).unwrap_or_else(|_| pattern_id.to_string());
    format!("phase6e_mission_{}", &sha1_hex(&serialized)[..12])
}

fn phase6e_candidate_title(pattern: &serde_json::Value) -> String {
    let kind = json_text_field(pattern, "kind").unwrap_or_else(|| "approved_pattern".to_string());
    format!(
        "Review approved Arandur pattern: {}",
        kind.replace('_', " ")
    )
}

fn phase6e_candidate_scope(
    pattern: &serde_json::Value,
    scout_records: &[&serde_json::Value],
) -> String {
    json_text_field(pattern, "scope").unwrap_or_else(|| {
        scout_records
            .iter()
            .filter_map(|record| json_text_field(record, "scope"))
            .next()
            .unwrap_or_else(|| "review-gated mission promotion".to_string())
    })
}

fn phase6e_evidence(scout_records: &[&serde_json::Value]) -> Vec<serde_json::Value> {
    scout_records
        .iter()
        .map(|record| {
            json!({
                "finding_id": json_text_field(record, "finding_id"),
                "mission_id": json_text_field(record, "mission_id"),
                "scope": json_text_field(record, "scope"),
                "source_urls": record
                    .get("source_urls")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "evidence_file": json_text_field(record, "evidence_file"),
                "evidence_sha1": json_text_field(record, "evidence_sha1")
            })
        })
        .collect()
}

fn json_id(value: &serde_json::Value) -> Option<&str> {
    value.get("id").and_then(serde_json::Value::as_str)
}

fn json_sources(value: &serde_json::Value) -> Vec<String> {
    value
        .get("sources")
        .and_then(serde_json::Value::as_array)
        .map(|sources| {
            sources
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn contains_review_metadata(content: &Option<String>) -> bool {
    content
        .as_deref()
        .map(|text| {
            text.contains("authority: agent_generated") && text.contains("review_required: true")
        })
        .unwrap_or(false)
}

fn review_gated_value(value: &serde_json::Value) -> bool {
    value
        .get("review_required")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn gate_bool(gates: &serde_json::Value, key: &str) -> bool {
    gates
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn file_byte_len(path: &Path) -> anyhow::Result<Option<u64>> {
    if !path.exists() {
        return Ok(None);
    }
    let len = fs::metadata(path)?.len();
    Ok(Some(len))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> anyhow::Result<PathBuf> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("arda_arandur_{name}_{nanos}"));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn write(root: &Path, relative: &str, content: &str) -> anyhow::Result<()> {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    #[test]
    fn observe_reports_state_without_mutating_queue() -> anyhow::Result<()> {
        let root = temp_root("observe")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"task-one\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(
            &root,
            "data/arandur/episodes.jsonl",
            "{\"episode_id\":\"ep1\",\"review_required\":true}\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate-one\",\"status\":\"candidate\"}\n",
        )?;
        let queue_before = fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?;

        let observed = observe_arandur(&root)?;

        assert_eq!(observed["contract"], "arda.arandur.observation.v1");
        assert_eq!(observed["mutation_policy"], "read_only_observation");
        assert_eq!(observed["queue"]["total_records"], 1);
        assert_eq!(observed["arandur"]["episode_count"], 1);
        assert_eq!(
            fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?,
            queue_before
        );
        Ok(())
    }

    #[test]
    fn queue_summary_reports_latest_effective_status_by_id() -> anyhow::Result<()> {
        let root = temp_root("queue_effective_status")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"task-a\",\"status\":\"pending\"}\n{\"id\":\"task-b\",\"status\":\"queued\"}\n{\"id\":\"task-a\",\"status\":\"completed\"}\n{\"id\":\"task-c\",\"status\":\"pending\"}\n",
        )?;

        let summary = summarize_jsonl_file(&root.join("core/projects/tasks/queue.jsonl"))?;

        assert_eq!(summary["total_records"], 4);
        assert_eq!(summary["status_counts"]["pending"], 2);
        assert_eq!(summary["effective_total_records"], 3);
        assert_eq!(summary["effective_status_counts"]["completed"], 1);
        assert_eq!(summary["effective_status_counts"]["queued"], 1);
        assert_eq!(summary["effective_status_counts"]["pending"], 1);
        assert_eq!(summary["superseded_record_count"], 1);
        Ok(())
    }

    #[test]
    fn mission_backlog_reports_no_open_tasks_when_raw_pending_records_are_superseded(
    ) -> anyhow::Result<()> {
        let root = temp_root("mission_backlog_no_open")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"task-phase6j\",\"status\":\"pending\",\"title\":\"Review mission candidate: stale\",\"phase\":\"6J\",\"source\":\"arandur_phase6j_canonical_queue_write\"}\n{\"id\":\"task-phase6j\",\"status\":\"completed\",\"title\":\"Review mission candidate: stale\",\"phase\":\"6J\",\"source\":\"arandur_canonical_queue_task_execution\"}\n",
        )?;

        let report = report_arandur_mission_backlog(&root)?;

        assert_eq!(report["contract"], "arda.arandur.mission_backlog.v1");
        assert_eq!(report["status"], "no_effective_open_tasks");
        assert_eq!(report["effective_open_task_count"], 0);
        assert_eq!(report["effective_mission_review_task_count"], 0);
        assert_eq!(
            report["next_recommended_action"],
            "generate_or_select_next_review_gated_arandur_gate"
        );
        assert_eq!(report["queue"]["status_counts"]["pending"], 1);
        assert_eq!(report["queue"]["effective_status_counts"]["completed"], 1);
        Ok(())
    }

    #[test]
    fn mission_backlog_selects_next_effective_mission_review_task() -> anyhow::Result<()> {
        let root = temp_root("mission_backlog_next_review")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"task-open\",\"status\":\"pending\",\"title\":\"Review mission candidate: live\",\"phase\":\"6J\",\"source\":\"arandur_phase6j_canonical_queue_write\"}\n",
        )?;

        let report = report_arandur_mission_backlog(&root)?;

        assert_eq!(report["status"], "effective_open_tasks_present");
        assert_eq!(report["effective_open_task_count"], 1);
        assert_eq!(report["effective_mission_review_task_count"], 1);
        assert_eq!(report["next_mission_review_task"]["id"], "task-open");
        assert_eq!(
            report["next_recommended_action"],
            "execute_next_effective_mission_review_task"
        );
        Ok(())
    }

    #[test]
    fn review_packet_detects_existing_queue_ids_and_review_gates() -> anyhow::Result<()> {
        let root = temp_root("review")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"candidate_a\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            "packet/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(&root, "packet/recommended_queue_entries.jsonl", "{\"id\":\"candidate_a\",\"status\":\"candidate\"}\n{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n")?;

        let review = review_arandur_packet(&root, &root.join("packet"))?;

        assert_eq!(review["contract"], "arda.arandur.packet_review.v1");
        assert_eq!(review["recommended_entries"]["total_records"], 2);
        assert_eq!(review["recommended_entries"]["already_in_queue"], 1);
        assert_eq!(review["promotion_gates"]["raw_inbox_read_only"], true);
        assert_eq!(
            review["promotion_gates"]["research_claims_review_gated"],
            true
        );
        Ok(())
    }

    #[test]
    fn recommend_next_appends_review_required_records() -> anyhow::Result<()> {
        let root = temp_root("recommend")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"candidate_a\",\"status\":\"pending\"}\n",
        )?;
        write(&root, "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl", "{\"id\":\"candidate_a\",\"title\":\"Existing\",\"status\":\"candidate\"}\n{\"id\":\"candidate_b\",\"title\":\"New\",\"status\":\"candidate\"}\n")?;

        let result = recommend_arandur_next(&root, false)?;

        assert_eq!(result["status"], "recommendations_recorded");
        assert_eq!(result["appended_count"], 1);
        let ledger = fs::read_to_string(root.join("data/arandur/recommendations.jsonl"))?;
        assert!(ledger.contains("agent_generated"));
        assert!(ledger.contains("review_required"));
        assert!(ledger.contains("candidate_b"));
        assert!(!ledger.contains("candidate_a\",\"title\":\"Existing"));
        Ok(())
    }

    #[test]
    fn recommend_next_dry_run_does_not_append_and_reports_candidates() -> anyhow::Result<()> {
        let root = temp_root("recommend_dry_run")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"candidate_a\",\"status\":\"pending\"}\n",
        )?;
        write(&root, "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl", "{\"id\":\"candidate_a\",\"title\":\"Existing\",\"status\":\"candidate\"}\n{\"id\":\"candidate_b\",\"title\":\"New\",\"status\":\"candidate\"}\n")?;

        let result = recommend_arandur_next(&root, true)?;

        assert_eq!(result["status"], "dry_run_no_mutation");
        assert_eq!(result["dry_run"], true);
        assert_eq!(result["append_only"], true);
        assert_eq!(result["appended_count"], 0);
        assert_eq!(result["candidate_append_count"], 1);
        assert!(!root.join("data/arandur/recommendations.jsonl").exists());
        Ok(())
    }

    #[test]
    fn recommend_next_is_idempotent_for_existing_candidate_recommendations() -> anyhow::Result<()> {
        let root = temp_root("recommend_idempotent")?;
        write(&root, "core/projects/tasks/queue.jsonl", "")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"title\":\"New\",\"status\":\"candidate\"}\n",
        )?;

        let first = recommend_arandur_next(&root, false)?;
        let second = recommend_arandur_next(&root, false)?;
        let ledger = read_jsonl_values(&root.join("data/arandur/recommendations.jsonl"))?;

        assert_eq!(first["appended_count"], 1);
        assert_eq!(second["appended_count"], 0);
        assert_eq!(second["skipped_existing_or_queued_count"], 1);
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger[0]["recommended_candidate_id"], "candidate_b");
        Ok(())
    }

    #[test]
    fn recommend_next_concurrent_writers_keep_one_candidate_record() -> anyhow::Result<()> {
        let root = temp_root("recommend_concurrent")?;
        write(&root, "core/projects/tasks/queue.jsonl", "")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"title\":\"New\",\"status\":\"candidate\"}\n",
        )?;
        let shared_root = std::sync::Arc::new(root);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
        let mut handles = Vec::new();

        for _ in 0..16 {
            let thread_root = std::sync::Arc::clone(&shared_root);
            let thread_barrier = std::sync::Arc::clone(&barrier);
            handles.push(std::thread::spawn(
                move || -> anyhow::Result<serde_json::Value> {
                    thread_barrier.wait();
                    recommend_arandur_next(&thread_root, false)
                },
            ));
        }

        let mut appended_total = 0u64;
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| anyhow::anyhow!("recommendation writer thread panicked"))??;
            appended_total += result["appended_count"].as_u64().unwrap_or(0);
        }
        let ledger = read_jsonl_values(&shared_root.join("data/arandur/recommendations.jsonl"))?;

        assert_eq!(appended_total, 1);
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger[0]["recommended_candidate_id"], "candidate_b");
        Ok(())
    }

    #[test]
    fn record_episode_concurrent_writers_keep_unique_review_gated_records() -> anyhow::Result<()> {
        let root = temp_root("episode_concurrent")?;
        let shared_root = std::sync::Arc::new(root);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
        let mut handles = Vec::new();

        for index in 0..16 {
            let thread_root = std::sync::Arc::clone(&shared_root);
            let thread_barrier = std::sync::Arc::clone(&barrier);
            handles.push(std::thread::spawn(
                move || -> anyhow::Result<serde_json::Value> {
                    thread_barrier.wait();
                    append_arandur_episode(
                        &thread_root,
                        "arandur_supervised_cycle",
                        &format!("concurrent episode {index}"),
                        vec!["evidence/path.md".to_string()],
                        Some("review before promotion".to_string()),
                    )
                },
            ));
        }

        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("episode writer thread panicked"))??;
        }
        let ledger = read_jsonl_values(&shared_root.join("data/arandur/episodes.jsonl"))?;
        let episode_ids: HashSet<String> = ledger
            .iter()
            .filter_map(|record| record.get("episode_id"))
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect();
        let episode_sequences: HashSet<u64> = ledger
            .iter()
            .filter_map(|record| record.get("episode_sequence"))
            .filter_map(serde_json::Value::as_u64)
            .collect();

        assert_eq!(ledger.len(), 16);
        assert_eq!(episode_ids.len(), 16);
        assert_eq!(episode_sequences.len(), 16);
        for expected in 1..=16 {
            assert!(
                episode_sequences.contains(&expected),
                "missing episode sequence {expected} in {episode_sequences:?}"
            );
        }
        assert!(ledger.iter().all(review_gated_value));
        Ok(())
    }

    #[test]
    fn cycle_dry_run_does_not_mutate_ledgers_or_queue() -> anyhow::Result<()> {
        let root = temp_root("cycle_dry_run")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"existing\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(
            &root,
            "packet/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_cycle\",\"title\":\"Cycle\",\"status\":\"candidate\"}\n",
        )?;
        let queue_before = fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?;

        let cycle = run_arandur_cycle(&root, &root.join("packet"), false, false)?;

        assert_eq!(cycle["contract"], "arda.arandur.supervised_cycle.v1");
        assert_eq!(cycle["cycle_status"], "completed_review_gated");
        assert_eq!(cycle["loop"]["decide"]["dry_run"], true);
        assert_eq!(cycle["loop"]["reflect"]["episode_recorded"], false);
        assert!(!root.join("data/arandur/recommendations.jsonl").exists());
        assert!(!root.join("data/arandur/episodes.jsonl").exists());
        assert_eq!(
            fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?,
            queue_before
        );
        Ok(())
    }

    #[test]
    fn presence_event_writer_is_bounded_append_only_and_idempotent() -> anyhow::Result<()> {
        let root = temp_root("presence_event_writer")?;
        let first = append_arandur_presence_event(
            &root,
            ArandurPresenceEventInput {
                event_id: Some("presence_test_event"),
                agent: "arandur",
                mode: "advising",
                attention: "elevated",
                accent: "cyan",
                anchor_target: "boardroom.hologram_anchor",
                mission_id: Some("mission_gate_3_5n"),
                correlation_id: Some("gate-3.5n-test"),
                timestamp_utc: Some("2026-05-18T00:00:00Z"),
            },
        )?;
        let second = append_arandur_presence_event(
            &root,
            ArandurPresenceEventInput {
                event_id: Some("presence_test_event"),
                agent: "arandur",
                mode: "advising",
                attention: "elevated",
                accent: "cyan",
                anchor_target: "boardroom.hologram_anchor",
                mission_id: Some("mission_gate_3_5n"),
                correlation_id: Some("gate-3.5n-test"),
                timestamp_utc: Some("2026-05-18T00:00:00Z"),
            },
        )?;

        assert_eq!(first["appended"], 1);
        assert_eq!(second["appended"], 0);
        assert_eq!(second["duplicates_ignored"], 1);
        let records = read_jsonl_values(&root.join(ARANDUR_PRESENCE_EVENTS_PATH))?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["schema"], "arda.arda.presence_event.v1");
        assert_eq!(records[0]["scene"]["presence"]["mode"], "advising");
        assert_eq!(
            records[0]["scene"]["presence"]["anchor_target"],
            "boardroom.hologram_anchor"
        );
        assert!(append_arandur_presence_event(
            &root,
            ArandurPresenceEventInput {
                event_id: Some("presence_invalid"),
                agent: "arandur",
                mode: "teleporting",
                attention: "elevated",
                accent: "cyan",
                anchor_target: "boardroom.hologram_anchor",
                mission_id: None,
                correlation_id: None,
                timestamp_utc: Some("2026-05-18T00:00:00Z"),
            },
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn cycle_can_append_review_gated_surfaces_without_queue_mutation() -> anyhow::Result<()> {
        let root = temp_root("cycle_append")?;
        write(&root, "core/projects/tasks/queue.jsonl", "")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(
            &root,
            "packet/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_cycle\",\"title\":\"Cycle\",\"status\":\"candidate\"}\n",
        )?;

        let cycle = run_arandur_cycle(&root, &root.join("packet"), true, true)?;

        assert_eq!(cycle["append_recommendations"], true);
        assert_eq!(cycle["record_episode"], true);
        assert_eq!(cycle["loop"]["verify"]["canonical_queue_unchanged"], true);
        assert_eq!(
            read_jsonl_values(&root.join("data/arandur/recommendations.jsonl"))?.len(),
            1
        );
        assert_eq!(
            read_jsonl_values(&root.join("data/arandur/episodes.jsonl"))?.len(),
            1
        );
        assert_eq!(
            fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?,
            ""
        );
        Ok(())
    }

    #[test]
    fn benchmark_reports_read_only_safety_without_mutating_ledgers_or_queue() -> anyhow::Result<()>
    {
        let root = temp_root("benchmark_read_only")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"existing\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":2,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(
            &root,
            "data/arandur/episodes.jsonl",
            "{\"episode_id\":\"ep1\",\"review_required\":true}\n",
        )?;
        write(
            &root,
            "data/arandur/recommendations.jsonl",
            "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_a\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;
        write(
            &root,
            "packet/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "packet/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"title\":\"Candidate\",\"status\":\"candidate\"}\n",
        )?;
        let queue_before = fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?;
        let episodes_before = fs::read_to_string(root.join("data/arandur/episodes.jsonl"))?;
        let recommendations_before =
            fs::read_to_string(root.join("data/arandur/recommendations.jsonl"))?;

        let benchmark = benchmark_arandur_safety(&root, &root.join("packet"))?;

        assert_eq!(
            benchmark["contract"],
            "arda.arandur.safety_benchmark.v1"
        );
        assert_eq!(benchmark["benchmark_status"], "passed_read_only_safety");
        assert_eq!(
            benchmark["mutation_checks"]["canonical_queue_unchanged"],
            true
        );
        assert_eq!(
            benchmark["mutation_checks"]["episode_ledger_unchanged"],
            true
        );
        assert_eq!(
            benchmark["mutation_checks"]["recommendation_ledger_unchanged"],
            true
        );
        assert_eq!(benchmark["loop"]["cycle"]["append_recommendations"], false);
        assert_eq!(benchmark["loop"]["cycle"]["record_episode"], false);
        assert_eq!(
            fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join("data/arandur/episodes.jsonl"))?,
            episodes_before
        );
        assert_eq!(
            fs::read_to_string(root.join("data/arandur/recommendations.jsonl"))?,
            recommendations_before
        );
        Ok(())
    }

    #[test]
    fn recommendation_summary_detects_duplicate_ids_and_review_gates() -> anyhow::Result<()> {
        let root = temp_root("recommendations_summary")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_a\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"recommendation_id\":\"rec2\",\"recommended_candidate_id\":\"candidate_a\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;

        let summary = summarize_arandur_recommendations(&root)?;

        assert_eq!(
            summary["contract"],
            "arda.arandur.recommendation_ledger_summary.v1"
        );
        assert_eq!(summary["total_records"], 2);
        assert_eq!(
            summary["duplicate_recommended_candidate_ids"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            summary["promotion_gates"]["duplicate_recommendations_absent"],
            false
        );
        assert_eq!(
            summary["promotion_gates"]["recommendation_review_surface_present"],
            true
        );
        Ok(())
    }

    #[test]
    fn readiness_reports_recommendation_ledger_quality_gates() -> anyhow::Result<()> {
        let root = temp_root("readiness_duplicate_gate")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;

        let readiness = assess_arandur_readiness(&root)?;

        assert_eq!(
            readiness["promotion_gates"]["recommendation_ledger_valid"],
            true
        );
        assert_eq!(
            readiness["promotion_gates"]["duplicate_recommendations_absent"],
            true
        );
        assert_eq!(
            readiness["promotion_gates"]["recommendation_review_surface_present"],
            true
        );
        Ok(())
    }

    #[test]
    fn readiness_requires_clean_level_two_gates() -> anyhow::Result<()> {
        let root = temp_root("readiness")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;

        let readiness = assess_arandur_readiness(&root)?;

        assert_eq!(readiness["contract"], "arda.arandur.readiness.v1");
        assert_eq!(readiness["current_level"], 1);
        assert_eq!(readiness["level_2_ready"], true);
        assert_eq!(
            readiness["promotion_gates"]["recommendations_review_required"],
            true
        );
        Ok(())
    }

    #[test]
    fn readiness_reports_active_level_two_without_regression_recommendation() -> anyhow::Result<()>
    {
        let root = temp_root("readiness_level_two_active")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":2,\"authority\":\"agent_generated\",\"review_required\":true,\"mode\":\"append_only_review_gated_candidate_surfaces\"}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;

        let readiness = assess_arandur_readiness(&root)?;

        assert_eq!(readiness["current_level"], 2);
        assert_eq!(readiness["level_2_active"], true);
        assert_eq!(readiness["readiness_gate_stack_clean"], true);
        assert_eq!(readiness["level_2_ready"], false);
        assert_eq!(
            readiness["recommendation"],
            "remain_level_2_append_only_review_gated_candidate_surfaces"
        );
        Ok(())
    }

    #[test]
    fn readiness_surfaces_effective_mission_backlog_for_next_gate_selection() -> anyhow::Result<()>
    {
        let root = temp_root("readiness_mission_backlog")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":2,\"authority\":\"agent_generated\",\"review_required\":true,\"mode\":\"append_only_review_gated_candidate_surfaces\"}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"task-phase6j\",\"status\":\"pending\",\"title\":\"Review mission candidate: stale\",\"phase\":\"6J\",\"source\":\"arandur_phase6j_canonical_queue_write\"}\n{\"id\":\"task-phase6j\",\"status\":\"completed\",\"title\":\"Review mission candidate: stale\",\"phase\":\"6J\",\"source\":\"arandur_canonical_queue_task_execution\"}\n",
        )?;

        let readiness = assess_arandur_readiness(&root)?;

        assert_eq!(
            readiness["mission_backlog"]["contract"],
            "arda.arandur.mission_backlog.v1"
        );
        assert_eq!(readiness["mission_backlog"]["effective_open_task_count"], 0);
        assert_eq!(
            readiness["next_recommended_action"],
            "generate_or_select_next_review_gated_arandur_gate"
        );
        assert_eq!(
            readiness["evidence"]["mission_backlog"],
            "core/projects/tasks/queue.jsonl"
        );
        Ok(())
    }

    #[test]
    fn system_map_reports_governance_crate_layers_and_athena_surfaces() -> anyhow::Result<()> {
        let root = temp_root("system_map_governance")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(
            &root,
            "core/projects/tasks/queue.jsonl",
            "{\"id\":\"task-one\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            "data/arandur/episodes.jsonl",
            "{\"episode_id\":\"ep1\",\"review_required\":true}\n",
        )?;
        write(
            &root,
            "data/arandur/recommendations.jsonl",
            "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;
        write(
            &root,
            "crates/arda-governance/src/triad.rs",
            "pub struct Triad;\n",
        )?;
        write(
            &root,
            "crates/arda-governance/src/resonance.rs",
            "pub fn calculate_resonance_basic() {}\n",
        )?;
        write(
            &root,
            "crates/arda-plutus/src/lib.rs",
            "pub struct JouleWorkUnit;\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/ATHENA_PACKET_PROMOTION_SURFACE.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;

        let map = map_arandur_system(&root)?;

        assert_eq!(map["contract"], "arda.arandur.system_map.v1");
        assert_eq!(map["mutation_policy"], "read_only_system_mapping");
        assert_eq!(map["athena"]["promotion_surface"]["present"], true);
        assert_eq!(map["governance"]["crate_layers"]["triad"]["present"], true);
        assert_eq!(
            map["governance"]["crate_layers"]["resonance"]["present"],
            true
        );
        assert_eq!(
            map["governance"]["crate_layers"]["joulework"]["present"],
            true
        );
        assert_eq!(
            map["governance"]["layering_gates"]["canonical_queue_mutation_allowed"],
            false
        );
        Ok(())
    }

    #[test]
    fn improvement_scan_emits_review_gated_governance_candidates_without_mutation(
    ) -> anyhow::Result<()> {
        let root = temp_root("improvement_scan_governance")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        write(&root, "core/projects/tasks/queue.jsonl", "")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;
        let queue_before = fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?;

        let scan = scan_arandur_improvements(&root)?;

        assert_eq!(scan["contract"], "arda.arandur.improvement_scan.v1");
        assert_eq!(scan["review_required"], true);
        assert_eq!(
            scan["mutation_policy"],
            "read_only_recommendations_no_state_mutation"
        );
        assert!(scan["improvements"]
            .as_array()
            .map(|items| items.iter().any(|item| {
                item["candidate_id"] == "governance_runtime_policy_surface"
                    && item["review_required"] == true
                    && item["domain"] == "governance"
            }))
            .unwrap_or(false));
        assert_eq!(
            fs::read_to_string(root.join("core/projects/tasks/queue.jsonl"))?,
            queue_before
        );
        Ok(())
    }

    #[test]
    fn promote_level_write_is_no_op_when_already_level_two() -> anyhow::Result<()> {
        let root = temp_root("promote_already_level_two")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":2,\"authority\":\"agent_generated\",\"review_required\":true,\"mode\":\"append_only_review_gated_candidate_surfaces\"}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        let runtime_before = fs::read_to_string(root.join("core/state/arandur/runtime.json"))?;

        let promotion = promote_arandur_level(
            &root,
            2,
            true,
            Some("operator re-ran promotion after Level 2 was active"),
        )?;

        assert_eq!(promotion["status"], "already_at_or_above_target_level");
        assert_eq!(promotion["runtime_mutated"], false);
        assert_eq!(
            promotion["mutation_policy"],
            "no_op_already_at_or_above_target_level"
        );
        assert_eq!(
            fs::read_to_string(root.join("core/state/arandur/runtime.json"))?,
            runtime_before
        );
        Ok(())
    }

    #[test]
    fn promote_level_dry_run_reports_approval_without_mutating_runtime() -> anyhow::Result<()> {
        let root = temp_root("promote_dry_run")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true,\"mode\":\"observe_plan_recommend\"}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;
        let runtime_before = fs::read_to_string(root.join("core/state/arandur/runtime.json"))?;

        let promotion =
            promote_arandur_level(&root, 2, false, Some("operator approved next-step dry-run"))?;

        assert_eq!(
            promotion["contract"],
            "arda.arandur.level_promotion.v1"
        );
        assert_eq!(promotion["target_level"], 2);
        assert_eq!(promotion["write"], false);
        assert_eq!(promotion["status"], "dry_run_ready_for_explicit_write");
        assert_eq!(promotion["runtime_mutated"], false);
        assert_eq!(
            fs::read_to_string(root.join("core/state/arandur/runtime.json"))?,
            runtime_before
        );
        Ok(())
    }

    #[test]
    fn bounded_mutation_classes_report_level_two_contract() -> anyhow::Result<()> {
        let root = temp_root("mutation_classes")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":2,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;

        let report = report_bounded_mutation_classes(&root)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.bounded_mutation_classes.v1"
        );
        assert_eq!(report["current_level"], 2);
        assert_eq!(report["mutation_policy"], "contract_only_no_mutation");
        assert_eq!(
            report["classes"].as_array().map(|classes| classes
                .iter()
                .any(|class| class["class_id"] == "recommendation_ledger_append")),
            Some(true)
        );
        Ok(())
    }

    #[test]
    fn verify_mutation_reports_pre_post_evidence_without_default_append() -> anyhow::Result<()> {
        let root = temp_root("verify_mutation")?;
        write(
            &root,
            "data/arandur/recommendations.jsonl",
            "{\"recommendation_id\":\"rec1\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;
        let snapshot = file_snapshot(&root, ARANDUR_RECOMMENDATIONS_PATH)?;
        let pre_sha1 = snapshot
            .get("sha1")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing sha1"))?;
        let pre_bytes = snapshot
            .get("bytes")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("missing bytes"))?;

        write(
            &root,
            "data/arandur/recommendations.jsonl",
            "{\"recommendation_id\":\"rec1\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"recommendation_id\":\"rec2\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;

        let report = verify_bounded_mutation(
            &root,
            "recommendation_ledger_append",
            ARANDUR_RECOMMENDATIONS_PATH,
            Some(pre_sha1),
            Some(pre_bytes),
            false,
        )?;

        assert_eq!(
            report["contract"],
            "arda.arandur.bounded_mutation_verification.v1"
        );
        assert_eq!(report["verification_status"], "passed");
        assert_eq!(report["checks"]["target_matches_class"], true);
        assert!(!root.join(ARANDUR_MUTATION_EVIDENCE_PATH).exists());
        Ok(())
    }

    #[test]
    fn rollback_report_appends_review_gated_evidence_when_requested() -> anyhow::Result<()> {
        let root = temp_root("rollback_report")?;
        write(
            &root,
            "data/arandur/recommendations.jsonl",
            "{\"recommendation_id\":\"rec1\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;

        let report = report_bounded_rollback_evidence(
            &root,
            "recommendation_ledger_append",
            ARANDUR_RECOMMENDATIONS_PATH,
            "postcheck mismatch in supervised mutation dry-run",
            true,
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MUTATION_EVIDENCE_PATH))?;

        assert_eq!(report["contract"], "arda.arandur.rollback_evidence.v1");
        assert_eq!(report["ledger_appended"], true);
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["evidence_type"], "rollback_report");
        Ok(())
    }

    #[test]
    fn promote_level_write_updates_runtime_only_when_gates_are_ready() -> anyhow::Result<()> {
        let root = temp_root("promote_write")?;
        write(
            &root,
            "core/state/arandur/runtime.json",
            "{\"autonomy_level\":1,\"authority\":\"agent_generated\",\"review_required\":true,\"mode\":\"observe_plan_recommend\"}",
        )?;
        write(&root, "data/arandur/episodes.jsonl", "{\"episode_id\":\"ep1\",\"review_required\":true}\n{\"episode_id\":\"ep2\",\"review_required\":true}\n")?;
        write(&root, "data/arandur/recommendations.jsonl", "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_b\",\"authority\":\"agent_generated\",\"review_required\":true}\n")?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/README.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/CORE_TASK_SYNTHESIS.md",
            "authority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/PROMOTION_MATRIX.md",
            "research claims remain review-gated\nauthority: agent_generated\nreview_required: true\n",
        )?;
        write(
            &root,
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17/recommended_queue_entries.jsonl",
            "{\"id\":\"candidate_b\",\"status\":\"candidate\"}\n",
        )?;

        let promotion = promote_arandur_level(
            &root,
            2,
            true,
            Some("operator approved Level 2 append-only surfaces"),
        )?;
        let runtime = read_json_file_optional(&root.join("core/state/arandur/runtime.json"))?
            .ok_or_else(|| anyhow::anyhow!("runtime missing after promotion"))?;

        assert_eq!(promotion["status"], "promoted_to_level_2");
        assert_eq!(promotion["runtime_mutated"], true);
        assert_eq!(runtime["autonomy_level"], 2);
        assert_eq!(
            runtime["mode"],
            "append_only_review_gated_candidate_surfaces"
        );
        assert_eq!(runtime["review_required"], true);
        assert_eq!(
            runtime["level_2"]["canonical_queue_mutation_allowed"],
            false
        );
        assert_eq!(runtime["level_2"]["recommendation_append_allowed"], true);
        Ok(())
    }

    #[test]
    fn phase6b_scout_plan_is_review_gated_and_read_only() -> anyhow::Result<()> {
        let root = temp_root("phase6b_scout")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"market_watch\",\"title\":\"Scout public model launch signals\",\"status\":\"pending\",\"owner\":\"athena\"}\n",
        )?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"recommendation_id\":\"rec-1\",\"recommended_candidate_id\":\"pricing_watch\",\"authority\":\"agent_generated\",\"review_required\":true,\"summary\":\"Scout pricing signals without promotion\"}\n",
        )?;
        write(
            &root,
            ARANDUR_EPISODES_PATH,
            "{\"episode_id\":\"ep1\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;
        write(
            &root,
            ARANDUR_RUNTIME_PATH,
            "{\"autonomy_level\":2,\"authority\":\"agent_generated\",\"review_required\":true}",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;
        let recommendations_before = fs::read_to_string(root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
        let episodes_before = fs::read_to_string(root.join(ARANDUR_EPISODES_PATH))?;

        let plan = plan_arandur_phase6b_scout(&root, "public internet opportunity scouting", 2)?;

        assert_eq!(plan["contract"], "arda.arandur.phase6b_scout_plan.v1");
        assert_eq!(plan["authority"], "agent_generated");
        assert_eq!(plan["review_required"], true);
        assert_eq!(plan["phase"], "6B");
        assert_eq!(
            plan["mutation_policy"],
            "read_only_mission_planning_no_network_calls"
        );
        assert_eq!(plan["scout_policy"]["internet_access_performed"], false);
        assert_eq!(plan["scout_policy"]["execution_allowed"], false);
        assert_eq!(plan["candidate_missions"].as_array().map(Vec::len), Some(2));
        assert!(plan["candidate_missions"]
            .as_array()
            .map(|missions| missions.iter().all(review_gated_value))
            .unwrap_or(false));
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_RECOMMENDATIONS_PATH))?,
            recommendations_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_EPISODES_PATH))?,
            episodes_before
        );
        Ok(())
    }

    #[test]
    fn phase6c_scout_execute_is_dry_run_and_review_gated_by_default() -> anyhow::Result<()> {
        let root = temp_root("phase6c_scout_execute")?;
        write(&root, TASK_QUEUE_PATH, "{\"id\":\"existing\"}\n")?;
        write(
            &root,
            "tmp/scout_notes.md",
            "Observed three vendor pricing changes.\nCitations checked manually.\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;

        let report = execute_arandur_phase6c_scout(
            &root,
            "mission_public_pricing_watch",
            "public internet pricing scouting",
            vec!["https://example.com/pricing".to_string()],
            Some(Path::new("tmp/scout_notes.md")),
            false,
            None,
        )?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6c_scout_execution.v1"
        );
        assert_eq!(report["phase"], "6C");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(
            report["scout_policy"]["network_access_performed_by_command"],
            false
        );
        assert_eq!(report["scout_policy"]["research_claims_review_gated"], true);
        assert_eq!(report["candidate_finding"]["authority"], "agent_generated");
        assert!(!root.join(ARANDUR_SCOUT_FINDINGS_PATH).exists());
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        Ok(())
    }

    #[test]
    fn phase6c_scout_execute_appends_only_with_justification_and_is_idempotent(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6c_scout_execute_write")?;
        write(
            &root,
            "tmp/scout_notes.md",
            "Finding: demand for local sovereign agents.\n",
        )?;

        let first = execute_arandur_phase6c_scout(
            &root,
            "mission_sovereign_agents",
            "public internet opportunity scouting",
            vec!["https://example.com/agents".to_string()],
            Some(Path::new("tmp/scout_notes.md")),
            true,
            Some("operator reviewed cited scout notes"),
        )?;
        let second = execute_arandur_phase6c_scout(
            &root,
            "mission_sovereign_agents",
            "public internet opportunity scouting",
            vec!["https://example.com/agents".to_string()],
            Some(Path::new("tmp/scout_notes.md")),
            true,
            Some("operator reviewed cited scout notes"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_SCOUT_FINDINGS_PATH))?;

        assert_eq!(first["status"], "scout_finding_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["mission_id"], "mission_sovereign_agents");
        Ok(())
    }

    #[test]
    fn phase6d_pattern_synthesis_aggregates_ledgers_without_default_mutation() -> anyhow::Result<()>
    {
        let root = temp_root("phase6d_pattern_synthesis")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"task_a\",\"owner\":\"athena\",\"status\":\"pending\"}\n{\"id\":\"task_b\",\"owner\":\"athena\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"cand1\",\"authority\":\"agent_generated\",\"review_required\":true,\"domain\":\"knowledge\"}\n",
        )?;
        write(
            &root,
            ARANDUR_SCOUT_FINDINGS_PATH,
            "{\"finding_id\":\"find1\",\"mission_id\":\"mission1\",\"authority\":\"agent_generated\",\"review_required\":true,\"scope\":\"public internet opportunity scouting\",\"source_urls\":[\"https://example.com\"]}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;

        let synthesis = synthesize_arandur_phase6d_patterns(&root, false, None)?;

        assert_eq!(
            synthesis["contract"],
            "arda.arandur.phase6d_pattern_synthesis.v1"
        );
        assert_eq!(synthesis["phase"], "6D");
        assert_eq!(synthesis["status"], "dry_run_no_mutation");
        assert_eq!(synthesis["review_required"], true);
        assert_eq!(
            synthesis["pattern_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert!(synthesis["patterns"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false));
        assert!(!root.join(ARANDUR_PATTERN_SYNTHESIS_PATH).exists());
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        Ok(())
    }

    #[test]
    fn phase6d_pattern_synthesis_write_is_append_only_and_idempotent() -> anyhow::Result<()> {
        let root = temp_root("phase6d_pattern_synthesis_write")?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"cand1\",\"authority\":\"agent_generated\",\"review_required\":true,\"domain\":\"governance\"}\n",
        )?;
        write(
            &root,
            ARANDUR_SCOUT_FINDINGS_PATH,
            "{\"finding_id\":\"find1\",\"mission_id\":\"mission1\",\"authority\":\"agent_generated\",\"review_required\":true,\"scope\":\"governance scouting\",\"source_urls\":[\"https://example.com\"]}\n",
        )?;

        let first = synthesize_arandur_phase6d_patterns(
            &root,
            true,
            Some("operator requested review-gated pattern ledger append"),
        )?;
        let second = synthesize_arandur_phase6d_patterns(
            &root,
            true,
            Some("operator requested review-gated pattern ledger append"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_PATTERN_SYNTHESIS_PATH))?;

        assert_eq!(first["status"], "patterns_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["phase"], "6D");
        Ok(())
    }

    #[test]
    fn phase6e_mission_promotion_is_dry_run_and_review_gated_by_default() -> anyhow::Result<()> {
        let root = temp_root("phase6e_mission_promotion_dry_run")?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_business_opportunities\",\"authority\":\"agent_generated\",\"review_required\":true,\"recommended_action\":\"review_candidate_for_possible_canonical_queue_promotion\"}\n",
        )?;
        write(
            &root,
            ARANDUR_SCOUT_FINDINGS_PATH,
            "{\"finding_id\":\"find1\",\"mission_id\":\"candidate_business_opportunities_phase6c\",\"authority\":\"agent_generated\",\"review_required\":true,\"scope\":\"public internet opportunity scouting\",\"source_urls\":[\"https://example.com/agents\"]}\n",
        )?;
        write(
            &root,
            ARANDUR_PATTERN_SYNTHESIS_PATH,
            "{\"pattern_synthesis_id\":\"phase6d_patterns_abc\",\"authority\":\"agent_generated\",\"review_required\":true,\"patterns\":[{\"pattern_id\":\"scout_scope_public_internet_opportunity_scouting\",\"kind\":\"scout_scope_focus\",\"summary\":\"Scout findings repeatedly target public internet opportunity scouting.\",\"authority\":\"agent_generated\",\"review_required\":true}]}\n",
        )?;
        let recommendations_before = fs::read_to_string(root.join(ARANDUR_RECOMMENDATIONS_PATH))?;

        let report = promote_arandur_phase6e_missions(&root, false, None)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6e_mission_promotion.v1"
        );
        assert_eq!(report["phase"], "6E");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(
            report["promotion_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert_eq!(report["promotion_policy"]["dry_run_default"], true);
        assert!(report["candidate_missions"]
            .as_array()
            .map(|items| items.iter().all(review_gated_value))
            .unwrap_or(false));
        assert!(!root.join(ARANDUR_MISSION_CANDIDATES_PATH).exists());
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_RECOMMENDATIONS_PATH))?,
            recommendations_before
        );
        Ok(())
    }

    #[test]
    fn phase6f_mission_candidate_review_is_dry_run_and_review_gated_by_default(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6f_mission_candidate_review_dry_run")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"existing_task\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_CANDIDATES_PATH,
            "{\"mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"source_pattern_id\":\"scout_scope_public_internet_opportunity_scouting\",\"supporting_scout_finding_ids\":[\"find1\"],\"supporting_recommendation_ids\":[\"rec1\"],\"promotion_gate\":{\"human_review_required\":true,\"canonical_queue_mutation_allowed\":false}}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;
        let candidates_before = fs::read_to_string(root.join(ARANDUR_MISSION_CANDIDATES_PATH))?;

        let report = review_arandur_phase6f_mission_candidates(&root, None, false, None)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6f_mission_candidate_review.v1"
        );
        assert_eq!(report["phase"], "6F");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(report["review_policy"]["dry_run_default"], true);
        assert_eq!(
            report["review_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert!(report["review_packets"]
            .as_array()
            .map(|items| items.iter().all(review_gated_value))
            .unwrap_or(false));
        assert!(!root.join(ARANDUR_MISSION_REVIEWS_PATH).exists());
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_MISSION_CANDIDATES_PATH))?,
            candidates_before
        );
        Ok(())
    }

    #[test]
    fn phase6f_mission_candidate_review_write_requires_justification_and_is_idempotent(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6f_mission_candidate_review_write")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_CANDIDATES_PATH,
            "{\"mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"source_pattern_id\":\"scout_scope_public_internet_opportunity_scouting\",\"supporting_scout_finding_ids\":[\"find1\"],\"supporting_recommendation_ids\":[\"rec1\"],\"promotion_gate\":{\"human_review_required\":true,\"canonical_queue_mutation_allowed\":false}}\n",
        )?;
        let missing_justification =
            review_arandur_phase6f_mission_candidates(&root, None, true, None);
        assert!(missing_justification.is_err());

        let first = review_arandur_phase6f_mission_candidates(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded review bridge append"),
        )?;
        let second = review_arandur_phase6f_mission_candidates(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded review bridge append"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MISSION_REVIEWS_PATH))?;

        assert_eq!(first["status"], "mission_reviews_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["phase"], "6F");
        assert_eq!(ledger[0]["authority"], "agent_generated");
        assert_eq!(
            ledger[0]["source_mission_candidate_id"],
            "phase6e_candidate_business"
        );
        assert_eq!(
            ledger[0]["decision"],
            "approved_for_bounded_mission_packet_drafting"
        );
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn phase6e_mission_promotion_write_requires_justification_and_is_idempotent(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6e_mission_promotion_write")?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"recommendation_id\":\"rec1\",\"recommended_candidate_id\":\"candidate_business_opportunities\",\"authority\":\"agent_generated\",\"review_required\":true,\"candidate\":{\"title\":\"Review ATHENA human info cluster: Business Opportunities\",\"owner\":\"athena\",\"priority\":\"medium\"}}\n",
        )?;
        write(
            &root,
            ARANDUR_SCOUT_FINDINGS_PATH,
            "{\"finding_id\":\"find1\",\"mission_id\":\"candidate_business_opportunities_phase6c\",\"authority\":\"agent_generated\",\"review_required\":true,\"scope\":\"public internet opportunity scouting\",\"source_urls\":[\"https://example.com/agents\"]}\n",
        )?;
        write(
            &root,
            ARANDUR_PATTERN_SYNTHESIS_PATH,
            "{\"pattern_synthesis_id\":\"phase6d_patterns_abc\",\"authority\":\"agent_generated\",\"review_required\":true,\"patterns\":[{\"pattern_id\":\"scout_scope_public_internet_opportunity_scouting\",\"kind\":\"scout_scope_focus\",\"summary\":\"Scout findings repeatedly target public internet opportunity scouting.\",\"authority\":\"agent_generated\",\"review_required\":true}]}\n",
        )?;

        let missing_justification = promote_arandur_phase6e_missions(&root, true, None);
        assert!(missing_justification.is_err());

        let first = promote_arandur_phase6e_missions(
            &root,
            true,
            Some("operator approved bounded 6E mission candidate append"),
        )?;
        let second = promote_arandur_phase6e_missions(
            &root,
            true,
            Some("operator approved bounded 6E mission candidate append"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MISSION_CANDIDATES_PATH))?;

        assert_eq!(first["status"], "mission_candidates_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["phase"], "6E");
        assert_eq!(ledger[0]["authority"], "agent_generated");
        assert_eq!(
            ledger[0]["mutation_policy"]["canonical_queue"],
            "not_mutated_by_phase6e_mission_promotion"
        );
        Ok(())
    }

    #[test]
    fn phase6g_mission_approval_request_is_dry_run_and_review_gated_by_default(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6g_mission_approval_request_dry_run")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"existing_task\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_CANDIDATES_PATH,
            "{\"mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"promotion_gate\":{\"human_review_required\":true,\"canonical_queue_mutation_allowed\":false}}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_REVIEWS_PATH,
            "{\"mission_review_id\":\"phase6f_review_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6F\",\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"decision\":\"approved_for_bounded_mission_packet_drafting\"}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;
        let reviews_before = fs::read_to_string(root.join(ARANDUR_MISSION_REVIEWS_PATH))?;

        let report = request_arandur_phase6g_mission_approval(&root, None, false, None)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6g_mission_approval_request_surface.v1"
        );
        assert_eq!(report["phase"], "6G");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(report["approval_request_policy"]["dry_run_default"], true);
        assert_eq!(
            report["approval_request_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert_eq!(report["queue_integrity"]["canonical_queue_unchanged"], true);
        assert!(report["approval_requests"]
            .as_array()
            .map(|items| items.iter().all(review_gated_value))
            .unwrap_or(false));
        assert!(!root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH).exists());
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_MISSION_REVIEWS_PATH))?,
            reviews_before
        );
        Ok(())
    }

    #[test]
    fn phase6g_mission_approval_request_write_requires_justification_and_is_idempotent(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6g_mission_approval_request_write")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_CANDIDATES_PATH,
            "{\"mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"promotion_gate\":{\"human_review_required\":true,\"canonical_queue_mutation_allowed\":false}}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_REVIEWS_PATH,
            "{\"mission_review_id\":\"phase6f_review_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6F\",\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"decision\":\"approved_for_bounded_mission_packet_drafting\"}\n",
        )?;
        let missing_justification =
            request_arandur_phase6g_mission_approval(&root, None, true, None);
        assert!(missing_justification.is_err());

        let first = request_arandur_phase6g_mission_approval(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded 6G approval request append"),
        )?;
        let second = request_arandur_phase6g_mission_approval(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded 6G approval request append"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?;

        assert_eq!(first["status"], "mission_approval_requests_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["phase"], "6G");
        assert_eq!(ledger[0]["authority"], "agent_generated");
        assert_eq!(
            ledger[0]["source_mission_candidate_id"],
            "phase6e_candidate_business"
        );
        assert_eq!(
            ledger[0]["bounded_output"]["emits_canonical_task_queue_entry"],
            false
        );
        assert_eq!(
            ledger[0]["mutation_policy"]["canonical_queue"],
            "not_mutated_by_phase6g_mission_approval_request"
        );
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn phase6g_mission_approval_decision_is_append_only_and_idempotent() -> anyhow::Result<()> {
        let root = temp_root("phase6g_mission_approval_decision")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "{\"approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"source_mission_review_id\":\"phase6f_review_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6G\",\"bounded_recommendation\":{\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\"},\"bounded_output\":{\"emits_canonical_task_queue_entry\":false}}\n",
        )?;
        let missing_justification = record_arandur_phase6g_mission_approval_decision(
            &root,
            "phase6g_approval_business",
            "approved",
            "",
        );
        assert!(missing_justification.is_err());

        let first = record_arandur_phase6g_mission_approval_decision(
            &root,
            "phase6g_approval_business",
            "approved",
            "operator approved Phase 6G request for bounded queue staging",
        )?;
        let second = record_arandur_phase6g_mission_approval_decision(
            &root,
            "phase6g_approval_business",
            "approved",
            "operator approved Phase 6G request for bounded queue staging",
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?;

        assert_eq!(first["status"], "mission_approval_decision_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 2);
        let decision = ledger
            .iter()
            .find(|record| record.get("approval_decision_id").is_some())
            .expect("approval decision record");
        assert_eq!(decision["approval_request_id"], "phase6g_approval_business");
        assert_eq!(decision["approval_status"], "approved");
        assert_eq!(decision["authority"], "operator_approved_agent_execution");
        assert_eq!(decision["review_required"], true);
        assert_eq!(
            decision["mutation_policy"]["canonical_queue"],
            "not_mutated_by_phase6g_mission_approval_decision"
        );
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn phase6h_mission_queue_proposal_is_dry_run_and_review_gated_by_default() -> anyhow::Result<()>
    {
        let root = temp_root("phase6h_mission_queue_proposal_dry_run")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"existing_task\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "{\"approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6G\",\"bounded_recommendation\":{\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"future_action_required\":\"separate explicit human/operator approval command before canonical queue creation\"},\"bounded_output\":{\"emits_canonical_task_queue_entry\":false}}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;
        let approvals_before =
            fs::read_to_string(root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?;

        let report = propose_arandur_phase6h_mission_queue_entries(&root, None, false, None)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6h_mission_queue_proposal_surface.v1"
        );
        assert_eq!(report["phase"], "6H");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(report["queue_proposal_policy"]["dry_run_default"], true);
        assert_eq!(
            report["queue_proposal_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert_eq!(report["queue_integrity"]["canonical_queue_unchanged"], true);
        assert!(report["queue_proposals"]
            .as_array()
            .map(|items| items.iter().all(review_gated_value))
            .unwrap_or(false));
        assert!(!root.join(ARANDUR_MISSION_QUEUE_PROPOSALS_PATH).exists());
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_MISSION_APPROVAL_REQUESTS_PATH))?,
            approvals_before
        );
        Ok(())
    }

    #[test]
    fn phase6h_mission_queue_proposal_write_requires_justification_and_is_idempotent(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6h_mission_queue_proposal_write")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "{\"approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6G\",\"bounded_recommendation\":{\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\"},\"bounded_output\":{\"emits_canonical_task_queue_entry\":false}}\n",
        )?;
        let missing_justification =
            propose_arandur_phase6h_mission_queue_entries(&root, None, true, None);
        assert!(missing_justification.is_err());

        let first = propose_arandur_phase6h_mission_queue_entries(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded 6H queue proposal append"),
        )?;
        let second = propose_arandur_phase6h_mission_queue_entries(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded 6H queue proposal append"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MISSION_QUEUE_PROPOSALS_PATH))?;

        assert_eq!(first["status"], "mission_queue_proposals_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["phase"], "6H");
        assert_eq!(ledger[0]["authority"], "agent_generated");
        assert_eq!(
            ledger[0]["source_mission_candidate_id"],
            "phase6e_candidate_business"
        );
        assert_eq!(
            ledger[0]["bounded_output"]["emits_canonical_task_queue_entry"],
            false
        );
        assert_eq!(
            ledger[0]["mutation_policy"]["canonical_queue"],
            "not_mutated_by_phase6h_mission_queue_proposal"
        );
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn phase6i_mission_queue_write_request_is_dry_run_and_review_gated_by_default(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6i_mission_queue_write_request_dry_run")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"existing_task\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_QUEUE_PROPOSALS_PATH,
            "{\"queue_proposal_id\":\"phase6h_queue_business\",\"source_approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6H\",\"proposed_queue_entry\":{\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"authority\":\"agent_generated\",\"review_required\":true,\"requires_separate_operator_queue_write\":true},\"bounded_output\":{\"emits_canonical_task_queue_entry\":false}}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;
        let proposals_before = fs::read_to_string(root.join(ARANDUR_MISSION_QUEUE_PROPOSALS_PATH))?;

        let report = request_arandur_phase6i_mission_queue_write(&root, None, false, None)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6i_mission_queue_write_request_surface.v1"
        );
        assert_eq!(report["phase"], "6I");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(
            report["queue_write_request_policy"]["dry_run_default"],
            true
        );
        assert_eq!(
            report["queue_write_request_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert_eq!(report["queue_integrity"]["canonical_queue_unchanged"], true);
        assert!(report["queue_write_requests"]
            .as_array()
            .map(|items| items.iter().all(review_gated_value))
            .unwrap_or(false));
        assert!(!root
            .join(ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH)
            .exists());
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_MISSION_QUEUE_PROPOSALS_PATH))?,
            proposals_before
        );
        Ok(())
    }

    #[test]
    fn phase6i_mission_queue_write_request_write_requires_justification_and_is_idempotent(
    ) -> anyhow::Result<()> {
        let root = temp_root("phase6i_mission_queue_write_request_write")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_QUEUE_PROPOSALS_PATH,
            "{\"queue_proposal_id\":\"phase6h_queue_business\",\"source_approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6H\",\"proposed_queue_entry\":{\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"authority\":\"agent_generated\",\"review_required\":true},\"bounded_output\":{\"emits_canonical_task_queue_entry\":false}}\n",
        )?;
        let missing_justification =
            request_arandur_phase6i_mission_queue_write(&root, None, true, None);
        assert!(missing_justification.is_err());

        let first = request_arandur_phase6i_mission_queue_write(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded 6I queue write request append"),
        )?;
        let second = request_arandur_phase6i_mission_queue_write(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved bounded 6I queue write request append"),
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH))?;

        assert_eq!(first["status"], "mission_queue_write_requests_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(ledger.len(), 1);
        assert!(ledger.iter().all(review_gated_value));
        assert_eq!(ledger[0]["phase"], "6I");
        assert_eq!(ledger[0]["authority"], "agent_generated");
        assert_eq!(ledger[0]["write_pending"], true);
        assert_eq!(
            ledger[0]["source_approval_request_id"],
            "phase6g_approval_business"
        );
        assert_eq!(
            ledger[0]["source_mission_candidate_id"],
            "phase6e_candidate_business"
        );
        assert_eq!(
            ledger[0]["bounded_output"]["emits_canonical_task_queue_entry"],
            false
        );
        assert_eq!(
            ledger[0]["mutation_policy"]["canonical_queue"],
            "not_mutated_by_phase6i_mission_queue_write_request"
        );
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn phase6j_queue_write_is_dry_run_and_requires_approved_requests() -> anyhow::Result<()> {
        let root = temp_root("phase6j_queue_write_dry_run")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"existing_task\",\"status\":\"pending\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "{\"approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"approval_status\":\"approved\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH,
            "{\"queue_write_request_id\":\"phase6i_write_business\",\"source_approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6I\",\"write_pending\":true,\"justification\":\"operator staged request\",\"requested_queue_entry\":{\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"authority\":\"agent_generated\",\"review_required\":true}}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;
        let requests_before =
            fs::read_to_string(root.join(ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH))?;

        let report = execute_arandur_phase6j_queue_write(&root, None, false, None)?;

        assert_eq!(
            report["contract"],
            "arda.arandur.phase6j_canonical_queue_write_surface.v1"
        );
        assert_eq!(report["phase"], "6J");
        assert_eq!(report["status"], "dry_run_no_mutation");
        assert_eq!(report["review_required"], true);
        assert_eq!(report["queue_write_policy"]["dry_run_default"], true);
        assert_eq!(
            report["queue_write_policy"]["approval_status_required"],
            "approved"
        );
        assert_eq!(report["queue_integrity"]["canonical_queue_mutated"], false);
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        assert_eq!(
            fs::read_to_string(root.join(ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH))?,
            requests_before
        );
        Ok(())
    }

    #[test]
    fn phase6j_queue_write_fails_fast_without_matching_approved_approval() -> anyhow::Result<()> {
        let root = temp_root("phase6j_queue_write_missing_approval")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "{\"approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"approval_status\":\"needs_changes\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH,
            "{\"queue_write_request_id\":\"phase6i_write_business\",\"source_approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6I\",\"write_pending\":true,\"justification\":\"operator staged request\",\"requested_queue_entry\":{\"title\":\"Business opportunity scout mission\"}}\n",
        )?;

        let result = execute_arandur_phase6j_queue_write(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved canonical queue write"),
        );

        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("approved approval record"));
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn gate_next_selects_review_gated_recommendation_without_queue_mutation() -> anyhow::Result<()>
    {
        let root = temp_root("gate_next")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"stale-objective\",\"status\":\"pending\"}\n{\"id\":\"stale-objective\",\"status\":\"completed\"}\n",
        )?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"objective_id\":\"stale-objective\",\"status\":\"candidate\",\"action_class\":\"recommendation_ledger_append\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"objective_id\":\"next-objective\",\"status\":\"candidate\",\"action_class\":\"recommendation_ledger_append\",\"authority\":\"agent_generated\",\"review_required\":true,\"title\":\"Next automation gate\"}\n",
        )?;
        let queue_before = fs::read_to_string(root.join(TASK_QUEUE_PATH))?;

        let report = report_arandur_gate_next(&root)?;

        assert_eq!(report["contract"], "arda.arandur.gate_next.v1");
        assert_eq!(report["status"], "next_review_gated_candidate_selected");
        assert_eq!(
            report["selected_candidate"]["objective_id"],
            "next-objective"
        );
        assert_eq!(
            report["selected_candidate"]["blocked_reason_code"],
            "operator_approval_packet_missing"
        );
        assert_eq!(
            report["selected_candidate"]["governance_class"],
            "review_gated_recommendation"
        );
        assert_eq!(
            report["mutation_policy"]["canonical_queue_mutation_allowed"],
            false
        );
        assert_eq!(
            fs::read_to_string(root.join(TASK_QUEUE_PATH))?,
            queue_before
        );
        Ok(())
    }

    #[test]
    fn gate_blocked_groups_candidates_by_reason_and_class() -> anyhow::Result<()> {
        let root = temp_root("gate_blocked")?;
        write(
            &root,
            TASK_QUEUE_PATH,
            "{\"id\":\"stale-objective\",\"status\":\"pending\"}\n{\"id\":\"stale-objective\",\"status\":\"completed\"}\n",
        )?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"objective_id\":\"valid-objective\",\"status\":\"candidate\",\"action_class\":\"recommendation_ledger_append\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"objective_id\":\"stale-objective\",\"status\":\"candidate\",\"action_class\":\"recommendation_ledger_append\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"objective_id\":\"unknown-objective\",\"status\":\"candidate\",\"action_class\":\"mystery_mutation\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"objective_id\":\"unsafe-objective\",\"status\":\"candidate\",\"action_class\":\"canonical_queue_write\",\"authority\":\"agent_generated\",\"review_required\":true}\n{\"objective_id\":\"approved-objective\",\"status\":\"candidate\",\"action_class\":\"recommendation_ledger_append\",\"authority\":\"agent_generated\",\"review_required\":true,\"approval_packet\":{\"status\":\"approved\",\"id\":\"APPR-approved\"}}\n",
        )?;

        let report = report_arandur_gate_blocked(&root)?;

        assert_eq!(report["contract"], "arda.arandur.gate_blocked.v1");
        assert_eq!(
            report["blocked_groups"]["operator_approval_packet_missing"],
            1
        );
        assert_eq!(
            report["blocked_groups"]["stale_or_superseded_queue_record"],
            1
        );
        assert_eq!(report["blocked_groups"]["unknown_action_class"], 1);
        assert_eq!(report["blocked_groups"]["unsafe_action_class"], 1);
        assert_eq!(report["class_groups"]["review_gated_recommendation"], 1);
        assert_eq!(
            report["class_groups"]["stale_superseded_raw_queue_record"],
            1
        );
        assert_eq!(report["class_groups"]["unknown_action_class"], 1);
        assert_eq!(report["class_groups"]["unsafe_blocked"], 1);
        assert_eq!(report["class_groups"]["operator_approved"], 1);
        Ok(())
    }

    #[test]
    fn gate_approve_appends_status_approved_packet_and_is_idempotent() -> anyhow::Result<()> {
        let root = temp_root("gate_approve")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"objective_id\":\"next-objective\",\"status\":\"candidate\",\"action_class\":\"recommendation_ledger_append\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;

        let first = approve_arandur_gate_candidate(
            &root,
            "next-objective",
            "operator approved next review-gated automation lane",
        )?;
        let second = approve_arandur_gate_candidate(
            &root,
            "next-objective",
            "operator approved next review-gated automation lane",
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
        let approval_records: Vec<_> = ledger
            .iter()
            .filter(|record| record.get("approval_packet").is_some())
            .collect();

        assert_eq!(first["contract"], "arda.arandur.gate_approval.v1");
        assert_eq!(first["status"], "approval_packet_recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(approval_records.len(), 1);
        assert_eq!(approval_records[0]["approval_packet"]["status"], "approved");
        assert_eq!(approval_records[0]["objective_id"], "next-objective");
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn gate_deny_appends_blocking_decision_without_approving_candidate() -> anyhow::Result<()> {
        let root = temp_root("gate_deny")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_RECOMMENDATIONS_PATH,
            "{\"objective_id\":\"unsafe-objective\",\"status\":\"candidate\",\"action_class\":\"canonical_queue_write\",\"authority\":\"agent_generated\",\"review_required\":true}\n",
        )?;

        let report = deny_arandur_gate_candidate(
            &root,
            "unsafe-objective",
            "unsafe action class remains blocked at Level 2",
        )?;
        let ledger = read_jsonl_values(&root.join(ARANDUR_RECOMMENDATIONS_PATH))?;
        let denial = ledger
            .iter()
            .find(|record| record["approval_packet"]["status"] == "denied")
            .expect("denial packet appended");

        assert_eq!(report["contract"], "arda.arandur.gate_denial.v1");
        assert_eq!(report["status"], "denial_packet_recorded");
        assert_eq!(denial["approval_packet"]["status"], "denied");
        assert_eq!(denial["blocked_reason_code"], "unsafe_action_class");
        assert_eq!(
            report_arandur_gate_next(&root)?["status"],
            "no_review_gated_candidate_available"
        );
        assert_eq!(fs::read_to_string(root.join(TASK_QUEUE_PATH))?, "");
        Ok(())
    }

    #[test]
    fn phase6j_queue_write_appends_canonical_queue_once_with_integrity_report() -> anyhow::Result<()>
    {
        let root = temp_root("phase6j_queue_write_append")?;
        write(&root, TASK_QUEUE_PATH, "")?;
        write(
            &root,
            ARANDUR_MISSION_APPROVAL_REQUESTS_PATH,
            "{\"approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"approval_status\":\"approved\"}\n",
        )?;
        write(
            &root,
            ARANDUR_MISSION_QUEUE_WRITE_REQUESTS_PATH,
            "{\"queue_write_request_id\":\"phase6i_write_business\",\"source_approval_request_id\":\"phase6g_approval_business\",\"source_mission_candidate_id\":\"phase6e_candidate_business\",\"authority\":\"agent_generated\",\"review_required\":true,\"phase\":\"6I\",\"write_pending\":true,\"justification\":\"operator staged request\",\"requested_queue_entry\":{\"id\":\"task_phase6j_business\",\"title\":\"Business opportunity scout mission\",\"scope\":\"public internet opportunity scouting\",\"authority\":\"agent_generated\",\"review_required\":true}}\n",
        )?;
        let missing_justification = execute_arandur_phase6j_queue_write(&root, None, true, None);
        assert!(missing_justification.is_err());

        let first = execute_arandur_phase6j_queue_write(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved canonical queue write"),
        )?;
        let second = execute_arandur_phase6j_queue_write(
            &root,
            Some("phase6e_candidate_business"),
            true,
            Some("operator approved canonical queue write"),
        )?;
        let queue = read_jsonl_values(&root.join(TASK_QUEUE_PATH))?;

        assert_eq!(first["status"], "canonical_queue_entries_appended");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0]["id"], "task_phase6j_business");
        assert_eq!(queue[0]["source"], "arandur_phase6j_canonical_queue_write");
        assert_eq!(
            queue[0]["source_queue_write_request_id"],
            "phase6i_write_business"
        );
        assert_eq!(
            queue[0]["source_approval_request_id"],
            "phase6g_approval_business"
        );
        assert_eq!(first["queue_integrity"]["canonical_queue_mutated"], true);
        assert_ne!(
            first["queue_integrity"]["before"]["sha1"],
            first["queue_integrity"]["after"]["sha1"]
        );
        Ok(())
    }
}
