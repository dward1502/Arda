use super::{load_providers_from_config, refresh_provider_windows, CharonService};
use crate::adaptive::service::runtime_state::{failure_backoff_seconds, merge_runtime_state};
use crate::adaptive::service::state_io::append_jsonl;
use crate::adaptive::types::RouteDecision;
use arda_core::error::Result;
use arda_core::machine_sigil_or_default;
use arda_economics::JouleWorkUnit;
use chrono::{Duration, Utc};
use fs2::FileExt;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ToolFitObservation {
    pub ts_utc: String,
    pub provider_id: String,
    pub model_id: String,
    pub task_type: String,
    pub agent_id: String,
    pub route_class: String,
    pub execution_lane: String,
    pub route_id: String,
    pub tool_request: bool,
    pub tool_choice: Option<String>,
    pub tool_schema_count: usize,
    pub tool_history_present: bool,
    pub structured_output_request: bool,
    pub streaming_request: bool,
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub status_code: Option<u16>,
    pub outcome_class: String,
    pub error: Option<String>,
}

pub(crate) struct ToolFitOutcome {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub status_code: Option<u16>,
    pub outcome_class: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProviderCapabilityReceiptsFile {
    schema_version: String,
    generated_at_utc: String,
    receipts: BTreeMap<String, ProviderModelCapabilityReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProviderModelCapabilityReceipt {
    provider_id: String,
    model_id: String,
    updated_at_utc: String,
    capabilities: BTreeMap<String, CapabilityReceiptEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapabilityReceiptEntry {
    state: String,
    source: String,
    observed_at_utc: String,
    expires_at_utc: String,
    outcome_class: String,
    status_code: Option<u16>,
}

impl CharonService {
    pub fn record_probe_result(&self, payload: JsonValue) -> Result<()> {
        self.append_state_event("probe_result", payload)
    }

    pub async fn apply_provider_rate_limit_hints(
        &self,
        provider_id: &str,
        headers: &HeaderMap,
    ) -> Result<()> {
        let mut providers = self.providers.write().await;
        let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) else {
            return Ok(());
        };
        let now = Utc::now();

        let minute_limit = header_u64(
            headers,
            &[
                "x-ratelimit-limit-requests",
                "ratelimit-limit-requests",
                "x-ratelimit-limit-minute",
                "x-ratelimit-limit-minute-requests",
                "ratelimit-limit",
            ],
        );
        let minute_remaining = header_u64(
            headers,
            &[
                "x-ratelimit-remaining-requests",
                "ratelimit-remaining-requests",
                "x-ratelimit-remaining-minute",
                "x-ratelimit-remaining-minute-requests",
                "ratelimit-remaining",
            ],
        );
        let minute_reset_seconds = header_reset_seconds(
            headers,
            &[
                "x-ratelimit-reset-requests",
                "ratelimit-reset-requests",
                "x-ratelimit-reset-minute",
                "x-ratelimit-reset",
                "ratelimit-reset",
            ],
            now,
        );
        let day_limit = header_u64(
            headers,
            &[
                "x-ratelimit-limit-day",
                "x-ratelimit-limit-day-requests",
                "ratelimit-limit-day",
                "x-ratelimit-limit-requests-day",
            ],
        );
        let day_remaining = header_u64(
            headers,
            &[
                "x-ratelimit-remaining-day",
                "x-ratelimit-remaining-day-requests",
                "ratelimit-remaining-day",
                "x-ratelimit-remaining-requests-day",
            ],
        );
        let day_reset_seconds = header_reset_seconds(
            headers,
            &[
                "x-ratelimit-reset-day",
                "x-ratelimit-reset-requests-day",
                "ratelimit-reset-day",
            ],
            now,
        );
        let mut pressure_event = None;

        if let Some(limit) = minute_limit.filter(|limit| *limit > 0) {
            provider.requests_per_minute = Some(limit);
            provider.minute_window_started_utc =
                Some(window_started_from_reset(now, 60, minute_reset_seconds));
            if let Some(remaining) = minute_remaining {
                provider.requests_used_minute = if remaining_is_critical(limit, remaining) {
                    pressure_event = Some(serde_json::json!({
                        "provider_id": provider_id,
                        "window": "minute",
                        "limit": limit,
                        "remaining": remaining,
                        "reset_seconds": minute_reset_seconds,
                        "action": "preemptively_exhausted"
                    }));
                    limit
                } else {
                    limit.saturating_sub(remaining.min(limit))
                };
            }
        }

        if let Some(limit) = day_limit.filter(|limit| *limit > 0) {
            let configured_limit = provider.requests_per_day;
            let openrouter_downstream_hint = provider_id == "openrouter"
                && configured_limit.is_some_and(|configured| configured > limit);
            let effective_limit = if openrouter_downstream_hint {
                configured_limit.unwrap_or(limit)
            } else {
                limit
            };
            provider.requests_per_day = Some(effective_limit);
            provider.day_window_started_utc =
                Some(window_started_from_reset(now, 86_400, day_reset_seconds));
            if !openrouter_downstream_hint {
                if let Some(remaining) = day_remaining {
                    provider.requests_used_day =
                        if remaining_is_critical(effective_limit, remaining) {
                            pressure_event = Some(serde_json::json!({
                                "provider_id": provider_id,
                                "window": "day",
                                "limit": effective_limit,
                                "reported_limit": limit,
                                "remaining": remaining,
                                "reset_seconds": day_reset_seconds,
                                "action": "preemptively_exhausted"
                            }));
                            effective_limit
                        } else {
                            effective_limit.saturating_sub(remaining.min(effective_limit))
                        };
                }
            }
        }
        drop(providers);
        if let Some(payload) = pressure_event {
            self.append_state_event("provider_rate_limit_pressure", payload)?;
        }

        Ok(())
    }

    pub async fn mark_provider_cooldown(&self, provider_id: &str, seconds: i64) -> Result<()> {
        let mut providers = self.providers.write().await;
        if let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) {
            provider.in_cooldown = true;
            provider.cooldown_until_utc =
                Some((Utc::now() + Duration::seconds(seconds)).to_rfc3339());
            provider.cooldown_backoff_seconds = seconds.max(0) as u64;
            self.append_state_event(
                "provider_cooldown",
                serde_json::json!({"provider_id":provider_id,"seconds":seconds}),
            )?;
        }
        drop(providers);
        self.persist_provider_runtime_state().await?;
        Ok(())
    }

    pub async fn mark_provider_result(
        &self,
        provider_id: &str,
        ok: bool,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) -> Result<()> {
        let mut providers = self.providers.write().await;
        let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) else {
            return Ok(());
        };

        if ok {
            provider.consecutive_successes += 1;
            provider.consecutive_failures = 0;
            provider.last_error = None;
            provider.in_cooldown = false;
            provider.cooldown_until_utc = None;
            provider.cooldown_backoff_seconds = 0;
        } else {
            provider.error_count += 1;
            provider.consecutive_failures += 1;
            provider.consecutive_successes = 0;
            provider.last_error = error.clone();
            let reason_class =
                super::metrics::classify_failure_reason(error.as_deref().unwrap_or(""));
            self.metrics()
                .observe_provider_failure(provider_id, reason_class);
            if provider.consecutive_failures >= 3 {
                let backoff_seconds = failure_backoff_seconds(provider.consecutive_failures);
                provider.in_cooldown = true;
                provider.cooldown_backoff_seconds = backoff_seconds as u64;
                provider.cooldown_until_utc =
                    Some((Utc::now() + Duration::seconds(backoff_seconds)).to_rfc3339());
            }
        }
        self.observe_bandit_provider_result(provider_id, ok);

        if provider.active_connections > 0 {
            provider.active_connections -= 1;
        }
        if provider.active_connections == 0 {
            provider.last_reservation_utc = None;
        }
        provider.avg_latency_ms = super::merge_latency(provider.avg_latency_ms, latency_ms);

        let sigil = if ok {
            if provider.consecutive_failures == 0 && !provider.in_cooldown {
                machine_sigil_or_default(
                    "SG_ROUTE_PRIMARY",
                    vec![
                        "routing".to_string(),
                        "provider".to_string(),
                        "success".to_string(),
                    ],
                    "low",
                    "summarize",
                    "charon",
                )
            } else {
                machine_sigil_or_default(
                    "SG_ROUTE_FAILOVER",
                    vec![
                        "routing".to_string(),
                        "provider".to_string(),
                        "recovered".to_string(),
                    ],
                    "medium",
                    "summarize",
                    "charon",
                )
            }
        } else if provider.in_cooldown {
            machine_sigil_or_default(
                "SG_ROUTE_PROVIDER_COOLDOWN",
                vec![
                    "routing".to_string(),
                    "provider".to_string(),
                    "cooldown".to_string(),
                ],
                "medium",
                "summarize",
                "charon",
            )
        } else {
            machine_sigil_or_default(
                "SG_ROUTE_EDGE_DOWN",
                vec![
                    "routing".to_string(),
                    "provider".to_string(),
                    "failure".to_string(),
                ],
                "high",
                "keep",
                "charon",
            )
        };

        self.append_state_event(
            "provider_result",
            serde_json::json!({
                "provider_id": provider_id,
                "ok": ok,
                "latency_ms": latency_ms,
                "error": error,
                "error_count": provider.error_count,
                "consecutive_failures": provider.consecutive_failures,
                "in_cooldown": provider.in_cooldown,
                "cooldown_backoff_seconds": provider.cooldown_backoff_seconds,
                "soterion": sigil,
            }),
        )?;
        self.emit_work_signal_background(
            "charon",
            if ok { 0.25 } else { 0.18 },
            JouleWorkUnit::Network,
            Some(format!("provider_result:{provider_id}")),
        );
        self.emit_memory_event(
            "provider_result",
            &format!(
                "CHARON provider {} result ok={} failures={} cooldown={}",
                provider_id, ok, provider.consecutive_failures, provider.in_cooldown
            ),
            Some(if ok { 0.8 } else { 0.45 }),
            vec!["charon".to_string(), "provider".to_string()],
        );
        drop(providers);
        self.persist_provider_runtime_state().await?;
        Ok(())
    }

