use super::{
    action_record_matches_rule, append_jsonl, count_malformed_jsonl, hades_sweep_limit,
    json_value_matches_rule, read_recent_jsonl, read_sigil, scheduler_snapshot,
    should_skip_watch_file, sweep_interval_hours, HadesService, HadesState, HadesStatus,
};
use crate::types::{
    ActionKind, ActionRecord, QuorumProof, SigilState, SigilVacuumRule, SweepResult, TaskItem,
};
use arda_core::error::{ArdaError, Result};
use arda_core::task::Task;
use arda_core::try_run_bounded;
use arda_governance::record_bacon_lite;
use arda_plutus::JouleWorkUnit;
use chrono::{Duration, Utc};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

impl HadesService {
    pub fn status(&self) -> Result<HadesStatus> {
        let tasks = self.queue(10_000)?;
        let now = Utc::now();
        let mut orphans_active = 0usize;
        let mut condemned_pending = 0usize;
        let mut quarantined = 0usize;
        for t in &tasks {
            match t.action {
                ActionKind::InvestigateOrphan => orphans_active += 1,
                ActionKind::Remove => condemned_pending += 1,
                ActionKind::Quarantine => quarantined += 1,
                ActionKind::Archive => {}
            }
        }
        let state = self.load_state().unwrap_or_default();

        Ok(HadesStatus {
            last_sweep_utc: state.last_sweep_utc,
            next_sweep_utc: (now + Duration::hours(sweep_interval_hours())).to_rfc3339(),
            pending_actions: tasks.len(),
            orphans_active,
            condemned_pending,
            quarantined,
            warden_connected: true,
            malformed_queue_records: count_malformed_jsonl(&self.queue_path),
            malformed_log_records: count_malformed_jsonl(&self.log_path),
            malformed_joulework_records: count_malformed_jsonl(&self.joulework_path),
            malformed_warden_queue_records: count_malformed_jsonl(&self.warden_queue_path),
            malformed_athena_handoff_records: count_malformed_jsonl(
                &self.athena_handoff_queue_path,
            ),
            scheduler: scheduler_snapshot(),
        })
    }

    pub fn queue(&self, limit: usize) -> Result<Vec<TaskItem>> {
        let mut out = self.read_all_queue()?;
        out.reverse();
        out.truncate(limit.max(1));
        Ok(out)
    }

