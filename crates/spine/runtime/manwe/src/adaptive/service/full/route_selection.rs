use super::adaptive_routing::{
    pacing_state_for_provider, provider_available_after_pacing, RejectedRouteCandidate,
};
use super::echo_gate::{evaluate_pre_route_governance_with_options, GateAction};
use super::route_policy::{
    access_tier_class, apply_soft_lane_caps, is_background_priority, is_high_priority,
    is_local_provider, is_primary_local_surface_provider, parse_model_params_billions,
    model_request_incompatibilities, provider_eligible, provider_eligible_ignoring_cooldown,
    provider_score, provider_supports_request, provider_supports_request_capabilities,
    request_allows_hermes_cli_fast_lane, select_model_for_provider_request, HybridRoutePolicy,
    RouteExecutionProfile, RouteSelectionCandidate,
};
use super::{CharonService, JsonValue, PackageRuntimeSignals};
use crate::adaptive::types::{ManweRequestEnvelope, ProviderState};
use arda_core::error::{ArdaError, Result};

impl CharonService {
    fn retain_preferred_local_surface_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        route_profile: &RouteExecutionProfile,
    ) {
        match route_profile.execution_lane.as_str() {
            "execution" | "background" => {}
            _ => return,
        }

        let preferred_surface = std::env::var("ARDA_LOCAL_INFERENCE_SURFACE")
            .unwrap_or_else(|_| "hybrid".to_string())
            .trim()
            .to_ascii_lowercase();

        match preferred_surface.as_str() {
            "mesh" => {
                if candidates.iter().any(|candidate| {
                    is_primary_local_surface_provider(&providers[candidate.provider_index].id)
                }) {
                    candidates.retain(|candidate| {
                        is_primary_local_surface_provider(&providers[candidate.provider_index].id)
                    });
                }
            }
            "llamacpp"
                if candidates.iter().any(|candidate| {
                    let id = &providers[candidate.provider_index].id;
                    is_local_provider(id) && !is_primary_local_surface_provider(id)
                }) =>
            {
                candidates.retain(|candidate| {
                    let id = &providers[candidate.provider_index].id;
                    is_local_provider(id) && !is_primary_local_surface_provider(id)
                });
            }
            _ => {}
        }
    }

    pub(super) fn retain_orchestrator_context_fit_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        route_profile: &RouteExecutionProfile,
    ) {
        if route_profile.execution_lane != "orchestrator" {
            return;
        }
        candidates.retain(|candidate| {
            candidate.model.context_window >= route_profile.context_window_target
        });
    }

    fn retain_fast_lane_non_hermes_cli_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        req: &ManweRequestEnvelope,
        route_profile: &RouteExecutionProfile,
        forced_provider_id: Option<&str>,
    ) {
        if forced_provider_id.is_some() || request_allows_hermes_cli_fast_lane(req) {
            return;
        }
        if !matches!(
            route_profile.execution_lane.as_str(),
            "interactive" | "execution" | "planning"
        ) {
            return;
        }
        if candidates.iter().any(|candidate| {
            providers[candidate.provider_index].driver.as_str() != "hermes_agent_cli"
        }) {
            candidates.retain(|candidate| {
                providers[candidate.provider_index].driver.as_str() != "hermes_agent_cli"
            });
        }
    }

    fn apply_explicit_fallback_tier_order(
        candidates: &mut [RouteSelectionCandidate],
        providers: &[ProviderState],
        route_profile: &RouteExecutionProfile,
        forced_provider_id: Option<&str>,
    ) {
        if forced_provider_id.is_some()
            || !matches!(
                route_profile.execution_lane.as_str(),
                "execution" | "planning"
            )
        {
            return;
        }

        candidates.sort_by(|a, b| {
            let tier_a = fallback_tier_rank(&providers[a.provider_index]);
            let tier_b = fallback_tier_rank(&providers[b.provider_index]);
            tier_a.cmp(&tier_b).then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
    }

    fn retain_highest_explicit_fallback_tier(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        route_profile: &RouteExecutionProfile,
        forced_provider_id: Option<&str>,
    ) {
        if !explicit_fallback_tier_order_enabled(route_profile, forced_provider_id)
            || candidates.is_empty()
        {
            return;
        }
        let best_rank = candidates
            .iter()
            .map(|candidate| fallback_tier_rank(&providers[candidate.provider_index]))
            .min()
            .unwrap_or(usize::MAX);
        candidates.retain(|candidate| {
            fallback_tier_rank(&providers[candidate.provider_index]) == best_rank
        });
    }

    fn retain_free_external_tool_route_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        req: &ManweRequestEnvelope,
        policy: &HybridRoutePolicy,
        route_profile: &RouteExecutionProfile,
        forced_provider_id: Option<&str>,
    ) {
        if forced_provider_id.is_some()
            || policy.require_local
            || policy.origin_preference == "local"
            || !request_allows_free_tool_pool(req)
            || !matches!(
                route_profile.execution_lane.as_str(),
                "execution" | "planning"
            )
            || route_profile.route_class != "tool_oriented"
        {
            return;
        }

        if candidates
            .iter()
            .any(|candidate| !is_local_provider(&providers[candidate.provider_index].id))
        {
            let free_candidate_count = candidates
                .iter()
                .filter(|candidate| free_pool_candidate(candidate, providers))
                .count();
            if free_candidate_count >= free_external_tool_pool_min_candidates() {
                candidates.retain(|candidate| free_pool_candidate(candidate, providers));
            }
        }
    }

    fn retain_orchestration_grade_execution_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        policy: &HybridRoutePolicy,
        route_profile: &RouteExecutionProfile,
        forced_provider_id: Option<&str>,
    ) {
        if forced_provider_id.is_some()
            || policy.require_local
            || route_profile.execution_lane != "execution"
            || route_profile.route_class != "tool_oriented"
        {
            return;
        }

        if candidates
            .iter()
            .any(|candidate| orchestration_grade_candidate(candidate, providers))
        {
            candidates.retain(|candidate| orchestration_grade_candidate(candidate, providers));
        }
    }

    fn retain_large_context_external_tool_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        policy: &HybridRoutePolicy,
        route_profile: &RouteExecutionProfile,
        forced_provider_id: Option<&str>,
    ) {
        if forced_provider_id.is_some()
            || policy.require_local
            || policy.origin_preference == "local"
            || route_profile.execution_lane != "execution"
            || route_profile.route_class != "tool_oriented"
            || route_profile.context_window_target < large_context_tool_execution_threshold()
        {
            return;
        }

        if candidates
            .iter()
            .any(|candidate| external_tool_candidate(candidate, providers, route_profile))
        {
            candidates
                .retain(|candidate| external_tool_candidate(candidate, providers, route_profile));
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn select_route_candidate(
        &self,
        providers: &[ProviderState],
        req: &ManweRequestEnvelope,
        priority: &str,
        strict: bool,
        forced_provider_id: Option<&str>,
        forced_model_id: Option<&str>,
        excluded_provider_ids: &[String],
        policy: &HybridRoutePolicy,
        route_profile: &RouteExecutionProfile,
        package_runtime: &PackageRuntimeSignals,
    ) -> Result<RouteSelectionCandidate> {
        let mut candidates = self
            .cached_route_candidates(
                providers,
                req,
                priority,
                strict,
                forced_provider_id,
                forced_model_id,
                excluded_provider_ids,
                route_profile,
            )
            .unwrap_or_else(|| {
                let candidates = self.build_scored_route_candidates(
                    providers,
                    req,
                    priority,
                    strict,
                    forced_provider_id,
                    forced_model_id,
                    excluded_provider_ids,
                    policy,
                    route_profile,
                    package_runtime,
                );
                self.cache_route_candidates(
                    providers,
                    req,
                    priority,
                    strict,
                    forced_provider_id,
                    forced_model_id,
                    route_profile,
                    &candidates,
                );
                candidates
            });

        let input_text = req
            .messages
            .iter()
            .filter(|message| message.get("role").and_then(|value| value.as_str()) == Some("user"))
            .filter_map(|m| m.get("content").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ");

        let governance = evaluate_pre_route_governance_with_options(&input_text, &req.options);

        tracing::info!(
            rho = governance.rho,
            gamma = governance.gamma,
            delta = governance.delta,
            action = ?governance.action,
            trigger_reason = %governance.trigger_reason,
            agent_id = %req.agent_id,
            task_type = %req.task_type,
            "MANWE Echo Gate evaluated request"
        );
        self.append_state_event(
            "echo_gate",
            serde_json::json!({
                "rho": governance.rho,
                "gamma": governance.gamma,
                "delta": governance.delta,
                "governance_method": governance.governance_method,
                "philosopher_lens": governance.philosopher_lens,
                "chain_id": governance.chain_id,
                "bacon_evidence_score": governance.bacon_evidence_score,
                "soterion_protocol_markers": governance.soterion_protocol_markers,
                "action": governance.action,
                "trigger_reason": governance.trigger_reason,
                "agent_id": req.agent_id,
                "task_type": req.task_type,
                "priority": priority,
            }),
        )?;
        self.append_governance_event(
            "echo_gate",
            serde_json::json!({
                "agent_id": req.agent_id,
                "task_type": req.task_type,
                "priority": priority,
                "verdict": "evaluated",
                "rho": governance.rho,
                "gamma": governance.gamma,
                "delta": governance.delta,
                "governance_method": governance.governance_method,
                "philosopher_lens": governance.philosopher_lens,
                "chain_id": governance.chain_id,
                "bacon_evidence_score": governance.bacon_evidence_score,
                "soterion_protocol_markers": governance.soterion_protocol_markers,
                "action": governance.action,
                "trigger_reason": governance.trigger_reason,
            }),
        )?;

        match governance.action {
            GateAction::Abort => {
                self.append_governance_event(
                    "echo_gate_abort",
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "verdict": "blocked",
                        "failure_class": "echo_gate_abort",
                        "rho": governance.rho,
                        "gamma": governance.gamma,
                        "delta": governance.delta,
                        "governance_method": governance.governance_method,
                        "philosopher_lens": governance.philosopher_lens,
                        "chain_id": governance.chain_id,
                        "bacon_evidence_score": governance.bacon_evidence_score,
                        "soterion_protocol_markers": governance.soterion_protocol_markers,
                        "action": governance.action,
                        "trigger_reason": governance.trigger_reason,
                    }),
                )?;
                return Err(ArdaError::Agent {
                    agent: "manwe".to_string(),
                    message: format!(
                        "Echo Gate ABORT [{}] (rho={:.2}, gamma={:.2}, delta={:.2})",
                        governance.trigger_reason,
                        governance.rho,
                        governance.gamma,
                        governance.delta
                    ),
                });
            }
            GateAction::Pause => {}
            GateAction::Proceed => {}
        }

        if governance.delta > 0.0 {
            for candidate in candidates.iter_mut() {
                if !is_local_provider(&providers[candidate.provider_index].id) {
                    candidate.score = (candidate.score - governance.delta * 20.0).max(0.0);
                } else if matches!(governance.action, GateAction::Pause) {
                    candidate.score += 5.0;
                }
            }
        }

        candidates.retain(|candidate| {
            provider_available_after_pacing(
                &providers[candidate.provider_index],
                priority,
                route_profile,
            )
        });

        if policy.require_local {
            if candidates
                .iter()
                .any(|candidate| is_local_provider(&providers[candidate.provider_index].id))
            {
                candidates
                    .retain(|candidate| is_local_provider(&providers[candidate.provider_index].id));
            } else {
                self.append_state_event(
                    "route_failed_policy",
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "policy": policy,
                        "reason": "privacy_requires_local_but_no_local_provider_available"
                    }),
                )?;
                self.append_governance_event(
                    "route_failed_policy",
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "policy": policy,
                        "verdict": "blocked",
                        "failure_class": "policy_requires_local_no_local_available",
                        "reason": "privacy_requires_local_but_no_local_provider_available"
                    }),
                )?;
                return Err(ArdaError::Agent {
                    agent: "manwe".to_string(),
                    message:
                        "route blocked by policy: privacy tier requires local provider but none available"
                            .to_string(),
                });
            }
        }

        if !policy.require_local {
            if policy.origin_preference == "local"
                && candidates
                    .iter()
                    .any(|candidate| is_local_provider(&providers[candidate.provider_index].id))
            {
                candidates
                    .retain(|candidate| is_local_provider(&providers[candidate.provider_index].id));
            } else if policy.origin_preference == "cloud"
                && candidates
                    .iter()
                    .any(|candidate| !is_local_provider(&providers[candidate.provider_index].id))
            {
                candidates.retain(|candidate| {
                    !is_local_provider(&providers[candidate.provider_index].id)
                });
            }
        }

        Self::retain_low_cost_candidates(&mut candidates, providers, policy, forced_provider_id);
        Self::retain_orchestrator_context_fit_candidates(&mut candidates, route_profile);
        Self::retain_cost_tier_orchestrator_candidates(
            &mut candidates,
            providers,
            policy,
            route_profile,
        );
        Self::retain_free_external_tool_route_candidates(
            &mut candidates,
            providers,
            req,
            policy,
            route_profile,
            forced_provider_id,
        );
        Self::retain_orchestration_grade_execution_candidates(
            &mut candidates,
            providers,
            policy,
            route_profile,
            forced_provider_id,
        );
        Self::retain_large_context_external_tool_candidates(
            &mut candidates,
            providers,
            policy,
            route_profile,
            forced_provider_id,
        );
        Self::retain_preferred_local_surface_candidates(&mut candidates, providers, route_profile);
        Self::retain_fast_lane_non_hermes_cli_candidates(
            &mut candidates,
            providers,
            req,
            route_profile,
            forced_provider_id,
        );
        Self::retain_highest_explicit_fallback_tier(
            &mut candidates,
            providers,
            route_profile,
            forced_provider_id,
        );

        if is_high_priority(priority)
            && candidates
                .iter()
                .any(|candidate| !is_local_provider(&providers[candidate.provider_index].id))
        {
            candidates
                .retain(|candidate| !is_local_provider(&providers[candidate.provider_index].id));
        }

        if is_background_priority(priority)
            && candidates
                .iter()
                .any(|candidate| is_local_provider(&providers[candidate.provider_index].id))
        {
            candidates
                .retain(|candidate| is_local_provider(&providers[candidate.provider_index].id));
        }

        apply_soft_lane_caps(&mut candidates, providers, route_profile);
        Self::apply_explicit_fallback_tier_order(
            &mut candidates,
            providers,
            route_profile,
            forced_provider_id,
        );
        candidates.sort_by(|a, b| {
            if route_profile.execution_lane == "execution" {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        fallback_tier_rank(&providers[a.provider_index])
                            .cmp(&fallback_tier_rank(&providers[b.provider_index]))
                    })
            } else {
                fallback_tier_rank(&providers[a.provider_index])
                    .cmp(&fallback_tier_rank(&providers[b.provider_index]))
                    .then_with(|| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            }
        });

        if !candidates.is_empty() {
            let top_score = candidates[0].score;
            let pool_len = if top_score > 0.0 {
                let cutoff = top_score * (1.0 - policy.spread_score_band);
                candidates
                    .iter()
                    .take_while(|c| c.score >= cutoff)
                    .count()
                    .min(policy.spread_top_cap.max(1))
            } else {
                1
            };
            if pool_len <= 1 {
                return candidates
                    .into_iter()
                    .next()
                    .ok_or_else(|| ArdaError::Agent {
                        agent: "manwe".to_string(),
                        message: "route selection produced an empty candidate pool".to_string(),
                    });
            }
            let mut pool: Vec<RouteSelectionCandidate> =
                candidates.into_iter().take(pool_len).collect();
            let total: f64 = pool.iter().map(|c| c.score.max(0.0)).sum();
            if total <= 0.0 {
                return Ok(pool.swap_remove(0));
            }
            use rand::Rng;
            let mut roll: f64 = rand::thread_rng().gen_range(0.0..total);
            for idx in 0..pool.len() {
                let w = pool[idx].score.max(0.0);
                if roll < w {
                    return Ok(pool.swap_remove(idx));
                }
                roll -= w;
            }
            return Ok(pool.swap_remove(0));
        }

        if !policy.require_local && forced_provider_id.is_none() {
            let lane_fitness = self.read_lane_fitness_snapshot();
            let mut fallback: Vec<RouteSelectionCandidate> = providers
                .iter()
                .enumerate()
                .filter(|(_, provider)| {
                    provider_eligible_ignoring_cooldown(provider, priority)
                })
                .filter(|(_, provider)| self.provider_agent_quota_available(provider, req))
                .filter(|(_, provider)| provider_supports_request(provider, req))
                .filter(|(_, provider)| {
                    request_allows_hermes_cli_fast_lane(req)
                        || !matches!(
                            route_profile.execution_lane.as_str(),
                            "interactive" | "execution" | "planning"
                        )
                        || provider.driver != "hermes_agent_cli"
                })
                .filter(|(_, provider)| {
                    !excluded_provider_ids
                        .iter()
                        .any(|excluded| excluded == &provider.id)
                })
                .filter_map(|(provider_index, provider)| {
                    let model = select_model_for_provider_request(
                        provider,
                        &req.task_type,
                        forced_model_id,
                        Some(req),
                    )?;
                    let score = provider_score(
                        provider,
                        &model,
                        priority,
                        policy,
                        route_profile,
                        package_runtime,
                        &lane_fitness,
                    ) + self.bandit_score_bonus(req, &provider.id, &model.id);
                    Some(RouteSelectionCandidate {
                        provider_index,
                        model,
                        score,
                    })
                })
                .collect();

            fallback.sort_by(|left, right| {
                let left_failures = providers[left.provider_index].consecutive_failures;
                let right_failures = providers[right.provider_index].consecutive_failures;
                left_failures.cmp(&right_failures).then_with(|| {
                    right
                        .score
                        .partial_cmp(&left.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });

            if let Some(candidate) = fallback.into_iter().next() {
                self.append_state_event(
                    "route_cooldown_bypass",
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "provider_id": providers[candidate.provider_index].id,
                        "model_id": candidate.model.id,
                        "consecutive_failures": providers[candidate.provider_index].consecutive_failures,
                        "reason": "all_eligible_providers_in_cooldown",
                    }),
                )?;
                self.append_governance_event(
                    "route_cooldown_bypass",
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "provider_id": providers[candidate.provider_index].id,
                        "model_id": candidate.model.id,
                        "verdict": "cooldown_bypass",
                        "failure_class": null,
                        "reason": "all_eligible_providers_in_cooldown",
                    }),
                )?;
                return Ok(candidate);
            }
        }

        Err(ArdaError::Agent {
            agent: "manwe".to_string(),
            message: route_rejection_error_message(
                providers,
                req,
                priority,
                strict,
                forced_model_id,
                excluded_provider_ids,
                self,
            ),
        })
    }

    pub(super) fn retain_low_cost_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        policy: &HybridRoutePolicy,
        forced_provider_id: Option<&str>,
    ) {
        if forced_provider_id.is_some() || policy.cost_tier != "low" {
            return;
        }

        let low_cost_candidate_count = candidates
            .iter()
            .filter(|candidate| {
                is_local_provider(&providers[candidate.provider_index].id)
                    || free_pool_candidate(candidate, providers)
            })
            .count();
        let has_local_candidate = candidates
            .iter()
            .any(|candidate| is_local_provider(&providers[candidate.provider_index].id));
        if has_local_candidate
            || low_cost_candidate_count >= free_external_tool_pool_min_candidates()
        {
            candidates.retain(|candidate| {
                is_local_provider(&providers[candidate.provider_index].id)
                    || free_pool_candidate(candidate, providers)
            });
        }
    }

    pub(super) fn retain_cost_tier_orchestrator_candidates(
        candidates: &mut Vec<RouteSelectionCandidate>,
        providers: &[ProviderState],
        policy: &HybridRoutePolicy,
        route_profile: &RouteExecutionProfile,
    ) {
        if route_profile.execution_lane != "orchestrator" || policy.cost_tier != "low" {
            return;
        }

        let fits_context = |candidate: &RouteSelectionCandidate| {
            candidate.model.context_window >= route_profile.context_window_target
        };
        let tier_of = |candidate: &RouteSelectionCandidate| {
            access_tier_class(&providers[candidate.provider_index])
        };

        if candidates
            .iter()
            .any(|candidate| free_pool_candidate(candidate, providers) && fits_context(candidate))
        {
            candidates.retain(|candidate| {
                free_pool_candidate(candidate, providers) && fits_context(candidate)
            });
            return;
        }

        for tier in ["free_cloud", "local", "mixed", "paid_cloud"] {
            if candidates
                .iter()
                .any(|candidate| tier_of(candidate) == tier && fits_context(candidate))
            {
                candidates
                    .retain(|candidate| tier_of(candidate) == tier && fits_context(candidate));
                return;
            }
        }

        for tier in ["free_cloud", "local", "mixed", "paid_cloud"] {
            if candidates
                .iter()
                .any(|candidate| tier_of(candidate) == tier)
            {
                candidates.retain(|candidate| tier_of(candidate) == tier);
                return;
            }
        }
    }
}

