use arda_core::error::Result;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::{AthenaStore, BatchIngestReport};

impl AthenaStore {
    pub fn ingest_url_list_file(
        &self,
        path: impl AsRef<Path>,
        submitted_by: &str,
        task_context: &str,
    ) -> Result<BatchIngestReport> {
        let content = fs::read_to_string(path)?;
        let inputs = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        self.ingest_batch(&inputs, submitted_by, task_context)
    }

    pub fn ingest_x_bookmarks_export(
        &self,
        path: impl AsRef<Path>,
        submitted_by: &str,
        task_context: &str,
    ) -> Result<BatchIngestReport> {
        let content = fs::read_to_string(path)?;
        let json = serde_json::from_str::<Value>(&content).ok();
        let mut urls = BTreeSet::new();
        if let Some(value) = &json {
            collect_x_urls(value, &mut urls);
        } else {
            for token in content.split_whitespace() {
                let trimmed = token.trim_matches(|ch: char| {
                    matches!(ch, '"' | '\'' | ',' | ')' | ']' | '}' | '<' | '>')
                });
                if is_x_url(trimmed) {
                    urls.insert(trimmed.to_string());
                }
            }
        }
        let inputs = urls
            .into_iter()
            .map(|url| format!("x-bookmark:{url}"))
            .collect::<Vec<_>>();
        self.ingest_batch(&inputs, submitted_by, task_context)
    }

    pub fn ingest_x_search_capture(
        &self,
        path: impl AsRef<Path>,
        submitted_by: &str,
        task_context: &str,
    ) -> Result<BatchIngestReport> {
        let content = fs::read_to_string(path)?;
        let json = serde_json::from_str::<Value>(&content).ok();
        let mut urls = BTreeSet::new();
        if let Some(value) = &json {
            collect_x_urls(value, &mut urls);
        }
        for token in content.split_whitespace() {
            let trimmed = token.trim_matches(|ch: char| {
                matches!(ch, '"' | '\'' | ',' | ')' | ']' | '}' | '<' | '>' | '`')
            });
            if is_x_url(trimmed) {
                urls.insert(trimmed.to_string());
            }
        }
        let inputs = urls.into_iter().collect::<Vec<_>>();
        self.ingest_batch(&inputs, submitted_by, task_context)
    }

    pub fn ingest_ai_chat_export(
        &self,
        path: impl AsRef<Path>,
        submitted_by: &str,
        task_context: &str,
    ) -> Result<BatchIngestReport> {
        let content = fs::read_to_string(path)?;
        let value = serde_json::from_str::<Value>(&content).unwrap_or_else(|_| {
            serde_json::json!({
                "title": "plain text chat export",
                "conversation_turns": [{"role": "unknown", "content": content}]
            })
        });
        let mut conversations = Vec::new();
        collect_conversations(&value, &mut conversations);
        if conversations.is_empty() {
            conversations.push(serde_json::json!({
                "title": "chat export",
                "conversation_turns": extract_turns(&value)
            }));
        }
        let inputs = conversations
            .into_iter()
            .enumerate()
            .map(|(idx, conversation)| {
                format!(
                    "chat-export:{}\n{}",
                    idx + 1,
                    serde_json::to_string(&conversation).unwrap_or_else(|_| "{}".to_string())
                )
            })
            .collect::<Vec<_>>();
        self.ingest_batch(&inputs, submitted_by, task_context)
    }
}

fn collect_x_urls(value: &Value, urls: &mut BTreeSet<String>) {
    match value {
        Value::String(raw) => {
            for token in raw.split_whitespace() {
                let trimmed = token.trim_matches(|ch: char| {
                    matches!(ch, '"' | '\'' | ',' | ')' | ']' | '}' | '<' | '>')
                });
                if is_x_url(trimmed) {
                    urls.insert(trimmed.to_string());
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_x_urls(item, urls);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_x_urls(item, urls);
            }
        }
        _ => {}
    }
}

fn is_x_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    (lower.starts_with("https://x.com/")
        || lower.starts_with("https://twitter.com/")
        || lower.starts_with("https://www.x.com/")
        || lower.starts_with("https://www.twitter.com/")
        || lower.starts_with("https://mobile.twitter.com/"))
        && (lower.contains("/status/") || lower.contains("/i/web/status/"))
}

fn collect_conversations(value: &Value, conversations: &mut Vec<Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                if looks_like_conversation(item) {
                    conversations.push(normalize_conversation(item));
                } else {
                    collect_conversations(item, conversations);
                }
            }
        }
        Value::Object(map) => {
            if looks_like_conversation(value) {
                conversations.push(normalize_conversation(value));
                return;
            }
            for item in map.values() {
                collect_conversations(item, conversations);
            }
        }
        _ => {}
    }
}

fn looks_like_conversation(value: &Value) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    map.contains_key("mapping")
        || map.contains_key("chat_messages")
        || map.contains_key("conversation_turns")
        || map.contains_key("messages")
}

fn normalize_conversation(value: &Value) -> Value {
    let title = value
        .get("title")
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("chat export");
    serde_json::json!({
        "title": title,
        "conversation_turns": extract_turns(value),
        "raw_format_hint": detect_chat_format(value)
    })
}

fn detect_chat_format(value: &Value) -> &'static str {
    if value.get("mapping").is_some() {
        "chatgpt"
    } else if value.get("chat_messages").is_some() {
        "claude"
    } else {
        "generic"
    }
}

fn extract_turns(value: &Value) -> Vec<Value> {
    let mut turns = Vec::new();
    collect_turns(value, &mut turns);
    turns
}

fn collect_turns(value: &Value, turns: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            if let Some(message) = map.get("message") {
                collect_turns(message, turns);
            }
            if let Some(content) = map.get("content") {
                let role = map
                    .get("role")
                    .or_else(|| map.get("author").and_then(|v| v.get("role")))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                if let Some(text) = content_text(content) {
                    turns.push(serde_json::json!({"role": role, "content": text}));
                    return;
                }
            }
            if let Some(text) = map
                .get("text")
                .or_else(|| map.get("content"))
                .and_then(Value::as_str)
            {
                let role = map
                    .get("sender")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                turns.push(serde_json::json!({"role": role, "content": text}));
                return;
            }
            for item in map.values() {
                collect_turns(item, turns);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_turns(item, turns);
            }
        }
        _ => {}
    }
}

fn content_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(parts) = value.get("parts").and_then(Value::as_array) {
        let joined = parts
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        if !joined.trim().is_empty() {
            return Some(joined);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_x_urls_extracts_hermes_x_search_json() {
        let value = serde_json::json!({
            "query": "annunimas",
            "items": [
                {"url": "https://x.com/example/status/123"},
                {"text": "mirror https://twitter.com/other/status/456"},
                {"url": "https://x.com/example"}
            ]
        });
        let mut urls = BTreeSet::new();
        collect_x_urls(&value, &mut urls);
        assert!(urls.contains("https://x.com/example/status/123"));
        assert!(urls.contains("https://twitter.com/other/status/456"));
        assert!(!urls.contains("https://x.com/example"));
    }
}
