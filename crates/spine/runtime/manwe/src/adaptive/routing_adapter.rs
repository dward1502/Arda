//! Adaptive routing adapter for `manwe`.
//!
//! When enabled, this adapter builds a local provider catalog from
//! `crates/spine/runtime/manwe/src/adaptive/service/bootstrap_defaults`
//! and performs a lightweight selection/proxy flow against it.

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::adaptive::service::types::CharonService;

#[derive(Clone)]
pub struct AdaptiveRoutingAdapter {
    service: std::sync::Arc<CharonService>,
}

impl AdaptiveRoutingAdapter {
    pub fn new() -> Self {
        let service = std::sync::Arc::new(
            CharonService::new(".").expect("initialize governed adaptive routing service"),
        );
        Self { service }
    }

    /// Route an OpenAI-compatible chat request and return the complete response.
    pub fn route_chat_completions(&self, request: Value) -> Result<Value> {
        let _ = request;
        Err(anyhow!("adaptive routing adapter not wired yet"))
    }
}
