use crate::adaptive::service::types::ProviderState;
use serde_json::Value as JsonValue;
use crate::adaptive::service::error::{ArdaError, Result};
use base64::Engine;
use std::path::PathBuf;
use std::time::Duration;

pub(super) struct CodexResponsesRequest {
    pub url: String,
    pub bearer: String,
    pub chatgpt_account_id: Option<String>,
    pub body: JsonValue,
}

pub(super) struct CodexResponsesOutcome {
    pub status: u16,
    pub response: JsonValue,
    pub error: Option<String>,
}

pub(super) fn build_codex_responses_request(
    provider: &ProviderState,
    model_id: &str,
    body: &JsonValue,
) -> Result<CodexResponsesRequest> {
    let bearer = resolve_codex_access_token()?;
    let base_url = provider
        .base_url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("HERMES_CODEX_BASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "https://chatgpt.com/backend-api/codex".to_string())
        .trim_end_matches('/')
        .to_string();
    let url = format!("{base_url}/responses");
    let request_body = chat_body_to_responses_body(model_id, body);
    Ok(CodexResponsesRequest {
        url,
        chatgpt_account_id: chatgpt_account_id_from_jwt(&bearer),
        bearer,
        body: request_body,
    })
}

pub(super) async fn response_to_codex_outcome(
    response: reqwest::Response,
) -> CodexResponsesOutcome {
    let status = response.status().as_u16();
    let text = response.text().await.unwrap_or_default();
    let parsed: JsonValue = parse_codex_response_text(&text);
    if (200..300).contains(&status) {
        if parsed.get("error").is_some() {
            return CodexResponsesOutcome {
                status: 502,
                error: Some(error_preview(&parsed)),
                response: serde_json::json!({
                    "error": {
                        "message": error_preview(&parsed),
                        "type": "codex_responses_error",
                    },
                    "raw": parsed,
                    "_charon_driver": "codex_responses",
                }),
            };
        }
        CodexResponsesOutcome {
            status,
            response: responses_body_to_chat_completion(&parsed),
            error: None,
        }
    } else {
        CodexResponsesOutcome {
            status,
            error: Some(error_preview(&parsed)),
            response: serde_json::json!({
                "error": {
                    "message": error_preview(&parsed),
                    "type": "codex_responses_error",
                },
                "raw": parsed,
                "_charon_driver": "codex_responses",
            }),
        }
    }
}

pub(super) fn codex_responses_timeout(default_timeout: Duration) -> Duration {
    std::env::var("ARDA_CODEX_RESPONSES_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
        .unwrap_or(default_timeout)
}

fn chat_body_to_responses_body(model_id: &str, body: &JsonValue) -> JsonValue {
    let (instructions, input) = split_messages_for_responses(body);
    let mut payload = serde_json::json!({
        "model": model_id,
        "instructions": instructions,
        "input": input,
        "store": false,
        "stream": true,
    });
    if let Some(reasoning) = responses_reasoning(body) {
        payload["reasoning"] = reasoning;
        payload["include"] = serde_json::json!(["reasoning.encrypted_content"]);
    }
    if let Some(tools) = responses_tools(body) {
        payload["tools"] = tools;
    }
    if let Some(tool_choice) = responses_tool_choice(body) {
        payload["tool_choice"] = tool_choice;
    }
    payload
}

fn parse_codex_response_text(text: &str) -> JsonValue {
    if let Ok(parsed) = serde_json::from_str::<JsonValue>(text) {
        return parsed;
    }
    if let Some(parsed) = codex_sse_text_to_responses_body(text) {
        return parsed;
    }
    serde_json::json!({"raw": text})
}

fn codex_sse_text_to_responses_body(text: &str) -> Option<JsonValue> {
    if !text
        .lines()
        .any(|line| line.starts_with("data:") || line.starts_with("event:"))
    {
        return None;
    }

    let mut output_items = Vec::<JsonValue>::new();
    let mut output_text = String::new();
    let mut usage = JsonValue::Null;
    let mut response_id = JsonValue::Null;
    let mut status = "completed".to_string();
    let mut terminal_error = JsonValue::Null;
    let mut saw_terminal = false;
    let mut saw_event = false;

    for frame in text.split("\n\n") {
        let mut event_name = String::new();
        let mut data_lines = Vec::<String>::new();
        for line in frame.lines() {
            if let Some(rest) = line.strip_prefix("event:") {
                event_name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim_start().to_string());
            }
        }
        if data_lines.is_empty() {
            continue;
        }
        let data = data_lines.join("\n");
        if data.trim() == "[DONE]" {
            continue;
        }
        let Ok(event) = serde_json::from_str::<JsonValue>(&data) else {
            continue;
        };
        saw_event = true;
        let event_type = event
            .get("type")
            .and_then(JsonValue::as_str)
            .unwrap_or(event_name.as_str());

        if event_type == "error" {
            let message = event
                .get("message")
                .cloned()
                .or_else(|| event.get("error").cloned())
                .unwrap_or_else(|| serde_json::json!("Codex Responses stream emitted error"));
            return Some(serde_json::json!({
                "error": {
                    "message": message,
                    "type": "codex_responses_stream_error",
                },
                "_charon_driver": "codex_responses",
            }));
        }

        if event_type.contains("output_text.delta") || event_type == "response.output_text.delta" {
            if let Some(delta) = event.get("delta").and_then(JsonValue::as_str) {
                output_text.push_str(delta);
            }
            continue;
        }

        if event_type == "response.output_item.done" {
            if let Some(item) = event.get("item") {
                output_items.push(item.clone());
            }
            continue;
        }

        if matches!(
            event_type,
            "response.completed" | "response.incomplete" | "response.failed"
        ) {
            saw_terminal = true;
            if let Some(response) = event.get("response") {
                usage = response.get("usage").cloned().unwrap_or(JsonValue::Null);
                response_id = response.get("id").cloned().unwrap_or(JsonValue::Null);
                if let Some(response_status) = response.get("status").and_then(JsonValue::as_str) {
                    status = response_status.to_string();
                } else if event_type == "response.incomplete" {
                    status = "incomplete".to_string();
                } else if event_type == "response.failed" {
                    status = "failed".to_string();
                }
                if event_type == "response.failed" {
                    terminal_error = response.get("error").cloned().unwrap_or(JsonValue::Null);
                }
            }
            break;
        }
    }

    if !saw_event {
        return None;
    }
    if status == "failed" {
        return Some(serde_json::json!({
            "id": response_id,
            "status": status,
            "error": terminal_error,
            "_charon_driver": "codex_responses",
        }));
    }
    if output_items.is_empty() && !output_text.is_empty() {
        output_items.push(serde_json::json!({
            "type": "message",
            "role": "assistant",
            "status": "completed",
            "content": [{"type": "output_text", "text": output_text}],
        }));
    }
    if !saw_terminal && output_items.is_empty() && output_text.is_empty() {
        return Some(serde_json::json!({
            "error": {
                "message": "Codex Responses stream did not emit a terminal response",
                "type": "codex_responses_stream_error",
            },
            "_charon_driver": "codex_responses",
        }));
    }
    Some(serde_json::json!({
        "id": response_id,
        "status": status,
        "model": JsonValue::Null,
        "output": output_items,
        "output_text": output_text,
        "usage": usage,
        "_charon_driver": "codex_responses",
    }))
}

fn split_messages_for_responses(body: &JsonValue) -> (String, JsonValue) {
    let mut instructions = "You are a helpful assistant.".to_string();
    let mut input = Vec::new();
    for message in body
        .get("messages")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
    {
        let role = message
            .get("role")
            .and_then(JsonValue::as_str)
            .unwrap_or("user");
        let raw_content = message.get("content").unwrap_or(&JsonValue::Null);
        let content = flatten_content(raw_content);
        if role == "system" {
            if !content.trim().is_empty() {
                instructions = content;
            }
            continue;
        }
        let response_role = if role == "assistant" {
            "assistant"
        } else {
            "user"
        };
        let content_value = if raw_content.is_string() {
            JsonValue::String(content)
        } else {
            let text_type = if response_role == "assistant" {
                "output_text"
            } else {
                "input_text"
            };
            serde_json::json!([{"type": text_type, "text": content}])
        };
        input.push(serde_json::json!({
            "role": response_role,
            "content": content_value,
        }));
    }
    if input.is_empty() {
        input.push(serde_json::json!({
            "role": "user",
            "content": [{"type": "input_text", "text": ""}],
        }));
    }
    (instructions, JsonValue::Array(input))
}

fn responses_reasoning(body: &JsonValue) -> Option<JsonValue> {
    let extra_body = body.get("extra_body").and_then(JsonValue::as_object);
    let reasoning = body
        .get("reasoning")
        .or_else(|| extra_body.and_then(|extra| extra.get("reasoning")))?;
    let reasoning = reasoning.as_object()?;
    if reasoning
        .get("enabled")
        .and_then(JsonValue::as_bool)
        .is_some_and(|enabled| !enabled)
    {
        return None;
    }
    let effort = reasoning
        .get("effort")
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| if value == "minimal" { "low" } else { value })
        .unwrap_or("medium");
    Some(serde_json::json!({"effort": effort, "summary": "auto"}))
}

