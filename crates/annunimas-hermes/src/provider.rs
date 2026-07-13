// sigil: REPAIR
use crate::mcp::{DiscordChannel, EmailChannel, McpChannel, McpMessage, SlackChannel};
use crate::types::OutboundMessage;
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::try_run_bounded_async;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Discord,
    Email,
    Slack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub kind: ProviderType,
    pub enabled: bool,
    pub persistent: bool,
    #[serde(default)]
    pub fallback_to_direct_api: bool,
}

impl ProviderConfig {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                id: "discord".to_string(),
                kind: ProviderType::Discord,
                enabled: true,
                persistent: true,
                fallback_to_direct_api: true,
            },
            Self {
                id: "email".to_string(),
                kind: ProviderType::Email,
                enabled: true,
                persistent: true,
                fallback_to_direct_api: false,
            },
            Self {
                id: "slack".to_string(),
                kind: ProviderType::Slack,
                enabled: true,
                persistent: true,
                fallback_to_direct_api: true,
            },
        ]
    }
}

#[derive(Clone, Default)]
pub struct ProviderRuntime {
    configs: Vec<ProviderConfig>,
    channels: HashMap<String, Arc<dyn McpChannel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchReceipt {
    pub dispatched: bool,
    pub attempts: u32,
    pub streaming: bool,
    pub chunks_sent: u32,
    pub provider_id: String,
    pub error: Option<String>,
}

impl ProviderRuntime {
    pub fn from_defaults() -> Self {
        let configs = ProviderConfig::defaults();
        let mut channels: HashMap<String, Arc<dyn McpChannel>> = HashMap::new();

        for config in &configs {
            if !config.enabled {
                continue;
            }
            match config.kind {
                ProviderType::Discord => {
                    if let Some(discord) = DiscordChannel::from_env() {
                        if discord.is_configured() {
                            channels.insert(config.id.clone(), Arc::new(discord));
                        }
                    }
                }
                ProviderType::Email => {
                    channels.insert(config.id.clone(), Arc::new(EmailChannel::default_config()));
                }
                ProviderType::Slack => {
                    let slack = SlackChannel::from_env();
                    if slack.is_configured() {
                        channels.insert(config.id.clone(), Arc::new(slack));
                    }
                }
            }
        }

        Self { configs, channels }
    }

    #[cfg(test)]
    pub fn from_test_channels(
        configs: Vec<ProviderConfig>,
        channels: Vec<(String, Arc<dyn McpChannel>)>,
    ) -> Self {
        Self {
            configs,
            channels: channels.into_iter().collect(),
        }
    }

    pub fn configured_provider_ids(&self) -> Vec<String> {
        self.configs
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.id.clone())
            .collect()
    }

    pub async fn online_provider_ids(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (id, channel) in &self.channels {
            if channel.health_check().await {
                out.push(id.clone());
            }
        }
        out.sort();
        out
    }

    pub async fn offline_provider_ids(&self) -> Vec<String> {
        let online = self.online_provider_ids().await;
        let mut out = Vec::new();
        for id in self.configured_provider_ids() {
            if !online.iter().any(|x| x == &id) {
                out.push(id);
            }
        }
        out
    }

    pub async fn dispatch(&self, msg: &OutboundMessage) -> Result<()> {
        let channel = self
            .channels
            .get(&msg.provider)
            .ok_or_else(|| AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: format!("provider unavailable: {}", msg.provider),
            })?;
        channel
            .send(&msg.body, &msg.channel)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: format!("provider dispatch failed ({}): {}", msg.provider, e),
            })?;
        Ok(())
    }

    pub async fn dispatch_streaming(&self, msg: &OutboundMessage) -> Result<u32> {
        let channel = self
            .channels
            .get(&msg.provider)
            .ok_or_else(|| AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: format!("provider unavailable: {}", msg.provider),
            })?;
        let sent = channel
            .send_stream(&msg.body, &msg.channel)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: format!("provider dispatch failed ({}): {}", msg.provider, e),
            })?;
        Ok(sent as u32)
    }

    pub async fn dispatch_with_retry(
        &self,
        msg: &OutboundMessage,
        max_attempts: u32,
        backoff_ms: u64,
    ) -> DispatchReceipt {
        let provider_config = self.configs.iter().find(|c| c.id == msg.provider);

        if let Some(receipt) = try_run_bounded_async(
            "hermes_provider_runtime_dispatch",
            provider_dispatch_limit(),
            || async move {
                let attempts = max_attempts.max(1);
                let mut last_error: Option<String> = None;
                for attempt in 1..=attempts {
                    let send_result = if msg.stream {
                        self.dispatch_streaming(msg)
                            .await
                            .map(|chunks| (true, chunks))
                    } else {
                        self.dispatch(msg).await.map(|_| (false, 1))
                    };
                    match send_result {
                        Ok((streaming, chunks_sent)) => {
                            info!(
                                provider = %msg.provider,
                                attempt,
                                "dispatch succeeded"
                            );
                            return DispatchReceipt {
                                dispatched: true,
                                attempts: attempt,
                                streaming,
                                chunks_sent,
                                provider_id: msg.provider.clone(),
                                error: None,
                            };
                        }
                        Err(err) => {
                            last_error = Some(err.to_string());
                            warn!(
                                provider = %msg.provider,
                                attempt,
                                error = %err,
                                "dispatch attempt failed"
                            );
                            if attempt < attempts {
                                let delay = backoff_ms.saturating_mul(attempt as u64);
                                sleep(Duration::from_millis(delay)).await;
                            }
                        }
                    }
                }

                DispatchReceipt {
                    dispatched: false,
                    attempts,
                    streaming: msg.stream,
                    chunks_sent: 0,
                    provider_id: msg.provider.clone(),
                    error: last_error,
                }
            },
        )
        .await
        {
            receipt
        } else {
            warn!(
                provider = %msg.provider,
                "dispatch concurrency gate saturated; fallback_to_direct_api={}",
                provider_config.map(|c| c.fallback_to_direct_api).unwrap_or(false)
            );
            DispatchReceipt {
                dispatched: false,
                attempts: 0,
                streaming: msg.stream,
                chunks_sent: 0,
                provider_id: msg.provider.clone(),
                error: Some("provider runtime dispatch concurrency gate saturated".to_string()),
            }
        }
    }

    pub async fn poll_once(&self) -> Result<Vec<(String, McpMessage)>> {
        let Some(out) = try_run_bounded_async(
            "hermes_provider_runtime_poll",
            provider_poll_limit(),
            || async move {
                let mut out = Vec::new();
                for (id, channel) in &self.channels {
                    let messages = channel.receive().await.map_err(|e| AnnunimasError::Agent {
                        agent: "hermes".to_string(),
                        message: format!("provider receive failed ({}): {}", id, e),
                    })?;
                    for msg in messages {
                        out.push((id.clone(), msg));
                    }
                }
                Ok(out)
            },
        )
        .await
        else {
            return Err(AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: "provider runtime poll concurrency gate saturated".to_string(),
            });
        };

        out
    }
}

fn provider_dispatch_limit() -> usize {
    std::env::var("ANNUNIMAS_HERMES_PROVIDER_DISPATCH_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2)
}

fn provider_poll_limit() -> usize {
    std::env::var("ANNUNIMAS_HERMES_PROVIDER_POLL_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::ProviderRuntime;

    #[tokio::test]
    async fn default_runtime_has_email_provider() {
        let rt = ProviderRuntime::from_defaults();
        let ids = rt.configured_provider_ids();
        assert!(ids.iter().any(|x| x == "email"));
    }
}
