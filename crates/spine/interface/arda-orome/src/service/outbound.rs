use super::*;
use arda_core::machine_sigil_or_default;

#[derive(Debug, Clone)]
pub(super) struct RoutedOutboundMessage {
    pub(super) requested_provider: String,
    pub(super) resolved_transport: String,
    pub(super) msg: OutboundMessage,
    pub(super) manwe_route: Option<serde_json::Value>,
}

fn manwe_route_attribution(route: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let route = route?;
    let field = |key: &str| route.get(key).and_then(|value| value.as_str());
    let provider_id = field("provider_id")?;
    let model_id = field("model_id")?;
    Some(serde_json::json!({
        "provider_id": provider_id,
        "model_id": model_id,
        "route_class": field("route_class"),
        "execution_lane": field("execution_lane"),
        "route_id": field("route_id"),
    }))
}

impl HermesService {
    fn decorate_outbound_body(&self, msg: &OutboundMessage) -> String {
        if !msg.provider.eq_ignore_ascii_case("discord") {
            return msg.body.to_string();
        }
        let envelope = self.render_discord_envelope(msg);
        if !env_flag_enabled("ANNUNIMAS_HERMES_DISCORD_ACTIVITY_PANEL", true) {
            return envelope;
        }
        let panel = self.render_agent_activity_panel();
        if panel.is_empty() {
            return envelope;
        }
        let combined = format!("{envelope}\n\n{panel}");
        if combined.len() <= 1900 {
            return combined;
        }
        let summary = self.render_agent_activity_summary();
        let compact = format!("{envelope}\n\n{summary}");
        if compact.len() <= 1900 {
            return compact;
        }
        envelope
    }

    fn render_discord_envelope(&self, msg: &OutboundMessage) -> String {
        let lane = if msg.channel.trim().is_empty() {
            "discord"
        } else {
            msg.channel.trim()
        };
        let priority = msg.priority.trim().to_ascii_lowercase();
        let mut header = format!("𓅃 `{}`", lane);
        if !priority.is_empty() && priority != "normal" {
            header.push_str(&format!(" | {}", priority));
        }

        let title = msg.subject.trim();
        let body = msg.body.trim();

        let sender = "hermes_agent".to_string();
        let trace = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let metadata = format!(
            "`source: {}` | `trace: {}` | ⚡ `cost_est: 1.0`",
            sender, trace
        );

        if title.is_empty() && body.is_empty() {
            return format!("{}\n{}", header, metadata);
        }
        if title.is_empty() {
            return format!("{}\n{}\n\n{}", header, body, metadata);
        }
        if body.is_empty() {
            return format!("{} | **{}**\n\n{}", header, title, metadata);
        }
        format!("{} | **{}**\n{}\n\n{}", header, title, body, metadata)
    }

