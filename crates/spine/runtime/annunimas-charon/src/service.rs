// sigil: REPAIR
use crate::types::{CharonRequestEnvelope, ProviderState, RouteDecision};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::task::{JouleWorkMeasurementSource, Task};
use annunimas_governance::{load_governance_chain, record_bacon_lite, GovernanceChainConfig};
use annunimas_mnemosyne::MnemosyneService;
use annunimas_plutus::JouleWorkUnit;
use chrono::Utc;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant as StdInstant};
use tokio::sync::RwLock;

mod adaptive_routing;
mod agent_quotas;
mod bandit;
mod bootstrap;
mod bootstrap_defaults;
mod bootstrap_overlay;
mod bootstrap_runtime;
mod capabilities;
mod catalog_reconciliation;
mod codex_responses_driver;
mod echo_gate;
mod event_writer;
mod health_probe;
mod hermes_cli_driver;
mod hermes_proxy_driver;
mod http_clients;
mod metrics;
mod observability;
mod paths;
mod provider_admin;
mod proxy;
mod route_candidate_cache;
mod route_policy;
mod route_scoring;
mod route_selection;
mod route_sessions;
mod routing;
mod runtime_state;
mod service_events;
mod state_io;
mod state_mutation;
mod status;
use bootstrap::{
    default_bootstrap_state_path, default_provider_config_path, load_providers_from_config,
};
use bootstrap_defaults::default_providers;
use http_clients::HttpClientKey;
use route_policy::{
    build_route_decision_with_governance_chain, decay_lane_fitness_snapshot,
    derive_route_execution_profile, evaluate_route_governance_chain, excluded_provider_ids,
    is_high_priority, is_local_provider, merge_latency, model_supports_request, near_day_quota,
    provider_in_half_open, resolve_hybrid_route_policy, LaneFitnessSnapshot,
};
#[cfg(test)]
pub(crate) use routing::normalize_openai_request_payload;
pub(crate) use routing::{
    attach_charon_route_metadata, is_billing_or_credit_error, is_client_payload_error,
    is_context_overflow_error, is_reasoning_replay_required_error, is_request_scoped_retry_error,
    local_payload_requires_structured_tool_history, model_error_should_mark_unavailable,
    normalize_openai_request_payload_with_policy, provider_error_immediate_cooldown_seconds,
    provider_error_should_fallback, proxy_max_attempts, slim_local_attempt_body,
    strip_internal_openai_routing_fields, transport_failure_should_trigger_cooldown,
};
// proxy:: helpers re-exported only for tests that reach in via super::*.
// Production code paths inside the proxy module use them directly.
use echo_gate::{evaluate_pre_route_governance_with_options, GateAction};
pub use proxy::StreamingProxyOutcome;
#[cfg(test)]
pub(crate) use proxy::{
    provider_has_alternate_routable_model, proxy_timeout_for_provider, strip_optional_tool_payload,
};
pub use route_sessions::RouteHistoryEntry;
use route_sessions::{route_history_limit, StickyRouteSession};
use routing::normalize_openai_response;
use runtime_state::{
    merge_persisted_runtime_state, persist_runtime_state_snapshot, provider_unavailable_reason,
    refresh_provider_windows,
};
#[cfg(test)]
pub(crate) use state_io::append_jsonl;
pub(crate) use state_io::{
    count_malformed_jsonl, default_root, is_permission_error, read_recent_jsonl,
    runtime_build_cache_autorun_enabled, runtime_build_cache_command_args,
    runtime_build_cache_command_program, runtime_build_cache_state_path, touch,
};
pub(crate) use status::classify_provider_operational_state;
pub use status::CharonStatus;
use status::{build_budget_alerts, build_budget_pressure_summary, PackageRuntimeSignals};

pub(crate) use hermes_cli_driver::hermes_cli_readiness_summary;
pub(crate) use hermes_proxy_driver::hermes_proxy_base_url;

