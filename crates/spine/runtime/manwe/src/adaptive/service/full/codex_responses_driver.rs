use super::responses_messages::split_messages_for_responses;
use super::{ArdaError, JsonValue, ProviderState, Result};
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
    let request_body = chat_body_to_responses_body(model_id, body)?;
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
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            // A deadline or broken response body is not malformed model output.
            // Do not include reqwest's URL-bearing error in operator evidence.
            let (status, kind, message) = if error.is_timeout() {
                (
                    504,
                    "codex_responses_transport_timeout",
                    "Responses body read timed out",
                )
            } else {
                (
                    502,
                    "codex_responses_transport_error",
                    "Responses body read failed",
                )
            };
            return CodexResponsesOutcome {
                status,
                error: Some(message.to_owned()),
                response: serde_json::json!({
                    "error": {"message": message, "type": kind},
                    "_charon_driver": "codex_responses",
                }),
            };
        }
    };
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
        // Responses uses upstream SSE, buffered here through terminal completion.
        // The generic chat deadline is too short for multi-step tool reports.
        .unwrap_or_else(|| default_codex_responses_timeout(default_timeout))
}

fn default_codex_responses_timeout(default_timeout: Duration) -> Duration {
    default_timeout.max(Duration::from_secs(120))
}

fn chat_body_to_responses_body(model_id: &str, body: &JsonValue) -> Result<JsonValue> {
    let (instructions, input) = split_messages_for_responses(body)?;
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
    Ok(payload)
}

#[cfg(test)]
mod framing_regressions {
    use super::*;
    #[test]
    fn responses_normalization_preserves_distinct_linkage() {
        let mut body = serde_json::json!({"messages":[
            {"role":"assistant", "tool_calls":[
                {"id":"call_a", "function":{"name":"one", "arguments":"{}"}},
                {"id":"tc0000001", "function":{"name":"two", "arguments":"{}"}}]},
            {"role":"tool", "tool_call_id":"call_a", "content":"first"},
            {"role":"tool", "tool_call_id":"tc0000001", "content":"second"}
        ]});
        super::super::normalize_openai_request_payload_with_policy(&mut body, false);
        let calls = &body["messages"][0]["tool_calls"];
        assert_ne!(calls[0]["id"], calls[1]["id"]);
        assert_eq!(calls[0]["id"], body["messages"][1]["tool_call_id"]);
        assert_eq!(calls[1]["id"], body["messages"][2]["tool_call_id"]);
    }
    #[test]
    fn responses_default_deadline_preserves_larger_budgets() {
        assert_eq!(
            default_codex_responses_timeout(Duration::from_secs(35)),
            Duration::from_secs(120)
        );
        assert_eq!(
            default_codex_responses_timeout(Duration::from_secs(240)),
            Duration::from_secs(240)
        );
    }

    #[tokio::test]
    async fn body_read_timeout_is_not_a_parser_failure() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            socket.read(&mut request).await.unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 1000\r\n\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        let response = reqwest::Client::new()
            .get(format!("http://{address}"))
            .timeout(Duration::from_millis(150))
            .send()
            .await
            .unwrap();
        let outcome = response_to_codex_outcome(response).await;
        server.abort();
        assert_eq!(outcome.status, 504);
        assert_eq!(
            outcome.response["error"]["type"],
            "codex_responses_transport_timeout"
        );
        assert!(!outcome
            .error
            .unwrap()
            .contains("Unrecognized Responses payload"));
    }

    #[test]
    fn responses_incomplete_stream_fails_closed() {
        let event = serde_json::json!({"type":"response.incomplete", "response":{
            "status":"incomplete", "output":[{"type":"function_call", "call_id":"partial", "name":"read", "arguments":"{"}]}});
        assert!(parse_codex_response_text(&format!("data: {event}\n\n"))
            .get("error")
            .is_some());
    }
    #[test]
    fn terminal_snapshot_with_crlf_preserves_output() {
        let event = serde_json::json!({"type":"response.completed", "response":{
            "status":"completed", "output":[{"type":"message","role":"assistant",
            "content":[{"type":"output_text","text":"review result"}]}]}});
        let parsed = parse_codex_response_text(&format!("data: {event}\r\n\r\n"));
        assert_eq!(parsed["output"][0]["content"][0]["text"], "review result");
        assert!(parse_codex_response_text("garbage").get("error").is_some());
        assert_eq!(
            responses_body_to_chat_completion(&parsed)["choices"][0]["message"]["content"],
            "review result"
        );
        let delta = serde_json::json!({"type":"response.output_text.delta", "delta":"review"});
        let partial = parse_codex_response_text(&format!("data: {delta}\n\ndata: {event}\n\n"));
        assert_eq!(
            responses_body_to_chat_completion(&partial)["choices"][0]["message"]["content"],
            "review result"
        );
    }
}