    pub async fn send(&self, msg: OutboundMessage) -> Result<serde_json::Value> {
        let routed = self.resolve_outbound_message(msg).await;
        let manwe_route_attribution = manwe_route_attribution(routed.manwe_route.as_ref());
        let outbound_body = self.decorate_outbound_body(&routed.msg);
        let mut bacon_task = Task::new(
            format!("send {} {}", routed.requested_provider, routed.msg.subject),
            "send",
        );
        bacon_task.clarifications_resolved = if !routed.msg.channel.is_empty() { 1 } else { 0 };
        append_jsonl(
            &self.outbound_queue_path,
            &serde_json::json!({
                "message_id": format!("{}:{}:{}:{}", routed.requested_provider, routed.msg.channel, routed.msg.subject, routed.msg.created_at_utc),
                "provider": routed.msg.provider,
                "requested_provider": routed.requested_provider,
                "resolved_transport": routed.resolved_transport,
                "channel": routed.msg.channel,
                "subject": routed.msg.subject,
                "body": outbound_body,
                "stream": routed.msg.stream,
                "priority": routed.msg.priority,
                "created_at_utc": routed.msg.created_at_utc,
                "manwe_route": routed.manwe_route,
                "manwe_route_attribution": manwe_route_attribution.clone(),
                "status": "queued",
            }),
        )?;
        append_jsonl(
            &self.messages_path,
            &serde_json::json!({
                "direction": "outbound",
                "provider": routed.msg.provider,
                "requested_provider": routed.requested_provider,
                "resolved_transport": routed.resolved_transport,
                "channel": routed.msg.channel,
                "subject": routed.msg.subject,
                "body": outbound_body,
                "stream": routed.msg.stream,
                "priority": routed.msg.priority,
                "created_at_utc": routed.msg.created_at_utc,
                "manwe_route": routed.manwe_route,
                "manwe_route_attribution": manwe_route_attribution.clone(),
            }),
        )?;

        let mut send_msg = routed.msg.clone();
        send_msg.body = outbound_body;
        let receipt = if let Some(receipt) = try_run_bounded_async(
            "hermes_provider_send",
            Self::provider_send_limit(),
            || async { self.providers.dispatch_with_retry(&send_msg, 3, 250).await },
        )
        .await
        {
            receipt
        } else {
            DispatchReceipt {
                dispatched: false,
                attempts: 0,
                streaming: send_msg.stream,
                chunks_sent: 0,
                provider_id: send_msg.provider.clone(),
                error: Some("provider send concurrency gate saturated".to_string()),
            }
        };
        self.append_outbound_result(&routed, &receipt)?;

        self.emit_memory_event(
            "outbound_queued",
            &format!(
                "HERMES queued outbound message provider={} transport={} channel={} priority={} dispatched={}",
                routed.requested_provider,
                routed.resolved_transport,
                if routed.msg.channel.is_empty() {
                    "unknown"
                } else {
                    routed.msg.channel.as_str()
                },
                routed.msg.priority,
                receipt.dispatched
            ),
            Some(if receipt.dispatched { 0.8 } else { 0.5 }),
            vec![
                "hermes".to_string(),
                "outbound".to_string(),
                format!("provider_{}", routed.requested_provider.to_ascii_lowercase()),
                format!("transport_{}", routed.resolved_transport.to_ascii_lowercase()),
                format!("priority_{}", routed.msg.priority.to_ascii_lowercase()),
            ],
        );
        let trust = if receipt.dispatched { 0.72 } else { 0.55 };
        let reciprocity = if receipt.dispatched { 0.64 } else { 0.42 };
        let longevity = if routed.msg.priority.eq_ignore_ascii_case("urgent") {
            0.58
        } else {
            0.66
        };
        self.record_relationship_signal_async(
            "hermes",
            &normalize_relationship_target(&routed.msg.channel, &routed.requested_provider),
            trust,
            reciprocity,
            longevity,
            "outbound_queued",
        )
        .await;
        self.record_work_signal_async(
            "hermes",
            if receipt.dispatched { 0.85 } else { 0.45 },
            JouleWorkUnit::Network,
            "outbound_queued",
        )
        .await;
        if let Err(err) = record_bacon_lite(
            "hermes",
            "send",
            &bacon_task,
            serde_json::json!({
                "provider": routed.requested_provider,
                "resolved_transport": routed.resolved_transport,
                "channel": routed.msg.channel,
                "dispatched": receipt.dispatched,
                "attempts": receipt.attempts,
                "streaming": receipt.streaming,
                "chunks_sent": receipt.chunks_sent,
                "manwe_route": routed.manwe_route,
                "manwe_route_attribution": manwe_route_attribution.clone(),
            }),
        ) {
            tracing::debug!(error = %err, "HERMES bacon-lite send record failed");
        }

        Ok(serde_json::json!({
            "queued": true,
            "provider": routed.requested_provider,
            "resolved_transport": routed.resolved_transport,
            "channel": routed.msg.channel,
            "dispatched": receipt.dispatched,
            "attempts": receipt.attempts,
            "streaming": receipt.streaming,
            "chunks_sent": receipt.chunks_sent,
            "error": receipt.error,
            "manwe_route": routed.manwe_route,
            "manwe_route_attribution": manwe_route_attribution
        }))
    }