fn explicit_fallback_tier_order_enabled(
    route_profile: &RouteExecutionProfile,
    forced_provider_id: Option<&str>,
) -> bool {
    forced_provider_id.is_none() && route_profile.execution_lane == "planning"
}

fn fallback_tier_rank(provider: &ProviderState) -> usize {
    let tier = if is_local_provider(&provider.id) {
        "local".to_string()
    } else {
        access_tier_class(provider)
    };
    fallback_tier_order()
        .iter()
        .position(|candidate| candidate == &tier)
        .unwrap_or(usize::MAX)
}

fn fallback_tier_order() -> Vec<String> {
    std::env::var("ARDA_MANWE_FALLBACK_TIER_ORDER")
        .unwrap_or_else(|_| "free_cloud,local,paid_cloud,mixed".to_string())
        .split(',')
        .map(str::trim)
        .filter(|tier| !tier.is_empty())
        .map(str::to_string)
        .collect()
}

fn large_context_tool_execution_threshold() -> usize {
    std::env::var("ARDA_MANWE_LARGE_TOOL_CONTEXT_THRESHOLD")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 32_000)
        .unwrap_or(64_000)
}

fn external_tool_candidate(
    candidate: &RouteSelectionCandidate,
    providers: &[ProviderState],
    route_profile: &RouteExecutionProfile,
) -> bool {
    let provider = &providers[candidate.provider_index];
    !is_local_provider(&provider.id)
        && (provider.supports_tools || candidate.model.capabilities.tools == Some(true))
        && candidate.model.capabilities.tools != Some(false)
        && candidate.model.context_window >= route_profile.context_window_target
        && provider.driver != "hermes_agent_cli"
}

