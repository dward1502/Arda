use super::CharonService;
use crate::types::RouteLoveEquationGuard;
use arda_core::error::{ArdaError, Result};
use arda_mnemosyne::InformantEvent;
use arda_plutus::{JouleWorkUnit, PlutusService};
use chrono::Utc;

impl CharonService {
    pub(super) fn append_state_event(&self, event: &str, payload: serde_json::Value) -> Result<()> {
        // Hot path: enqueue onto the async event writer; falls back to a sync
        // append_jsonl if the writer task isn't running (no tokio runtime
        // present, e.g. in unit tests) or its channel is full.
        self.event_writer.send_state(&serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event,
            "payload": payload
        }))
    }

    pub(super) fn append_governance_event(
        &self,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.event_writer.send_governance(&serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event,
            "payload": payload
        }))
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
                informant_id: "charon_mneme".to_string(),
                crate_name: "charon".to_string(),
                event_type: event_type.to_string(),
                ts_utc: Utc::now().to_rfc3339(),
                content: content.to_string(),
                confidence_hint,
                tags,
            };
            if let Err(err) = service.encode(event) {
                tracing::debug!(error = %err, "CHARON memory emission failed");
            }
        }
    }

    async fn record_work_signal_async(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback().map_err(|err| {
            ArdaError::Agent {
                agent: "charon".to_string(),
                message: format!("plutus work signal init failed: {err}"),
            }
        })?;
        plutus
            .track_work(agent_id, amount, unit, task_id)
            .await
            .map_err(|err| ArdaError::Agent {
                agent: "charon".to_string(),
                message: format!("plutus work signal failed: {err}"),
            })?;
        Ok(())
    }

    async fn record_relationship_signal_async(
        &self,
        from: &str,
        to: &str,
        resonance: f64,
        attention: f64,
        reciprocity: f64,
    ) -> Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback().map_err(|err| {
            ArdaError::Agent {
                agent: "charon".to_string(),
                message: format!("plutus relationship signal init failed: {err}"),
            }
        })?;
        plutus
            .record_relationship(from, to, resonance, attention, reciprocity)
            .await
            .map_err(|err| ArdaError::Agent {
                agent: "charon".to_string(),
                message: format!("plutus relationship signal failed: {err}"),
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
        let agent_id = agent_id.to_string();
        tokio::spawn(async move {
            if let Err(err) = service
                .record_work_signal_async(&agent_id, amount, unit, task_id)
                .await
            {
                tracing::debug!(error = %err, "CHARON plutus work signal failed");
            }
        });
    }

    pub(super) fn emit_relationship_signal_background(
        &self,
        from: &str,
        to: &str,
        guard: &RouteLoveEquationGuard,
    ) {
        let service = self.clone();
        let from = from.to_string();
        let to = to.to_string();
        let resonance = guard.resonance;
        let attention = guard.attention;
        let reciprocity = guard.reciprocity;
        tokio::spawn(async move {
            if let Err(err) = service
                .record_relationship_signal_async(&from, &to, resonance, attention, reciprocity)
                .await
            {
                tracing::debug!(error = %err, "CHARON plutus relationship signal failed");
            }
        });
    }
}