    pub(crate) fn record_tool_fit_observation(
        &self,
        decision: &RouteDecision,
        req: &crate::adaptive::types::ManweRequestEnvelope,
        attempt_body: &JsonValue,
        outcome: ToolFitOutcome,
    ) -> Result<()> {
        let observation = ToolFitObservation {
            ts_utc: Utc::now().to_rfc3339(),
            provider_id: decision.provider_id.clone(),
            model_id: decision.model_id.clone(),
            task_type: req.task_type.clone(),
            agent_id: req.agent_id.clone(),
            route_class: decision.route_class.clone(),
            execution_lane: decision.execution_lane.clone(),
            route_id: decision.route_id.clone(),
            tool_request: request_has_tools(attempt_body),
            tool_choice: tool_choice_label(attempt_body),
            tool_schema_count: tool_schema_count(attempt_body),
            tool_history_present: tool_history_present(attempt_body),
            structured_output_request: attempt_body.get("response_format").is_some(),
            streaming_request: attempt_body
                .get("stream")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false),
            ok: outcome.ok,
            latency_ms: outcome.latency_ms,
            status_code: outcome.status_code,
            outcome_class: outcome.outcome_class,
            error: outcome.error.map(|error| truncate_error(&error)),
        };
        append_jsonl(&self.tool_fit_ledger_path, &observation)?;
        self.record_provider_capability_receipts(&observation)?;
        self.append_state_event(
            "tool_fit_observation",
            serde_json::json!({
                "provider_id": observation.provider_id,
                "model_id": observation.model_id,
                "task_type": observation.task_type,
                "route_class": observation.route_class,
                "execution_lane": observation.execution_lane,
                "route_id": observation.route_id,
                "tool_request": observation.tool_request,
                "tool_schema_count": observation.tool_schema_count,
                "tool_history_present": observation.tool_history_present,
                "structured_output_request": observation.structured_output_request,
                "streaming_request": observation.streaming_request,
                "ok": observation.ok,
                "latency_ms": observation.latency_ms,
                "status_code": observation.status_code,
                "outcome_class": observation.outcome_class,
            }),
        )?;
        Ok(())
    }

    fn record_provider_capability_receipts(&self, observation: &ToolFitObservation) -> Result<()> {
        let capabilities = capability_receipts_from_observation(observation);
        let visible_reasoning_leak = observation.outcome_class == "visible_reasoning_leak";
        if capabilities.is_empty() && !visible_reasoning_leak {
            return Ok(());
        }
        if capabilities.is_empty() && visible_reasoning_leak {
            self.apply_capability_receipt_to_runtime_model(observation);
            return Ok(());
        }

        let now = Utc::now();
        let path = self.provider_capability_receipts_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;
        file.lock_exclusive()?;
        let mut content = String::new();
        use std::io::Read;
        file.read_to_string(&mut content)?;
        let mut receipt_file = serde_json::from_str::<ProviderCapabilityReceiptsFile>(&content)
            .unwrap_or_else(|_| ProviderCapabilityReceiptsFile {
                schema_version: "annunimas.charon.provider-capability-receipts.v1".to_string(),
                generated_at_utc: now.to_rfc3339(),
                receipts: BTreeMap::new(),
            });
        receipt_file.schema_version =
            "annunimas.charon.provider-capability-receipts.v1".to_string();
        receipt_file.generated_at_utc = now.to_rfc3339();

        let key = capability_receipt_key(&observation.provider_id, &observation.model_id);
        let model_receipt =
            receipt_file
                .receipts
                .entry(key)
                .or_insert_with(|| ProviderModelCapabilityReceipt {
                    provider_id: observation.provider_id.clone(),
                    model_id: observation.model_id.clone(),
                    updated_at_utc: now.to_rfc3339(),
                    capabilities: BTreeMap::new(),
                });
        model_receipt.provider_id = observation.provider_id.clone();
        model_receipt.model_id = observation.model_id.clone();
        model_receipt.updated_at_utc = now.to_rfc3339();
        for capability in capabilities {
            let ttl_hours = if observation.ok {
                positive_receipt_ttl_hours()
            } else {
                negative_receipt_ttl_hours()
            };
            model_receipt.capabilities.insert(
                capability,
                CapabilityReceiptEntry {
                    state: if observation.ok {
                        "passed".to_string()
                    } else {
                        "failed".to_string()
                    },
                    source: "passive_proxy_observation".to_string(),
                    observed_at_utc: now.to_rfc3339(),
                    expires_at_utc: (now + Duration::hours(ttl_hours)).to_rfc3339(),
                    outcome_class: observation.outcome_class.clone(),
                    status_code: observation.status_code,
                },
            );
        }

        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(
            (serde_json::to_string_pretty(&receipt_file).unwrap_or_else(|_| "{}".to_string())
                + "\n")
                .as_bytes(),
        )?;
        file.sync_data()?;
        let unlock_result = file.unlock();
        if let Err(err) = unlock_result {
            return Err(arda_core::error::ArdaError::Ledger(err));
        }

        self.apply_capability_receipt_to_runtime_model(observation);
        Ok(())
    }

    fn apply_capability_receipt_to_runtime_model(&self, observation: &ToolFitObservation) {
        let capabilities = capability_receipts_from_observation(observation);
        if capabilities.is_empty() && observation.outcome_class != "visible_reasoning_leak" {
            return;
        }
        let Ok(mut providers) = self.providers.try_write() else {
            return;
        };
        let Some(provider) = providers
            .iter_mut()
            .find(|provider| provider.id == observation.provider_id)
        else {
            return;
        };
        let Some(model) = provider
            .models
            .iter_mut()
            .find(|model| model.id == observation.model_id)
        else {
            return;
        };
        for capability in capabilities {
            match (capability.as_str(), observation.ok) {
                ("tools", true) => model.capabilities.tools = Some(true),
                ("tools", false) => model.capabilities.tools = Some(false),
                ("structured_output", true) => model.capabilities.structured_output = Some(true),
                ("structured_output", false) => model.capabilities.structured_output = Some(false),
                ("streaming", true) => {
                    model.capabilities.streaming = Some(true);
                    model.streaming_validated = Some(true);
                }
                ("streaming", false) => {
                    model.capabilities.streaming = Some(false);
                    model.streaming_validated = Some(false);
                }
                _ => {}
            }
        }
        if observation.outcome_class == "visible_reasoning_leak" {
            model.capabilities.visible_reasoning = Some(true);
        }
    }

    /// Record a client-side payload error (HTTP 4xx where the request itself
    /// was malformed/incompatible). Logs telemetry and updates last_error,
    /// but deliberately does NOT advance consecutive_failures or trigger
    /// cooldown — the provider is healthy; our payload is the bug.
    pub async fn mark_provider_client_error(
        &self,
        provider_id: &str,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) -> Result<()> {
        let mut providers = self.providers.write().await;
        let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) else {
            return Ok(());
        };

        provider.error_count += 1;
        provider.last_error = error.clone();
        if provider.active_connections > 0 {
            provider.active_connections -= 1;
        }
        if provider.active_connections == 0 {
            provider.last_reservation_utc = None;
        }
        provider.avg_latency_ms = super::merge_latency(provider.avg_latency_ms, latency_ms);

        let sigil = machine_sigil_or_default(
            "SG_ROUTE_CLIENT_ERROR",
            vec![
                "routing".to_string(),
                "provider".to_string(),
                "client_error".to_string(),
            ],
            "medium",
            "keep",
            "charon",
        );
        self.append_state_event(
            "provider_client_error",
            serde_json::json!({
                "provider_id": provider_id,
                "latency_ms": latency_ms,
                "error": error,
                "error_count": provider.error_count,
                "consecutive_failures": provider.consecutive_failures,
                "in_cooldown": provider.in_cooldown,
                "soterion": sigil,
            }),
        )?;
        self.emit_work_signal_background(
            "charon",
            0.18,
            JouleWorkUnit::Network,
            Some(format!("provider_client_error:{provider_id}")),
        );
        self.emit_memory_event(
            "provider_client_error",
            &format!(
                "CHARON provider {} rejected payload (no cooldown advance) — {}",
                provider_id,
                error.as_deref().unwrap_or("unknown")
            ),
            Some(0.45),
            vec![
                "charon".to_string(),
                "provider".to_string(),
                "client_error".to_string(),
            ],
        );
        drop(providers);
        self.persist_provider_runtime_state().await?;
        Ok(())
    }

    pub async fn mark_model_result(
        &self,
        provider_id: &str,
        model_id: &str,
        ok: bool,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) -> Result<()> {
        let mut providers = self.providers.write().await;
        let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) else {
            return Ok(());
        };
        let Some(model) = provider.models.iter_mut().find(|m| m.id == model_id) else {
            return Ok(());
        };

        if ok {
            model.healthy = true;
            model.in_cooldown = false;
            model.cooldown_until_utc = None;
            model.consecutive_successes += 1;
            model.consecutive_failures = 0;
            model.last_error = None;
        } else {
            model.consecutive_failures += 1;
            model.consecutive_successes = 0;
            model.last_error = error.clone();
            model.healthy = false;
            let backoff_seconds = failure_backoff_seconds(model.consecutive_failures.max(3));
            model.in_cooldown = true;
            model.cooldown_until_utc =
                Some((Utc::now() + Duration::seconds(backoff_seconds)).to_rfc3339());
        }

        model.avg_latency_ms = super::merge_latency(model.avg_latency_ms, latency_ms);
        self.append_state_event(
            "model_result",
            serde_json::json!({
                "provider_id": provider_id,
                "model_id": model_id,
                "ok": ok,
                "latency_ms": latency_ms,
                "error": error,
                "healthy": model.healthy,
                "in_cooldown": model.in_cooldown,
                "cooldown_until_utc": model.cooldown_until_utc,
                "consecutive_failures": model.consecutive_failures,
            }),
        )?;
        drop(providers);
        self.persist_provider_runtime_state().await?;
        Ok(())
    }

    pub async fn mark_model_streaming_validation(
        &self,
        provider_id: &str,
        model_id: &str,
        streaming_validated: bool,
        error: Option<String>,
    ) -> Result<()> {
        let mut providers = self.providers.write().await;
        let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) else {
            return Ok(());
        };
        let Some(model) = provider.models.iter_mut().find(|m| m.id == model_id) else {
            return Ok(());
        };
        model.streaming_validated = Some(streaming_validated);
        if !streaming_validated {
            model.last_error = error.clone();
        }
        self.append_state_event(
            "model_streaming_validation",
            serde_json::json!({
                "provider_id": provider_id,
                "model_id": model_id,
                "streaming_validated": streaming_validated,
                "error": error,
            }),
        )?;
        drop(providers);
        self.persist_provider_runtime_state().await?;
        Ok(())
    }

    pub async fn release_provider_reservation(&self, provider_id: &str) -> Result<()> {
        let mut providers = self.providers.write().await;
        let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) else {
            return Ok(());
        };
        if provider.active_connections > 0 {
            provider.active_connections -= 1;
        }
        if provider.active_connections == 0 {
            provider.last_reservation_utc = None;
        }
        self.append_state_event(
            "provider_reservation_released",
            serde_json::json!({
                "provider_id": provider_id,
                "active_connections": provider.active_connections,
            }),
        )?;
        Ok(())
    }

    pub async fn tick_maintenance(&self) -> Result<()> {
        {
            let mut providers = self.providers.write().await;
            refresh_provider_windows(&mut providers, Utc::now());
        }
        self.refresh_local_provider_health().await?;
        self.append_state_event("maintenance_tick", serde_json::json!({}))?;
        Ok(())
    }

    pub async fn reload_provider_config(&self) -> Result<serde_json::Value> {
        let loaded = load_providers_from_config(&self.config_path, &self.bootstrap_state_path)?;
        let config_source = if self.config_path.exists() {
            "provider_file"
        } else {
            "governed_defaults"
        };
        let mut providers = self.providers.write().await;
        let previous = providers.clone();
        let merged = merge_runtime_state(previous, loaded);
        let total = merged.len();
        let enabled = merged.iter().filter(|provider| provider.enabled).count();
        *providers = merged;
        *self.config_source.write().await = config_source.to_string();
        let catalog_generation = self.catalog_generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.persist_provider_runtime_state_snapshot(&providers)?;
        self.append_state_event(
            "providers_reloaded",
            serde_json::json!({
                "config_path": self.config_path,
                "config_source": config_source,
                "bootstrap_state_path": self.bootstrap_state_path,
                "providers_total": total,
                "providers_enabled": enabled,
                "catalog_generation": catalog_generation
            }),
        )?;
        self.emit_memory_event(
            "providers_reloaded",
            &format!("CHARON reloaded provider config with {} providers", total),
            Some(0.75),
            vec!["charon".to_string(), "config".to_string()],
        );
        Ok(serde_json::json!({
            "ok": true,
            "config_path": self.config_path,
            "config_source": config_source,
            "bootstrap_state_path": self.bootstrap_state_path,
            "providers_total": total,
            "providers_enabled": enabled,
            "catalog_generation": catalog_generation
        }))
    }
}

