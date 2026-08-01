use super::{
    attach_manwe_route_metadata, evaluate_pre_route_governance_with_options,
    excluded_provider_ids, is_billing_or_credit_error, is_client_payload_error,
    is_context_overflow_error, is_local_provider, is_reasoning_replay_required_error,
    is_request_scoped_retry_error, local_payload_requires_structured_tool_history,
    model_error_should_mark_unavailable, normalize_openai_request_payload_with_policy,
    normalize_openai_response, provider_error_immediate_cooldown_seconds,
    provider_error_should_fallback, proxy_max_attempts, slim_local_attempt_body,
    strip_internal_openai_routing_fields, transport_failure_should_trigger_cooldown,
    ArdaError, CharonService, GateAction, JsonValue, ProviderState, Result, RouteDecision,
    StdDuration,
};
use crate::adaptive::service::adaptive_routing::classify_semantic_outcome;
use crate::adaptive::service::state_mutation::ToolFitOutcome;
use crate::adaptive::types::ManweRequestEnvelope;
use std::time::Instant;

fn apply_exclusions(
    routed_req: &mut ManweRequestEnvelope,
    excluded_provider_ids: &[String],
    excluded_model_ids: &[String],
) {
    if !excluded_provider_ids.is_empty() {
        routed_req.options["exclude_provider_ids"] = serde_json::json!(excluded_provider_ids);
    }
    if !excluded_model_ids.is_empty() {
        routed_req.options["exclude_model_ids"] = serde_json::json!(excluded_model_ids);
    }
}

fn push_excluded_model(excluded_model_ids: &mut Vec<String>, model_id: &str) {
    if !excluded_model_ids
        .iter()
        .any(|existing| existing == model_id)
    {
        excluded_model_ids.push(model_id.to_string());
    }
}

fn reasoning_replay_required_for_route(
    routes: &[(String, String)],
    provider_id: &str,
    model_id: &str,
) -> bool {
    routes
        .iter()
        .any(|(provider, model)| provider == provider_id && model == model_id)
}

fn remember_reasoning_replay_route(
    routes: &mut Vec<(String, String)>,
    provider_id: &str,
    model_id: &str,
) {
    if !reasoning_replay_required_for_route(routes, provider_id, model_id) {
        routes.push((provider_id.to_string(), model_id.to_string()));
    }
}

fn request_requires_tool_payload(req: &ManweRequestEnvelope) -> bool {
    req.options
        .get("tool_use_required")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
        || req
            .options
            .get("tool_choice")
            .is_some_and(tool_choice_requires_tool_call)
        || req.messages.iter().any(message_has_tool_history)
}

fn tool_choice_requires_tool_call(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => false,
        JsonValue::Bool(required) => *required,
        JsonValue::String(raw) => {
            let trimmed = raw.trim().to_ascii_lowercase();
            !trimmed.is_empty() && !matches!(trimmed.as_str(), "auto" | "none" | "off")
        }
        JsonValue::Object(map) => !map.is_empty(),
        JsonValue::Array(items) => !items.is_empty(),
        _ => true,
    }
}

fn message_has_tool_history(message: &JsonValue) -> bool {
    (message.get("role").and_then(JsonValue::as_str) == Some("tool")
        && message
            .get("tool_call_id")
            .and_then(JsonValue::as_str)
            .is_some_and(|value| !value.trim().is_empty()))
        || message
            .get("tool_calls")
            .and_then(JsonValue::as_array)
            .is_some_and(|items| !items.is_empty())
        || message
            .get("function_call")
            .is_some_and(|value| !value.is_null())
}

pub(crate) fn strip_optional_tool_payload(req: &ManweRequestEnvelope, body: &mut JsonValue) {
    if request_requires_tool_payload(req) {
        return;
    }
    if let Some(payload) = body.as_object_mut() {
        payload.remove("tools");
        payload.remove("tool_choice");
    }
}

/// Result of opening a streaming proxy connection. Carries the live upstream
/// response plus the provider/model/lane that produced it, so the transport
/// layer can wire chunk-level errors back into provider health (otherwise a
/// provider whose initial HTTP 200 came back fine but whose SSE body falls
/// over mid-stream will never be penalized and will keep getting picked).
pub struct StreamingProxyOutcome {
    pub response: reqwest::Response,
    pub provider_id: String,
    pub model_id: String,
    pub route_class: String,
    pub execution_lane: String,
    pub route_id: String,
}

/// Result of a non-streaming OpenAI-compatible proxy call. The response body
/// still carries `_manwe_route`; these fields let HTTP clients and Hermes
/// surface route attribution without parsing JSON.
pub struct PassthroughProxyOutcome {
    pub status: u16,
    pub response: JsonValue,
    pub provider_id: String,
    pub model_id: String,
    pub route_class: String,
    pub execution_lane: String,
    pub route_id: String,
}

#[derive(Debug, Clone)]
struct ProxyAttemptSummary {
    provider_id: String,
    model_id: String,
    route_id: String,
    route_class: String,
    execution_lane: String,
    status_code: Option<u16>,
    outcome_class: String,
    message: String,
}

fn push_proxy_attempt(
    attempts: &mut Vec<ProxyAttemptSummary>,
    decision: &RouteDecision,
    provider_id: &str,
    status_code: Option<u16>,
    outcome_class: impl Into<String>,
    message: impl Into<String>,
) {
    const MAX_ATTEMPTS_REPORTED: usize = 12;
    if attempts.len() >= MAX_ATTEMPTS_REPORTED {
        return;
    }
    attempts.push(ProxyAttemptSummary {
        provider_id: provider_id.to_string(),
        model_id: decision.model_id.clone(),
        route_id: decision.route_id.clone(),
        route_class: decision.route_class.clone(),
        execution_lane: decision.execution_lane.clone(),
        status_code,
        outcome_class: outcome_class.into(),
        message: truncate_attempt_message(&message.into()),
    });
}

fn proxy_attempts_json(attempts: &[ProxyAttemptSummary]) -> JsonValue {
    JsonValue::Array(
        attempts
            .iter()
            .map(|attempt| {
                serde_json::json!({
                    "provider_id": attempt.provider_id,
                    "model_id": attempt.model_id,
                    "route_id": attempt.route_id,
                    "route_class": attempt.route_class,
                    "execution_lane": attempt.execution_lane,
                    "status_code": attempt.status_code,
                    "outcome_class": attempt.outcome_class,
                    "message": attempt.message,
                })
            })
            .collect(),
    )
}

