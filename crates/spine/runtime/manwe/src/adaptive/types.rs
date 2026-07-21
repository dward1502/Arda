// sigil: REPAIR
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteLoveEquationGuard {
    pub resonance: f64,
    pub attention: f64,
    pub reciprocity: f64,
    pub score: f64,
}

impl Default for RouteLoveEquationGuard {
    fn default() -> Self {
        Self {
            resonance: 0.0,
            attention: 0.0,
            reciprocity: 0.0,
            score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteGovernanceLens {
    pub lens_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub outcome: String,
    pub score: f64,
    pub pass_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGovernance {
    pub triad_passed: bool,
    pub triad_aurelius_score: f64,
    pub triad_bacon_score: f64,
    pub triad_sun_tzu_score: f64,
    pub love_equation_guard: RouteLoveEquationGuard,
    #[serde(default)]
    pub governance_method: String,
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub chain_version: String,
    #[serde(default)]
    pub profile_source: String,
    #[serde(default)]
    pub review_mode: String,
    #[serde(default)]
    pub profile_maturity: String,
    #[serde(default)]
    pub autonomous_blocking_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub veto_reason: Option<String>,
    #[serde(default)]
    pub lenses: Vec<RouteGovernanceLens>,
    #[serde(default)]
    pub resonance_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triad_purity_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub love_projected_empathy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub love_delta_empathy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub philosopher_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub philosopher_alignment_score: Option<f64>,
    #[serde(default)]
    pub joule_measurement_source: String,
    #[serde(default)]
    pub joule_measurement_confidence: f64,
    #[serde(default)]
    pub joule_autonomy_truth_allowed: bool,
}

impl Default for RouteGovernance {
    fn default() -> Self {
        Self {
            triad_passed: true,
            triad_aurelius_score: 1.0,
            triad_bacon_score: 1.0,
            triad_sun_tzu_score: 1.0,
            love_equation_guard: RouteLoveEquationGuard::default(),
            governance_method: "triad".to_string(),
            chain_id: String::new(),
            chain_version: String::new(),
            profile_source: String::new(),
            review_mode: String::new(),
            profile_maturity: String::new(),
            autonomous_blocking_enabled: false,
            veto_reason: None,
            lenses: Vec::new(),
            resonance_score: 0.0,
            triad_purity_source: None,
            love_projected_empathy: None,
            love_delta_empathy: None,
            philosopher_action: None,
            philosopher_alignment_score: None,
            joule_measurement_source: String::new(),
            joule_measurement_confidence: 0.0,
            joule_autonomy_truth_allowed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManweRequestEnvelope {
    pub agent_id: String,
    pub task_type: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    pub options: serde_json::Value,
}

fn default_priority() -> String {
    "normal".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    #[serde(default)]
    pub tools: Option<bool>,
    #[serde(default)]
    pub streaming: Option<bool>,
    #[serde(default)]
    pub structured_output: Option<bool>,
    /// Some catalogs expose models whose normal answer surface includes
    /// visible reasoning/thinking text. Leave unset to infer from dynamic
    /// catalog names; set explicitly when provider metadata is more precise.
    #[serde(default)]
    pub visible_reasoning: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelState {
    pub id: String,
    /// Alternate names this model responds to (e.g. llama.cpp /model aliases,
    /// shortened names, or provider-specific naming conventions).
    #[serde(default)]
    pub aliases: Vec<String>,
    pub capable_tasks: Vec<String>,
    pub context_window: usize,
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub healthy: bool,
    #[serde(default)]
    pub in_cooldown: bool,
    #[serde(default)]
    pub cooldown_until_utc: Option<String>,
    #[serde(default)]
    pub consecutive_failures: u32,
    #[serde(default)]
    pub consecutive_successes: u32,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub avg_latency_ms: Option<u64>,
    #[serde(default)]
    pub cost_per_million_tokens_in: Option<f64>,
    #[serde(default)]
    pub cost_per_million_tokens_out: Option<f64>,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    /// Streaming probe result. `None` means unknown/backward-compatible and is
    /// still routable; explicit `false` blocks streaming routes for this model.
    #[serde(default)]
    pub streaming_validated: Option<bool>,
}

impl ModelState {
    /// Check if the given model_id matches this model's id or any alias.
    /// Normalizes by lowercasing and stripping common delimiters for comparison.
    pub fn alias_matches(&self, query: &str) -> bool {
        let normalized_query = Self::normalize_model_id(query);
        self.aliases
            .iter()
            .any(|alias| Self::normalize_model_id(alias) == normalized_query)
    }

    fn normalize_model_id(id: &str) -> String {
        id.to_ascii_lowercase().replace(['/', '-', '_', '.'], "")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderState {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default = "default_access_tier")]
    pub access_tier: String,
    #[serde(default = "default_quality_band")]
    pub quality_band: String,
    #[serde(default)]
    pub intelligence_refreshed_at_utc: Option<String>,
    /// Preferred low-latency, terse-response model for health probes. This is
    /// intentionally separate from the production default model because strong
    /// reasoning models are often poor marker-echo targets.
    #[serde(default)]
    pub probe_model: Option<String>,
    /// Human/operator-readable profile label for the current probe model
    /// choice, e.g. "low_latency_terse" or "catalog_default_fallback".
    #[serde(default)]
    pub probe_profile: Option<String>,
    pub enabled: bool,
    pub has_api_key: bool,
    pub healthy: bool,
    pub in_cooldown: bool,
    #[serde(default)]
    pub cooldown_until_utc: Option<String>,
    #[serde(default)]
    pub cooldown_backoff_seconds: u64,
    pub requests_per_minute: Option<u64>,
    pub requests_used_minute: u64,
    #[serde(default)]
    pub minute_window_started_utc: Option<String>,
    pub requests_per_day: Option<u64>,
    pub requests_used_day: u64,
    #[serde(default)]
    pub day_window_started_utc: Option<String>,
    pub models: Vec<ModelState>,
    #[serde(default)]
    pub error_count: u32,
    #[serde(default)]
    pub consecutive_failures: u32,
    #[serde(default)]
    pub consecutive_successes: u32,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub avg_latency_ms: Option<u64>,
    #[serde(default)]
    pub active_connections: u32,
    #[serde(default)]
    pub last_reservation_utc: Option<String>,
    #[serde(default)]
    pub supports_tools: bool,
    /// Whether this provider accepts `response_format` (JSON schema mode).
    #[serde(default = "default_true")]
    pub supports_structured_output: bool,
    /// Driver selects the inference transport. "openai_compat" (default)
    /// issues HTTPS POSTs to base_url. "hermes_proxy" keeps a local
    /// `hermes proxy start` process warm and forwards through its
    /// OpenAI-compatible endpoint. "hermes_agent_cli" invokes the local
    /// `hermes` CLI via subprocess, using its subscription-backed OAuth for
    /// providers that Hermes has not exposed through proxy mode yet.
    #[serde(default = "default_driver")]
    pub driver: String,
    /// For driver=hermes_agent_cli/hermes_proxy: path to the hermes binary.
    /// When empty, falls back to env ARDA_HERMES_BIN, then "hermes" on PATH.
    #[serde(default)]
    pub hermes_bin: Option<String>,
    /// For driver=hermes_agent_cli/hermes_proxy: the --provider argument
    /// passed to `hermes chat` or `hermes proxy start` (e.g. "anthropic",
    /// "openai-codex", "nous"). When empty, falls back to the Manwe provider id.
    #[serde(default)]
    pub hermes_provider: Option<String>,
    /// For driver=hermes_agent_cli: value passed to --toolsets. Default ""
    /// disables hermes's own tool loop so the CLI behaves as pure inference.
    #[serde(default)]
    pub hermes_toolsets: Option<String>,
}

fn default_access_tier() -> String {
    "mixed".to_string()
}

fn default_quality_band() -> String {
    "medium".to_string()
}

fn default_true() -> bool {
    true
}

fn default_driver() -> String {
    "openai_compat".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub provider_id: String,
    pub model_id: String,
    pub reason: String,
    pub route_class: String,
    pub execution_lane: String,
    pub context_window_target: usize,
    pub governance: RouteGovernance,
    /// Stable per-route correlation ID, minted in `ManweService::route` and
    /// surfaced as the `x-manwe-route-id` response header on proxy calls.
    /// Lets operators trace a single user request from gateway → manwe →
    /// upstream provider through state.jsonl/governance_events.jsonl.
    #[serde(default)]
    pub route_id: String,
}
