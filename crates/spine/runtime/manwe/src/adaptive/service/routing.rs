use crate::adaptive::service::route_policy::is_local_provider;
use crate::adaptive::service::types::RouteDecision;
use serde_json::Value as JsonValue;

pub(crate) fn strip_internal_openai_routing_fields(
    payload: &mut serde_json::Map<String, JsonValue>,
) {
    for key in [
        "agent_id",
        "source_agent",
        "routing",
        "workload_role",
        "context_priority",
        "quality_priority",
        "quality_tier",
        "cost_policy",
        "cost_tier",
        "privacy_requirement",
        "inference_origin",
        "origin_preference",
        "context_window_target",
        "execution_lane",
        "force_provider_id",
        "force_model_id",
        "dry_run",
        "exclude_provider_ids",
        "exclude_model_ids",
        "prefer_probe_model",
        "source_surface",
        "harness",
        "session_id",
        "conversation_id",
        "turn_id",
        "trace_id",
        "receipt_id",
        "skill",
        "skills",
        "toolset",
        "toolsets",
        "tool_mode",
        "agent_mode",
        "tool_use_required",
        "transform",
    ] {
        payload.remove(key);
    }
}

#[cfg(test)]
pub(crate) fn normalize_openai_request_payload(payload: &mut JsonValue) {
    normalize_openai_request_payload_with_policy(payload, false);
}

pub(crate) fn normalize_openai_request_payload_with_policy(
    payload: &mut JsonValue,
    preserve_reasoning_replay: bool,
) {
    let Some(messages) = payload
        .get_mut("messages")
        .and_then(JsonValue::as_array_mut)
    else {
        return;
    };

    let mut normalized_tool_call_ids = std::collections::BTreeMap::new();
    let mut next_tool_call_ordinal: u32 = 1;

    for message in messages.iter_mut() {
        let Some(msg_obj) = message.as_object_mut() else {
            continue;
        };
        if msg_obj.get("role").and_then(JsonValue::as_str) != Some("assistant") {
            continue;
        }
        if !preserve_reasoning_replay {
            msg_obj.remove("reasoning_content");
            msg_obj.remove("reasoning_details");
        }
        let Some(tool_calls) = msg_obj
            .get_mut("tool_calls")
            .and_then(JsonValue::as_array_mut)
        else {
            continue;
        };
        for tool_call in tool_calls.iter_mut() {
            let Some(tool_obj) = tool_call.as_object_mut() else {
                continue;
            };
            if let Some(existing_id) = tool_obj.get("id").and_then(JsonValue::as_str) {
                let normalized_id = normalize_tool_call_id(
                    existing_id,
                    &mut normalized_tool_call_ids,
                    &mut next_tool_call_ordinal,
                );
                if normalized_id != existing_id {
                    tool_obj.insert("id".to_string(), JsonValue::String(normalized_id));
                }
            }
            let has_function = tool_obj
                .get("function")
                .and_then(JsonValue::as_object)
                .is_some();
            let missing_type = tool_obj.get("type").is_none()
                || tool_obj.get("type").is_some_and(JsonValue::is_null);
            if has_function && missing_type {
                tool_obj.insert(
                    "type".to_string(),
                    JsonValue::String("function".to_string()),
                );
            }
        }
    }

    if normalized_tool_call_ids.is_empty() {
        return;
    }

    for message in messages.iter_mut() {
        let Some(msg_obj) = message.as_object_mut() else {
            continue;
        };
        if msg_obj.get("role").and_then(JsonValue::as_str) != Some("tool") {
            continue;
        }
        let Some(existing_id) = msg_obj.get("tool_call_id").and_then(JsonValue::as_str) else {
            continue;
        };
        if let Some(normalized_id) = normalized_tool_call_ids.get(existing_id) {
            msg_obj.insert(
                "tool_call_id".to_string(),
                JsonValue::String(normalized_id.clone()),
            );
        }
    }
}

