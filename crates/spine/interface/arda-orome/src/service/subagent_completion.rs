use super::council::redact_operating_room_body;
use super::*;

const SUBAGENT_COMPLETION_PACKET_SCHEMA: &str = "annunimas.hermes.subagent_completion_packet.v1";

impl HermesService {
    #[allow(clippy::too_many_arguments)]
    pub fn record_subagent_completion_packet(
        &self,
        task_id: &str,
        agent: &str,
        summary: &str,
        verification: Vec<String>,
        changed_paths: Vec<String>,
        blockers: Vec<String>,
        risk: CommsEventRisk,
        next_action: &str,
        verified: bool,
    ) -> Result<SubagentCompletionPacket> {
        let completion_id = format!(
            "sac_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let task_ref = canonical_task_ref(task_id);
        let summary = redact_operating_room_body(summary);
        let next_action = redact_operating_room_body(next_action);
        let cleaned_verification = clean_list(verification);
        let cleaned_changed_paths = clean_list(changed_paths);
        let cleaned_blockers = clean_list(blockers);
        let review_required = !verified || !cleaned_blockers.is_empty();
        let status = if review_required {
            "needs_review"
        } else {
            "completed"
        }
        .to_string();
        let canonical_refs = vec![task_ref.clone(), completion_id.clone()];
        let packet = SubagentCompletionPacket {
            schema_version: SUBAGENT_COMPLETION_PACKET_SCHEMA.to_string(),
            completion_id: completion_id.clone(),
            task_ref: task_ref.clone(),
            agent: normalize_agent(agent),
            summary: truncate_chars(&summary, 500),
            verification: cleaned_verification,
            changed_paths: cleaned_changed_paths,
            blockers: cleaned_blockers,
            risk: risk.clone(),
            next_action: truncate_chars(&next_action, 300),
            status: status.clone(),
            review_required,
            canonical_refs: canonical_refs.clone(),
            completed_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.messages_path, &packet)?;
        let comms_risk = if review_required && matches!(risk, CommsEventRisk::Low) {
            CommsEventRisk::Medium
        } else {
            risk
        };
        let comms_summary = format!(
            "Subagent {} reported {} for {}: {} Next: {}",
            packet.agent, packet.status, packet.task_ref, packet.summary, packet.next_action
        );
        self.record_comms_event(
            CommsEventType::Status,
            "subagents",
            CommsEventVisibility::OperatorVisible,
            comms_risk,
            &comms_summary,
            canonical_refs,
            PromotionState::Projected,
            true,
        )?;
        Ok(packet)
    }

    pub fn render_subagent_completion_projection(
        &self,
        packet: &SubagentCompletionPacket,
        surface: &str,
    ) -> SubagentCompletionProjection {
        let normalized_surface = normalize_completion_surface(surface);
        let resolution = self
            .resolve_semantic_discord_channel("subagents")
            .unwrap_or_else(|_| SemanticChannelResolution {
                requested: "subagents".to_string(),
                semantic_channel: "subagents".to_string(),
                env_key: "DISCORD_CHANNEL_SUBAGENTS".to_string(),
                discord_recipient: "tasks".to_string(),
                fallback_used: true,
                configured: false,
            });
        let title = format!("Subagent completion {}: {}", packet.status, packet.task_ref);
        let body = truncate_chars(
            &format!(
                "Agent: {}\nTask: {}\nStatus: {}\nRisk: {}\nSummary: {}\nVerification: {}\nChanged paths: {}\nBlockers: {}\nNext: {}",
                packet.agent,
                packet.task_ref,
                packet.status,
                format_completion_risk(&packet.risk),
                packet.summary,
                item_count(packet.verification.len()),
                packet.changed_paths.len(),
                packet.blockers.len(),
                packet.next_action
            ),
            600,
        );
        SubagentCompletionProjection {
            surface: normalized_surface,
            semantic_channel: resolution.semantic_channel,
            dispatch_channel: resolution.discord_recipient,
            completion_id: packet.completion_id.clone(),
            task_ref: packet.task_ref.clone(),
            title,
            body,
        }
    }
}

fn canonical_task_ref(task_id: &str) -> String {
    let trimmed = task_id.trim();
    if trimmed.starts_with("task:") {
        trimmed.to_string()
    } else {
        format!("task:{trimmed}")
    }
}

fn normalize_agent(agent: &str) -> String {
    let trimmed = agent.trim();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_completion_surface(surface: &str) -> String {
    match surface.trim().to_ascii_lowercase().as_str() {
        "discord" => "discord".to_string(),
        "arda" | "arda_hud" | "hud" => "arda".to_string(),
        "terminal" | "cli" | "tui" => "terminal".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "terminal".to_string(),
    }
}

fn clean_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| redact_operating_room_body(value.trim()))
        .filter(|value| !value.is_empty())
        .collect()
}

fn item_count(count: usize) -> String {
    if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    }
}

fn format_completion_risk(risk: &CommsEventRisk) -> &'static str {
    match risk {
        CommsEventRisk::Low => "low",
        CommsEventRisk::Medium => "medium",
        CommsEventRisk::High => "high",
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push('…');
    }
    output
}
