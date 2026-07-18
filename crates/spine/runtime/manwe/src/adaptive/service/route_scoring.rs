use crate::adaptive::service::route_policy::{
    HybridRoutePolicy, LaneFitnessSnapshot, RouteExecutionProfile, RouteSelectionCandidate,
};
use crate::adaptive::service::status::PackageRuntimeSignals;
use crate::adaptive::service::types::{ModelState, ProviderState};

pub(super) fn apply_soft_lane_caps(
    candidates: &mut Vec<RouteSelectionCandidate>,
    providers: &[ProviderState],
    route_profile: &RouteExecutionProfile,
) {
    let constrained = candidates
        .iter()
        .filter(|candidate| {
            let provider = &providers[candidate.provider_index];
            provider.active_connections < provider_soft_lane_cap(&provider.id, route_profile)
        })
        .cloned()
        .collect::<Vec<_>>();
    if !constrained.is_empty() {
        *candidates = constrained;
    }
}

fn provider_soft_lane_cap(id: &str, route_profile: &RouteExecutionProfile) -> u32 {
    match route_profile.execution_lane.as_str() {
        "orchestrator" => match id {
            "edge_backbone" => 2,
            "edge_backbone_long" => 1,
            "edge_worker_light" => 1,
            "edge_guardhouse" => 0,
            _ => 3,
        },
        "execution" => match id {
            "edge_backbone" => 6,
            "edge_backbone_long" => 2,
            "edge_worker_light" => 2,
            "edge_laptop" => 1,
            "edge_guardhouse" => 0,
            _ => 1,
        },
        "background" => match id {
            "edge_backbone" => 8,
            "edge_backbone_long" => 1,
            "edge_guardhouse" => 2,
            "edge_laptop" => 2,
            "edge_worker_light" => 2,
            _ => 1,
        },
        _ => match id {
            "edge_backbone" => 6,
            "edge_backbone_long" => 1,
            "edge_worker_light" => 3,
            "edge_laptop" => 2,
            "edge_guardhouse" => 1,
            _ => 2,
        },
    }
}

fn provider_hard_lane_cap(id: &str, route_profile: &RouteExecutionProfile) -> u32 {
    match route_profile.execution_lane.as_str() {
        "orchestrator" => match id {
            "edge_backbone" => 4,
            "edge_backbone_long" => 1,
            "edge_worker_light" => 2,
            "edge_guardhouse" => 1,
            _ => 5,
        },
        "execution" => match id {
            "edge_backbone" => 10,
            "edge_backbone_long" => 2,
            "edge_worker_light" => 3,
            "edge_laptop" => 2,
            "edge_guardhouse" => 1,
            _ => 2,
        },
        "compression" => match id {
            "edge_backbone" => 3,
            "edge_backbone_long" => 1,
            "edge_worker_light" => 1,
            "edge_laptop" => 1,
            "edge_guardhouse" => 1,
            _ => 3,
        },
        "background" => match id {
            "edge_backbone" => 12,
            "edge_backbone_long" => 1,
            "edge_guardhouse" => 3,
            "edge_laptop" => 3,
            "edge_worker_light" => 3,
            _ => 2,
        },
        _ => match id {
            "edge_backbone" => 10,
            "edge_backbone_long" => 2,
            "edge_worker_light" => 4,
            "edge_laptop" => 3,
            "edge_guardhouse" => 2,
            _ => 3,
        },
    }
}