fn normalize_tool_call_id(
    value: &str,
    normalized_ids: &mut std::collections::BTreeMap<String, String>,
    next_ordinal: &mut u32,
) -> String {
    if let Some(existing) = normalized_ids.get(value) {
        return existing.clone();
    }

    let normalized = if value.len() == 9 && value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        value.to_string()
    } else {
        let mut candidate = format!("tc{:07}", *next_ordinal);
        while normalized_ids
            .values()
            .any(|existing| existing == &candidate)
        {
            *next_ordinal += 1;
            candidate = format!("tc{:07}", *next_ordinal);
        }
        *next_ordinal += 1;
        candidate
    };

    normalized_ids.insert(value.to_string(), normalized.clone());
    normalized
}

pub(crate) fn normalize_openai_response(response: &mut JsonValue) {
    let Some(choices) = response
        .get_mut("choices")
        .and_then(JsonValue::as_array_mut)
    else {
        return;
    };

    let mut next_tool_call_ordinal: u32 = 1;
    for choice in choices {
        let has_tool_calls = {
            let Some(message) = choice.get_mut("message").and_then(JsonValue::as_object_mut) else {
                continue;
            };
            normalize_response_tool_calls(message, &mut next_tool_call_ordinal);
            let has_tool_calls = message
                .get("tool_calls")
                .and_then(JsonValue::as_array)
                .is_some_and(|tool_calls| !tool_calls.is_empty());

            if message
                .get("content")
                .and_then(JsonValue::as_str)
                .is_some_and(|content| content.trim().is_empty())
            {
                if let Some(reasoning) = message
                    .get("reasoning_content")
                    .and_then(JsonValue::as_str)
                    .map(str::trim)
                    .filter(|reasoning| !reasoning.is_empty())
                {
                    message.insert(
                        "content".to_string(),
                        JsonValue::String(reasoning.to_string()),
                    );
                }
            }

            if let Some(content) = message.get("content").and_then(JsonValue::as_str) {
                let normalized = strip_visible_think_block(content);
                if normalized != content {
                    message.insert(
                        "content".to_string(),
                        JsonValue::String(normalized.to_string()),
                    );
                }
            }
            has_tool_calls
        };

        if has_tool_calls
            && choice
                .get("finish_reason")
                .and_then(JsonValue::as_str)
                .is_some_and(|reason| matches!(reason, "function_call" | "stop"))
        {
            choice["finish_reason"] = JsonValue::String("tool_calls".to_string());
        }
    }
}

fn strip_visible_think_block(content: &str) -> &str {
    let trimmed_start = content.trim_start();
    if let Some(rest) = trimmed_start.strip_prefix("</think>") {
        return rest.trim_start();
    }

    let Some(after_open) = trimmed_start.strip_prefix("<think>") else {
        return content;
    };
    let Some((_, after_close)) = after_open.split_once("</think>") else {
        return "";
    };
    after_close.trim_start()
}