fn load_route_governance_chain() -> GovernanceChainConfig {
    let path = paths::annunimas_root().join("config/governance/chains.toml");
    load_governance_chain(&path).unwrap_or_else(|err| {
        tracing::debug!(
            error = %err,
            path = %path.display(),
            "CHARON governance chain config load failed; using default triad"
        );
        GovernanceChainConfig::default_triad()
    })
}

fn route_governance_task(req: &CharonRequestEnvelope) -> Task {
    let request_text = req
        .messages
        .iter()
        .filter_map(|message| message.get("content").and_then(|value| value.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    let description = if request_text.trim().is_empty() {
        format!("route {} {}", req.agent_id, req.task_type)
    } else {
        format!(
            "route request for agent={} task_type={} prompt={}",
            req.agent_id, req.task_type, request_text
        )
    };
    let mut task = Task::new(description, "dispatch");
    task.assigned_agent = Some("charon".to_string());
    task.clarifications_resolved = if !req.priority.is_empty() { 1 } else { 0 };
    task.joule_cost_estimated = 1.0;
    task.joule_cost_actual = 1.0;
    task.joulework_measurement_source = JouleWorkMeasurementSource::OperatorEstimate;
    task.joulework_measurement_confidence = 0.55;
    task
}

#[derive(Clone)]
pub struct CharonService {
    root: PathBuf,
    state_path: PathBuf,
    governance_events_path: PathBuf,
    tool_fit_ledger_path: PathBuf,
    provider_capability_receipts_path: PathBuf,
    socket_path: PathBuf,
    config_path: PathBuf,
    bootstrap_state_path: PathBuf,
    providers: Arc<RwLock<Vec<ProviderState>>>,
    capacity_probe_cache: Arc<RwLock<BTreeMap<String, ProviderCapacityProbeRecord>>>,
    mnemosyne: Option<MnemosyneService>,
    metrics: Arc<metrics::CharonMetrics>,
    http_clients: Arc<RwLock<std::collections::HashMap<HttpClientKey, Arc<reqwest::Client>>>>,
    event_writer: event_writer::EventWriter,
    route_history: Arc<RwLock<VecDeque<RouteHistoryEntry>>>,
    sticky_sessions: Arc<RwLock<BTreeMap<String, StickyRouteSession>>>,
    route_candidate_cache: Arc<route_candidate_cache::RouteCandidateCache>,
    agent_quota_windows: Arc<agent_quotas::AgentQuotaWindows>,
    bandit: Arc<bandit::BanditStore>,
}

impl CharonService {
    pub fn with_socket_path(mut self, socket_path: impl AsRef<Path>) -> Self {
        self.socket_path = socket_path.as_ref().to_path_buf();
        self
    }

    pub(crate) fn metrics(&self) -> &metrics::CharonMetrics {
        &self.metrics
    }

    /// Acquire a read guard on the providers list. Used by the active health
    /// probe loop (D1) to take a quick snapshot.
    pub(crate) async fn providers_read(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, Vec<ProviderState>> {
        self.providers.read().await
    }

    /// Spawn the in-process active health probe loop (D1). Idempotent on
    /// service-level caller responsibility — callers should invoke once at
    /// daemon start.
    pub fn spawn_health_probe(&self) {
        health_probe::spawn(self.clone());
    }

    /// Spawn the provider catalog reconciliation loop. The loop is delayed so
    /// daemon startup remains cheap; operators can run the same job immediately
    /// via `/reconcile_catalogs`.
    pub fn spawn_catalog_reconciliation(&self) {
        catalog_reconciliation::spawn(self.clone());
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderCapacityProbeRecord {
    pub provider_id: String,
    pub source: String,
    pub state: String,
    pub reason: String,
    pub blocked: bool,
    pub checked_at_utc: String,
    pub next_refresh_at_utc: String,
    pub meta: JsonValue,
}

impl CharonService {
    pub fn from_default_or_fallback() -> Result<Self> {
        let primary = default_root();
        match Self::new(&primary) {
            Ok(v) => Ok(v),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                Self::new(paths::annunimas_root().join("data").join("charon"))
            }
        }
    }

    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let state_path = root.join("state.jsonl");
        let governance_events_path = root.join("governance_events.jsonl");
        let tool_fit_ledger_path = root.join("tool_fit_ledger.jsonl");
        let provider_capability_receipts_path = root.join("provider_capability_receipts.json");
        let socket_path = root.join("charon.sock");
        let config_path = default_provider_config_path();
        let bootstrap_state_path = default_bootstrap_state_path();
        touch(&state_path)?;
        touch(&governance_events_path)?;
        touch(&tool_fit_ledger_path)?;
        let providers = match load_providers_from_config(&config_path, &bootstrap_state_path) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, path = %config_path.display(), "CHARON provider config load failed, using defaults");
                default_providers()
            }
        };
        let provider_runtime_state_path = root.join("provider_runtime_state.json");
        let providers = match merge_persisted_runtime_state(&provider_runtime_state_path, providers)
        {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, path = %provider_runtime_state_path.display(), "CHARON provider runtime state load failed; continuing from config state");
                match load_providers_from_config(&config_path, &bootstrap_state_path) {
                    Ok(v) => v,
                    Err(_) => default_providers(),
                }
            }
        };
        if let Err(err) = persist_runtime_state_snapshot(&provider_runtime_state_path, &providers) {
            tracing::warn!(error = %err, path = %provider_runtime_state_path.display(), "CHARON provider runtime state persist failed after config merge");
        }
        let event_writer =
            event_writer::EventWriter::new(state_path.clone(), governance_events_path.clone());
        let bandit_path = root.join("bandit.json");
        let service = Self {
            root,
            state_path,
            governance_events_path,
            tool_fit_ledger_path,
            provider_capability_receipts_path,
            socket_path,
            config_path,
            bootstrap_state_path,
            providers: Arc::new(RwLock::new(providers)),
            capacity_probe_cache: Arc::new(RwLock::new(BTreeMap::new())),
            mnemosyne: MnemosyneService::from_default_or_fallback().ok(),
            metrics: Arc::new(metrics::CharonMetrics::new()),
            http_clients: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_writer,
            route_history: Arc::new(RwLock::new(VecDeque::with_capacity(route_history_limit()))),
            sticky_sessions: Arc::new(RwLock::new(BTreeMap::new())),
            route_candidate_cache: Arc::new(route_candidate_cache::RouteCandidateCache::new()),
            agent_quota_windows: Arc::new(agent_quotas::AgentQuotaWindows::new()),
            bandit: Arc::new(bandit::BanditStore::new(bandit_path)),
        };
        Ok(service)
    }

    pub(crate) async fn persist_provider_runtime_state(&self) -> Result<()> {
        let providers = self.providers.read().await.clone();
        persist_runtime_state_snapshot(&self.provider_runtime_state_path(), &providers)
    }

    pub(crate) fn persist_provider_runtime_state_snapshot(
        &self,
        providers: &[ProviderState],
    ) -> Result<()> {
        persist_runtime_state_snapshot(&self.provider_runtime_state_path(), providers)
    }

    pub async fn state(&self) -> Result<serde_json::Value> {
        let mut providers = self.providers.write().await;
        refresh_provider_windows(&mut providers, Utc::now());
        let package_runtime = self.read_package_runtime_signals();
        let build_cache = self.read_runtime_build_cache_signals();
        let budget_pressure = build_budget_pressure_summary(&providers);
        let alerts = build_budget_alerts(&budget_pressure);
        Ok(serde_json::json!({
            "charon_version": "0.1.0",
            "timestamp_utc": Utc::now().to_rfc3339(),
            "providers": providers.clone(),
            "budget_pressure": budget_pressure,
            "alerts": alerts,
            "package_runtime_signals": package_runtime,
            "runtime_build_cache": build_cache,
        }))
    }

    pub async fn providers(&self) -> Vec<ProviderState> {
        let mut providers = self.providers.write().await;
        refresh_provider_windows(&mut providers, Utc::now());
        providers.clone()
    }

    pub(crate) async fn capacity_probe_record(
        &self,
        provider_id: &str,
    ) -> Option<ProviderCapacityProbeRecord> {
        self.capacity_probe_cache
            .read()
            .await
            .get(provider_id)
            .cloned()
    }

    pub fn recent_state_events(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.state_path, limit)
    }

    pub fn recent_governance_events(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.governance_events_path, limit)
    }

    pub async fn route_preview(&self, req: CharonRequestEnvelope) -> Result<RouteDecision> {
        let governance_task = route_governance_task(&req);
        let governance_chain = load_route_governance_chain();
        let chain_result =
            evaluate_route_governance_chain(&governance_task, &req.options, &governance_chain);
        let mut providers = self.providers.write().await;
        let now = Utc::now();
        refresh_provider_windows(&mut providers, now);
        let priority = req.priority.to_ascii_lowercase();
        let strict = req
            .options
            .get("strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut forced_provider_id = req
            .options
            .get("force_provider_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let mut forced_model_id = req
            .options
            .get("force_model_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        if forced_provider_id.is_none() && forced_model_id.is_none() {
            if let Some((provider_id, model_id)) = self.sticky_route_override(&req).await {
                forced_provider_id = Some(provider_id);
                forced_model_id = Some(model_id);
            }
        }
        let excluded_provider_ids = excluded_provider_ids(&req.options);
        let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);
        let route_profile = derive_route_execution_profile(&req, &priority);
        let package_runtime = self.read_package_runtime_signals();

        let candidate = self.select_route_candidate(
            &providers,
            &req,
            &priority,
            strict,
            forced_provider_id.as_deref(),
            forced_model_id.as_deref(),
            &excluded_provider_ids,
            &policy,
            &route_profile,
            &package_runtime,
        )?;

        Ok(build_route_decision_with_governance_chain(
            &providers[candidate.provider_index],
            candidate.model,
            candidate.score,
            &req,
            &priority,
            strict,
            &policy,
            &route_profile,
            &governance_task,
            chain_result,
        ))
    }

    pub async fn route(&self, req: CharonRequestEnvelope) -> Result<RouteDecision> {
        self.route_and_resolve(req)
            .await
            .map(|(decision, _)| decision)
    }

    /// Like `route()` but also returns a snapshot of the resolved provider.
    /// Saves a `providers.read().await` round-trip in the proxy retry loops
    /// (B1 in OPTIMIZATION_PLAN.md) where every attempt previously did
    /// `route()` then `providers.read()` to look up the matched provider's
    /// connection metadata.
    pub async fn route_and_resolve(
        &self,
        req: CharonRequestEnvelope,
    ) -> Result<(RouteDecision, ProviderState)> {
        // C2: per-route correlation ID. 16 hex chars from rand — uuid would be
        // overkill and adds a dep. Surfaces in `route_selected` events and as
        // the `x-charon-route-id` HTTP response header on proxy paths so an
        // operator can trace one user request gateway → charon → upstream.
        let route_id = format!("{:016x}", rand::random::<u64>());
        let bacon_task = route_governance_task(&req);
        let governance_chain = load_route_governance_chain();
        let chain_result =
            evaluate_route_governance_chain(&bacon_task, &req.options, &governance_chain);
        let mut providers = self.providers.write().await;
        let now = Utc::now();
        refresh_provider_windows(&mut providers, now);
        let priority = req.priority.to_ascii_lowercase();
        let strict = req
            .options
            .get("strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let forced_provider_id = req
            .options
            .get("force_provider_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let forced_model_id = req
            .options
            .get("force_model_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let excluded_provider_ids = excluded_provider_ids(&req.options);
        let policy = resolve_hybrid_route_policy(&req.task_type, &req.options);

        let route_profile = derive_route_execution_profile(&req, &priority);
        let package_runtime = self.read_package_runtime_signals();
        let candidate = match self.select_route_candidate(
            &providers,
            &req,
            &priority,
            strict,
            forced_provider_id.as_deref(),
            forced_model_id.as_deref(),
            &excluded_provider_ids,
            &policy,
            &route_profile,
            &package_runtime,
        ) {
            Ok(candidate) => candidate,
            Err(err) => {
                self.append_state_event(
                    "route_failed",
                    serde_json::json!({
                        "task_type": req.task_type,
                        "agent_id": req.agent_id,
                        "priority": priority,
                        "policy": policy,
                        "strict": strict,
                        "forced_provider_id": forced_provider_id,
                        "unavailable": providers.iter().map(|p| {
                            provider_unavailable_reason(p, &priority, strict, now)
                                .unwrap_or_else(|| serde_json::json!({"provider_id": p.id, "reason": "filtered"}))
                        }).collect::<Vec<_>>(),
                    }),
                )?;
                self.append_governance_event(
                    "route_failed",
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "policy": policy,
                        "strict": strict,
                        "forced_provider_id": forced_provider_id,
                        "forced_model_id": forced_model_id,
                        "verdict": "failed_no_route",
                        "failure_class": "route_unavailable",
                        "unavailable": providers.iter().map(|p| {
                            provider_unavailable_reason(p, &priority, strict, now)
                                .unwrap_or_else(|| serde_json::json!({"provider_id": p.id, "reason": "filtered"}))
                        }).collect::<Vec<_>>(),
                    }),
                )?;
                self.emit_work_signal_background(
                    "charon",
                    0.2,
                    JouleWorkUnit::Reasoning,
                    Some(format!("route_failed:{}:{}", req.agent_id, req.task_type)),
                );
                self.emit_memory_event(
                    "route_failed",
                    &format!(
                        "CHARON failed route for {}:{} priority={}",
                        req.agent_id, req.task_type, priority
                    ),
                    Some(0.35),
                    vec![
                        "charon".to_string(),
                        "route".to_string(),
                        "failure".to_string(),
                    ],
                );
                if let Err(record_err) = record_bacon_lite(
                    "charon",
                    "route_failed",
                    &bacon_task,
                    serde_json::json!({
                        "agent_id": req.agent_id,
                        "task_type": req.task_type,
                        "priority": priority,
                        "policy": policy,
                    }),
                ) {
                    tracing::debug!(error = %record_err, "CHARON bacon-lite route_failed record failed");
                }
                return Err(err);
            }
        };

        // Hold the write lock only long enough to mutate provider state and
        // build the decision. Drop it before recording events / memory /
        // metrics so concurrent routes don't serialize behind the disk-bound
        // bookkeeping. (B1: previously the lock was held through the
        // emit_*_event / append_* calls — even with the new async event
        // writer, we'd still have been holding the providers lock through
        // the bacon-lite recorder and mnemosyne emits.)
        let pick_score = candidate.score;
        let rejected = route_selection::route_rejection_records(
            &providers,
            &req,
            &priority,
            strict,
            forced_model_id.as_deref(),
            &excluded_provider_ids,
            &route_profile,
            self,
        )
        .into_iter()
        .filter(|record| record.provider_id != providers[candidate.provider_index].id)
        .collect::<Vec<_>>();
        let route_explanation = adaptive_routing::build_route_explanation(
            &route_id,
            &providers,
            &candidate,
            &req,
            &priority,
            &policy,
            &route_profile,
            rejected,
        );
        let (decision, resolved_provider) = {
            let provider = &mut providers[candidate.provider_index];
            if provider.requests_used_minute == 0 {
                provider.minute_window_started_utc = Some(now.to_rfc3339());
            }
            if provider.requests_used_day == 0 {
                provider.day_window_started_utc = Some(now.to_rfc3339());
            }
            self.record_bandit_route(&req, &provider.id, &candidate.model.id);
            provider.requests_used_minute += 1;
            provider.requests_used_day += 1;
            let was_half_open = provider_in_half_open(provider);
            if !was_half_open {
                provider.consecutive_successes += 1;
                provider.consecutive_failures = 0;
                provider.last_error = None;
            }
            provider.active_connections += 1;
            provider.last_reservation_utc = Some(now.to_rfc3339());
            self.reserve_agent_quota(provider, &req);
            let mut decision = build_route_decision_with_governance_chain(
                provider,
                candidate.model,
                candidate.score,
                &req,
                &priority,
                strict,
                &policy,
                &route_profile,
                &bacon_task,
                chain_result.clone(),
            );
            decision.route_id = route_id.clone();
            let resolved = provider.clone();
            (decision, resolved)
        };
        // From here on we no longer touch the providers vector — drop the
        // write guard so concurrent routers can proceed.
        drop(providers);

        {
            if let Err(err) = record_bacon_lite(
                "charon",
                "route_selected",
                &bacon_task,
                serde_json::json!({
                    "agent_id": req.agent_id,
                    "task_type": req.task_type,
                    "priority": priority,
                    "policy": policy,
                    "package_runtime": {
                        "llmfit_backend": package_runtime.llmfit_backend,
                        "llmfit_recommendation_count": package_runtime.llmfit_recommendation_count,
                        "nanoclaw_runtime_ready": package_runtime.nanoclaw_runtime_ready,
                        "nanoclaw_probe_state": package_runtime.nanoclaw_probe_state,
                    },
                    "route_profile": {
                        "route_class": route_profile.route_class,
                        "execution_lane": route_profile.execution_lane,
                        "context_window_target": route_profile.context_window_target,
                    },
                    "governance_chain": {
                        "chain_id": chain_result.chain_id,
                        "chain_version": chain_result.chain_version,
                        "profile_source": chain_result.profile_source,
                        "review_mode": chain_result.review_mode,
                        "profile_maturity": chain_result.profile_maturity,
                        "passed": chain_result.passed,
                        "veto_reason": chain_result.veto_reason,
                        "autonomous_blocking_enabled": chain_result.autonomous_blocking_enabled,
                    },
                    "provider_id": decision.provider_id,
                    "model_id": decision.model_id,
                }),
            ) {
                tracing::debug!(error = %err, "CHARON bacon-lite route_selected record failed");
            }
            self.append_state_event(
                "route_selected",
                serde_json::json!({
                    "decision": decision,
                    "explanation": route_explanation,
                }),
            )?;
            self.append_state_event(
                "route_explanation",
                serde_json::to_value(&route_explanation)?,
            )?;
            self.append_governance_event(
                "route_selected",
                serde_json::json!({
                    "agent_id": req.agent_id,
                    "task_type": req.task_type,
                    "priority": priority,
                    "policy": policy,
                    "strict": strict,
                    "verdict": "selected",
                    "provider_id": decision.provider_id,
                    "model_id": decision.model_id,
                    "route_class": decision.route_class,
                    "execution_lane": decision.execution_lane,
                    "governance": decision.governance,
                }),
            )?;
            self.emit_work_signal_background(
                "charon",
                (candidate.score / 100.0).clamp(0.2, 1.0),
                JouleWorkUnit::Reasoning,
                Some(format!("route:{}:{}", req.agent_id, req.task_type)),
            );
            self.emit_relationship_signal_background(
                &req.agent_id,
                &decision.provider_id,
                &decision.governance.love_equation_guard,
            );
            self.emit_memory_event(
                "route_selected",
                &format!(
                    "CHARON routed {}:{} [{}:{}] -> {}:{} triad_passed={} love_eq={:.2}",
                    req.agent_id,
                    req.task_type,
                    decision.execution_lane,
                    decision.route_class,
                    decision.provider_id,
                    decision.model_id,
                    decision.governance.triad_passed,
                    decision.governance.love_equation_guard.score
                ),
                Some((candidate.score / 100.0).clamp(0.0, 1.0)),
                vec![
                    "charon".to_string(),
                    "route".to_string(),
                    decision.execution_lane.clone(),
                    if decision.governance.triad_passed {
                        "triad_passed".to_string()
                    } else {
                        "triad_failed".to_string()
                    },
                ],
            );
            self.metrics.observe_route_pick(
                &decision.provider_id,
                &decision.model_id,
                &req.task_type,
                &decision.execution_lane,
                pick_score,
            );
            self.record_route_history(RouteHistoryEntry {
                ts_utc: Utc::now().to_rfc3339(),
                route_id: decision.route_id.clone(),
                agent_id: req.agent_id.clone(),
                task_type: req.task_type.clone(),
                priority: priority.clone(),
                provider_id: decision.provider_id.clone(),
                model_id: decision.model_id.clone(),
                route_class: decision.route_class.clone(),
                execution_lane: decision.execution_lane.clone(),
                score: pick_score,
                explanation: Some(route_explanation.clone()),
            })
            .await;
            self.update_sticky_route_session(&req, &decision).await;
            Ok((decision, resolved_provider))
        }
    }

    pub fn paths(&self) -> serde_json::Value {
        serde_json::json!({
            "root": self.root,
            "state_path": self.state_path,
            "governance_events_path": self.governance_events_path,
            "tool_fit_ledger_path": self.tool_fit_ledger_path,
            "socket_path": self.socket_path,
            "config_path": self.config_path,
            "bootstrap_state_path": self.bootstrap_state_path,
            "lane_fitness_path": self.lane_fitness_path(),
        })
    }
}

fn classify_models_probe_status(
    provider_id: &str,
    status: u16,
    raw_text: &str,
    model_count: Option<usize>,
) -> (String, String, bool, i64) {
    let lowered = raw_text.to_ascii_lowercase();
    match status {
        200 => (
            "ready".to_string(),
            format!(
                "{provider_id} model catalog reachable{}",
                model_count
                    .map(|count| format!(" ({count} models visible)"))
                    .unwrap_or_default()
            ),
            false,
            10,
        ),
        401..=403
            if [
                "insufficient balance",
                "insufficient credits",
                "creditserror",
                "billing",
                "out of credits",
                "requires more credits",
            ]
            .iter()
            .any(|needle| lowered.contains(needle)) =>
        {
            (
                "spend_blocked".to_string(),
                format!("{provider_id} credits or billing are exhausted"),
                true,
                30,
            )
        }
        401 | 403 => (
            "auth_failed".to_string(),
            format!("{provider_id} models probe was unauthorized"),
            true,
            15,
        ),
        429 => (
            "rate_limited".to_string(),
            format!("{provider_id} models probe hit rate limits"),
            true,
            5,
        ),
        404 | 405 => (
            "probe_error".to_string(),
            format!("{provider_id} does not expose a usable /models probe on this surface"),
            false,
            30,
        ),
        _ => (
            "probe_error".to_string(),
            format!("{provider_id} models probe returned HTTP {status}"),
            false,
            5,
        ),
    }
}

impl CharonService {}

fn command_output_with_timeout(
    command: &mut Command,
    timeout: StdDuration,
) -> std::io::Result<std::process::Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let start = StdInstant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut stream) = child.stdout.take() {
                let _ = std::io::Read::read_to_end(&mut stream, &mut stdout);
            }
            if let Some(mut stream) = child.stderr.take() {
                let _ = std::io::Read::read_to_end(&mut stream, &mut stderr);
            }
            return Ok(std::process::Output {
                status,
                stdout,
                stderr,
            });
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("command timed out after {}ms", timeout.as_millis()),
            ));
        }

        std::thread::sleep(StdDuration::from_millis(50));
    }
}

#[cfg(test)]
mod tests;
