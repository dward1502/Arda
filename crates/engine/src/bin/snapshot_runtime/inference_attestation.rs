//! Validate gateway attestation before any successful body reaches the payload.
use super::Policy;
use anyhow::{bail, Context, Result};
use arda_engine::adapters::{InferenceRouteEvidence, ObservedInferenceRoute, MAX_INFERENCE_ROUTES};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct RouteLog(Arc<Mutex<InferenceRouteEvidence>>);

impl RouteLog {
    pub(super) fn new(profile_digest: String, policy: &Policy) -> Self {
        Self(Arc::new(Mutex::new(InferenceRouteEvidence {
            profile_digest,
            selected_provider: policy.provider.clone(),
            selected_model: policy.model.clone(),
            routes: Vec::new(),
        })))
    }

    pub(super) fn ensure_capacity(&self) -> Result<()> {
        if self.0.lock().unwrap().routes.len() >= MAX_INFERENCE_ROUTES {
            bail!("inference route evidence capacity exhausted");
        }
        Ok(())
    }

    pub(super) fn record(&self, headers: &serde_json::Map<String, Value>) -> Result<()> {
        self.ensure_capacity()?;
        let value = |key| {
            headers
                .get(key)
                .and_then(Value::as_str)
                .context("missing route evidence")
                .map(str::to_owned)
        };
        self.0.lock().unwrap().routes.push(ObservedInferenceRoute {
            route_id: value("x-manwe-route-id")?,
            provider_id: value("x-manwe-provider-id")?,
            model_id: value("x-manwe-model-id")?,
            route_class: value("x-manwe-route-class")?,
        });
        Ok(())
    }

    pub(super) fn snapshot(&self) -> InferenceRouteEvidence {
        self.0.lock().unwrap().clone()
    }
}

pub(super) fn verified_headers(
    response: &reqwest::Response,
    policy: &Policy,
) -> Result<serde_json::Map<String, Value>> {
    let mut headers = serde_json::Map::new();
    for key in ["route-id", "provider-id", "model-id", "route-class"] {
        let name = format!("x-manwe-{key}");
        let value = response
            .headers()
            .get(&name)
            .context("missing route evidence")?
            .to_str()?;
        if value.is_empty() || value.len() > 256 || !value.bytes().all(|b| (32..127).contains(&b)) {
            bail!("malformed route evidence");
        }
        if (key == "provider-id" && value != policy.provider)
            || (key == "model-id" && value != policy.model)
            || (key == "route-class" && value != "tool_oriented")
        {
            bail!("unapproved inference route");
        }
        headers.insert(name, json!(value));
    }
    Ok(headers)
}

#[cfg(test)]
#[test]
fn route_log_is_bounded_without_silent_truncation() {
    let policy = Policy {
        model: "model".into(),
        provider: "provider".into(),
        routing: json!({}),
    };
    let routes = RouteLog::new(format!("sha256:{}", "1".repeat(64)), &policy);
    let headers = json!({"x-manwe-route-id":"route","x-manwe-provider-id":"provider",
        "x-manwe-model-id":"model","x-manwe-route-class":"tool_oriented"});
    for _ in 0..MAX_INFERENCE_ROUTES {
        routes.record(headers.as_object().unwrap()).unwrap();
    }
    assert!(routes.snapshot().validate());
    assert!(routes.ensure_capacity().is_err());
    assert!(routes.record(headers.as_object().unwrap()).is_err());
    assert_eq!(routes.snapshot().routes.len(), MAX_INFERENCE_ROUTES);
}