#[expect(
    clippy::too_many_arguments,
    reason = "rejection receipts preserve explicit request, policy, and runtime evidence inputs"
)]
pub(super) fn route_rejection_records(
    providers: &[ProviderState],
    req: &ManweRequestEnvelope,
    priority: &str,
    strict: bool,
    forced_model_id: Option<&str>,
    excluded_provider_ids: &[String],
    route_profile: &RouteExecutionProfile,
    service: &CharonService,
) -> Vec<RejectedRouteCandidate> {
    providers
        .iter()
        .map(|provider| {
            let forced_provider_id = req
                .options
                .get("force_provider_id")
                .and_then(JsonValue::as_str);
            let allow_forced_provider_fallback = req
                .options
                .get("allow_forced_provider_fallback")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            let effective_forced_model_id = if allow_forced_provider_fallback
                && forced_provider_id.is_some_and(|forced| forced != provider.id)
            {
                None
            } else {
                forced_model_id
            };
            let reason = provider_rejection_reason(
                provider,
                req,
                priority,
                strict,
                effective_forced_model_id,
                excluded_provider_ids,
                service,
            );
            let details = if reason == "no_compatible_model" {
                serde_json::json!({
                    "requested_task_type": req.task_type,
                    "model_rejections": provider.models.iter().map(|model| serde_json::json!({
                        "model_id": model.id,
                        "missing_capabilities": model_request_incompatibilities(
                            &provider.id,
                            model,
                            req,
                            effective_forced_model_id,
                        ),
                    })).collect::<Vec<_>>(),
                })
            } else {
                JsonValue::Null
            };
            RejectedRouteCandidate {
                provider_id: provider.id.clone(),
                tier: if is_local_provider(&provider.id) {
                    "local".to_string()
                } else {
                    access_tier_class(provider)
                },
                reason,
                pacing_state: pacing_state_for_provider(provider, priority, route_profile),
                details,
            }
        })
        .collect()
}