fn responses_body_to_chat_completion(body: &JsonValue) -> JsonValue {
    let content = extract_responses_text(body);
    let tool_calls = extract_responses_tool_calls(body);
    let finish_reason = if tool_calls.is_empty() {
        "stop"
    } else {
        "tool_calls"
    };
    let mut message = serde_json::json!({
        "role": "assistant",
        "content": content,
    });
    if !tool_calls.is_empty() {
        message["tool_calls"] = JsonValue::Array(tool_calls);
    }
    serde_json::json!({
        "id": body.get("id").cloned().unwrap_or_else(|| serde_json::json!("chatcmpl-codex-responses")),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": body.get("model").cloned().unwrap_or(JsonValue::Null),
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason,
        }],
        "usage": body.get("usage").cloned().unwrap_or_else(|| serde_json::json!({
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        })),
        "_charon_driver": "codex_responses",
    })
}

fn responses_tools(body: &JsonValue) -> Option<JsonValue> {
    let tools = body.get("tools").and_then(JsonValue::as_array)?;
    let mapped = tools
        .iter()
        .filter_map(|tool| {
            let function = tool.get("function").and_then(JsonValue::as_object)?;
            let name = function.get("name").and_then(JsonValue::as_str)?;
            let mut mapped = serde_json::Map::new();
            mapped.insert(
                "type".to_string(),
                JsonValue::String("function".to_string()),
            );
            mapped.insert("name".to_string(), JsonValue::String(name.to_string()));
            if let Some(description) = function.get("description") {
                mapped.insert("description".to_string(), description.clone());
            }
            mapped.insert(
                "parameters".to_string(),
                function
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({"type": "object"})),
            );
            if let Some(strict) = function.get("strict").or_else(|| tool.get("strict")) {
                mapped.insert("strict".to_string(), strict.clone());
            }
            Some(JsonValue::Object(mapped))
        })
        .collect::<Vec<_>>();
    (!mapped.is_empty()).then(|| JsonValue::Array(mapped))
}

