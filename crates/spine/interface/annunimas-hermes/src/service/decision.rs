use super::*;

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

#[derive(Debug, Clone)]
struct QueuedTask {
    task_id: String,
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
    lines.push("Reply with A, B, or C.".to_string());
    lines.join("\n")
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
            return Err(AnnunimasError::Agent {
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
    ) -> Option<(DecisionPrompt, DecisionOption)> {
        let prompts = fs::read_to_string(&self.decision_prompts_path).ok()?;
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
        let prompt = candidates.pop()?;
        let option = prompt
            .options
            .iter()
            .find(|o| normalize_choice(&o.key).as_deref() == Some(choice))
            .cloned()?;
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
        let _ = append_jsonl(&self.decision_responses_path, &response);
        Some((prompt, option))
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

        let queued = self.load_queued_tasks(3)?;
        let (question, options) = if queued.is_empty() {
            (
                "Queue is empty. Choose next mode.".to_string(),
                vec![
                    DecisionOption {
                        key: "a".to_string(),
                        label: "Enter chat mode".to_string(),
                        action: "enter chat mode".to_string(),
                    },
                    DecisionOption {
                        key: "b".to_string(),
                        label: "Run maintenance sweep".to_string(),
                        action: "execute hades maintenance sweep".to_string(),
                    },
                    DecisionOption {
                        key: "c".to_string(),
                        label: "Run ATHENA research".to_string(),
                        action: "execute athena research pass".to_string(),
                    },
                ],
            )
        } else {
            let top = &queued[0];
            (
                format!(
                    "Queue has {} pending tasks. Choose next action.",
                    queued.len()
                ),
                vec![
                    DecisionOption {
                        key: "a".to_string(),
                        label: "Drain queued tasks".to_string(),
                        action: "drain queued tasks".to_string(),
                    },
                    DecisionOption {
                        key: "b".to_string(),
                        label: format!("Execute {}", top.task_id),
                        action: format!("execute queued task {}", top.task_id),
                    },
                    DecisionOption {
                        key: "c".to_string(),
                        label: "Review top tasks".to_string(),
                        action: "show top queued tasks and plan".to_string(),
                    },
                ],
            )
        };
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

    fn load_queued_tasks(&self, limit: usize) -> Result<Vec<QueuedTask>> {
        let path = default_task_queue_path();
        let content = match fs::read_to_string(path) {
            Ok(v) => v,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };
        let mut out = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if value.get("status").and_then(|v| v.as_str()) != Some("queued") {
                continue;
            }
            let Some(task_id) = value.get("task_id").and_then(|v| v.as_str()) else {
                continue;
            };
            out.push(QueuedTask {
                task_id: task_id.to_string(),
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }
}
