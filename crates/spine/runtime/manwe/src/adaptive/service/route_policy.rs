// PackageRuntimeSignals is consumed by the route_policy_tests child module.
// The dead-code lint doesn't trace cfg(test) re-uses, so keep it gated.
pub(super) use super::route_scoring::parse_model_params_billions;
pub(super) use super::route_scoring::{
    access_tier_class, apply_soft_lane_caps, configured_local_device_pressure,
    is_background_priority, is_high_priority, is_local_fallback, is_local_provider,
    is_primary_local_surface_provider, local_device_pressure_adjustment, near_day_quota,
    provider_score,
};
#[cfg(test)]
#[allow(unused_imports)]
use crate::adaptive::service::status::PackageRuntimeSignals;
use crate::adaptive::service::types::{
    CharonRequestEnvelope, ModelState, ProviderState, RouteDecision, RouteGovernance,
    RouteGovernanceLens, RouteLoveEquationGuard,
};
use arda_core::JouleWorkMeasurementSource;
use arda_governance::{
    calculate_resonance_with_governance_chain, evaluate_governance_chain, profile_joulework,
    GateOutcome, GovernanceChainConfig, GovernanceChainResult, GovernanceLensConfig,
    GovernanceReviewMode, TriadPuritySource,
};
use arda_economics::LoveEquation;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub(super) struct HybridRoutePolicy {
    pub(super) privacy_tier: String,
    pub(super) cost_tier: String,
    pub(super) quality_tier: String,
    pub(super) origin_preference: String,
    pub(super) latency_sla_ms: Option<u64>,
    pub(super) require_local: bool,
    pub(super) spread_score_band: f64,
    pub(super) spread_top_cap: usize,
}

#[derive(Debug, Clone)]
pub(super) struct RouteExecutionProfile {
    pub(super) route_class: String,
    pub(super) execution_lane: String,
    pub(super) context_window_target: usize,
}

#[derive(Debug, Clone)]
pub(super) struct RouteSelectionCandidate {
    pub(super) provider_index: usize,
    pub(super) model: ModelState,
    pub(super) score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct LaneFitnessSnapshot {
    pub(super) generated_at_utc: String,
    pub(super) lanes:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, LaneFitnessState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct LaneFitnessState {
    pub(super) avg_latency_ms: Option<u64>,
    pub(super) success_count: u64,
    pub(super) failure_count: u64,
    pub(super) last_result_utc: Option<String>,
}

fn option_string<'a>(options: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    options.get(key).and_then(|v| v.as_str())
}

fn context_target_for_priority(value: Option<&str>, default_value: usize) -> usize {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "low" => 16_000,
        "medium" => default_value,
        "high" => default_value.max(128_000),
        _ => default_value,
    }
}

fn compute_route_love_equation(
    agent_id: &str,
    provider_id: &str,
    score: f64,
    triad_passed: bool,
    strict: bool,
) -> RouteLoveEquationGuard {
    let resonance = (score / 100.0).clamp(0.0, 1.0);
    let attention = if strict { 0.82 } else { 0.68 };
    let reciprocity = if triad_passed { 0.76 } else { 0.44 };
    let score =
        LoveEquation::new().calculate(agent_id, provider_id, resonance, attention, reciprocity);
    RouteLoveEquationGuard {
        resonance,
        attention,
        reciprocity,
        score,
    }
}

fn option_string_value<'a>(options: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| option_string(options, key))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn route_governance_chain_config(
    options: &serde_json::Value,
    default_chain: &GovernanceChainConfig,
) -> GovernanceChainConfig {
    let method = option_string_value(options, &["governance_method", "philosopher_method"])
        .unwrap_or("triad")
        .to_ascii_lowercase();
    if matches!(
        method.as_str(),
        "single" | "single_philosopher" | "philosopher"
    ) {
        let lens_id = option_string_value(
            options,
            &["governance_philosopher", "philosopher_lens", "philosopher"],
        )
        .unwrap_or("bacon")
        .to_ascii_lowercase();
        return GovernanceChainConfig {
            chain_id: format!("single_{lens_id}"),
            chain_version: default_chain.chain_version.clone(),
            profile_source: default_chain.profile_source.clone(),
            review_mode: default_chain.review_mode,
            profile_maturity: default_chain.profile_maturity.clone(),
            strict: true,
            required_passes: Some(1),
            autonomous_blocking_enabled: false,
            lenses: vec![GovernanceLensConfig {
                id: lens_id.clone(),
                display_name: display_name_for_lens(&lens_id).to_string(),
                profile_id: Some(lens_id),
                pass_threshold: 0.50,
            }],
            ..GovernanceChainConfig::default_triad()
        };
    }

    let mut config = default_chain.clone();
    if matches!(method.as_str(), "chain" | "governance_chain") {
        config.strict = true;
        config.required_passes = Some(config.lenses.len() as u32);
    }
    config
}

fn governance_method_label(options: &serde_json::Value) -> String {
    option_string_value(options, &["governance_method", "philosopher_method"])
        .unwrap_or("triad")
        .to_ascii_lowercase()
}

fn display_name_for_lens(lens_id: &str) -> &str {
    match lens_id {
        "aurelius" => "Marcus Aurelius",
        "bacon" => "Francis Bacon",
        "sun_tzu" => "Sun Tzu",
        _ => lens_id,
    }
}

fn gate_outcome_label(outcome: GateOutcome) -> String {
    match outcome {
        GateOutcome::Pass => "pass",
        GateOutcome::Conditional => "conditional",
        GateOutcome::Fail => "fail",
    }
    .to_string()
}

fn review_mode_label(mode: GovernanceReviewMode) -> String {
    match mode {
        GovernanceReviewMode::HeuristicLocal => "heuristic_local",
        GovernanceReviewMode::IndependentAgent => "independent_agent",
        GovernanceReviewMode::HumanReviewed => "human_reviewed",
        GovernanceReviewMode::ConsensusReceipted => "consensus_receipted",
    }
    .to_string()
}

fn triad_purity_source_label(source: TriadPuritySource) -> String {
    match source {
        TriadPuritySource::LiveTriad => "live_triad",
        TriadPuritySource::LiveGovernanceChain => "live_governance_chain",
        TriadPuritySource::Absent => "absent",
        TriadPuritySource::CompatibilityDefault => "compatibility_default",
    }
    .to_string()
}

