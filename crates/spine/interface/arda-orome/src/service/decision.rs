use super::*;

pub(super) fn require_available_decision_action(action: &str) -> Result<()> {
    let action = action.trim().to_ascii_lowercase();
    if action == "drain queued tasks"
        || action.starts_with("execute queued task ")
        || action == "execute queued task"
    {
        return Err(ArdaError::Task(
            "legacy JSONL queue decisions are retired; use authenticated Engine objective control"
                .into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub key: String,
    pub label: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPrompt {
    pub prompt_id: String,
    pub source: String,
    pub sender: String,
    pub channel: String,
    pub question: String,
    pub options: Vec<DecisionOption>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionResponse {
    prompt_id: String,
    source: String,
    sender: String,
    channel: String,
    choice: String,
    selected_action: String,
    selected_label: String,
    ts_utc: String,
}

#[derive(Debug, Clone)]
pub(super) struct DecisionExecutionContext {
    pub(super) prompt_id: String,
    pub(super) choice: String,
    pub(super) selected_action: String,
    pub(super) selected_label: String,
}

pub(super) fn format_decision_prompt_message(prompt: &DecisionPrompt) -> String {
    let mut lines = vec![
        format!("Decision: {}", prompt.question),
        format!("Prompt ID: {}", prompt.prompt_id),
        "".to_string(),
    ];
    for option in &prompt.options {
        lines.push(format!(
            "{}. {}",
            option.key.to_ascii_uppercase(),
            option.label
        ));
    }
    lines.push("".to_string());
    lines.push(format!(
        "Reply with {}.",
        prompt
            .options
            .iter()
            .map(|option| option.key.to_ascii_uppercase())
            .collect::<Vec<_>>()
            .join(" or ")
    ));
    lines.join("\n")
}

fn automatic_decision_options() -> (String, Vec<DecisionOption>) {
    (
        "Legacy queue history is read-only; execution uses authenticated Engine objective control."
            .into(),
        vec![
            DecisionOption {
                key: "a".into(),
                label: "Enter chat mode".into(),
                action: "enter chat mode".into(),
            },
            DecisionOption {
                key: "b".into(),
                label: "Review historical queue records (not live work)".into(),
                action: "show top queued tasks and plan".into(),
            },
        ],
    )
}

impl HermesService {
    pub fn create_decision_prompt(
        &self,
        source: &str,
        sender: &str,
        channel: &str,
        question: &str,
        options: Vec<DecisionOption>,
    ) -> Result<DecisionPrompt> {
        let options = options
            .into_iter()
            .filter(|o| !o.key.trim().is_empty() && !o.action.trim().is_empty())
            .collect::<Vec<_>>();
        if options.is_empty() {
            return Err(ArdaError::Agent {
                agent: "hermes".to_string(),
                message: "decision prompt requires at least one option".to_string(),
            });
        }
        let prompt = DecisionPrompt {
            prompt_id: format!("dpr_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]),
            source: source.to_string(),
            sender: sender.to_string(),
            channel: channel.to_string(),
            question: question.to_string(),
            options,
            created_at_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.decision_prompts_path, &prompt)?;
        Ok(prompt)
    }

    pub(super) fn resolve_decision_choice(
        &self,
        source: &str,
        sender: &str,
        channel: &str,
        choice: &str,
    ) -> Result<Option<(DecisionPrompt, DecisionOption)>> {
        let prompts = fs::read_to_string(&self.decision_prompts_path)?;
        let responses = fs::read_to_string(&self.decision_responses_path).unwrap_or_default();
        let mut resolved_prompt_ids = HashSet::new();
        for line in responses.lines() {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(id) = value.get("prompt_id").and_then(|v| v.as_str()) {
                resolved_prompt_ids.insert(id.to_string());
            }
        }
        let mut candidates = prompts
            .lines()
            .filter_map(|line| serde_json::from_str::<DecisionPrompt>(line).ok())
            .filter(|p| p.source == source && p.sender == sender && p.channel == channel)
            .filter(|p| !resolved_prompt_ids.contains(&p.prompt_id))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| a.created_at_utc.cmp(&b.created_at_utc));
        let Some(prompt) = candidates.pop() else {
            return Ok(None);
        };
        let Some(option) = prompt
            .options
            .iter()
            .find(|o| normalize_choice(&o.key).as_deref() == Some(choice))
            .cloned()
        else {
            return Ok(None);
        };
        require_available_decision_action(&option.action)?;
        let response = DecisionResponse {
            prompt_id: prompt.prompt_id.clone(),
            source: source.to_string(),
            sender: sender.to_string(),
            channel: channel.to_string(),
            choice: choice.to_string(),
            selected_action: option.action.clone(),
            selected_label: option.label.clone(),
            ts_utc: Utc::now().to_rfc3339(),
        };
        append_jsonl(&self.decision_responses_path, &response)?;
        Ok(Some((prompt, option)))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_decision_hop(
        &self,
        stage: &str,
        provider: &str,
        channel: &str,
        sender: &str,
        prompt_id: Option<&str>,
        choice: Option<&str>,
        action: Option<&str>,
        report_excerpt: Option<&str>,
        ok: bool,
        error: Option<&str>,
    ) {
        let payload = serde_json::json!({
            "ts_utc": Utc::now().to_rfc3339(),
            "stage": stage,
            "provider": provider,
            "channel": channel,
            "sender": sender,
            "prompt_id": prompt_id,
            "choice": choice,
            "action": action,
            "report_excerpt": report_excerpt,
            "ok": ok,
            "error": error,
        });
        let _ = append_jsonl(&self.decision_metrics_path, &payload);
    }

    pub(super) async fn maybe_send_illuvatar_decision_prompt(
        &self,
        provider_id: &str,
        msg: &InboundMessage,
    ) -> Result<()> {
        if !provider_id.eq_ignore_ascii_case("discord") {
            return Ok(());
        }
        let expected_sender = std::env::var("ANNUNIMAS_ILLUVATAR_DISCORD_USER")
            .unwrap_or_else(|_| "illuvatar".to_string());
        if !msg.sender.eq_ignore_ascii_case(&expected_sender) {
            return Ok(());
        }
        if normalize_choice(&msg.content).is_some() {
            return Ok(());
        }

        let (question, options) = automatic_decision_options();
        let prompt = self.create_decision_prompt(
            provider_id,
            &msg.sender,
            msg.channel.as_deref().unwrap_or("discord"),
            &question,
            options,
        )?;
        let body = format_decision_prompt_message(&prompt);
        let outbound = OutboundMessage::new(
            provider_id.to_string(),
            msg.channel.clone().unwrap_or_else(|| "discord".to_string()),
            format!("Decision Prompt {}", prompt.prompt_id),
            body,
        );
        let _ = self.send(outbound).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_prompts_only_offer_supported_read_only_actions() {
        {
            let (question, options) = automatic_decision_options();
            assert!(!question.contains("pending tasks"));
            for option in options {
                assert!(
                    matches!(
                        option.action.as_str(),
                        "enter chat mode" | "show top queued tasks and plan"
                    ),
                    "unsupported action: {}",
                    option.action
                );
            }
        }
    }
}
