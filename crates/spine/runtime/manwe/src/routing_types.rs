/// Keeps Manwe's HTTP routing types in one place so the gRPC service can reuse
/// them without coupling manwe to tonic directly.
use axum::http::StatusCode;
use serde_json::Value;
use std::sync::Arc;

use crate::{ManweConfig, AdaptiveRoutingAdapter};

/// Shared state passed through the harness/router layer.
#[derive(Clone)]
pub struct ManweState {
    pub config: Arc<ManweConfig>,
    pub client: reqwest::Client,
    pub adapter: Arc<AdaptiveRoutingAdapter>,
    pub adaptive: bool,
}

/// Outcome of routing a chat-completions request.
pub struct RouteChatOutcome {
    pub upstream: String,
    pub req: Value,
}

impl RouteChatOutcome {
    pub fn new(upstream: impl Into<String>, req: Value) -> Self {
        Self {
            upstream: upstream.into(),
            req,
        }
    }
}

/// Liveness signal used by the typed health endpoint.
#[derive(Clone, Copy, Debug)]
pub struct HealthSignal(&'static str);

impl HealthSignal {
    pub const fn ok() -> Self { Self("ok") }
    pub fn as_str(self) -> &'static str { self.0 }
    pub fn status(self) -> StatusCode { StatusCode::OK }
}