fn route_rejection_error_message(
    providers: &[ProviderState],
    req: &ManweRequestEnvelope,
    priority: &str,
    strict: bool,
    forced_model_id: Option<&str>,
    excluded_provider_ids: &[String],
    service: &CharonService,
) -> String {
    let route_profile = super::route_policy::derive_route_execution_profile(req, priority);
    let rejected = route_rejection_records(
        providers,
        req,
        priority,
        strict,
        forced_model_id,
        excluded_provider_ids,
        &route_profile,
        service,
    )
    .into_iter()
    .map(|record| {
        serde_json::json!({
            "provider_id": record.provider_id,
            "tier": record.tier,
            "reason": record.reason,
            "pacing_state": record.pacing_state,
            "details": record.details,
        })
    })
    .collect::<Vec<_>>();
    format!(
        "no provider available for request (priority={}, strict={}); rejected_providers={}",
        priority,
        strict,
        JsonValue::Array(rejected)
    )
}

fn provider_rejection_reason(
    provider: &ProviderState,
    req: &ManweRequestEnvelope,
    priority: &str,
    strict: bool,
    forced_model_id: Option<&str>,
    excluded_provider_ids: &[String],
    service: &CharonService,
) -> String {
    if excluded_provider_ids
        .iter()
        .any(|excluded| excluded == &provider.id)
    {
        return "excluded_after_prior_attempt".to_string();
    }
    if !provider.enabled {
        return "provider_disabled".to_string();
    }
    if !provider.has_api_key {
        return "missing_api_key".to_string();
    }
    if !provider.healthy {
        return "provider_unhealthy".to_string();
    }
    if provider.in_cooldown {
        return "provider_cooldown".to_string();
    }
    if provider
        .requests_per_minute
        .is_some_and(|max| provider.requests_used_minute >= max)
    {
        return "minute_quota_exhausted".to_string();
    }
    if provider
        .requests_per_day
        .is_some_and(|max| provider.requests_used_day >= max)
    {
        return "daily_quota_exhausted".to_string();
    }
    if !provider_eligible(provider, priority, strict) {
        return "provider_policy_ineligible".to_string();
    }
    if !service.provider_agent_quota_available(provider, req) {
        return "agent_quota_exhausted".to_string();
    }
    if !provider_supports_request_capabilities(provider, req) {
        return "provider_declared_capability_mismatch".to_string();
    }
    if !provider_supports_request(provider, req) {
        return "provider_request_capability_mismatch".to_string();
    }
    if provider
        .base_url
        .as_deref()
        .is_some_and(|base_url| base_url.contains("${"))
    {
        return "unresolved_base_url_template".to_string();
    }
    if select_model_for_provider_request(provider, &req.task_type, forced_model_id, Some(req))
        .is_none()
    {
        return "no_compatible_model".to_string();
    }
    "filtered_by_route_policy".to_string()
}

