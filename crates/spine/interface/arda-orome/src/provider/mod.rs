// sigil: REPAIR
//! Provider adapter registry and live streaming surface abstraction.

pub use adapter::{ProviderAdapter, ProviderAdapterError, ProviderCapabilities, ProviderKind};
#[cfg(feature = "service-runtime")]
pub use http::HttpJsonTransport;
pub use orchestration::{
    DispatchMetricsSnapshot, DispatchPolicy, EdgeCommunicationPolicy, FanoutReceipt, FleetScope,
    ManualTransport, ProviderTransport, RoutingIntent, TransportOutcome, TransportRequest,
};
pub use registry::{ProviderHandle, ProviderRegistry};
pub use runtime::{DispatchReceipt, ProviderConfig, ProviderRuntime, ProviderType};
pub use streaming::{StreamChunk, StreamEnded, StreamEvent, StreamSession, StreamingSurface};

pub mod adapter;
#[cfg(feature = "service-runtime")]
pub mod http;
pub mod orchestration;
pub mod registry;
pub mod runtime;
pub mod streaming;
#[cfg(test)]
pub mod tests;