fn joule_measurement_source_label(source: JouleWorkMeasurementSource) -> String {
    match source {
        JouleWorkMeasurementSource::OperatorEstimate => "operator_estimate",
        JouleWorkMeasurementSource::DefaultFallback => "default_fallback",
        JouleWorkMeasurementSource::RuntimeTimer => "runtime_timer",
        JouleWorkMeasurementSource::ProcessResourceSample => "process_resource_sample",
        JouleWorkMeasurementSource::ProviderUsageReport => "provider_usage_report",
        JouleWorkMeasurementSource::ExternalPowerMeter => "external_power_meter",
    }
    .to_string()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_route_decision(
    provider: &ProviderState,
    model: ModelState,
    score: f64,
    req: &CharonRequestEnvelope,
    priority: &str,
    strict: bool,
    policy: &HybridRoutePolicy,
    route_profile: &RouteExecutionProfile,
) -> RouteDecision {
    let governance = RouteGovernance {
        triad_passed: true,
        triad_aurelius_score: 1.0,
        triad_bacon_score: 1.0,
        triad_sun_tzu_score: 1.0,
        love_equation_guard: compute_route_love_equation(
            &req.agent_id,
            &provider.id,
            score,
            true,
            strict,
        ),
        ..RouteGovernance::default()
    };
    RouteDecision {
        provider_id: provider.id.clone(),
        model_id: model.id,
        reason: format!(
            "policy route priority={} strict={} score={:.2} task_type={} privacy={} cost={} quality={} origin={} latency_sla_ms={} lane={} class={} context_target={}",
            priority,
            strict,
            score,
            req.task_type,
            policy.privacy_tier,
            policy.cost_tier,
            policy.quality_tier,
            policy.origin_preference,
            policy.latency_sla_ms.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            route_profile.execution_lane,
            route_profile.route_class,
            route_profile.context_window_target,
        ),
        route_class: route_profile.route_class.clone(),
        execution_lane: route_profile.execution_lane.clone(),
        context_window_target: route_profile.context_window_target,
        governance,
        // route_id is filled in by CharonService::route once it knows the
        // call is a real route (not a /route preview). Default-empty here.
        route_id: String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_route_decision_with_governance_chain(
    provider: &ProviderState,
    model: ModelState,
    score: f64,
    req: &CharonRequestEnvelope,
    priority: &str,
    strict: bool,
    policy: &HybridRoutePolicy,
    route_profile: &RouteExecutionProfile,
    route_task: &arda_core::Task,
    chain: GovernanceChainResult,
) -> RouteDecision {
    let mut decision = build_route_decision(
        provider,
        model,
        score,
        req,
        priority,
        strict,
        policy,
        route_profile,
    );
    let resonance = calculate_resonance_with_governance_chain(route_task, &chain, None, None);
    let components = resonance.ecst_components.as_ref();
    let joule = profile_joulework(route_task);
    decision.governance = RouteGovernance {
        triad_passed: chain.passed,
        triad_aurelius_score: lens_score(&chain, "aurelius"),
        triad_bacon_score: lens_score(&chain, "bacon"),
        triad_sun_tzu_score: lens_score(&chain, "sun_tzu"),
        love_equation_guard: compute_route_love_equation(
            &req.agent_id,
            &provider.id,
            score,
            chain.passed,
            strict,
        ),
        governance_method: governance_method_label(&req.options),
        chain_id: chain.chain_id.clone(),
        chain_version: chain.chain_version.clone(),
        profile_source: chain.profile_source.clone(),
        review_mode: review_mode_label(chain.review_mode),
        profile_maturity: chain.profile_maturity.clone(),
        autonomous_blocking_enabled: chain.autonomous_blocking_enabled,
        veto_reason: chain.veto_reason.clone(),
        lenses: chain
            .lenses
            .iter()
            .map(|lens| RouteGovernanceLens {
                lens_id: lens.lens_id.clone(),
                display_name: lens.display_name.clone(),
                profile_id: lens.profile_id.clone(),
                outcome: gate_outcome_label(lens.outcome),
                score: lens.score,
                pass_threshold: lens.pass_threshold,
            })
            .collect(),
        resonance_score: resonance.value,
        triad_purity_source: components
            .and_then(|c| c.triad_purity_source)
            .map(triad_purity_source_label),
        love_projected_empathy: components.and_then(|c| c.love_projected_empathy),
        love_delta_empathy: components.and_then(|c| c.love_delta_empathy),
        philosopher_action: resonance
            .triad_philosopher
            .as_ref()
            .map(|verdict| format!("{:?}", verdict.action).to_ascii_lowercase()),
        philosopher_alignment_score: resonance
            .triad_philosopher
            .as_ref()
            .map(|verdict| verdict.alignment_score),
        joule_measurement_source: joule_measurement_source_label(joule.measurement_source),
        joule_measurement_confidence: joule.measurement_confidence,
        joule_autonomy_truth_allowed: joule.autonomy_truth_allowed,
    };
    decision
}

pub(super) fn evaluate_route_governance_chain(
    route_task: &arda_core::Task,
    options: &serde_json::Value,
    default_chain: &GovernanceChainConfig,
) -> GovernanceChainResult {
    let chain_config = route_governance_chain_config(options, default_chain);
    let result = evaluate_governance_chain(route_task, &chain_config);
    if matches!(
        governance_method_label(options).as_str(),
        "single" | "single_philosopher" | "philosopher"
    ) {
        normalize_single_lens_result(result)
    } else {
        result
    }
}

fn lens_score(chain: &GovernanceChainResult, lens_id: &str) -> f64 {
    chain
        .lenses
        .iter()
        .find(|lens| lens.lens_id == lens_id)
        .map(|lens| lens.score)
        .unwrap_or(0.0)
}

fn normalize_single_lens_result(mut result: GovernanceChainResult) -> GovernanceChainResult {
    if result.lenses.len() != 1 {
        return result;
    }
    let lens = &mut result.lenses[0];
    if lens.outcome != GateOutcome::Pass {
        lens.outcome = GateOutcome::Fail;
        result.passed = false;
        result.veto_reason = Some(format!("{}_FAIL", lens.lens_id.to_ascii_uppercase()));
    } else {
        result.passed = true;
        result.veto_reason = None;
    }
    result
}

// Convenience wrapper used by tests; production code calls the *_for_request
// variant directly. Gated to test builds to avoid dead-code warnings.
#[cfg(test)]
pub(super) fn select_model(
    models: &[ModelState],
    task_type: &str,
    forced_model_id: Option<&str>,
) -> Option<ModelState> {
    select_model_for_request("", models, task_type, forced_model_id, None)
}

pub(super) fn select_model_for_request(
    provider_id: &str,
    models: &[ModelState],
    task_type: &str,
    forced_model_id: Option<&str>,
    req: Option<&CharonRequestEnvelope>,
) -> Option<ModelState> {
    if let Some(forced_model_id) = forced_model_id {
        return models
            .iter()
            .find(|m| {
                (m.id == forced_model_id || m.alias_matches(forced_model_id))
                    && m.healthy
                    && !m.in_cooldown
                    && model_supports_request(provider_id, m, req)
            })
            .cloned();
    }
    let excluded_model_ids = req
        .map(|req| excluded_model_ids(&req.options))
        .unwrap_or_default();
    let model_is_excluded = |model: &ModelState| {
        excluded_model_ids
            .iter()
            .any(|excluded| model.id == *excluded || model.alias_matches(excluded))
    };
    let mut candidates = models
        .iter()
        .filter(|m| m.healthy && !m.in_cooldown)
        .filter(|m| !model_is_excluded(m))
        .filter(|m| m.capable_tasks.iter().any(|t| t == task_type))
        .filter(|m| model_supports_request(provider_id, m, req))
        .cloned()
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        candidates = models
            .iter()
            .filter(|m| m.healthy && !m.in_cooldown)
            .filter(|m| !model_is_excluded(m))
            .filter(|m| m.is_default)
            .filter(|m| model_supports_request(provider_id, m, req))
            .cloned()
            .collect::<Vec<_>>();
    }
    candidates.into_iter().max_by_key(|m| {
        (
            task_affinity_score(m, task_type),
            usize::from(m.is_default),
            m.context_window,
        )
    })
}

pub(super) fn select_model_for_provider_request(
    provider: &ProviderState,
    task_type: &str,
    forced_model_id: Option<&str>,
    req: Option<&CharonRequestEnvelope>,
) -> Option<ModelState> {
    if forced_model_id.is_none()
        && req
            .and_then(|req| req.options.get("prefer_probe_model"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    {
        if let Some(probe_model) = provider.probe_model.as_deref() {
            if let Some(model) = provider.models.iter().find(|model| {
                (model.id == probe_model || model.alias_matches(probe_model))
                    && model.healthy
                    && !model.in_cooldown
                    && model.capable_tasks.iter().any(|task| task == task_type)
                    && model_supports_request(&provider.id, model, req)
            }) {
                return Some(model.clone());
            }
        }
    }
    select_model_for_request(
        &provider.id,
        &provider.models,
        task_type,
        forced_model_id,
        req,
    )
}

pub(super) fn candidate_models_for_provider_request(
    provider: &ProviderState,
    task_type: &str,
    forced_model_id: Option<&str>,
    req: Option<&CharonRequestEnvelope>,
) -> Vec<ModelState> {
    if let Some(forced_model_id) = forced_model_id {
        return select_model_for_provider_request(provider, task_type, Some(forced_model_id), req)
            .into_iter()
            .collect();
    }

    let excluded_model_ids = req
        .map(|req| excluded_model_ids(&req.options))
        .unwrap_or_default();
    let model_is_excluded = |model: &ModelState| {
        excluded_model_ids
            .iter()
            .any(|excluded| model.id == *excluded || model.alias_matches(excluded))
    };
    let mut candidates = provider
        .models
        .iter()
        .filter(|m| m.healthy && !m.in_cooldown)
        .filter(|m| !model_is_excluded(m))
        .filter(|m| m.capable_tasks.iter().any(|t| t == task_type) || m.is_default)
        .filter(|m| model_supports_request(&provider.id, m, req))
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by_key(|m| {
        (
            task_affinity_score(m, task_type),
            usize::from(m.is_default),
            m.context_window,
        )
    });
    candidates.reverse();

    if req
        .and_then(|req| req.options.get("prefer_probe_model"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        if let Some(probe_model) = provider.probe_model.as_deref() {
            if let Some(idx) = candidates
                .iter()
                .position(|model| model.id == probe_model || model.alias_matches(probe_model))
            {
                let probe = candidates.remove(idx);
                candidates.insert(0, probe);
            }
        }
    }

    candidates
}

fn task_affinity_score(model: &ModelState, task_type: &str) -> u8 {
    let model_id = model.id.to_ascii_lowercase();
    match task_type {
        "code" => {
            if contains_any(
                &model_id,
                &[
                    "codestral",
                    "devstral",
                    "coder",
                    "codegemma",
                    "starcoder",
                    "deepseek-coder",
                ],
            ) {
                4
            } else if contains_any(
                &model_id,
                &["nemotron", "glm-5", "glm-5.1", "glm-4.7", "qwen"],
            ) {
                3
            } else {
                1
            }
        }
        "reasoning" => {
            if contains_any(
                &model_id,
                &[
                    "reason",
                    "think",
                    "magistral",
                    "nemotron",
                    "glm",
                    "deepseek-v3",
                ],
            ) {
                4
            } else if contains_any(&model_id, &["mistral-medium", "deepseek", "gpt-oss"]) {
                3
            } else {
                1
            }
        }
        "research" => {
            if model.context_window >= 128_000
                || contains_any(
                    &model_id,
                    &["medium", "ultra", "large", "deepseek-v3", "glm"],
                )
            {
                3
            } else {
                1
            }
        }
        "chat" | "summary" | "background" => {
            if model.is_default {
                2
            } else {
                1
            }
        }
        _ => 1,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn contains_token(text: &str, needle: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| token == needle)
}

pub(super) fn model_has_visible_reasoning_surface(model: &ModelState) -> bool {
    if let Some(visible_reasoning) = model.capabilities.visible_reasoning {
        return visible_reasoning;
    }

    std::iter::once(model.id.as_str())
        .chain(model.aliases.iter().map(String::as_str))
        .map(str::to_ascii_lowercase)
        .any(|signal| {
            contains_any(
                &signal,
                &[
                    "thinking",
                    "reasoning",
                    "reasoner",
                    "gpt-oss",
                    "lfm2.5",
                    "lfm-2.5",
                    "deepseek-r1",
                    "r1-distill",
                ],
            ) || contains_token(&signal, "r1")
        })
}

fn request_allows_visible_reasoning(req: &CharonRequestEnvelope) -> bool {
    req.task_type == "reasoning"
        || [
            "allow_visible_reasoning",
            "allow_thinking_models",
            "require_reasoning_model",
        ]
        .iter()
        .any(|key| {
            req.options
                .get(key)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
}

pub(super) fn excluded_provider_ids(options: &serde_json::Value) -> Vec<String> {
    options
        .get("exclude_provider_ids")
        .or_else(|| options.get("excluded_provider_ids"))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn excluded_model_ids(options: &serde_json::Value) -> Vec<String> {
    options
        .get("exclude_model_ids")
        .or_else(|| options.get("excluded_model_ids"))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn request_allows_hermes_cli_fast_lane(req: &CharonRequestEnvelope) -> bool {
    req.options
        .get("allow_hermes_cli")
        .or_else(|| req.options.get("allow_slow_subscription_routes"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub(super) fn decay_lane_fitness_snapshot(snapshot: &mut LaneFitnessSnapshot) -> bool {
    let now = Utc::now();
    let half_life_hours = lane_fitness_half_life_hours();
    let prune_hours = lane_fitness_prune_hours();
    let mut changed = false;

    snapshot.lanes.retain(|_, providers| {
        providers.retain(|_, state| {
            let Some(last_result_utc) = state.last_result_utc.as_deref() else {
                changed = true;
                return false;
            };
            let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(last_result_utc) else {
                changed = true;
                return false;
            };
            let age_hours =
                ((now - parsed.with_timezone(&Utc)).num_seconds().max(0) as f64) / 3600.0;
            if age_hours >= prune_hours {
                changed = true;
                return false;
            }
            if age_hours <= 0.0 {
                return true;
            }
            let decay_factor = 0.5_f64.powf(age_hours / half_life_hours.max(0.25));
            let decayed_success = ((state.success_count as f64) * decay_factor).round() as u64;
            let decayed_failure = ((state.failure_count as f64) * decay_factor).round() as u64;
            if decayed_success != state.success_count || decayed_failure != state.failure_count {
                state.success_count = decayed_success;
                state.failure_count = decayed_failure;
                changed = true;
            }
            if state.success_count == 0 && state.failure_count == 0 {
                changed = true;
                return false;
            }
            true
        });
        !providers.is_empty()
    });

    if changed {
        snapshot.generated_at_utc = now.to_rfc3339();
    }
    changed
}

fn lane_fitness_half_life_hours() -> f64 {
    std::env::var("ARDA_CHARON_LANE_FITNESS_HALF_LIFE_HOURS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(12.0)
}

fn lane_fitness_prune_hours() -> f64 {
    std::env::var("ARDA_CHARON_LANE_FITNESS_PRUNE_HOURS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(72.0)
}

fn parse_freeze_until(
    p: &ProviderState,
) -> Option<chrono::DateTime<chrono::Utc>> {
    p.cooldown_until_utc
        .as_deref()?
        .parse::<chrono::DateTime<chrono::Utc>>()
        .ok()
}

fn provider_freeze_threshold(p: &ProviderState) -> u32 {
    match p.access_tier.as_str() {
        "paid_cloud" | "local" => 2,
        _ => 3,
    }
}

fn model_consecutive_failures(models: &[ModelState]) -> u32 {
    models.iter().map(|m| m.consecutive_failures).max().unwrap_or(0)
}

pub(super) fn provider_freeze_banned(p: &ProviderState) -> bool {
    parse_freeze_until(p)
        .map(|until| Utc::now() < until)
        .unwrap_or(false)
}

pub(super) fn provider_freeze_gate_blackout_allowed(
    p: &ProviderState,
) -> bool {
    if provider_freeze_banned(p) {
        return true;
    }
    provider_freeze_request_has_clear_failure(p)
}

pub(super) fn provider_freeze_request_has_clear_failure(p: &ProviderState) -> bool {
    !matches!(p.access_tier.as_str(), "paid_cloud" | "local") && p.consecutive_failures > 0
        || p.last_error.as_deref().is_some_and(|raw| {
            !raw.is_empty()
                && !contains_any(
                    raw,
                    &[
                        "transport_failure",
                        "tls",
                        "dns",
                        "connection refused",
                        "client_payload_error",
                        "invalid_request",
                        "none",
                    ],
                )
        })
}

pub(super) fn provider_freeze_record_metadata(
    p: &ProviderState,
) -> (String, String) {
    let banned = provider_freeze_banned(p);
    match p.access_tier.as_str() {
        "paid_cloud" | "local" => {
            ("heavy_frz".to_string(), if banned { "freeze" } else { "retain" }.to_string())
        }
        _ => (
            "light_frz".to_string(),
            if banned { "freeze" } else { "retry" }.to_string(),
        ),
    }
}

pub(super) fn provider_freeze_failure_class_allowed(p: &ProviderState) -> bool {
    matches!(p.access_tier.as_str(), "paid_cloud" | "local")
}

pub(super) fn gate_update_freeze_until(
    p: &mut ProviderState,
    freeze_until: Option<String>,
) {
    if freeze_until
        .as_deref()
        .is_some_and(|raw| !raw.trim().is_empty())
    {
        p.cooldown_until_utc = freeze_until;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GateFreezeMetadataRequest {
    pub provider_id: String,
    pub access_tier_class: String,
}

pub(super) fn gate_request_freeze_metadata(
    p: &ProviderState,
    _req: &CharonRequestEnvelope,
) -> GateFreezeMetadataRequest {
    GateFreezeMetadataRequest {
        provider_id: p.id.clone(),
        access_tier_class: gate_provider_freeze_recovery_tier(p).to_string(),
    }
}

pub(super) fn gate_provider_freeze_recovery_tier(p: &ProviderState) -> &'static str {
    match p.access_tier.as_str() {
        "paid_cloud" | "local" => "fallback_instance",
        _ => "alternate_route",
    }
}

pub(super) fn gate_blackout_freeze_parameters() -> GateFreezeParameters {
    GateFreezeParameters {
        min_heavy_requests_below_ceiling: 2,
        min_heavy_requests_below_ceiling_scoped: 4,
        min_contiguous_failures_per_tier_check: 3,
        min_contiguous_failures_per_tier_band: 3,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GateFreezeParameters {
    pub(super) min_heavy_requests_below_ceiling: u32,
    pub(super) min_heavy_requests_below_ceiling_scoped: u32,
    pub(super) min_contiguous_failures_per_tier_check: u32,
    pub(super) min_contiguous_failures_per_tier_band: u32,
}

pub(super) fn gate_blackout_freeze_metadata(p: &ProviderState) -> GateFreezeRecordMetadata {
    GateFreezeRecordMetadata {
        operator_action: match p.access_tier.as_str() {
            "paid_cloud" | "local" => "freeze",
            _ => "escalate",
        },
        recovery_tier: gate_provider_freeze_recovery_tier(p).to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GateFreezeRecordMetadata {
    pub(super) operator_action: &'static str,
    pub(super) recovery_tier: String,
}

pub(super) fn gate_blackout_freeze_tag(p: &ProviderState) -> Option<String> {
    match p.access_tier.as_str() {
        "paid_cloud" | "local" => Some("heavy_freeze".to_string()),
        _ => {
            if provider_freeze_banned(p) {
                Some("light_freeze_banned".to_string())
            } else {
                Some("light_freeze_escalated".to_string())
            }
        }
    }
}

pub(super) fn gate_blackout_requires_freeze_clearance(p: &ProviderState) -> bool {
    if provider_freeze_banned(p) && !provider_freeze_failure_class_allowed(p) {
        return true;
    }
    matches!(p.access_tier.as_str(), "paid_cloud" | "local") && !provider_freeze_banned(p)
}

pub(super) fn provider_freeze_current_until_utc(p: &ProviderState) -> Option<String> {
    parse_freeze_until(p).map(|value| value.to_rfc3339())
}

pub(super) fn gate_update_blackout_freeze_until(p: &mut ProviderState, until: Option<String>) {
    p.cooldown_until_utc = until;
}

pub(super) fn provider_eligible(p: &ProviderState, priority: &str, strict: bool) -> bool {
    if !p.enabled || !p.has_api_key || !p.healthy || p.in_cooldown {
        return false;
    }
    if provider_freeze_banned(p) {
        return false;
    }
    if !provider_half_open_probe_allowed(p) {
        return false;
    }
    if p.requests_per_minute
        .is_some_and(|max| p.requests_used_minute >= max)
    {
        return false;
    }
    if p.requests_per_day
        .is_some_and(|max| p.requests_used_day >= max)
    {
        return false;
    }
    if strict && near_day_quota(p, 0.85) {
        return false;
    }
    let threshold = provider_freeze_threshold(&p);
    let consecutive = p.consecutive_failures.saturating_add(model_consecutive_failures(&p.models));
    if consecutive >= threshold {
        return false;
    }
    if is_high_priority(priority) && p.consecutive_failures >= 2 {
        return false;
    }
    true
}

/// Same as `provider_eligible` but ignores short transient cooldowns. Used as a
/// last-resort escape valve when all providers are simultaneously in cooldown.
///
/// Deliberately does not bypass long/account/model cooldowns. Those cooldowns
/// are not transient liveness blips; retrying them just burns attempts and can
/// starve the fallback chain.
pub(super) fn provider_eligible_ignoring_cooldown(p: &ProviderState, priority: &str) -> bool {
    if !p.enabled || !p.has_api_key || !p.healthy {
        return false;
    }
    if p.cooldown_until_utc
        .as_deref()
        .is_some_and(|raw| !raw.trim().is_empty())
    {
        return false;
    }
    if !provider_half_open_probe_allowed(p) {
        return false;
    }
    if p.requests_per_day
        .is_some_and(|max| p.requests_used_day >= max)
    {
        return false;
    }
    let threshold = provider_freeze_threshold(&p).saturating_add(2);
    let consecutive = p.consecutive_failures.saturating_add(model_consecutive_failures(&p.models));
    if consecutive >= threshold {
        return false;
    }
    if is_high_priority(priority) && p.consecutive_failures >= 5 {
        return false;
    }
    true
}

pub(super) fn provider_cooldown_bypass_allowed(p: &ProviderState) -> bool {
    if !p.in_cooldown {
        return true;
    }
    if p.cooldown_backoff_seconds > cooldown_bypass_max_seconds() {
        return false;
    }
    let last_error = p
        .last_error
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if last_error.is_empty() {
        return true;
    }
    ![
        "insufficient balance",
        "billing",
        "payment required",
        "credit",
        "no resource package",
        "quota",
        "rate limit",
        "rate_limited",
        "token_quota",
        "auth",
        "unauthorized",
        "forbidden",
        "not found for account",
        "model_not_found",
        "model is not supported",
        "model not supported",
        "not currently available",
    ]
    .iter()
    .any(|needle| last_error.contains(needle))
}

fn cooldown_bypass_max_seconds() -> u64 {
    std::env::var("ARDA_CHARON_COOLDOWN_BYPASS_MAX_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(120)
}

pub(super) fn provider_in_half_open(p: &ProviderState) -> bool {
    !p.in_cooldown && p.cooldown_until_utc.is_none() && p.consecutive_failures >= 3
}

pub(super) fn provider_half_open_probe_allowed(p: &ProviderState) -> bool {
    if !provider_in_half_open(p) {
        return true;
    }
    let stride = half_open_probe_stride();
    let roll = rand::random::<u32>() % stride;
    provider_half_open_probe_allowed_for_roll(p, roll)
}

pub(super) fn provider_half_open_probe_allowed_for_roll(p: &ProviderState, roll: u32) -> bool {
    !provider_in_half_open(p) || roll == 0
}

fn half_open_probe_stride() -> u32 {
    std::env::var("ARDA_CHARON_HALF_OPEN_PROBE_STRIDE")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(10)
        .min(1_000)
}

fn request_requires_agentic_tool_use(req: &CharonRequestEnvelope) -> bool {
    // Explicit flags in options
    if req
        .options
        .get("tool_use_required")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || req
            .options
            .get("tool_choice")
            .is_some_and(tool_choice_requires_tool_call)
    {
        return true;
    }
    // Tool-use continuation: the message history contains tool result messages
    // or an assistant message with pending tool_calls. This covers follow-up turns
    // where the client may not re-send the `tools` array.
    req.messages.iter().any(message_has_tool_history)
}

fn message_has_tool_history(message: &serde_json::Value) -> bool {
    (message.get("role").and_then(|r| r.as_str()) == Some("tool")
        && message
            .get("tool_call_id")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.trim().is_empty()))
        || message
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .is_some_and(|arr| !arr.is_empty())
        || message
            .get("function_call")
            .is_some_and(|value| !value.is_null())
}

fn tool_choice_requires_tool_call(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(required) => *required,
        serde_json::Value::String(raw) => {
            let trimmed = raw.trim().to_ascii_lowercase();
            !trimmed.is_empty() && !matches!(trimmed.as_str(), "auto" | "none" | "off")
        }
        serde_json::Value::Object(map) => !map.is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

pub(super) fn tool_execution_min_context_window() -> usize {
    std::env::var("ARDA_CHARON_TOOL_EXECUTION_MIN_CONTEXT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 16_000)
        .unwrap_or(64_000)
}

fn compression_min_context_window() -> usize {
    std::env::var("ARDA_CHARON_COMPRESSION_MIN_CONTEXT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 16_000)
        .unwrap_or(96_000)
}

fn local_tool_context_headroom_tokens() -> usize {
    std::env::var("ARDA_CHARON_LOCAL_TOOL_CONTEXT_HEADROOM")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 4_096)
        .unwrap_or(8_192)
}

fn request_allows_low_context_tool_fallback(req: &CharonRequestEnvelope) -> bool {
    req.options
        .get("allow_low_context_tool_fallback")
        .or_else(|| req.options.get("emergency_low_context_tool_fallback"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn request_needs_tool_capable_route(req: &CharonRequestEnvelope) -> bool {
    if request_requires_agentic_tool_use(req) {
        return true;
    }

    let workload_role = option_string(&req.options, "workload_role")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let execution_lane = option_string(&req.options, "execution_lane")
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(workload_role.as_str(), "execution" | "subagent")
        || execution_lane == "execution"
        || req.task_type == "code"
}

fn request_prefers_high_context_tool_use(req: &CharonRequestEnvelope) -> bool {
    if !request_requires_agentic_tool_use(req) {
        return false;
    }

    if req
        .options
        .get("context_window_target")
        .and_then(|value| value.as_u64())
        .is_some_and(|value| value >= 128_000)
    {
        return true;
    }

    if req
        .options
        .get("context_priority")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("high"))
    {
        return true;
    }

    req.options
        .get("workload_role")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("orchestrator"))
}

fn request_prefers_audit_stability(req: &CharonRequestEnvelope) -> bool {
    if !request_prefers_high_context_tool_use(req) {
        return false;
    }

    if req
        .options
        .get("audit_stability")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        return true;
    }

    let haystack = format!(
        "{} {} {}",
        req.task_type,
        serde_json::to_string(&req.messages).unwrap_or_default(),
        serde_json::to_string(&req.options).unwrap_or_default()
    )
    .to_ascii_lowercase();

    haystack.contains("deep audit")
        || haystack.contains("audit")
        || haystack.contains("action items")
        || haystack.contains("crate by crate")
        || haystack.contains("crate-by-crate")
        || haystack.contains("service.rs")
}

/// Filter providers based on declared capability flags vs request payload.
/// If the request asks for `tools`/`tool_choice`/structured output and the
/// provider config marks itself as not supporting them, the provider is
/// removed from candidates *before* a 400 ever happens.
pub(super) fn provider_supports_request_capabilities(
    p: &ProviderState,
    req: &CharonRequestEnvelope,
) -> bool {
    if request_needs_tool_capable_route(req) && !provider_or_model_supports_tools(p, req) {
        return false;
    }
    let request_uses_structured_output = req.options.get("response_format").is_some();
    if request_uses_structured_output && !provider_or_model_supports_structured_output(p, req) {
        return false;
    }
    true
}

fn provider_or_model_supports_tools(p: &ProviderState, req: &CharonRequestEnvelope) -> bool {
    p.supports_tools
        || select_model_for_provider_request(p, &req.task_type, None, Some(req))
            .is_some_and(|model| model.capabilities.tools == Some(true))
}

fn provider_or_model_supports_structured_output(
    p: &ProviderState,
    req: &CharonRequestEnvelope,
) -> bool {
    p.supports_structured_output
        || select_model_for_provider_request(p, &req.task_type, None, Some(req))
            .is_some_and(|model| model.capabilities.structured_output == Some(true))
}

pub(super) fn provider_supports_request(p: &ProviderState, req: &CharonRequestEnvelope) -> bool {
    if request_requires_streaming(req)
        && matches!(p.driver.as_str(), "hermes_agent_cli" | "codex_responses")
    {
        return false;
    }
    if p.base_url
        .as_deref()
        .is_some_and(|base_url| base_url.contains("${"))
    {
        return false;
    }

    if !request_requires_agentic_tool_use(req) {
        return true;
    }

    // No vendor hard-coding. Let the general eligibility + scoring decide.
    // We only gate truly incompatible surfaces here.
    if request_prefers_high_context_tool_use(req) && request_prefers_audit_stability(req) {
        // For deep audit + heavy tool use, we can be stricter if you want,
        // but no forced "anthropic" anymore.
        return !is_local_fallback(&p.id); // optional: block tiny local fallback for audit
    }

    true
}

pub(super) fn model_supports_request(
    provider_id: &str,
    model: &ModelState,
    req: Option<&CharonRequestEnvelope>,
) -> bool {
    let Some(req) = req else {
        return true;
    };
    let priority = req.priority.to_ascii_lowercase();
    let route_profile = derive_route_execution_profile(req, &priority);
    let mut context_window_target =
        if is_local_provider(provider_id) && request_requires_agentic_tool_use(req) {
            route_profile
                .context_window_target
                .min(local_slimmed_tool_context_target(req))
        } else {
            route_profile.context_window_target
        };
    if request_needs_tool_capable_route(req) && !request_allows_low_context_tool_fallback(req) {
        context_window_target = context_window_target.max(tool_execution_min_context_window());
    }
    if route_profile.execution_lane == "execution" && !request_allows_low_context_tool_fallback(req)
    {
        context_window_target = context_window_target.max(tool_execution_min_context_window());
    }
    if route_profile.execution_lane == "compression" {
        context_window_target = context_window_target.max(compression_min_context_window());
    }
    if is_local_provider(provider_id)
        && request_requires_agentic_tool_use(req)
        && !request_allows_low_context_tool_fallback(req)
    {
        context_window_target =
            context_window_target.saturating_add(local_tool_context_headroom_tokens());
    }
    if model.context_window < context_window_target {
        return false;
    }
    if known_generation_incompatible_model(&model.id) {
        return false;
    }
    if request_requires_streaming(req)
        && (model.capabilities.streaming == Some(false)
            || (model.capabilities.streaming != Some(true)
                && model.streaming_validated == Some(false)))
    {
        return false;
    }
    if request_uses_structured_output(req) && model.capabilities.structured_output == Some(false) {
        return false;
    }
    let needs_tool_capable_model = request_requires_agentic_tool_use(req)
        || route_profile.route_class == "tool_oriented"
        || route_profile.execution_lane == "execution";
    if needs_tool_capable_model
        && provider_id == "openrouter"
        && openrouter_free_model_id(&model.id)
        && !request_allows_free_tool_pool(req)
    {
        return false;
    }
    if needs_tool_capable_model && model.capabilities.tools == Some(false) {
        return false;
    }
    if needs_tool_capable_model
        && model.capabilities.tools != Some(true)
        && known_tool_incompatible_model(provider_id, &model.id)
    {
        return false;
    }
    if model_has_visible_reasoning_surface(model) && !request_allows_visible_reasoning(req) {
        return false;
    }

    true
}

fn known_generation_incompatible_model(model_id: &str) -> bool {
    let model_id = model_id.to_ascii_lowercase();
    contains_any(
        &model_id,
        &[
            "prompt-guard",
            "safeguard",
            "content-safety",
            "whisper",
            "orpheus",
            "bge-",
            "embedding",
            "rerank",
        ],
    )
}

fn known_tool_incompatible_model(provider_id: &str, model_id: &str) -> bool {
    let provider_id = provider_id.to_ascii_lowercase();
    let model_id = model_id.to_ascii_lowercase();
    if configured_tool_incompatible_model(&provider_id, &model_id) {
        return true;
    }
    if provider_id == "openrouter" && model_id.ends_with(":free") {
        return true;
    }
    (provider_id == "groq" && model_id.contains("compound"))
        || (provider_id == "nvidia" && model_id.contains("starcoder"))
        || (provider_id == "cerebras" && model_id.contains("gpt-oss"))
}

fn openrouter_free_model_id(model_id: &str) -> bool {
    let model_id = model_id.to_ascii_lowercase();
    model_id.ends_with(":free") || model_id.ends_with("/free") || model_id.contains("/free/")
}

fn request_allows_free_tool_pool(req: &CharonRequestEnvelope) -> bool {
    req.options
        .get("allow_free_tool_pool")
        .or_else(|| req.options.get("free_tool_pool"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || req
            .options
            .get("tool_pool_strategy")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.eq_ignore_ascii_case("free_first"))
}

fn configured_tool_incompatible_model(provider_id: &str, model_id: &str) -> bool {
    std::env::var("ARDA_CHARON_TOOL_INCOMPATIBLE_MODELS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .any(|entry| {
            let entry = entry.to_ascii_lowercase();
            if let Some((provider_pat, model_pat)) = entry.split_once('/') {
                provider_id.contains(provider_pat) && model_id.contains(model_pat)
            } else {
                model_id.contains(&entry)
            }
        })
}

fn local_slimmed_tool_context_target(req: &CharonRequestEnvelope) -> usize {
    let message_tokens = req
        .messages
        .iter()
        .map(|message| {
            let role = message
                .get("role")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let serialized_tokens = estimate_json_tokens(message);
            if matches!(role, "system" | "developer") {
                serialized_tokens.min(2_500)
            } else {
                serialized_tokens
            }
        })
        .sum::<usize>();
    let tool_tokens = req
        .options
        .get("tools")
        .map(|tools| estimate_json_tokens(tools).min(4_000))
        .unwrap_or(0);
    let output_tokens = req
        .options
        .get("max_tokens")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(2_048)
        .min(4_096);
    round_up_context_tier(message_tokens + tool_tokens + output_tokens).min(16_000)
}

fn request_requires_streaming(req: &CharonRequestEnvelope) -> bool {
    req.options
        .get("stream")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn request_uses_structured_output(req: &CharonRequestEnvelope) -> bool {
    req.options.get("response_format").is_some()
}

pub(super) fn resolve_hybrid_route_policy(
    task_type: &str,
    options: &serde_json::Value,
) -> HybridRoutePolicy {
    let env_privacy =
        std::env::var("ARDA_ROUTE_PRIVACY_DEFAULT").unwrap_or_else(|_| "public".to_string());
    let env_cost =
        std::env::var("ARDA_ROUTE_COST_DEFAULT").unwrap_or_else(|_| "balanced".to_string());
    let env_quality =
        std::env::var("ARDA_ROUTE_QUALITY_DEFAULT").unwrap_or_else(|_| "balanced".to_string());
    let env_origin =
        std::env::var("ARDA_ROUTE_ORIGIN_DEFAULT").unwrap_or_else(|_| "auto".to_string());
    let env_latency = std::env::var("ARDA_ROUTE_LATENCY_SLA_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

    let privacy_tier = option_string(options, "privacy_requirement")
        .or_else(|| option_string(options, "privacy_tier"))
        .unwrap_or(&env_privacy)
        .to_ascii_lowercase();
    let workload_role = option_string(options, "workload_role")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let cost_policy = option_string(options, "cost_policy")
        .or_else(|| option_string(options, "cost_target"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let quality_priority = option_string(options, "quality_priority")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let cost_tier = option_string(options, "cost_tier")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| match cost_policy.as_str() {
            "free_first" | "cheap" | "low" => "low".to_string(),
            "paid_only" | "premium" | "high" => "high".to_string(),
            "paid_allowed" | "balanced" => "balanced".to_string(),
            _ => env_cost.clone(),
        });
    let quality_tier = option_string(options, "quality_tier")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| match quality_priority.as_str() {
            "high" => "high".to_string(),
            "low" => "low".to_string(),
            _ => env_quality.clone(),
        });
    let mut origin_preference = options
        .get("inference_origin")
        .or_else(|| options.get("origin_preference"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if origin_preference.is_empty() {
        origin_preference = match workload_role.as_str() {
            "orchestrator" => "auto".to_string(),
            "execution" | "subagent" => "auto".to_string(),
            _ => env_origin.clone(),
        };
    }
    if !matches!(origin_preference.as_str(), "local" | "cloud" | "auto") {
        origin_preference = "auto".to_string();
    }
    let latency_sla_ms = options
        .get("latency_sla_ms")
        .and_then(|v| v.as_u64())
        .or(env_latency);
    let explicit_local_only = options
        .get("local_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let require_local = explicit_local_only
        || matches!(
            privacy_tier.as_str(),
            "restricted" | "confidential" | "local_only"
        );
    let (default_spread_score_band, default_spread_top_cap) =
        route_spread_defaults_for_task(task_type);
    let spread_score_band = options
        .get("route_spread_score_band")
        .and_then(|v| v.as_f64())
        .filter(|v| (0.0..=1.0).contains(v))
        .unwrap_or(default_spread_score_band);
    let spread_top_cap = options
        .get("route_spread_top_cap")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .filter(|v| *v > 0)
        .unwrap_or(default_spread_top_cap);

    HybridRoutePolicy {
        privacy_tier,
        cost_tier,
        quality_tier,
        origin_preference,
        latency_sla_ms,
        require_local,
        spread_score_band,
        spread_top_cap,
    }
}

fn route_spread_defaults_for_task(task_type: &str) -> (f64, usize) {
    match task_type.to_ascii_lowercase().as_str() {
        "chat" | "summary" => (0.10, 5),
        "code" | "reasoning" | "research" => (0.03, 3),
        "monitoring" | "background" => (0.02, 2),
        _ => (0.05, 4),
    }
}

/// Rough token estimate from serialized request fragments (4 chars ≈ 1 token).
fn estimate_json_tokens(value: &serde_json::Value) -> usize {
    serde_json::to_string(value).map(|s| s.len()).unwrap_or(0) / 4
}

fn estimate_message_tokens(messages: &[serde_json::Value]) -> usize {
    estimate_json_tokens(&serde_json::Value::Array(messages.to_vec()))
}

fn estimate_request_tokens(req: &CharonRequestEnvelope) -> usize {
    let mut total = estimate_message_tokens(&req.messages);
    for key in ["tools", "tool_choice", "response_format", "stop"] {
        if let Some(value) = req.options.get(key) {
            total = total.saturating_add(estimate_json_tokens(value));
        }
    }
    if let Some(max_tokens) = req
        .options
        .get("max_tokens")
        .and_then(|value| value.as_u64())
    {
        total = total.saturating_add(max_tokens as usize);
    }
    total
}

/// Round an estimated token count up to the nearest standard context-window tier.
fn round_up_context_tier(tokens: usize) -> usize {
    match tokens {
        0..=16_000 => 16_000,
        16_001..=32_000 => 32_000,
        32_001..=64_000 => 64_000,
        64_001..=128_000 => 128_000,
        128_001..=200_000 => 200_000,
        _ => 256_000,
    }
}

pub(super) fn derive_route_execution_profile(
    req: &CharonRequestEnvelope,
    priority: &str,
) -> RouteExecutionProfile {
    let options = &req.options;
    let workload_role = option_string(options, "workload_role")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let context_priority = option_string(options, "context_priority");
    let explicit_context_target = options
        .get("context_window_target")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    if options
        .get("prefer_probe_model")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        let lane = option_string(options, "execution_lane")
            .or_else(|| option_string(options, "workload_role"))
            .unwrap_or("interactive")
            .to_ascii_lowercase();
        return RouteExecutionProfile {
            route_class: "health_probe".to_string(),
            execution_lane: lane,
            context_window_target: explicit_context_target.unwrap_or(1024),
        };
    }

    match workload_role.as_str() {
        "orchestrator" => {
            return RouteExecutionProfile {
                route_class: if request_prefers_audit_stability(req) {
                    "audit_stability".to_string()
                } else {
                    "context_heavy".to_string()
                },
                execution_lane: "orchestrator".to_string(),
                context_window_target: explicit_context_target
                    .unwrap_or_else(|| context_target_for_priority(context_priority, 128_000)),
            };
        }
        "execution" | "subagent" => {
            let target = explicit_context_target.unwrap_or_else(|| {
                let priority_base = context_target_for_priority(context_priority, 32_000);
                let estimated = round_up_context_tier(estimate_request_tokens(req));
                priority_base.max(estimated)
            });
            return RouteExecutionProfile {
                route_class: "tool_oriented".to_string(),
                execution_lane: "execution".to_string(),
                context_window_target: target,
            };
        }
        "validator" => {
            return RouteExecutionProfile {
                route_class: "governance_strict".to_string(),
                execution_lane: "validator".to_string(),
                context_window_target: explicit_context_target
                    .unwrap_or_else(|| context_target_for_priority(context_priority, 128_000)),
            };
        }
        "background" => {
            return RouteExecutionProfile {
                route_class: "background_maintenance".to_string(),
                execution_lane: "background".to_string(),
                context_window_target: explicit_context_target.unwrap_or_else(|| {
                    let priority_base = context_target_for_priority(context_priority, 16_000);
                    let estimated = round_up_context_tier(estimate_request_tokens(req));
                    priority_base.max(estimated)
                }),
            };
        }
        "compression" | "compaction" | "context_compaction" | "summarization_compression" => {
            return RouteExecutionProfile {
                route_class: "compression".to_string(),
                execution_lane: "compression".to_string(),
                context_window_target: explicit_context_target.unwrap_or_else(|| {
                    let priority_base = context_target_for_priority(context_priority, 64_000);
                    let estimated = round_up_context_tier(estimate_request_tokens(req));
                    priority_base.max(estimated)
                }),
            };
        }
        _ => {}
    }

    if let Some(lane) = options.get("execution_lane").and_then(|v| v.as_str()) {
        let lane = lane.to_ascii_lowercase();
        return RouteExecutionProfile {
            route_class: if lane == "execution" {
                "tool_oriented".to_string()
            } else {
                "operator_override".to_string()
            },
            context_window_target: explicit_context_target.unwrap_or_else(|| {
                let base = if lane == "planning" { 128_000 } else { 32_000 };
                let estimated = round_up_context_tier(estimate_request_tokens(req));
                base.max(estimated)
            }),
            execution_lane: lane,
        };
    }

    let messages_len = req.messages.len();
    let payload_text = serde_json::to_string(&req.options)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let optional_tool_schema_only = req
        .options
        .get("tools_available")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && !request_requires_agentic_tool_use(req);
    let option_text_implies_execution = !optional_tool_schema_only
        && (payload_text.contains("tool")
            || payload_text.contains("command")
            || payload_text.contains("shell"));
    if matches!(priority, "background" | "low" | "deferred") {
        return RouteExecutionProfile {
            route_class: "background_maintenance".to_string(),
            execution_lane: "background".to_string(),
            context_window_target: 16_000,
        };
    }
    if req.task_type == "code"
        || request_requires_agentic_tool_use(req)
        || option_text_implies_execution
    {
        let estimated = round_up_context_tier(estimate_request_tokens(req));
        return RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: explicit_context_target.unwrap_or_else(|| estimated.max(32_000)),
        };
    }
    if matches!(req.task_type.as_str(), "research" | "reasoning" | "summary") || messages_len >= 6 {
        let estimated = round_up_context_tier(estimate_request_tokens(req));
        let default_target =
            if context_priority.is_some_and(|value| value.eq_ignore_ascii_case("high")) {
                128_000
            } else {
                64_000
            };
        return RouteExecutionProfile {
            route_class: "context_heavy".to_string(),
            execution_lane: "planning".to_string(),
            context_window_target: explicit_context_target.unwrap_or_else(|| {
                context_target_for_priority(context_priority, default_target).max(estimated)
            }),
        };
    }
    RouteExecutionProfile {
        route_class: "interactive".to_string(),
        execution_lane: "interactive".to_string(),
        context_window_target: explicit_context_target
            .unwrap_or_else(|| round_up_context_tier(estimate_request_tokens(req))),
    }
}

pub(super) fn merge_latency(current: Option<u64>, observed: Option<u64>) -> Option<u64> {
    match (current, observed) {
        (Some(a), Some(b)) => Some(((a as f64 * 0.8) + (b as f64 * 0.2)).round() as u64),
        (None, Some(v)) => Some(v),
        (Some(v), None) => Some(v),
        (None, None) => None,
    }
}

#[cfg(test)]
#[path = "route_policy_tests.rs"]
mod tests;