fn parse_codex_response_text(text: &str) -> JsonValue {
    if let Ok(parsed) = serde_json::from_str::<JsonValue>(text) {
        return parsed;
    }
    if let Some(parsed) = codex_sse_text_to_responses_body(text) {
        return parsed;
    }
    serde_json::json!({"error": {"type": "invalid_response", "message": "Unrecognized Responses payload"}})
}

fn codex_sse_text_to_responses_body(text: &str) -> Option<JsonValue> {
    let normalized = text.replace("\r\n", "\n");
    let text = normalized.as_str();
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
                // Some endpoints emit only the terminal snapshot, not item deltas.
                if let Some(items) = response.get("output").and_then(JsonValue::as_array) {
                    if !items.is_empty() {
                        output_items = items.clone();
                        // Terminal output is authoritative over absent/partial deltas.
                        output_text = extract_responses_text(&serde_json::json!({"output": items}));
                    }
                }
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
    if status == "incomplete" {
        return Some(serde_json::json!({"error": {
            "type": "codex_responses_incomplete",
            "message": "Codex Responses stream is incomplete; tool calls withheld"
        }}));
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
    if !saw_terminal || (output_items.is_empty() && output_text.is_empty()) {
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
    let reasoning: Vec<_> = body
        .get("output")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item["type"] == "reasoning" && item["encrypted_content"].is_string())
        .cloned()
        .collect();
    if !reasoning.is_empty() {
        message["codex_reasoning_items"] = JsonValue::Array(reasoning);
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
    (!mapped.is_empty()).then_some(JsonValue::Array(mapped))
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

fn resolve_codex_access_token() -> Result<String> {
    if let Ok(token) = std::env::var("HERMES_CODEX_ACCESS_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    let path = auth_store_path();
    let raw = std::fs::read_to_string(&path).map_err(|err| ArdaError::Agent {
        agent: "manwe".to_string(),
        message: format!("codex_responses could not read {}: {err}", path.display()),
    })?;
    let parsed: JsonValue = serde_json::from_str(&raw)?;
    select_codex_access_token(&parsed, chrono::Utc::now().timestamp())
        .map(str::to_owned)
        .ok_or_else(|| ArdaError::Agent {
            agent: "manwe".to_string(),
            message: "codex_responses found no usable OpenAI Codex credential; refresh Hermes auth"
                .to_string(),
        })
}

// Read-only recovery honors the pool's explicit reset deadline. Without one,
// Hermes must refresh the row; do not invent a TTL or revive terminal states.
fn codex_pool_entry_available(entry: &JsonValue, now: i64) -> bool {
    match entry["last_status"].as_str() {
        None | Some("ok") => true,
        Some("exhausted") => {
            let value = &entry["last_error_reset_at"];
            let numeric = value
                .as_f64()
                .or_else(|| value.as_str()?.trim().parse::<f64>().ok());
            let reset = numeric.or_else(|| {
                chrono::DateTime::parse_from_rfc3339(value.as_str()?.trim())
                    .ok()
                    .map(|time| time.timestamp_millis() as f64 / 1000.0)
            });
            reset.is_some_and(|reset| {
                let seconds = if reset > 1_000_000_000_000.0 {
                    reset / 1000.0
                } else {
                    reset
                };
                seconds.is_finite() && seconds > 0.0 && seconds <= now as f64
            })
        }
        _ => false,
    }
}

// Hermes's credential pool is authoritative when present. Never fall back to
// legacy state after a pool credential is revoked, exhausted or refreshed.
fn select_codex_access_token(store: &JsonValue, now: i64) -> Option<&str> {
    let usable = |token: &str| {
        if token.trim().is_empty() {
            return false;
        }
        let expiry = token
            .split('.')
            .nth(1)
            .and_then(|part| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(part)
                    .ok()
            })
            .and_then(|bytes| serde_json::from_slice::<JsonValue>(&bytes).ok())
            .and_then(|claims| claims["exp"].as_i64());
        expiry.is_none_or(|expiry| expiry > now + 30)
    };
    if let Some(pool) = store
        .get("credential_pool")
        .and_then(|p| p.get("openai-codex"))
    {
        return pool
            .as_array()?
            .iter()
            .filter(|entry| codex_pool_entry_available(entry, now))
            .filter(|entry| entry["disabled"] != true)
            .filter_map(|entry| {
                let token = entry["access_token"].as_str()?;
                usable(token).then_some((entry["priority"].as_i64().unwrap_or(i64::MAX), token))
            })
            .min_by_key(|(priority, _)| *priority)
            .map(|(_, token)| token);
    }
    store
        .pointer("/providers/openai-codex/tokens/access_token")
        .and_then(JsonValue::as_str)
        .filter(|token| usable(token))
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
    #[test]
    fn converts_chat_messages_to_codex_responses_request() {
        let req = chat_body_to_responses_body(
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

        assert_eq!(req["model"], "gpt-5.5");
        assert_eq!(req["instructions"], "System prompt");
        assert_eq!(req["stream"], true);
        assert_eq!(req["input"][0]["content"], "Hello");
        assert_eq!(req["reasoning"]["effort"], "low");
    }

    #[test]
    fn preserves_structured_chat_content_for_codex_responses() {
        let req = chat_body_to_responses_body(
            "gpt-5.5",
            &serde_json::json!({
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hello"}]},
                    {"role": "assistant", "content": [{"type": "text", "text": "Hi"}]}
                ]
            }),
        )
        .expect("request");

        assert_eq!(req["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(req["input"][0]["content"][0]["text"], "Hello");
        assert_eq!(req["input"][1]["content"][0]["type"], "output_text");
        assert_eq!(req["input"][1]["content"][0]["text"], "Hi");
    }

    #[test]
    fn maps_chat_tools_to_codex_responses_tools() {
        let req = chat_body_to_responses_body(
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

        assert_eq!(req["tools"][0]["type"], "function");
        assert_eq!(req["tools"][0]["name"], "read_file");
        assert_eq!(req["tools"][0]["parameters"]["required"][0], "path");
        assert_eq!(req["tool_choice"]["name"], "read_file");
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

    #[test]
    fn responses_tool_round_trip_preserves_linkage_and_reasoning() {
        let reasoning = serde_json::json!({"type":"reasoning", "id":"rs_fixture", "encrypted_content":"opaque", "summary":[]});
        let completion = responses_body_to_chat_completion(&serde_json::json!({
            "output":[reasoning, {"type":"function_call", "call_id":"call_fixture", "name":"readiness", "arguments":"{}"}]
        }));
        let mut next = serde_json::json!({"messages":[
            {"role":"user", "content":"Check readiness"},
            completion["choices"][0]["message"],
            {"role":"tool", "tool_call_id":"call_fixture", "content":"ready"}
        ]});
        super::super::normalize_openai_request_payload_with_policy(&mut next, false);
        let body = chat_body_to_responses_body("configured-model", &next).unwrap();
        assert_eq!(body["input"][1], reasoning);
        assert_eq!(body["input"][2]["type"], "function_call");
        assert_eq!(body["input"][2]["call_id"], body["input"][3]["call_id"]);
        assert_eq!(body["input"][3]["type"], "function_call_output");
        assert_eq!(body["input"][3]["output"], "ready");
    }
}

#[cfg(test)]
mod credential_selection_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn refreshed_pool_precedes_legacy_and_respects_unavailable_entries() {
        let mut store = json!({"providers":{"openai-codex":{"tokens":{"access_token":"legacy"}}},
            "credential_pool":{"openai-codex":[{"priority":0,"last_status":"ok","access_token":"fresh"}]}});
        assert_eq!(select_codex_access_token(&store, 100), Some("fresh"));
        store["credential_pool"]["openai-codex"][0]["last_status"] = json!("exhausted");
        assert_eq!(select_codex_access_token(&store, 100), None);
        store["credential_pool"]["openai-codex"] = json!([]);
        assert_eq!(select_codex_access_token(&store, 100), None);
        store.as_object_mut().unwrap().remove("credential_pool");
        assert_eq!(select_codex_access_token(&store, 100), Some("legacy"));
    }
    #[test]
    fn authoritative_elapsed_exhaustion_recovers_without_reviving_dead_entries() {
        let now = 1_800_000_000;
        let mut store = json!({"credential_pool":{"openai-codex":[
            {"priority":0,"last_status":"exhausted","access_token":"recovered","last_error_reset_at":now-1},
            {"priority":5,"last_status":"ok","access_token":"fallback"}
        ]}});
        assert_eq!(select_codex_access_token(&store, now), Some("recovered"));
        for reset in [
            json!(now),
            json!((now - 1) * 1000_i64),
            json!((now - 1).to_string()),
            json!(chrono::DateTime::from_timestamp(now - 1, 0)
                .unwrap()
                .to_rfc3339()),
        ] {
            store["credential_pool"]["openai-codex"][0]["last_error_reset_at"] = reset;
            assert_eq!(select_codex_access_token(&store, now), Some("recovered"));
        }
        for reset in [json!(now + 1), json!(null), json!("bad"), json!(-1)] {
            store["credential_pool"]["openai-codex"][0]["last_error_reset_at"] = reset;
            assert_eq!(select_codex_access_token(&store, now), Some("fallback"));
        }
        store["credential_pool"]["openai-codex"][0]["last_error_reset_at"] = json!(now - 1);
        store["credential_pool"]["openai-codex"][0]["last_status"] = json!("dead");
        assert_eq!(select_codex_access_token(&store, now), Some("fallback"));
        store["credential_pool"]["openai-codex"][0]["last_status"] = json!("exhausted");
        store["credential_pool"]["openai-codex"][0]["disabled"] = json!(true);
        assert_eq!(select_codex_access_token(&store, now), Some("fallback"));
    }
    #[test]
    fn credential_expiry_safety_boundary() {
        for (expiry, accepted) in [(130, false), (131, true)] {
            let jwt = format!(
                "fixture.{}.signature",
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(json!({"exp":expiry}).to_string())
            );
            let store = json!({"credential_pool":{"openai-codex":[{"access_token":jwt}]}});
            assert_eq!(select_codex_access_token(&store, 100).is_some(), accepted);
        }
    }
    #[test]
    fn expired_credentials_are_not_sent() {
        let jwt = format!(
            "fixture.{}.signature",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp":100}"#)
        );
        let store = json!({"providers":{"openai-codex":{"tokens":{"access_token":jwt}}}});
        assert_eq!(select_codex_access_token(&store, 100), None);
    }
}
