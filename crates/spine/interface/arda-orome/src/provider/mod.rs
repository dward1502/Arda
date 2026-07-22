// sigil: REPAIR
//! Provider adapter registry and live streaming surface abstraction.

pub mod adapter;
pub mod registry;
pub mod streaming;
pub mod tests;

pub use adapter::{ProviderAdapter, ProviderAdapterError, ProviderCapabilities, ProviderKind};
pub use registry::{ProviderHandle, ProviderRegistry};
pub use streaming::{StreamChunk, StreamEnded, StreamEvent, StreamSession, StreamingSurface};
