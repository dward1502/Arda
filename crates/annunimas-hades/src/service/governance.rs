use super::{
    append_jsonl, background_signal_limit, is_low_value_warden_repair_target,
    low_value_warden_repair_class, DestructiveQuorumPolicy, HadesService, QuorumEvaluation,
};
use annunimas_core::daemon::{CommandEnvelope, ResponseEnvelope};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::spawn_bounded_background;
use annunimas_core::task::Task;
use annunimas_governance::{calculate_resonance_basic, triad_validate, TriadConfig};
use annunimas_plutus::{JouleWorkUnit, LoveEquation, PlutusService};
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

impl HadesService {
    pub(super) fn evaluate_destructive_quorum(
        &self,
        file: &str,
        authorized_by: &str,
        proof: Option<&crate::types::QuorumProof>,
    ) -> Result<QuorumEvaluation> {
        let policy = self.load_destructive_quorum_policy();
        if !policy.enabled {
            return Ok(QuorumEvaluation {
                allowed: true,
                required_approvers: 0,
                triad_approvers: Vec::new(),
                approved_count: 0,
                approved_by: Vec::new(),
                has_evidence: true,
                reason: "quorum disabled by policy".to_owned(),
                love_equation_score: 0.72,
            });
        }

        let proof = match proof {
            Some(p) => p,
            None => {
                return Ok(QuorumEvaluation {
                    allowed: false,
                    required_approvers: policy.required_approvers,
                    triad_approvers: policy.triad_approvers.clone(),
                    approved_count: 0,
                    approved_by: Vec::new(),
                    has_evidence: false,
                    reason: format!(
                        "missing quorum_proof for destructive action authorized_by={authorized_by}"
                    ),
                    love_equation_score: 0.22,
                });
            }
        };

        let triad: HashSet<String> = policy
            .triad_approvers
            .iter()
            .map(|v| v.trim().to_ascii_lowercase())
            .collect();
        let mut approved_set: HashSet<String> = HashSet::new();
        for approver in &proof.approvers {
            let normalized = approver.trim().to_ascii_lowercase();
            if triad.contains(&normalized) {
                approved_set.insert(normalized);
            }
        }
        let mut approved_by: Vec<String> = approved_set.into_iter().collect();
        approved_by.sort();
        let approved_count = approved_by.len();
        let has_evidence = !proof.evidence.is_empty();

        let enough_approvers = approved_count >= policy.required_approvers;
        let enough_evidence = !policy.require_evidence || has_evidence;
        let allowed = enough_approvers && enough_evidence;
        let love_equation_score = LoveEquation::new().calculate(
            "hades",
            authorized_by,
            if allowed { 0.84 } else { 0.41 },
            if has_evidence { 0.78 } else { 0.35 },
            (approved_count as f64 / policy.required_approvers.max(1) as f64).clamp(0.0, 1.0),
        );

        let reason = if !enough_approvers {
            format!(
                "requires {} triad approvers, got {}",
                policy.required_approvers, approved_count
            )
        } else if !enough_evidence {
            format!(
                "missing evidence for destructive quorum on {}",
                Path::new(file).display()
            )
        } else {
            "2-of-3 governance quorum satisfied".to_owned()
        };

        Ok(QuorumEvaluation {
            allowed,
            required_approvers: policy.required_approvers,
            triad_approvers: policy.triad_approvers,
            approved_count,
            approved_by,
            has_evidence,
            reason,
            love_equation_score,
        })
    }

    fn load_destructive_quorum_policy(&self) -> DestructiveQuorumPolicy {
        let path = &self.destructive_policy_path;
        let raw = match fs::read_to_string(path) {
            Ok(v) => v,
            Err(err) => {
                tracing::debug!(
                    error = %err,
                    path = %path.display(),
                    "HADES destructive quorum policy missing; using defaults"
                );
                return DestructiveQuorumPolicy::default();
            }
        };
        serde_json::from_str::<DestructiveQuorumPolicy>(&raw).unwrap_or_else(|err| {
            tracing::warn!(
                error = %err,
                path = %path.display(),
                "HADES destructive quorum policy invalid JSON; using defaults"
            );
            DestructiveQuorumPolicy::default()
        })
    }

