use super::*;

pub(super) fn classify(service: &HermesService, msg: InboundMessage) -> Result<IntentResult> {
    let (classify_msg, choice_meta) = service.expand_choice_if_needed(&msg);
    let mut classification = classify_message(&classify_msg);
    if classification.tier == "tier3_fallback" {
        if let Some(route_hint) = service.charon_route_hint(&classify_msg) {
            classification.confidence = (classification.confidence + 0.08).min(0.9);
            classification.tier = "tier3_charon".to_string();
            classification.reason = format!("{}; charon_hint={route_hint}", classification.reason);
        }
    }
    let mut bacon_task = Task::new(
        format!("classify {} {}", classify_msg.source, classify_msg.content),
        "classify",
    );
    bacon_task.clarifications_resolved = if classify_msg.content.contains('?') {
        1
    } else {
        0
    };
    let triad = governance_triad_for_message(&classify_msg, &classification);
    classification.triad_passed = Some(triad.passed);
    classification.triad_score =
        Some((triad.aurelius_score + triad.bacon_score + triad.sun_tzu_score) / 3.0);
    if !triad.passed {
        classification.route_to = crate::types::IntentRoute::Prometheus;
        classification.priority = "urgent".to_string();
        classification.reason = format!(
            "{}; triad gate rerouted to prometheus ({})",
            classification.reason,
            triad
                .veto_reason
                .as_deref()
                .unwrap_or("insufficient_pass_count")
        );
    }
    let discord_source = msg.source.eq_ignore_ascii_case("discord");
    let authority = classify_inbound_authority(&msg);
    let receipt_contract = if discord_source {
        "hermes.discord.inbound_receipt.v1"
    } else {
        "hermes.inbound_receipt.v1"
    };
    let (content, content_redacted) = if discord_source {
        ("[REDACTED]".to_string(), true)
    } else {
        (msg.content.clone(), false)
    };
    append_jsonl(
        &service.messages_path,
        &serde_json::json!({
            "direction": "inbound",
            "receipt_contract": receipt_contract,
            "source": msg.source,
            "sender": msg.sender,
            "content": content,
            "content_redacted": content_redacted,
            "received_at_utc": msg.received_at_utc,
            "channel": msg.channel,
            "classification": classification,
            "authority": authority,
            "choice_meta": choice_meta,
            "escalated_to_prometheus": matches!(classification.route_to, crate::types::IntentRoute::Prometheus),
        }),
    )?;
    if msg.source.eq_ignore_ascii_case("discord") {
        append_discord_inbound_runtime(&msg, &classification, &authority, receipt_contract)?;
    }
    service.emit_memory_event(
        "inbound_classified",
        &format!(
            "HERMES classified inbound message from {} on {} to {:?} tier={} triad_passed={}",
            msg.sender,
            msg.channel.as_deref().unwrap_or("unknown"),
            classification.route_to,
            classification.tier,
            triad.passed
        ),
        Some(classification.confidence),
        vec![
            "hermes".to_string(),
            classification.tier.clone(),
            format!("route_{:?}", classification.route_to).to_ascii_lowercase(),
            format!("source_{}", msg.source.to_ascii_lowercase()),
        ],
    );
    emit_relationship_signal_background(
        "hermes".to_string(),
        normalize_relationship_target(msg.channel.as_deref().unwrap_or(&msg.sender), &msg.source),
        classification.confidence.clamp(0.35, 0.94),
        classification.love_eq.clamp(0.25, 0.92),
        if triad.passed { 0.72 } else { 0.48 },
        "inbound_classified",
    );
    emit_work_signal_background(
        "hermes".to_string(),
        classification.joulework.clamp(0.15, 1.0),
        JouleWorkUnit::Attention,
        "inbound_classified",
    );
    if let Err(err) = record_bacon_lite(
        "hermes",
        "classify",
        &bacon_task,
        serde_json::json!({
            "source": msg.source,
            "sender": msg.sender,
            "route_to": format!("{:?}", classification.route_to),
            "triad_passed": classification.triad_passed,
            "triad_score": classification.triad_score,
        }),
    ) {
        tracing::debug!(error = %err, "HERMES bacon-lite classify record failed");
    }
    Ok(classification)
}

