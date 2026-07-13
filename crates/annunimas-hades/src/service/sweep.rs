use super::{
    append_jsonl, default_watch_paths, hades_removal_step_limit, read_sigil,
    should_skip_watch_file, sigil_label, HadesService,
};
use crate::types::{ActionKind, SigilState, TaskItem};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::try_run_bounded;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

impl HadesService {
    pub(super) fn watch_paths(&self, single: Option<&str>) -> Vec<PathBuf> {
        if let Some(path) = single {
            return vec![PathBuf::from(path)];
        }
        let mut out = default_watch_paths();
        out.retain(|path| path.exists());
        out
    }

    pub(super) fn write_sigil_transition(
        &self,
        file: &Path,
        from: SigilState,
        to: SigilState,
        reason: &str,
    ) -> Result<()> {
        let content = match fs::read_to_string(file) {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };
        let to_label = sigil_label(to);
        let file_display = file.display().to_string();

        if let Some(first) = content.lines().next() {
            if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(first) {
                value["sigil"] = serde_json::json!(to_label);
                let updated_first = serde_json::to_string(&value)?;
                let rest = content.lines().skip(1).collect::<Vec<_>>().join("\n");
                let rewritten = if rest.is_empty() {
                    updated_first
                } else {
                    format!("{updated_first}\n{rest}")
                };
                fs::write(file, rewritten)?;
                self.log_event(
                    "sigil_transition",
                    Some(&file_display),
                    serde_json::json!({
                        "from": format!("{from:?}"),
                        "to": to_label,
                        "reason": reason,
                        "mode": "json_first_line"
                    }),
                )?;
                return Ok(());
            }
        }

        if content.contains("sigil:") {
            let mut changed = false;
            let mut rewritten = Vec::new();
            for line in content.lines() {
                let trimmed = line.trim_start();
                if trimmed.to_ascii_lowercase().starts_with("sigil:") {
                    let indent = line.len() - trimmed.len();
                    rewritten.push(format!("{}sigil: {}", " ".repeat(indent), to_label));
                    changed = true;
                } else {
                    rewritten.push(line.to_owned());
                }
            }
            if changed {
                fs::write(file, rewritten.join("\n"))?;
                self.log_event(
                    "sigil_transition",
                    Some(&file_display),
                    serde_json::json!({
                        "from": format!("{from:?}"),
                        "to": to_label,
                        "reason": reason,
                        "mode": "frontmatter_or_kv"
                    }),
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn handle_orphan(
        &self,
        file: &Path,
        known_orphans: &mut HashSet<String>,
    ) -> Result<bool> {
        let path_str = file.display().to_string();
        if known_orphans.contains(&path_str) {
            self.log_event(
                "orphan_existing",
                Some(&path_str),
                serde_json::json!({
                    "action": "dedup_skip"
                }),
            )?;
            return Ok(false);
        }
        let task = TaskItem {
            task_id: format!("hds_{}", uuid::Uuid::new_v4().simple()),
            queued_at_utc: Utc::now().to_rfc3339(),
            action: ActionKind::InvestigateOrphan,
            file: path_str.clone(),
            authorized_by: None,
            reason: "missing sigil header".to_owned(),
            execute_after_utc: None,
            quorum_proof: None,
        };
        append_jsonl(&self.queue_path, &task)?;
        self.log_event(
            "orphan_found",
            Some(&path_str),
            serde_json::json!({
                "action": "orphan_temp_written",
                "athena_task_queued": true
            }),
        )?;
        let athena = self.handoff_athena("orphan_found", file);
        self.log_event("athena_handoff", Some(&path_str), athena)?;
        let warden = self.notify_warden("orphan_found", file)?;
        self.log_event("warden_handoff", Some(&path_str), warden)?;
        self.emit_memory_event(
            "orphan_found",
            "HADES discovered orphan file",
            Some(0.75),
            vec!["hades".to_owned(), "orphan".to_owned()],
        );
        known_orphans.insert(path_str);
        Ok(true)
    }

    pub(super) fn process_pending_removals(&self) -> Result<usize> {
        let Some(result) = try_run_bounded(
            "hades_removal_step",
            hades_removal_step_limit(),
            || {
                let tasks = self.read_all_queue()?;
                let mut keep = Vec::new();
                let mut removed_count = 0usize;

                for task in tasks.into_iter().rev() {
                    let is_remove = matches!(task.action, ActionKind::Remove);
                    if !is_remove {
                        if matches!(task.action, ActionKind::InvestigateOrphan) {
                            let file_path = PathBuf::from(&task.file);
                            if file_path.exists() {
                                let sigil = read_sigil(&file_path).unwrap_or(SigilState::Unknown);
                                if !matches!(sigil, SigilState::Unknown) {
                                    self.log_event(
                                        "orphan_task_dropped",
                                        Some(&task.file),
                                        serde_json::json!({
                                            "reason":"sigil_present",
                                            "sigil": format!("{sigil:?}")
                                        }),
                                    )?;
                                    continue;
                                }
                            }
                        }
                        if matches!(task.action, ActionKind::InvestigateOrphan)
                            && should_skip_watch_file(Path::new(&task.file))
                        {
                            self.log_event(
                                "orphan_task_dropped",
                                Some(&task.file),
                                serde_json::json!({"reason":"skip_rule_match"}),
                            )?;
                            continue;
                        }
                        if matches!(task.action, ActionKind::InvestigateOrphan)
                            && !PathBuf::from(&task.file).exists()
                        {
                            self.log_event(
                                "orphan_task_dropped",
                                Some(&task.file),
                                serde_json::json!({"reason":"file no longer exists"}),
                            )?;
                            continue;
                        }
                        keep.push(task);
                        continue;
                    }
                    let due = task
                        .execute_after_utc
                        .as_deref()
                        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                        .map(|value| value.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);
                    if due > Utc::now() {
                        keep.push(task);
                        continue;
                    }
                    let precheck_file = PathBuf::from(&task.file);
                    if !precheck_file.exists() {
                        self.log_event(
                            "file_removed",
                            Some(&task.file),
                            serde_json::json!({"note":"already absent"}),
                        )?;
                        removed_count += 1;
                        continue;
                    }
                    let quorum = self.evaluate_destructive_quorum(
                        &task.file,
                        task.authorized_by.as_deref().unwrap_or("unknown"),
                        task.quorum_proof.as_ref(),
                    )?;
                    if !quorum.allowed {
                        self.log_event(
                            "destructive_quorum_blocked_execution",
                            Some(&task.file),
                            serde_json::json!({
                                "task_id": task.task_id,
                                "authorized_by": task.authorized_by,
                                "required_approvers": quorum.required_approvers,
                                "approved_count": quorum.approved_count,
                                "approved_by": quorum.approved_by,
                                "reason": quorum.reason,
                                "love_equation_score": quorum.love_equation_score
                            }),
                        )?;
                        let _ = self.notify_warden(
                            "destructive_quorum_blocked_execution",
                            Path::new(&task.file),
                        );
                        keep.push(task);
                        continue;
                    }
                    let file = PathBuf::from(&task.file);
                    if !file.exists() {
                        self.log_event(
                            "file_removed",
                            Some(&task.file),
                            serde_json::json!({"note":"already absent"}),
                        )?;
                        removed_count += 1;
                        continue;
                    }
                    let lifecycle = self.lifecycle_decision_for(&file);
                    if !lifecycle.consistency_ok {
                        self.log_event(
                            "soterion_consistency_hold",
                            Some(&task.file),
                            serde_json::json!({
                                "memory_scope": lifecycle.memory_scope,
                                "recommended_sigil_retention": lifecycle.recommended_sigil_retention,
                                "issues": lifecycle.consistency_issues,
                                "rationale": lifecycle.rationale
                            }),
                        )?;
                        let _ = self.notify_warden("destructive_quorum_blocked_execution", &file);
                        keep.push(task);
                        continue;
                    }
                    if self.memory_referenced(&file) {
                        self.log_event(
                            "memory_hold",
                            Some(&task.file),
                            serde_json::json!({
                                "reason":"memory system reference present",
                                "memory_scope": lifecycle.memory_scope,
                                "recommended_sigil_retention": lifecycle.recommended_sigil_retention
                            }),
                        )?;
                        keep.push(task);
                        continue;
                    }
                    if matches!(
                        lifecycle.disposition,
                        super::lifecycle_policy::LifecycleDisposition::Archive
                    ) {
                        let _ = self.write_sigil_transition(
                            &file,
                            SigilState::Condemned,
                            SigilState::Condemned,
                            "final_archive",
                        );
                        self.archive_file(&file)?;
                        self.log_event(
                            "file_archived",
                            Some(&task.file),
                            serde_json::json!({
                                "memory_scope": lifecycle.memory_scope,
                                "recommended_sigil_retention": lifecycle.recommended_sigil_retention,
                                "rationale": lifecycle.rationale
                            }),
                        )?;
                    } else {
                        let _ = self.write_sigil_transition(
                            &file,
                            SigilState::Condemned,
                            SigilState::Condemned,
                            "final_remove",
                        );
                        fs::remove_file(&file)?;
                        self.log_event(
                            "file_removed",
                            Some(&task.file),
                            serde_json::json!({
                                "final_sigil": "CONDEMNED",
                                "memory_refs": 0,
                                "memory_scope": lifecycle.memory_scope,
                                "recommended_sigil_retention": lifecycle.recommended_sigil_retention,
                                "rationale": lifecycle.rationale
                            }),
                        )?;
                    }
                    let warden = self.notify_warden("file_removed", &file)?;
                    self.log_event("warden_handoff", Some(&task.file), warden)?;
                    self.emit_memory_event(
                        "file_removed",
                        "HADES removed or archived a file",
                        Some(0.8),
                        vec!["hades".to_owned(), "cleanup".to_owned()],
                    );
                    removed_count += 1;
                }

                std::fs::write(&self.queue_path, "")?;
                for task in keep {
                    append_jsonl(&self.queue_path, &task)?;
                }
                Ok(removed_count)
            },
        ) else {
            return Err(AnnunimasError::Agent {
                agent: "hades".to_owned(),
                message: "removal-step concurrency gate saturated".to_owned(),
            });
        };

        result
    }
}