    pub(super) fn multi_gate_safety_check(&self, file: &Path) -> (bool, serde_json::Value) {
        let file_name = file.file_name().and_then(|v| v.to_str()).unwrap_or("");
        let filename_block = file_name.contains("keep") || file_name.contains("critical");

        let mut task = Task::new(format!("remove {}", file.display()), "remove");
        task.clarifications_resolved = 1;
        task.joule_cost_estimated = 1.0;
        task.joule_cost_actual = 1.0;

        let triad = triad_validate(
            &task,
            Some(&TriadConfig {
                strict: false,
                required_passes: Some(2),
            }),
        );
        let triad_avg = (triad.aurelius_score + triad.bacon_score + triad.sun_tzu_score) / 3.0;
        let resonance = calculate_resonance_basic(&task);
        let joule_balance = resonance
            .ecst_components
            .as_ref()
            .map(|c| c.joule_balance / 100.0)
            .unwrap_or(0.5);
        let love_equation_score = LoveEquation::new().calculate(
            "hades",
            "destructive_guard",
            (resonance.value / 100.0).clamp(0.0, 1.0),
            triad_avg.clamp(0.0, 1.0),
            if triad.passed { 0.74 } else { 0.32 },
        );
        let safe = !filename_block && triad.passed && triad_avg >= 0.55 && joule_balance >= 0.50;

        (
            safe,
            serde_json::json!({
                "tier": "triad_full",
                "filename_blocked": filename_block,
                "triad_passed": triad.passed,
                "triad_average": triad_avg,
                "aurelius_score": triad.aurelius_score,
                "bacon_score": triad.bacon_score,
                "sun_tzu_score": triad.sun_tzu_score,
                "joule_balance": joule_balance,
                "resonance": resonance.value,
                "love_equation_score": love_equation_score,
                "reason": if safe { "multi_gate_pass" } else { "multi_gate_blocked" }
            }),
        )
    }