fn provider_queue_pressure_penalty(
    p: &ProviderState,
    route_profile: &RouteExecutionProfile,
) -> f64 {
    let soft_cap = provider_soft_lane_cap(&p.id, route_profile);
    let hard_cap = provider_hard_lane_cap(&p.id, route_profile).max(soft_cap.max(1));
    let active = p.active_connections;
    let utilization = active as f64 / hard_cap as f64;
    let mut penalty = 0.0;

    if active > 0 {
        penalty += provider_connection_penalty(&p.id, active);
    }
    if active >= soft_cap.max(1) {
        penalty += ((active.saturating_sub(soft_cap) + 1) as f64) * 18.0;
    }
    if utilization >= 0.85 {
        penalty += 28.0;
    } else if utilization >= 0.60 {
        penalty += 14.0;
    }

    if let Some(lat) = p.avg_latency_ms {
        let latency_penalty = match route_profile.execution_lane.as_str() {
            "execution" => {
                if lat > 45_000 {
                    72.0
                } else if lat > 30_000 {
                    56.0
                } else if lat > 20_000 {
                    40.0
                } else if lat > 10_000 {
                    24.0
                } else if lat > 5_000 {
                    10.0
                } else {
                    0.0
                }
            }
            "compression" | "background" => {
                if lat > 60_000 {
                    48.0
                } else if lat > 45_000 {
                    36.0
                } else if lat > 25_000 {
                    18.0
                } else {
                    0.0
                }
            }
            _ => {
                if lat > 45_000 {
                    54.0
                } else if lat > 30_000 {
                    36.0
                } else if lat > 15_000 {
                    18.0
                } else if lat > 8_000 {
                    10.0
                } else if lat > 4_000 {
                    4.0
                } else {
                    0.0
                }
            }
        };
        penalty += latency_penalty;
    }

    penalty
}

