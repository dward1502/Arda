use super::{ArdaError, ProviderState, Result};
use std::collections::BTreeMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration as StdDuration;

static PROXY_CHILDREN: OnceLock<Mutex<BTreeMap<String, Child>>> = OnceLock::new();

pub(super) async fn ensure_hermes_proxy(provider: &ProviderState) -> Result<String> {
    let base_url = hermes_proxy_base_url(provider);
    if hermes_proxy_ready(&base_url).await {
        return Ok(base_url);
    }

    let provider_key = provider.id.clone();
    {
        let mut children = proxy_children().lock().map_err(|_| ArdaError::Agent {
            agent: "charon".to_string(),
            message: "hermes_proxy child registry lock poisoned".to_string(),
        })?;
        let should_spawn = children
            .get_mut(&provider_key)
            .map(|child| match child.try_wait() {
                Ok(Some(_)) => true,
                Ok(None) => false,
                Err(_) => true,
            })
            .unwrap_or(true);
        if should_spawn {
            children.remove(&provider_key);
            let child = spawn_hermes_proxy(provider, &base_url)?;
            children.insert(provider_key, child);
        }
    }

    let deadline_attempts = hermes_proxy_startup_attempts();
    for _ in 0..deadline_attempts {
        tokio::time::sleep(StdDuration::from_millis(250)).await;
        if hermes_proxy_ready(&base_url).await {
            return Ok(base_url);
        }
    }

    Err(ArdaError::Agent {
        agent: "charon".to_string(),
        message: format!(
            "hermes_proxy driver started proxy for {} but {} did not become ready",
            provider.id, base_url
        ),
    })
}

fn proxy_children() -> &'static Mutex<BTreeMap<String, Child>> {
    PROXY_CHILDREN.get_or_init(|| Mutex::new(BTreeMap::new()))
}

async fn hermes_proxy_ready(base_url: &str) -> bool {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(StdDuration::from_millis(hermes_proxy_readiness_timeout_ms()))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    client
        .get(url)
        .bearer_auth("charon-readiness")
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn spawn_hermes_proxy(provider: &ProviderState, base_url: &str) -> Result<Child> {
    let bin = resolve_hermes_bin(provider);
    let upstream = provider
        .hermes_provider
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(provider.id.as_str());
    let (host, port) = hermes_proxy_host_port(base_url);
    Command::new(&bin)
        .arg("proxy")
        .arg("start")
        .arg("--provider")
        .arg(upstream)
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| ArdaError::Agent {
            agent: "charon".to_string(),
            message: format!(
                "failed to spawn hermes_proxy driver '{}' for provider {}: {err}",
                bin, provider.id
            ),
        })
}

pub(crate) fn hermes_proxy_base_url(provider: &ProviderState) -> String {
    provider
        .base_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let port = std::env::var("ARDA_HERMES_PROXY_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(8645);
            format!("http://127.0.0.1:{port}/v1")
        })
}

fn hermes_proxy_host_port(base_url: &str) -> (String, u16) {
    let without_scheme = base_url
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let authority = without_scheme.split('/').next().unwrap_or("127.0.0.1:8645");
    let mut parts = authority.rsplitn(2, ':');
    let port = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8645);
    let host = parts.next().unwrap_or(authority).to_string();
    (host, port)
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

fn hermes_proxy_startup_attempts() -> usize {
    std::env::var("ARDA_HERMES_PROXY_STARTUP_ATTEMPTS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

fn hermes_proxy_readiness_timeout_ms() -> u64 {
    std::env::var("ARDA_HERMES_PROXY_READINESS_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(500)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::types::{ModelCapabilities, ModelState};

    fn provider(id: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: id.to_string(),
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
            models: vec![ModelState {
                id: "auto".to_string(),
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
            supports_structured_output: true,
            driver: "hermes_proxy".to_string(),
            hermes_bin: None,
            hermes_provider: Some("nous".to_string()),
            hermes_toolsets: None,
        }
    }

    #[test]
    fn hermes_proxy_base_url_defaults_to_local_v1_endpoint() {
        let provider = provider("nous_portal");
        assert_eq!(hermes_proxy_base_url(&provider), "http://127.0.0.1:8645/v1");
    }

    #[test]
    fn hermes_proxy_host_port_parses_configured_base_url() {
        let parsed = hermes_proxy_host_port("http://127.0.0.1:8765/v1");
        assert_eq!(parsed, ("127.0.0.1".to_string(), 8765));
    }
}
