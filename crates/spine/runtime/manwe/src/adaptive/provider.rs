// sigil: REPAIR
// Provider capabilities, health probe state, rate limits, token caps, and
// feature facades for the adaptive routing tree.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Unknown,
    Probing,
    Healthy,
    Degraded,
    Down,
}

impl Default for HealthState {
    fn default() -> Self {
        HealthState::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFeature {
    Tools,
    Streaming,
    StructuredOutput,
    VisibleReasoning,
    PrivateLocal,
    HermesAgentCli,
    HermesProxy,
}

impl std::fmt::Display for ProviderFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ProviderFeature::Tools => "tools",
            ProviderFeature::Streaming => "streaming",
            ProviderFeature::StructuredOutput => "structured_output",
            ProviderFeature::VisibleReasoning => "visible_reasoning",
            ProviderFeature::PrivateLocal => "private_local",
            ProviderFeature::HermesAgentCli => "hermes_agent_cli",
            ProviderFeature::HermesProxy => "hermes_proxy",
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderFeatures {
    pub enabled: Vec<ProviderFeature>,
}

impl ProviderFeatures {
    pub fn new(enabled: Vec<ProviderFeature>) -> Self {
        Self { enabled }
    }

    pub fn supports(&self, feature: ProviderFeature) -> bool {
        self.enabled.contains(&feature)
    }

    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: Option<u64>,
    pub requests_per_day: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenCaps {
    pub context_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilitySummary {
    pub provider_id: String,
    pub health: HealthState,
    pub features: ProviderFeatures,
    pub rate_limits: RateLimits,
    pub token_caps: TokenCaps,
    pub has_api_key: bool,
    pub in_cooldown: bool,
    pub last_error: Option<String>,
}

impl ProviderCapabilitySummary {
    pub fn healthy(&self) -> bool {
        matches!(self.health, HealthState::Healthy)
    }

    pub fn routable(&self) -> bool {
        self.healthy() && !self.in_cooldown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilitySummary {
    pub model_id: String,
    pub provider_id: String,
    pub features: ProviderFeatures,
    pub token_caps: TokenCaps,
    pub streaming_validated: Option<bool>,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_state_default_is_unknown() {
        assert_eq!(HealthState::default(), HealthState::Unknown);
    }

    #[test]
    fn provider_feature_display_matches_snake_case() {
        assert_eq!(ProviderFeature::Tools.to_string(), "tools");
        assert_eq!(
            ProviderFeature::HermesAgentCli.to_string(),
            "hermes_agent_cli"
        );
    }

    #[test]
    fn provider_features_supports_known_capability() {
        let features = ProviderFeatures::new(vec![
            ProviderFeature::Tools,
            ProviderFeature::Streaming,
        ]);
        assert!(features.supports(ProviderFeature::Tools));
        assert!(!features.supports(ProviderFeature::PrivateLocal));
        assert!(!features.is_empty());
        assert!(ProviderFeatures::default().is_empty());
    }

    #[test]
    fn provider_summary_healthy_routable_gate() {
        let summary = ProviderCapabilitySummary {
            provider_id: "edge_core".to_string(),
            health: HealthState::Healthy,
            features: ProviderFeatures::default(),
            rate_limits: RateLimits::default(),
            token_caps: TokenCaps { context_window: 32768 },
            has_api_key: true,
            in_cooldown: false,
            last_error: None,
        };
        assert!(summary.healthy());
        assert!(summary.routable());
    }

    #[test]
    fn provider_summary_cooldown_blocks_routing() {
        let summary = ProviderCapabilitySummary {
            provider_id: "edge_guardhouse".to_string(),
            health: HealthState::Healthy,
            features: ProviderFeatures::default(),
            rate_limits: RateLimits::default(),
            token_caps: TokenCaps::default(),
            has_api_key: true,
            in_cooldown: true,
            last_error: None,
        };
        assert!(summary.healthy());
        assert!(!summary.routable());
    }
}
