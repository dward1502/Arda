use super::{append_jsonl, lifecycle_policy::LifecycleDisposition, HadesService};
use crate::types::{
    LifecycleAuditFinding, LifecycleAuditReport, WardenHadesEvidenceArtifact,
    WardenHadesEvidenceArtifactRecord, WardenHadesQueueOutcome, WardenHadesQueueStats,
    WardenHadesReviewItem, WardenHadesReviewPacket,
};
use annunimas_core::error::{AnnunimasError, Result};
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const LIFECYCLE_AUDIT_CONTRACT: &str = "annunimas.hades.lifecycle_audit_report.v1";
const LIFECYCLE_FINDING_CONTRACT: &str = "annunimas.hades.lifecycle_finding.v1";
const LIFECYCLE_REVIEW_QUEUE_CONTRACT: &str =
    "annunimas.hades.lifecycle_review_queue_projection.v1";
const LIFECYCLE_APPROVAL_PACKET_CONTRACT: &str =
    "annunimas.hades.lifecycle_operator_approval_packet.v1";
const LIFECYCLE_CLEANUP_CONTRACT: &str = "annunimas.hades.lifecycle_cleanup_executor.v1";
const LIFECYCLE_POLICY_AUTOMATION_CONTRACT: &str =
    "annunimas.hades.lifecycle_policy_automation_report.v1";
const WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT: &str =
    "annunimas.hades.warden_operator_review_packet.v1";
const WARDEN_OPERATOR_SIGNED_APPROVAL_PACKET_CONTRACT: &str =
    "annunimas.hades.warden_operator_signed_approval_packet.v1";
const WARDEN_OPERATOR_DRY_RUN_RECEIPT_CONTRACT: &str =
    "annunimas.hades.warden_operator_dry_run_receipt.v1";
const WARDEN_OPERATOR_SIGNED_MUTATION_APPROVAL_PACKET_CONTRACT: &str =
    "annunimas.hades.warden_operator_signed_mutation_approval_packet.v1";
const WARDEN_OPERATOR_MUTATION_PLAN_RECEIPT_CONTRACT: &str =
    "annunimas.hades.warden_operator_mutation_plan_receipt.v1";
const WARDEN_OPERATOR_FINAL_APPLY_APPROVAL_PACKET_CONTRACT: &str =
    "annunimas.hades.warden_operator_final_apply_approval_packet.v1";
const WARDEN_OPERATOR_FINAL_APPLY_EXECUTION_RECEIPT_CONTRACT: &str =
    "annunimas.hades.warden_operator_final_apply_execution_receipt.v1";

impl HadesService {
    pub fn audit_lifecycle_review(
        &self,
        root_path: impl AsRef<Path>,
        limit: usize,
    ) -> Result<LifecycleAuditReport> {
        let root_path = root_path.as_ref();
        let bounded_limit = limit.max(1);
        let mut findings = Vec::new();
        let mut scanned_files_total = 0usize;

        for scan_root in lifecycle_scan_roots(root_path) {
            if !scan_root.exists() {
                continue;
            }
            for entry in WalkDir::new(&scan_root)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                let path = entry.path();
                if path.is_dir() || should_skip_lifecycle_path(path) {
                    continue;
                }
                scanned_files_total += 1;
                if findings.len() >= bounded_limit {
                    break;
                }

                let relative = display_relative(root_path, path);
                if let Some(finding) = classify_lifecycle_path(root_path, path, &relative) {
                    findings.push(finding);
                }
            }
            if findings.len() >= bounded_limit {
                break;
            }
        }

        let task_queue = root_path.join("core/projects/tasks/queue.jsonl");
        if findings.len() < bounded_limit && task_queue.exists() {
            let remaining = bounded_limit.saturating_sub(findings.len());
            findings.extend(task_queue_hygiene_findings(
                root_path,
                &task_queue,
                remaining,
            )?);
        }

        let stale_plan_total = findings
            .iter()
            .filter(|finding| finding.finding_type == "stale_plan")
            .count();
        let archive_candidate_total = findings
            .iter()
            .filter(|finding| finding.finding_type == "archive_candidate")
            .count();
        let task_queue_hygiene_total = findings
            .iter()
            .filter(|finding| finding.finding_type == "task_queue_hygiene")
            .count();

        for finding in &findings {
            append_jsonl(&self.lifecycle_findings_path(), &finding_record(finding))?;
        }

        let report = LifecycleAuditReport {
            contract: LIFECYCLE_AUDIT_CONTRACT.to_owned(),
            generated_at_utc: Utc::now().to_rfc3339(),
            root_path: root_path.display().to_string(),
            findings_total: findings.len(),
            stale_plan_total,
            archive_candidate_total,
            task_queue_hygiene_total,
            scanned_files_total,
            no_delete: true,
            findings,
        };

