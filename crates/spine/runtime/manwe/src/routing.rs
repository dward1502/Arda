//! Adaptive routing feature gate for the `manwe` gateway binary.
//!
//! Without `adaptive`, the gateway keeps its frozen 7171 behavior and only
//! supports static upstream forwarding. With `adaptive`, chat completion
//! requests can pass through the Phase 1 routing adapter.

#[cfg(feature = "adaptive")]
mod adaptive {
    pub struct RouteDecision(pub serde_json::Value);

    impl RouteDecision {
        pub fn into_inner(self) -> serde_json::Value {
            self.0
        }
    }
}

#[cfg(feature = "adaptive")]
pub mod routing_adapter;
