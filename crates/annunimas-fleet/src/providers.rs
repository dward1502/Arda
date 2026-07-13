use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
    pub capable_tasks: Vec<String>,
    pub context_window: u64,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub healthy: bool,
    pub api_key_env: Option<String>,
    pub requests_per_minute: u32,
    pub requests_per_day: Option<u32>,
    pub models: Vec<ProviderModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersToml {
    pub provider: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokenUsage {
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub requests: u64,
    pub cost_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokenUsageSnapshot {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub providers: Vec<ProviderTokenUsage>,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub active_provider: Option<String>,
    pub fallback_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiResponse {
    pub id: String,
    pub choices: Vec<ApiChoice>,
    pub usage: ApiUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChoice {
    pub message: ApiMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    pub provider_id: String,
    pub failures: u32,
    pub last_failure: Option<String>,
    pub state: CircuitBreakerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CircuitBreakerStatus {
    Closed,
    Open,
    HalfOpen,
}

pub struct ProviderTokenTracker {
    usage: RwLock<HashMap<String, ProviderTokenUsage>>,
    providers: RwLock<Vec<ProviderConfig>>,
    circuit_breakers: RwLock<HashMap<String, CircuitBreakerState>>,
    state_path: String,
    max_failures: u32,
}

impl Default for ProviderTokenTracker {
    fn default() -> Self {
        Self::new(".")
    }
}

impl ProviderTokenTracker {
    pub fn new(config_root: impl AsRef<Path>) -> Self {
        let state_path = config_root
            .as_ref()
            .join("core/state")
            .to_string_lossy()
            .into_owned();
        let config_path = config_root.as_ref().join("config/charon.providers.toml");

        let providers = if config_path.exists() {
            Self::load_providers_from_toml(&config_path)
        } else {
            Self::default_providers()
        };

        Self {
            usage: RwLock::new(HashMap::new()),
            providers: RwLock::new(providers),
            circuit_breakers: RwLock::new(HashMap::new()),
            state_path,
            max_failures: 3,
        }
    }

    fn load_providers_from_toml(path: &Path) -> Vec<ProviderConfig> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let parsed: ProvidersToml = toml::from_str(&content).unwrap_or_else(|_| ProvidersToml {
            provider: Vec::new(),
        });
        parsed.provider
    }

    fn default_providers() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                id: "openrouter".to_owned(),
                name: "OpenRouter".to_owned(),
                base_url: "https://openrouter.ai/api/v1".to_owned(),
                enabled: true,
                healthy: true,
                api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
                requests_per_minute: 30,
                requests_per_day: Some(100000),
                models: vec![ProviderModel {
                    id: "openrouter/auto".to_owned(),
                    capable_tasks: vec![
                        "research".to_owned(),
                        "reasoning".to_owned(),
                        "chat".to_owned(),
                        "summary".to_owned(),
                    ],
                    context_window: 128000,
                    is_default: true,
                }],
            },
            ProviderConfig {
                id: "cerebras".to_owned(),
                name: "Cerebras".to_owned(),
                base_url: "https://api.cerebras.ai/v1".to_owned(),
                enabled: false,
                healthy: true,
                api_key_env: Some("CEREBRAS_API_KEY".to_owned()),
                requests_per_minute: 30,
                requests_per_day: Some(1000000),
                models: vec![],
            },
        ]
    }

    pub fn get_enabled_providers(&self) -> Vec<ProviderConfig> {
        self.providers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|p| p.enabled && p.healthy)
            .cloned()
            .collect()
    }

    pub fn get_provider(&self, provider_id: &str) -> Option<ProviderConfig> {
        self.providers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|p| p.id == provider_id)
            .cloned()
    }

    pub fn get_available_provider(&self) -> Option<ProviderConfig> {
        self.get_enabled_providers().into_iter().next()
    }

    pub fn has_fallback(&self) -> bool {
        !self.get_enabled_providers().is_empty()
    }

    pub async fn call_api(
        &self,
        provider_id: &str,
        model: &str,
        messages: Vec<serde_json::Value>,
    ) -> anyhow::Result<ExternalApiResponse> {
        // Check circuit breaker
        if let Some(cb) = self
            .circuit_breakers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(provider_id)
        {
            if cb.state == CircuitBreakerStatus::Open {
                return Err(anyhow::anyhow!(
                    "Circuit breaker OPEN for provider: {}",
                    provider_id
                ));
            }
        }

        let provider = self
            .get_provider(provider_id)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_id))?;

        let api_key = if let Some(env_var) = &provider.api_key_env {
            std::env::var(env_var).ok()
        } else {
            None
        };

        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
        });

        // Retry loop with exponential backoff
        let max_retries = 3;
        let base_delay_ms = 500;

        for attempt in 0..max_retries {
            let mut request = client
                .post(format!("{}/chat/completions", provider.base_url))
                .header("Content-Type", "application/json")
                .json(&request_body);

            if let Some(key) = &api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        let api_response: ExternalApiResponse = response.json().await?;

                        // Success - record usage and reset circuit breaker
                        self.record_usage(
                            provider_id,
                            api_response.usage.prompt_tokens,
                            api_response.usage.completion_tokens,
                        );
                        self.reset_circuit_breaker(provider_id);

                        return Ok(api_response);
                    } else if status.is_server_error() && attempt < max_retries - 1 {
                        // Server error - retry
                        let error_text = response.text().await.unwrap_or_default();
                        tracing::warn!("API server error {}: {}, retrying...", status, error_text);
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            base_delay_ms * 2_u64.pow(attempt),
                        ))
                        .await;
                        continue;
                    } else {
                        // Client error or final retry - record failure
                        let error_text = response.text().await.unwrap_or_default();
                        self.record_circuit_breaker_failure(provider_id, &error_text);
                        return Err(anyhow::anyhow!(
                            "API call failed: {} - {}",
                            status,
                            error_text
                        ));
                    }
                }
                Err(e) if attempt < max_retries - 1 => {
                    // Network error - retry
                    tracing::warn!("API network error: {}, retrying...", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        base_delay_ms * 2_u64.pow(attempt),
                    ))
                    .await;
                    continue;
                }
                Err(e) => {
                    // Final retry failed
                    self.record_circuit_breaker_failure(provider_id, &e.to_string());
                    return Err(anyhow::anyhow!(
                        "API call failed after {} retries: {}",
                        max_retries,
                        e
                    ));
                }
            }
        }

        Err(anyhow::anyhow!(
            "API call failed after {} attempts",
            max_retries
        ))
    }

    fn record_circuit_breaker_failure(&self, provider_id: &str, error: &str) {
        if let Ok(mut cbs) = self.circuit_breakers.write() {
            let cb = cbs
                .entry(provider_id.to_owned())
                .or_insert(CircuitBreakerState {
                    provider_id: provider_id.to_owned(),
                    failures: 0,
                    last_failure: None,
                    state: CircuitBreakerStatus::Closed,
                });
            cb.failures += 1;
            cb.last_failure = Some(error.to_string());

            if cb.failures >= self.max_failures {
                cb.state = CircuitBreakerStatus::Open;
                tracing::warn!("Circuit breaker OPEN for provider: {}", provider_id);
            }
        }
    }

    fn reset_circuit_breaker(&self, provider_id: &str) {
        if let Ok(mut cbs) = self.circuit_breakers.write() {
            if let Some(cb) = cbs.get_mut(provider_id) {
                cb.failures = 0;
                cb.state = CircuitBreakerStatus::Closed;
                cb.last_failure = None;
            }
        }
    }

    pub fn record_usage(&self, provider_id: &str, prompt_tokens: u64, completion_tokens: u64) {
        let total = prompt_tokens + completion_tokens;
        let cost = Self::estimate_cost(provider_id, prompt_tokens, completion_tokens);

        if let Ok(mut usage) = self.usage.write() {
            let entry = usage
                .entry(provider_id.to_owned())
                .or_insert(ProviderTokenUsage {
                    provider_id: provider_id.to_owned(),
                    provider_name: self
                        .get_provider(provider_id)
                        .map(|p| p.name)
                        .unwrap_or_else(|| "Unknown".to_owned()),
                    model: "default".to_owned(),
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    requests: 0,
                    cost_estimate: 0.0,
                });

            entry.prompt_tokens += prompt_tokens;
            entry.completion_tokens += completion_tokens;
            entry.total_tokens += total;
            entry.requests += 1;
            entry.cost_estimate += cost;
        }
    }

    fn estimate_cost(provider_id: &str, prompt: u64, completion: u64) -> f64 {
        let prompt_cost_per_1k = match provider_id {
            "openrouter" => 0.0001,
            "cerebras" => 0.0004,
            "groq" => 0.0002,
            "google" => 0.0001,
            _ => 0.001,
        };
        let completion_cost_per_1k = match provider_id {
            "openrouter" => 0.0004,
            "cerebras" => 0.0004,
            "groq" => 0.0004,
            "google" => 0.0004,
            _ => 0.001,
        };

        (prompt as f64 * prompt_cost_per_1k / 1000.0)
            + (completion as f64 * completion_cost_per_1k / 1000.0)
    }

    pub fn snapshot(&self) -> ProviderTokenUsageSnapshot {
        let usage = self.usage.read().unwrap_or_else(|e| e.into_inner());
        let providers: Vec<ProviderTokenUsage> = usage.values().cloned().collect();
        let total_tokens: u64 = providers.iter().map(|p| p.total_tokens).sum();
        let total_cost: f64 = providers.iter().map(|p| p.cost_estimate).sum();

        ProviderTokenUsageSnapshot {
            schema_version: "annunimas.provider-usage.v1".to_owned(),
            generated_at_utc: chrono::Utc::now().to_rfc3339(),
            providers,
            total_tokens,
            total_cost,
            active_provider: self.get_available_provider().map(|p| p.id),
            fallback_available: self.has_fallback(),
        }
    }

    pub fn write_snapshot(&self) -> anyhow::Result<()> {
        let snapshot = self.snapshot();
        let path = format!("{}/provider_token_usage.json", self.state_path);
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load_usage_from_disk(&self) -> anyhow::Result<()> {
        let path = format!("{}/provider_token_usage.json", self.state_path);
        if !std::path::Path::new(&path).exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path)?;
        let snapshot: ProviderTokenUsageSnapshot = serde_json::from_str(&content)?;

        if let Ok(mut usage) = self.usage.write() {
            for provider_usage in snapshot.providers {
                usage.insert(provider_usage.provider_id.clone(), provider_usage);
            }
        }

        Ok(())
    }

    pub fn save_state(&self) -> anyhow::Result<()> {
        // Save usage
        self.write_snapshot()?;

        // Save circuit breaker state
        let cb_path = format!("{}/circuit_breakers.json", self.state_path);
        if let Ok(cbs) = self.circuit_breakers.read() {
            let cb_values: Vec<&CircuitBreakerState> = cbs.values().collect();
            let json = serde_json::to_string_pretty(&cb_values)?;
            std::fs::write(&cb_path, json)?;
        }

        Ok(())
    }

    pub fn load_state(&self) -> anyhow::Result<()> {
        self.load_usage_from_disk()?;

        // Load circuit breaker state
        let cb_path = format!("{}/circuit_breakers.json", self.state_path);
        if std::path::Path::new(&cb_path).exists() {
            let content = std::fs::read_to_string(&cb_path)?;
            let cbs: Vec<CircuitBreakerState> = serde_json::from_str(&content)?;

            if let Ok(mut circuit_breakers) = self.circuit_breakers.write() {
                for cb in cbs {
                    circuit_breakers.insert(cb.provider_id.clone(), cb);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_provider_config(root: &Path, content: &str) {
        let config_dir = root.join("config");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        std::fs::write(config_dir.join("charon.providers.toml"), content).expect("provider config");
    }

    fn init_state_dir(root: &Path) {
        std::fs::create_dir_all(root.join("core/state")).expect("state dir");
    }

    #[test]
    fn invalid_provider_toml_falls_back_to_empty_provider_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_provider_config(dir.path(), "provider = [");

        let providers = ProviderTokenTracker::load_providers_from_toml(
            &dir.path().join("config/charon.providers.toml"),
        );

        assert!(providers.is_empty());
    }

    #[test]
    fn enabled_providers_and_fallback_only_include_healthy_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_state_dir(dir.path());
        write_provider_config(
            dir.path(),
            r#"
[[provider]]
id = "openrouter"
name = "OpenRouter"
base_url = "https://openrouter.ai/api/v1"
enabled = true
healthy = true
requests_per_minute = 30
models = []

[[provider]]
id = "cerebras"
name = "Cerebras"
base_url = "https://api.cerebras.ai/v1"
enabled = true
healthy = false
requests_per_minute = 30
models = []

[[provider]]
id = "groq"
name = "Groq"
base_url = "https://api.groq.com/openai/v1"
enabled = false
healthy = true
requests_per_minute = 30
models = []
"#,
        );

        let tracker = ProviderTokenTracker::new(dir.path());
        let enabled = tracker.get_enabled_providers();

        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "openrouter");
        assert_eq!(
            tracker.get_available_provider().map(|provider| provider.id),
            Some("openrouter".to_owned())
        );
        assert!(tracker.has_fallback());
    }

    #[test]
    fn record_usage_snapshot_and_state_round_trip_persist() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_state_dir(dir.path());
        let tracker = ProviderTokenTracker::new(dir.path());

        tracker.record_usage("openrouter", 120, 80);
        tracker.record_usage("openrouter", 30, 20);
        tracker.record_usage("cerebras", 50, 50);
        tracker
            .save_state()
            .expect("provider usage and circuit breakers saved");

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.total_tokens, 350);
        assert_eq!(snapshot.providers.len(), 2);
        assert_eq!(snapshot.active_provider.as_deref(), Some("openrouter"));
        assert!(snapshot.fallback_available);

        let restored = ProviderTokenTracker::new(dir.path());
        restored.load_state().expect("state loaded");
        let restored_snapshot = restored.snapshot();

        assert_eq!(restored_snapshot.total_tokens, 350);
        let openrouter = restored_snapshot
            .providers
            .iter()
            .find(|usage| usage.provider_id == "openrouter")
            .expect("openrouter usage");
        assert_eq!(openrouter.requests, 2);
        assert_eq!(openrouter.prompt_tokens, 150);
        assert_eq!(openrouter.completion_tokens, 100);

        let cerebras = restored_snapshot
            .providers
            .iter()
            .find(|usage| usage.provider_id == "cerebras")
            .expect("cerebras usage");
        assert_eq!(cerebras.total_tokens, 100);
    }

    #[test]
    fn circuit_breaker_opens_after_repeated_failures_and_resets() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_state_dir(dir.path());
        let tracker = ProviderTokenTracker::new(dir.path());

        tracker.record_circuit_breaker_failure("openrouter", "timeout 1");
        tracker.record_circuit_breaker_failure("openrouter", "timeout 2");
        tracker.record_circuit_breaker_failure("openrouter", "timeout 3");

        {
            let breakers = tracker
                .circuit_breakers
                .read()
                .unwrap_or_else(|e| e.into_inner());
            let state = breakers.get("openrouter").expect("breaker state");
            assert_eq!(state.failures, 3);
            assert_eq!(state.state, CircuitBreakerStatus::Open);
            assert_eq!(state.last_failure.as_deref(), Some("timeout 3"));
        }

        tracker.reset_circuit_breaker("openrouter");

        let breakers = tracker
            .circuit_breakers
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let state = breakers.get("openrouter").expect("breaker state");
        assert_eq!(state.failures, 0);
        assert_eq!(state.state, CircuitBreakerStatus::Closed);
        assert!(state.last_failure.is_none());
    }
}
