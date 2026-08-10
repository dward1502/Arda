//! Provider-neutral council worker execution through the governed Manwë gateway.
//!
//! This module does not own run state or approval. It produces bounded opinion
//! receipts whose provenance can be validated against `arda-core::CouncilRun`.

use arda_core::council_run::{CouncilRoleKind, CouncilRun};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

const RECEIPT_SCHEMA_VERSION: &str = "arda.council-worker-receipt.v1";

#[derive(Debug, Clone)]
pub struct ManweCouncilConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub max_prompt_bytes: usize,
    pub max_context_items: usize,
    pub max_output_tokens: u32,
    pub preferred_local_provider: Option<String>,
    pub preferred_local_model: Option<String>,
}

impl Default for ManweCouncilConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:7171".into(),
            timeout: Duration::from_secs(30),
            max_prompt_bytes: 16 * 1024,
            max_context_items: 32,
            max_output_tokens: 768,
            preferred_local_provider: None,
            preferred_local_model: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouncilFallbackPolicy {
    LocalOnly,
    AllowHosted,
}

#[derive(Debug, Clone)]
pub struct CouncilWorkerRequest {
    pub run_id: String,
    pub node_id: String,
    pub worker_id: String,
    pub role: CouncilRoleKind,
    pub question: String,
    pub evidence_boundary: Vec<String>,
    pub fallback_policy: CouncilFallbackPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilOpinion {
    pub summary: String,
    pub confidence: f64,
    pub uncertainty: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilWorkerReceiptStatus {
    Succeeded,
    Unavailable,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilWorkerReceipt {
    pub schema_version: String,
    pub run_id: String,
    pub node_id: String,
    pub worker_id: String,
    pub role: CouncilRoleKind,
    pub status: CouncilWorkerReceiptStatus,
    pub opinion: Option<CouncilOpinion>,
    pub opinion_digest: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub route_id: Option<String>,
    pub route_class: Option<String>,
    pub latency_ms: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
    pub unavailable_reason: Option<String>,
    pub non_approval: bool,
}

impl CouncilWorkerReceipt {
    pub fn evidence_ref(&self) -> Option<String> {
        self.opinion_digest
            .as_ref()
            .map(|digest| format!("receipt:{}:{digest}", self.node_id))
    }
}

#[derive(Clone)]
pub struct ManweCouncilClient {
    config: ManweCouncilConfig,
    client: reqwest::Client,
}

impl ManweCouncilClient {
    pub fn new(config: ManweCouncilConfig) -> Result<Self, CouncilWorkerError> {
        if config.base_url.trim().is_empty()
            || config.max_prompt_bytes == 0
            || config.max_context_items == 0
            || config.max_output_tokens == 0
            || config.preferred_local_provider.is_some() != config.preferred_local_model.is_some()
        {
            return Err(CouncilWorkerError::InvalidConfig);
        }
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(CouncilWorkerError::Transport)?;
        Ok(Self { config, client })
    }

    pub async fn execute(&self, request: &CouncilWorkerRequest) -> CouncilWorkerReceipt {
        let started = Instant::now();
        match self.execute_inner(request).await {
            Ok(mut receipt) => {
                receipt.latency_ms = elapsed_ms(started);
                receipt
            }
            Err(error) => unavailable_receipt(request, elapsed_ms(started), error.to_string()),
        }
    }

    async fn execute_inner(
        &self,
        request: &CouncilWorkerRequest,
    ) -> Result<CouncilWorkerReceipt, CouncilWorkerError> {
        validate_request(request, &self.config)?;
        let health: Value = self
            .client
            .get(format!(
                "{}/healthz",
                self.config.base_url.trim_end_matches('/')
            ))
            .send()
            .await
            .map_err(CouncilWorkerError::Transport)?
            .error_for_status()
            .map_err(CouncilWorkerError::Transport)?
            .json()
            .await
            .map_err(CouncilWorkerError::Transport)?;
        if health.get("ok").and_then(Value::as_bool) != Some(true)
            || health
                .get("providers_healthy")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                == 0
        {
            return Err(CouncilWorkerError::Unavailable(
                "Manwë has no healthy provider".into(),
            ));
        }

        let capabilities: Value = self
            .client
            .get(format!(
                "{}/v1/capabilities",
                self.config.base_url.trim_end_matches('/')
            ))
            .send()
            .await
            .map_err(CouncilWorkerError::Transport)?
            .error_for_status()
            .map_err(CouncilWorkerError::Transport)?
            .json()
            .await
            .map_err(CouncilWorkerError::Transport)?;
        if capabilities
            .get("adaptive_routing")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(CouncilWorkerError::Unavailable(
                "Manwë adaptive routing is unavailable".into(),
            ));
        }
        let provider_capabilities: Value = self
            .client
            .get(format!(
                "{}/providers/capabilities",
                self.config.base_url.trim_end_matches('/')
            ))
            .send()
            .await
            .map_err(CouncilWorkerError::Transport)?
            .error_for_status()
            .map_err(CouncilWorkerError::Transport)?
            .json()
            .await
            .map_err(CouncilWorkerError::Transport)?;
        let eligible_local_candidates = eligible_local_candidates(&provider_capabilities);
        if request.fallback_policy == CouncilFallbackPolicy::LocalOnly
            && eligible_local_candidates.is_empty()
        {
            return Err(CouncilWorkerError::Unavailable(
                "Manwë reports no enabled, healthy local basic-chat provider".into(),
            ));
        }

        let prompt = bounded_prompt(request);
        let (origin, privacy_requirement) = match request.fallback_policy {
            CouncilFallbackPolicy::LocalOnly => ("local", "local_only"),
            CouncilFallbackPolicy::AllowHosted => ("local", "internal"),
        };
        let forced_local_candidate = if request.fallback_policy == CouncilFallbackPolicy::LocalOnly
        {
            match (
                self.config.preferred_local_provider.as_deref(),
                self.config.preferred_local_model.as_deref(),
            ) {
                (Some(provider), Some(model))
                    if eligible_local_candidates
                        .iter()
                        .any(|candidate| candidate == &(provider, model)) =>
                {
                    Some((provider, model))
                }
                (Some(provider), Some(_)) => {
                    return Err(CouncilWorkerError::Unavailable(format!(
                        "configured local provider `{provider}` is not eligible"
                    )))
                }
                _ => eligible_local_candidates.first().copied(),
            }
        } else {
            None
        };
        let body = json!({
            "model": "auto",
            "agent_id": request.worker_id,
            "messages": [
                {
                    "role": "system",
                    "content": "Return one JSON object only: summary must be a string; confidence must be a JSON number from 0 to 1; uncertainty must be a string; evidence_refs must be an array of strings. You are a read-only council critic. Never approve, execute, or claim operator authority."
                },
                {"role": "user", "content": prompt}
            ],
            "max_tokens": self.config.max_output_tokens,
            "temperature": 0.1,
            "stream": false,
            "allow_visible_reasoning": false,
            "allow_thinking_models": false,
            "tools": [],
            "routing": {
                "workload_role": role_name(request.role),
                "privacy_requirement": privacy_requirement,
                "inference_origin": origin,
                "origin_preference": origin,
                "execution_lane": "council_read_only",
                "context_window_target": self.config.max_prompt_bytes,
                "tool_use_required": false,
                "force_provider_id": forced_local_candidate.map(|candidate| candidate.0),
                "force_model_id": forced_local_candidate.map(|candidate| candidate.1),
                "allow_forced_provider_fallback": matches!(request.fallback_policy, CouncilFallbackPolicy::AllowHosted)
            }
        });
        let response = self
            .client
            .post(format!(
                "{}/v1/chat/completions",
                self.config.base_url.trim_end_matches('/')
            ))
            .json(&body)
            .send()
            .await
            .map_err(CouncilWorkerError::Transport)?
            .error_for_status()
            .map_err(CouncilWorkerError::Transport)?;
        let headers = response.headers().clone();
        let payload: Value = response
            .json()
            .await
            .map_err(CouncilWorkerError::Transport)?;
        let route_class = required_header(&headers, "x-manwe-route-class")?;
        let provider_id = required_header(&headers, "x-manwe-provider-id")?;
        let fallback_used = !eligible_local_candidates
            .iter()
            .any(|candidate| candidate.0 == provider_id);
        if fallback_used && request.fallback_policy == CouncilFallbackPolicy::LocalOnly {
            return Err(CouncilWorkerError::Unavailable(format!(
                "Manwë returned disallowed route class `{route_class}`"
            )));
        }
        let content = payload
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or(CouncilWorkerError::InvalidResponse(
                "missing message content",
            ))?;
        let opinion = parse_opinion_content(content)?;
        validate_opinion(&opinion)?;
        let opinion_digest = digest(&opinion)?;
        let usage = payload.get("usage").cloned().unwrap_or(Value::Null);
        Ok(CouncilWorkerReceipt {
            schema_version: RECEIPT_SCHEMA_VERSION.into(),
            run_id: request.run_id.clone(),
            node_id: request.node_id.clone(),
            worker_id: request.worker_id.clone(),
            role: request.role,
            status: if fallback_used {
                CouncilWorkerReceiptStatus::Degraded
            } else {
                CouncilWorkerReceiptStatus::Succeeded
            },
            opinion: Some(opinion),
            opinion_digest: Some(opinion_digest),
            provider_id: Some(provider_id),
            model_id: Some(required_header(&headers, "x-manwe-model-id")?),
            route_id: Some(required_header(&headers, "x-manwe-route-id")?),
            route_class: Some(route_class.clone()),
            latency_ms: 0,
            input_tokens: usage
                .get("prompt_tokens")
                .or_else(|| usage.get("input_tokens"))
                .and_then(Value::as_u64),
            output_tokens: usage
                .get("completion_tokens")
                .or_else(|| usage.get("output_tokens"))
                .and_then(Value::as_u64),
            fallback_used,
            fallback_reason: fallback_used.then(|| {
                format!("local route unavailable; Manwë disclosed `{route_class}` fallback")
            }),
            unavailable_reason: None,
            non_approval: true,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CouncilOperatorProjection {
    pub council_id: String,
    pub run_id: String,
    pub state: String,
    pub synthesis: String,
    pub material_tension: Option<String>,
    pub requested_decision: Option<String>,
    pub evidence_available: bool,
    pub non_approval: bool,
}

impl CouncilOperatorProjection {
    pub fn from_run(run: &CouncilRun) -> Self {
        Self {
            council_id: run.council_id.clone(),
            run_id: run.run_id.clone(),
            state: format!("{:?}", run.state).to_ascii_lowercase(),
            synthesis: run.synthesis.clone(),
            material_tension: run
                .material_tensions
                .iter()
                .find(|tension| !tension.resolved)
                .map(|tension| tension.summary.clone()),
            requested_decision: matches!(
                run.authority,
                arda_core::council_run::CouncilAuthority::HumanDecisionRequired
            )
            .then(|| run.escalation_recommendation.clone()),
            evidence_available: !run.evidence_boundary.is_empty()
                && run
                    .participants
                    .iter()
                    .all(|participant| !participant.evidence_refs.is_empty()),
            non_approval: run.non_approval,
        }
    }

    pub fn concise_message(&self) -> String {
        let mut parts = vec![format!("Council: {}", self.synthesis)];
        if let Some(tension) = &self.material_tension {
            parts.push(format!("Material tension: {tension}"));
        }
        if let Some(decision) = &self.requested_decision {
            parts.push(format!("Decision requested: {decision}"));
        }
        parts.push("Advisory only; operator approval has not been granted.".into());
        parts.join(" ")
    }
}

pub fn council_is_warranted(
    explicitly_requested: bool,
    material_risk_domains: usize,
    deterministic_check_sufficient: bool,
) -> bool {
    !deterministic_check_sufficient && (explicitly_requested || material_risk_domains >= 2)
}

fn validate_request(
    request: &CouncilWorkerRequest,
    config: &ManweCouncilConfig,
) -> Result<(), CouncilWorkerError> {
    if request.run_id.trim().is_empty()
        || request.node_id.trim().is_empty()
        || request.worker_id.trim().is_empty()
        || request.question.trim().is_empty()
        || request.evidence_boundary.is_empty()
    {
        return Err(CouncilWorkerError::InvalidRequest);
    }
    if request.question.len() > config.max_prompt_bytes
        || request.evidence_boundary.len() > config.max_context_items
    {
        return Err(CouncilWorkerError::BoundsExceeded);
    }
    Ok(())
}

fn bounded_prompt(request: &CouncilWorkerRequest) -> String {
    format!(
        "Council role: {}\nQuestion: {}\nEvidence boundary (cite only these references):\n{}",
        role_name(request.role),
        request.question,
        request.evidence_boundary.join("\n")
    )
}

fn role_name(role: CouncilRoleKind) -> &'static str {
    match role {
        CouncilRoleKind::Proposer => "planner_proposer",
        CouncilRoleKind::SecurityCritic => "security_privacy_critic",
        CouncilRoleKind::ImplementationCritic => "implementation_risk_critic",
        CouncilRoleKind::Adjudicator => "adjudicator",
    }
}

fn validate_opinion(opinion: &CouncilOpinion) -> Result<(), CouncilWorkerError> {
    if opinion.summary.trim().is_empty()
        || opinion.uncertainty.trim().is_empty()
        || opinion.evidence_refs.is_empty()
        || !opinion.confidence.is_finite()
        || !(0.0..=1.0).contains(&opinion.confidence)
    {
        return Err(CouncilWorkerError::InvalidResponse(
            "opinion fields failed validation",
        ));
    }
    Ok(())
}

fn parse_opinion_content(content: &str) -> Result<CouncilOpinion, CouncilWorkerError> {
    if let Ok(opinion) = serde_json::from_str(content.trim()) {
        return Ok(opinion);
    }
    let start = content
        .find('{')
        .ok_or(CouncilWorkerError::InvalidResponse("invalid opinion JSON"))?;
    let end = content
        .rfind('}')
        .filter(|end| *end >= start)
        .ok_or(CouncilWorkerError::InvalidResponse("invalid opinion JSON"))?;
    serde_json::from_str(&content[start..=end])
        .map_err(|_| CouncilWorkerError::InvalidResponse("invalid opinion JSON"))
}

fn eligible_local_candidates(payload: &Value) -> Vec<(&str, &str)> {
    payload
        .pointer("/capabilities/providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|provider| {
            provider.get("access_tier").and_then(Value::as_str) == Some("local")
                && provider.get("enabled").and_then(Value::as_bool) == Some(true)
        })
        .flat_map(|provider| {
            let provider_id = provider.get("provider_id").and_then(Value::as_str);
            provider
                .get("models")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|model| {
                    model.get("healthy").and_then(Value::as_bool) == Some(true)
                        && model
                            .pointer("/capabilities/basic_chat/state")
                            .and_then(Value::as_str)
                            == Some("passed")
                })
                .filter_map(move |model| {
                    Some((provider_id?, model.get("model_id").and_then(Value::as_str)?))
                })
        })
        .collect()
}

fn digest<T: Serialize>(value: &T) -> Result<String, CouncilWorkerError> {
    let bytes = serde_json::to_vec(value).map_err(CouncilWorkerError::Serialize)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn required_header(headers: &HeaderMap, name: &'static str) -> Result<String, CouncilWorkerError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or(CouncilWorkerError::MissingRouteProvenance(name))
}

fn unavailable_receipt(
    request: &CouncilWorkerRequest,
    latency_ms: u64,
    reason: String,
) -> CouncilWorkerReceipt {
    CouncilWorkerReceipt {
        schema_version: RECEIPT_SCHEMA_VERSION.into(),
        run_id: request.run_id.clone(),
        node_id: request.node_id.clone(),
        worker_id: request.worker_id.clone(),
        role: request.role,
        status: CouncilWorkerReceiptStatus::Unavailable,
        opinion: None,
        opinion_digest: None,
        provider_id: None,
        model_id: None,
        route_id: None,
        route_class: None,
        latency_ms,
        input_tokens: None,
        output_tokens: None,
        fallback_used: false,
        fallback_reason: None,
        unavailable_reason: Some(reason),
        non_approval: true,
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[derive(Debug, thiserror::Error)]
pub enum CouncilWorkerError {
    #[error("invalid Manwë council configuration")]
    InvalidConfig,
    #[error("invalid council worker request")]
    InvalidRequest,
    #[error("council prompt or context bounds exceeded")]
    BoundsExceeded,
    #[error("Manwë transport failed: {0}")]
    Transport(reqwest::Error),
    #[error("local council worker unavailable: {0}")]
    Unavailable(String),
    #[error("Manwë response omitted route provenance header `{0}`")]
    MissingRouteProvenance(&'static str),
    #[error("invalid Manwë council response: {0}")]
    InvalidResponse(&'static str),
    #[error("failed to serialize council opinion: {0}")]
    Serialize(serde_json::Error),
}
