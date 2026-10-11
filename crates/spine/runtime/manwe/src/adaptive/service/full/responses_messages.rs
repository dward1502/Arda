use super::{ArdaError, JsonValue, Result};

pub(super) fn split_messages_for_responses(body: &JsonValue) -> Result<(String, JsonValue)> {
    let mut instructions = Vec::new();
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
        if matches!(role, "system" | "developer") {
            if !content.trim().is_empty() {
                instructions.push(content);
            }
            continue;
        }
        if role == "tool" {
            let call_id = required_string(message, "tool_call_id")?;
            input.push(serde_json::json!({
                "type": "function_call_output", "call_id": call_id, "output": content,
            }));
            continue;
        }
        let tool_calls = message.get("tool_calls").and_then(JsonValue::as_array);
        if role == "assistant" {
            if let Some(items) = message.get("codex_reasoning_items").and_then(JsonValue::as_array) {
                input.extend(items.iter().filter(|item| {
                    item["type"] == "reasoning" && item["encrypted_content"].is_string()
                }).cloned());
            }
        }
        let response_role = if role == "assistant" {
            "assistant"
        } else {
            "user"
        };
        let content_value = if raw_content.is_string() {
            JsonValue::String(content.clone())
        } else {
            let text_type = if response_role == "assistant" {
                "output_text"
            } else {
                "input_text"
            };
            serde_json::json!([{"type": text_type, "text": content}])
        };
        if !content.is_empty() || tool_calls.is_none_or(Vec::is_empty) {
            input.push(serde_json::json!({
                "role": response_role,
                "content": content_value,
            }));
        }
        if role == "assistant" {
            for call in tool_calls.into_iter().flatten() {
                let call_id = required_string(call, "id")?;
                let function = &call["function"];
                let name = required_string(function, "name")?;
                let arguments = required_string(function, "arguments")?;
                input.push(serde_json::json!({
                    "type": "function_call", "call_id": call_id,
                    "name": name, "arguments": arguments,
                }));
            }
        }
    }
    if input.is_empty() {
        input.push(serde_json::json!({
            "role": "user",
            "content": [{"type": "input_text", "text": ""}],
        }));
    }
    let instructions = if instructions.is_empty() {
        "You are a helpful assistant.".to_string()
    } else {
        instructions.join("\n\n")
    };
    Ok((instructions, JsonValue::Array(input)))
}

fn required_string<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str> {
    value.get(key).and_then(JsonValue::as_str).filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ArdaError::Task(format!("Responses tool history requires {key}")))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn responses_preserves_parallel_calls_and_out_of_order_results() {
        let (_, input) = split_messages_for_responses(&json!({"messages": [
            {"role":"user", "content":"Check both"},
            {"role":"assistant", "content":null, "tool_calls":[
                {"id":"call_a", "function":{"name":"read", "arguments":"{\"path\":\"a\"}"}},
                {"id":"call_b", "function":{"name":"read", "arguments":"{\"path\":\"b\"}"}}
            ]},
            {"role":"tool", "tool_call_id":"call_b", "content":"B"},
            {"role":"tool", "tool_call_id":"call_a", "content":"A"}
        ]})).unwrap();
        assert_eq!(input.as_array().unwrap().len(), 5);
        assert_eq!(input[1], json!({"type":"function_call", "call_id":"call_a", "name":"read", "arguments":"{\"path\":\"a\"}"}));
        assert_eq!(input[2]["call_id"], "call_b");
        assert_eq!(input[3], json!({"type":"function_call_output", "call_id":"call_b", "output":"B"}));
        assert_eq!(input[4]["call_id"], "call_a");
    }

    #[test]
    fn responses_refuses_missing_tool_linkage() {
        for message in [
            json!({"role":"tool", "content":"unlinked"}),
            json!({"role":"assistant", "tool_calls":[{"function":{"name":"read","arguments":"{}"}}]}),
        ] {
            assert!(split_messages_for_responses(&json!({"messages":[message]})).is_err());
        }
    }

    #[test]
    fn responses_preserves_instructions_and_opaque_reasoning() {
        let reasoning = json!({"type":"reasoning", "id":"rs_fixture", "encrypted_content":"opaque", "summary":[]});
        let (instructions, input) = split_messages_for_responses(&json!({"messages":[
            {"role":"system", "content":"Read only."},
            {"role":"developer", "content":"Preserve staged work."},
            {"role":"assistant", "content":"Checking.", "codex_reasoning_items":[reasoning]}
        ]})).unwrap();
        assert_eq!(instructions, "Read only.\n\nPreserve staged work.");
        assert_eq!(input[0], reasoning);
        assert_eq!(input[1]["role"], "assistant");
    }
}