fn truncate_attempt_message(message: &str) -> String {
    const MAX_LEN: usize = 180;
    let compact = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= MAX_LEN {
        return compact;
    }
    let mut out = compact.chars().take(MAX_LEN).collect::<String>();
    out.push('…');
    out
}

fn format_proxy_attempt_summary(attempts: &[ProxyAttemptSummary]) -> String {
    if attempts.is_empty() {
        return "proxy routing exhausted all fallback providers; attempts=[]".to_string();
    }
    let items = attempts
        .iter()
        .map(|attempt| {
            let status = attempt
                .status_code
                .map(|status| format!("HTTP {status}"))
                .unwrap_or_else(|| "transport".to_string());
            format!(
                "{}:{} {} {} ({})",
                attempt.provider_id,
                attempt.model_id,
                status,
                attempt.outcome_class,
                attempt.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "proxy routing exhausted fallback providers after {} attempt(s): [{}]",
        attempts.len(),
        items
    )
}

fn concrete_forced_route_value(value: Option<&JsonValue>) -> Option<String> {
    let value = value?.as_str()?.trim();
    if value.is_empty() || matches!(value, "auto" | "default" | "charon/auto") {
        None
    } else {
        Some(value.to_string())
    }
}

impl CharonService {
    fn record_proxy_fallback_chain(
        &self,
        req: &ManweRequestEnvelope,
        attempts: &[ProxyAttemptSummary],
        reason: &str,
    ) {
        if attempts.is_empty() {
            return;
        }
        let _ = self.append_state_event(
            "route_fallback_chain",
            serde_json::json!({
                "agent_id": req.agent_id,
                "task_type": req.task_type,
                "priority": req.priority,
                "attempt_count": attempts.len(),
                "reason": reason,
                "attempts": proxy_attempts_json(attempts),
            }),
        );
    }

    pub async fn proxy_openai(&self, req: ManweRequestEnvelope) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({
            "model": req
                .options
                .get("force_model_id")
                .cloned()
                .unwrap_or(JsonValue::Null),
            "messages": req.messages,
        });
        for key in [
            "max_tokens",
            "temperature",
            "top_p",
            "stream",
            "response_format",
            "tools",
            "tool_choice",
            "stop",
        ] {
            if let Some(v) = req.options.get(key) {
                body[key] = v.clone();
            }
        }
        if body.get("model").is_some_and(|v| v.is_null()) {
            body.as_object_mut().map(|obj| obj.remove("model"));
        }
        strip_optional_tool_payload(&req, &mut body);

        let (decision, provider_id, url, latency_ms, status, parsed) =
            self.proxy_openai_request(req, body).await?;
        Ok(serde_json::json!({
            "ok": true,
            "dry_run": false,
            "route": decision,
            "provider_id": provider_id,
            "url": url,
            "status": status,
            "latency_ms": latency_ms,
            "response": parsed
        }))
    }

    pub async fn proxy_openai_passthrough(
        &self,
        req: ManweRequestEnvelope,
        body: JsonValue,
    ) -> Result<(u16, JsonValue)> {
        let outcome = self.proxy_openai_passthrough_with_route(req, body).await?;
        Ok((outcome.status, outcome.response))
    }

    pub async fn proxy_openai_passthrough_with_route(
        &self,
        req: ManweRequestEnvelope,
        body: JsonValue,
    ) -> Result<PassthroughProxyOutcome> {
        let (decision, provider_id, _, _, status, parsed) =
            self.proxy_openai_request(req, body).await?;
        Ok(PassthroughProxyOutcome {
            status,
            response: parsed,
            provider_id,
            model_id: decision.model_id,
            route_class: decision.route_class,
            execution_lane: decision.execution_lane,
            route_id: decision.route_id,
        })
    }

    pub async fn proxy_openai_streaming(
        &self,
        req: ManweRequestEnvelope,
        body: JsonValue,
    ) -> Result<StreamingProxyOutcome> {
        let endpoint = req
            .options
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("/chat/completions");

        let mut excluded = excluded_provider_ids(&req.options);
        let mut excluded_models = super::route_policy::excluded_model_ids(&req.options);
        let mut routed_req = req.clone();
        let forced_provider_id = concrete_forced_route_value(req.options.get("force_provider_id"));
        let forced_model_id = concrete_forced_route_value(req.options.get("force_model_id"));
        let max_attempts = {
            let providers = self.providers.read().await;
            proxy_max_attempts(providers.len())
        };
        let mut attempts: Vec<ProxyAttemptSummary> = Vec::new();
        let mut reasoning_replay_routes: Vec<(String, String)> = Vec::new();

        for _attempt in 0..max_attempts {
            apply_exclusions(&mut routed_req, &excluded, &excluded_models);
            // B1: route_and_resolve returns the resolved provider snapshot
            // in the same lock window as the route decision, replacing what
            // used to be route() + providers.read() per attempt.
            let (decision, provider) = match self.route_and_resolve(routed_req.clone()).await {
                Ok(pair) => pair,
                Err(err) => {
                    if !attempts.is_empty() {
                        self.record_proxy_fallback_chain(&req, &attempts, "route_selection_failed");
                    }
                    return Err(ArdaError::Agent {
                        agent: "manwe".to_string(),
                        message: if attempts.is_empty() {
                            err.to_string()
                        } else {
                            format_proxy_attempt_summary(&attempts)
                        },
                    });
                }
            };

            // The hermes_agent_cli driver bypasses HTTP entirely (it shells
            // out to hermes-agent), so it cannot serve a streaming proxy
            // response. Exclude it from this request and let the router pick
            // the next candidate.
            if provider.driver == "hermes_agent_cli" {
                push_proxy_attempt(
                    &mut attempts,
                    &decision,
                    &provider.id,
                    None,
                    "driver_incompatible",
                    "hermes_agent_cli driver cannot serve streaming proxy response",
                );
                if !excluded.contains(&provider.id) {
                    excluded.push(provider.id.clone());
                }
                continue;
            }

            let base_url = if provider.driver == "hermes_proxy" {
                super::hermes_proxy_driver::ensure_hermes_proxy(&provider).await?
            } else {
                provider
                    .base_url
                    .clone()
                    .ok_or_else(|| ArdaError::Agent {
                        agent: "manwe".to_string(),
                        message: format!(
                            "provider {} missing base_url for streaming proxy",
                            provider.id
                        ),
                    })?
            };
            let url = format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                endpoint.trim_start_matches('/')
            );

            let mut attempt_body = body.clone();
            if let Some(payload) = attempt_body.as_object_mut() {
                payload.insert(
                    "model".to_string(),
                    JsonValue::String(decision.model_id.clone()),
                );
                strip_internal_openai_routing_fields(payload);
                payload.remove("extra_body");
            } else {
                attempt_body = serde_json::json!({
                    "model": decision.model_id,
                    "messages": [],
                    "stream": true
                });
            }
            strip_optional_tool_payload(&routed_req, &mut attempt_body);
            let preserve_reasoning_replay = reasoning_replay_required_for_route(
                &reasoning_replay_routes,
                &provider.id,
                &decision.model_id,
            );
            normalize_openai_request_payload_with_policy(
                &mut attempt_body,
                preserve_reasoning_replay,
            );
            if is_local_provider(&provider.id)
                && !local_payload_requires_structured_tool_history(&attempt_body)
            {
                slim_local_attempt_body(&mut attempt_body);
            }

            // Streaming uses a pooled client built with connect_timeout +
            // read_timeout (per-read, not total) so long SSE bodies don't die
            // mid-stream while reasoning models stream tokens. See B4 in
            // OPTIMIZATION_PLAN.md for the pooling rationale; the timeout
            // tuning is the original streaming-fix invariant.
            let client = self
                .http_client_for(&provider.id, true, &decision.execution_lane)
                .await?;
            let mut request = client.post(&url).json(&attempt_body);
            if let Some(env_key) = provider.api_key_env.as_deref() {
                if let Ok(key) = std::env::var(env_key) {
                    if !key.trim().is_empty() {
                        request = request.bearer_auth(key);
                    }
                }
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let _ = self.record_tool_fit_observation(
                            &decision,
                            &routed_req,
                            &attempt_body,
                            ToolFitOutcome {
                                ok: true,
                                latency_ms: None,
                                status_code: Some(response.status().as_u16()),
                                outcome_class: "stream_open".to_string(),
                                error: None,
                            },
                        );
                        let _ = self.release_provider_reservation(&provider.id).await;
                        return Ok(StreamingProxyOutcome {
                            response,
                            provider_id: provider.id.clone(),
                            model_id: decision.model_id.clone(),
                            route_class: decision.route_class.clone(),
                            execution_lane: decision.execution_lane.clone(),
                            route_id: decision.route_id.clone(),
                        });
                    }
                    let status_u16 = response.status().as_u16();
                    let text = response.text().await.unwrap_or_default();
                    let parsed: JsonValue = serde_json::from_str(&text)
                        .unwrap_or_else(|_| serde_json::json!({"raw": text}));
                    let err_msg = format!("provider {} HTTP {}", provider.id, status_u16);
                    let client_payload_error = is_client_payload_error(status_u16, &parsed);
                    let reasoning_replay_required =
                        is_reasoning_replay_required_error(status_u16, &parsed);
                    let outcome_class = if client_payload_error {
                        "client_payload_error".to_string()
                    } else if reasoning_replay_required {
                        "payload_dialect_retry".to_string()
                    } else {
                        outcome_class_for_http_error(status_u16, Some(&parsed))
                    };
                    let _ = self.record_tool_fit_observation(
                        &decision,
                        &routed_req,
                        &attempt_body,
                        ToolFitOutcome {
                            ok: false,
                            latency_ms: None,
                            status_code: Some(status_u16),
                            outcome_class: outcome_class.clone(),
                            error: Some(format!("{err_msg}: {parsed}")),
                        },
                    );
                    let model_scoped_error =
                        model_error_should_mark_unavailable(status_u16, &parsed)
                            && provider_has_alternate_routable_model(
                                &provider,
                                &decision.model_id,
                                &req.task_type,
                                Some(&req),
                            );
                    let request_scoped_retry_error =
                        is_request_scoped_retry_error(status_u16, &parsed);
                    if reasoning_replay_required && !preserve_reasoning_replay {
                        let _ = self.release_provider_reservation(&provider.id).await;
                        remember_reasoning_replay_route(
                            &mut reasoning_replay_routes,
                            &provider.id,
                            &decision.model_id,
                        );
                    } else if client_payload_error {
                        let _ = self
                            .mark_provider_client_error(
                                &provider.id,
                                None,
                                Some(format!("{err_msg}: {parsed}")),
                            )
                            .await;
                    } else if model_scoped_error || request_scoped_retry_error {
                        let _ = self.release_provider_reservation(&provider.id).await;
                        if request_scoped_retry_error {
                            if let Some(cooldown_seconds) =
                                provider_error_immediate_cooldown_seconds(
                                    &provider.id,
                                    status_u16,
                                    &parsed,
                                )
                            {
                                let _ = self
                                    .mark_provider_cooldown(&provider.id, cooldown_seconds)
                                    .await;
                            }
                        }
                    } else {
                        let _ = self
                            .mark_provider_result(&provider.id, false, None, Some(err_msg.clone()))
                            .await;
                        if let Some(cooldown_seconds) = provider_error_immediate_cooldown_seconds(
                            &provider.id,
                            status_u16,
                            &parsed,
                        ) {
                            let _ = self
                                .mark_provider_cooldown(&provider.id, cooldown_seconds)
                                .await;
                        }
                    }
                    if model_error_should_mark_unavailable(status_u16, &parsed) {
                        let _ = self
                            .mark_model_result(
                                &provider.id,
                                &decision.model_id,
                                false,
                                None,
                                Some(format!("{err_msg}: {parsed}")),
                            )
                            .await;
                    }
                    push_proxy_attempt(
                        &mut attempts,
                        &decision,
                        &provider.id,
                        Some(status_u16),
                        outcome_class,
                        format!("{err_msg}: {parsed}"),
                    );
                    if reasoning_replay_required && !preserve_reasoning_replay {
                        continue;
                    }
                    if provider_error_should_fallback(status_u16, &parsed) {
                        if !model_scoped_error {
                            excluded.push(provider.id.clone());
                        } else {
                            push_excluded_model(&mut excluded_models, &decision.model_id);
                        }
                    } else {
                        excluded.push(provider.id.clone());
                    }
                    if forced_provider_id.as_deref() == Some(provider.id.as_str())
                        && (!model_scoped_error || forced_model_id.is_some())
                    {
                        break;
                    }
                }
                Err(err) => {
                    let err_msg =
                        format!("streaming proxy request failed to {}: {err}", provider.id);
                    let _ = self.record_tool_fit_observation(
                        &decision,
                        &routed_req,
                        &attempt_body,
                        ToolFitOutcome {
                            ok: false,
                            latency_ms: None,
                            status_code: None,
                            outcome_class: "transport_failure".to_string(),
                            error: Some(err_msg.clone()),
                        },
                    );
                    let _ = self
                        .mark_provider_result(&provider.id, false, None, Some(err_msg.clone()))
                        .await;
                    push_proxy_attempt(
                        &mut attempts,
                        &decision,
                        &provider.id,
                        None,
                        "transport_failure",
                        err_msg,
                    );
                    excluded.push(provider.id.clone());
                    if forced_provider_id.as_deref() == Some(provider.id.as_str()) {
                        break;
                    }
                }
            }
        }

        self.record_proxy_fallback_chain(&req, &attempts, "streaming_proxy_exhausted");
        Err(ArdaError::Agent {
            agent: "manwe".to_string(),
            message: format_proxy_attempt_summary(&attempts),
        })
    }

    async fn proxy_openai_request(
        &self,
        req: ManweRequestEnvelope,
        body: JsonValue,
    ) -> Result<(RouteDecision, String, String, u64, u16, JsonValue)> {
        let input_text = req
            .messages
            .iter()
            .filter_map(|m| m.get("content").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        let governance = evaluate_pre_route_governance_with_options(&input_text, &req.options);
        if matches!(governance.action, GateAction::Abort) {
            self.append_state_event(
                "echo_gate_proxy_abort",
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
                }),
            )?;
            self.append_governance_event(
                "echo_gate_proxy_abort",
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
                    "verdict": "blocked",
                    "failure_class": "echo_gate_abort_proxy",
                }),
            )?;
            return Err(ArdaError::Agent {
                agent: "manwe".to_string(),
                message: format!(
                    "Echo Gate ABORT [{}] blocked proxy routing before provider fallback (rho={:.2}, gamma={:.2}, delta={:.2})",
                    governance.trigger_reason,
                    governance.rho,
                    governance.gamma,
                    governance.delta
                ),
            });
        }

        let endpoint = req
            .options
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("/chat/completions");
        let dry_run = req
            .options
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| {
                std::env::var("ARDA_MANWE_PROXY_DRY_RUN")
                    .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                    .unwrap_or(false)
            });

        let mut excluded = excluded_provider_ids(&req.options);
        let mut excluded_models = super::route_policy::excluded_model_ids(&req.options);
        let mut routed_req = req.clone();
        let mut attempts: Vec<ProxyAttemptSummary> = Vec::new();
        let forced_provider_id = concrete_forced_route_value(req.options.get("force_provider_id"));
        let forced_model_id = concrete_forced_route_value(req.options.get("force_model_id"));
        let max_attempts = {
            let providers = self.providers.read().await;
            proxy_max_attempts(providers.len())
        };
        let mut reasoning_replay_routes: Vec<(String, String)> = Vec::new();

        for _attempt in 0..max_attempts {
            apply_exclusions(&mut routed_req, &excluded, &excluded_models);
            let (decision, provider) = match self.route_and_resolve(routed_req.clone()).await {
                Ok(pair) => pair,
                Err(err) => {
                    if !attempts.is_empty() {
                        self.record_proxy_fallback_chain(&req, &attempts, "route_selection_failed");
                    }
                    return Err(ArdaError::Agent {
                        agent: "manwe".to_string(),
                        message: if attempts.is_empty() {
                            err.to_string()
                        } else {
                            format_proxy_attempt_summary(&attempts)
                        },
                    });
                }
            };

            if let Some(probe) = self
                .maybe_refresh_provider_capacity_probe(&provider)
                .await?
            {
                if probe.blocked {
                    excluded.push(provider.id.clone());
                    routed_req.options["exclude_provider_ids"] = serde_json::json!(excluded);
                    push_proxy_attempt(
                        &mut attempts,
                        &decision,
                        &provider.id,
                        None,
                        "preflight_blocked",
                        format!("provider preflight blocked: {}", probe.reason),
                    );
                    if forced_provider_id.as_deref() == Some(provider.id.as_str()) {
                        break;
                    }
                    continue;
                }
            }

            // The hermes_agent_cli driver doesn't use HTTP at all, so
            // skip the base_url requirement for it.
            let url = if provider.driver == "hermes_agent_cli" {
                format!("hermes-cli://{}", provider.id)
            } else {
                let base_url = if provider.driver == "hermes_proxy" {
                    super::hermes_proxy_driver::ensure_hermes_proxy(&provider).await?
                } else {
                    provider
                        .base_url
                        .clone()
                        .ok_or_else(|| ArdaError::Agent {
                            agent: "manwe".to_string(),
                            message: format!(
                                "provider {} missing base_url for proxy forwarding",
                                provider.id
                            ),
                        })?
                };
                format!(
                    "{}/{}",
                    base_url.trim_end_matches('/'),
                    endpoint.trim_start_matches('/')
                )
            };

            let mut attempt_body = body.clone();
            if let Some(payload) = attempt_body.as_object_mut() {
                payload.insert(
                    "model".to_string(),
                    JsonValue::String(decision.model_id.clone()),
                );
                strip_internal_openai_routing_fields(payload);
                payload.remove("extra_body");
            } else {
                attempt_body = serde_json::json!({
                    "model": decision.model_id,
                    "messages": []
                });
            }
            strip_optional_tool_payload(&routed_req, &mut attempt_body);
            let preserve_reasoning_replay = reasoning_replay_required_for_route(
                &reasoning_replay_routes,
                &provider.id,
                &decision.model_id,
            );
            normalize_openai_request_payload_with_policy(
                &mut attempt_body,
                preserve_reasoning_replay,
            );
            if is_local_provider(&provider.id)
                && !local_payload_requires_structured_tool_history(&attempt_body)
            {
                slim_local_attempt_body(&mut attempt_body);
            }

            if dry_run {
                let _ = self.release_provider_reservation(&provider.id).await;
                return Ok((
                    decision,
                    provider.id,
                    url,
                    0,
                    200,
                    serde_json::json!({"ok": true, "dry_run": true, "request": attempt_body}),
                ));
            }

            if provider.driver == "codex_responses" {
                let request_timeout = super::codex_responses_driver::codex_responses_timeout(
                    proxy_timeout_for_provider(&provider.id, &decision.execution_lane),
                );
                let codex_request =
                    match super::codex_responses_driver::build_codex_responses_request(
                        &provider,
                        &decision.model_id,
                        &attempt_body,
                    ) {
                        Ok(request) => request,
                        Err(err) => {
                            let err_msg = err.to_string();
                            let _ = self
                                .mark_provider_result(
                                    &provider.id,
                                    false,
                                    None,
                                    Some(err_msg.clone()),
                                )
                                .await;
                            push_proxy_attempt(
                                &mut attempts,
                                &decision,
                                &provider.id,
                                None,
                                "auth_error",
                                err_msg,
                            );
                            excluded.push(provider.id.clone());
                            if forced_provider_id.as_deref() == Some(provider.id.as_str()) {
                                break;
                            }
                            continue;
                        }
                    };
                let start = Instant::now();
                let client = self
                    .http_client_for(&provider.id, false, &decision.execution_lane)
                    .await?;
                let mut request = client
                    .post(&codex_request.url)
                    .timeout(request_timeout)
                    .bearer_auth(&codex_request.bearer)
                    .header("User-Agent", "codex_cli_rs/0.0.0 (Arda Charon)")
                    .header("originator", "codex_cli_rs")
                    .json(&codex_request.body);
                if let Some(account_id) = codex_request.chatgpt_account_id.as_deref() {
                    request = request.header("ChatGPT-Account-ID", account_id);
                }
                let outcome = match request.send().await {
                    Ok(response) => {
                        super::codex_responses_driver::response_to_codex_outcome(response).await
                    }
                    Err(err) => {
                        let err_msg =
                            format!("codex_responses request failed to {}: {err}", provider.id);
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let _ = self.record_tool_fit_observation(
                            &decision,
                            &routed_req,
                            &attempt_body,
                            ToolFitOutcome {
                                ok: false,
                                latency_ms: Some(latency_ms),
                                status_code: None,
                                outcome_class: "transport_failure".to_string(),
                                error: Some(err_msg.clone()),
                            },
                        );
                        let _ = self
                            .mark_provider_result(
                                &provider.id,
                                false,
                                Some(latency_ms),
                                Some(err_msg.clone()),
                            )
                            .await;
                        push_proxy_attempt(
                            &mut attempts,
                            &decision,
                            &provider.id,
                            None,
                            "transport_failure",
                            err_msg,
                        );
                        excluded.push(provider.id.clone());
                        if forced_provider_id.as_deref() == Some(provider.id.as_str()) {
                            break;
                        }
                        continue;
                    }
                };
                let latency_ms = start.elapsed().as_millis() as u64;
                let mut parsed = outcome.response;
                if (200..300).contains(&outcome.status) {
                    normalize_openai_response(&mut parsed);
                    attach_manwe_route_metadata(&mut parsed, &decision, &provider.id, latency_ms);
                    let semantic_outcome = classify_semantic_outcome(&parsed, &attempt_body);
                    let _ = self.record_tool_fit_observation(
                        &decision,
                        &routed_req,
                        &attempt_body,
                        ToolFitOutcome {
                            ok: !semantic_outcome.is_negative(),
                            latency_ms: Some(latency_ms),
                            status_code: Some(outcome.status),
                            outcome_class: semantic_outcome.as_str().to_string(),
                            error: None,
                        },
                    );
                    let _ = self
                        .mark_provider_result(&provider.id, true, Some(latency_ms), None)
                        .await;
                    let _ = self
                        .mark_model_result(
                            &provider.id,
                            &decision.model_id,
                            true,
                            Some(latency_ms),
                            None,
                        )
                        .await;
                    let _ = self.update_lane_fitness(
                        &decision.execution_lane,
                        &provider.id,
                        true,
                        Some(latency_ms),
                    );
                    return Ok((
                        decision,
                        provider.id.clone(),
                        codex_request.url,
                        latency_ms,
                        outcome.status,
                        parsed,
                    ));
                }
                let raw_err_msg = outcome.error.unwrap_or_else(|| {
                    format!("codex_responses {} HTTP {}", provider.id, outcome.status)
                });
                let client_payload_error = is_client_payload_error(outcome.status, &parsed);
                let outcome_class = if client_payload_error {
                    "client_payload_error".to_string()
                } else {
                    outcome_class_for_http_error(outcome.status, Some(&parsed))
                };
                let err_msg = format!(
                    "codex_responses {} HTTP {} {}: {}",
                    provider.id, outcome.status, outcome_class, raw_err_msg
                );
                let _ = self.record_tool_fit_observation(
                    &decision,
                    &routed_req,
                    &attempt_body,
                    ToolFitOutcome {
                        ok: false,
                        latency_ms: Some(latency_ms),
                        status_code: Some(outcome.status),
                        outcome_class: outcome_class.clone(),
                        error: Some(err_msg.clone()),
                    },
                );
                let model_scoped_error =
                    model_error_should_mark_unavailable(outcome.status, &parsed)
                        && provider_has_alternate_routable_model(
                            &provider,
                            &decision.model_id,
                            &req.task_type,
                            Some(&req),
                        );
                let request_scoped_retry_error =
                    is_request_scoped_retry_error(outcome.status, &parsed);
                if client_payload_error {
                    let _ = self
                        .mark_provider_client_error(
                            &provider.id,
                            Some(latency_ms),
                            Some(err_msg.clone()),
                        )
                        .await;
                } else if model_scoped_error || request_scoped_retry_error {
                    let _ = self.release_provider_reservation(&provider.id).await;
                    if request_scoped_retry_error {
                        if let Some(cooldown_seconds) = provider_error_immediate_cooldown_seconds(
                            &provider.id,
                            outcome.status,
                            &parsed,
                        ) {
                            let _ = self
                                .mark_provider_cooldown(&provider.id, cooldown_seconds)
                                .await;
                        }
                    }
                } else {
                    let _ = self
                        .mark_provider_result(
                            &provider.id,
                            false,
                            Some(latency_ms),
                            Some(err_msg.clone()),
                        )
                        .await;
                    if let Some(cooldown_seconds) = provider_error_immediate_cooldown_seconds(
                        &provider.id,
                        outcome.status,
                        &parsed,
                    ) {
                        let _ = self
                            .mark_provider_cooldown(&provider.id, cooldown_seconds)
                            .await;
                    }
                }
                if model_error_should_mark_unavailable(outcome.status, &parsed) {
                    let _ = self
                        .mark_model_result(
                            &provider.id,
                            &decision.model_id,
                            false,
                            Some(latency_ms),
                            Some(err_msg.clone()),
                        )
                        .await;
                }
                if !client_payload_error && !model_scoped_error && !request_scoped_retry_error {
                    let _ = self.update_lane_fitness(
                        &decision.execution_lane,
                        &provider.id,
                        false,
                        Some(latency_ms),
                    );
                }
                push_proxy_attempt(
                    &mut attempts,
                    &decision,
                    &provider.id,
                    Some(outcome.status),
                    outcome_class,
                    err_msg,
                );
                if !model_scoped_error {
                    excluded.push(provider.id.clone());
                } else {
                    push_excluded_model(&mut excluded_models, &decision.model_id);
                }
                if forced_provider_id.as_deref() == Some(provider.id.as_str())
                    && (!model_scoped_error || forced_model_id.is_some())
                {
                    break;
                }
                continue;
            }

            // Alternate driver: hermes_agent_cli. Bypasses HTTP entirely
            // and shells out to the local hermes CLI, which owns the
            // subscription-backed auth for anthropic and openai-codex.
            if provider.driver == "hermes_agent_cli" {
                let request_timeout =
                    proxy_timeout_for_provider(&provider.id, &decision.execution_lane);
                let outcome = super::hermes_cli_driver::invoke_hermes_cli(
                    &provider,
                    &decision.model_id,
                    &attempt_body,
                    request_timeout,
                )
                .await?;
                let mut parsed = outcome.response;
                if outcome.status == 200 {
                    normalize_openai_response(&mut parsed);
                    attach_manwe_route_metadata(
                        &mut parsed,
                        &decision,
                        &provider.id,
                        outcome.latency_ms,
                    );
                    let semantic_outcome = classify_semantic_outcome(&parsed, &attempt_body);
                    let _ = self.record_tool_fit_observation(
                        &decision,
                        &routed_req,
                        &attempt_body,
                        ToolFitOutcome {
                            ok: !semantic_outcome.is_negative(),
                            latency_ms: Some(outcome.latency_ms),
                            status_code: Some(outcome.status),
                            outcome_class: semantic_outcome.as_str().to_string(),
                            error: None,
                        },
                    );
                    let _ = self
                        .mark_provider_result(&provider.id, true, Some(outcome.latency_ms), None)
                        .await;
                    let _ = self
                        .mark_model_result(
                            &provider.id,
                            &decision.model_id,
                            true,
                            Some(outcome.latency_ms),
                            None,
                        )
                        .await;
                    let _ = self.update_lane_fitness(
                        &decision.execution_lane,
                        &provider.id,
                        true,
                        Some(outcome.latency_ms),
                    );
                    return Ok((
                        decision,
                        provider.id.clone(),
                        format!("hermes-cli://{}", provider.id),
                        outcome.latency_ms,
                        outcome.status,
                        parsed,
                    ));
                }
                // Non-200 — surface as a regular failure and let the
                // fallback machinery try the next candidate.
                let err_msg = outcome.error.clone().map_or_else(
                    || {
                        format!(
                            "hermes_agent_cli {} exit status {}",
                            provider.id, outcome.status
                        )
                    },
                    |error| {
                        format!(
                            "hermes_agent_cli {} exit status {}: {}",
                            provider.id, outcome.status, error
                        )
                    },
                );
                let _ = self
                    .mark_provider_result(
                        &provider.id,
                        false,
                        Some(outcome.latency_ms),
                        Some(err_msg.clone()),
                    )
                    .await;
                let _ = self.record_tool_fit_observation(
                    &decision,
                    &routed_req,
                    &attempt_body,
                    ToolFitOutcome {
                        ok: false,
                        latency_ms: Some(outcome.latency_ms),
                        status_code: Some(outcome.status),
                        outcome_class: outcome_class_for_http_error(outcome.status, Some(&parsed)),
                        error: Some(err_msg.clone()),
                    },
                );
                let outcome_class = outcome_class_for_http_error(outcome.status, Some(&parsed));
                push_proxy_attempt(
                    &mut attempts,
                    &decision,
                    &provider.id,
                    Some(outcome.status),
                    outcome_class,
                    format!("{err_msg}: {parsed}"),
                );
                excluded.push(provider.id.clone());
                routed_req.options["exclude_provider_ids"] = serde_json::json!(excluded);
                if forced_provider_id.as_deref() == Some(provider.id.as_str()) {
                    break;
                }
                continue;
            }

            let start = Instant::now();
            let client = self
                .http_client_for(&provider.id, false, &decision.execution_lane)
                .await?;
            let mut request = client.post(&url).json(&attempt_body);
            if let Some(env_key) = provider.api_key_env.as_deref() {
                if let Ok(key) = std::env::var(env_key) {
                    if !key.trim().is_empty() {
                        request = request.bearer_auth(key);
                    }
                }
            }

            let response = match request.send().await {
                Ok(response) => response,
                Err(err) => {
                    let err_msg = format!("proxy request failed to {}: {err}", provider.id);
                    let latency_ms = start.elapsed().as_millis() as u64;
                    let _ = self.record_tool_fit_observation(
                        &decision,
                        &routed_req,
                        &attempt_body,
                        ToolFitOutcome {
                            ok: false,
                            latency_ms: Some(latency_ms),
                            status_code: None,
                            outcome_class: "transport_failure".to_string(),
                            error: Some(err_msg.clone()),
                        },
                    );
                    let _ = self
                        .mark_provider_result(
                            &provider.id,
                            false,
                            Some(latency_ms),
                            Some(err_msg.clone()),
                        )
                        .await;
                    let _ = self
                        .mark_model_result(
                            &provider.id,
                            &decision.model_id,
                            false,
                            Some(latency_ms),
                            Some(err_msg.clone()),
                        )
                        .await;
                    let _ = self.update_lane_fitness(
                        &decision.execution_lane,
                        &provider.id,
                        false,
                        Some(latency_ms),
                    );
                    if transport_failure_should_trigger_cooldown(&provider.id, &err_msg)
                        && !provider_has_alternate_routable_model(
                            &provider,
                            &decision.model_id,
                            &req.task_type,
                            Some(&req),
                        )
                    {
                        let _ = self.mark_provider_cooldown(&provider.id, 300).await;
                    }
                    if !provider_has_alternate_routable_model(
                        &provider,
                        &decision.model_id,
                        &req.task_type,
                        Some(&req),
                    ) {
                        excluded.push(provider.id.clone());
                        routed_req.options["exclude_provider_ids"] = serde_json::json!(excluded);
                    }
                    push_proxy_attempt(
                        &mut attempts,
                        &decision,
                        &provider.id,
                        None,
                        "transport_failure",
                        err_msg,
                    );
                    if forced_provider_id.as_deref() == Some(provider.id.as_str()) {
                        break;
                    }
                    continue;
                }
            };
            let latency_ms = start.elapsed().as_millis() as u64;
            let status = response.status();
            let status_u16 = status.as_u16();
            let response_headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            let mut parsed = serde_json::from_str::<serde_json::Value>(&text)
                .unwrap_or_else(|_| serde_json::json!({"raw": text}));

            let _ = self
                .apply_provider_rate_limit_hints(&provider.id, &response_headers)
                .await;

            if status.is_success() {
                self.metrics().observe_proxy_latency(
                    &provider.id,
                    &decision.route_class,
                    latency_ms,
                );
                normalize_openai_response(&mut parsed);
                attach_manwe_route_metadata(&mut parsed, &decision, &provider.id, latency_ms);
                let semantic_outcome = classify_semantic_outcome(&parsed, &attempt_body);
                let _ = self.record_tool_fit_observation(
                    &decision,
                    &routed_req,
                    &attempt_body,
                    ToolFitOutcome {
                        ok: !semantic_outcome.is_negative(),
                        latency_ms: Some(latency_ms),
                        status_code: Some(status_u16),
                        outcome_class: semantic_outcome.as_str().to_string(),
                        error: None,
                    },
                );
                let _ = self
                    .mark_provider_result(&provider.id, true, Some(latency_ms), None)
                    .await;
                let _ = self
                    .mark_model_result(
                        &provider.id,
                        &decision.model_id,
                        true,
                        Some(latency_ms),
                        None,
                    )
                    .await;
                let _ = self.update_lane_fitness(
                    &decision.execution_lane,
                    &provider.id,
                    true,
                    Some(latency_ms),
                );
                self.emit_memory_event(
                    "proxy_success",
                    &format!(
                        "MANWE proxy forwarded via {} in {}ms",
                        provider.id, latency_ms
                    ),
                    Some(0.85),
                    vec!["manwe".to_string(), "proxy".to_string()],
                );
                return Ok((decision, provider.id, url, latency_ms, status_u16, parsed));
            }

            let err_msg = format!("provider {} HTTP {}", provider.id, status_u16);
            let client_payload_error = is_client_payload_error(status_u16, &parsed);
            let reasoning_replay_required = is_reasoning_replay_required_error(status_u16, &parsed);
            let outcome_class = if client_payload_error {
                "client_payload_error".to_string()
            } else if reasoning_replay_required {
                "payload_dialect_retry".to_string()
            } else {
                outcome_class_for_http_error(status_u16, Some(&parsed))
            };
            let _ = self.record_tool_fit_observation(
                &decision,
                &routed_req,
                &attempt_body,
                ToolFitOutcome {
                    ok: false,
                    latency_ms: Some(latency_ms),
                    status_code: Some(status_u16),
                    outcome_class: outcome_class.clone(),
                    error: Some(format!("{err_msg}: {parsed}")),
                },
            );
            let model_scoped_error = model_error_should_mark_unavailable(status_u16, &parsed)
                && provider_has_alternate_routable_model(
                    &provider,
                    &decision.model_id,
                    &req.task_type,
                    Some(&req),
                );
            let client_payload_model_scoped_error = client_payload_error
                && provider_has_alternate_routable_model(
                    &provider,
                    &decision.model_id,
                    &req.task_type,
                    Some(&req),
                );
            let request_scoped_retry_error = is_request_scoped_retry_error(status_u16, &parsed);
            if reasoning_replay_required && !preserve_reasoning_replay {
                let _ = self.release_provider_reservation(&provider.id).await;
                remember_reasoning_replay_route(
                    &mut reasoning_replay_routes,
                    &provider.id,
                    &decision.model_id,
                );
            } else if client_payload_error {
                let _ = self
                    .mark_provider_client_error(
                        &provider.id,
                        Some(latency_ms),
                        Some(format!("{err_msg}: {parsed}")),
                    )
                    .await;
                if client_payload_model_scoped_error {
                    let _ = self
                        .mark_model_result(
                            &provider.id,
                            &decision.model_id,
                            false,
                            Some(latency_ms),
                            Some(format!("{err_msg}: {parsed}")),
                        )
                        .await;
                }
            } else if model_scoped_error || request_scoped_retry_error {
                let _ = self.release_provider_reservation(&provider.id).await;
                if request_scoped_retry_error {
                    if let Some(cooldown_seconds) =
                        provider_error_immediate_cooldown_seconds(&provider.id, status_u16, &parsed)
                    {
                        let _ = self
                            .mark_provider_cooldown(&provider.id, cooldown_seconds)
                            .await;
                    }
                }
            } else {
                let _ = self
                    .mark_provider_result(
                        &provider.id,
                        false,
                        Some(latency_ms),
                        Some(err_msg.clone()),
                    )
                    .await;
                if let Some(cooldown_seconds) =
                    provider_error_immediate_cooldown_seconds(&provider.id, status_u16, &parsed)
                {
                    let _ = self
                        .mark_provider_cooldown(&provider.id, cooldown_seconds)
                        .await;
                }
            }
            if model_error_should_mark_unavailable(status_u16, &parsed) {
                let _ = self
                    .mark_model_result(
                        &provider.id,
                        &decision.model_id,
                        false,
                        Some(latency_ms),
                        Some(format!("{err_msg}: {parsed}")),
                    )
                    .await;
            }
            if !client_payload_error && !model_scoped_error && !request_scoped_retry_error {
                let _ = self.update_lane_fitness(
                    &decision.execution_lane,
                    &provider.id,
                    false,
                    Some(latency_ms),
                );
            }
            self.emit_memory_event(
                "proxy_failure",
                &format!("MANWE proxy failure via {}: {}", provider.id, err_msg),
                Some(0.4),
                vec![
                    "manwe".to_string(),
                    "proxy".to_string(),
                    "failure".to_string(),
                ],
            );
            if reasoning_replay_required && !preserve_reasoning_replay {
                push_proxy_attempt(
                    &mut attempts,
                    &decision,
                    &provider.id,
                    Some(status_u16),
                    outcome_class,
                    format!("{err_msg}: {parsed}"),
                );
                continue;
            }
            if provider_error_should_fallback(status_u16, &parsed) || client_payload_error {
                if model_scoped_error || client_payload_model_scoped_error {
                    push_excluded_model(&mut excluded_models, &decision.model_id);
                } else {
                    excluded.push(provider.id.clone());
                }
                push_proxy_attempt(
                    &mut attempts,
                    &decision,
                    &provider.id,
                    Some(status_u16),
                    outcome_class,
                    format!("{err_msg}: {parsed}"),
                );
                if forced_provider_id.as_deref() == Some(provider.id.as_str())
                    && ((!model_scoped_error && !client_payload_error) || forced_model_id.is_some())
                {
                    break;
                }
                continue;
            }
            return Err(ArdaError::Agent {
                agent: "manwe".to_string(),
                message: format!("{err_msg}: {parsed}"),
            });
        }

        self.record_proxy_fallback_chain(&req, &attempts, "proxy_exhausted");
        Err(ArdaError::Agent {
            agent: "manwe".to_string(),
            message: format_proxy_attempt_summary(&attempts),
        })
    }
}

