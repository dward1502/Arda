//! Adaptive routing adapter for `manwe`.
//!
//! When the `adaptive` feature is enabled, callers may use
//! [`route_chat_completions()`] to run the ported charon routing logic.
//! Without `adaptive`, the adapter is a placeholder that returns the input
//! request unchanged so existing static gateway behavior is preserved.

use anyhow::{anyhow, Result};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct AdaptiveRoutingAdapter {
    _flag: (),
}

impl AdaptiveRoutingAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Route an OpenAI-compatible chat request.
    ///
    /// * `adaptive` feature: returns a routing decision envelope.
    /// * default feature: echoes the input payload back unchanged so the
    ///   gateway can keep using the static upstream path.
    pub fn route_chat_completions(&self, request: Value) -> Result<Value> {
        #[cfg(feature = "adaptive")]
        {
            let _ = request;
            Err(anyhow!("adaptive routing adapter not wired yet"))
        }

        #[cfg(not(feature = "adaptive"))]
        {
            let _ = self;
            Ok(request)
        }
    }
}
