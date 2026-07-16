//! Transport interfaces for the gateway.
//!
//! Local crate-local placeholders avoid hard runtime dependencies during
//! incremental bootstrap. Real implementations are injected by surrounding
//! systems when those targets come online.

use std::future::Future;
use std::path::PathBuf;

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

// ---------------------------------------------------------------------------
// Phase 3 transport stubs: adaptive feature only
// ---------------------------------------------------------------------------

/// Minimal config for the adaptive HTTP transport.
#[derive(Debug, Clone, Default)]
pub struct HttpServerConfig {
    /// Bind address, e.g. `127.0.0.1:7172`.
    pub addr: String,
}

impl HttpServerConfig {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Adaptive HTTP transport shell.
///
/// * `adaptive` feature: starts a placeholder listener bound to `addr`.
/// * other builds: returns an explicit `Err` so call sites can opt out.
#[derive(Debug, Clone, Default)]
pub struct HttpServerTransport;

impl HttpServerTransport {
    pub async fn run(self, _config: HttpServerConfig) -> Result<()> {
        #[cfg(feature = "adaptive")]
        {
            let _listener = tokio::net::TcpListener::bind(&config.addr)
                .await
                .map_err(|e| anyhow::anyhow!("failed to bind adaptive HTTP listener: {e}"))?;
            tracing::info!(addr = %config.addr, "manwe adaptive HTTP transport started");
            loop {
                tokio::time::sleep(std::time::Duration::MAX).await;
            }
        }

        #[cfg(not(feature = "adaptive"))]
        Err(anyhow::anyhow!("adaptive HTTP transport disabled; enable the `adaptive` feature"))
    }
}

/// Adaptive IPC transport shell.
///
/// * `adaptive` feature: binds a unix socket at `socket_path`.
/// * other builds: returns an explicit `Err` so call sites can opt out.
#[derive(Debug, Clone, Default)]
pub struct IpcServerTransport;

impl IpcServerTransport {
    pub async fn run(self, _socket_path: PathBuf) -> Result<()> {
        #[cfg(feature = "adaptive")]
        {
            if let Some(parent) = socket_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
            }
            let _listener = tokio::net::UnixListener::bind(&socket_path)
                .map_err(|e| anyhow::anyhow!("failed to bind adaptive IPC socket: {e}"))?;
            tracing::info!(socket = %socket_path.display(), "manwe adaptive IPC transport started");
            loop {
                tokio::time::sleep(std::time::Duration::MAX).await;
            }
        }

        #[cfg(not(feature = "adaptive"))]
        Err(anyhow::anyhow!("adaptive IPC transport disabled; enable the `adaptive` feature"))
    }
}