    pub fn log(
        &self,
        limit: usize,
        event_filter: Option<&str>,
        sigil_rule: Option<&SigilVacuumRule>,
    ) -> Result<Vec<ActionRecord>> {
        let content = fs::read_to_string(&self.log_path)?;
        let mut out = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let item: ActionRecord = match serde_json::from_str(line) {
                Ok(item) => item,
                Err(err) => {
                    tracing::warn!(
                        line_number = idx + 1,
                        error = %err,
                        "skipping malformed HADES log line"
                    );
                    continue;
                }
            };
            if let Some(filter) = event_filter {
                if item.event != filter {
                    continue;
                }
            }
            if let Some(rule) = sigil_rule {
                if !action_record_matches_rule(&item, rule)? {
                    continue;
                }
            }
            out.push(item);
        }
        out.reverse();
        out.truncate(limit.max(1));
        Ok(out)
    }

    pub fn sigil_match(
        &self,
        path: &str,
        rule: &SigilVacuumRule,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let mut out = Vec::new();
        for value in read_recent_jsonl(Path::new(path), limit.max(1))? {
            if json_value_matches_rule(&value, rule)? {
                out.push(value);
            }
        }
        Ok(out)
    }

    pub fn queue_remove(&self, file: &str, authorized_by: &str) -> Result<TaskItem> {
        self.queue_remove_with_proof(file, authorized_by, None)
    }

    pub fn queue_remove_with_proof(
        &self,
        file: &str,
        authorized_by: &str,
        quorum_proof: Option<QuorumProof>,
    ) -> Result<TaskItem> {
        let quorum =
            self.evaluate_destructive_quorum(file, authorized_by, quorum_proof.as_ref())?;
        if !quorum.allowed {
            self.log_event(
                "destructive_quorum_denied",
                Some(file),
                serde_json::json!({
                        "authorized_by": authorized_by,
                        "required_approvers": quorum.required_approvers,
                        "triad_approvers": quorum.triad_approvers,
                    "approved_count": quorum.approved_count,
                    "approved_by": quorum.approved_by,
                    "has_evidence": quorum.has_evidence,
                    "reason": quorum.reason,
                    "love_equation_score": quorum.love_equation_score
                }),
            )?;
            let _ = self.notify_warden("destructive_quorum_denied", Path::new(file));
            return Err(ArdaError::Agent {
                agent: "hades".to_owned(),
                message: format!("destructive quorum denied for remove: {}", quorum.reason),
            });
        }
        let task = TaskItem {
            task_id: format!("hds_{}", uuid::Uuid::new_v4().simple()),
            queued_at_utc: Utc::now().to_rfc3339(),
            action: ActionKind::Remove,
            file: file.to_owned(),
            authorized_by: Some(authorized_by.to_owned()),
            reason: "manual remove command".to_owned(),
            execute_after_utc: Some((Utc::now() + Duration::hours(1)).to_rfc3339()),
            quorum_proof,
        };
        append_jsonl(&self.queue_path, &task)?;
        self.log_event(
            "coin_detected",
            Some(file),
            serde_json::json!({
                "authorized_by": authorized_by,
                "safety_check": "bacon_lite",
                "quorum_check": "2_of_3",
                "love_equation_score": quorum.love_equation_score
            }),
        )?;
        let mut bacon_task = Task::new(format!("remove {}", file), "remove");
        bacon_task.clarifications_resolved = 1;
        if let Err(err) = record_bacon_lite(
            "hades",
            "queue_remove",
            &bacon_task,
            serde_json::json!({
                "file": file,
                "authorized_by": authorized_by,
                "quorum_approvers": quorum.approved_by,
                "quorum_approved_count": quorum.approved_count,
                "love_equation_score": quorum.love_equation_score,
            }),
        ) {
            tracing::debug!(error = %err, "HADES bacon-lite queue_remove record failed");
        }
        Ok(task)
    }

    pub fn sweep(&self, sweep_type: &str, path: Option<&str>) -> Result<SweepResult> {
        let Some(result) = try_run_bounded("hades_sweep", hades_sweep_limit(), || {
            let started = Utc::now();
            let mut known_orphans = self.existing_orphan_files()?;
            self.log_event(
                "sweep_start",
                None,
                serde_json::json!({
                    "sweep_type": sweep_type,
                    "watch_paths": self.watch_paths(path),
                }),
            )?;

            let mut files_scanned = 0usize;
            let mut actions_taken = 0usize;
            let mut orphans_found = 0usize;
            let mut held_for_review = 0usize;

            for watch_path in self.watch_paths(path) {
                for entry in WalkDir::new(&watch_path).into_iter().filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.is_dir() {
                        continue;
                    }
                    if should_skip_watch_file(p) {
                        continue;
                    }
                    files_scanned += 1;
                    let path_display = p.display().to_string();
                    let sigil = read_sigil(p).unwrap_or(SigilState::Unknown);
                    match sigil {
                        SigilState::Unknown => {
                            orphans_found += 1;
                            let _ = self.write_sigil_transition(
                                p,
                                SigilState::Unknown,
                                SigilState::Repair,
                                "auto_orphan_repair_mark",
                            );
                            if self.handle_orphan(p, &mut known_orphans)? {
                                actions_taken += 1;
                            }
                        }
                        SigilState::Coin => {
                            let (safe, safety) = self.multi_gate_safety_check(p);
                            if safe {
                                match self
                                    .queue_remove(&path_display, "sigil_authorized_orchestrator")
                                {
                                    Ok(_) => {
                                        actions_taken += 1;
                                        let _ = self.write_sigil_transition(
                                            p,
                                            SigilState::Coin,
                                            SigilState::Condemned,
                                            "coin_remove_queued",
                                        );
                                        self.log_event(
                                            "safety_check_pass",
                                            Some(&path_display),
                                            safety,
                                        )?;
                                    }
                                    Err(err) => {
                                        held_for_review += 1;
                                        self.log_event(
                                            "safety_check_fail",
                                            Some(&path_display),
                                            serde_json::json!({
                                                "reason": "destructive_quorum_denied",
                                                "error": err.to_string()
                                            }),
                                        )?;
                                    }
                                }
                            } else {
                                held_for_review += 1;
                                self.log_event("safety_check_fail", Some(&path_display), safety)?;
                            }
                        }
                        SigilState::Repair => {
                            actions_taken += 1;
                            self.log_event(
                                "repair_detected",
                                Some(&path_display),
                                serde_json::json!({
                                    "athena_task_queued": true
                                }),
                            )?;
                            let athena = self.handoff_athena("repair_detected", p);
                            self.log_event("athena_handoff", Some(&path_display), athena)?;
                            let warden = self.notify_warden("repair_detected", p)?;
                            self.log_event("warden_handoff", Some(&path_display), warden)?;
                        }
                        _ => {
                            self.log_event(
                                "sigil_read",
                                Some(&path_display),
                                serde_json::json!({
                                    "sigil": format!("{sigil:?}"),
                                    "action": "none"
                                }),
                            )?;
                        }
                    }
                }
            }

            actions_taken += self.process_pending_removals()?;

            let completed = Utc::now();
            let result = SweepResult {
                sweep_type: sweep_type.to_owned(),
                started_at_utc: started.to_rfc3339(),
                completed_at_utc: completed.to_rfc3339(),
                files_scanned,
                actions_taken,
                orphans_found,
                held_for_review,
            };
            self.log_event(
                "sweep_complete",
                None,
                serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({})),
            )?;
            self.record_joulework("sweep", &result)?;
            self.emit_work_signal_background(
                "hades",
                self.estimated_sweep_work_amount(&result),
                JouleWorkUnit::Storage,
                Some(format!("sweep:{sweep_type}")),
            );
            let mut bacon_task = Task::new(
                format!("sweep {} scanned {}", sweep_type, files_scanned),
                "sweep",
            );
            bacon_task.clarifications_resolved = if files_scanned > 0 { 1 } else { 0 };
            if let Err(err) = record_bacon_lite(
                "hades",
                "sweep",
                &bacon_task,
                serde_json::json!({
                    "sweep_type": sweep_type,
                    "files_scanned": files_scanned,
                    "actions_taken": actions_taken,
                    "orphans_found": orphans_found,
                }),
            ) {
                tracing::debug!(error = %err, "HADES bacon-lite sweep record failed");
            }
            self.save_state(HadesState {
                last_sweep_utc: Some(completed.to_rfc3339()),
            })?;
            Ok(result)
        }) else {
            return Err(ArdaError::Agent {
                agent: "hades".to_owned(),
                message: "sweep concurrency gate saturated".to_owned(),
            });
        };

        result
    }

    pub fn paths(&self) -> serde_json::Value {
        serde_json::json!({
            "root": self.root,
            "log": self.log_path,
            "joulework": self.joulework_path,
            "queue": self.queue_path,
            "warden_queue": self.warden_queue_path,
            "athena_handoff_queue": self.athena_handoff_queue_path,
            "archive": self.archive_root,
        })
    }

    pub fn recent_joulework(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        read_recent_jsonl(&self.joulework_path, limit)
    }

    pub fn recent_warden_queue(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        read_recent_jsonl(&self.warden_queue_path, limit)
    }

    pub fn recent_athena_handoffs(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        read_recent_jsonl(&self.athena_handoff_queue_path, limit)
    }
}
