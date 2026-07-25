#![cfg(feature = "full-cli")]

#[derive(Debug, Clone)]
pub struct DecisionOption {
    pub key: String,
    pub label: String,
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct DecisionPrompt {
    pub question: String,
    pub prompt_id: String,
    pub options: Vec<DecisionOption>,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub provider: String,
    pub channel: String,
    pub subject: String,
    pub body: String,
}

impl OutboundMessage {
    pub fn new(provider: String, channel: String, subject: String, body: String) -> Self {
        Self {
            provider,
            channel,
            subject,
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub provider: String,
    pub channel: String,
    pub sender: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct InterruptionMessage {
    pub sender: String,
    pub channel: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct BoardroomPost {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct HermesService;

impl HermesService {
    pub fn create_decision_prompt(
        &self,
        provider: &str,
        sender: &str,
        channel: &str,
        question: &str,
        options: Vec<DecisionOption>,
    ) -> anyhow::Result<DecisionPrompt> {
        let _ = (provider, sender, channel);
        Ok(DecisionPrompt {
            question: question.to_string(),
            prompt_id: format!("prompt-{}", uuid::Uuid::new_v4()),
            options,
        })
    }

    pub async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
        Ok(())
    }
}

use std::fs;
use std::io::{BufRead, BufReader};

use crate::support::load_queued_tasks;

pub(crate) fn resolve_athena_source_id(candidate: &str) -> String {
    let trimmed = candidate.trim();
    if trimmed.is_empty() || trimmed.starts_with("src_") {
        return trimmed.to_string();
    }

    let digest_path = std::path::Path::new("data/athena/digest.jsonl");
    let file = match fs::File::open(digest_path) {
        Ok(file) => file,
        Err(_) => return trimmed.to_string(),
    };

    let mut resolved: Option<String> = None;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(id) = value.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let raw_input = value.get("raw_input").and_then(|v| v.as_str());
        let url = value.get("url").and_then(|v| v.as_str());
        let book_ref = value.get("book_ref").and_then(|v| v.as_str());
        if raw_input == Some(trimmed) || url == Some(trimmed) || book_ref == Some(trimmed) {
            resolved = Some(id.to_string());
        }
    }

    resolved.unwrap_or_else(|| trimmed.to_string())
}

pub(crate) fn format_decision_prompt_message(prompt: &arda_orome::DecisionPrompt) -> String {
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

pub(crate) async fn maybe_send_illuvatar_decision_prompt(
    service: &HermesService,
    provider: &str,
    sender: &str,
    channel: &str,
    content: &str,
    is_illuvatar: bool,
) -> anyhow::Result<()> {
    let expected_sender =
        std::env::var("ARDA_ILLUVATAR_DISCORD_USER").unwrap_or_else(|_| "illuvatar".to_string());
    if !is_illuvatar
        || !provider.eq_ignore_ascii_case("discord")
        || !sender.eq_ignore_ascii_case(&expected_sender)
    {
        return Ok(());
    }
    if is_option_reply(content) {
        return Ok(());
    }

    let queued = load_queued_tasks(3)?;
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
                    label: format!("Execute {}", top.task_id),
                    action: format!("execute queued task {}", top.task_id),
                },
                DecisionOption {
                    key: "b".to_string(),
                    label: "Review top tasks".to_string(),
                    action: "show top queued tasks and plan".to_string(),
                },
                DecisionOption {
                    key: "c".to_string(),
                    label: "Enter chat mode".to_string(),
                    action: "enter chat mode".to_string(),
                },
            ],
        )
    };

    let prompt =
        service.create_decision_prompt("discord", "illuvatar", channel, &question, options)?;
    let msg = OutboundMessage::new(
        provider.to_string(),
        channel.to_string(),
        format!("Decision Prompt {}", prompt.prompt_id),
        format_decision_prompt_message(&prompt),
    );
    let _ = service.send(msg).await?;
    Ok(())
}

fn is_option_reply(content: &str) -> bool {
    matches!(
        content.trim().to_ascii_lowercase().as_str(),
        "a" | "a." | "a)" | "b" | "b." | "b)" | "c" | "c." | "c)"
    )
}
