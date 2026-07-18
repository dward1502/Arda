use crate::adaptive::service::route_policy::{
    access_tier_class, configured_local_device_pressure, is_high_priority, is_local_provider,
    local_device_pressure_adjustment, near_day_quota, HybridRoutePolicy, RouteExecutionProfile,
    RouteSelectionCandidate,
};
use crate::adaptive::service::state_io::{append_jsonl, read_recent_jsonl};
use crate::adaptive::service::types::CharonService;
use serde_json::Value as JsonValue;
#[cfg(test)]
use crate::adaptive::service::types::ModelState;
use crate::adaptive::service::types::{CharonRequestEnvelope, ProviderState};
use arda_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticOutcomeClass {
    ToolProtocolLeak,
    VisibleReasoningLeak,
    MalformedStructuredOutput,
    EmptyOrTrivialCompletion,
    TaskAbandonment,
    ValidToolCall,
    ValidStructuredOutput,
    Success,
}

impl SemanticOutcomeClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ToolProtocolLeak => "tool_protocol_leak",
            Self::VisibleReasoningLeak => "visible_reasoning_leak",
            Self::MalformedStructuredOutput => "malformed_structured_output",
            Self::EmptyOrTrivialCompletion => "empty_or_trivial_completion",
            Self::TaskAbandonment => "task_abandonment",
            Self::ValidToolCall => "valid_tool_call",
            Self::ValidStructuredOutput => "valid_structured_output",
            Self::Success => "success",
        }
    }

    pub(crate) fn is_negative(self) -> bool {
        matches!(
            self,
            Self::ToolProtocolLeak
                | Self::VisibleReasoningLeak
                | Self::MalformedStructuredOutput
                | Self::EmptyOrTrivialCompletion
                | Self::TaskAbandonment
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CapabilityTruth {
    pub(crate) provider_id: String,
    pub(crate) model_id: String,
    pub(crate) tool_calls_validated: Option<bool>,
    pub(crate) structured_output_validated: Option<bool>,
    pub(crate) streaming_validated: Option<bool>,
    pub(crate) visible_reasoning_leak_seen: bool,
    pub(crate) max_reliable_context_observed: Option<usize>,
    pub(crate) last_validated_at_utc: Option<String>,
    pub(crate) evidence_route_id: Option<String>,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteExplanation {
    pub(crate) route_id: String,
    pub(crate) selected_provider_id: String,
    pub(crate) selected_model_id: String,
    pub(crate) route_class: String,
    pub(crate) execution_lane: String,
    pub(crate) score: f64,
    pub(crate) confidence: f64,
    pub(crate) fallback_tier: String,
    pub(crate) pacing_state: String,
    pub(crate) quota_state: JsonValue,
    pub(crate) score_components: JsonValue,
    pub(crate) rejected_providers: Vec<RejectedRouteCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedRouteCandidate {
    pub(crate) provider_id: String,
    pub(crate) tier: String,
    pub(crate) reason: String,
    pub(crate) pacing_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CharonEvalReceipt {
    ts_utc: String,
    eval_id: String,
    family: String,
    dry_run: bool,
    status: String,
    route_class: String,
    execution_lane: String,
    expected_origin_preference: String,
    budget_class: String,
    local_pressure_sensitive: bool,
    latency_target_ms: u64,
    expected_context_window_target: usize,
}

pub(crate) fn classify_semantic_outcome(
    response: &JsonValue,
    attempt_body: &JsonValue,
) -> SemanticOutcomeClass {
    let tool_request =
        attempt_body.get("tools").is_some() || attempt_body.get("tool_choice").is_some();
    let structured_request = attempt_body.get("response_format").is_some();
    let text = response_text(response);

    if contains_tool_protocol_leak(&text) {
        return SemanticOutcomeClass::ToolProtocolLeak;
    }
    if contains_visible_reasoning_leak(&text) {
        return SemanticOutcomeClass::VisibleReasoningLeak;
    }
    if text.trim().is_empty() || trivial_completion(&text) {
        return SemanticOutcomeClass::EmptyOrTrivialCompletion;
    }
    if task_abandonment(&text) {
        return SemanticOutcomeClass::TaskAbandonment;
    }
    if structured_request && !structured_response_valid(response, &text) {
        return SemanticOutcomeClass::MalformedStructuredOutput;
    }
    if response
        .pointer("/choices/0/message/tool_calls")
        .and_then(JsonValue::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        return SemanticOutcomeClass::ValidToolCall;
    }
    if tool_request && text.contains("\"tool_calls\"") {
        return SemanticOutcomeClass::ValidToolCall;
    }
    if structured_request {
        return SemanticOutcomeClass::ValidStructuredOutput;
    }
    SemanticOutcomeClass::Success
}

fn response_text(response: &JsonValue) -> String {
    let mut values = Vec::new();
    collect_strings(response, &mut values);
    values.join("\n")
}

fn collect_strings(value: &JsonValue, out: &mut Vec<String>) {
    match value {
        JsonValue::String(text) => out.push(text.clone()),
        JsonValue::Array(items) => {
            for item in items {
                collect_strings(item, out);
            }
        }
        JsonValue::Object(map) => {
            for value in map.values() {
                collect_strings(value, out);
            }
        }
        _ => {}
    }
}

fn contains_tool_protocol_leak(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "<|tool_call>",
        "<tool_call>",
        "</tool_call>",
        "call:execute_code",
        "call:execute_shell",
        "<tool_call|>",
        "tool_call>{",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn contains_visible_reasoning_leak(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "<think>",
        "</think>",
        "we need to use the tool",
        "i will call the tool",
        "let's issue terminal command",
        "we need to actually use the tool",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn trivial_completion(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.len() < 6 || matches!(trimmed.to_ascii_lowercase().as_str(), "ok" | "done" | "yes")
}

fn task_abandonment(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "i cannot continue",
        "i can't continue",
        "let me know if you want me to",
        "if you'd like me to",
        "i will now remove",
        "i'll proceed now",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn structured_response_valid(response: &JsonValue, text: &str) -> bool {
    if response
        .pointer("/choices/0/message/content")
        .is_some_and(JsonValue::is_object)
    {
        return true;
    }
    let trimmed = text.trim();
    trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && serde_json::from_str::<JsonValue>(trimmed).is_ok()
}

pub(crate) fn pacing_state_for_provider(
    provider: &ProviderState,
    priority: &str,
    route_profile: &RouteExecutionProfile,
) -> String {
    if provider
        .requests_per_day
        .is_some_and(|max| max > 0 && provider.requests_used_day >= max)
        || provider
            .requests_per_minute
            .is_some_and(|max| max > 0 && provider.requests_used_minute >= max)
    {
        return "exhausted".to_string();
    }
    if provider.in_cooldown {
        return "exhausted".to_string();
    }
    if lane_reserve_applies(provider, priority, route_profile) {
        return "reserved".to_string();
    }
    if near_day_quota(provider, 0.80)
        || provider.requests_per_minute.is_some_and(|max| {
            max > 0 && provider.requests_used_minute >= max.saturating_mul(4) / 5
        })
    {
        return "paced".to_string();
    }
    "available".to_string()
}

pub(crate) fn provider_available_after_pacing(
    provider: &ProviderState,
    priority: &str,
    route_profile: &RouteExecutionProfile,
) -> bool {
    match pacing_state_for_provider(provider, priority, route_profile).as_str() {
        "available" => true,
        "paced" => !matches!(
            route_profile.execution_lane.as_str(),
            "execution" | "planning"
        ),
        "reserved" => is_high_priority(priority),
        "exhausted" => false,
        _ => true,
    }
}

fn lane_reserve_applies(
    provider: &ProviderState,
    priority: &str,
    route_profile: &RouteExecutionProfile,
) -> bool {
    if is_local_provider(&provider.id) || is_high_priority(priority) {
        return false;
    }
    if !matches!(
        route_profile.execution_lane.as_str(),
        "execution" | "planning"
    ) {
        return false;
    }
    near_day_quota(provider, 0.70)
}

pub(crate) fn route_confidence(score: f64, provider: &ProviderState, pacing_state: &str) -> f64 {
    let mut confidence = (score / 140.0).clamp(0.10, 0.98);
    if provider.consecutive_failures > 0 {
        confidence -= (provider.consecutive_failures as f64 * 0.08).min(0.30);
    }
    if matches!(pacing_state, "paced" | "reserved") {
        confidence -= 0.10;
    } else if pacing_state == "exhausted" {
        confidence -= 0.35;
    }
    confidence.clamp(0.0, 1.0)
}

pub(crate) fn capability_truth_from_provider_model(
    provider: &ProviderState,
    model_id: &str,
) -> Option<CapabilityTruth> {
    let model = provider.models.iter().find(|model| model.id == model_id)?;
    Some(CapabilityTruth {
        provider_id: provider.id.clone(),
        model_id: model.id.clone(),
        tool_calls_validated: model.capabilities.tools,
        structured_output_validated: model.capabilities.structured_output,
        streaming_validated: model.capabilities.streaming.or(model.streaming_validated),
        visible_reasoning_leak_seen: model.capabilities.visible_reasoning == Some(true),
        max_reliable_context_observed: Some(model.context_window),
        last_validated_at_utc: None,
        evidence_route_id: None,
        source: "runtime_model_state".to_string(),
    })
}

pub(crate) fn capability_truth_rows(providers: &[ProviderState]) -> Vec<CapabilityTruth> {
    providers
        .iter()
        .flat_map(|provider| {
            provider
                .models
                .iter()
                .filter_map(|model| capability_truth_from_provider_model(provider, &model.id))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn build_route_explanation(
    route_id: &str,
    providers: &[ProviderState],
    candidate: &RouteSelectionCandidate,
    req: &CharonRequestEnvelope,
    priority: &str,
    policy: &HybridRoutePolicy,
    route_profile: &RouteExecutionProfile,
    rejected: Vec<RejectedRouteCandidate>,
) -> RouteExplanation {
    let provider = &providers[candidate.provider_index];
    let pacing_state = pacing_state_for_provider(provider, priority, route_profile);
    let local_device_pressure = configured_local_device_pressure();
    let local_pressure_score_adjustment =
        local_device_pressure_adjustment(provider, policy, route_profile);
    let access_tier = access_tier_class(provider);
    let latency_target_ms = lane_latency_target_ms(route_profile);
    let observed_latency_ms = provider.avg_latency_ms.or(candidate.model.avg_latency_ms);
    RouteExplanation {
        route_id: route_id.to_string(),
        selected_provider_id: provider.id.clone(),
        selected_model_id: candidate.model.id.clone(),
        route_class: route_profile.route_class.clone(),
        execution_lane: route_profile.execution_lane.clone(),
        score: candidate.score,
        confidence: route_confidence(candidate.score, provider, &pacing_state),
        fallback_tier: if is_local_provider(&provider.id) {
            "local".to_string()
        } else {
            access_tier_class(provider)
        },
        pacing_state,
        quota_state: serde_json::json!({
            "requests_used_day": provider.requests_used_day,
            "requests_per_day": provider.requests_per_day,
            "requests_used_minute": provider.requests_used_minute,
            "requests_per_minute": provider.requests_per_minute,
            "in_cooldown": provider.in_cooldown,
            "cooldown_until_utc": provider.cooldown_until_utc,
        }),
        score_components: serde_json::json!({
            "score": candidate.score,
            "priority": priority,
            "agent_id": req.agent_id,
            "task_type": req.task_type,
            "workload_role": req.options.get("workload_role").and_then(JsonValue::as_str),
            "origin_preference": policy.origin_preference,
            "privacy_tier": policy.privacy_tier,
            "budget_class": budget_class(policy),
            "cost_tier": policy.cost_tier,
            "quality_tier": policy.quality_tier,
            "require_local": policy.require_local,
            "access_tier": access_tier,
            "latency_sla_ms": policy.latency_sla_ms,
            "lane_latency_target_ms": latency_target_ms,
            "context_window_target": route_profile.context_window_target,
            "selected_model_context_window": candidate.model.context_window,
            "provider_failures": provider.consecutive_failures,
            "provider_latency_ms": provider.avg_latency_ms,
            "selected_model_latency_ms": candidate.model.avg_latency_ms,
            "observed_latency_ms": observed_latency_ms,
            "local_device_pressure": local_device_pressure,
            "local_pressure_score_adjustment": local_pressure_score_adjustment,
            "local_pressure_applied": local_pressure_score_adjustment != 0.0,
            "selection_summary": route_selection_summary(
                provider,
                policy,
                route_profile,
                local_device_pressure,
                local_pressure_score_adjustment,
            ),
        }),
        rejected_providers: rejected,
    }
}

fn budget_class(policy: &HybridRoutePolicy) -> &'static str {
    match policy.cost_tier.as_str() {
        "free" | "free_only" => "free_only",
        "low" => "free_or_low_cost",
        "high" => "paid_allowed",
        _ => "balanced",
    }
}

fn lane_latency_target_ms(route_profile: &RouteExecutionProfile) -> u64 {
    match route_profile.execution_lane.as_str() {
        "orchestrator" => 15_000,
        "execution" => 7_500,
        "compression" => 20_000,
        "background" => 30_000,
        "planning" => 12_000,
        _ => 5_000,
    }
}

fn route_selection_summary(
    provider: &ProviderState,
    policy: &HybridRoutePolicy,
    route_profile: &RouteExecutionProfile,
    local_device_pressure: Option<f64>,
    local_pressure_score_adjustment: f64,
) -> String {
    if policy.require_local {
        return "local route required by privacy/offline policy".to_string();
    }
    if is_local_provider(&provider.id) {
        if local_pressure_score_adjustment < 0.0 {
            return format!(
                "local route selected despite device pressure {:.2} because final score won",
                local_device_pressure.unwrap_or_default()
            );
        }
        if policy.origin_preference == "local" {
            return "local route selected by explicit local origin preference".to_string();
        }
        return "local route selected because latency, capability, budget, and quota scored best"
            .to_string();
    }
    if matches!(access_tier_class(provider).as_str(), "free_cloud" | "mixed") {
        if route_profile.execution_lane == "compression" {
            return "cloud/free route selected for compression after latency, pressure, and quota scoring"
                .to_string();
        }
        return "cloud/free route selected because capability, latency, budget, and quota scored best"
            .to_string();
    }
    "paid or subscription route selected because quality/capability scoring beat cheaper candidates"
        .to_string()
}

impl CharonService {
    pub(crate) fn recent_semantic_failures(&self, limit: usize) -> Vec<JsonValue> {
        read_recent_jsonl(&self.tool_fit_ledger_path, limit)
            .into_iter()
            .filter(|row| {
                row.get("outcome_class")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|class| {
                        matches!(
                            class,
                            "tool_protocol_leak"
                                | "visible_reasoning_leak"
                                | "malformed_structured_output"
                                | "empty_or_trivial_completion"
                                | "task_abandonment"
                        )
                    })
            })
            .collect()
    }

    pub async fn charon_eval(&self, dry_run: bool) -> Result<JsonValue> {
        let evals = [
            (
                "hermes_tool_call",
                "tool_oriented",
                "execution",
                "auto",
                "balanced",
                true,
                7_500,
                32_000,
            ),
            (
                "multi_tool_turn",
                "tool_oriented",
                "execution",
                "auto",
                "balanced",
                true,
                7_500,
                64_000,
            ),
            (
                "code_generation",
                "tool_oriented",
                "execution",
                "auto",
                "balanced",
                true,
                7_500,
                64_000,
            ),
            (
                "structured_json",
                "structured_output",
                "planning",
                "auto",
                "balanced",
                false,
                12_000,
                32_000,
            ),
            (
                "long_context_planning",
                "long_context",
                "orchestrator",
                "auto",
                "paid_allowed",
                true,
                15_000,
                128_000,
            ),
            (
                "streaming_response",
                "streaming",
                "interactive",
                "auto",
                "balanced",
                false,
                5_000,
                16_000,
            ),
            (
                "summary_background",
                "summary",
                "background",
                "auto",
                "free_or_low_cost",
                false,
                30_000,
                16_000,
            ),
            (
                "context_compression",
                "compression",
                "compression",
                "auto",
                "free_or_low_cost",
                true,
                20_000,
                64_000,
            ),
            (
                "compaction_large_context",
                "compression",
                "compression",
                "auto",
                "paid_allowed",
                true,
                20_000,
                128_000,
            ),
            (
                "private_local_preprocess",
                "private_local",
                "background",
                "local",
                "local_only",
                false,
                30_000,
                16_000,
            ),
            (
                "health_probe_low_end",
                "health_probe",
                "monitoring",
                "auto",
                "free_or_low_cost",
                false,
                5_000,
                1_024,
            ),
        ];
        let receipts = evals
            .iter()
            .map(
                |(
                    family,
                    route_class,
                    execution_lane,
                    expected_origin_preference,
                    budget_class,
                    local_pressure_sensitive,
                    latency_target_ms,
                    expected_context_window_target,
                )| CharonEvalReceipt {
                    ts_utc: Utc::now().to_rfc3339(),
                    eval_id: format!("charon_eval_v1_{family}"),
                    family: (*family).to_string(),
                    dry_run,
                    status: if dry_run { "planned" } else { "queued" }.to_string(),
                    route_class: (*route_class).to_string(),
                    execution_lane: (*execution_lane).to_string(),
                    expected_origin_preference: (*expected_origin_preference).to_string(),
                    budget_class: (*budget_class).to_string(),
                    local_pressure_sensitive: *local_pressure_sensitive,
                    latency_target_ms: *latency_target_ms,
                    expected_context_window_target: *expected_context_window_target,
                },
            )
            .collect::<Vec<_>>();
        if !dry_run {
            for receipt in &receipts {
                append_jsonl(&self.charon_eval_receipts_path(), receipt)?;
            }
        }
        Ok(serde_json::json!({
            "ok": true,
            "dry_run": dry_run,
            "receipt_path": self.charon_eval_receipts_path(),
            "evals": receipts,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model(id: &str) -> ModelState {
        ModelState {
            aliases: vec![],
            id: id.to_string(),
            capable_tasks: vec!["chat".to_string(), "summary".to_string()],
            context_window: 128_000,
            is_default: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: Some(2_000),
            cost_per_million_tokens_in: Some(0.0),
            cost_per_million_tokens_out: Some(0.0),
            capabilities: crate::types::ModelCapabilities::default(),
            streaming_validated: None,
        }
    }

    fn test_provider(id: &str, access_tier: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: id.to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            api_key_env: None,
            access_tier: access_tier.to_string(),
            quality_band: "medium".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 0,
            requests_per_minute: Some(60),
            requests_used_minute: 3,
            minute_window_started_utc: None,
            requests_per_day: Some(1_000),
            requests_used_day: 10,
            day_window_started_utc: None,
            models: vec![test_model("test-model")],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 4,
            last_error: None,
            avg_latency_ms: Some(2_500),
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        }
    }

    #[test]
    fn semantic_classifier_detects_tool_protocol_leak() {
        let response = serde_json::json!({
            "choices": [{"message": {"content": "<|tool_call>call:execute_code{code:\"ls\"}<|tool_call|>"}}]
        });
        let body = serde_json::json!({"tools": [{"type": "function"}]});
        assert_eq!(
            classify_semantic_outcome(&response, &body),
            SemanticOutcomeClass::ToolProtocolLeak
        );
    }

    #[test]
    fn route_explanation_exposes_low_power_governor_signals() {
        std::env::set_var("ARDA_CHARON_LOCAL_DEVICE_PRESSURE", "0.82");
        let provider = test_provider("openrouter", "free_cloud");
        let providers = vec![provider.clone()];
        let candidate = RouteSelectionCandidate {
            provider_index: 0,
            model: provider.models[0].clone(),
            score: 142.0,
        };
        let req = CharonRequestEnvelope {
            agent_id: "hermes".to_string(),
            task_type: "summary".to_string(),
            priority: "normal".to_string(),
            messages: vec![serde_json::json!({"role":"user","content":"compact this context"})],
            options: serde_json::json!({"workload_role": "compression"}),
        };
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "low".to_string(),
            quality_tier: "medium".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.05,
            spread_top_cap: 4,
        };
        let profile = RouteExecutionProfile {
            route_class: "compression".to_string(),
            execution_lane: "compression".to_string(),
            context_window_target: 64_000,
        };

        let explanation = build_route_explanation(
            "route_test",
            &providers,
            &candidate,
            &req,
            "normal",
            &policy,
            &profile,
            vec![],
        );

        std::env::remove_var("ARDA_CHARON_LOCAL_DEVICE_PRESSURE");
        assert_eq!(explanation.execution_lane, "compression");
        assert_eq!(
            explanation
                .score_components
                .get("origin_preference")
                .and_then(JsonValue::as_str),
            Some("auto")
        );
        assert_eq!(
            explanation
                .score_components
                .get("budget_class")
                .and_then(JsonValue::as_str),
            Some("free_or_low_cost")
        );
        assert_eq!(
            explanation
                .score_components
                .get("local_device_pressure")
                .and_then(JsonValue::as_f64),
            Some(0.82)
        );
        assert_eq!(
            explanation
                .score_components
                .get("lane_latency_target_ms")
                .and_then(JsonValue::as_u64),
            Some(20_000)
        );
        assert!(explanation
            .score_components
            .get("selection_summary")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .contains("compression"));
    }

    #[test]
    fn semantic_classifier_detects_visible_reasoning_leak() {
        let response = serde_json::json!({
            "choices": [{"message": {"content": "We need to use the tool. I will call the tool now."}}]
        });
        assert_eq!(
            classify_semantic_outcome(&response, &serde_json::json!({})),
            SemanticOutcomeClass::VisibleReasoningLeak
        );
    }

    #[test]
    fn pacing_exhausts_provider_at_daily_limit() {
        let provider = ProviderState {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            api_key_env: Some("OPENROUTER_API_KEY".to_string()),
            access_tier: "free_cloud".to_string(),
            quality_band: "medium".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 0,
            requests_per_minute: Some(20),
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: Some(50),
            requests_used_day: 50,
            day_window_started_utc: None,
            models: vec![],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };
        assert_eq!(
            pacing_state_for_provider(&provider, "normal", &profile),
            "exhausted"
        );
        assert!(!provider_available_after_pacing(
            &provider, "normal", &profile
        ));
    }
}