// sigil: REPAIR

//! Adaptive routing adapter for `manwe`.
//!
//! This surface is always available; default builds expose a stub adapter,
//! while the `adaptive` feature swaps in the real adaptive implementation.

#[cfg(feature = "adaptive")]
pub use crate::adaptive::routing_adapter::AdaptiveRoutingAdapter;

#[cfg(not(feature = "adaptive"))]
use anyhow::anyhow;
#[cfg(not(feature = "adaptive"))]
use serde_json::Value;

#[cfg(not(feature = "adaptive"))]
#[derive(Debug, Clone)]
pub struct AdaptiveRoutingAdapter {
    _flag: (),
}

#[cfg(not(feature = "adaptive"))]
impl AdaptiveRoutingAdapter {
    pub fn new() -> Self {
        Self { _flag: () }
    }

    /// Route an OpenAI-compatible chat request and return the complete response.
    pub fn route_chat_completions(&self, request: Value) -> anyhow::Result<Value> {
        let _ = request;
        Err(anyhow!("adaptive routing adapter not wired yet"))
    }
}
