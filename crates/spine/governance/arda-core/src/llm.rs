// sigil: REPAIR
//! LLM Provider Abstraction Layer
//!
//! Provider-agnostic interface for language model interactions.
//! One `OpenAiCompatible` implementation covers Ollama, OpenRouter,
//! vLLM, LM Studio, Anthropic-compatible proxies — anything that
//! speaks the OpenAI chat completions API.
//!
//! Swap `base_url` + `api_key` in config. Zero code changes.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::error::{ArdaError, Result};

// ── Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    /// Override the provider's default model
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Provider Trait ─────────────────────────────────────────────

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Provider display name (e.g. "Ollama (local)", "OpenRouter")
    fn provider_name(&self) -> &str;

    /// Default model for this provider
    fn default_model(&self) -> &str;
}

// ── OpenAI-Compatible Provider ─────────────────────────────────
//
// Works with: Ollama, OpenRouter, vLLM, LM Studio, OpenAI,
// Azure OpenAI, Together.ai, Groq, Fireworks — anything that
// implements POST /v1/chat/completions with the standard schema.

pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    name: String,
    chat_completions_url: String,
    api_key: Option<String>,
    default_model: String,
}

fn default_ollama_base_url() -> String {
    format!("http://{}:{}/v1", "127.0.0.1", 11434)
}

impl OpenAiCompatibleProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into();
        Self {
            client: reqwest::Client::new(),
            name: name.into(),
            chat_completions_url: chat_completions_url(&base_url),
            api_key,
            default_model: default_model.into(),
        }
    }

    /// Convenience: create a provider pointing at local Ollama
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new(
            "ollama",
            default_ollama_base_url(),
            Some("ollama".into()),
            model,
        )
    }

    /// Convenience: create a provider pointing at OpenRouter
    pub fn openrouter(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(
            "openrouter",
            "https://openrouter.ai/api/v1",
            Some(api_key.into()),
            model,
        )
    }

    fn agent_error(&self, message: impl Into<String>) -> ArdaError {
        ArdaError::Agent {
            agent: self.name.clone(),
            message: message.into(),
        }
    }
}

// Wire format for the OpenAI chat completions API
#[derive(Serialize)]
struct ApiRequest<'a> {
    model: Cow<'a, str>,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
    model: String,
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct ApiChoice {
    message: ApiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ApiMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

fn chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    format!("{trimmed}/chat/completions")
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let ChatRequest {
            messages,
            model,
            temperature,
            max_tokens,
        } = request;

        let api_request = ApiRequest {
            model: model
                .map(Cow::Owned)
                .unwrap_or_else(|| Cow::Borrowed(self.default_model.as_str())),
            messages,
            temperature,
            max_tokens,
        };

        let mut http_req = self
            .client
            .post(&self.chat_completions_url)
            .json(&api_request);

        if let Some(key) = &self.api_key {
            http_req = http_req.bearer_auth(key);
        }

        let response = http_req
            .send()
            .await
            .map_err(|e| self.agent_error(format!("LLM request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(self.agent_error(format!("LLM API error {status}: {body}")));
        }

        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| self.agent_error(format!("Failed to parse LLM response: {e}")))?;

        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| self.agent_error("LLM returned no choices"))?;

        Ok(ChatResponse {
            content: choice.message.content.unwrap_or_default(),
            model: api_response.model,
            usage: api_response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason,
        })
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }
}

// ── Config Types ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Which provider key to use by default
    pub default_provider: String,
    /// Named provider configurations
    pub providers: std::collections::HashMap<String, ProviderConfig>,
    /// Model routes: task_type -> model override
    /// E.g., {"research": "llama3.1:8b", "code": "qwen2.5-coder:3b"}
    #[serde(default)]
    pub model_routes: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub base_url: String,
    /// Direct API key value (prefer api_key_env in production)
    pub api_key: Option<String>,
    /// Environment variable name containing the API key
    pub api_key_env: Option<String>,
    pub default_model: String,
}

impl ProviderConfig {
    /// Resolve the API key: check direct value, then env var
    pub fn resolve_api_key(&self) -> Option<String> {
        if let Some(key) = &self.api_key {
            return Some(key.clone());
        }
        if let Some(env_var) = &self.api_key_env {
            return std::env::var(env_var).ok();
        }
        None
    }

    /// Build an OpenAiCompatibleProvider from this config
    pub fn into_provider(self, name: String) -> OpenAiCompatibleProvider {
        let api_key = self.resolve_api_key();
        OpenAiCompatibleProvider::new(name, self.base_url, api_key, self.default_model)
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "local".into(),
            ProviderConfig {
                base_url: default_ollama_base_url(),
                api_key: Some("ollama".into()),
                api_key_env: None,
                default_model: "qwen2.5-coder:3b".into(),
            },
        );
        Self {
            default_provider: "local".into(),
            providers,
            model_routes: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        chat_completions_url, default_ollama_base_url, ChatRequest, OpenAiCompatibleProvider,
    };

    #[test]
    fn chat_completions_url_trims_trailing_slash() {
        assert_eq!(
            chat_completions_url(&format!("{}/", default_ollama_base_url())),
            format!("{}/chat/completions", default_ollama_base_url())
        );
        assert_eq!(
            chat_completions_url("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
    }

    #[test]
    fn provider_caches_normalized_chat_endpoint() {
        let provider = OpenAiCompatibleProvider::new(
            "local",
            format!("{}/", default_ollama_base_url()),
            Some("ollama".into()),
            "qwen2.5-coder:3b",
        );

        assert_eq!(
            provider.chat_completions_url,
            format!("{}/chat/completions", default_ollama_base_url())
        );
    }

    #[test]
    fn chat_request_override_model_is_preserved() {
        let request = ChatRequest::new(Vec::new()).with_model("qwen2.5-coder:7b");
        assert_eq!(request.model.as_deref(), Some("qwen2.5-coder:7b"));
    }
}
