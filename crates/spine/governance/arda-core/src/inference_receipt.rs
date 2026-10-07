//! Durable successful-route evidence supplied by the retained supervisor, never stdout/stderr.
use serde::{Deserialize, Serialize};

pub const MAX_INFERENCE_ROUTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedInferenceRoute {
    pub route_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub route_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRouteEvidence {
    /// SHA-256 of the exact captured profile bytes read through the retained descriptor.
    pub profile_digest: String,
    pub selected_provider: String,
    pub selected_model: String,
    /// One entry per complete successful upstream response, in observation order.
    pub routes: Vec<ObservedInferenceRoute>,
}

impl InferenceRouteEvidence {
    pub fn validate(&self) -> bool {
        valid_digest(&self.profile_digest)
            && valid_identity(&self.selected_provider)
            && valid_identity(&self.selected_model)
            && !self.routes.is_empty()
            && self.routes.len() <= MAX_INFERENCE_ROUTES
            && self.routes.iter().all(|route| {
                valid_identity(&route.route_id)
                    && route.provider_id == self.selected_provider
                    && route.model_id == self.selected_model
                    && route.route_class == "tool_oriented"
            })
    }
}

fn valid_identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && value.bytes().all(|b| (32..127).contains(&b))
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

pub fn valid_receipt_routes(
    schema: &str,
    model: Option<&str>,
    evidence: Option<&InferenceRouteEvidence>,
) -> bool {
    match (schema, evidence) {
        ("arda.execution-receipt.v3", None) => true,
        ("arda.execution-receipt.v4", Some(evidence)) => {
            evidence.validate() && model == Some(evidence.selected_model.as_str())
        }
        _ => false,
    }
}