fn orchestration_grade_candidate(
    candidate: &RouteSelectionCandidate,
    providers: &[ProviderState],
) -> bool {
    let provider = &providers[candidate.provider_index];
    let model_id = candidate.model.id.to_ascii_lowercase();

    if is_primary_local_surface_provider(&provider.id) {
        return true;
    }
    if provider.driver == "hermes_agent_cli" || provider.driver == "codex_responses" {
        return true;
    }
    if matches!(provider.id.as_str(), "openai_sub" | "anthropic") {
        return true;
    }
    if let Some(params_b) = parse_model_params_billions(&model_id) {
        return params_b >= 30.0;
    }
    if contains_any(
        &model_id,
        &[
            "gpt-5",
            "claude",
            "opus",
            "sonnet",
            "codestral",
            "devstral",
            "nemotron-3-super",
            "nemotron-3-ultra",
            "qwen3-coder-480b",
            "mistral-large",
            "mistral-code-agent",
            "kimi-k2",
            "glm-5",
        ],
    ) {
        return true;
    }
    !contains_any(
        &model_id,
        &[
            "1.2b",
            "3b",
            "4b",
            "7b",
            "8b",
            "9b",
            "mini",
            "nano",
            "small",
            "flash-lite",
        ],
    ) && access_tier_class(provider) == "paid_cloud"
        && provider.quality_band.eq_ignore_ascii_case("high")
}

fn free_pool_candidate(candidate: &RouteSelectionCandidate, providers: &[ProviderState]) -> bool {
    let provider = &providers[candidate.provider_index];
    if provider_free_pool_quota_blocked(provider) {
        return false;
    }
    if access_tier_class(provider) == "free_cloud" {
        return true;
    }
    let model_id = candidate.model.id.to_ascii_lowercase();
    if model_id.contains(":free") || model_id.ends_with("-free") || model_id.contains("/free") {
        return true;
    }
    configured_free_pool_provider(&provider.id)
        && provider.has_api_key
        && provider.healthy
        && !provider.in_cooldown
}