fn normalize_response_tool_calls(
    message: &mut serde_json::Map<String, JsonValue>,
    next_ordinal: &mut u32,
) {
    if message.get("tool_calls").is_none() {
        if let Some(function_call) = message.remove("function_call") {
            if let Some(tool_call) = legacy_function_call_to_tool_call(function_call, next_ordinal)
            {
                message.insert("tool_calls".to_string(), JsonValue::Array(vec![tool_call]));
            }
        }
    } else {
        message.remove("function_call");
    }

    let Some(tool_calls) = message
        .get_mut("tool_calls")
        .and_then(JsonValue::as_array_mut)
    else {
        return;
    };

    for tool_call in tool_calls.iter_mut() {
        let Some(tool_obj) = tool_call.as_object_mut() else {
            continue;
        };
        if tool_obj
            .get("id")
            .and_then(JsonValue::as_str)
            .is_none_or(|id| id.trim().is_empty())
        {
            tool_obj.insert(
                "id".to_string(),
                JsonValue::String(next_response_tool_call_id(next_ordinal)),
            );
        }
        if tool_obj
            .get("type")
            .and_then(JsonValue::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            tool_obj.insert(
                "type".to_string(),
                JsonValue::String("function".to_string()),
            );
        }

        if tool_obj.get("function").is_none() {
            let mut function = serde_json::Map::new();
            if let Some(name) = tool_obj.remove("name").and_then(|value| match value {
                JsonValue::String(name) => Some(name),
                _ => None,
            }) {
                function.insert("name".to_string(), JsonValue::String(name));
            }
            if let Some(arguments) = tool_obj.remove("arguments") {
                function.insert(
                    "arguments".to_string(),
                    JsonValue::String(normalize_tool_arguments(arguments)),
                );
            }
            if !function.is_empty() {
                tool_obj.insert("function".to_string(), JsonValue::Object(function));
            }
        }

        if let Some(function) = tool_obj
            .get_mut("function")
            .and_then(JsonValue::as_object_mut)
        {
            let embedded_arguments = function
                .get("name")
                .and_then(JsonValue::as_str)
                .and_then(split_embedded_tool_arguments);
            if let Some((name, arguments)) = embedded_arguments {
                let arguments_missing_or_empty = function
                    .get("arguments")
                    .and_then(JsonValue::as_str)
                    .is_none_or(|arguments| {
                        let trimmed = arguments.trim();
                        trimmed.is_empty() || trimmed == "{}"
                    });
                function.insert("name".to_string(), JsonValue::String(name));
                if arguments_missing_or_empty {
                    function.insert("arguments".to_string(), JsonValue::String(arguments));
                }
            }
            if function
                .get("arguments")
                .and_then(JsonValue::as_str)
                .is_none()
            {
                let arguments = function
                    .remove("arguments")
                    .map(normalize_tool_arguments)
                    .unwrap_or_else(|| "{}".to_string());
                function.insert("arguments".to_string(), JsonValue::String(arguments));
            }
        }
    }
}

fn legacy_function_call_to_tool_call(
    function_call: JsonValue,
    next_ordinal: &mut u32,
) -> Option<JsonValue> {
    let function_obj = function_call.as_object()?;
    let name = function_obj
        .get("name")
        .and_then(JsonValue::as_str)
        .filter(|name| !name.trim().is_empty())?;
    let embedded_arguments = split_embedded_tool_arguments(name);
    let normalized_name = embedded_arguments
        .as_ref()
        .map(|(name, _)| name.as_str())
        .unwrap_or(name);
    let arguments = function_obj
        .get("arguments")
        .cloned()
        .map(normalize_tool_arguments)
        .unwrap_or_else(|| {
            embedded_arguments
                .as_ref()
                .map(|(_, arguments)| arguments.clone())
                .unwrap_or_else(|| "{}".to_string())
        });
    Some(serde_json::json!({
        "id": next_response_tool_call_id(next_ordinal),
        "type": "function",
        "function": {
            "name": normalized_name,
            "arguments": arguments
        }
    }))
}

fn split_embedded_tool_arguments(name: &str) -> Option<(String, String)> {
    let trimmed = name.trim();
    let split_at = trimmed.char_indices().find_map(|(index, ch)| {
        if ch.is_whitespace() || ch == '{' || ch == '[' {
            Some(index)
        } else {
            None
        }
    })?;
    let (tool_name, arguments) = trimmed.split_at(split_at);
    let tool_name = tool_name.trim();
    let arguments = arguments.trim();
    if tool_name.is_empty() || !matches!(arguments.as_bytes().first(), Some(b'{') | Some(b'[')) {
        return None;
    }
    if serde_json::from_str::<JsonValue>(arguments).is_err() {
        return None;
    }
    Some((tool_name.to_string(), arguments.to_string()))
}