    pub async fn retry_outbound_queue(&self, limit: usize) -> Result<serde_json::Value> {
        let Some(result) = try_run_bounded_async(
            "hermes_outbound_retry",
            Self::outbound_retry_limit(),
            || async move {
                let content = fs::read_to_string(&self.outbound_queue_path)?;
                let mut candidates = Vec::new();
                for line in content.lines() {
                    let value: serde_json::Value = match serde_json::from_str(line) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let provider = value.get("provider").and_then(|v| v.as_str()).unwrap_or("");
                    let channel = value.get("channel").and_then(|v| v.as_str()).unwrap_or("");
                    let subject = value.get("subject").and_then(|v| v.as_str()).unwrap_or("");
                    let body = value.get("body").and_then(|v| v.as_str()).unwrap_or("");
                    if provider.is_empty() || channel.is_empty() {
                        continue;
                    }
                    let mut msg = OutboundMessage::new(provider, channel, subject, body);
                    msg.priority = value
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .unwrap_or("normal")
                        .to_string();
                    msg.stream = value
                        .get("stream")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    msg.created_at_utc = value
                        .get("created_at_utc")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if msg.created_at_utc.is_empty() {
                        msg.created_at_utc = Utc::now().to_rfc3339();
                    }
                    candidates.push(msg);
                }

                if candidates.len() > limit {
                    candidates = candidates[candidates.len() - limit..].to_vec();
                }

                let mut succeeded = 0usize;
                let mut failed = 0usize;
                let mut retried = 0usize;
                for msg in candidates {
                    let receipt = self.providers.dispatch_with_retry(&msg, 3, 250).await;
                    if receipt.attempts > 1 {
                        retried += 1;
                    }
                    if receipt.dispatched {
                        succeeded += 1;
                    } else {
                        failed += 1;
                    }
                    let routed = RoutedOutboundMessage {
                        requested_provider: msg.provider.clone(),
                        resolved_transport: msg.provider.clone(),
                        msg,
                        manwe_route: None,
                    };
                    self.append_outbound_result(&routed, &receipt)?;
                }

                Ok(serde_json::json!({
                    "attempted": succeeded + failed,
                    "succeeded": succeeded,
                    "failed": failed,
                    "retried": retried
                }))
            },
        )
        .await
        else {
            return Err(ArdaError::Agent {
                agent: "hermes".to_string(),
                message: "outbound retry concurrency gate saturated".to_string(),
            });
        };

        result
    }

    fn append_outbound_result(
        &self,
        routed: &RoutedOutboundMessage,
        receipt: &DispatchReceipt,
    ) -> Result<()> {
        let sigil = if receipt.dispatched {
            machine_sigil_or_default(
                "SG_HERMES_DELIVERY_OK",
                vec!["comms".to_string(), "delivery".to_string()],
                "low",
                "summarize",
                "hermes",
            )
        } else if receipt.attempts > 1 {
            machine_sigil_or_default(
                "SG_HERMES_DELIVERY_RETRY",
                vec![
                    "comms".to_string(),
                    "delivery".to_string(),
                    "retry".to_string(),
                ],
                "medium",
                "summarize",
                "hermes",
            )
        } else {
            machine_sigil_or_default(
                "SG_HERMES_DELIVERY_FAILED",
                vec![
                    "comms".to_string(),
                    "delivery".to_string(),
                    "failed".to_string(),
                ],
                "high",
                "keep",
                "hermes",
            )
        };
        append_jsonl(
            &self.outbound_queue_path,
            &serde_json::json!({
                "message_id": format!("{}:{}:{}:{}", routed.requested_provider, routed.msg.channel, routed.msg.subject, routed.msg.created_at_utc),
                "provider": routed.msg.provider,
                "requested_provider": routed.requested_provider,
                "resolved_transport": routed.resolved_transport,
                "channel": routed.msg.channel,
                "subject": routed.msg.subject,
                "created_at_utc": routed.msg.created_at_utc,
                "status": if receipt.dispatched { "completed" } else { "failed" },
                "dispatched": receipt.dispatched,
                "attempts": receipt.attempts,
                "streaming": receipt.streaming,
                "chunks_sent": receipt.chunks_sent,
                "error": receipt.error,
                "manwe_route": routed.manwe_route,
                "manwe_route_attribution": manwe_route_attribution(routed.manwe_route.as_ref()),
                "soterion": sigil.clone(),
                "reported_at_utc": Utc::now().to_rfc3339(),
            }),
        )?;
        let receipt_contract = if routed.resolved_transport.eq_ignore_ascii_case("discord") {
            "hermes.discord.outbound_receipt.v1"
        } else {
            "hermes.outbound_receipt.v1"
        };
        append_jsonl(
            &self.messages_path,
            &serde_json::json!({
                "direction": "outbound_result",
                "receipt_contract": receipt_contract,
                "provider": routed.msg.provider,
                "requested_provider": routed.requested_provider,
                "resolved_transport": routed.resolved_transport,
                "transport": routed.resolved_transport,
                "channel": routed.msg.channel,
                "recipient_class": recipient_class_for_channel(&routed.msg.channel),
                "subject": routed.msg.subject,
                "created_at_utc": routed.msg.created_at_utc,
                "dispatched": receipt.dispatched,
                "attempts": receipt.attempts,
                "streaming": receipt.streaming,
                "chunks_sent": receipt.chunks_sent,
                "error": receipt.error,
                "policy_decision": "allowed",
                "content_redacted": true,
                "manwe_route": routed.manwe_route,
                "manwe_route_attribution": manwe_route_attribution(routed.manwe_route.as_ref()),
                "soterion": sigil,
                "reported_at_utc": Utc::now().to_rfc3339(),
            }),
        )
    }

