use super::route_policy::{
    candidate_models_for_provider_request, excluded_model_ids, model_supports_request,
    provider_eligible, provider_score, provider_supports_request,
    provider_supports_request_capabilities, HybridRoutePolicy, RouteExecutionProfile,
    RouteSelectionCandidate,
};
use super::{CharonService, PackageRuntimeSignals};
use crate::types::{CharonRequestEnvelope, ProviderState};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration as StdDuration, Instant as StdInstant};

#[derive(Debug, Default)]
pub(super) struct RouteCandidateCache {
    entries: Mutex<BTreeMap<RouteCandidateCacheKey, RouteCandidateCacheEntry>>,
}

impl RouteCandidateCache {
    pub(super) fn new() -> Self {
        Self::default()
    }

    fn get(&self, key: &RouteCandidateCacheKey) -> Option<RouteCandidateCacheEntry> {
        let now = StdInstant::now();
        let mut entries = self.entries.lock().ok()?;
        entries.retain(|_, entry| entry.expires_at > now);
        entries.get(key).cloned()
    }

    fn insert(
        &self,
        key: RouteCandidateCacheKey,
        candidates: &[RouteSelectionCandidate],
        providers: &[ProviderState],
    ) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        let now = StdInstant::now();
        entries.retain(|_, entry| entry.expires_at > now);
        entries.insert(
            key,
            RouteCandidateCacheEntry {
                expires_at: now + route_candidate_cache_ttl(),
                candidates: candidates
                    .iter()
                    .map(|candidate| CachedRouteSelectionCandidate {
                        provider_id: providers[candidate.provider_index].id.clone(),
                        model_id: candidate.model.id.clone(),
                        score: candidate.score,
                    })
                    .collect(),
            },
        );
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct RouteCandidateCacheKey {
    task_type: String,
    priority: String,
    strict: bool,
    route_class: String,
    execution_lane: String,
    context_window_target: usize,
    forced_provider_id: Option<String>,
    forced_model_id: Option<String>,
    options_hash: u64,
}

#[derive(Debug, Clone)]
struct RouteCandidateCacheEntry {
    expires_at: StdInstant,
    candidates: Vec<CachedRouteSelectionCandidate>,
}

#[derive(Debug, Clone)]
struct CachedRouteSelectionCandidate {
    provider_id: String,
    model_id: String,
    score: f64,
}

fn route_candidate_cache_ttl() -> StdDuration {
    let millis = std::env::var("ARDA_CHARON_ROUTE_CANDIDATE_CACHE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(500);
    StdDuration::from_millis(millis.min(5_000))
}

fn route_options_hash(options: &serde_json::Value) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_string(options)
        .unwrap_or_else(|_| "null".to_string())
        .hash(&mut hasher);
    hasher.finish()
}

impl CharonService {
    fn route_candidate_cache_key(
        req: &CharonRequestEnvelope,
        priority: &str,
        strict: bool,
        forced_provider_id: Option<&str>,
        forced_model_id: Option<&str>,
        route_profile: &RouteExecutionProfile,
    ) -> RouteCandidateCacheKey {
        RouteCandidateCacheKey {
            task_type: req.task_type.clone(),
            priority: priority.to_string(),
            strict,
            route_class: route_profile.route_class.clone(),
            execution_lane: route_profile.execution_lane.clone(),
            context_window_target: route_profile.context_window_target,
            forced_provider_id: forced_provider_id.map(str::to_string),
            forced_model_id: forced_model_id.map(str::to_string),
            options_hash: route_options_hash(&req.options),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn cached_route_candidates(
        &self,
        providers: &[ProviderState],
        req: &CharonRequestEnvelope,
        priority: &str,
        strict: bool,
        forced_provider_id: Option<&str>,
        forced_model_id: Option<&str>,
        excluded_provider_ids: &[String],
        route_profile: &RouteExecutionProfile,
    ) -> Option<Vec<RouteSelectionCandidate>> {
        let key = Self::route_candidate_cache_key(
            req,
            priority,
            strict,
            forced_provider_id,
            forced_model_id,
            route_profile,
        );
        let cached = self.route_candidate_cache.get(&key)?;
        let excluded_model_ids = excluded_model_ids(&req.options);

        let mut candidates = Vec::with_capacity(cached.candidates.len());
        for cached_candidate in cached.candidates {
            let Some((provider_index, provider)) = providers
                .iter()
                .enumerate()
                .find(|(_, provider)| provider.id == cached_candidate.provider_id)
            else {
                continue;
            };
            if !provider_eligible(provider, priority, strict) {
                continue;
            }
            if !self.provider_agent_quota_available(provider, req) {
                continue;
            }
            if forced_provider_id
                .map(|forced| provider.id != forced)
                .unwrap_or(false)
            {
                continue;
            }
            if excluded_provider_ids
                .iter()
                .any(|excluded| excluded == &provider.id)
            {
                continue;
            }
            if forced_provider_id.is_none()
                && (!provider_supports_request(provider, req)
                    || !provider_supports_request_capabilities(provider, req))
            {
                continue;
            }
            let Some(model) = provider.models.iter().find(|model| {
                model.id == cached_candidate.model_id
                    && model.healthy
                    && !model.in_cooldown
                    && forced_model_id
                        .map(|forced| model.id == forced)
                        .unwrap_or(true)
                    && !excluded_model_ids
                        .iter()
                        .any(|excluded| model.id == *excluded || model.alias_matches(excluded))
                    && model_supports_request(&provider.id, model, Some(req))
            }) else {
                continue;
            };
            candidates.push(RouteSelectionCandidate {
                provider_index,
                model: model.clone(),
                score: cached_candidate.score,
            });
        }

        if candidates.is_empty() {
            None
        } else {
            Some(candidates)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn cache_route_candidates(
        &self,
        providers: &[ProviderState],
        req: &CharonRequestEnvelope,
        priority: &str,
        strict: bool,
        forced_provider_id: Option<&str>,
        forced_model_id: Option<&str>,
        route_profile: &RouteExecutionProfile,
        candidates: &[RouteSelectionCandidate],
    ) {
        let key = Self::route_candidate_cache_key(
            req,
            priority,
            strict,
            forced_provider_id,
            forced_model_id,
            route_profile,
        );
        self.route_candidate_cache
            .insert(key, candidates, providers);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_scored_route_candidates(
        &self,
        providers: &[ProviderState],
        req: &CharonRequestEnvelope,
        priority: &str,
        strict: bool,
        forced_provider_id: Option<&str>,
        forced_model_id: Option<&str>,
        excluded_provider_ids: &[String],
        policy: &HybridRoutePolicy,
        route_profile: &RouteExecutionProfile,
        package_runtime: &PackageRuntimeSignals,
    ) -> Vec<RouteSelectionCandidate> {
        let lane_fitness = self.read_lane_fitness_snapshot();
        let mut candidates = Vec::new();
        for (idx, p) in providers
            .iter()
            .enumerate()
            .filter(|(_, p)| provider_eligible(p, priority, strict))
            .filter(|(_, p)| self.provider_agent_quota_available(p, req))
            .filter(|(_, p)| {
                forced_provider_id
                    .map(|forced| p.id == forced)
                    .unwrap_or_else(|| provider_supports_request(p, req))
            })
            .filter(|(_, p)| {
                forced_provider_id
                    .map(|forced| p.id == forced)
                    .unwrap_or(true)
            })
            .filter(|(_, p)| {
                !excluded_provider_ids
                    .iter()
                    .any(|excluded| excluded == &p.id)
            })
            .filter(|(_, p)| {
                forced_provider_id
                    .map(|forced| p.id == forced)
                    .unwrap_or_else(|| provider_supports_request_capabilities(p, req))
            })
        {
            for model in
                candidate_models_for_provider_request(p, &req.task_type, forced_model_id, Some(req))
            {
                let score = provider_score(
                    p,
                    &model,
                    priority,
                    policy,
                    route_profile,
                    package_runtime,
                    &lane_fitness,
                ) + self.bandit_score_bonus(req, &p.id, &model.id);
                candidates.push(RouteSelectionCandidate {
                    provider_index: idx,
                    model,
                    score,
                });
            }
        }
        candidates
    }
}
