//! Driver that routes inference through the local `hermes` CLI instead of
//! an OpenAI-compatible HTTPS endpoint. Used for providers whose only
//! usable auth path is a subscription (Anthropic, OpenAI Codex), which
//! hermes-agent already handles via its provider registry.
//!
//! Flow: flatten the chat messages into a single prompt, invoke
//! `hermes chat -q <prompt> --provider <name> -Q --toolsets ""`, capture
//! stdout, and wrap it in an OpenAI `chat.completion` envelope so the
//! rest of charon doesn't need to know the difference.
use super::{ArdaError, JsonValue, ProviderState, Result};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::process::Command;

static READINESS_CACHE: OnceLock<Mutex<BTreeMap<String, HermesReadinessRecord>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct HermesReadinessRecord {
    status: HermesReadinessStatus,
    checked_at: Instant,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HermesReadinessStatus {
    Ready,
    Blocked,
}

pub(super) struct HermesCliOutcome {
    pub status: u16,
    pub latency_ms: u64,
    pub response: JsonValue,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct HermesCliReadinessSummary {
    pub status: String,
    pub cached: bool,
    pub checked_age_ms: Option<u64>,
    pub error: Option<String>,
}

pub(crate) fn hermes_cli_readiness_summary(
    provider: &ProviderState,
    model_id: Option<&str>,
) -> HermesCliReadinessSummary {
    let hermes_provider = provider
        .hermes_provider
        .clone()
        .unwrap_or_else(|| provider.id.clone());
    let key = readiness_cache_key(provider, &hermes_provider, model_id.unwrap_or(""));
    let cache = READINESS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let Ok(guard) = cache.lock() else {
        return HermesCliReadinessSummary {
            status: "unknown".to_string(),
            cached: false,
            checked_age_ms: None,
            error: Some("readiness cache lock poisoned".to_string()),
        };
    };
    let Some(record) = guard.get(&key) else {
        return HermesCliReadinessSummary {
            status: "unknown".to_string(),
            cached: false,
            checked_age_ms: None,
            error: None,
        };
    };
    HermesCliReadinessSummary {
        status: match record.status {
            HermesReadinessStatus::Ready => "ready",
            HermesReadinessStatus::Blocked => "blocked",
        }
        .to_string(),
        cached: true,
        checked_age_ms: Some(record.checked_at.elapsed().as_millis() as u64),
        error: record.error.clone(),
    }
}

pub(super) async fn invoke_hermes_cli(
    provider: &ProviderState,
    model_id: &str,
    body: &JsonValue,
    timeout: Duration,
) -> Result<HermesCliOutcome> {
    let prompt = flatten_messages_to_prompt(body);
    if prompt.trim().is_empty() {
        return Ok(HermesCliOutcome {
            status: 400,
            latency_ms: 0,
            response: serde_json::json!({
                "error": {
                    "message": "hermes_agent_cli driver received no user content",
                    "type": "invalid_request_error",
                }
            }),
            error: Some("empty prompt".to_string()),
        });
    }

    let bin = resolve_hermes_bin(provider);
    let hermes_provider = provider
        .hermes_provider
        .clone()
        .unwrap_or_else(|| provider.id.clone());
    let toolsets = provider.hermes_toolsets.clone().unwrap_or_default();
    let readiness_key = readiness_cache_key(provider, &hermes_provider, model_id);
    if let Some(error) = cached_blocked_readiness(&readiness_key) {
        return Ok(HermesCliOutcome {
            status: 503,
            latency_ms: 0,
            response: serde_json::json!({
                "error": {
                    "message": format!("hermes_agent_cli readiness cache blocked route: {error}"),
                    "type": "cached_provider_unavailable",
                },
                "_charon_driver": "hermes_agent_cli",
                "_manwe_aule_readiness_cache": "blocked",
            }),
            error: Some(error),
        });
    }

    let mut cmd = Command::new(&bin);
    cmd.arg("chat")
        .arg("-q")
        .arg(&prompt)
        .arg("-Q")
        .arg("--provider")
        .arg(&hermes_provider)
        .arg("--ignore-rules")
        .arg("--source")
        .arg("tool");
    if !model_id.is_empty() {
        cmd.arg("-m").arg(model_id);
    }
    // Pass --toolsets only when non-empty; the CLI treats a missing
    // flag as "inherit config defaults" but an empty-string value as
    // "explicitly disable tools."
    if toolsets.is_empty() {
        cmd.arg("--toolsets").arg("");
    } else {
        cmd.arg("--toolsets").arg(&toolsets);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let child = cmd.spawn().map_err(|err| ArdaError::Agent {
        agent: "manwe".to_string(),
        message: format!("hermes_agent_cli driver failed to spawn '{}': {err}", bin),
    })?;

    let output_result = tokio::time::timeout(timeout, child.wait_with_output()).await;
    let latency_ms = started.elapsed().as_millis() as u64;

    let output = match output_result {
        Ok(Ok(out)) => out,
        Ok(Err(err)) => {
            return Ok(HermesCliOutcome {
                status: 500,
                latency_ms,
                response: serde_json::json!({
                    "error": {
                        "message": format!("hermes_agent_cli io error: {err}"),
                        "type": "upstream_error",
                    }
                }),
                error: Some(err.to_string()),
            });
        }
        Err(_) => {
            return Ok(HermesCliOutcome {
                status: 504,
                latency_ms,
                response: serde_json::json!({
                    "error": {
                        "message": format!(
                            "hermes_agent_cli timed out after {}s",
                            timeout.as_secs()
                        ),
                        "type": "timeout",
                    }
                }),
                error: Some("timeout".to_string()),
            });
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let detail = if !stderr.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        let preview = detail.chars().take(400).collect::<String>();
        record_readiness(
            &readiness_key,
            if hermes_error_should_cache_block(&preview) {
                HermesReadinessStatus::Blocked
            } else {
                HermesReadinessStatus::Ready
            },
            Some(preview.clone()),
        );
        return Ok(HermesCliOutcome {
            status: 502,
            latency_ms,
            response: serde_json::json!({
                "error": {
                    "message": format!("hermes CLI exit {}: {preview}", output.status),
                    "type": "upstream_error",
                }
            }),
            error: Some(preview),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let content = strip_hermes_trailing_session_lines(&stdout);
    record_readiness(&readiness_key, HermesReadinessStatus::Ready, None);

    let response = serde_json::json!({
        "id": format!("chatcmpl-hermescli-{}", uuid_like_timestamp()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model_id,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        },
        "_charon_driver": "hermes_agent_cli",
    });

    Ok(HermesCliOutcome {
        status: 200,
        latency_ms,
        response,
        error: None,
    })
}

fn resolve_hermes_bin(provider: &ProviderState) -> String {
    if let Some(bin) = provider.hermes_bin.as_deref() {
        if !bin.trim().is_empty() {
            return bin.to_string();
        }
    }
    if let Ok(env) = std::env::var("ARDA_HERMES_BIN") {
        if !env.trim().is_empty() {
            return env;
        }
    }
    "hermes".to_string()
}

fn readiness_cache_key(provider: &ProviderState, hermes_provider: &str, model_id: &str) -> String {
    format!("{}|{}|{}", provider.id, hermes_provider, model_id)
}

fn cached_blocked_readiness(key: &str) -> Option<String> {
    let cache = READINESS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut guard = cache.lock().ok()?;
    let record = guard.get(key)?;
    if record.status != HermesReadinessStatus::Blocked {
        return None;
    }
    if record.checked_at.elapsed() > blocked_readiness_ttl() {
        guard.remove(key);
        return None;
    }
    Some(
        record
            .error
            .clone()
            .unwrap_or_else(|| "cached Hermes route unavailable".to_string()),
    )
}

fn record_readiness(key: &str, status: HermesReadinessStatus, error: Option<String>) {
    let cache = READINESS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(
            key.to_string(),
            HermesReadinessRecord {
                status,
                checked_at: Instant::now(),
                error,
            },
        );
    }
}

fn hermes_error_should_cache_block(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    [
        "not supported",
        "unsupported",
        "insufficient balance",
        "billing",
        "no resource package",
        "recharge",
        "unauthorized",
        "authentication",
        "login",
        "oauth",
        "quota",
        "rate limit",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn blocked_readiness_ttl() -> Duration {
    Duration::from_secs(
        std::env::var("ARDA_HERMES_CLI_BLOCKED_READINESS_TTL_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(900),
    )
}

fn flatten_messages_to_prompt(body: &JsonValue) -> String {
    let Some(messages) = body.get("messages").and_then(|v| v.as_array()) else {
        return String::new();
    };
    let mut sections: Vec<String> = Vec::new();
    for message in messages {
        let role = message
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("user");
        let content = message
            .get("content")
            .map(flatten_content_value)
            .unwrap_or_default();
        if content.trim().is_empty() {
            continue;
        }
        let label = match role {
            "system" => "System",
            "assistant" => "Assistant",
            "tool" => "Tool",
            _ => "User",
        };
        sections.push(format!("[{label}]\n{content}"));
    }
    sections.join("\n\n")
}

fn flatten_content_value(value: &JsonValue) -> String {
    if let Some(s) = value.as_str() {
        return s.to_string();
    }
    if let Some(array) = value.as_array() {
        let mut parts = Vec::new();
        for item in array {
            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                parts.push(text.to_string());
            } else if let Some(s) = item.as_str() {
                parts.push(s.to_string());
            }
        }
        return parts.join("\n");
    }
    String::new()
}

/// Hermes CLI appends a session-info footer (like `Session: 20260424_...`)
/// after the response in quiet mode. Strip obvious trailing metadata so
/// the assistant content is clean.
fn strip_hermes_trailing_session_lines(stdout: &str) -> String {
    let trimmed = stdout.trim_end_matches('\n');
    let lines: Vec<&str> = trimmed.lines().collect();
    let mut drop_from = lines.len();
    for (idx, line) in lines.iter().enumerate().rev() {
        let lowered = line.trim().to_lowercase();
        if lowered.starts_with("session:")
            || lowered.starts_with("session id:")
            || lowered.starts_with("(session ")
            || lowered.starts_with("tokens:")
            || lowered.starts_with("usage:")
        {
            drop_from = idx;
        } else if !line.trim().is_empty() {
            break;
        }
    }
    lines[..drop_from].join("\n").trim().to_string()
}

fn uuid_like_timestamp() -> String {
    format!("{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_messages_joins_roles_and_strips_empty() {
        let body = serde_json::json!({
            "messages": [
                {"role": "system", "content": "Be terse."},
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": ""},
                {"role": "user", "content": "How are you?"},
            ]
        });
        let prompt = flatten_messages_to_prompt(&body);
        assert!(prompt.contains("[System]\nBe terse."));
        assert!(prompt.contains("[User]\nHello"));
        assert!(prompt.contains("[User]\nHow are you?"));
        assert!(!prompt.contains("[Assistant]"));
    }

    #[test]
    fn flatten_content_value_handles_array_of_text_parts() {
        let value = serde_json::json!([
            {"type": "text", "text": "one"},
            {"type": "text", "text": "two"},
        ]);
        let flat = flatten_content_value(&value);
        assert_eq!(flat, "one\ntwo");
    }

    #[test]
    fn strip_trailing_session_footer_preserves_body() {
        let raw = "Hello world\n\nThe quick brown fox.\n\nSession: 20260424_abc\nTokens: 42";
        let cleaned = strip_hermes_trailing_session_lines(raw);
        assert_eq!(cleaned, "Hello world\n\nThe quick brown fox.");
    }

    #[test]
    fn readiness_cache_blocks_repeated_unsupported_model_failures() {
        let provider = ProviderState {
            id: "openai_sub".to_string(),
            name: "OpenAI Sub".to_string(),
            base_url: None,
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
            models: vec![],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: false,
            supports_structured_output: false,
            driver: "hermes_agent_cli".to_string(),
            hermes_bin: None,
            hermes_provider: Some("openai-codex".to_string()),
            hermes_toolsets: None,
        };
        let key = readiness_cache_key(&provider, "openai-codex", "gpt-5-codex");
        assert!(hermes_error_should_cache_block(
            "The 'gpt-5-codex' model is not supported when using Codex with a ChatGPT account."
        ));

        record_readiness(
            &key,
            HermesReadinessStatus::Blocked,
            Some("model is not supported".to_string()),
        );

        assert_eq!(
            cached_blocked_readiness(&key).as_deref(),
            Some("model is not supported")
        );
    }
}
