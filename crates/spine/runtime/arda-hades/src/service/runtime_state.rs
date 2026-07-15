use super::{append_jsonl, hades_event_sigil, HadesService, HadesState, JouleWorkRecord};
use crate::types::{ActionKind, ActionRecord, SweepResult, TaskItem};
use arda_core::error::Result;
use arda_mnemosyne::InformantEvent;
use chrono::Utc;
use std::collections::HashSet;
use std::fs;

impl HadesService {
    pub(super) fn load_state(&self) -> Result<HadesState> {
        let content = std::fs::read_to_string(&self.state_path)?;
        if content.trim().is_empty() {
            return Ok(HadesState::default());
        }
        let parsed = serde_json::from_str::<HadesState>(&content).unwrap_or_default();
        Ok(parsed)
    }

    pub(super) fn save_state(&self, state: HadesState) -> Result<()> {
        std::fs::write(&self.state_path, serde_json::to_string_pretty(&state)?)?;
        Ok(())
    }

    pub(super) fn log_event(
        &self,
        event: &str,
        file: Option<&str>,
        details: serde_json::Value,
    ) -> Result<()> {
        let sigil = hades_event_sigil(event, &details);
        append_jsonl(
            &self.log_path,
            &ActionRecord {
                ts: Utc::now().to_rfc3339(),
                event: event.to_owned(),
                file: file.map(str::to_owned),
                sigil_code: sigil.as_ref().map(|value| value.sigil_code.clone()),
                sigil_tags: sigil
                    .as_ref()
                    .map(|value| value.sigil_tags.clone())
                    .unwrap_or_default(),
                sigil_retention: sigil
                    .as_ref()
                    .and_then(|value| value.sigil_retention.clone()),
                sigil_source: sigil.as_ref().and_then(|value| value.sigil_source.clone()),
                details,
            },
        )
    }

    pub(super) fn emit_memory_event(
        &self,
        event_type: &str,
        content: &str,
        confidence_hint: Option<f64>,
        tags: Vec<String>,
    ) {
        if let Some(service) = &self.mnemosyne {
            let event = InformantEvent {
                informant_id: "hades_mneme".to_owned(),
                crate_name: "hades".to_owned(),
                event_type: event_type.to_owned(),
                ts_utc: Utc::now().to_rfc3339(),
                content: content.to_owned(),
                confidence_hint,
                tags,
            };
            if let Err(err) = service.encode(event) {
                tracing::debug!(error = %err, "failed to emit HADES mnemosyne event");
            }
        }
    }

    pub(super) fn existing_orphan_files(&self) -> Result<HashSet<String>> {
        let content = fs::read_to_string(&self.queue_path)?;
        let mut out = HashSet::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let task: TaskItem = match serde_json::from_str(line) {
                Ok(task) => task,
                Err(_) => continue,
            };
            if matches!(task.action, ActionKind::InvestigateOrphan) {
                out.insert(task.file);
            }
        }
        Ok(out)
    }

    pub(super) fn read_all_queue(&self) -> Result<Vec<TaskItem>> {
        let content = fs::read_to_string(&self.queue_path)?;
        let mut out = Vec::new();
        let mut malformed_count = 0usize;
        let mut first_bad_line: Option<usize> = None;
        let mut first_bad_error: Option<String> = None;
        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<TaskItem>(line) {
                Ok(task) => out.push(task),
                Err(err) => {
                    malformed_count += 1;
                    if first_bad_line.is_none() {
                        first_bad_line = Some(idx + 1);
                        first_bad_error = Some(err.to_string());
                    }
                }
            }
        }
        if malformed_count > 0 {
            tracing::warn!(
                malformed_count,
                first_bad_line = first_bad_line.unwrap_or(0),
                first_bad_error = first_bad_error.as_deref().unwrap_or("unknown"),
                "skipping malformed HADES queue lines"
            );
        }
        Ok(out)
    }

    pub(super) fn record_joulework(&self, operation: &str, result: &SweepResult) -> Result<()> {
        let baseline_joules = std::env::var("ARDA_HADES_JOULE_BASELINE")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(250.0);
        let estimated_joules = (result.files_scanned as f64 * 0.05)
            + (result.actions_taken as f64 * 0.8)
            + (result.held_for_review as f64 * 0.2);
        let inference_provider =
            std::env::var("ARDA_ACTIVE_PROVIDER").unwrap_or_else(|_| "none".to_owned());
        let inference_model =
            std::env::var("ARDA_ACTIVE_MODEL").unwrap_or_else(|_| "none".to_owned());
        let inference_origin =
            std::env::var("ARDA_ACTIVE_INFERENCE_ORIGIN").unwrap_or_else(|_| {
                if inference_provider.contains("local") || inference_provider.contains("ollama") {
                    "local".to_owned()
                } else if inference_provider == "none" {
                    "none".to_owned()
                } else {
                    "cloud".to_owned()
                }
            });

        append_jsonl(
            &self.joulework_path,
            &JouleWorkRecord {
                ts_utc: Utc::now().to_rfc3339(),
                component: "hades".to_owned(),
                operation: operation.to_owned(),
                files_scanned: result.files_scanned,
                actions_taken: result.actions_taken,
                orphans_found: result.orphans_found,
                held_for_review: result.held_for_review,
                estimated_joules,
                baseline_joules,
                outside_historical_scope: estimated_joules > baseline_joules,
                inference_provider,
                inference_model,
                inference_origin,
                notes: "HADES maintenance sweep telemetry".to_owned(),
            },
        )
    }
}
