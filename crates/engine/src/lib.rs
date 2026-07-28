//! arda-engine: the single dependency surface the `arda` daemon (and later the
//! launcher / HUD bridge) uses to reach system services. Re-exports the core
//! spine so callers import from `arda_engine` rather than reaching into the
//! vendored crates directly.

pub use arda_orome::provider::{
    DispatchMetricsSnapshot, DispatchReceipt, ManualTransport, ProviderConfig, ProviderRuntime,
    ProviderType, RoutingIntent, TransportRequest,
};
pub mod harness;
pub mod observability;
pub mod orome;
pub mod registry;
pub mod supervisor;
pub use arda_core::loop_observability;
pub use arda_core::service_registry;