fn normalize_tool_arguments(arguments: JsonValue) -> String {
    match arguments {
        JsonValue::String(value) => value,
        JsonValue::Null => "{}".to_string(),
        other => serde_json::to_string(&other).unwrap_or_else(|_| "{}".to_string()),
    }
}

fn next_response_tool_call_id(next_ordinal: &mut u32) -> String {
    let id = format!("call_charon_{:06}", *next_ordinal);
    *next_ordinal += 1;
    id
}

pub(crate) fn attach_charon_route_metadata(
    response: &mut JsonValue,
    decision: &RouteDecision,
    provider_id: &str,
    latency_ms: u64,
) {
    let Some(obj) = response.as_object_mut() else {
        return;
    };
    obj.insert(
        "_charon_route".to_string(),
        serde_json::json!({
            "provider_id": provider_id,
            "model_id": decision.model_id,
            "execution_lane": decision.execution_lane,
            "route_class": decision.route_class,
            "context_window_target": decision.context_window_target,
            "latency_ms": latency_ms,
            "route_id": decision.route_id,
        }),
    );
}

pub(crate) fn proxy_max_attempts(provider_count: usize) -> usize {
    provider_count.clamp(5, 14)
}

pub(crate) fn provider_error_should_fallback(status_u16: u16, parsed: &JsonValue) -> bool {
    if status_u16 >= 500 || matches!(status_u16, 408 | 413 | 429) {
        return true;
    }

    let body = parsed.to_string().to_lowercase();
    if is_billing_or_credit_body(&body) {
        return true;
    }

    if body.contains("mesh request")
        || body.contains("tunnel(s) to hosts")
        || body.contains("hosts for none failed")
    {
        return true;
    }

    if status_u16 == 400
        && (body.contains("not currently available")
            || body.contains("model is not supported")
            || body.contains("model not supported")
            || is_context_overflow_body(&body))
    {
        return true;
    }

    if is_reasoning_replay_required_error(status_u16, parsed) {
        return true;
    }

    if status_u16 == 404
        && ["function", "not found for account", "\"detail\""]
            .iter()
            .all(|needle| body.contains(needle))
    {
        return true;
    }

    if status_u16 == 404
        && (body.contains("model_not_found")
            || body.contains("does not exist")
            || body.contains("no such model")
            || body.contains("unknown model")
            || (body.contains("not found") && body.contains("model"))
            || (body.contains("not found") && body.contains("available")))
    {
        return true;
    }

    if matches!(status_u16, 401..=403) {
        return [
            "requires more credits",
            "check credits",
            "can only afford",
            "fewer max_tokens",
            "quota",
            "insufficient credits",
            "insufficient balance",
            "creditserror",
            "billing",
            "out of credits",
            "payment required",
            "resource exhausted",
            "rate limit exceeded",
            "daily limit",
            "spend limit",
        ]
        .iter()
        .any(|needle| body.contains(needle));
    }

    false
}

pub(crate) fn is_context_overflow_error(status_u16: u16, parsed: &JsonValue) -> bool {
    matches!(status_u16, 400 | 413 | 500)
        && is_context_overflow_body(&parsed.to_string().to_lowercase())
}

pub(crate) fn is_billing_or_credit_error(status_u16: u16, parsed: &JsonValue) -> bool {
    matches!(status_u16, 400..=403 | 429)
        && is_billing_or_credit_body(&parsed.to_string().to_lowercase())
}

pub(crate) fn is_request_scoped_quota_error(status_u16: u16, parsed: &JsonValue) -> bool {
    if !matches!(status_u16, 413 | 429) {
        return false;
    }
    let body = parsed.to_string().to_lowercase();
    is_request_scoped_quota_body(&body) && !is_billing_or_credit_body(&body)
}

