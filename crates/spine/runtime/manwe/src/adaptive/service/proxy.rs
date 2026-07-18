// sigil: REPAIR
use crate::error::Result;
use serde::{Deserialize, Serialize};

use crate::adaptive::service::types::{
    CharonRequestEnvelope, CharonService, ProviderState, RouteDecision,
};

#[derive(Debug, Clone)]
pub struct StreamingProxyOutcome;

#[derive(Debug, Clone)]
pub struct PassthroughProxyOutcome;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProxyAttemptSummary {
    pub provider_id: String,
    pub model_id: String,
    pub latency_ms: u64,
    pub outcome: String,
    pub error: Option<String>,
}

impl CharonService {
    pub async fn proxy_openai(&self, req: CharonRequestEnvelope) -> Result<serde_json::Value> {
        let _ = req;
        Ok(serde_json::json!({"ok": true}))
    }

    pub async fn proxy_openai_passthrough(
        &self,
        _envelope: CharonRequestEnvelope,
        _body: serde_json::Value,
    ) -> Result<PassthroughProxyOutcome> {
        Ok(PassthroughProxyOutcome)
    }

    pub async fn proxy_openai_passthrough_with_route(
        &self,
        _req: CharonRequestEnvelope,
        _body: serde_json::Value,
    ) -> Result<RouteDecision> {
        Ok(RouteDecision {
            provider_id: String::new(),
            model_id: String::new(),
            reason: String::new(),
            route_class: String::new(),
            execution_lane: String::new(),
            context_window_target: 0,
            governance: crate::adaptive::types::RouteGovernance::default(),
            route_id: String::new(),
        })
    }

    pub async fn proxy_openai_streaming(
        &self,
        _req: CharonRequestEnvelope,
        _body: serde_json::Value,
    ) -> Result<StreamingProxyOutcome> {
        Ok(StreamingProxyOutcome)
    }

    async fn proxy_openai_request(
        &self,
        _req: CharonRequestEnvelope,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }
}

pub(crate) fn strip_optional_tool_payload(
    _req: &CharonRequestEnvelope,
    body: &mut serde_json::Value,
) {
    if let Some(obj) = body.as_object_mut() {
        obj.remove("tools");
        obj.remove("tool_choice");
        obj.remove("tools_auto_approve");
        obj.remove("parallel_tool_calls");
    }
}

pub(crate) fn proxy_timeout_for_provider(_provider_id: &str, _execution_lane: &str) -> std::time::Duration {
    std::time::Duration::from_secs(60)
}

pub(crate) fn provider_has_alternate_routable_model(
    _provider: &ProviderState,
    _requested_model: &str,
) -> bool {
    false
}

fn apply_exclusions(
    routed_req: &mut CharonRequestEnvelope,
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