fn classify_inbound_authority(msg: &InboundMessage) -> serde_json::Value {
    let expected_illuvatar = std::env::var("ANNUNIMAS_ILLUVATAR_DISCORD_USER")
        .unwrap_or_else(|_| "illuvatar".to_string());
    let sender = msg.sender.trim();
    let (level, action_execution_allowed, reason) = if msg.is_illuvatar
        || sender.eq_ignore_ascii_case(expected_illuvatar.trim())
    {
        (
            "sovereign",
            true,
            "sender matched ANNUNIMAS_ILLUVATAR_DISCORD_USER or explicit Illuvatar flag",
        )
    } else if env_csv_contains("ANNUNIMAS_HERMES_DISCORD_GUARDIANS", sender) {
        (
            "guardian",
            true,
            "sender matched ANNUNIMAS_HERMES_DISCORD_GUARDIANS",
        )
    } else if env_csv_contains("ANNUNIMAS_HERMES_DISCORD_WORKERS", sender) {
        (
            "worker",
            false,
            "sender matched ANNUNIMAS_HERMES_DISCORD_WORKERS; execution still requires explicit policy/action gate",
        )
    } else {
        (
            "observer",
            false,
            "sender is not in the Discord authority allowlists",
        )
    };

    serde_json::json!({
        "schema": "annunimas.hermes.discord.authority.v1",
        "level": level,
        "action_execution_allowed": action_execution_allowed,
        "reason": reason,
        "policy": "discord_inbound_bounded_optional_bridge",
    })
}

fn env_csv_contains(var_name: &str, needle: &str) -> bool {
    std::env::var(var_name)
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .any(|entry| entry.eq_ignore_ascii_case(needle))
        })
        .unwrap_or(false)
}

fn append_discord_inbound_runtime(
    msg: &InboundMessage,
    classification: &IntentResult,
    authority: &serde_json::Value,
    receipt_contract: &str,
) -> Result<()> {
    let path = default_discord_runtime_state_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let action_execution_allowed = authority
        .get("action_execution_allowed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let receipt = serde_json::json!({
        "direction": "inbound",
        "receipt_contract": receipt_contract,
        "provider": msg.source,
        "sender": msg.sender,
        "channel": msg.channel,
        "received_at_utc": msg.received_at_utc,
        "classification": {
            "intent": classification.intent,
            "priority": classification.priority,
            "route_to": classification.route_to,
            "tier": classification.tier,
            "confidence": classification.confidence,
            "triad_passed": classification.triad_passed,
            "triad_score": classification.triad_score,
        },
        "authority": authority,
        "policy_decision": if action_execution_allowed {
            "action_execution_allowed"
        } else {
            "classified_only"
        },
        "content_redacted": true,
    });

    let mut state = fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "schema": "annunimas.hermes.discord.runtime.v1",
                "last_inbound": null,
                "last_outbound": null,
                "inbound_receipts": [],
                "outbound_receipts": [],
            })
        });

    if !state.is_object() {
        state = serde_json::json!({
            "schema": "annunimas.hermes.discord.runtime.v1",
            "last_inbound": null,
            "last_outbound": null,
            "inbound_receipts": [],
            "outbound_receipts": [],
        });
    }
    state["schema"] = serde_json::Value::String("annunimas.hermes.discord.runtime.v1".to_string());
    state["last_inbound"] = receipt.clone();
    if !state
        .get("inbound_receipts")
        .map(|value| value.is_array())
        .unwrap_or(false)
    {
        state["inbound_receipts"] = serde_json::Value::Array(Vec::new());
    }
    if let Some(receipts) = state
        .get_mut("inbound_receipts")
        .and_then(|value| value.as_array_mut())
    {
        receipts.push(receipt);
        let keep_from = receipts.len().saturating_sub(20);
        if keep_from > 0 {
            receipts.drain(0..keep_from);
        }
    }
    let encoded = serde_json::to_string_pretty(&state)?;
    fs::write(path, format!("{encoded}\n"))?;
    Ok(())
}

impl HermesService {
    pub(super) async fn record_relationship_signal_async(
        &self,
        from: &str,
        to: &str,
        trust: f64,
        reciprocity: f64,
        longevity: f64,
        context: &'static str,
    ) {
        record_relationship_signal_async(from, to, trust, reciprocity, longevity, context).await;
    }

    pub(super) async fn record_work_signal_async(
        &self,
        agent: &str,
        amount: f64,
        unit: JouleWorkUnit,
        context: &'static str,
    ) {
        record_work_signal_async(agent, amount, unit, context).await;
    }

