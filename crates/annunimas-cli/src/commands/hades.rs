use super::super::*;
use sha1::{Digest, Sha1};

pub(crate) async fn handle(command: HadesCommands) -> anyhow::Result<()> {
    let service = HadesService::from_default_or_fallback()?;
    let default_socket_path =
        socket_path_from_env("ANNUNIMAS_HADES_SOCKET", "data/hades/hades.sock");
    match command {
        HadesCommands::Start {
            socket_path,
            http_addr,
            http_enabled,
        } => {
            let daemon = HadesDaemon::new(
                service,
                HadesDaemonConfig {
                    socket_path: expand_home(&socket_path),
                    http_enabled,
                    http_addr,
                },
            );
            daemon.run().await?;
        }
        HadesCommands::Status => {
            let out = hades_call_or_local(
                &default_socket_path,
                "status",
                serde_json::json!({}),
                || Ok(serde_json::to_value(service.status()?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::Sweep { sweep_type, path } => {
            let out = hades_call_or_local(
                &default_socket_path,
                "sweep",
                serde_json::json!({
                    "type": sweep_type,
                    "path": path
                }),
                || {
                    Ok(serde_json::to_value(
                        service.sweep(&sweep_type, path.as_deref())?,
                    )?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::Queue { limit } => {
            let out = hades_call_or_local(
                &default_socket_path,
                "queue",
                serde_json::json!({ "limit": limit }),
                || Ok(serde_json::to_value(service.queue(limit)?)?),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::Log {
            limit,
            event_filter,
        } => {
            let out = hades_call_or_local(
                &default_socket_path,
                "log",
                serde_json::json!({
                    "limit": limit,
                    "event_filter": event_filter
                }),
                || {
                    Ok(serde_json::to_value(service.log(
                        limit,
                        event_filter.as_deref(),
                        None,
                    )?)?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::QueueCompactionPlan {
            root,
            out_dir,
            operator_id,
            approved,
        } => {
            let out = queue_compaction_plan(
                &PathBuf::from(root),
                &PathBuf::from(out_dir),
                &operator_id,
                approved,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::QueueCompactionApply {
            plan,
            approval_packet,
            root,
            rollback_dir,
            apply,
        } => {
            let out = queue_compaction_apply(
                &PathBuf::from(root),
                &PathBuf::from(plan),
                &PathBuf::from(approval_packet),
                &PathBuf::from(rollback_dir),
                apply,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::Remove {
            file,
            authorized_by,
            quorum_approvers,
            quorum_evidence,
            quorum_asserted_at_utc,
        } => {
            let quorum_proof = if quorum_approvers.is_empty() && quorum_evidence.is_empty() {
                None
            } else {
                Some(QuorumProof {
                    approvers: quorum_approvers.clone(),
                    evidence: quorum_evidence.clone(),
                    asserted_at_utc: quorum_asserted_at_utc.clone(),
                })
            };
            let out = hades_call_or_local(
                &default_socket_path,
                "remove",
                serde_json::json!({
                    "file": file,
                    "authorized_by": authorized_by,
                    "quorum_proof": quorum_proof.clone()
                }),
                || {
                    Ok(serde_json::to_value(service.queue_remove_with_proof(
                        &file,
                        &authorized_by,
                        quorum_proof.clone(),
                    )?)?)
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::ImportHumanReviews { input, limit } => {
            let out = serde_json::to_value(service.import_human_lifecycle_reviews(&input, limit)?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::LifecycleAudit { root, limit } => {
            let out = serde_json::to_value(service.audit_lifecycle_review(&root, limit)?)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::LifecycleReviewQueue { root, limit } => {
            let out = service.project_lifecycle_review_queue(&root, limit)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::LifecyclePolicyReport { root, limit } => {
            let out = service.lifecycle_policy_automation_report(&root, limit)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::OrganizationAudit { root, limit } => {
            let out = service.organization_audit_report(&root, limit)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::OrganizationPlan { root, scope, limit } => {
            let out = service.organization_plan_report(&root, Some(&scope), limit)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::OrganizationApprovalPacket {
            root,
            scope,
            limit,
            operator_id,
            approved,
            out_path,
        } => {
            let out = service.organization_approval_packet(
                &root,
                Some(&scope),
                limit,
                &out_path,
                &operator_id,
                approved,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::OrganizationApply {
            approval_packet,
            root,
            apply,
        } => {
            let out = service.execute_organization_apply(&approval_packet, &root, apply)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesReviewPacket {
            root,
            raw_queue,
            limit,
            out_dir,
        } => {
            let out = service
                .project_warden_hades_operator_review_packet(&root, &raw_queue, limit, &out_dir)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesApprovalPacket {
            review_packet,
            review_ids,
            operator_id,
            decision,
            evidence,
            out_path,
        } => {
            let out = service.warden_hades_signed_approval_packet(
                &review_packet,
                &review_ids,
                &operator_id,
                &decision,
                &evidence,
                &out_path,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesDryRunReceipt {
            approval_packet,
            review_packet,
            action,
            out_path,
        } => {
            let out = service.warden_hades_dry_run_receipt(
                &approval_packet,
                &review_packet,
                &action,
                &out_path,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesMutationApprovalPacket {
            dry_run_receipt,
            operator_id,
            action,
            evidence,
            out_path,
        } => {
            let out = service.warden_hades_signed_mutation_approval_packet(
                &dry_run_receipt,
                &operator_id,
                &action,
                &evidence,
                &out_path,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesMutationPlanReceipt {
            mutation_approval_packet,
            review_packet,
            dry_run_receipt,
            action,
            out_path,
        } => {
            let out = service.warden_hades_mutation_plan_receipt(
                &mutation_approval_packet,
                &review_packet,
                &dry_run_receipt,
                &action,
                &out_path,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesFinalApplyApprovalPacket {
            mutation_plan_receipt,
            operator_id,
            action,
            rollback_plan,
            evidence,
            out_path,
        } => {
            let out = service.warden_hades_final_apply_approval_packet(
                &mutation_plan_receipt,
                &operator_id,
                &action,
                &rollback_plan,
                &evidence,
                &out_path,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::WardenHadesFinalApplyExecution {
            final_apply_approval_packet,
            action,
            archive_path,
            receipt_path,
        } => {
            let out = service.warden_hades_final_apply_execution(
                &final_apply_approval_packet,
                &action,
                &archive_path,
                &receipt_path,
            )?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::LifecycleApprovalPacket {
            root,
            limit,
            out_path,
        } => {
            let out = service.lifecycle_operator_approval_packet(&root, limit, &out_path)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::LifecycleCleanup {
            approval_packet,
            rollback_out,
            apply,
        } => {
            let out =
                service.execute_lifecycle_cleanup_plan(&approval_packet, apply, &rollback_out)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::AuditOutcomes { root, limit } => {
            let root = PathBuf::from(root);
            let out = audit_outcomes(&root, limit)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        HadesCommands::Paths => {
            let out =
                hades_call_or_local(&default_socket_path, "paths", serde_json::json!({}), || {
                    Ok(service.paths())
                })
                .await?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}

fn queue_compaction_plan(
    root: &Path,
    out_dir: &Path,
    operator_id: &str,
    approved: bool,
) -> anyhow::Result<serde_json::Value> {
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let raw = fs::read_to_string(&queue_path)?;
    let before_sha1 = sha1_hex(&raw);
    let generated_at = Utc::now();
    let run_id = format!("queue-compaction-{}", generated_at.format("%Y%m%dT%H%M%SZ"));
    let run_dir = root
        .join(out_dir)
        .join(generated_at.format("%Y-%m-%d").to_string())
        .join(&run_id);
    fs::create_dir_all(&run_dir)?;

    let mut parsed_rows = Vec::new();
    let mut invalid_lines = Vec::new();
    let mut latest_index_by_id = std::collections::BTreeMap::<String, usize>::new();

    for (line_index, line) in raw.lines().enumerate() {
        let line_no = line_index + 1;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => {
                if let Some(id) = value.get("id").and_then(|id| id.as_str()).map(str::trim) {
                    if !id.is_empty() {
                        latest_index_by_id.insert(id.to_string(), parsed_rows.len());
                    }
                }
                parsed_rows.push((line_no, line.to_string(), value));
            }
            Err(err) => invalid_lines.push(json!({
                "line": line_no,
                "error": err.to_string()
            })),
        }
    }

    let mut retained = Vec::new();
    let mut archived = Vec::new();
    for (parsed_index, (line_no, line, value)) in parsed_rows.iter().enumerate() {
        let id = value.get("id").and_then(|id| id.as_str()).map(str::trim);
        let superseded = id
            .and_then(|task_id| latest_index_by_id.get(task_id))
            .is_some_and(|latest_index| *latest_index != parsed_index);
        let record = json!({
            "source_line": line_no,
            "record": value
        });
        if superseded {
            archived.push(record);
        } else {
            retained.push((line_no, line.clone(), value.clone()));
        }
    }

    let retained_content = retained
        .iter()
        .map(|(_, _, value)| serde_json::to_string(value))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    let retained_content = if retained_content.is_empty() {
        String::new()
    } else {
        format!("{retained_content}\n")
    };
    let archive_content = archived
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    let archive_content = if archive_content.is_empty() {
        String::new()
    } else {
        format!("{archive_content}\n")
    };

    let retained_path = run_dir.join("retained_queue.jsonl");
    let archive_path = run_dir.join("archived_superseded_rows.jsonl");
    fs::write(&retained_path, retained_content.as_bytes())?;
    fs::write(&archive_path, archive_content.as_bytes())?;

    let plan_id = format!("hades_queue_compaction_{}", &sha1_hex(&before_sha1)[..12]);
    let ready_for_apply = invalid_lines.is_empty() && !archived.is_empty();
    let plan_path = run_dir.join("plan.json");
    let approval_path = run_dir.join("approval_packet.json");
    let plan = json!({
        "schema_version": "annunimas.hades.queue-compaction-plan.v1",
        "plan_id": plan_id,
        "generated_at_utc": generated_at.to_rfc3339(),
        "authority": "annunimas-cli hades queue-compaction-plan",
        "mutation_policy": "dry_run_no_queue_mutation",
        "source_queue": rel_to(root, &queue_path),
        "queue_before_sha1": before_sha1,
        "retained_queue_path": rel_to(root, &retained_path),
        "archive_path": rel_to(root, &archive_path),
        "approval_packet_path": rel_to(root, &approval_path),
        "summary": {
            "raw_rows_total": raw.lines().filter(|line| !line.trim().is_empty()).count(),
            "parsed_rows_total": parsed_rows.len(),
            "latest_task_rows_total": retained.iter().filter(|(_, _, value)| value.get("id").is_some()).count(),
            "retained_rows_total": retained.len(),
            "archive_candidate_rows_total": archived.len(),
            "invalid_json_lines_total": invalid_lines.len(),
            "ready_for_apply": ready_for_apply
        },
        "invalid_lines": invalid_lines,
        "apply_requirements": {
            "approval_packet_approved": true,
            "approval_packet_plan_id_matches": true,
            "approval_packet_queue_before_sha1_matches_current_queue": true,
            "apply_flag_required": true
        }
    });
    write_json(&plan_path, &plan)?;

    let approval = json!({
        "schema_version": "annunimas.hades.queue-compaction-approval.v1",
        "plan_id": plan["plan_id"],
        "generated_at_utc": generated_at.to_rfc3339(),
        "operator_id": operator_id,
        "approved": approved,
        "approved_at_utc": if approved { json!(generated_at.to_rfc3339()) } else { serde_json::Value::Null },
        "status": if approved { "approved" } else { "pending_operator_approval" },
        "queue_before_sha1": plan["queue_before_sha1"],
        "plan_path": rel_to(root, &plan_path),
        "archive_candidate_rows_total": archived.len(),
        "retained_rows_total": retained.len(),
        "mutation_scope": "rewrite core/projects/tasks/queue.jsonl to retained latest-by-id rows and preserve superseded rows in archive_path"
    });
    write_json(&approval_path, &approval)?;

    Ok(json!({
        "status": if ready_for_apply { "compaction_plan_ready" } else { "compaction_plan_not_ready" },
        "plan": rel_to(root, &plan_path),
        "approval_packet": rel_to(root, &approval_path),
        "archive_candidate_rows_total": archived.len(),
        "retained_rows_total": retained.len(),
        "invalid_json_lines_total": plan["summary"]["invalid_json_lines_total"],
        "approved": approved
    }))
}

fn queue_compaction_apply(
    root: &Path,
    plan_path: &Path,
    approval_path: &Path,
    rollback_dir: &Path,
    apply: bool,
) -> anyhow::Result<serde_json::Value> {
    let full_plan_path = root.join(plan_path);
    let full_approval_path = root.join(approval_path);
    let plan: serde_json::Value = serde_json::from_str(&fs::read_to_string(&full_plan_path)?)?;
    let approval: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&full_approval_path)?)?;
    let queue_rel = plan
        .get("source_queue")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("plan missing source_queue"))?;
    let queue_path = root.join(queue_rel);
    let current_queue = fs::read_to_string(&queue_path)?;
    let current_sha1 = sha1_hex(&current_queue);
    let expected_sha1 = plan
        .get("queue_before_sha1")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let plan_id = plan
        .get("plan_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let approval_plan_id = approval
        .get("plan_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let approval_sha1 = approval
        .get("queue_before_sha1")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let approved = approval
        .get("approved")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let invalid_lines_total = plan
        .pointer("/summary/invalid_json_lines_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    let mut blockers = Vec::new();
    if !approved {
        blockers.push("approval_packet_not_approved");
    }
    if approval_plan_id != plan_id {
        blockers.push("approval_packet_plan_id_mismatch");
    }
    if approval_sha1 != expected_sha1 {
        blockers.push("approval_packet_queue_sha1_mismatch");
    }
    if current_sha1 != expected_sha1 {
        blockers.push("current_queue_sha1_changed_since_plan");
    }
    if invalid_lines_total > 0 {
        blockers.push("plan_contains_invalid_json_lines");
    }
    if !blockers.is_empty() {
        return Ok(json!({
            "status": "blocked",
            "mutation_performed": false,
            "blockers": blockers,
            "current_queue_sha1": current_sha1,
            "expected_queue_sha1": expected_sha1
        }));
    }

    let retained_rel = plan
        .get("retained_queue_path")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("plan missing retained_queue_path"))?;
    let retained_path = root.join(retained_rel);
    let retained_content = fs::read_to_string(&retained_path)?;
    let retained_sha1 = sha1_hex(&retained_content);
    if !apply {
        return Ok(json!({
            "status": "dry_run_ready",
            "mutation_performed": false,
            "apply_required": true,
            "plan_id": plan_id,
            "source_queue": queue_rel,
            "queue_before_sha1": current_sha1,
            "queue_after_sha1": retained_sha1,
            "archive_path": plan["archive_path"],
            "retained_queue_path": plan["retained_queue_path"]
        }));
    }

    let generated_at = Utc::now();
    let full_rollback_dir = root.join(rollback_dir);
    fs::create_dir_all(&full_rollback_dir)?;
    let backup_path = full_rollback_dir.join(format!(
        "queue-before-compaction-{}.jsonl",
        generated_at.format("%Y%m%dT%H%M%SZ")
    ));
    fs::write(&backup_path, current_queue.as_bytes())?;
    fs::write(&queue_path, retained_content.as_bytes())?;
    let after_sha1 = sha1_hex(&fs::read_to_string(&queue_path)?);
    let receipt = json!({
        "schema_version": "annunimas.hades.queue-compaction-apply-receipt.v1",
        "generated_at_utc": generated_at.to_rfc3339(),
        "status": "applied",
        "mutation_performed": true,
        "plan_id": plan_id,
        "approval_packet": rel_to(root, &full_approval_path),
        "source_queue": queue_rel,
        "backup_path": rel_to(root, &backup_path),
        "archive_path": plan["archive_path"],
        "queue_before_sha1": current_sha1,
        "queue_after_sha1": after_sha1
    });
    let receipt_path = full_rollback_dir.join(format!(
        "queue-compaction-apply-{}.json",
        generated_at.format("%Y%m%dT%H%M%SZ")
    ));
    write_json(&receipt_path, &receipt)?;
    Ok(json!({
        "status": "applied",
        "mutation_performed": true,
        "receipt": rel_to(root, &receipt_path),
        "backup_path": rel_to(root, &backup_path),
        "queue_before_sha1": current_sha1,
        "queue_after_sha1": after_sha1
    }))
}

fn audit_outcomes(root: &Path, limit: usize) -> anyhow::Result<serde_json::Value> {
    let task_queue = read_jsonl(root.join("core/projects/tasks/queue.jsonl"), limit)?;
    let recommendations = read_jsonl(root.join("data/arandur/recommendations.jsonl"), limit)?;
    let hades_log = read_jsonl(root.join("data/hades/hades_log.jsonl"), limit)?;
    let completed_tasks = task_queue
        .iter()
        .filter(|entry| {
            entry.get("status").and_then(|value| value.as_str()) == Some("completed")
                || entry
                    .get("completed_at_utc")
                    .and_then(|value| value.as_str())
                    .is_some()
        })
        .count();
    let review_required_recommendations = recommendations
        .iter()
        .filter(|entry| {
            entry
                .get("review_required")
                .and_then(|value| value.as_bool())
                .unwrap_or(true)
        })
        .count();

    Ok(json!({
        "contract": "annunimas.hades.audit_outcomes.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "root": root,
        "limit": limit,
        "mutation_policy": "read_only_audit_no_cleanup",
        "summary": {
            "recent_task_records": task_queue.len(),
            "recent_completed_task_records": completed_tasks,
            "recent_recommendation_records": recommendations.len(),
            "review_required_recommendations": review_required_recommendations,
            "recent_hades_log_records": hades_log.len()
        },
        "evidence": {
            "task_queue": "core/projects/tasks/queue.jsonl",
            "recommendations": "data/arandur/recommendations.jsonl",
            "hades_log": "data/hades/hades_log.jsonl"
        },
        "recent": {
            "tasks": task_queue,
            "recommendations": recommendations,
            "hades_log": hades_log
        }
    }))
}

fn write_json(path: &Path, value: &serde_json::Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn sha1_hex(content: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn rel_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn read_jsonl(path: PathBuf, limit: usize) -> anyhow::Result<Vec<serde_json::Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    let mut entries = content
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .take(limit.max(1))
        .collect::<Vec<_>>();
    entries.reverse();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("annunimas-hades-{name}-{nanos}"))
    }

    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("parse json")
    }

    #[test]
    fn queue_compaction_requires_approval_and_apply_flag() -> anyhow::Result<()> {
        let root = temp_root("queue-compaction");
        let queue_path = root.join("core/projects/tasks/queue.jsonl");
        fs::create_dir_all(queue_path.parent().expect("queue parent"))?;
        fs::write(
            &queue_path,
            concat!(
                "{\"id\":\"task_a\",\"status\":\"queued\",\"title\":\"A v1\"}\n",
                "{\"id\":\"task_b\",\"status\":\"queued\",\"title\":\"B\"}\n",
                "{\"id\":\"task_a\",\"status\":\"completed\",\"title\":\"A done\"}\n"
            ),
        )?;

        let pending = queue_compaction_plan(
            &root,
            Path::new("audit/hades-queue-compaction-runs"),
            "codex",
            false,
        )?;
        assert_eq!(pending["status"], "compaction_plan_ready");
        assert_eq!(pending["archive_candidate_rows_total"], 1);
        assert_eq!(pending["retained_rows_total"], 2);

        let blocked = queue_compaction_apply(
            &root,
            Path::new(pending["plan"].as_str().expect("plan")),
            Path::new(
                pending["approval_packet"]
                    .as_str()
                    .expect("approval packet"),
            ),
            Path::new("audit/hades-queue-compaction-runs/rollback"),
            false,
        )?;
        assert_eq!(blocked["status"], "blocked");
        assert_eq!(blocked["mutation_performed"], false);
        assert_eq!(blocked["blockers"][0], "approval_packet_not_approved");

        let approved = queue_compaction_plan(
            &root,
            Path::new("audit/hades-queue-compaction-runs"),
            "codex",
            true,
        )?;
        let dry_run = queue_compaction_apply(
            &root,
            Path::new(approved["plan"].as_str().expect("plan")),
            Path::new(
                approved["approval_packet"]
                    .as_str()
                    .expect("approval packet"),
            ),
            Path::new("audit/hades-queue-compaction-runs/rollback"),
            false,
        )?;
        assert_eq!(dry_run["status"], "dry_run_ready");
        assert_eq!(dry_run["mutation_performed"], false);
        assert_eq!(fs::read_to_string(&queue_path)?.lines().count(), 3);

        let applied = queue_compaction_apply(
            &root,
            Path::new(approved["plan"].as_str().expect("plan")),
            Path::new(
                approved["approval_packet"]
                    .as_str()
                    .expect("approval packet"),
            ),
            Path::new("audit/hades-queue-compaction-runs/rollback"),
            true,
        )?;
        assert_eq!(applied["status"], "applied");
        assert_eq!(applied["mutation_performed"], true);
        let compacted = fs::read_to_string(&queue_path)?;
        assert_eq!(compacted.lines().count(), 2);
        assert!(compacted.contains("\"task_b\""));
        assert!(compacted.contains("\"A done\""));
        assert!(root
            .join(applied["backup_path"].as_str().expect("backup"))
            .exists());
        assert!(root
            .join(applied["receipt"].as_str().expect("receipt"))
            .exists());

        let plan = read_json(&root.join(approved["plan"].as_str().expect("plan")));
        assert_eq!(plan["summary"]["invalid_json_lines_total"], 0);

        let _ = fs::remove_dir_all(&root);
        Ok(())
    }
}
