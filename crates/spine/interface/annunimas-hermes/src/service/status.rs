use super::{
    count_malformed_jsonl, count_outbound_queue_pending, default_world_state_path,
    load_personality_registry, HermesService,
};
use crate::discord_health::{DiscordBridgeEvidence, DiscordBridgeReadiness};
use annunimas_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStats {
    pub inbound: usize,
    pub outbound: usize,
    pub outbound_failed: usize,
    pub outbound_retried: usize,
    pub tier1_resolved: usize,
    pub tier2_resolved: usize,
    pub tier3_resolved: usize,
    pub escalated_to_prometheus: usize,
    pub avg_joulework: f64,
    pub avg_love_eq: f64,
    pub triad_pass_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesStatus {
    pub providers_online: Vec<String>,
    pub providers_offline: Vec<String>,
    pub messages_today: MessageStats,
    pub boardroom_active: bool,
    pub subcomponents_running: usize,
    pub queue_depth: usize,
    pub agents_active: usize,
    pub agents_idle: usize,
    pub agent_activity: Vec<AgentActivity>,
    pub discord_bridge: DiscordBridgeReadiness,
    pub malformed_message_records: usize,
    pub malformed_boardroom_records: usize,
    pub malformed_queue_records: usize,
    pub malformed_interrupt_records: usize,
    pub l3_readiness_projection: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivity {
    pub agent_id: String,
    pub status: String,
    pub active_tasks: u64,
    pub personality: Option<String>,
    pub comms_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesSubcomponent {
    pub id: String,
    pub kind: String,
    pub status: String,
}

impl HermesService {
    pub async fn status(&self) -> Result<HermesStatus> {
        let providers_online = self.providers.online_provider_ids().await;
        let providers_offline = self.providers.offline_provider_ids().await;
        let discord_bridge =
            self.discord_bridge_readiness_from_provider_snapshot(&providers_online);
        let messages_today = self.message_stats_today()?;
        let queue_depth = count_outbound_queue_pending(&self.outbound_queue_path)?;
        let agent_activity = self.load_agent_activity();
        let agents_active = agent_activity
            .iter()
            .filter(|a| a.active_tasks > 0 || a.status.eq_ignore_ascii_case("online"))
            .count();
        let agents_idle = agent_activity.len().saturating_sub(agents_active);
        Ok(HermesStatus {
            providers_online,
            providers_offline,
            messages_today,
            boardroom_active: true,
            subcomponents_running: self.subcomponents().len(),
            queue_depth,
            agents_active,
            agents_idle,
            agent_activity,
            discord_bridge,
            malformed_message_records: count_malformed_jsonl(&self.messages_path),
            malformed_boardroom_records: count_malformed_jsonl(&self.boardroom_path),
            malformed_queue_records: count_malformed_jsonl(&self.outbound_queue_path),
            malformed_interrupt_records: count_malformed_jsonl(&self.interruptions_path),
            l3_readiness_projection: self.l3_readiness_projection()?,
        })
    }

    pub async fn providers_status(&self) -> serde_json::Value {
        let configured = self.providers.configured_provider_ids();
        let online = self.providers.online_provider_ids().await;
        let offline = self.providers.offline_provider_ids().await;
        let discord_bridge = self.discord_bridge_readiness_from_provider_snapshot(&online);
        serde_json::json!({
            "configured": configured,
            "online": online,
            "offline": offline,
            "discord_bridge": discord_bridge,
        })
    }

    pub fn subcomponents(&self) -> Vec<HermesSubcomponent> {
        vec![
            HermesSubcomponent {
                id: "discord_listener".to_string(),
                kind: "persistent".to_string(),
                status: "running".to_string(),
            },
            HermesSubcomponent {
                id: "boardroom_manager".to_string(),
                kind: "persistent".to_string(),
                status: "running".to_string(),
            },
            HermesSubcomponent {
                id: "outbound_queue".to_string(),
                kind: "persistent".to_string(),
                status: "running".to_string(),
            },
        ]
    }

    pub(super) fn discord_bridge_readiness_from_provider_snapshot(
        &self,
        providers_online: &[String],
    ) -> DiscordBridgeReadiness {
        let configured = self.providers.configured_provider_ids();
        let discord_configured = configured
            .iter()
            .any(|id| id.eq_ignore_ascii_case("discord"))
            || providers_online
                .iter()
                .any(|id| id.eq_ignore_ascii_case("discord"));
        let (recent_outbound_success, recent_inbound_observed) =
            self.recent_discord_delivery_evidence();
        let evidence = DiscordBridgeEvidence {
            bridge_enabled: discord_configured,
            configured: discord_configured,
            listener_running: self.subcomponents().iter().any(|component| {
                component.id == "discord_listener"
                    && component.status.eq_ignore_ascii_case("running")
            }),
            provider_online: providers_online
                .iter()
                .any(|id| id.eq_ignore_ascii_case("discord")),
            recent_outbound_success,
            recent_inbound_observed,
            policy_guard_active: self.discord_policy_guard_active(),
        };
        DiscordBridgeReadiness::classify(&evidence)
    }

    fn recent_discord_delivery_evidence(&self) -> (bool, bool) {
        let content = fs::read_to_string(&self.messages_path).unwrap_or_default();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let mut recent_outbound_success = false;
        let mut recent_inbound_observed = false;
        for line in content.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let ts = value
                .get("reported_at_utc")
                .or_else(|| value.get("received_at_utc"))
                .or_else(|| value.get("created_at_utc"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !ts.starts_with(&today) {
                continue;
            }
            let receipt_contract_is_discord = value
                .get("receipt_contract")
                .and_then(|v| v.as_str())
                .map(|contract| contract == "hermes.discord.outbound_receipt.v1")
                .unwrap_or(false);
            let provider_is_discord = receipt_contract_is_discord
                || value
                    .get("provider")
                    .or_else(|| value.get("requested_provider"))
                    .or_else(|| value.get("resolved_transport"))
                    .or_else(|| value.get("transport"))
                    .or_else(|| value.get("source"))
                    .and_then(|v| v.as_str())
                    .map(|id| id.eq_ignore_ascii_case("discord"))
                    .unwrap_or(false);
            if !provider_is_discord {
                continue;
            }
            match value.get("direction").and_then(|v| v.as_str()) {
                Some("outbound_result") => {
                    if value
                        .get("dispatched")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        recent_outbound_success = true;
                    }
                }
                Some("inbound") => recent_inbound_observed = true,
                _ => {}
            }
        }
        (recent_outbound_success, recent_inbound_observed)
    }

    fn discord_policy_guard_active(&self) -> bool {
        let config_path = self.root.join("config/federated_comms.toml");
        fs::read_to_string(config_path)
            .map(|content| content.contains("policy_guard_required") && content.contains("true"))
            .unwrap_or(false)
    }

    pub(super) fn render_agent_activity_summary(&self) -> String {
        let activity = self.load_agent_activity();
        if activity.is_empty() {
            return "[agent-status] unavailable".to_string();
        }
        let active = activity
            .iter()
            .filter(|a| a.active_tasks > 0 || a.status.eq_ignore_ascii_case("online"))
            .count();
        let idle = activity.len().saturating_sub(active);
        format!(
            "[agent-status] active={} idle={} total={}",
            active,
            idle,
            activity.len()
        )
    }

    pub(super) fn render_agent_activity_panel(&self) -> String {
        let activity = self.load_agent_activity();
        if activity.is_empty() {
            return String::new();
        }
        let active = activity
            .iter()
            .filter(|a| a.active_tasks > 0 || a.status.eq_ignore_ascii_case("online"))
            .count();
        let idle = activity.len().saturating_sub(active);
        let mut lines = vec![format!(
            "AGENTS active={} idle={} total={}",
            active,
            idle,
            activity.len()
        )];
        for entry in activity.iter().take(12) {
            let persona = entry.personality.as_deref().unwrap_or("-");
            let style = entry.comms_style.as_deref().unwrap_or("-");
            lines.push(format!(
                "{} status={} tasks={} persona={} style={}",
                entry.agent_id, entry.status, entry.active_tasks, persona, style
            ));
        }
        format!("```text\n{}\n```", lines.join("\n"))
    }

    fn message_stats_today(&self) -> Result<MessageStats> {
        let content = fs::read_to_string(&self.messages_path)?;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let mut stats = MessageStats {
            inbound: 0,
            outbound: 0,
            outbound_failed: 0,
            outbound_retried: 0,
            tier1_resolved: 0,
            tier2_resolved: 0,
            tier3_resolved: 0,
            escalated_to_prometheus: 0,
            avg_joulework: 0.0,
            avg_love_eq: 0.0,
            triad_pass_rate: 0.0,
        };
        let mut inbound_scored = 0usize;
        let mut joulework_sum = 0.0f64;
        let mut love_eq_sum = 0.0f64;
        let mut triad_present = 0usize;
        let mut triad_passed = 0usize;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let ts = value
                .get("received_at_utc")
                .or_else(|| value.get("created_at_utc"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !ts.starts_with(&today) {
                continue;
            }
            match value.get("direction").and_then(|v| v.as_str()) {
                Some("inbound") => {
                    stats.inbound += 1;
                    if let Some(tier) = value
                        .get("classification")
                        .and_then(|c| c.get("tier"))
                        .and_then(|v| v.as_str())
                    {
                        match tier {
                            "tier1_rule" => stats.tier1_resolved += 1,
                            "tier2_heuristic" => stats.tier2_resolved += 1,
                            "tier3_fallback" | "tier3_charon" => stats.tier3_resolved += 1,
                            _ => {}
                        }
                    }
                    if value
                        .get("escalated_to_prometheus")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        stats.escalated_to_prometheus += 1;
                    }
                    if let Some(classification) = value.get("classification") {
                        if let Some(v) = classification.get("joulework").and_then(|v| v.as_f64()) {
                            joulework_sum += v;
                            inbound_scored += 1;
                        }
                        if let Some(v) = classification.get("love_eq").and_then(|v| v.as_f64()) {
                            love_eq_sum += v;
                        }
                        if let Some(v) =
                            classification.get("triad_passed").and_then(|v| v.as_bool())
                        {
                            triad_present += 1;
                            if v {
                                triad_passed += 1;
                            }
                        }
                    }
                }
                Some("outbound") => stats.outbound += 1,
                Some("boardroom") => {
                    stats.outbound += 1;
                    joulework_sum += 0.9;
                    love_eq_sum += 0.74;
                    inbound_scored += 1;
                }
                Some("interrupt") => {
                    joulework_sum += 0.92;
                    love_eq_sum += 0.61;
                    inbound_scored += 1;
                    triad_present += 1;
                    if value
                        .get("triad_passed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        triad_passed += 1;
                    }
                }
                Some("outbound_result") => {
                    if value
                        .get("dispatched")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        if value.get("attempts").and_then(|v| v.as_u64()).unwrap_or(1) > 1 {
                            stats.outbound_retried += 1;
                        }
                    } else {
                        stats.outbound_failed += 1;
                    }
                    let dispatched = value
                        .get("dispatched")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    joulework_sum += if dispatched { 0.85 } else { 0.45 };
                    love_eq_sum += if dispatched { 0.68 } else { 0.52 };
                    inbound_scored += 1;
                }
                _ => {}
            }
        }
        if inbound_scored > 0 {
            stats.avg_joulework = joulework_sum / inbound_scored as f64;
            stats.avg_love_eq = love_eq_sum / inbound_scored as f64;
        }
        if triad_present > 0 {
            stats.triad_pass_rate = triad_passed as f64 / triad_present as f64;
        }
        Ok(stats)
    }

    fn load_agent_activity(&self) -> Vec<AgentActivity> {
        let world_path = default_world_state_path();
        let personalities = load_personality_registry();
        let content = match fs::read_to_string(world_path) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let agents = value
            .get("agents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut activity = agents
            .into_iter()
            .filter_map(|entry| {
                let agent_id = entry.get("id").and_then(|v| v.as_str())?.to_string();
                let status = entry
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let active_tasks = entry
                    .get("active_tasks")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let persona = personalities.get(&agent_id);
                Some(AgentActivity {
                    agent_id,
                    status,
                    active_tasks,
                    personality: persona
                        .and_then(|v| v.get("personality"))
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string()),
                    comms_style: persona
                        .and_then(|v| v.get("comms_style"))
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string()),
                })
            })
            .collect::<Vec<_>>();
        activity.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        activity
    }
}