        self.log_event(
            "lifecycle_audit_report_generated",
            Some(&root_path.display().to_string()),
            serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({})),
        )?;
        Ok(report)
    }

    pub fn project_lifecycle_review_queue(
        &self,
        root_path: impl AsRef<Path>,
        limit: usize,
    ) -> Result<Value> {
        let report = self.audit_lifecycle_review(root_path, limit)?;
        let queue_path = self.lifecycle_review_queue_path();
        let mut queued_total = 0usize;
        for finding in &report.findings {
            let record = serde_json::json!({
                "contract": LIFECYCLE_REVIEW_QUEUE_CONTRACT,
                "queued_at_utc": Utc::now().to_rfc3339(),
                "review_id": format!("hlq_{}", finding.finding_id),
                "finding_id": finding.finding_id,
                "path": finding.path,
                "classification": finding.lifecycle_class,
                "severity": finding.severity,
                "recommendation": finding.recommendation,
                "allowed_actions": allowed_actions_for(&finding.lifecycle_class),
                "evidence_path": finding.evidence.get("evidence_path").cloned().unwrap_or(Value::Null),
                "evidence": finding.evidence,
                "review_required": true,
                "destructive_allowed": false
            });
            append_jsonl(&queue_path, &record)?;
            queued_total += 1;
        }
        let out = serde_json::json!({
            "contract": LIFECYCLE_REVIEW_QUEUE_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "queue_path": queue_path.display().to_string(),
            "source_findings_total": report.findings_total,
            "queued_total": queued_total,
            "no_delete": true
        });
        self.log_event(
            "lifecycle_review_queue_projected",
            Some(&queue_path.display().to_string()),
            out.clone(),
        )?;
        Ok(out)
    }

    pub fn lifecycle_policy_automation_report(
        &self,
        root_path: impl AsRef<Path>,
        limit: usize,
    ) -> Result<Value> {
        let root_path = root_path.as_ref();
        let source_report = self.audit_lifecycle_review(root_path, limit)?;
        let mut by_memory_scope: HashMap<String, usize> = HashMap::new();
        let mut by_disposition: HashMap<String, usize> = HashMap::new();
        let mut consistency_holds_total = 0usize;
        let mut policy_items = Vec::new();

        for finding in &source_report.findings {
            let absolute_path = root_path.join(&finding.path);
            let decision = self.lifecycle_decision_for(&absolute_path);
            let disposition = disposition_key(&decision.disposition);
            *by_memory_scope
                .entry(decision.memory_scope.clone())
                .or_insert(0) += 1;
            *by_disposition.entry(disposition.to_owned()).or_insert(0) += 1;
            if !decision.consistency_ok
                || matches!(decision.disposition, LifecycleDisposition::Hold)
            {
                consistency_holds_total += 1;
            }

            policy_items.push(serde_json::json!({
                "finding_id": finding.finding_id,
                "path": finding.path,
                "finding_type": finding.finding_type,
                "lifecycle_class": finding.lifecycle_class,
                "severity": finding.severity,
                "recommendation": finding.recommendation,
                "policy_decision": decision,
                "policy_disposition": disposition,
                "allowed_actions": policy_allowed_actions(disposition),
                "review_required": true,
                "destructive_allowed": false,
                "mutation_requires_operator_approval": true,
                "evidence": finding.evidence,
            }));
        }

        let out_path = self.lifecycle_policy_automation_report_path();
        let out = serde_json::json!({
            "contract": LIFECYCLE_POLICY_AUTOMATION_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "root_path": root_path.display().to_string(),
            "source_report_contract": source_report.contract,
            "source_findings_total": source_report.findings_total,
            "report_path": out_path.display().to_string(),
            "policy_summary": {
                "findings_total": policy_items.len(),
                "by_memory_scope": by_memory_scope,
                "by_disposition": by_disposition,
                "consistency_holds_total": consistency_holds_total,
            },
            "policy_items": policy_items,
            "review_queue_projection_recommended": true,
            "cleanup_authorized": false,
            "requires_operator_approval_for_mutation": true,
            "no_delete": true,
            "no_file_moves_or_deletes_performed": true,
        });
        fs::write(&out_path, serde_json::to_vec_pretty(&out)?)?;
        self.log_event(
            "lifecycle_policy_automation_report_generated",
            Some(&out_path.display().to_string()),
            out.clone(),
        )?;
        Ok(out)
    }

    pub fn lifecycle_operator_approval_packet(
        &self,
        root_path: impl AsRef<Path>,
        limit: usize,
        out_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let report = self.audit_lifecycle_review(root_path, limit)?;
        let out_path = out_path.as_ref();
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let candidates: Vec<Value> = report.findings.iter().map(|finding| {
            serde_json::json!({
                "finding_id": finding.finding_id,
                "path": finding.path,
                "classification": finding.lifecycle_class,
                "recommendation": finding.recommendation,
                "reason": finding.evidence.get("reason").cloned().unwrap_or(Value::Null),
                "evidence_path": finding.evidence.get("evidence_path").cloned().unwrap_or(Value::Null),
                "operator_decision_required": matches!(finding.lifecycle_class.as_str(), "archive_candidate" | "quarantine_candidate" | "generated_delete_candidate"),
                "destructive_allowed_before_approval": false,
                "rollback_required_for_cleanup": true
            })
        }).collect();
        let packet = serde_json::json!({
            "contract": LIFECYCLE_APPROVAL_PACKET_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "approval_status": "pending_operator_review",
            "required_before_cleanup": ["human_or_triad_approval", "dry_run_receipt", "rollback_evidence"],
            "source_report_contract": report.contract,
            "candidates_total": candidates.len(),
            "candidates": candidates,
            "no_file_moves_or_deletes_performed": true
        });
        fs::write(out_path, serde_json::to_vec_pretty(&packet)?)?;
        self.log_event(
            "lifecycle_operator_approval_packet_generated",
            Some(&out_path.display().to_string()),
            packet.clone(),
        )?;
        Ok(packet)
    }

    pub fn execute_lifecycle_cleanup_plan(
        &self,
        approval_packet: impl AsRef<Path>,
        apply: bool,
        rollback_out: impl AsRef<Path>,
    ) -> Result<Value> {
        let approval_packet = approval_packet.as_ref();
        let rollback_out = rollback_out.as_ref();
        let packet: Value = serde_json::from_str(&fs::read_to_string(approval_packet)?)?;
        let approved = packet
            .get("approval_status")
            .and_then(Value::as_str)
            .map(|status| status == "approved")
            .unwrap_or(false);
        let dry_run_required = !apply;
        let mut planned_actions = Vec::new();
        if let Some(candidates) = packet.get("candidates").and_then(Value::as_array) {
            for candidate in candidates {
                let classification = candidate
                    .get("classification")
                    .and_then(Value::as_str)
                    .unwrap_or("review");
                if !matches!(
                    classification,
                    "archive_candidate" | "quarantine_candidate" | "generated_delete_candidate"
                ) {
                    continue;
                }
                let path = candidate.get("path").and_then(Value::as_str).unwrap_or("");
                let action = match classification {
                    "archive_candidate" => "archive",
                    "quarantine_candidate" => "quarantine",
                    "generated_delete_candidate" => "delete_generated",
                    _ => "review",
                };
                planned_actions.push(serde_json::json!({
                    "path": path,
                    "classification": classification,
                    "planned_action": action,
                    "would_execute": apply && approved,
                    "rollback_evidence": {
                        "pre_exists": Path::new(path).exists(),
                        "pre_path": path,
                        "rollback_strategy": "restore_from_archive_or_vcs_before_final_delete"
                    }
                }));
            }
        }
        if let Some(parent) = rollback_out.parent() {
            fs::create_dir_all(parent)?;
        }
        let execution = serde_json::json!({
            "contract": LIFECYCLE_CLEANUP_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "approval_packet": approval_packet.display().to_string(),
            "mode": if apply { "apply_requested" } else { "dry_run" },
            "approved": approved,
            "executed": false,
            "blocked_reason": if apply && !approved { "approval_packet_not_approved" } else if dry_run_required { "dry_run_first_no_files_moved_or_deleted" } else { "cleanup_executor_is_gated_audit_receipt_only_until_operator_approved_rollback_path" },
            "planned_actions_total": planned_actions.len(),
            "planned_actions": planned_actions,
            "rollback_evidence_path": rollback_out.display().to_string(),
            "no_file_moves_or_deletes_performed": true
        });
        fs::write(rollback_out, serde_json::to_vec_pretty(&execution)?)?;
        self.log_event(
            "lifecycle_cleanup_executor_receipt",
            Some(&rollback_out.display().to_string()),
            execution.clone(),
        )?;
        Ok(execution)
    }

    pub fn warden_hades_signed_approval_packet(
        &self,
        review_packet_path: impl AsRef<Path>,
        selected_review_ids: &[String],
        operator_id: &str,
        operator_decision: &str,
        approval_evidence: &str,
        out_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let review_packet_path = review_packet_path.as_ref();
        let out_path = out_path.as_ref();
        if selected_review_ids.is_empty() {
            return Err(AnnunimasError::Task(
                "signed approval packet requires at least one selected review_id".to_owned(),
            ));
        }
        if operator_id.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "signed approval packet requires a non-empty operator_id".to_owned(),
            ));
        }
        let review_packet: Value = serde_json::from_str(&fs::read_to_string(review_packet_path)?)?;
        if review_packet.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "source review packet must use contract {WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT}"
            )));
        }

        let review_items = review_packet
            .get("review_items")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AnnunimasError::Task("source review packet has no review_items array".to_owned())
            })?;
        let wanted: HashSet<&str> = selected_review_ids.iter().map(String::as_str).collect();
        let mut selected_items = Vec::new();
        for item in review_items {
            let review_id = item.get("review_id").and_then(Value::as_str).unwrap_or("");
            if wanted.contains(review_id) {
                selected_items.push(item.clone());
            }
        }
        if selected_items.len() != wanted.len() {
            let found: HashSet<&str> = selected_items
                .iter()
                .filter_map(|item| item.get("review_id").and_then(Value::as_str))
                .collect();
            let missing = selected_review_ids
                .iter()
                .filter(|review_id| !found.contains(review_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            return Err(AnnunimasError::Task(format!(
                "selected review_id values not found in source packet: {}",
                missing.join(",")
            )));
        }

        let source_packet_sha256 = sha256_file(review_packet_path)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let packet = serde_json::json!({
            "contract": WARDEN_OPERATOR_SIGNED_APPROVAL_PACKET_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "approval_status": "signed_operator_decision_recorded",
            "operator_id": operator_id,
            "operator_decision": operator_decision,
            "approval_evidence": approval_evidence,
            "source_review_packet_path": review_packet_path.display().to_string(),
            "source_review_packet_sha256": source_packet_sha256,
            "source_packet_contract": review_packet.get("contract").cloned().unwrap_or(Value::Null),
            "source_packet_is_authorization": review_packet.get("packet_is_authorization").cloned().unwrap_or(Value::Bool(false)),
            "selected_review_ids_total": selected_review_ids.len(),
            "selected_review_ids": selected_review_ids,
            "selected_items_total": selected_items.len(),
            "selected_items": selected_items,
            "approval_scope": "selected_review_ids_only",
            "cleanup_authorized": false,
            "authorizes_next_gate": "dry_run_receipt_only",
            "mutation_authorized_without_dry_run": false,
            "requires_rollback_evidence_before_apply": true,
            "raw_queue_retention_required": true,
            "no_file_moves_or_deletes_performed": true,
            "packet_path": out_path.display().to_string(),
        });
        fs::write(out_path, serde_json::to_vec_pretty(&packet)?)?;
        self.log_event(
            "warden_hades_operator_signed_approval_packet_generated",
            Some(&out_path.display().to_string()),
            packet.clone(),
        )?;
        Ok(packet)
    }

    pub fn warden_hades_dry_run_receipt(
        &self,
        approval_packet_path: impl AsRef<Path>,
        review_packet_path: impl AsRef<Path>,
        intended_action: &str,
        out_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let approval_packet_path = approval_packet_path.as_ref();
        let review_packet_path = review_packet_path.as_ref();
        let out_path = out_path.as_ref();
        if intended_action.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "dry-run receipt requires a non-empty intended action".to_owned(),
            ));
        }

        let approval_packet: Value =
            serde_json::from_str(&fs::read_to_string(approval_packet_path)?)?;
        if approval_packet.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_SIGNED_APPROVAL_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "approval packet must use contract {WARDEN_OPERATOR_SIGNED_APPROVAL_PACKET_CONTRACT}"
            )));
        }
        if approval_packet
            .get("authorizes_next_gate")
            .and_then(Value::as_str)
            != Some("dry_run_receipt_only")
        {
            return Err(AnnunimasError::Task(
                "approval packet does not authorize dry-run receipt gate".to_owned(),
            ));
        }

        let review_packet: Value = serde_json::from_str(&fs::read_to_string(review_packet_path)?)?;
        if review_packet.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "source review packet must use contract {WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT}"
            )));
        }

        let source_review_packet_sha256 = sha256_file(review_packet_path)?;
        if approval_packet
            .get("source_review_packet_sha256")
            .and_then(Value::as_str)
            != Some(source_review_packet_sha256.as_str())
        {
            return Err(AnnunimasError::Task(
                "approval packet source review hash does not match supplied review packet"
                    .to_owned(),
            ));
        }

        let selected_review_ids = approval_packet
            .get("selected_review_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "approval packet must include selected_review_ids array".to_owned(),
                )
            })?;
        if selected_review_ids.is_empty() {
            return Err(AnnunimasError::Task(
                "dry-run receipt requires selected review IDs".to_owned(),
            ));
        }
        let selected_review_ids = selected_review_ids
            .iter()
            .map(|id| {
                id.as_str().map(str::to_owned).ok_or_else(|| {
                    AnnunimasError::Task("selected_review_ids entries must be strings".to_owned())
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let selected_items = approval_packet
            .get("selected_items")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task("approval packet must include selected_items array".to_owned())
            })?;
        let raw_queue_path = review_packet
            .get("raw_queue_path")
            .and_then(Value::as_str)
            .or_else(|| {
                review_packet
                    .get("raw_queue")
                    .and_then(|raw_queue| raw_queue.get("path"))
                    .and_then(Value::as_str)
            });
        let expected_raw_queue_sha256 = review_packet
            .get("raw_queue_sha256")
            .and_then(Value::as_str)
            .or_else(|| {
                review_packet
                    .get("raw_queue")
                    .and_then(|raw_queue| raw_queue.get("sha256"))
                    .and_then(Value::as_str)
            });
        let raw_queue_sha256 = raw_queue_path
            .filter(|path| !path.trim().is_empty() && Path::new(path).exists())
            .map(|path| sha256_file(Path::new(path)))
            .transpose()?;
        let raw_queue_retention_verified = match (&raw_queue_sha256, expected_raw_queue_sha256) {
            (Some(actual), Some(expected)) => actual == expected,
            _ => true,
        };

        let review_queue_path = review_packet
            .get("review_queue_path")
            .and_then(Value::as_str);
        let review_queue_sha256 = review_queue_path
            .filter(|path| !path.trim().is_empty() && Path::new(path).exists())
            .map(|path| sha256_file(Path::new(path)))
            .transpose()?;

        let source_approval_packet_sha256 = sha256_file(approval_packet_path)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let receipt = serde_json::json!({
            "contract": WARDEN_OPERATOR_DRY_RUN_RECEIPT_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "receipt_status": "dry_run_receipt_generated",
            "source_approval_packet_path": approval_packet_path.display().to_string(),
            "source_approval_packet_sha256": source_approval_packet_sha256,
            "source_review_packet_path": review_packet_path.display().to_string(),
            "source_review_packet_sha256": source_review_packet_sha256,
            "selected_review_ids_total": selected_review_ids.len(),
            "selected_review_ids": selected_review_ids,
            "selected_items_total": selected_items.len(),
            "selected_items": selected_items,
            "intended_action": intended_action,
            "dry_run_only": true,
            "mutation_performed": false,
            "apply_authorized": false,
            "cleanup_authorized": false,
            "archive_authorized": false,
            "delete_authorized": false,
            "clear_authorized": false,
            "rollback_plan_required": true,
            "raw_queue_path": raw_queue_path,
            "raw_queue_sha256": raw_queue_sha256,
            "raw_queue_retention_verified": raw_queue_retention_verified,
            "review_queue_path": review_queue_path,
            "review_queue_sha256": review_queue_sha256,
            "review_queue_retention_verified": true,
            "next_gate_requires_mutation_specific_approval": true,
            "no_file_moves_or_deletes_performed": true,
            "receipt_path": out_path.display().to_string(),
        });
        fs::write(out_path, serde_json::to_vec_pretty(&receipt)?)?;
        self.log_event(
            "warden_hades_operator_dry_run_receipt_generated",
            Some(&out_path.display().to_string()),
            receipt.clone(),
        )?;
        Ok(receipt)
    }

    pub fn warden_hades_signed_mutation_approval_packet(
        &self,
        dry_run_receipt_path: impl AsRef<Path>,
        operator_id: &str,
        mutation_action: &str,
        approval_evidence: &str,
        out_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let dry_run_receipt_path = dry_run_receipt_path.as_ref();
        let out_path = out_path.as_ref();
        if operator_id.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "mutation approval packet requires a non-empty operator_id".to_owned(),
            ));
        }
        if mutation_action.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "mutation approval packet requires a non-empty mutation action".to_owned(),
            ));
        }
        let dry_run_receipt: Value =
            serde_json::from_str(&fs::read_to_string(dry_run_receipt_path)?)?;
        if dry_run_receipt.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_DRY_RUN_RECEIPT_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "dry-run receipt must use contract {WARDEN_OPERATOR_DRY_RUN_RECEIPT_CONTRACT}"
            )));
        }
        if dry_run_receipt
            .get("mutation_performed")
            .and_then(Value::as_bool)
            != Some(false)
        {
            return Err(AnnunimasError::Task(
                "mutation approval requires a non-mutating dry-run receipt".to_owned(),
            ));
        }
        if dry_run_receipt
            .get("next_gate_requires_mutation_specific_approval")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(AnnunimasError::Task(
                "dry-run receipt does not require mutation-specific approval".to_owned(),
            ));
        }
        let selected_review_ids = dry_run_receipt
            .get("selected_review_ids")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "dry-run receipt must include selected_review_ids array".to_owned(),
                )
            })?;
        if selected_review_ids.is_empty() {
            return Err(AnnunimasError::Task(
                "mutation approval requires selected review IDs".to_owned(),
            ));
        }
        let selected_items = dry_run_receipt
            .get("selected_items")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task("dry-run receipt must include selected_items array".to_owned())
            })?;
        let source_dry_run_receipt_sha256 = sha256_file(dry_run_receipt_path)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let packet = serde_json::json!({
            "contract": WARDEN_OPERATOR_SIGNED_MUTATION_APPROVAL_PACKET_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "approval_status": "signed_mutation_plan_decision_recorded",
            "operator_id": operator_id,
            "mutation_action": mutation_action,
            "approval_evidence": approval_evidence,
            "source_dry_run_receipt_path": dry_run_receipt_path.display().to_string(),
            "source_dry_run_receipt_sha256": source_dry_run_receipt_sha256,
            "source_review_packet_path": dry_run_receipt.get("source_review_packet_path").cloned().unwrap_or(Value::Null),
            "source_review_packet_sha256": dry_run_receipt.get("source_review_packet_sha256").cloned().unwrap_or(Value::Null),
            "source_approval_packet_path": dry_run_receipt.get("source_approval_packet_path").cloned().unwrap_or(Value::Null),
            "source_approval_packet_sha256": dry_run_receipt.get("source_approval_packet_sha256").cloned().unwrap_or(Value::Null),
            "selected_review_ids_total": selected_review_ids.len(),
            "selected_review_ids": selected_review_ids,
            "selected_items_total": selected_items.len(),
            "selected_items": selected_items,
            "approval_scope": "selected_review_ids_for_mutation_plan_only",
            "authorizes_next_gate": "mutation_plan_receipt_only",
            "apply_authorized": false,
            "mutation_authorized_without_final_apply_approval": false,
            "requires_final_apply_approval": true,
            "raw_queue_retention_required": true,
            "no_file_moves_or_deletes_performed": true,
            "packet_path": out_path.display().to_string(),
        });
        fs::write(out_path, serde_json::to_vec_pretty(&packet)?)?;
        self.log_event(
            "warden_hades_operator_signed_mutation_approval_packet_generated",
            Some(&out_path.display().to_string()),
            packet.clone(),
        )?;
        Ok(packet)
    }

    pub fn warden_hades_mutation_plan_receipt(
        &self,
        mutation_approval_packet_path: impl AsRef<Path>,
        review_packet_path: impl AsRef<Path>,
        dry_run_receipt_path: impl AsRef<Path>,
        mutation_action: &str,
        out_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let mutation_approval_packet_path = mutation_approval_packet_path.as_ref();
        let review_packet_path = review_packet_path.as_ref();
        let dry_run_receipt_path = dry_run_receipt_path.as_ref();
        let out_path = out_path.as_ref();
        if mutation_action.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "mutation plan receipt requires a non-empty mutation action".to_owned(),
            ));
        }
        let mutation_approval: Value =
            serde_json::from_str(&fs::read_to_string(mutation_approval_packet_path)?)?;
        if mutation_approval.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_SIGNED_MUTATION_APPROVAL_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "mutation approval packet must use contract {WARDEN_OPERATOR_SIGNED_MUTATION_APPROVAL_PACKET_CONTRACT}"
            )));
        }
        if mutation_approval
            .get("authorizes_next_gate")
            .and_then(Value::as_str)
            != Some("mutation_plan_receipt_only")
        {
            return Err(AnnunimasError::Task(
                "mutation approval packet does not authorize mutation plan receipt gate".to_owned(),
            ));
        }
        if mutation_approval
            .get("mutation_action")
            .and_then(Value::as_str)
            != Some(mutation_action)
        {
            return Err(AnnunimasError::Task(
                "mutation approval action does not match requested plan action".to_owned(),
            ));
        }
        let dry_run_receipt: Value =
            serde_json::from_str(&fs::read_to_string(dry_run_receipt_path)?)?;
        if dry_run_receipt.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_DRY_RUN_RECEIPT_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "dry-run receipt must use contract {WARDEN_OPERATOR_DRY_RUN_RECEIPT_CONTRACT}"
            )));
        }
        let source_dry_run_receipt_sha256 = sha256_file(dry_run_receipt_path)?;
        if mutation_approval
            .get("source_dry_run_receipt_sha256")
            .and_then(Value::as_str)
            != Some(source_dry_run_receipt_sha256.as_str())
        {
            return Err(AnnunimasError::Task(
                "mutation approval source dry-run hash does not match supplied dry-run receipt"
                    .to_owned(),
            ));
        }

        let review_packet: Value = serde_json::from_str(&fs::read_to_string(review_packet_path)?)?;
        if review_packet.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "source review packet must use contract {WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT}"
            )));
        }
        let source_review_packet_sha256 = sha256_file(review_packet_path)?;
        if mutation_approval
            .get("source_review_packet_sha256")
            .and_then(Value::as_str)
            != Some(source_review_packet_sha256.as_str())
        {
            return Err(AnnunimasError::Task(
                "mutation approval source review hash does not match supplied review packet"
                    .to_owned(),
            ));
        }

        let selected_review_ids = mutation_approval
            .get("selected_review_ids")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation approval packet must include selected_review_ids array".to_owned(),
                )
            })?;
        let selected_items = mutation_approval
            .get("selected_items")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation approval packet must include selected_items array".to_owned(),
                )
            })?;
        if selected_review_ids.is_empty() || selected_items.is_empty() {
            return Err(AnnunimasError::Task(
                "mutation plan receipt requires selected review IDs and items".to_owned(),
            ));
        }

        let raw_queue_path = review_packet
            .get("raw_queue_path")
            .and_then(Value::as_str)
            .or_else(|| {
                review_packet
                    .get("raw_queue")
                    .and_then(|raw_queue| raw_queue.get("path"))
                    .and_then(Value::as_str)
            });
        let expected_raw_queue_sha256 = review_packet
            .get("raw_queue_sha256")
            .and_then(Value::as_str)
            .or_else(|| {
                review_packet
                    .get("raw_queue")
                    .and_then(|raw_queue| raw_queue.get("sha256"))
                    .and_then(Value::as_str)
            });
        let raw_queue_sha256 = raw_queue_path
            .filter(|path| !path.trim().is_empty() && Path::new(path).exists())
            .map(|path| sha256_file(Path::new(path)))
            .transpose()?;
        let raw_queue_retention_verified = match (&raw_queue_sha256, expected_raw_queue_sha256) {
            (Some(actual), Some(expected)) => actual == expected,
            _ => true,
        };
        let review_queue_path = review_packet
            .get("review_queue_path")
            .and_then(Value::as_str);
        let review_queue_sha256 = review_queue_path
            .filter(|path| !path.trim().is_empty() && Path::new(path).exists())
            .map(|path| sha256_file(Path::new(path)))
            .transpose()?;
        let source_mutation_approval_packet_sha256 = sha256_file(mutation_approval_packet_path)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let receipt = serde_json::json!({
            "contract": WARDEN_OPERATOR_MUTATION_PLAN_RECEIPT_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "receipt_status": "mutation_plan_receipt_generated",
            "source_mutation_approval_packet_path": mutation_approval_packet_path.display().to_string(),
            "source_mutation_approval_packet_sha256": source_mutation_approval_packet_sha256,
            "source_dry_run_receipt_path": dry_run_receipt_path.display().to_string(),
            "source_dry_run_receipt_sha256": source_dry_run_receipt_sha256,
            "source_review_packet_path": review_packet_path.display().to_string(),
            "source_review_packet_sha256": source_review_packet_sha256,
            "planned_mutation_action": mutation_action,
            "selected_review_ids_total": selected_review_ids.len(),
            "selected_review_ids": selected_review_ids,
            "selected_items_total": selected_items.len(),
            "selected_items": selected_items,
            "mutation_performed": false,
            "apply_authorized": false,
            "cleanup_authorized": false,
            "archive_authorized": false,
            "delete_authorized": false,
            "clear_authorized": false,
            "requires_final_apply_approval": true,
            "rollback_plan_required": true,
            "raw_queue_path": raw_queue_path,
            "raw_queue_sha256": raw_queue_sha256,
            "raw_queue_retention_verified": raw_queue_retention_verified,
            "review_queue_path": review_queue_path,
            "review_queue_sha256": review_queue_sha256,
            "review_queue_retention_verified": true,
            "next_gate_requires_final_apply_approval": true,
            "no_file_moves_or_deletes_performed": true,
            "receipt_path": out_path.display().to_string(),
        });
        fs::write(out_path, serde_json::to_vec_pretty(&receipt)?)?;
        self.log_event(
            "warden_hades_operator_mutation_plan_receipt_generated",
            Some(&out_path.display().to_string()),
            receipt.clone(),
        )?;
        Ok(receipt)
    }

    pub fn warden_hades_final_apply_approval_packet(
        &self,
        mutation_plan_receipt_path: impl AsRef<Path>,
        operator_id: &str,
        mutation_action: &str,
        rollback_plan: &str,
        approval_evidence: &str,
        out_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let mutation_plan_receipt_path = mutation_plan_receipt_path.as_ref();
        let out_path = out_path.as_ref();
        if operator_id.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "final apply approval packet requires a non-empty operator_id".to_owned(),
            ));
        }
        if mutation_action.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "final apply approval packet requires a non-empty mutation action".to_owned(),
            ));
        }
        if rollback_plan.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "final apply approval packet requires rollback plan evidence".to_owned(),
            ));
        }
        if approval_evidence.trim().is_empty() {
            return Err(AnnunimasError::Task(
                "final apply approval packet requires approval evidence".to_owned(),
            ));
        }

        let mutation_plan: Value =
            serde_json::from_str(&fs::read_to_string(mutation_plan_receipt_path)?)?;
        if mutation_plan.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_MUTATION_PLAN_RECEIPT_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "mutation plan receipt must use contract {WARDEN_OPERATOR_MUTATION_PLAN_RECEIPT_CONTRACT}"
            )));
        }
        if mutation_plan
            .get("planned_mutation_action")
            .and_then(Value::as_str)
            != Some(mutation_action)
        {
            return Err(AnnunimasError::Task(
                "mutation plan action does not match requested final apply action".to_owned(),
            ));
        }
        if mutation_plan
            .get("next_gate_requires_final_apply_approval")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(AnnunimasError::Task(
                "mutation plan receipt does not require final apply approval".to_owned(),
            ));
        }
        if mutation_plan
            .get("mutation_performed")
            .and_then(Value::as_bool)
            != Some(false)
        {
            return Err(AnnunimasError::Task(
                "final apply approval requires a non-mutating mutation plan receipt".to_owned(),
            ));
        }
        if mutation_plan
            .get("rollback_plan_required")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(AnnunimasError::Task(
                "mutation plan receipt must require rollback plan".to_owned(),
            ));
        }

        let selected_review_ids = mutation_plan
            .get("selected_review_ids")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation plan receipt must include selected_review_ids array".to_owned(),
                )
            })?;
        let selected_items = mutation_plan
            .get("selected_items")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation plan receipt must include selected_items array".to_owned(),
                )
            })?;
        if selected_review_ids.is_empty() || selected_items.is_empty() {
            return Err(AnnunimasError::Task(
                "final apply approval requires selected review IDs and items".to_owned(),
            ));
        }

        let raw_queue_path = mutation_plan
            .get("raw_queue_path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation plan receipt must include raw_queue_path for fresh hash verification"
                        .to_owned(),
                )
            })?;
        let expected_raw_queue_sha256 = mutation_plan
            .get("raw_queue_sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation plan receipt must include raw_queue_sha256 for fresh hash verification"
                        .to_owned(),
                )
            })?;
        let actual_raw_queue_sha256 = sha256_file(Path::new(raw_queue_path))?;
        if actual_raw_queue_sha256 != expected_raw_queue_sha256 {
            return Err(AnnunimasError::Task(format!(
                "raw queue hash drift blocks final apply approval: expected {expected_raw_queue_sha256}, found {actual_raw_queue_sha256}"
            )));
        }

        let review_queue_path = mutation_plan
            .get("review_queue_path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation plan receipt must include review_queue_path for fresh hash verification"
                        .to_owned(),
                )
            })?;
        let expected_review_queue_sha256 = mutation_plan
            .get("review_queue_sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "mutation plan receipt must include review_queue_sha256 for fresh hash verification"
                        .to_owned(),
                )
            })?;
        let actual_review_queue_sha256 = sha256_file(Path::new(review_queue_path))?;
        if actual_review_queue_sha256 != expected_review_queue_sha256 {
            return Err(AnnunimasError::Task(format!(
                "review queue hash drift blocks final apply approval: expected {expected_review_queue_sha256}, found {actual_review_queue_sha256}"
            )));
        }

        let source_mutation_plan_receipt_sha256 = sha256_file(mutation_plan_receipt_path)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let archive_authorized = mutation_action == "archive_after_approval";
        let packet = serde_json::json!({
            "contract": WARDEN_OPERATOR_FINAL_APPLY_APPROVAL_PACKET_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "approval_status": "signed_final_apply_decision_recorded",
            "operator_id": operator_id,
            "approved_mutation_action": mutation_action,
            "approval_evidence": approval_evidence,
            "rollback_plan": rollback_plan,
            "rollback_plan_recorded": true,
            "source_mutation_plan_receipt_path": mutation_plan_receipt_path.display().to_string(),
            "source_mutation_plan_receipt_sha256": source_mutation_plan_receipt_sha256,
            "source_mutation_approval_packet_path": mutation_plan.get("source_mutation_approval_packet_path").cloned().unwrap_or(Value::Null),
            "source_mutation_approval_packet_sha256": mutation_plan.get("source_mutation_approval_packet_sha256").cloned().unwrap_or(Value::Null),
            "source_review_packet_path": mutation_plan.get("source_review_packet_path").cloned().unwrap_or(Value::Null),
            "source_review_packet_sha256": mutation_plan.get("source_review_packet_sha256").cloned().unwrap_or(Value::Null),
            "selected_review_ids_total": selected_review_ids.len(),
            "selected_review_ids": selected_review_ids,
            "selected_items_total": selected_items.len(),
            "selected_items": selected_items,
            "raw_queue_path": raw_queue_path,
            "raw_queue_sha256": actual_raw_queue_sha256,
            "review_queue_path": review_queue_path,
            "review_queue_sha256": actual_review_queue_sha256,
            "fresh_hash_verification_passed": true,
            "apply_authorized": true,
            "archive_authorized": archive_authorized,
            "cleanup_authorized": false,
            "delete_authorized": false,
            "clear_authorized": false,
            "mutation_performed": false,
            "authorizes_next_gate": "final_apply_execution",
            "final_apply_execution_must_reverify_hashes": true,
            "raw_queue_retention_required_until_apply": true,
            "review_queue_retention_required_until_apply": true,
            "no_file_moves_or_deletes_performed": true,
            "packet_path": out_path.display().to_string(),
        });
        fs::write(out_path, serde_json::to_vec_pretty(&packet)?)?;
        self.log_event(
            "warden_hades_operator_final_apply_approval_packet_generated",
            Some(&out_path.display().to_string()),
            packet.clone(),
        )?;
        Ok(packet)
    }

    pub fn warden_hades_final_apply_execution(
        &self,
        final_apply_approval_packet_path: impl AsRef<Path>,
        mutation_action: &str,
        archive_path: impl AsRef<Path>,
        execution_receipt_path: impl AsRef<Path>,
    ) -> Result<Value> {
        let final_apply_approval_packet_path = final_apply_approval_packet_path.as_ref();
        let archive_path = archive_path.as_ref();
        let execution_receipt_path = execution_receipt_path.as_ref();
        if mutation_action != "archive_after_approval" {
            return Err(AnnunimasError::Task(
                "final apply execution only supports archive_after_approval".to_owned(),
            ));
        }

        let approval: Value =
            serde_json::from_str(&fs::read_to_string(final_apply_approval_packet_path)?)?;
        if approval.get("contract").and_then(Value::as_str)
            != Some(WARDEN_OPERATOR_FINAL_APPLY_APPROVAL_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "final apply approval packet must use contract {WARDEN_OPERATOR_FINAL_APPLY_APPROVAL_PACKET_CONTRACT}"
            )));
        }
        if approval
            .get("approved_mutation_action")
            .and_then(Value::as_str)
            != Some(mutation_action)
        {
            return Err(AnnunimasError::Task(
                "final apply approval action does not match requested execution action".to_owned(),
            ));
        }
        if approval.get("apply_authorized").and_then(Value::as_bool) != Some(true)
            || approval.get("archive_authorized").and_then(Value::as_bool) != Some(true)
        {
            return Err(AnnunimasError::Task(
                "final apply execution requires explicit archive authorization".to_owned(),
            ));
        }
        if approval.get("delete_authorized").and_then(Value::as_bool) == Some(true)
            || approval.get("clear_authorized").and_then(Value::as_bool) == Some(true)
            || approval.get("cleanup_authorized").and_then(Value::as_bool) == Some(true)
        {
            return Err(AnnunimasError::Task(
                "final apply execution refuses packets authorizing clear/delete/cleanup".to_owned(),
            ));
        }
        if approval.get("mutation_performed").and_then(Value::as_bool) != Some(false) {
            return Err(AnnunimasError::Task(
                "final apply execution requires a non-executed approval packet".to_owned(),
            ));
        }
        if approval
            .get("final_apply_execution_must_reverify_hashes")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(AnnunimasError::Task(
                "final apply approval packet must require fresh execution hash verification"
                    .to_owned(),
            ));
        }

        let raw_queue_path = approval
            .get("raw_queue_path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "final apply approval packet must include raw_queue_path".to_owned(),
                )
            })?;
        let expected_raw_queue_sha256 = approval
            .get("raw_queue_sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "final apply approval packet must include raw_queue_sha256".to_owned(),
                )
            })?;
        let actual_raw_queue_sha256 = sha256_file(Path::new(raw_queue_path))?;
        if actual_raw_queue_sha256 != expected_raw_queue_sha256 {
            return Err(AnnunimasError::Task(format!(
                "raw queue hash drift blocks final apply execution: expected {expected_raw_queue_sha256}, found {actual_raw_queue_sha256}"
            )));
        }

        let review_queue_path = approval
            .get("review_queue_path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "final apply approval packet must include review_queue_path".to_owned(),
                )
            })?;
        let expected_review_queue_sha256 = approval
            .get("review_queue_sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "final apply approval packet must include review_queue_sha256".to_owned(),
                )
            })?;
        let actual_review_queue_sha256 = sha256_file(Path::new(review_queue_path))?;
        if actual_review_queue_sha256 != expected_review_queue_sha256 {
            return Err(AnnunimasError::Task(format!(
                "review queue hash drift blocks final apply execution: expected {expected_review_queue_sha256}, found {actual_review_queue_sha256}"
            )));
        }

        let selected_items = approval
            .get("selected_items")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AnnunimasError::Task(
                    "final apply approval packet must include selected_items array".to_owned(),
                )
            })?;
        if selected_items.is_empty() {
            return Err(AnnunimasError::Task(
                "final apply execution requires at least one selected item".to_owned(),
            ));
        }
        if archive_path.exists() {
            return Err(AnnunimasError::Task(format!(
                "archive path already exists; refusing to duplicate final apply evidence: {}",
                archive_path.display()
            )));
        }
        if execution_receipt_path.exists() {
            return Err(AnnunimasError::Task(format!(
                "execution receipt path already exists; refusing to overwrite: {}",
                execution_receipt_path.display()
            )));
        }
        if let Some(parent) = archive_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = execution_receipt_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let approval_sha256 = sha256_file(final_apply_approval_packet_path)?;
        let executed_at_utc = Utc::now().to_rfc3339();
        let mut archive_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(archive_path)?;
        for item in selected_items {
            let archived_item = serde_json::json!({
                "contract": "annunimas.hades.warden_operator_archived_review_record.v1",
                "archived_at_utc": executed_at_utc,
                "source_final_apply_approval_packet_path": final_apply_approval_packet_path.display().to_string(),
                "source_final_apply_approval_packet_sha256": approval_sha256,
                "executed_mutation_action": mutation_action,
                "review_item": item,
            });
            writeln!(archive_file, "{}", serde_json::to_string(&archived_item)?)?;
        }
        drop(archive_file);
        let archive_sha256 = sha256_file(archive_path)?;

        let receipt = serde_json::json!({
            "contract": WARDEN_OPERATOR_FINAL_APPLY_EXECUTION_RECEIPT_CONTRACT,
            "executed_at_utc": executed_at_utc,
            "execution_status": "archive_after_approval_executed",
            "source_final_apply_approval_packet_path": final_apply_approval_packet_path.display().to_string(),
            "source_final_apply_approval_packet_sha256": approval_sha256,
            "executed_mutation_action": mutation_action,
            "mutation_performed": true,
            "archive_performed": true,
            "clear_performed": false,
            "delete_performed": false,
            "cleanup_performed": false,
            "raw_queue_path": raw_queue_path,
            "raw_queue_sha256_before": actual_raw_queue_sha256,
            "raw_queue_sha256_after": sha256_file(Path::new(raw_queue_path))?,
            "raw_queue_retained": true,
            "review_queue_path": review_queue_path,
            "review_queue_sha256_before": actual_review_queue_sha256,
            "review_queue_sha256_after": sha256_file(Path::new(review_queue_path))?,
            "review_queue_retained": true,
            "archive_path": archive_path.display().to_string(),
            "archive_sha256": archive_sha256,
            "archive_record_count": selected_items.len(),
            "selected_review_ids": approval.get("selected_review_ids").cloned().unwrap_or(Value::Null),
            "selected_review_ids_total": approval.get("selected_review_ids_total").cloned().unwrap_or(Value::Null),
            "rollback_plan": approval.get("rollback_plan").cloned().unwrap_or(Value::Null),
            "rollback_evidence": "raw queue and review queue retained unchanged; archive artifact can be removed if execution receipt is rejected",
            "execution_receipt_path": execution_receipt_path.display().to_string(),
        });
        fs::write(execution_receipt_path, serde_json::to_vec_pretty(&receipt)?)?;
        self.log_event(
            "warden_hades_operator_final_apply_execution_completed",
            Some(&execution_receipt_path.display().to_string()),
            receipt.clone(),
        )?;
        Ok(receipt)
    }

    pub fn project_warden_hades_operator_review_packet(
        &self,
        root_path: impl AsRef<Path>,
        raw_queue_path: impl AsRef<Path>,
        limit: usize,
        out_dir: impl AsRef<Path>,
    ) -> Result<Value> {
        let root_path = root_path.as_ref();
        let raw_queue_path = raw_queue_path.as_ref();
        let out_dir = out_dir.as_ref();
        fs::create_dir_all(out_dir)?;

        let bounded_limit = limit.max(1);
        let raw_queue_stats = raw_queue_stats(raw_queue_path)?;
        let evidence_dir = out_dir.join("evidence");
        fs::create_dir_all(&evidence_dir)?;
        let mut review_items: Vec<WardenHadesReviewItem> = Vec::new();

        for (idx, line) in fs::read_to_string(raw_queue_path)?.lines().enumerate() {
            if review_items.len() >= bounded_limit {
                break;
            }
            if line.trim().is_empty() {
                continue;
            }
            let parsed = serde_json::from_str::<Value>(line).unwrap_or_else(|err| {
                serde_json::json!({
                    "parse_error": err.to_string(),
                    "raw_line": line,
                })
            });
            let item = warden_review_item(idx + 1, parsed, &evidence_dir)?;
            review_items.push(item);
        }

        let policy_report = self.lifecycle_policy_automation_report(root_path, bounded_limit)?;
        if let Some(items) = policy_report.get("policy_items").and_then(Value::as_array) {
            for item in items {
                if review_items.len() >= bounded_limit {
                    break;
                }
                review_items.push(policy_review_item(item.clone(), &evidence_dir)?);
            }
        }

        let review_queue_path = out_dir.join("warden_hades_operator_review_queue.jsonl");
        let mut queue_lines = String::new();
        for item in &review_items {
            queue_lines.push_str(&serde_json::to_string(item)?);
            queue_lines.push('\n');
        }
        fs::write(&review_queue_path, queue_lines)?;

        let markdown_summary_path = out_dir.join("WARDEN_HADES_OPERATOR_REVIEW_PACKET.md");
        let packet_path = out_dir.join("warden_hades_operator_review_packet.json");
        let packet = WardenHadesReviewPacket {
            contract: WARDEN_OPERATOR_REVIEW_PACKET_CONTRACT.to_owned(),
            generated_at_utc: Utc::now().to_rfc3339(),
            root_path: root_path.display().to_string(),
            packet_path: packet_path.display().to_string(),
            review_queue_path: review_queue_path.display().to_string(),
            evidence_dir: evidence_dir.display().to_string(),
            markdown_summary_path: markdown_summary_path.display().to_string(),
            raw_queue: raw_queue_stats,
            raw_queue_retained: true,
            policy_report_path: policy_report
                .get("report_path")
                .cloned()
                .unwrap_or(Value::Null),
            policy_report_contract: policy_report
                .get("contract")
                .cloned()
                .unwrap_or(Value::Null),
            review_items_total: review_items.len(),
            review_items,
            operator_decision_required: true,
            packet_is_authorization: false,
            clear_archive_allowed: false,
            delete_allowed: false,
            move_allowed: false,
            archive_allowed: false,
            requires_explicit_operator_approval_for_any_mutation: true,
            no_file_moves_or_deletes_performed: true,
        };
        let packet_value = serde_json::to_value(&packet)?;
        fs::write(&packet_path, serde_json::to_vec_pretty(&packet)?)?;
        fs::write(
            &markdown_summary_path,
            review_packet_markdown(&packet_value),
        )?;
        self.log_event(
            "warden_hades_operator_review_packet_projected",
            Some(&packet_path.display().to_string()),
            packet_value.clone(),
        )?;
        Ok(packet_value)
    }

    fn lifecycle_findings_path(&self) -> PathBuf {
        self.root.join("lifecycle_findings.jsonl")
    }

    fn lifecycle_review_queue_path(&self) -> PathBuf {
        self.root.join("lifecycle_review_queue.jsonl")
    }

    fn lifecycle_policy_automation_report_path(&self) -> PathBuf {
        self.root.join("lifecycle_policy_automation_report.json")
    }
}