fn responses_tool_choice(body: &JsonValue) -> Option<JsonValue> {
    let choice = body.get("tool_choice")?;
    if choice.is_string() {
        return Some(choice.clone());
    }
    let function_name = choice
        .get("function")
        .and_then(|function| function.get("name"))
        .and_then(JsonValue::as_str)?;
    Some(serde_json::json!({
        "type": "function",
        "name": function_name
    }))
}

fn extract_responses_tool_calls(body: &JsonValue) -> Vec<JsonValue> {
    body.get("output")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(JsonValue::as_str) == Some("function_call"))
        .filter_map(|item| {
            let name = item.get("name").and_then(JsonValue::as_str)?;
            let arguments = item
                .get("arguments")
                .and_then(JsonValue::as_str)
                .unwrap_or("{}");
            let id = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(JsonValue::as_str)
                .unwrap_or("call_codex");
            Some(serde_json::json!({
                "id": id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": arguments,
                }
            }))
        })
        .collect()
}

fn extract_responses_text(body: &JsonValue) -> String {
    if let Some(text) = body.get("output_text").and_then(JsonValue::as_str) {
        return text.to_string();
    }
    let mut parts = Vec::new();
    for item in body
        .get("output")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
    {
        if item.get("type").and_then(JsonValue::as_str) != Some("message") {
            continue;
        }
        for content in item
            .get("content")
            .and_then(JsonValue::as_array)
            .into_iter()
            .flatten()
        {
            if matches!(
                content.get("type").and_then(JsonValue::as_str),
                Some("output_text" | "text")
            ) {
                if let Some(text) = content.get("text").and_then(JsonValue::as_str) {
                    parts.push(text.to_string());
                }
            }
        }
    }
    parts.join("")
}