pub(crate) fn is_request_scoped_retry_error(status_u16: u16, parsed: &JsonValue) -> bool {
    is_request_scoped_quota_error(status_u16, parsed)
        || (status_u16 == 429
            && !is_billing_or_credit_error(status_u16, parsed)
            && !is_plain_rate_limit_error(parsed))
        || is_reasoning_replay_required_error(status_u16, parsed)
}

pub(crate) fn is_reasoning_replay_required_error(status_u16: u16, parsed: &JsonValue) -> bool {
    if !matches!(status_u16, 400 | 422) {
        return false;
    }
    let body = parsed.to_string().to_lowercase();
    body.contains("reasoning_content")
        && (body.contains("must be passed back")
            || body.contains("must be passed")
            || body.contains("thinking mode"))
}

fn is_billing_or_credit_body(body: &str) -> bool {
    [
        "requires more credits",
        "check credits",
        "can only afford",
        "insufficient credits",
        "insufficient balance",
        "no resource package",
        "please recharge",
        "creditserror",
        "billing",
        "out of credits",
        "payment required",
        "resource exhausted",
        "spend limit",
    ]
    .iter()
    .any(|needle| body.contains(needle))
}

fn is_request_scoped_quota_body(body: &str) -> bool {
    [
        "token_quota_exceeded",
        "tokens per minute",
        " tpm",
        "(tpm)",
        "too many tokens processed",
        "request too large for model",
        "requested",
    ]
    .iter()
    .any(|needle| body.contains(needle))
}

fn is_plain_rate_limit_error(parsed: &JsonValue) -> bool {
    let body = parsed.to_string().to_lowercase();
    let error_type = parsed
        .get("type")
        .or_else(|| parsed.pointer("/error/type"))
        .and_then(JsonValue::as_str)
        .map(str::to_ascii_lowercase);
    let error_code = parsed
        .get("code")
        .or_else(|| parsed.pointer("/error/code"))
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string())
        })
        .unwrap_or_default()
        .to_ascii_lowercase()
        .trim_matches('"')
        .to_string();

    matches!(
        error_type.as_deref(),
        Some("rate_limited" | "rate_limit_exceeded")
    ) || error_code == "1300"
        || ((body.contains("rate limit exceeded") || body.contains("rate limited"))
            && !is_request_scoped_quota_body(&body))
}

fn is_context_overflow_body(body: &str) -> bool {
    [
        "exceeds the available context size",
        "available context size",
        "context length",
        "context size",
        "maximum context",
        "max context",
        "context window",
        "too many tokens",
        "prompt is too long",
        "reduce the length",
    ]
    .iter()
    .any(|needle| body.contains(needle))
}

/// True when an HTTP failure indicates a CLIENT-side payload problem
/// (the provider is fine; our request is malformed/incompatible). These
/// must not advance the cooldown counter — a payload bug doesn't get
/// better by waiting.
pub(crate) fn is_client_payload_error(status_u16: u16, parsed: &JsonValue) -> bool {
    if !(400..500).contains(&status_u16) {
        return false;
    }
    if matches!(status_u16, 408 | 429) {
        return false;
    }
    if provider_error_should_fallback(status_u16, parsed) {
        return false;
    }
    true
}

pub(crate) fn provider_error_immediate_cooldown_seconds(
    provider_id: &str,
    status_u16: u16,
    parsed: &JsonValue,
) -> Option<i64> {
    if is_local_provider(provider_id) {
        return None;
    }

    if is_billing_or_credit_error(status_u16, parsed) {
        return Some(86_400);
    }

    if is_request_scoped_quota_error(status_u16, parsed) {
        return match status_u16 {
            413 => Some(900),
            429 => Some(300),
            _ => None,
        };
    }

    match status_u16 {
        408 => Some(60),
        429 if is_plain_rate_limit_error(parsed) => Some(300),
        429 => None,
        413 => Some(900),
        500..=599 => Some(120),
        401..=403 if provider_error_should_fallback(status_u16, parsed) => Some(1_800),
        404 if provider_error_should_fallback(status_u16, parsed) => Some(900),
        _ => None,
    }
}

