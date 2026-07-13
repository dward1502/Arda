use crate::heartbeat::HeartbeatMode;
use crate::service::{
    append_jsonl, read_recent_jsonl, ContextEngineeringPolicy, PrometheusService, PrometheusStatus,
};
use annunimas_core::error::Result;
use annunimas_hermes::HermesService;
use serde_json::Value;
use std::{fs, path::Path};

impl PrometheusService {
    pub fn status(&self) -> Result<PrometheusStatus> {
        let roster = self.load_roster().or_else(|| self.roster.clone());
        let agents_online = roster.as_ref().map(|r| r.online_agents).unwrap_or(0);
        let agents_silent = roster.as_ref().map(|r| r.silent_agents).unwrap_or(0);
        let thought_count_today = self.thought_ledger.count_today()?;
        let active_orders = self.order_store.active_orders_count()?;
        let pending_escalations = self.order_store.pending_escalations_count()?;
        let resource_state = if self.heartbeat.mode == HeartbeatMode::Interval {
            "stable"
        } else {
            "constrained"
        };
        let total_agents = (agents_online + agents_silent).max(1) as f64;
        let online_ratio = (agents_online as f64 / total_agents).clamp(0.0, 1.0);
        let silent_ratio = (agents_silent as f64 / total_agents).clamp(0.0, 1.0);
        let thought_bonus = (thought_count_today.min(40) as f64) * 0.25;
        let retinue_game_theory_score = (55.0 + online_ratio * 40.0
            - silent_ratio * 10.0
            - (pending_escalations as f64 * 2.0)
            - (active_orders as f64 * 0.25)
            + thought_bonus)
            .clamp(0.0, 100.0);
        let (continuity_events_48h, identity_focus) = self
            .mnemosyne
            .as_ref()
            .and_then(|svc| svc.identity_state().ok())
            .map(|identity| {
                (
                    identity.recent_events.len(),
                    Some(identity.current_mission_focus),
                )
            })
            .unwrap_or((0, None));
        let context_engineering = context_engineering_policy(
            self.heartbeat.interval_ms,
            active_orders,
            pending_escalations,
            continuity_events_48h,
        );
        let (triad_philosopher, triad_philosopher_evidence) =
            load_triad_philosopher_status(&self.core_root);

        Ok(PrometheusStatus {
            heartbeat_mode: self.heartbeat.mode.to_string(),
            heartbeat_interval_ms: self.heartbeat.interval_ms,
            confidence_threshold: self.confidence_threshold,
            agents_online,
            agents_silent,
            active_orders,
            pending_escalations,
            thought_count_today,
            resource_state: resource_state.to_string(),
            retinue_game_theory_score,
            continuity_events_48h,
            identity_focus,
            context_engineering,
            triad_philosopher,
            triad_philosopher_evidence,
        })
    }

    pub fn recent_council_events(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.council_events_path, limit)
    }

    pub fn council_fanout(
        &self,
        topic: &str,
        participants: Vec<String>,
        context: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let participants = if participants.is_empty() {
            vec![
                "athena".to_string(),
                "hades".to_string(),
                "charon".to_string(),
                "mnemosyne".to_string(),
            ]
        } else {
            participants
        };
        let hermes = HermesService::from_default_or_fallback()?;
        let opened = hermes.council_open(topic, participants.clone())?;
        let session_id = opened
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut reports = Vec::new();
        for participant in &participants {
            let body = format!(
                "[{}] council input for topic '{}'. context={}",
                participant,
                topic,
                context
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "{}".to_string())
            );
            let report = hermes.council_report(&session_id, participant, &body)?;
            reports.push(report);
        }
        let outcome = format!(
            "Council fanout complete with {} participants.",
            participants.len()
        );
        let closed = hermes.council_close(&session_id, &outcome)?;
        let out = serde_json::json!({
            "session_id": session_id,
            "topic": topic,
            "participants": participants,
            "reports": reports,
            "closed": closed
        });
        append_jsonl(
            &self.council_events_path,
            &serde_json::json!({
                "ts_utc": chrono::Utc::now().to_rfc3339(),
                "event": "council_fanout",
                "payload": out
            }),
        )?;
        Ok(out)
    }
}

fn load_triad_philosopher_status(core_root: &Path) -> (Option<Value>, Vec<String>) {
    let path = core_root.join("metrics/by_crate/governance/signals.json");
    let Ok(content) = fs::read_to_string(path) else {
        return (None, Vec::new());
    };
    let Ok(signals) = serde_json::from_str::<Value>(&content) else {
        return (None, Vec::new());
    };
    let goal = signals.get("goal");
    let triad_philosopher = goal
        .and_then(|value| value.get("triad_philosopher"))
        .cloned();
    let triad_philosopher_evidence = goal
        .and_then(|value| value.get("triad_philosopher_evidence"))
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .filter(|entry| entry.starts_with("triad_philosopher:"))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    (triad_philosopher, triad_philosopher_evidence)
}

fn context_engineering_policy(
    heartbeat_interval_ms: u64,
    active_orders: usize,
    pending_escalations: usize,
    continuity_events_48h: usize,
) -> ContextEngineeringPolicy {
    let mut context_budget_chars = if heartbeat_interval_ms <= 500 {
        24_000
    } else {
        18_000
    };
    let workload_pressure = active_orders + pending_escalations + continuity_events_48h / 3;
    if workload_pressure >= 12 {
        context_budget_chars = context_budget_chars.min(14_000);
    } else if workload_pressure >= 6 {
        context_budget_chars = context_budget_chars.min(18_000);
    }
    let compaction_target_ratio = if workload_pressure >= 10 { 0.42 } else { 0.58 };
    let reminder_interval_messages = if workload_pressure >= 10 { 4 } else { 7 };

    ContextEngineeringPolicy {
        context_budget_chars,
        compaction_target_ratio,
        reminder_interval_messages,
        required_sections: vec![
            "mission".to_string(),
            "active_constraints".to_string(),
            "recent_decisions".to_string(),
            "open_work".to_string(),
            "verification_targets".to_string(),
        ],
    }
}