fn provider_free_pool_quota_blocked(provider: &ProviderState) -> bool {
    provider
        .requests_per_minute
        .is_some_and(|max| max > 0 && provider.requests_used_minute >= max.saturating_mul(9) / 10)
        || provider
            .requests_per_day
            .is_some_and(|max| max > 0 && provider.requests_used_day >= max.saturating_mul(9) / 10)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn free_external_tool_pool_min_candidates() -> usize {
    std::env::var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(3)
        .min(16)
}

fn request_allows_free_tool_pool(req: &ManweRequestEnvelope) -> bool {
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

fn configured_free_pool_provider(provider_id: &str) -> bool {
    let configured = std::env::var("ARDA_MANWE_FREE_POOL_PROVIDER_IDS")
        .unwrap_or_else(|_| default_free_pool_provider_ids().join(","));
    configured
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .any(|item| item == provider_id)
}

fn default_free_pool_provider_ids() -> Vec<&'static str> {
    vec!["openrouter", "groq", "cerebras", "google", "opencode"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::service::{
        derive_route_execution_profile, resolve_hybrid_route_policy,
    };
    use crate::adaptive::types::{ModelCapabilities, ModelState, ProviderState};
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn provider(id: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: id.to_string(),
            base_url: Some("https://example.invalid/v1".to_string()),
            api_key_env: Some("EXAMPLE_API_KEY".to_string()),
            access_tier: "mixed".to_string(),
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
            requests_per_minute: None,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: None,
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                id: "provider/model".to_string(),
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
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        }
    }

    fn provider_with_model(id: &str, model_id: &str) -> ProviderState {
        let mut provider = provider(id);
        provider.models[0].id = model_id.to_string();
        provider.models[0].capable_tasks = vec!["code".to_string(), "chat".to_string()];
        provider
    }

    fn execution_req() -> ManweRequestEnvelope {
        ManweRequestEnvelope {
            agent_id: "hermes".to_string(),
            task_type: "code".to_string(),
            priority: "normal".to_string(),
            messages: vec![serde_json::json!({"role":"user","content":"edit files"})],
            options: serde_json::json!({
                "tool_use_required": true,
                "workload_role": "execution"
            }),
        }
    }

    fn package_runtime_signals() -> PackageRuntimeSignals {
        PackageRuntimeSignals {
            generated_at_utc: String::new(),
            llmfit_backend: "optional_signal_absent".to_string(),
            llmfit_recommendation_count: 0,
            llmfit_local_max_params_b: None,
            llmfit_top_model_names: Vec::new(),
            nanoclaw_binary_present: false,
            nanoclaw_runtime_ready: false,
            nanoclaw_probe_state: "not_configured".to_string(),
        }
    }

    #[test]
    fn free_pool_provider_ids_are_env_configurable() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var(
            "ARDA_MANWE_FREE_POOL_PROVIDER_IDS",
            "openrouter,google",
        );
        assert!(configured_free_pool_provider("openrouter"));
        assert!(configured_free_pool_provider("google"));
        assert!(!configured_free_pool_provider("nvidia"));
        std::env::remove_var("ARDA_MANWE_FREE_POOL_PROVIDER_IDS");
    }

    #[test]
    fn default_free_pool_provider_ids_cover_volatile_cloud_pool() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FREE_POOL_PROVIDER_IDS");
        for provider_id in ["openrouter", "groq", "cerebras", "google", "opencode"] {
            assert!(
                configured_free_pool_provider(provider_id),
                "{provider_id} should be admitted to the default free provider pool"
            );
        }
        assert!(!configured_free_pool_provider("nvidia"));
    }

    #[test]
    fn mixed_tier_provider_can_enter_free_pool_by_configured_id() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("ARDA_MANWE_FREE_POOL_PROVIDER_IDS", "google");
        let providers = vec![provider("google")];
        let candidate = RouteSelectionCandidate {
            provider_index: 0,
            model: providers[0].models[0].clone(),
            score: 0.0,
        };

        assert!(free_pool_candidate(&candidate, &providers));
        std::env::remove_var("ARDA_MANWE_FREE_POOL_PROVIDER_IDS");
    }

    #[test]
    fn free_tool_pool_does_not_collapse_to_tiny_free_set() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES");
        let mut free = provider("cerebras");
        free.access_tier = "free_cloud".to_string();
        let mut paid = provider("mistral");
        paid.access_tier = "paid_cloud".to_string();
        let providers = vec![free, paid];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "low".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_free_external_tool_route_candidates(
            &mut candidates,
            &providers,
            &execution_req(),
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn low_cost_policy_does_not_hide_fallbacks_behind_tiny_free_set() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES");
        let mut free = provider("cerebras");
        free.access_tier = "free_cloud".to_string();
        let mut paid = provider("mistral");
        paid.access_tier = "paid_cloud".to_string();
        let providers = vec![free, paid];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "low".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };

        CharonService::retain_low_cost_candidates(&mut candidates, &providers, &policy, None);

        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn explicit_fallback_tier_order_keeps_local_before_paid_cloud_for_planning() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FALLBACK_TIER_ORDER");
        let mut local = provider("local_fallback");
        local.access_tier = "mixed".to_string();
        let mut paid = provider("openai_sub");
        paid.access_tier = "paid_cloud".to_string();
        let providers = vec![local, paid];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: if provider.id == "openai_sub" {
                    100.0
                } else {
                    10.0
                },
            })
            .collect::<Vec<_>>();
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "planning".to_string(),
            context_window_target: 16_000,
        };

        CharonService::retain_highest_explicit_fallback_tier(
            &mut candidates,
            &providers,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(providers[candidates[0].provider_index].id, "local_fallback");
        std::env::remove_var("ARDA_MANWE_FALLBACK_TIER_ORDER");
    }

    #[test]
    fn explicit_fallback_tier_order_does_not_collapse_execution_candidates() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FALLBACK_TIER_ORDER");
        let mut free = provider("cerebras");
        free.access_tier = "free_cloud".to_string();
        let mut paid = provider("openai_sub");
        paid.access_tier = "paid_cloud".to_string();
        let providers = vec![free, paid];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: if provider.id == "openai_sub" {
                    100.0
                } else {
                    10.0
                },
            })
            .collect::<Vec<_>>();
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 16_000,
        };

        CharonService::retain_highest_explicit_fallback_tier(
            &mut candidates,
            &providers,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 2);
        std::env::remove_var("ARDA_MANWE_FALLBACK_TIER_ORDER");
    }

    #[test]
    fn orchestration_grade_execution_filter_drops_slim_worker_when_executor_exists() {
        let mut worker = provider_with_model("edge_core", "LFM2.5-8B-A1B-Q4_K_M");
        worker.access_tier = "local".to_string();
        let mut backbone = provider_with_model("edge_backbone", "Qwen3.6-35B-A3B-UD-Q4_K_XL");
        backbone.access_tier = "local".to_string();
        let mut subscription = provider_with_model("openai_sub", "gpt-5.5");
        subscription.access_tier = "paid_cloud".to_string();
        subscription.driver = "codex_responses".to_string();
        let providers = vec![worker, backbone, subscription];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "balanced".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_orchestration_grade_execution_candidates(
            &mut candidates,
            &providers,
            &policy,
            &profile,
            None,
        );

        let retained = candidates
            .iter()
            .map(|candidate| providers[candidate.provider_index].id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(retained, vec!["edge_backbone", "openai_sub"]);
    }

    #[test]
    fn orchestration_grade_execution_filter_keeps_backbone_coder_surface() {
        let mut worker = provider_with_model("edge_core", "LFM2.5-8B-A1B-Q4_K_M");
        worker.access_tier = "local".to_string();
        let mut coder = provider_with_model(
            "edge_backbone_coder",
            "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL",
        );
        coder.access_tier = "local".to_string();
        let mut subscription = provider_with_model("openai_sub", "gpt-5.5");
        subscription.access_tier = "paid_cloud".to_string();
        subscription.driver = "codex_responses".to_string();
        let providers = vec![worker, coder, subscription];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "balanced".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_orchestration_grade_execution_candidates(
            &mut candidates,
            &providers,
            &policy,
            &profile,
            None,
        );

        let retained = candidates
            .iter()
            .map(|candidate| providers[candidate.provider_index].id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(retained, vec!["edge_backbone_coder", "openai_sub"]);
    }

    #[test]
    fn orchestration_grade_execution_filter_keeps_slim_worker_when_it_is_only_candidate() {
        let mut worker = provider_with_model("edge_core", "LFM2.5-8B-A1B-Q4_K_M");
        worker.access_tier = "local".to_string();
        let providers = vec![worker];
        let mut candidates = vec![RouteSelectionCandidate {
            provider_index: 0,
            model: providers[0].models[0].clone(),
            score: 100.0,
        }];
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "balanced".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_orchestration_grade_execution_candidates(
            &mut candidates,
            &providers,
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(providers[candidates[0].provider_index].id, "edge_core");
    }

    #[test]
    fn large_context_tool_execution_prefers_external_tool_candidate() {
        let mut coder = provider_with_model(
            "edge_backbone_coder",
            "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL",
        );
        coder.access_tier = "local".to_string();
        coder.models[0].context_window = 65_536;
        let mut subscription = provider_with_model("openai_sub", "gpt-5.5");
        subscription.access_tier = "paid_cloud".to_string();
        subscription.driver = "codex_responses".to_string();
        subscription.models[0].context_window = 1_050_000;
        let providers = vec![coder, subscription];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: if provider.id == "edge_backbone_coder" {
                    120.0
                } else {
                    80.0
                },
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "balanced".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_large_context_external_tool_candidates(
            &mut candidates,
            &providers,
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(providers[candidates[0].provider_index].id, "openai_sub");
    }

    #[test]
    fn large_context_tool_execution_keeps_local_when_privacy_requires_it() {
        let mut coder = provider_with_model(
            "edge_backbone_coder",
            "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL",
        );
        coder.access_tier = "local".to_string();
        let mut subscription = provider_with_model("openai_sub", "gpt-5.5");
        subscription.access_tier = "paid_cloud".to_string();
        subscription.driver = "codex_responses".to_string();
        let providers = vec![coder, subscription];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "confidential".to_string(),
            cost_tier: "balanced".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: true,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_large_context_external_tool_candidates(
            &mut candidates,
            &providers,
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn large_context_tool_execution_keeps_external_model_level_tool_truth() {
        let mut stale_wrapper = provider_with_model("subscription_wrapper", "frontier-tool-model");
        stale_wrapper.access_tier = "paid_cloud".to_string();
        stale_wrapper.supports_tools = false;
        stale_wrapper.models[0].capabilities.tools = Some(true);
        stale_wrapper.models[0].context_window = 128_000;
        let mut local = provider_with_model("edge_backbone_coder", "local-coder");
        local.access_tier = "local".to_string();
        local.models[0].context_window = 128_000;
        let providers = vec![local, stale_wrapper];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "balanced".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_large_context_external_tool_candidates(
            &mut candidates,
            &providers,
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            providers[candidates[0].provider_index].id,
            "subscription_wrapper"
        );
    }

    #[test]
    fn route_rejection_message_explains_provider_filters() {
        let dir = tempdir().expect("tempdir");
        let service = CharonService::new(dir.path()).expect("service");
        let mut no_tools = provider("openai_sub");
        no_tools.access_tier = "paid_cloud".to_string();
        no_tools.supports_tools = false;
        let mut exhausted = provider("openrouter");
        exhausted.access_tier = "free_cloud".to_string();
        exhausted.requests_per_day = Some(10);
        exhausted.requests_used_day = 10;
        let req = ManweRequestEnvelope {
            agent_id: "hermes".to_string(),
            task_type: "code".to_string(),
            priority: "normal".to_string(),
            messages: vec![serde_json::json!({"role":"user","content":"edit"})],
            options: serde_json::json!({
                "tools": [{
                    "type": "function",
                    "function": {"name": "apply_patch", "parameters": {"type": "object"}}
                }]
            }),
        };

        let message = route_rejection_error_message(
            &[no_tools, exhausted],
            &req,
            "normal",
            false,
            None,
            &[],
            &service,
        );

        assert!(message.contains("rejected_providers="));
        assert!(message.contains("provider_declared_capability_mismatch"));
        assert!(message.contains("daily_quota_exhausted"));
    }

    #[test]
    fn forced_provider_fallback_keeps_compatible_beelink_candidate() {
        let dir = tempdir().expect("tempdir");
        let service = CharonService::new(dir.path()).expect("service");
        let mut edge_core = provider_with_model("edge_core", "LFM2.5-8B-A1B-Q4_K_M");
        edge_core.enabled = false;
        let mut beelink = provider_with_model(
            "edge_beelink_light",
            "Ternary-Bonsai-8B-Q2_0",
        );
        beelink.access_tier = "local".to_string();
        beelink.models[0].context_window = 131_072;
        beelink.models[0].capabilities.tools = Some(true);
        let providers = vec![edge_core, beelink];
        let mut req = execution_req();
        req.options["force_provider_id"] = serde_json::json!("edge_core");
        req.options["force_model_id"] = serde_json::json!("LFM2.5-8B-A1B-Q4_K_M");
        req.options["allow_forced_provider_fallback"] = serde_json::json!(true);
        req.options["origin_preference"] = serde_json::json!("local");
        let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
        let profile = derive_route_execution_profile(&req, "normal");

        let candidates = service.build_scored_route_candidates(
            &providers,
            &req,
            "normal",
            false,
            Some("edge_core"),
            Some("LFM2.5-8B-A1B-Q4_K_M"),
            &[],
            &policy,
            &profile,
            &package_runtime_signals(),
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(providers[candidates[0].provider_index].id, "edge_beelink_light");
        assert_eq!(candidates[0].model.id, "Ternary-Bonsai-8B-Q2_0");

        service.cache_route_candidates(
            &providers,
            &req,
            "normal",
            false,
            Some("edge_core"),
            Some("LFM2.5-8B-A1B-Q4_K_M"),
            &profile,
            &candidates,
        );
        let cached = service
            .cached_route_candidates(
                &providers,
                &req,
                "normal",
                false,
                Some("edge_core"),
                Some("LFM2.5-8B-A1B-Q4_K_M"),
                &[],
                &profile,
            )
            .expect("fallback candidate should survive a cache hit");
        assert_eq!(cached.len(), 1);
        assert_eq!(providers[cached[0].provider_index].id, "edge_beelink_light");
    }

    #[test]
    fn no_compatible_model_diagnostic_names_each_missing_requirement() {
        let dir = tempdir().expect("tempdir");
        let service = CharonService::new(dir.path()).expect("service");
        let mut provider = provider_with_model("edge_beelink_light", "small-chat-model");
        provider.supports_tools = true;
        provider.supports_structured_output = true;
        provider.models[0].capable_tasks = vec!["chat".to_string()];
        provider.models[0].is_default = false;
        provider.models[0].context_window = 32_768;
        provider.models[0].capabilities.tools = Some(false);
        provider.models[0].capabilities.structured_output = Some(false);
        let mut req = execution_req();
        req.options["response_format"] = serde_json::json!({"type": "json_object"});
        req.options["context_window_target"] = serde_json::json!(64_000);

        let message = route_rejection_error_message(
            &[provider],
            &req,
            "normal",
            false,
            None,
            &[],
            &service,
        );

        assert!(message.contains("model_role:missing_task_type:code"), "{message}");
        assert!(message.contains("context_window:required=72192,available=32768"), "{message}");
        assert!(message.contains("tools:required_but_unsupported"), "{message}");
        assert!(message.contains("structured_output:required_but_unsupported"), "{message}");
    }

    #[test]
    fn transient_cooldown_uses_bounded_fallback_candidate() {
        let dir = tempdir().expect("tempdir");
        let service = CharonService::new(dir.path()).expect("service");
        let mut provider = provider_with_model("transient", "chat-model");
        provider.in_cooldown = true;
        provider.cooldown_backoff_seconds = 30;
        provider.last_error = Some("temporary upstream 502".to_string());
        let providers = vec![provider];
        let req = ManweRequestEnvelope {
            agent_id: "agent".to_string(),
            task_type: "chat".to_string(),
            priority: "normal".to_string(),
            messages: vec![serde_json::json!({"role": "user", "content": "hello"})],
            options: serde_json::json!({}),
        };
        let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
        let profile = derive_route_execution_profile(&req, "normal");

        let selected = service
            .select_route_candidate(
                &providers,
                &req,
                "normal",
                false,
                None,
                None,
                &[],
                &policy,
                &profile,
                &package_runtime_signals(),
            )
            .expect("short transient cooldown should retain a fallback route");

        assert_eq!(selected.provider_index, 0);
        assert_eq!(selected.model.id, "chat-model");
    }

    #[test]
    fn free_tool_pool_retains_free_set_when_viable() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES");
        let providers = ["cerebras", "opencode", "groq", "mistral"]
            .into_iter()
            .map(|id| {
                let mut provider = provider(id);
                provider.access_tier = if id == "mistral" {
                    "paid_cloud".to_string()
                } else {
                    "free_cloud".to_string()
                };
                provider
            })
            .collect::<Vec<_>>();
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "low".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };
        let mut req = execution_req();
        req.options["allow_free_tool_pool"] = serde_json::Value::Bool(true);

        CharonService::retain_free_external_tool_route_candidates(
            &mut candidates,
            &providers,
            &req,
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 3);
        assert!(candidates
            .iter()
            .all(|candidate| providers[candidate.provider_index].access_tier == "free_cloud"));
    }

    #[test]
    fn free_tool_pool_ignores_near_exhausted_free_providers() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES", "2");
        let mut openrouter = provider("openrouter");
        openrouter.access_tier = "mixed".to_string();
        openrouter.requests_per_day = Some(50);
        openrouter.requests_used_day = 45;
        openrouter.models[0].id = "nvidia/nemotron-3-ultra-550b-a55b:free".to_string();

        let mut groq = provider("groq");
        groq.access_tier = "free_cloud".to_string();
        let mut cerebras = provider("cerebras");
        cerebras.access_tier = "free_cloud".to_string();
        let mut paid = provider("mistral");
        paid.access_tier = "paid_cloud".to_string();

        let providers = vec![openrouter, groq, cerebras, paid];
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "low".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };
        let mut req = execution_req();
        req.options["allow_free_tool_pool"] = serde_json::Value::Bool(true);

        CharonService::retain_free_external_tool_route_candidates(
            &mut candidates,
            &providers,
            &req,
            &policy,
            &profile,
            None,
        );

        let retained = candidates
            .iter()
            .map(|candidate| providers[candidate.provider_index].id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(retained, vec!["groq", "cerebras"]);
        std::env::remove_var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES");
    }

    #[test]
    fn free_tool_pool_is_opt_in_for_execution_routes() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_FREE_TOOL_POOL_MIN_CANDIDATES");
        let providers = ["cerebras", "opencode", "groq", "mistral"]
            .into_iter()
            .map(|id| {
                let mut provider = provider(id);
                provider.access_tier = if id == "mistral" {
                    "paid_cloud".to_string()
                } else {
                    "free_cloud".to_string()
                };
                provider
            })
            .collect::<Vec<_>>();
        let mut candidates = providers
            .iter()
            .enumerate()
            .map(|(provider_index, provider)| RouteSelectionCandidate {
                provider_index,
                model: provider.models[0].clone(),
                score: 100.0,
            })
            .collect::<Vec<_>>();
        let policy = HybridRoutePolicy {
            privacy_tier: "public".to_string(),
            cost_tier: "low".to_string(),
            quality_tier: "high".to_string(),
            origin_preference: "auto".to_string(),
            latency_sla_ms: None,
            require_local: false,
            spread_score_band: 0.03,
            spread_top_cap: 3,
        };
        let profile = RouteExecutionProfile {
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            context_window_target: 64_000,
        };

        CharonService::retain_free_external_tool_route_candidates(
            &mut candidates,
            &providers,
            &execution_req(),
            &policy,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 4);
    }

    #[test]
    fn fast_lane_prefers_direct_provider_over_hermes_cli_when_available() {
        let mut hermes = provider("openai_sub");
        hermes.driver = "hermes_agent_cli".to_string();
        let direct = provider("openrouter");
        let providers = vec![hermes, direct];
        let mut candidates = vec![
            RouteSelectionCandidate {
                provider_index: 0,
                model: providers[0].models[0].clone(),
                score: 100.0,
            },
            RouteSelectionCandidate {
                provider_index: 1,
                model: providers[1].models[0].clone(),
                score: 20.0,
            },
        ];
        let req = ManweRequestEnvelope {
            agent_id: "hermes".to_string(),
            task_type: "chat".to_string(),
            priority: "normal".to_string(),
            messages: vec![serde_json::json!({"role":"user","content":"hello"})],
            options: serde_json::json!({}),
        };
        let profile = RouteExecutionProfile {
            route_class: "interactive_chat".to_string(),
            execution_lane: "interactive".to_string(),
            context_window_target: 16_000,
        };

        CharonService::retain_fast_lane_non_hermes_cli_candidates(
            &mut candidates,
            &providers,
            &req,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(providers[candidates[0].provider_index].id, "openrouter");
    }

    #[test]
    fn fast_lane_allows_hermes_cli_when_explicitly_requested() {
        let mut hermes = provider("openai_sub");
        hermes.driver = "hermes_agent_cli".to_string();
        let direct = provider("openrouter");
        let providers = vec![hermes, direct];
        let mut candidates = vec![
            RouteSelectionCandidate {
                provider_index: 0,
                model: providers[0].models[0].clone(),
                score: 100.0,
            },
            RouteSelectionCandidate {
                provider_index: 1,
                model: providers[1].models[0].clone(),
                score: 20.0,
            },
        ];
        let req = ManweRequestEnvelope {
            agent_id: "hermes".to_string(),
            task_type: "chat".to_string(),
            priority: "normal".to_string(),
            messages: vec![serde_json::json!({"role":"user","content":"hello"})],
            options: serde_json::json!({"allow_slow_subscription_routes": true}),
        };
        let profile = RouteExecutionProfile {
            route_class: "interactive_chat".to_string(),
            execution_lane: "interactive".to_string(),
            context_window_target: 16_000,
        };

        CharonService::retain_fast_lane_non_hermes_cli_candidates(
            &mut candidates,
            &providers,
            &req,
            &profile,
            None,
        );

        assert_eq!(candidates.len(), 2);
    }
}