pub(crate) fn slim_local_attempt_body(body: &mut JsonValue) {
    let Some(obj) = body.as_object_mut() else {
        return;
    };

    if let Some(messages) = obj.get_mut("messages").and_then(JsonValue::as_array_mut) {
        flatten_local_tool_history(messages);
        for message in messages.iter_mut() {
            let Some(msg_obj) = message.as_object_mut() else {
                continue;
            };
            let role = msg_obj
                .get("role")
                .and_then(JsonValue::as_str)
                .unwrap_or_default();
            if !matches!(role, "system" | "developer") {
                continue;
            }
            let Some(content) = msg_obj.get_mut("content") else {
                continue;
            };
            if let Some(text) = content.as_str() {
                let trimmed = slim_prompt_text(text, 8_000);
                if trimmed.len() != text.len() {
                    *content = JsonValue::String(trimmed);
                }
            }
        }
    }

    if let Some(tools) = obj.get_mut("tools").and_then(JsonValue::as_array_mut) {
        for tool in tools.iter_mut() {
            slim_tool_definition(tool);
        }
    }
}

pub(crate) fn local_payload_requires_structured_tool_history(body: &JsonValue) -> bool {
    body.get("tool_choice").is_some()
        || body
            .get("messages")
            .and_then(JsonValue::as_array)
            .is_some_and(|messages| {
                messages.iter().any(|message| {
                    message.get("role").and_then(JsonValue::as_str) == Some("tool")
                        || message
                            .get("tool_calls")
                            .and_then(JsonValue::as_array)
                            .is_some_and(|calls| !calls.is_empty())
                })
            })
}

fn flatten_local_tool_history(messages: &mut [JsonValue]) {
    for message in messages.iter_mut() {
        let Some(msg_obj) = message.as_object_mut() else {
            continue;
        };
        match msg_obj
            .get("role")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
        {
            "assistant" => {
                let tool_names = msg_obj
                    .get("tool_calls")
                    .and_then(JsonValue::as_array)
                    .map(|calls| {
                        calls
                            .iter()
                            .filter_map(|call| {
                                call.get("function")
                                    .and_then(JsonValue::as_object)
                                    .and_then(|function| function.get("name"))
                                    .and_then(JsonValue::as_str)
                                    .map(str::to_string)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !tool_names.is_empty() {
                    let existing = msg_obj
                        .get("content")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default();
                    let summary = format!(
                        "{}\n\n[Tool calls issued: {}]",
                        existing,
                        tool_names.join(", ")
                    );
                    msg_obj.insert(
                        "content".to_string(),
                        JsonValue::String(summary.trim().to_string()),
                    );
                    msg_obj.remove("tool_calls");
                }
            }
            "tool" => {
                let existing = msg_obj
                    .get("content")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default();
                let tool_id = msg_obj
                    .get("tool_call_id")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default();
                let flattened = if tool_id.is_empty() {
                    format!(
                        "[Tool result]\n{}",
                        truncate_local_tool_text(existing, 4000)
                    )
                } else {
                    format!(
                        "[Tool result for {}]\n{}",
                        tool_id,
                        truncate_local_tool_text(existing, 4000)
                    )
                };
                msg_obj.insert("role".to_string(), JsonValue::String("user".to_string()));
                msg_obj.insert("content".to_string(), JsonValue::String(flattened));
                msg_obj.remove("tool_call_id");
            }
            _ => {}
        }
    }
}

fn truncate_local_tool_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let head = safe_prefix(text, max_chars.saturating_sub(300));
    let tail = safe_suffix(text, 180);
    format!("{head}\n\n[tool output truncated]\n\n{tail}")
}

fn slim_prompt_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let head_chars = max_chars.saturating_sub(900);
    let tail_chars = 700.min(max_chars / 4);
    let head = safe_prefix(text, head_chars);
    let tail = safe_suffix(text, tail_chars);
    format!("{head}\n\n[CHARON local proxy trimmed oversized prompt for local inference]\n\n{tail}")
}

fn safe_prefix(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        return text;
    }
    let mut end = 0usize;
    for (idx, _) in text.char_indices() {
        if idx > max_chars {
            break;
        }
        end = idx;
    }
    if end == 0 {
        ""
    } else {
        &text[..end]
    }
}

fn safe_suffix(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        return text;
    }
    let target = text.len().saturating_sub(max_chars);
    let mut start = text.len();
    for (idx, _) in text.char_indices() {
        if idx >= target {
            start = idx;
            break;
        }
    }
    &text[start..]
}