fn outcome_class_for_http_error(status: u16, parsed: Option<&JsonValue>) -> String {
    if let Some(parsed) = parsed {
        if is_billing_or_credit_error(status, parsed) {
            return "billing_or_credit_error".to_string();
        }
        if model_error_should_mark_unavailable(status, parsed) {
            return "model_unavailable".to_string();
        }
        if is_context_overflow_error(status, parsed) {
            return "context_overflow".to_string();
        }
        if is_client_payload_error(status, parsed) {
            return "client_payload_error".to_string();
        }
    }
    match status {
        401 | 403 => "auth_error".to_string(),
        402 => "billing_or_credit_error".to_string(),
        404 => "not_found".to_string(),
        408 | 409 | 413 | 425 | 429 => "rate_or_retry_error".to_string(),
        500..=599 => "provider_server_error".to_string(),
        _ => "provider_http_error".to_string(),
    }
}

pub(crate) fn proxy_timeout_for_provider(provider_id: &str, execution_lane: &str) -> StdDuration {
    // Hermes tool iterations against the Beelink Ternary lane can spend close
    // to the generic local deadline producing the first tool call. Preserve
    // enough budget for the follow-up instead of cooling down a healthy lane.
    if provider_id == "edge_beelink_light" {
        return match execution_lane {
            "orchestrator" => StdDuration::from_secs(120),
            "execution" => StdDuration::from_secs(90),
            "background" => StdDuration::from_secs(60),
            _ => StdDuration::from_secs(90),
        };
    }
    if provider_id == "edge_backbone_coder" {
        let default_secs = match execution_lane {
            "orchestrator" => 420,
            "execution" => 360,
            "background" => 240,
            _ => 300,
        };
        return StdDuration::from_secs(
            std::env::var("ARDA_MANWE_EDGE_CODER_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|value| *value >= 60)
                .unwrap_or(default_secs),
        );
    }
    // edge_backbone* serves the largest local models on the dual RTX 2080S
    // backbone. CPU/GPU-staged coder lanes also need longer timeouts than
    // smaller direct local providers.
    if provider_id == "edge_backbone" {
        return match execution_lane {
            "orchestrator" => StdDuration::from_secs(120),
            "execution" => StdDuration::from_secs(90),
            "background" => StdDuration::from_secs(60),
            _ => StdDuration::from_secs(90),
        };
    }
    if is_local_provider(provider_id) {
        return match execution_lane {
            "orchestrator" => StdDuration::from_secs(30),
            "execution" => StdDuration::from_secs(20),
            "background" => StdDuration::from_secs(15),
            _ => StdDuration::from_secs(25),
        };
    }
    match execution_lane {
        "orchestrator" => StdDuration::from_secs(45),
        "execution" => StdDuration::from_secs(35),
        "background" => StdDuration::from_secs(25),
        _ => StdDuration::from_secs(40),
    }
}

pub(crate) fn provider_has_alternate_routable_model(
    provider: &ProviderState,
    current_model_id: &str,
    task_type: &str,
    req: Option<&ManweRequestEnvelope>,
) -> bool {
    provider.models.iter().any(|model| {
        model.id != current_model_id
            && model.healthy
            && !model.in_cooldown
            && (model.capable_tasks.iter().any(|task| task == task_type) || model.is_default)
            && super::model_supports_request(&provider.id, model, req)
    })
}