fn raw_queue_stats(raw_queue_path: &Path) -> Result<WardenHadesQueueStats> {
    let content = fs::read_to_string(raw_queue_path)?;
    let line_count = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let sha256 = sha256_file(raw_queue_path)?;
    Ok(WardenHadesQueueStats {
        path: raw_queue_path.display().to_string(),
        line_count,
        sha256,
        retention_required: true,
        clear_allowed: false,
        archive_allowed_without_operator_approval: false,
    })
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format_sha256(hasher.finalize().as_slice()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format_sha256(hasher.finalize().as_slice())
}

fn format_sha256(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn warden_review_item(
    line: usize,
    record: Value,
    evidence_dir: &Path,
) -> Result<WardenHadesReviewItem> {
    let severity = record
        .get("severity")
        .and_then(Value::as_str)
        .unwrap_or("review")
        .to_owned();
    let event_type = record
        .get("event_type")
        .or_else(|| record.get("event"))
        .and_then(Value::as_str)
        .unwrap_or("warden_informant_record")
        .to_owned();
    let review_id = format!("whr_raw_{line}");
    let evidence_artifact = write_review_evidence_artifact(
        evidence_dir,
        &review_id,
        "warden_informant_queue",
        Some(line),
        &record,
    )?;
    Ok(WardenHadesReviewItem {
        review_id,
        source: "warden_informant_queue".to_owned(),
        raw_queue_line: Some(line),
        event_type: Some(event_type.clone()),
        severity: severity.clone(),
        classification: warden_record_classification(&event_type, &severity).to_owned(),
        finding_id: None,
        path: None,
        policy_disposition: None,
        record,
        evidence_artifact,
        outcome: queue_outcome(
            "pending_operator_review",
            "retain_raw_or_classify",
            "separate_operator_mutation_packet_required",
            &["raw_queue_record", "operator_decision"],
            true,
            false,
        ),
        approval_status: "pending_operator_review".to_owned(),
        review_required: true,
        destructive_allowed: false,
        allowed_actions: vec![
            "retain_raw".to_owned(),
            "classify".to_owned(),
            "defer".to_owned(),
            "approve_specific_mutation_in_separate_packet".to_owned(),
        ],
    })
}

fn policy_review_item(item: Value, evidence_dir: &Path) -> Result<WardenHadesReviewItem> {
    let finding_id = item
        .get("finding_id")
        .and_then(Value::as_str)
        .unwrap_or("policy_item")
        .to_owned();
    let review_id = format!("whr_policy_{finding_id}");
    let severity = item
        .get("severity")
        .and_then(Value::as_str)
        .unwrap_or("review")
        .to_owned();
    let classification = item
        .get("lifecycle_class")
        .and_then(Value::as_str)
        .unwrap_or("review")
        .to_owned();
    let path = item.get("path").and_then(Value::as_str).map(str::to_owned);
    let policy_disposition = item
        .get("policy_disposition")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| Some("hold".to_owned()));
    let evidence_artifact = write_review_evidence_artifact(
        evidence_dir,
        &review_id,
        "hades_lifecycle_policy_report",
        None,
        &item,
    )?;
    Ok(WardenHadesReviewItem {
        review_id,
        source: "hades_lifecycle_policy_report".to_owned(),
        raw_queue_line: None,
        event_type: None,
        severity,
        classification,
        finding_id: Some(finding_id),
        path,
        policy_disposition,
        record: item,
        evidence_artifact,
        outcome: queue_outcome(
            "pending_operator_review",
            "operator_review_policy_disposition",
            "separate_operator_mutation_packet_required",
            &["policy_report_item", "operator_decision"],
            true,
            false,
        ),
        approval_status: "pending_operator_review".to_owned(),
        review_required: true,
        destructive_allowed: false,
        allowed_actions: vec![
            "retain".to_owned(),
            "operator_approve_archive".to_owned(),
            "operator_approve_quarantine".to_owned(),
            "operator_reject".to_owned(),
        ],
    })
}

fn write_review_evidence_artifact(
    evidence_dir: &Path,
    review_id: &str,
    source: &str,
    raw_queue_line: Option<usize>,
    record: &Value,
) -> Result<WardenHadesEvidenceArtifact> {
    let artifact_path = evidence_dir.join(format!("{}.json", safe_artifact_stem(review_id)));
    let artifact = WardenHadesEvidenceArtifactRecord {
        contract: "annunimas.hades.warden_queue_evidence_artifact.v1".to_owned(),
        generated_at_utc: Utc::now().to_rfc3339(),
        review_id: review_id.to_owned(),
        source: source.to_owned(),
        raw_queue_line,
        record: record.clone(),
        record_sha256: sha256_bytes(serde_json::to_string(record)?.as_bytes()),
        mutation_authorized: false,
        destructive_allowed: false,
    };
    let bytes = serde_json::to_vec_pretty(&artifact)?;
    fs::write(&artifact_path, &bytes)?;
    Ok(WardenHadesEvidenceArtifact {
        contract: artifact.contract,
        path: artifact_path.display().to_string(),
        sha256: sha256_bytes(&bytes),
    })
}

fn queue_outcome(
    status: &str,
    recommended_next_action: &str,
    mutation_gate: &str,
    evidence_required: &[&str],
    append_only_closeout: bool,
    destructive_allowed: bool,
) -> WardenHadesQueueOutcome {
    WardenHadesQueueOutcome {
        status: status.to_owned(),
        recommended_next_action: recommended_next_action.to_owned(),
        mutation_gate: mutation_gate.to_owned(),
        evidence_required: evidence_required
            .iter()
            .map(|evidence| (*evidence).to_owned())
            .collect(),
        append_only_closeout,
        destructive_allowed,
    }
}

fn safe_artifact_stem(review_id: &str) -> String {
    review_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn warden_record_classification(event_type: &str, severity: &str) -> &'static str {
    let event = event_type.to_lowercase();
    let severity = severity.to_lowercase();
    if severity == "critical" || event.contains("destructive") || event.contains("denied") {
        "operator_required_critical"
    } else if severity == "warning" || event.contains("orphan") || event.contains("stale") {
        "operator_review_warning"
    } else {
        "operator_review_info"
    }
}

fn review_packet_markdown(packet: &Value) -> String {
    let generated = packet
        .get("generated_at_utc")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let raw_queue_path = packet
        .get("raw_queue")
        .and_then(|raw| raw.get("path"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let raw_queue_lines = packet
        .get("raw_queue")
        .and_then(|raw| raw.get("line_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let review_items_total = packet
        .get("review_items_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    format!(
        "# WARDEN/HADES Operator Review Packet\n\nGenerated: {generated}\n\nRaw queue: `{raw_queue_path}`\n\nRaw queue records: {raw_queue_lines}\n\nReview items projected: {review_items_total}\n\nThis packet is not authorization. Clear/archive/delete/move actions remain blocked until explicit operator approval is recorded in a separate approval packet. Raw WARDEN queue retention remains required.\n"
    )
}

fn lifecycle_scan_roots(root_path: &Path) -> Vec<PathBuf> {
    [
        "human",
        "docs/plans",
        "audit",
        "core/projects/tasks",
        "core/state",
    ]
    .iter()
    .map(|relative| root_path.join(relative))
    .collect()
}

fn classify_lifecycle_path(
    root_path: &Path,
    path: &Path,
    relative: &str,
) -> Option<LifecycleAuditFinding> {
    let normalized = relative.replace('\\', "/").to_lowercase();
    let content = fs::read_to_string(path).unwrap_or_default();
    if normalized.starts_with("docs/plans/") && is_archive_candidate_content(&content) {
        return Some(lifecycle_finding(
            "archive_candidate",
            "archive_candidate",
            relative,
            "low",
            "archive-after-human-approval",
            evidence(
                root_path,
                path,
                "docs plan explicitly marks obsolete/superseded archive candidacy",
                archive_candidate_signals(&content),
            ),
        ));
    }
    if normalized.starts_with("docs/plans/") && is_stale_plan_content(&content) {
        return Some(lifecycle_finding(
            "stale_plan",
            "review",
            relative,
            "medium",
            "review-plan-authority",
            evidence(root_path, path, "plan has stale/open status signals and needs HADES review before archive decisions", stale_plan_signals(&content)),
        ));
    }
    if normalized.starts_with("human/") && is_archive_candidate_content(&content) {
        return Some(lifecycle_finding(
            "archive_candidate",
            "archive_candidate",
            relative,
            "low",
            "archive-after-human-approval",
            evidence(
                root_path,
                path,
                "human context explicitly marks obsolete/superseded archive candidacy",
                archive_candidate_signals(&content),
            ),
        ));
    }
    if normalized.starts_with("audit/") && generated_delete_candidate(&normalized, &content) {
        return Some(lifecycle_finding(
            "generated_delete_candidate",
            "generated_delete_candidate",
            relative,
            "low",
            "delete-generated-only-after-approval-and-rollback-evidence",
            evidence(
                root_path,
                path,
                "generated audit artifact has cache/temp/generated markers",
                generated_signals(&normalized, &content),
            ),
        ));
    }
    if normalized.starts_with("core/state/") && malformed_json_candidate(path, &content) {
        return Some(lifecycle_finding(
            "state_hygiene",
            "quarantine_candidate",
            relative,
            "high",
            "quarantine-malformed-state-after-operator-approval",
            evidence(
                root_path,
                path,
                "state/task file has malformed JSON evidence and must not be deleted",
                vec!["malformed_json".to_owned()],
            ),
        ));
    }
    if normalized.starts_with("core/state/") && authoritative_state_candidate(&normalized) {
        return Some(lifecycle_finding(
            "retain",
            "retain",
            relative,
            "info",
            "retain-authoritative-runtime-state",
            evidence(root_path, path, "authoritative runtime state file; retain unless a later approved review says otherwise", vec!["core_state_authority".to_owned()]),
        ));
    }
    None
}

fn task_queue_hygiene_findings(
    root_path: &Path,
    task_queue: &Path,
    limit: usize,
) -> Result<Vec<LifecycleAuditFinding>> {
    let content = fs::read_to_string(task_queue)?;
    let mut findings = Vec::new();
    let mut seen_task_ids: HashMap<String, usize> = HashMap::new();
    let relative = display_relative(root_path, task_queue);

    for (idx, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if findings.len() >= limit {
            break;
        }

        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                findings.push(lifecycle_finding(
                    "task_queue_hygiene",
                    "quarantine_candidate",
                    &relative,
                    "high",
                    "repair-task-queue-record",
                    evidence_with_extra(
                        root_path,
                        task_queue,
                        "task queue JSONL line is malformed and needs quarantine/repair review",
                        vec!["malformed_jsonl".to_owned()],
                        serde_json::json!({
                            "detector": "hades_task_queue_hygiene_detector",
                            "issue": "malformed_jsonl",
                            "line": idx + 1,
                            "error": err.to_string(),
                            "safety": no_delete_safety()
                        }),
                    ),
                ));
                continue;
            }
        };

        let task_id = queue_record_task_id(&value);
        if task_id.is_empty() {
            findings.push(lifecycle_finding(
                "task_queue_hygiene",
                "review",
                &relative,
                "medium",
                "repair-task-queue-record",
                evidence_with_extra(
                    root_path,
                    task_queue,
                    "task queue record is missing task_id",
                    vec!["missing_task_id".to_owned()],
                    serde_json::json!({
                        "detector": "hades_task_queue_hygiene_detector",
                        "issue": "missing_task_id",
                        "line": idx + 1,
                        "safety": no_delete_safety()
                    }),
                ),
            ));
        } else if let Some(first_line) = seen_task_ids.get(&task_id) {
            findings.push(lifecycle_finding(
                "task_queue_hygiene",
                "review",
                &relative,
                "medium",
                "deduplicate-task-queue-after-review",
                evidence_with_extra(
                    root_path,
                    task_queue,
                    "task queue contains duplicate task_id",
                    vec!["duplicate_task_id".to_owned()],
                    serde_json::json!({
                        "detector": "hades_task_queue_hygiene_detector",
                        "issue": "duplicate_task_id",
                        "task_id": task_id,
                        "first_line": first_line,
                        "duplicate_line": idx + 1,
                        "safety": no_delete_safety()
                    }),
                ),
            ));
        } else {
            seen_task_ids.insert(task_id, idx + 1);
        }

        if findings.len() >= limit {
            break;
        }

        let status = value.get("status").and_then(Value::as_str).unwrap_or("");
        let owner_missing = value
            .get("owner")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .is_empty();
        if matches!(status, "pending" | "in_progress") && owner_missing {
            findings.push(lifecycle_finding(
                "task_queue_hygiene",
                "review",
                &relative,
                "low",
                "assign-owner-or-close-task-after-review",
                evidence_with_extra(
                    root_path,
                    task_queue,
                    "active task queue record is missing owner",
                    vec!["active_task_missing_owner".to_owned()],
                    serde_json::json!({
                        "detector": "hades_task_queue_hygiene_detector",
                        "issue": "active_task_missing_owner",
                        "line": idx + 1,
                        "status": status,
                        "safety": no_delete_safety()
                    }),
                ),
            ));
        }
    }

    Ok(findings)
}

fn queue_record_task_id(value: &Value) -> String {
    ["task_id", "id", "source_record_id"]
        .iter()
        .find_map(|key| {
            value
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|candidate| !candidate.is_empty())
        })
        .unwrap_or("")
        .to_owned()
}

fn lifecycle_finding(
    finding_type: &str,
    lifecycle_class: &str,
    path: &str,
    severity: &str,
    recommendation: &str,
    evidence: Value,
) -> LifecycleAuditFinding {
    LifecycleAuditFinding {
        finding_id: format!("hla_{}", uuid::Uuid::new_v4().simple()),
        finding_type: finding_type.to_owned(),
        lifecycle_class: lifecycle_class.to_owned(),
        path: path.to_owned(),
        severity: severity.to_owned(),
        recommendation: recommendation.to_owned(),
        evidence,
        review_required: lifecycle_class != "retain",
        destructive_allowed: false,
    }
}

fn finding_record(finding: &LifecycleAuditFinding) -> Value {
    serde_json::json!({
        "contract": LIFECYCLE_FINDING_CONTRACT,
        "appended_at_utc": Utc::now().to_rfc3339(),
        "finding": finding,
        "append_only": true,
        "no_file_moves_or_deletes_performed": true
    })
}

fn evidence(root_path: &Path, path: &Path, reason: &str, matched_signals: Vec<String>) -> Value {
    evidence_with_extra(
        root_path,
        path,
        reason,
        matched_signals,
        serde_json::json!({}),
    )
}

fn evidence_with_extra(
    root_path: &Path,
    path: &Path,
    reason: &str,
    matched_signals: Vec<String>,
    extra: Value,
) -> Value {
    serde_json::json!({
        "evidence_path": display_relative(root_path, path),
        "reason": reason,
        "matched_signals": matched_signals,
        "issue": extra.get("issue").cloned().unwrap_or(Value::Null),
        "extra": extra,
        "safety": no_delete_safety()
    })
}

fn allowed_actions_for(classification: &str) -> Vec<&'static str> {
    match classification {
        "retain" => vec!["retain"],
        "archive_candidate" => vec!["retain", "archive_after_approval"],
        "quarantine_candidate" => vec!["retain", "quarantine_after_approval"],
        "generated_delete_candidate" => vec!["retain", "delete_generated_after_approval"],
        _ => vec!["retain", "review"],
    }
}

fn disposition_key(disposition: &LifecycleDisposition) -> &'static str {
    match disposition {
        LifecycleDisposition::Hold => "hold",
        LifecycleDisposition::Archive => "archive",
        LifecycleDisposition::Remove => "remove",
    }
}