    pub(super) async fn resolve_outbound_message(
        &self,
        msg: OutboundMessage,
    ) -> RoutedOutboundMessage {
        let requested_provider = msg.provider.clone();
        if !requested_provider.eq_ignore_ascii_case("auto")
            && !requested_provider.eq_ignore_ascii_case("manwe")
        {
            return RoutedOutboundMessage {
                requested_provider: requested_provider.clone(),
                resolved_transport: requested_provider,
                msg,
                manwe_route: None,
            };
        }

        let fallback_transport = std::env::var("ANNUNIMAS_HERMES_AUTO_TRANSPORT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "discord".to_string());
        let route = self.manwe_outbound_route(&msg).ok();
        if let Some(attribution) = manwe_route_attribution(route.as_ref()) {
            tracing::info!(
                provider_id = attribution
                    .get("provider_id")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                model_id = attribution
                    .get("model_id")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                route_class = attribution
                    .get("route_class")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                execution_lane = attribution
                    .get("execution_lane")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                route_id = attribution
                    .get("route_id")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "HERMES resolved Charon route attribution"
            );
        }
        let mut resolved = msg;
        resolved.provider = fallback_transport.clone();
        RoutedOutboundMessage {
            requested_provider,
            resolved_transport: fallback_transport,
            msg: resolved,
            manwe_route: route,
        }
    }

    fn outbound_retry_limit() -> usize {
        std::env::var("ANNUNIMAS_HERMES_OUTBOUND_RETRY_MAX_CONCURRENCY")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1)
    }

    fn provider_send_limit() -> usize {
        std::env::var("ANNUNIMAS_HERMES_SEND_MAX_CONCURRENCY")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(2)
    }
}

fn recipient_class_for_channel(channel: &str) -> &'static str {
    let trimmed = channel.trim();
    if trimmed.is_empty() {
        "unknown"
    } else if trimmed.starts_with('@') || trimmed.starts_with("dm:") {
        "direct"
    } else {
        "channel"
    }
}

pub(super) fn count_outbound_queue_pending(path: &Path) -> Result<usize> {
    let content = fs::read_to_string(path)?;
    let mut latest = std::collections::HashMap::<String, String>::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(message_id) = value.get("message_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(status) = value.get("status").and_then(|v| v.as_str()) else {
            continue;
        };
        latest.insert(message_id.to_string(), status.to_string());
    }
    Ok(latest
        .values()
        .filter(|status| status.as_str() == "queued" || status.as_str() == "pending")
        .count())
}
