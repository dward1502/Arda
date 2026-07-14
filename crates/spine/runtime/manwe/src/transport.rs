//! Transport interfaces for the gateway.
//!
//! Local crate-local placeholders avoid hard runtime dependencies during
//! incremental bootstrap. Real implementations are injected by surrounding
//! systems when those targets come online.

use std::future::Future;

use anyhow::Result;
use serde_json::Value;

/// Local inference call.
pub trait ApiTransport: Send + Sync {
    fn complete(
        &self,
        model: &str,
        request: Value,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>>;
}

/// Charon remote execution transport.
pub trait CharonTransport: Send + Sync {
    fn complete(
        &self,
        request: Value,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>>;
}

/// Runtime transport shim.
pub enum Transport {
    Local(Box<dyn ApiTransport + Send + Sync>),
    Charon(Box<dyn CharonTransport + Send + Sync>),
}