    pub(super) fn notify_warden(&self, event: &str, file: &Path) -> Result<serde_json::Value> {
        let low_value_repair =
            event == "repair_detected" && is_low_value_warden_repair_target(file);
        let severity = match event {
            "repair_detected" if low_value_repair => "info",
            "file_removed" => "info",
            "destructive_quorum_denied" | "destructive_quorum_blocked_execution" => "warning",
            "orphan_found" | "repair_detected" => "warning",
            _ => "info",
        };
        let status = match event {
            "repair_detected" if low_value_repair => "observed",
            "file_removed" => "healthy",
            "destructive_quorum_denied" | "destructive_quorum_blocked_execution" => {
                "attention_required"
            }
            "orphan_found" | "repair_detected" => "attention_required",
            _ => "observed",
        };
        let source = match event {
            "repair_detected" if low_value_repair => "repair_pipeline_low_value",
            "repair_detected" | "orphan_found" | "file_removed" => "repair_pipeline",
            "destructive_quorum_denied" | "destructive_quorum_blocked_execution" => {
                "destructive_quorum"
            }
            _ => "hades",
        };
        let file_display = file.display().to_string();
        let record = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event,
            "event_type": event,
            "file": file_display,
            "crate_name": "hades",
            "source": source,
            "severity": severity,
            "status": status,
            "synced": low_value_repair || event == "file_removed",
            "repair_class": if low_value_repair {
                serde_json::Value::String(low_value_warden_repair_class(file).to_owned())
            } else {
                serde_json::Value::Null
            }
        });

        append_jsonl(&self.warden_queue_path, &record)?;
        let mut out = serde_json::json!({
            "event": event,
            "file": record["file"].clone(),
            "local_queue_written": true,
            "global_queue_written": false,
            "global_queue_error": null
        });

        let global_path = std::env::var("ANNUNIMAS_WARDEN_QUEUE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.root.join("warden").join("informant_queue.jsonl"));
        if global_path != self.warden_queue_path {
            if let Some(parent) = global_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let write_result: Result<()> = append_jsonl(&global_path, &record);
            match write_result {
                Ok(_) => out["global_queue_written"] = serde_json::json!(true),
                Err(err) => out["global_queue_error"] = serde_json::json!(err.to_string()),
            }
        }
        Ok(out)
    }

    pub(super) fn handoff_athena(&self, event: &str, file: &Path) -> serde_json::Value {
        let file_display = file.display().to_string();
        let socket_path = std::env::var("ANNUNIMAS_ATHENA_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/athena/athena.sock"));
        let payload = serde_json::json!({
            "input": file_display,
            "submitted_by": "hades",
            "task_context": format!("hades_{event}")
        });
        let mut out = serde_json::json!({
            "event": event,
            "file": payload["input"].clone(),
            "athena_socket": socket_path,
            "live_handoff_attempted": false,
            "live_handoff_ok": false,
            "live_handoff_error": null,
            "fallback_queue_written": false
        });

        if socket_path.exists() {
            out["live_handoff_attempted"] = serde_json::json!(true);
            let send = self.send_athena_ipc(&socket_path, "ingest", payload.clone());
            match send {
                Ok(response) => {
                    out["live_handoff_ok"] = serde_json::json!(true);
                    out["live_handoff_response"] = response;
                }
                Err(err) => {
                    out["live_handoff_error"] = serde_json::json!(err.to_string());
                }
            }
        } else {
            out["live_handoff_error"] =
                serde_json::json!(format!("athena socket missing: {}", socket_path.display()));
        }

        let fallback_record = serde_json::json!({
            "ts_utc": Utc::now().to_rfc3339(),
            "event": event,
            "file": payload["input"].clone(),
            "payload": payload,
            "reason": out["live_handoff_error"].clone(),
            "status": if out["live_handoff_ok"].as_bool() == Some(true) {
                "live_sent"
            } else {
                "queued_fallback"
            }
        });
        if append_jsonl(&self.athena_handoff_queue_path, &fallback_record).is_ok() {
            out["fallback_queue_written"] = serde_json::json!(true);
        }
        out
    }

    fn send_athena_ipc(
        &self,
        socket_path: &Path,
        cmd: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(socket_path).map_err(|e| AnnunimasError::Agent {
            agent: "athena".to_owned(),
            message: format!(
                "failed to connect to ATHENA socket {}: {e}",
                socket_path.display()
            ),
        })?;
        let req = CommandEnvelope::new(cmd, payload);
        let mut encoded = serde_json::to_vec(&req)?;
        encoded.push(b'\n');
        stream
            .write_all(&encoded)
            .map_err(|e| AnnunimasError::Agent {
                agent: "athena".to_owned(),
                message: format!("failed to write ATHENA IPC request: {e}"),
            })?;

        let mut line = String::new();
        let mut reader = BufReader::new(stream);
        reader
            .read_line(&mut line)
            .map_err(|e| AnnunimasError::Agent {
                agent: "athena".to_owned(),
                message: format!("failed to read ATHENA IPC response: {e}"),
            })?;
        let response = serde_json::from_str::<ResponseEnvelope>(line.trim()).map_err(|e| {
            AnnunimasError::Agent {
                agent: "athena".to_owned(),
                message: format!("invalid ATHENA IPC response: {e}"),
            }
        })?;
        response.into_result("athena")
    }

    pub(super) fn estimated_sweep_work_amount(&self, result: &crate::types::SweepResult) -> f64 {
        let scanned = result.files_scanned as f64;
        let actions = result.actions_taken as f64;
        let held = result.held_for_review as f64;
        ((scanned / 80.0) + (actions / 8.0) + (held / 4.0)).clamp(0.25, 1.5)
    }

    async fn record_work_signal_async(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback().map_err(|err| {
            AnnunimasError::Agent {
                agent: "hades".to_owned(),
                message: format!("plutus work signal init failed: {err}"),
            }
        })?;
        plutus
            .track_work(agent_id, amount, unit, task_id)
            .await
            .map_err(|err| AnnunimasError::Agent {
                agent: "hades".to_owned(),
                message: format!("plutus work signal failed: {err}"),
            })?;
        Ok(())
    }

    pub(super) fn emit_work_signal_background(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) {
        let service = self.clone();
        let agent_id = agent_id.to_owned();
        let _ = spawn_bounded_background(
            "hades_plutus_signal",
            background_signal_limit(),
            move || async move {
                if let Err(err) = service
                    .record_work_signal_async(&agent_id, amount, unit, task_id)
                    .await
                {
                    tracing::debug!(error = %err, "HADES plutus work signal failed");
                }
            },
        );
    }
}
