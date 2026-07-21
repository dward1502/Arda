//! Thin compatibility layer so the manually implemented manwe servers can
//! delegate to the same state/state adapters the axum handlers already use.

use crate::{ManweConfig, routing_types::ManweState, AdaptiveRoutingAdapter};

/// Derive the full `ManweState` from runtime inputs.
pub fn manwe_state(
    config: ManweConfig,
    adapter: AdaptiveRoutingAdapter,
    adaptive: bool,
) -> ManweState {
    ManweState {
        config: std::sync::Arc::new(config),
        client: reqwest::Client::new(),
        adapter: std::sync::Arc::new(adapter),
        adaptive,
    }
}
