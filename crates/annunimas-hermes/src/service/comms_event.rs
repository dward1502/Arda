use super::council::redact_operating_room_body;
use super::*;

impl HermesService {
    #[allow(clippy::too_many_arguments)]
    pub fn record_comms_event(
        &self,
        event_type: CommsEventType,
        semantic_channel: &str,
        visibility: CommsEventVisibility,
        risk: CommsEventRisk,
        summary: &str,
        canonical_refs: Vec<String>,
        promotion_state: PromotionState,
        raw_content_redacted: bool,
    ) -> Result<CommsEvent> {
        let resolution = self.resolve_semantic_discord_channel(semantic_channel)?;
        let created_at_utc = Utc::now().to_rfc3339();
        let event_id = format!(
            "comms_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let event = CommsEvent {
            schema_version: "annunimas.hermes.comms_event.v1".to_string(),
            event_id,
            event_type,
            semantic_channel: resolution.semantic_channel,
            visibility,
            risk,
            summary: truncate_comms_summary(&redact_operating_room_body(summary)),
            canonical_refs,
            promotion_state,
            raw_content_redacted,
            created_at_utc,
        };
        append_jsonl(&self.comms_events_path, &event)?;
        Ok(event)
    }

    pub fn record_operating_room_comms_event(
        &self,
        event: &OperatingRoomEvent,
        semantic_channel: &str,
    ) -> Result<CommsEvent> {
        let event_type = match event.kind {
            OperatingRoomEventKind::Status => CommsEventType::Status,
            OperatingRoomEventKind::Alert => CommsEventType::Alert,
            OperatingRoomEventKind::Decision => CommsEventType::Decision,
            OperatingRoomEventKind::Command => CommsEventType::OutboundProjection,
        };
        let risk = match event.kind {
            OperatingRoomEventKind::Status => CommsEventRisk::Low,
            OperatingRoomEventKind::Alert => CommsEventRisk::Medium,
            OperatingRoomEventKind::Decision if event.discord_projection_permitted => {
                CommsEventRisk::Medium
            }
            OperatingRoomEventKind::Decision | OperatingRoomEventKind::Command => {
                CommsEventRisk::High
            }
        };
        let visibility = if event.discord_projection_permitted {
            CommsEventVisibility::OperatorVisible
        } else {
            CommsEventVisibility::Internal
        };
        let promotion_state = if event.discord_projection_permitted {
            PromotionState::Projected
        } else {
            PromotionState::Unpromoted
        };
        let mut canonical_refs = Vec::with_capacity(event.evidence_paths.len() + 1);
        canonical_refs.push(event.event_id.clone());
        canonical_refs.extend(event.evidence_paths.iter().cloned());
        let summary = format!("{} — {}", event.subject, event.body);
        self.record_comms_event(
            event_type,
            semantic_channel,
            visibility,
            risk,
            &summary,
            canonical_refs,
            promotion_state,
            true,
        )
    }

    pub fn record_blocked_operating_room_comms_event(
        &self,
        event: &OperatingRoomEvent,
        semantic_channel: &str,
        blocked_reason: &str,
    ) -> Result<CommsEvent> {
        let mut canonical_refs = Vec::with_capacity(event.evidence_paths.len() + 2);
        canonical_refs.push(event.event_id.clone());
        canonical_refs.push(format!("blocked_reason:{blocked_reason}"));
        canonical_refs.extend(event.evidence_paths.iter().cloned());
        let summary = format!("{} — {}", event.subject, event.body);
        self.record_comms_event(
            CommsEventType::OutboundProjection,
            semantic_channel,
            CommsEventVisibility::Internal,
            CommsEventRisk::High,
            &summary,
            canonical_refs,
            PromotionState::Blocked,
            true,
        )
    }
}

fn truncate_comms_summary(summary: &str) -> String {
    const MAX_SUMMARY_CHARS: usize = 500;
    let mut output = summary.chars().take(MAX_SUMMARY_CHARS).collect::<String>();
    if summary.chars().count() > MAX_SUMMARY_CHARS {
        output.push('…');
    }
    output
}