fn header_u64(headers: &HeaderMap, names: &[&str]) -> Option<u64> {
    names.iter().find_map(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok())
    })
}

fn header_reset_seconds(
    headers: &HeaderMap,
    names: &[&str],
    now: chrono::DateTime<Utc>,
) -> Option<i64> {
    names.iter().find_map(|name| {
        let raw = headers.get(*name)?.to_str().ok()?.trim();
        if let Ok(value) = raw.parse::<i64>() {
            if value > 1_000_000_000 {
                return Some((value - now.timestamp()).max(0));
            }
            return Some(value.max(0));
        }
        chrono::DateTime::parse_from_rfc3339(raw)
            .ok()
            .map(|dt| (dt.with_timezone(&Utc) - now).num_seconds().max(0))
    })
}

fn window_started_from_reset(
    now: chrono::DateTime<Utc>,
    window_seconds: i64,
    reset_seconds: Option<i64>,
) -> String {
    let elapsed = reset_seconds
        .map(|seconds| window_seconds.saturating_sub(seconds.min(window_seconds)))
        .unwrap_or(0);
    (now - Duration::seconds(elapsed)).to_rfc3339()
}

fn request_has_tools(body: &JsonValue) -> bool {
    tool_schema_count(body) > 0 || body.get("tool_choice").is_some() || tool_history_present(body)
}