fn policy_allowed_actions(disposition: &str) -> Vec<&'static str> {
    match disposition {
        "hold" => vec!["retain", "review"],
        "archive" => vec!["retain", "archive_after_operator_approval"],
        "remove" => vec!["retain", "remove_after_operator_approval"],
        _ => vec!["retain", "review"],
    }
}

fn should_skip_lifecycle_path(path: &Path) -> bool {
    path.components().any(|component| {
        let raw = component.as_os_str().to_string_lossy();
        matches!(
            raw.as_ref(),
            ".git" | "target" | "node_modules" | ".venv" | "archive"
        )
    })
}

fn display_relative(root_path: &Path, path: &Path) -> String {
    path.strip_prefix(root_path)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn is_stale_plan_content(content: &str) -> bool {
    let lower = content.to_lowercase();
    let has_open_status = lower.contains("status: in progress")
        || lower.contains("status: pending")
        || lower.contains("status: blocked")
        || lower.contains("pending");
    let lacks_recent_completion =
        !lower.contains("status: completed") && !lower.contains("completed 2026");
    let has_stale_marker = lower.contains("stale")
        || lower.contains("last updated: 2024")
        || lower.contains("last updated: 2023")
        || lower.contains("last updated: 2022")
        || lower.contains("last updated: 2021")
        || lower.contains("last updated: 2020");
    has_open_status && lacks_recent_completion && has_stale_marker
}

fn stale_plan_signals(content: &str) -> Vec<String> {
    let lower = content.to_lowercase();
    let mut signals = HashSet::new();
    for signal in [
        "status: in progress",
        "status: pending",
        "status: blocked",
        "pending",
        "stale",
        "last updated: 2024",
        "last updated: 2023",
        "last updated: 2022",
        "last updated: 2021",
        "last updated: 2020",
    ] {
        if lower.contains(signal) {
            signals.insert(signal.to_owned());
        }
    }
    let mut out: Vec<String> = signals.into_iter().collect();
    out.sort();
    out
}

fn is_archive_candidate_content(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("status: superseded")
        || lower.contains("status: obsolete")
        || lower.contains("status: archived_candidate")
        || lower.contains("archive after approval")
        || lower.contains("obsolete")
}

fn archive_candidate_signals(content: &str) -> Vec<String> {
    let lower = content.to_lowercase();
    let mut signals = HashSet::new();
    for signal in [
        "status: superseded",
        "status: obsolete",
        "status: archived_candidate",
        "archive after approval",
        "obsolete",
    ] {
        if lower.contains(signal) {
            signals.insert(signal.to_owned());
        }
    }
    let mut out: Vec<String> = signals.into_iter().collect();
    out.sort();
    out
}

fn generated_delete_candidate(normalized: &str, content: &str) -> bool {
    normalized.contains("generated")
        || normalized.contains("tmp")
        || normalized.contains("cache")
        || content
            .to_lowercase()
            .contains("generated-delete-candidate")
}

fn generated_signals(normalized: &str, content: &str) -> Vec<String> {
    let lower = content.to_lowercase();
    let mut out = Vec::new();
    for signal in ["generated", "tmp", "cache"] {
        if normalized.contains(signal) || lower.contains(signal) {
            out.push(signal.to_owned());
        }
    }
    if lower.contains("generated-delete-candidate") {
        out.push("generated-delete-candidate".to_owned());
    }
    out.sort();
    out.dedup();
    out
}

fn malformed_json_candidate(path: &Path, content: &str) -> bool {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if extension == "json" {
        return serde_json::from_str::<Value>(content).is_err();
    }
    if extension == "jsonl" {
        return content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .any(|line| serde_json::from_str::<Value>(line).is_err());
    }
    false
}

fn authoritative_state_candidate(normalized: &str) -> bool {
    normalized.ends_with(".json") || normalized.ends_with(".toml") || normalized.ends_with(".jsonl")
}

fn no_delete_safety() -> Value {
    serde_json::json!({
        "read_only_audit": true,
        "moves_files": false,
        "deletes_files": false,
        "archives_files": false,
        "destructive_actions_require_quorum": true
    })
}