fn flatten_content(value: &JsonValue) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(parts) = value.as_array() {
        return parts
            .iter()
            .filter_map(|part| {
                if let Some(text) = part.as_str() {
                    return Some(text.to_string());
                }
                part.get("text")
                    .and_then(JsonValue::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    String::new()
}

fn resolve_codex_access_token() -> Result<String> {
    if let Ok(token) = std::env::var("HERMES_CODEX_ACCESS_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    let path = auth_store_path();
    let raw = std::fs::read_to_string(&path).map_err(|err| ArdaError::Agent {
        agent: "charon".to_string(),
        message: format!("codex_responses could not read {}: {err}", path.display()),
    })?;
    let parsed: JsonValue = serde_json::from_str(&raw)?;
    if let Some(token) = parsed
        .get("providers")
        .and_then(|providers| providers.get("openai-codex"))
        .and_then(|state| state.get("tokens"))
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(JsonValue::as_str)
        .filter(|token| !token.trim().is_empty())
    {
        return Ok(token.to_string());
    }
    if let Some(token) = parsed
        .get("credential_pool")
        .and_then(|pool| pool.get("openai-codex"))
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("access_token").and_then(JsonValue::as_str))
        .find(|token| !token.trim().is_empty())
    {
        return Ok(token.to_string());
    }
    Err(ArdaError::Agent {
        agent: "charon".to_string(),
        message: "codex_responses found no OpenAI Codex access token in Hermes auth store"
            .to_string(),
    })
}

fn auth_store_path() -> PathBuf {
    if let Ok(path) = std::env::var("HERMES_AUTH_STORE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    if let Ok(home) = std::env::var("HERMES_HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home).join("auth.json");
        }
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".hermes")
        .join("auth.json")
}

fn chatgpt_account_id_from_jwt(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .ok()
        .or_else(|| {
            base64::engine::general_purpose::URL_SAFE
                .decode(payload.as_bytes())
                .ok()
        })?;
    let claims: JsonValue = serde_json::from_slice(&decoded).ok()?;
    claims
        .get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn error_preview(value: &JsonValue) -> String {
    if let Some(message) = value
        .get("error")
        .and_then(|error| error.get("message").or_else(|| error.get("code")))
    {
        if let Some(text) = message.as_str() {
            return text.chars().take(400).collect();
        }
        return message.to_string().chars().take(400).collect();
    }
    value
        .get("raw")
        .and_then(JsonValue::as_str)
        .unwrap_or("codex responses upstream error")
        .chars()
        .take(400)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::service::types::{ModelCapabilities, ModelState};

    fn provider() -> ProviderState {
        ProviderState {
            id: "openai_sub".to_string(),
            name: "OpenAI Codex".to_string(),
            base_url: Some("https://chatgpt.com/backend-api/codex".to_string()),
            api_key_env: None,
            access_tier: "paid_cloud".to_string(),
            quality_band: "high".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 0,
            requests_per_minute: None,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: None,
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                id: "gpt-5.5".to_string(),
                aliases: vec![],
                capable_tasks: vec!["chat".to_string()],
                context_window: 128_000,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: ModelCapabilities::default(),
                streaming_validated: None,
            }],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: false,
            driver: "codex_responses".to_string(),
            hermes_bin: None,
            hermes_provider: Some("openai-codex".to_string()),
            hermes_toolsets: None,
        }
    }

    #[test]
    fn converts_chat_messages_to_codex_responses_request() {
        std::env::set_var("HERMES_CODEX_ACCESS_TOKEN", "header.payload.signature");
        let req = build_codex_responses_request(
            &provider(),
            "gpt-5.5",
            &serde_json::json!({
                "messages": [
                    {"role": "system", "content": "System prompt"},
                    {"role": "user", "content": "Hello"}
                ],
                "extra_body": {"reasoning": {"effort": "minimal"}}
            }),
        )
        .expect("request");
        std::env::remove_var("HERMES_CODEX_ACCESS_TOKEN");

        assert_eq!(req.url, "https://chatgpt.com/backend-api/codex/responses");
        assert_eq!(req.body["model"], "gpt-5.5");
        assert_eq!(req.body["instructions"], "System prompt");
        assert_eq!(req.body["stream"], true);
        assert_eq!(req.body["input"][0]["content"], "Hello");
        assert_eq!(req.body["reasoning"]["effort"], "low");
    }

    #[test]
    fn preserves_structured_chat_content_for_codex_responses() {
        std::env::set_var("HERMES_CODEX_ACCESS_TOKEN", "header.payload.signature");
        let req = build_codex_responses_request(
            &provider(),
            "gpt-5.5",
            &serde_json::json!({
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hello"}]},
                    {"role": "assistant", "content": [{"type": "text", "text": "Hi"}]}
                ]
            }),
        )
        .expect("request");
        std::env::remove_var("HERMES_CODEX_ACCESS_TOKEN");

        assert_eq!(req.body["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(req.body["input"][0]["content"][0]["text"], "Hello");
        assert_eq!(req.body["input"][1]["content"][0]["type"], "output_text");
        assert_eq!(req.body["input"][1]["content"][0]["text"], "Hi");
    }

    #[test]
    fn maps_chat_tools_to_codex_responses_tools() {
        std::env::set_var("HERMES_CODEX_ACCESS_TOKEN", "header.payload.signature");
        let req = build_codex_responses_request(
            &provider(),
            "gpt-5.5",
            &serde_json::json!({
                "messages": [{"role": "user", "content": "read a file"}],
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "description": "Read a UTF-8 file",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"}
                            },
                            "required": ["path"]
                        }
                    }
                }],
                "tool_choice": {
                    "type": "function",
                    "function": {"name": "read_file"}
                }
            }),
        )
        .expect("request");
        std::env::remove_var("HERMES_CODEX_ACCESS_TOKEN");

        assert_eq!(req.body["tools"][0]["type"], "function");
        assert_eq!(req.body["tools"][0]["name"], "read_file");
        assert_eq!(req.body["tools"][0]["parameters"]["required"][0], "path");
        assert_eq!(req.body["tool_choice"]["name"], "read_file");
    }

    #[test]
    fn parses_codex_sse_text_deltas_into_response_body() {
        let body = codex_sse_text_to_responses_body(concat!(
            "event: response.output_text.delta\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"AN\"}\n\n",
            "event: response.output_text.delta\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"KH\"}\n\n",
            "event: response.completed\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2},\"output\":null}}\n\n",
        ))
        .expect("sse body");

        assert_eq!(body["id"], "resp_1");
        assert_eq!(body["output_text"], "ANKH");
        assert_eq!(body["output"][0]["content"][0]["text"], "ANKH");
    }

    #[test]
    fn parses_codex_sse_error_event() {
        let body = codex_sse_text_to_responses_body(concat!(
            "event: error\n",
            "data: {\"type\":\"error\",\"message\":\"model unavailable\",\"code\":\"bad_model\"}\n\n",
        ))
        .expect("sse body");

        assert_eq!(body["error"]["message"], "model unavailable");
    }

    #[test]
    fn converts_responses_output_to_chat_completion() {
        let out = responses_body_to_chat_completion(&serde_json::json!({
            "id": "resp_1",
            "model": "gpt-5.5",
            "output": [{
                "type": "message",
                "content": [{"type": "output_text", "text": "ok"}]
            }]
        }));
        assert_eq!(out["choices"][0]["message"]["content"], "ok");
        assert_eq!(out["_charon_driver"], "codex_responses");
    }

    #[test]
    fn converts_responses_function_calls_to_chat_tool_calls() {
        let out = responses_body_to_chat_completion(&serde_json::json!({
            "id": "resp_1",
            "model": "gpt-5.5",
            "output": [{
                "type": "function_call",
                "call_id": "call_abc",
                "name": "read_file",
                "arguments": "{\"path\":\"/tmp/x\"}"
            }]
        }));

        assert_eq!(out["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(
            out["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "read_file"
        );
        assert_eq!(
            out["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"],
            "{\"path\":\"/tmp/x\"}"
        );
    }
}