fn tool_schema_count(body: &JsonValue) -> usize {
    body.get("tools")
        .and_then(JsonValue::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn tool_history_present(body: &JsonValue) -> bool {
    body.get("messages")
        .and_then(JsonValue::as_array)
        .is_some_and(|messages| {
            messages.iter().any(|message| {
                message.get("role").and_then(JsonValue::as_str) == Some("tool")
                    || message.get("tool_calls").is_some()
            })
        })
}

fn tool_choice_label(body: &JsonValue) -> Option<String> {
    let value = body.get("tool_choice")?;
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if value.is_null() {
        return None;
    }
    Some("structured".to_string())
}

fn capability_receipts_from_observation(observation: &ToolFitObservation) -> Vec<String> {
    if observation.ok {
        let mut capabilities = vec!["basic_chat".to_string()];
        if observation.tool_request {
            capabilities.push("tools".to_string());
        }
        if observation.structured_output_request {
            capabilities.push("structured_output".to_string());
        }
        if observation.streaming_request {
            capabilities.push("streaming".to_string());
        }
        return capabilities;
    }

    if !capability_failure_should_gate(observation) {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    if observation.tool_request {
        capabilities.push("tools".to_string());
    }
    if observation.structured_output_request {
        capabilities.push("structured_output".to_string());
    }
    if observation.streaming_request {
        capabilities.push("streaming".to_string());
    }
    capabilities
}

fn capability_failure_should_gate(observation: &ToolFitObservation) -> bool {
    matches!(
        observation.outcome_class.as_str(),
        "client_payload_error"
            | "payload_dialect_retry"
            | "tool_protocol_leak"
            | "visible_reasoning_leak"
            | "malformed_structured_output"
    )
}

fn capability_receipt_key(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}::{model_id}")
}

fn positive_receipt_ttl_hours() -> i64 {
    std::env::var("ARDA_MANWE_CAPABILITY_POSITIVE_TTL_HOURS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(168)
}

fn negative_receipt_ttl_hours() -> i64 {
    std::env::var("ARDA_MANWE_CAPABILITY_NEGATIVE_TTL_HOURS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(24)
}

fn truncate_error(error: &str) -> String {
    const MAX: usize = 500;
    let mut out = String::new();
    for ch in error.chars().take(MAX) {
        out.push(ch);
    }
    if error.chars().count() > MAX {
        out.push_str("...");
    }
    out
}

fn remaining_is_critical(limit: u64, remaining: u64) -> bool {
    remaining <= 1 || (remaining as f64 / limit.max(1) as f64) <= 0.05
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};
    use tempfile::tempdir;

    #[test]
    fn malformed_structured_output_demotes_schema_without_demoting_tools() {
        let observation = ToolFitObservation {
            ts_utc: "2026-06-07T00:00:00Z".to_string(),
            route_id: "route-1".to_string(),
            provider_id: "edge_backbone_coder".to_string(),
            model_id: "Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL".to_string(),
            agent_id: "openai_shim".to_string(),
            task_type: "code".to_string(),
            route_class: "tool_oriented".to_string(),
            execution_lane: "execution".to_string(),
            ok: false,
            outcome_class: "malformed_structured_output".to_string(),
            status_code: Some(200),
            latency_ms: Some(742),
            error: None,
            tool_request: false,
            structured_output_request: true,
            streaming_request: false,
            tool_choice: None,
            tool_schema_count: 0,
            tool_history_present: false,
        };

        assert_eq!(
            capability_receipts_from_observation(&observation),
            vec!["structured_output".to_string()]
        );
    }

    #[tokio::test]
    async fn openrouter_rate_limit_hints_do_not_lower_configured_daily_cap() {
        let dir = tempdir().expect("tempdir");
        let service = CharonService::new(dir.path()).expect("service");
        {
            let mut providers = service.providers.write().await;
            let openrouter_index = providers
                .iter()
                .position(|provider| provider.id == "openrouter")
                .unwrap_or(0);
            providers[openrouter_index].id = "openrouter".to_string();
            let openrouter = &mut providers[openrouter_index];
            openrouter.requests_per_day = Some(2_000);
            openrouter.requests_used_day = 7;
        }

        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit-day", HeaderValue::from_static("50"));
        headers.insert("x-ratelimit-remaining-day", HeaderValue::from_static("49"));

        service
            .apply_provider_rate_limit_hints("openrouter", &headers)
            .await
            .expect("apply hints");

        let providers = service.providers.read().await;
        let openrouter = providers
            .iter()
            .find(|provider| provider.id == "openrouter")
            .expect("openrouter provider");
        assert_eq!(openrouter.requests_per_day, Some(2_000));
        assert_eq!(openrouter.requests_used_day, 7);
    }
}