fn provider_queue_headroom(p: &ProviderState, route_profile: &RouteExecutionProfile) -> f64 {
    let hard_cap = provider_hard_lane_cap(&p.id, route_profile).max(1);
    ((hard_cap.saturating_sub(p.active_connections)) as f64 / hard_cap as f64).clamp(0.0, 1.0)
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

fn provider_throughput_adjustment(p: &ProviderState, route_profile: &RouteExecutionProfile) -> f64 {
    let Some(latency_ms) = p.avg_latency_ms else {
        return 0.0;
    };
    let target = lane_latency_target_ms(route_profile) as f64;
    let observed = latency_ms as f64;
    let ratio = observed / target.max(1.0);

    let latency_sensitive_lane = matches!(
        route_profile.execution_lane.as_str(),
        "interactive" | "monitoring" | "execution"
    );
    let mut adjustment = if ratio <= 0.60 {
        14.0
    } else if ratio <= 0.85 {
        8.0
    } else if ratio <= 1.10 {
        2.0
    } else if ratio <= 1.50 {
        -8.0
    } else if ratio <= 2.00 {
        -16.0
    } else if latency_sensitive_lane && ratio > 4.0 {
        -76.0
    } else if latency_sensitive_lane {
        -56.0
    } else if route_profile.execution_lane == "background" {
        -36.0
    } else {
        -34.0
    };

    if p.consecutive_successes >= 4 && ratio <= 1.10 {
        adjustment += 4.0;
    }
    if p.consecutive_failures >= 2 {
        adjustment -= 8.0;
    }

    adjustment
}

fn lane_fitness_adjustment(
    p: &ProviderState,
    route_profile: &RouteExecutionProfile,
    snapshot: &LaneFitnessSnapshot,
) -> f64 {
    let Some(provider_lane) = snapshot
        .lanes
        .get(&route_profile.execution_lane)
        .and_then(|lane| lane.get(&p.id))
    else {
        return 0.0;
    };

    let reliability = provider_lane.success_count as f64
        / (provider_lane.success_count + provider_lane.failure_count).max(1) as f64;
    let mut adjustment = 0.0;

    if let Some(latency_ms) = provider_lane.avg_latency_ms {
        let target = lane_latency_target_ms(route_profile) as f64;
        let ratio = latency_ms as f64 / target.max(1.0);
        let latency_sensitive_lane = matches!(
            route_profile.execution_lane.as_str(),
            "interactive" | "monitoring" | "execution"
        );
        adjustment += if ratio <= 0.75 {
            10.0
        } else if ratio <= 1.0 {
            4.0
        } else if latency_sensitive_lane && ratio > 4.0 {
            -64.0
        } else if latency_sensitive_lane && ratio > 2.0 {
            -44.0
        } else if ratio <= 1.5 {
            -6.0
        } else if route_profile.execution_lane == "background" {
            -30.0
        } else {
            -24.0
        };
    }

    if provider_lane.success_count >= 3 {
        adjustment += ((reliability - 0.5) * 20.0).clamp(-8.0, 10.0);
    }

    adjustment
}

pub(super) fn provider_score(
    p: &ProviderState,
    model: &ModelState,
    priority: &str,
    policy: &HybridRoutePolicy,
    route_profile: &RouteExecutionProfile,
    package_runtime: &PackageRuntimeSignals,
    lane_fitness: &LaneFitnessSnapshot,
) -> f64 {
    let mut score = 100.0;
    let local_context_viable = is_primary_local_surface_provider(&p.id)
        && model.context_window >= route_profile.context_window_target;
    score += provider_lane_bias(p, route_profile, priority, policy);
    if let Some(max) = p.requests_per_day {
        if max > 0 {
            let used_ratio = p.requests_used_day as f64 / max as f64;
            score -= used_ratio * 25.0;
        }
    }
    if let Some(max) = p.requests_per_minute {
        if max > 0 {
            let used_ratio = p.requests_used_minute as f64 / max as f64;
            score -= used_ratio * 12.0;
        }
    }
    score -= p.consecutive_failures as f64 * 10.0;
    score -= p.error_count.min(50) as f64 * 0.3;
    score -= provider_queue_pressure_penalty(p, route_profile);
    score += provider_queue_headroom(p, route_profile) * 8.0;
    score += provider_throughput_adjustment(p, route_profile);
    score += lane_fitness_adjustment(p, route_profile, lane_fitness);
    if route_profile.route_class == "health_probe" {
        score += health_probe_bias(p, model);
    }
    if p.driver == "hermes_agent_cli"
        && matches!(
            route_profile.execution_lane.as_str(),
            "interactive" | "monitoring"
        )
    {
        score -= 55.0;
    }

    if is_local_fallback(&p.id) {
        if is_high_priority(priority) {
            score -= 40.0;
        } else if is_background_priority(priority) {
            score += 20.0;
        }
    }
    if policy.cost_tier == "low" {
        match access_tier_class(p).as_str() {
            "local" => score += 26.0,
            "free_cloud" => score += 18.0,
            "mixed" => score -= 4.0,
            "paid_cloud" => score -= 24.0,
            _ => score -= 12.0,
        }
    }
    score += model_cost_adjustment(model, policy);
    if route_profile.execution_lane == "orchestrator" && policy.cost_tier == "low" {
        if policy.origin_preference == "local" {
            if access_tier_class(p) == "free_cloud" {
                score -= 12.0;
            } else if access_tier_class(p) == "paid_cloud" {
                score -= 36.0;
            } else if access_tier_class(p) == "mixed" {
                score -= 18.0;
            }
        } else if access_tier_class(p) == "free_cloud" {
            score += 24.0;
        } else if access_tier_class(p) == "paid_cloud" {
            score -= 28.0;
        } else if is_local_fallback(&p.id) || is_direct_local_provider(&p.id) {
            score -= 18.0;
        }
    }
    if policy.quality_tier == "high" {
        match quality_band_class(p).as_str() {
            "high" => score += 18.0,
            "medium" => score += 4.0,
            "low" => score -= 10.0,
            _ => {}
        }
    }
    if model.context_window >= route_profile.context_window_target {
        score += 14.0;
    } else {
        let miss_ratio = route_profile
            .context_window_target
            .saturating_sub(model.context_window) as f64
            / route_profile.context_window_target.max(1) as f64;
        score -= (miss_ratio * 22.0).min(22.0);
    }
    match route_profile.execution_lane.as_str() {
        "orchestrator" => {
            if local_context_viable {
                score += 34.0;
                if matches!(policy.cost_tier.as_str(), "low" | "balanced") {
                    score += 10.0;
                }
            } else if is_primary_local_surface_provider(&p.id) {
                score += 10.0;
            }

            if policy.origin_preference == "local" {
                if matches!(access_tier_class(p).as_str(), "free_cloud" | "mixed") {
                    score -= 18.0;
                } else if access_tier_class(p) == "paid_cloud" {
                    score -= 28.0;
                } else if is_cloud_provider(&p.id) {
                    score -= 10.0;
                }
            } else if access_tier_class(p) == "free_cloud" {
                score += 14.0;
            } else if access_tier_class(p) == "mixed" {
                score -= 10.0;
            } else if access_tier_class(p) == "paid_cloud" {
                score -= 18.0;
            } else if is_cloud_provider(&p.id) {
                score += 10.0;
            }

            if route_profile.context_window_target >= 128_000 {
                if access_tier_class(p) == "mixed" {
                    score -= 8.0;
                }
                if access_tier_class(p) == "paid_cloud" {
                    score -= 12.0;
                }
                if is_primary_local_surface_provider(&p.id)
                    && p.avg_latency_ms.is_some_and(|lat| lat <= 25_000)
                {
                    score += 10.0;
                }
            }

            if !local_context_viable {
                if is_primary_local_surface_provider(&p.id) {
                    score += 8.0;
                }
            } else if is_direct_local_provider(&p.id) || is_local_fallback(&p.id) {
                score -= 10.0;
            }
        }
        "planning" => {
            if !is_local_provider(&p.id) {
                score += 10.0;
            }
        }
        "execution" => {
            if is_primary_local_surface_provider(&p.id) {
                score += 26.0;
            } else if is_direct_local_provider(&p.id) {
                score += 4.0;
            } else if is_local_fallback(&p.id) {
                score -= 4.0;
            } else {
                score -= 8.0;
            }
        }
        "background" => {
            if is_primary_local_surface_provider(&p.id) || is_local_fallback(&p.id) {
                score += 12.0;
            }
        }
        _ => {}
    }
    if route_profile.route_class == "audit_stability" {
        score += audit_stability_bias(p, model);
    }
    if let Some(sla) = policy.latency_sla_ms {
        if let Some(lat) = p.avg_latency_ms {
            if lat <= sla {
                score += 12.0;
            } else {
                score -= ((lat - sla) as f64 / sla.max(1) as f64 * 25.0).min(35.0);
            }
        } else if sla <= 1500 {
            score -= 6.0;
        }
    } else if let Some(lat) = effective_route_latency_ms(p, model) {
        // Soft latency awareness even without an explicit SLA. Execution and
        // interactive routes are operator-facing tool loops, so stale 30s+
        // latency must outweigh local-surface bias unless policy requires local.
        let lane_floor_ms: u64 = match route_profile.execution_lane.as_str() {
            "interactive" | "monitoring" => 2_000,
            "compression" => 20_000,
            "background" => 30_000,
            "execution" => 7_500,
            "orchestrator" => 10_000,
            _ => 5_000,
        };
        if lat > lane_floor_ms {
            let over = (lat - lane_floor_ms) as f64;
            let lane_max_penalty: f64 = match route_profile.execution_lane.as_str() {
                "interactive" | "monitoring" => 45.0,
                "execution" => 50.0,
                "compression" => 36.0,
                "background" => 25.0,
                "orchestrator" => 35.0,
                _ => 20.0,
            };
            score -= (over / lane_floor_ms as f64 * 6.0).min(lane_max_penalty);
        }
    }
    score += local_device_pressure_adjustment(p, policy, route_profile);
    if near_day_quota(p, 0.90) {
        score -= 20.0;
    }
    if p.requests_per_minute
        .filter(|max| *max > 0)
        .map(|max| (p.requests_used_minute as f64 / max as f64) >= 0.90)
        .unwrap_or(false)
    {
        score -= 12.0;
    }
    if let Some(max_params_b) = package_runtime.llmfit_local_max_params_b {
        if let Some(model_params_b) = parse_model_params_billions(&model.id) {
            if is_localish_provider(&p.id) {
                if model_params_b <= max_params_b + 0.75 {
                    score += 14.0;
                } else {
                    score -= ((model_params_b - max_params_b) * 6.0).min(18.0);
                }
            } else if is_edge_provider(&p.id) && model_params_b > max_params_b + 1.0 {
                score += 8.0;
            }
        }
    }
    if route_profile.execution_lane == "execution" {
        if package_runtime.nanoclaw_runtime_ready {
            if is_edge_provider(&p.id) || is_localish_provider(&p.id) {
                score += 6.0;
            }
        } else if matches!(
            package_runtime.nanoclaw_probe_state.as_str(),
            "runtime_blocked" | "auth_required" | "probe_error" | "error"
        ) {
            if is_localish_provider(&p.id) {
                score -= 8.0;
            }
            if is_edge_provider(&p.id) {
                score += 6.0;
            }
        }
    }
    // A2: Failure-aware multiplicative score decay. The linear `-10 *
    // consecutive_failures` above is easily canceled by lane bias / cost
    // bonuses for a "favored" provider, which kept a flaky upstream in the
    // pool too long. Apply a geometric multiplier on top: each consecutive
    // failure shrinks the score by ~12%, capped at 8 failures so we don't
    // collapse below a meaningful pick floor (cooldown handles the >=3
    // case from a different angle). Skips when score is already non-positive
    // — sign-flipping near zero would create routing oscillation.
    if score > 0.0 && p.consecutive_failures > 0 {
        let exp = (p.consecutive_failures as i32).min(8);
        score *= 0.88_f64.powi(exp);
    }
    score
}

fn effective_route_latency_ms(p: &ProviderState, model: &ModelState) -> Option<u64> {
    match (p.avg_latency_ms, model.avg_latency_ms) {
        (Some(provider_latency), Some(model_latency)) => Some(provider_latency.max(model_latency)),
        (Some(provider_latency), None) => Some(provider_latency),
        (None, Some(model_latency)) => Some(model_latency),
        (None, None) => None,
    }
}

fn model_cost_adjustment(model: &ModelState, policy: &HybridRoutePolicy) -> f64 {
    let Some(total_cost_per_million) = model_total_cost_per_million(model) else {
        return match policy.cost_tier.as_str() {
            "low" => -3.0,
            "high" => -1.0,
            _ => 0.0,
        };
    };

    match policy.cost_tier.as_str() {
        "low" => (12.0 - total_cost_per_million * 1.5).clamp(-35.0, 12.0),
        "high" => (total_cost_per_million * 0.5).clamp(0.0, 10.0),
        _ => 0.0,
    }
}

fn model_total_cost_per_million(model: &ModelState) -> Option<f64> {
    match (
        model.cost_per_million_tokens_in,
        model.cost_per_million_tokens_out,
    ) {
        (Some(input), Some(output)) => Some(input.max(0.0) + output.max(0.0)),
        (Some(input), None) => Some(input.max(0.0)),
        (None, Some(output)) => Some(output.max(0.0)),
        (None, None) => None,
    }
}

fn provider_lane_bias(
    p: &ProviderState,
    route_profile: &RouteExecutionProfile,
    priority: &str,
    policy: &HybridRoutePolicy,
) -> f64 {
    let mut score = 0.0;
    let preferred_local_surface = preferred_local_surface();
    match route_profile.execution_lane.as_str() {
        "orchestrator" => match policy.origin_preference.as_str() {
            "local" => match p.id.as_str() {
                "edge_backbone" => score += 48.0,
                "edge_worker_light" => score -= 8.0,
                "edge_laptop" => score -= 18.0,
                "edge_guardhouse" => score -= 18.0,
                _ if access_tier_class(p) == "free_cloud" => score -= 24.0,
                _ if access_tier_class(p) == "paid_cloud" => score -= 36.0,
                _ if access_tier_class(p) == "mixed" => score -= 18.0,
                _ if is_cloud_provider(&p.id) => score -= 22.0,
                _ => {}
            },
            "cloud" => match p.id.as_str() {
                "edge_backbone" => score += 4.0,
                "edge_worker_light" => score -= 16.0,
                "edge_laptop" => score -= 24.0,
                "edge_guardhouse" => score -= 24.0,
                _ if access_tier_class(p) == "free_cloud" => score += 42.0,
                _ if access_tier_class(p) == "paid_cloud" => score += 14.0,
                _ if is_cloud_provider(&p.id) => score += 36.0,
                _ => {}
            },
            _ => match p.id.as_str() {
                "edge_backbone" => score += 40.0,
                "edge_worker_light" => score -= 10.0,
                "edge_laptop" => score -= 20.0,
                "edge_guardhouse" => score -= 20.0,
                _ if access_tier_class(p) == "free_cloud" => score += 8.0,
                _ if access_tier_class(p) == "paid_cloud" => score -= 12.0,
                _ if access_tier_class(p) == "mixed" => score -= 8.0,
                _ if is_cloud_provider(&p.id) => score += 2.0,
                _ => {}
            },
        },
        "execution" => match p.id.as_str() {
            "edge_backbone" | "edge_backbone_coder" => score += 58.0,
            "edge_worker_light" => score += 2.0,
            "edge_laptop" => score -= 28.0,
            "edge_guardhouse" => score -= 36.0,
            "local_fallback" => score -= 12.0,
            _ => {}
        },
        "compression" => match access_tier_class(p).as_str() {
            "local" => score += 2.0,
            "free_cloud" => score += 18.0,
            "mixed" => score += 12.0,
            "paid_cloud" => score += 8.0,
            _ => {}
        },
        "background" => match p.id.as_str() {
            "edge_backbone" | "edge_backbone_coder" => score += 16.0,
            "edge_guardhouse" => score += 24.0,
            "edge_laptop" => score -= 6.0,
            "edge_worker_light" => score += 10.0,
            _ => {}
        },
        _ => match p.id.as_str() {
            "edge_backbone" | "edge_backbone_coder" => score += 28.0,
            "edge_worker_light" => score += 18.0,
            "edge_laptop" => score -= 18.0,
            "edge_guardhouse" => score -= 8.0,
            _ => {}
        },
    }

    match preferred_local_surface.as_str() {
        "mesh" => {
            if is_primary_local_surface_provider(&p.id) {
                score += 36.0;
            } else if is_direct_local_provider(&p.id) {
                score -= 16.0;
            } else if is_local_fallback(&p.id) {
                score -= 12.0;
            }
        }
        "llamacpp" => {
            // edge_backbone is the primary local surface and serves llama.cpp
            // directly, so it picks up the +36 mesh boost via the "mesh" arm
            // when that surface is preferred and stays neutral here.
        }
        _ => {}
    }

    if is_high_priority(priority)
        && matches!(p.id.as_str(), "edge_backbone" | "edge_backbone_coder")
    {
        score += 8.0;
    }
    if is_background_priority(priority)
        && matches!(p.id.as_str(), "edge_guardhouse" | "edge_laptop")
    {
        score += 8.0;
    }
    if p.id == "edge_laptop" {
        score -= 16.0;
    }
    if policy.origin_preference == "local" && is_edge_provider(&p.id) {
        score += 4.0;
    } else if policy.origin_preference == "cloud" && is_cloud_provider(&p.id) {
        score += 8.0;
    }
    if route_profile.execution_lane == "orchestrator" && access_tier_class(p) == "free_cloud" {
        score += free_cloud_pool_bias(p);
    }
    score
}

pub(super) fn local_device_pressure_adjustment(
    p: &ProviderState,
    policy: &HybridRoutePolicy,
    route_profile: &RouteExecutionProfile,
) -> f64 {
    if policy.require_local || !is_localish_provider(&p.id) {
        return 0.0;
    }

    let Some(pressure) = configured_local_device_pressure() else {
        return 0.0;
    };

    let lane_multiplier = match route_profile.execution_lane.as_str() {
        "execution" | "orchestrator" => 1.0,
        "compression" => 0.90,
        "background" => 0.55,
        "interactive" | "monitoring" => 0.45,
        _ => 0.65,
    };
    let origin_multiplier = if policy.origin_preference == "local" {
        0.35
    } else {
        1.0
    };

    let base_penalty = if pressure >= 0.90 {
        86.0
    } else if pressure >= 0.75 {
        58.0
    } else if pressure >= 0.55 {
        28.0
    } else if pressure >= 0.35 {
        10.0
    } else {
        0.0
    };

    -(base_penalty * lane_multiplier * origin_multiplier)
}

pub(super) fn configured_local_device_pressure() -> Option<f64> {
    [
        "ARDA_CHARON_LOCAL_DEVICE_PRESSURE",
        "ARDA_LOCAL_DEVICE_PRESSURE",
    ]
    .into_iter()
    .find_map(|key| {
        std::env::var(key)
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .map(|value| value.clamp(0.0, 1.0))
    })
}

fn health_probe_bias(p: &ProviderState, model: &ModelState) -> f64 {
    let mut bias = 0.0;
    if p.probe_model
        .as_deref()
        .is_some_and(|probe_model| model.id == probe_model || model.alias_matches(probe_model))
    {
        bias += 80.0;
    } else if p.probe_model.is_some() {
        bias -= 20.0;
    }

    if let Some(latency) = model.avg_latency_ms.or(p.avg_latency_ms) {
        if latency <= 750 {
            bias += 28.0;
        } else if latency <= 2_000 {
            bias += 14.0;
        } else if latency > 8_000 {
            bias -= 24.0;
        } else if latency > 4_000 {
            bias -= 12.0;
        }
    }

    if let Some(params_b) = parse_model_params_billions(&model.id) {
        if params_b <= 10.0 {
            bias += 18.0;
        } else if params_b >= 30.0 {
            bias -= 32.0;
        } else if params_b >= 14.0 {
            bias -= 14.0;
        }
    }

    let model_id = model.id.to_ascii_lowercase();
    for needle in ["flash", "instant", "nano", "mini", "haiku", "8b", "7b"] {
        if model_id.contains(needle) {
            bias += 10.0;
        }
    }
    for needle in [
        "reason", "thinking", "ultra", "large", "35b", "120b", "405b",
    ] {
        if model_id.contains(needle) {
            bias -= 16.0;
        }
    }
    bias
}

fn audit_stability_bias(p: &ProviderState, model: &ModelState) -> f64 {
    let mut bias = match access_tier_class(p).as_str() {
        "local" => 10.0,
        "free_cloud" => free_cloud_pool_bias(p) * 0.45,
        "mixed" => 6.0,
        "paid_cloud" => 12.0,
        _ => 0.0,
    };

    if quality_band_class(p) == "high" {
        bias += 8.0;
    }
    if model.context_window >= 128_000 {
        bias += 6.0;
    }
    if model_is_free(model) {
        bias += 3.0;
    }
    if p.avg_latency_ms.is_some_and(|lat| lat > 30_000) {
        bias -= 12.0;
    }
    if p.consecutive_failures > 0 {
        bias -= (p.consecutive_failures as f64 * 6.0).min(24.0);
    }

    bias
}

fn free_cloud_pool_bias(p: &ProviderState) -> f64 {
    let mut bias = 10.0;

    match quality_band_class(p).as_str() {
        "high" => bias += 10.0,
        "medium" => bias += 2.0,
        "low" => bias -= 8.0,
        _ => {}
    }
    if p.probe_model.is_some() {
        bias += 4.0;
    }
    if p.consecutive_successes >= 2 {
        bias += 5.0;
    }
    if p.consecutive_failures > 0 {
        bias -= (p.consecutive_failures as f64 * 8.0).min(32.0);
    }
    if p.error_count > 0 {
        bias -= (p.error_count as f64 * 0.8).min(16.0);
    }
    if near_day_quota(p, 0.80) {
        bias -= 18.0;
    }
    if p.requests_per_minute
        .filter(|max| *max > 0)
        .map(|max| (p.requests_used_minute as f64 / max as f64) >= 0.80)
        .unwrap_or(false)
    {
        bias -= 10.0;
    }

    match p.avg_latency_ms {
        Some(lat) if lat <= 1_500 => bias += 12.0,
        Some(lat) if lat <= 4_000 => bias += 8.0,
        Some(lat) if lat <= 10_000 => bias += 3.0,
        Some(lat) if lat > 25_000 => bias -= 10.0,
        Some(lat) if lat > 15_000 => bias -= 5.0,
        None => bias -= 2.0,
        _ => {}
    }

    let free_model_count = p.models.iter().filter(|model| model_is_free(model)).count();
    if free_model_count >= 3 {
        bias += 5.0;
    } else if free_model_count > 0 {
        bias += 2.0;
    }
    let healthy_model_count = p
        .models
        .iter()
        .filter(|model| model.healthy && !model.in_cooldown)
        .count();
    if healthy_model_count >= 3 {
        bias += 3.0;
    } else if healthy_model_count == 0 {
        bias -= 18.0;
    }

    bias
}

fn model_is_free(model: &ModelState) -> bool {
    let id = model.id.to_ascii_lowercase();
    id.ends_with(":free")
        || id.ends_with("-free")
        || id.contains("/free")
        || model_total_cost_per_million(model).is_some_and(|cost| cost <= 0.0)
}

fn provider_connection_penalty(id: &str, active_connections: u32) -> f64 {
    let active_connections = active_connections as f64;
    match id {
        "edge_backbone" => active_connections * 7.0,
        "edge_worker_light" => active_connections * 10.0,
        "edge_laptop" => active_connections * 7.0,
        "edge_guardhouse" => active_connections * 6.0,
        _ if is_local_fallback(id) => active_connections * 12.0,
        _ => active_connections * 6.0,
    }
}

pub(super) fn near_day_quota(p: &ProviderState, threshold: f64) -> bool {
    p.requests_per_day
        .filter(|max| *max > 0)
        .map(|max| (p.requests_used_day as f64 / max as f64) >= threshold)
        .unwrap_or(false)
}

pub(super) fn is_local_fallback(id: &str) -> bool {
    id == "local_fallback"
}

fn is_localish_provider(id: &str) -> bool {
    is_local_provider(id) || id == "litellm_gateway"
}

fn is_edge_provider(id: &str) -> bool {
    id.starts_with("edge_")
}

pub(super) fn is_primary_local_surface_provider(id: &str) -> bool {
    matches!(
        id,
        "edge_backbone" | "edge_backbone_coder" | "edge_backbone_long"
    )
}

pub(super) fn is_local_provider(id: &str) -> bool {
    is_primary_local_surface_provider(id) || is_direct_local_provider(id) || is_local_fallback(id)
}

pub(super) fn access_tier_class(p: &ProviderState) -> String {
    match p.access_tier.trim().to_ascii_lowercase().as_str() {
        "local" => "local".to_string(),
        "free_cloud" | "free" => "free_cloud".to_string(),
        "paid_cloud" | "paid" => "paid_cloud".to_string(),
        "mixed" | "hybrid" | "hybrid_cloud" => "mixed".to_string(),
        _ => "mixed".to_string(),
    }
}

fn quality_band_class(p: &ProviderState) -> String {
    match p.quality_band.trim().to_ascii_lowercase().as_str() {
        "high" => "high".to_string(),
        "low" => "low".to_string(),
        _ => "medium".to_string(),
    }
}

fn is_cloud_provider(id: &str) -> bool {
    !is_primary_local_surface_provider(id)
        && !is_direct_local_provider(id)
        && !matches!(id, "local_fallback" | "litellm_gateway")
}

fn is_direct_local_provider(id: &str) -> bool {
    id.starts_with("edge_") && !is_primary_local_surface_provider(id)
}

fn preferred_local_surface() -> String {
    std::env::var("ARDA_LOCAL_INFERENCE_SURFACE")
        .unwrap_or_else(|_| "hybrid".to_string())
        .trim()
        .to_ascii_lowercase()
}

pub(super) fn is_high_priority(priority: &str) -> bool {
    matches!(priority, "urgent" | "high" | "critical")
}

pub(super) fn is_background_priority(priority: &str) -> bool {
    matches!(priority, "background" | "low" | "deferred")
}

pub(super) fn parse_model_params_billions(model_id: &str) -> Option<f64> {
    let lowered = model_id.to_ascii_lowercase();
    for token in lowered.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.')) {
        if let Some(value) = token.strip_suffix('b') {
            if let Ok(parsed) = value.parse::<f64>() {
                return Some(parsed);
            }
        }
    }
    None
}