fn slim_tool_definition(tool: &mut JsonValue) {
    let Some(tool_obj) = tool.as_object_mut() else {
        return;
    };
    let Some(function) = tool_obj
        .get_mut("function")
        .and_then(JsonValue::as_object_mut)
    else {
        return;
    };

    if let Some(description) = function.get_mut("description") {
        if let Some(text) = description.as_str() {
            *description = JsonValue::String(text.chars().take(180).collect());
        }
    }

    if let Some(parameters) = function.get_mut("parameters") {
        slim_json_schema(parameters);
    }
}

fn slim_json_schema(value: &mut JsonValue) {
    match value {
        JsonValue::Object(map) => {
            map.remove("description");
            map.remove("examples");
            map.remove("example");
            map.remove("default");
            map.remove("title");
            map.remove("$comment");
            for key in ["properties", "definitions", "$defs"] {
                if let Some(JsonValue::Object(props)) = map.get_mut(key) {
                    for child in props.values_mut() {
                        slim_json_schema(child);
                    }
                }
            }
            for key in [
                "items",
                "additionalProperties",
                "contains",
                "if",
                "then",
                "else",
            ] {
                if let Some(child) = map.get_mut(key) {
                    slim_json_schema(child);
                }
            }
            for key in ["oneOf", "anyOf", "allOf", "prefixItems"] {
                if let Some(JsonValue::Array(items)) = map.get_mut(key) {
                    for child in items.iter_mut() {
                        slim_json_schema(child);
                    }
                }
            }
        }
        JsonValue::Array(items) => {
            for item in items.iter_mut() {
                slim_json_schema(item);
            }
        }
        _ => {}
    }
}

pub(crate) fn model_error_should_mark_unavailable(status_u16: u16, parsed: &JsonValue) -> bool {
    let body = parsed.to_string().to_lowercase();
    (status_u16 == 400
        && (body.contains("not currently available")
            || body.contains("model is not supported")
            || body.contains("model not supported")))
        || (status_u16 >= 500
            && (body.contains("currently experiencing high demand")
                || body.contains("\"status\":\"unavailable\"")
                || body.contains("\"status\": \"unavailable\"")
                || body.contains("try again later")))
        || (status_u16 == 404
            && body.contains("function")
            && body.contains("not found for account"))
        || (status_u16 == 404
            && (body.contains("model_not_found")
                || body.contains("does not exist")
                || body.contains("no such model")
                || body.contains("unknown model")
                || (body.contains("not found") && body.contains("model"))
                || (body.contains("not found") && body.contains("available"))))
        || body.contains("mesh request")
        || body.contains("tunnel(s) to hosts")
        || body.contains("hosts for none failed")
}

pub(crate) fn transport_failure_should_trigger_cooldown(provider_id: &str, err_msg: &str) -> bool {
    let lowered = err_msg.to_ascii_lowercase();
    is_local_provider(provider_id)
        && [
            "connection refused",
            "error sending request",
            "tcp connect error",
            "dns error",
            "channel closed",
            "mesh request",
            "tunnel(s) to hosts",
            "hosts for none failed",
        ]
        .iter()
        .any(|needle| lowered.contains(needle))
}