    pub(super) fn emit_relationship_signal_background(
        &self,
        from: String,
        to: String,
        trust: f64,
        reciprocity: f64,
        longevity: f64,
        context: &'static str,
    ) {
        emit_relationship_signal_background(from, to, trust, reciprocity, longevity, context);
    }

    pub(super) fn emit_work_signal_background(
        &self,
        agent: String,
        amount: f64,
        unit: JouleWorkUnit,
        context: &'static str,
    ) {
        emit_work_signal_background(agent, amount, unit, context);
    }

    pub(super) fn expand_choice_if_needed(
        &self,
        msg: &InboundMessage,
    ) -> (InboundMessage, Option<serde_json::Value>) {
        let Some(choice) = normalize_choice(&msg.content) else {
            return (msg.clone(), None);
        };
        let Some((prompt, option)) = self.resolve_decision_choice(
            &msg.source,
            &msg.sender,
            msg.channel.as_deref().unwrap_or(""),
            &choice,
        ) else {
            return (msg.clone(), None);
        };
        let mut routed = msg.clone();
        routed.content = option.action.clone();
        (
            routed,
            Some(serde_json::json!({
                "prompt_id": prompt.prompt_id,
                "choice": choice,
                "selected_label": option.label,
                "selected_action": option.action,
            })),
        )
    }
}

fn governance_triad_for_message(
    msg: &InboundMessage,
    classification: &IntentResult,
) -> arda_governance::TriadResult {
    let mut task = Task::new(&msg.content, "query");
    task.joule_cost_estimated = (classification.joulework * 10.0).max(0.1);
    task.joule_cost_actual = (classification.joulework * 10.0).max(0.1);
    task.clarifications_requested = if msg.content.contains('?') { 1 } else { 0 };
    task.clarifications_resolved = if classification.confidence >= 0.55 {
        1
    } else {
        0
    };
    let cfg = TriadConfig {
        strict: false,
        required_passes: Some(2),
    };
    triad_validate(&task, Some(&cfg))
}

async fn record_relationship_signal_async(
    from: &str,
    to: &str,
    trust: f64,
    reciprocity: f64,
    longevity: f64,
    context: &'static str,
) {
    match PlutusService::from_default_or_workspace_fallback() {
        Ok(service) => {
            if let Err(err) = service
                .record_relationship(from, to, trust, reciprocity, longevity)
                .await
            {
                tracing::debug!(error = %err, context, "HERMES relationship signal failed");
            }
        }
        Err(err) => {
            tracing::debug!(error = %err, context, "HERMES could not open PLUTUS service");
        }
    }
}

async fn record_work_signal_async(
    agent: &str,
    amount: f64,
    unit: JouleWorkUnit,
    context: &'static str,
) {
    match PlutusService::from_default_or_workspace_fallback() {
        Ok(service) => {
            if let Err(err) = service.track_work(agent, amount, unit, None).await {
                tracing::debug!(error = %err, context, "HERMES work signal failed");
            }
        }
        Err(err) => {
            tracing::debug!(error = %err, context, "HERMES could not open PLUTUS service");
        }
    }
}

fn emit_relationship_signal_background(
    from: String,
    to: String,
    trust: f64,
    reciprocity: f64,
    longevity: f64,
    context: &'static str,
) {
    let _ = spawn_bounded_background(
        "hermes_plutus_signal",
        background_signal_limit(),
        move || async move {
            record_relationship_signal_async(&from, &to, trust, reciprocity, longevity, context)
                .await;
        },
    );
}

fn emit_work_signal_background(
    agent: String,
    amount: f64,
    unit: JouleWorkUnit,
    context: &'static str,
) {
    let _ = spawn_bounded_background(
        "hermes_plutus_signal",
        background_signal_limit(),
        move || async move {
            record_work_signal_async(&agent, amount, unit, context).await;
        },
    );
}

fn background_signal_limit() -> usize {
    std::env::var("ANNUNIMAS_BACKGROUND_SIGNAL_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn classify_routes_ambiguous_message_through_fallback_path() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        let msg = InboundMessage::new("discord", "operator", "blorb");

        let result = service.classify(msg).expect("classify");

        assert_eq!(result.tier, "tier3_fallback");
        assert!(matches!(result.intent, crate::types::IntentClass::Unknown));
        assert!(matches!(
            result.route_to,
            crate::types::IntentRoute::Prometheus
        ));
    }
}
