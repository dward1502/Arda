// sigil: REPAIR
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscordSafeMessageState {
    Active,
    Idle,
    Degraded,
    Escalation,
    Vetoed,
    Dead,
}

impl DiscordSafeMessageState {
    fn is_high_risk(self) -> bool {
        matches!(
            self,
            DiscordSafeMessageState::Escalation
                | DiscordSafeMessageState::Vetoed
                | DiscordSafeMessageState::Dead
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordSafeMessage<'a> {
    pub text: &'a str,
    pub state: DiscordSafeMessageState,
    pub receipt: Option<&'a str>,
    pub cause: Option<&'a str>,
    pub next_action: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordSafeMessageValidation {
    pub allowed: bool,
    pub reasons: Vec<String>,
}

pub fn validate_discord_safe_message(
    message: &DiscordSafeMessage<'_>,
) -> DiscordSafeMessageValidation {
    let mut reasons = Vec::new();

    if lacks_plain_language(message.text) {
        reasons.push(
            "Discord human-facing messages require plain-language fallback prose".to_string(),
        );
    }

    if contains_token_like_secret(message.text) {
        reasons.push(
            "Discord human-facing messages must not expose token-like secret values".to_string(),
        );
    }

    if message.state.is_high_risk() {
        if is_blank(message.receipt) {
            reasons.push("High-risk Discord state messages require a receipt".to_string());
        }
        if is_blank(message.cause) {
            reasons.push("High-risk Discord state messages require a cause".to_string());
        }
        if is_blank(message.next_action) {
            reasons.push("High-risk Discord state messages require a next action".to_string());
        }
    }

    DiscordSafeMessageValidation {
        allowed: reasons.is_empty(),
        reasons,
    }
}

fn is_blank(value: Option<&str>) -> bool {
    value.map(str::trim).unwrap_or_default().is_empty()
}

fn lacks_plain_language(text: &str) -> bool {
    !text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .any(is_plain_language_line)
}

fn is_plain_language_line(line: &str) -> bool {
    let alphabetic = line.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    let words = line
        .split_whitespace()
        .filter(|part| part.chars().any(|ch| ch.is_ascii_alphabetic()))
        .count();
    alphabetic >= 12 && words >= 3
}

fn contains_token_like_secret(text: &str) -> bool {
    text.split(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | '`'))
        .any(is_token_like_secret)
}

fn is_token_like_secret(token: &str) -> bool {
    let cleaned = token.trim_matches(|ch: char| matches!(ch, ',' | ';' | ')' | '(' | ']' | '['));
    let value = cleaned
        .split_once('=')
        .map(|(_, value)| value)
        .unwrap_or(cleaned);

    looks_like_named_secret(cleaned) || looks_like_secret_value(value)
}

fn looks_like_named_secret(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    [
        "token=",
        "secret=",
        "api_key=",
        "apikey=",
        "authorization:",
        "bearer ",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        && token.len() >= 16
}

fn looks_like_secret_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("ghp_")
        || lower.starts_with("gho_")
        || lower.starts_with("github_pat_")
        || lower.starts_with("xoxb-")
        || lower.starts_with("discord.")
    {
        return true;
    }

    let long_alnum = value.len() >= 32 && value.chars().all(|ch| ch.is_ascii_alphanumeric());
    let mixed = value.chars().any(|ch| ch.is_ascii_alphabetic())
        && value.chars().any(|ch| ch.is_ascii_digit());
    long_alnum && mixed
}

#[cfg(test)]
mod tests {
    use super::{validate_discord_safe_message, DiscordSafeMessage, DiscordSafeMessageState};

    #[test]
    fn discord_safe_message_blocks_glyph_only_human_facing_messages() {
        let message = DiscordSafeMessage {
            text: "🜁◀∇ ◆ 🜏◕ c=0.91 r=core/state/hermes_discord_runtime.json",
            state: DiscordSafeMessageState::Active,
            receipt: Some("core/state/hermes_discord_runtime.json"),
            cause: None,
            next_action: None,
        };

        let result = validate_discord_safe_message(&message);

        assert!(!result.allowed);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("plain-language")));
    }

    #[test]
    fn discord_safe_message_blocks_token_like_secrets() {
        let message = DiscordSafeMessage {
            text: "🜁◀∇ ◆ 🜏◕ c=0.91 r=core/state/hermes_discord_runtime.json\nHermes bridge is healthy. token=ghp_1234567890abcdefghijklmnopqrstuvwx",
            state: DiscordSafeMessageState::Active,
            receipt: Some("core/state/hermes_discord_runtime.json"),
            cause: None,
            next_action: None,
        };

        let result = validate_discord_safe_message(&message);

        assert!(!result.allowed);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("secret")));
    }

    #[test]
    fn discord_safe_message_blocks_high_risk_state_without_receipt_cause_and_next_action() {
        let message = DiscordSafeMessage {
            text: "🜁◀∇ ▲ 🜏◕ c=0.91\nHermes bridge escalated.",
            state: DiscordSafeMessageState::Escalation,
            receipt: None,
            cause: None,
            next_action: None,
        };

        let result = validate_discord_safe_message(&message);

        assert!(!result.allowed);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("receipt")));
        assert!(result.reasons.iter().any(|reason| reason.contains("cause")));
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("next action")));
    }

    #[test]
    fn discord_safe_message_allows_receipt_backed_plain_language_status() {
        let message = DiscordSafeMessage {
            text: "🜁◀∇ ◆ 🜏◕ c=0.91 r=core/state/hermes_discord_runtime.json\nHermes Discord bridge is online with recent receipt-backed delivery proof.",
            state: DiscordSafeMessageState::Active,
            receipt: Some("core/state/hermes_discord_runtime.json"),
            cause: None,
            next_action: None,
        };

        let result = validate_discord_safe_message(&message);

        assert!(result.allowed, "unexpected reasons: {:?}", result.reasons);
        assert!(result.reasons.is_empty());
    }
}
