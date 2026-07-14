// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::ingest::AthenaStore;
use annunimas_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AthenaDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for AthenaDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: expand_home("data/athena/athena.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5111),
        }
    }
}

pub struct AthenaDaemon {
    store: AthenaStore,
    config: AthenaDaemonConfig,
}

impl AthenaDaemon {
    pub fn new(store: AthenaStore, config: AthenaDaemonConfig) -> Self {
        Self { store, config }
    }

    pub async fn run(self) -> Result<()> {
        let store_for_ipc = self.store.clone();
        let socket_path = self.config.socket_path.clone();

        #[cfg(feature = "http")]
        {
            if self.config.http_enabled {
                let store_for_http = self.store.clone();
                let http_addr = self.config.http_addr.clone();

                let ipc_task =
                    tokio::spawn(
                        async move { ipc::run_ipc_server(store_for_ipc, socket_path).await },
                    );
                let http_task =
                    tokio::spawn(
                        async move { http::run_http_server(store_for_http, &http_addr).await },
                    );

                let (ipc_result, http_result) = tokio::join!(ipc_task, http_task);
                let ipc_inner = ipc_result.map_err(join_error)?;
                let http_inner = http_result.map_err(join_error)?;
                ipc_inner?;
                http_inner?;
                Ok(())
            } else {
                ipc::run_ipc_server(store_for_ipc, socket_path).await
            }
        }

        #[cfg(not(feature = "http"))]
        {
            ipc::run_ipc_server(store_for_ipc, socket_path).await
        }
    }
}

fn join_error(err: tokio::task::JoinError) -> annunimas_core::error::AnnunimasError {
    annunimas_core::error::AnnunimasError::Agent {
        agent: "athena".to_string(),
        message: format!("daemon task failed: {err}"),
    }
}

pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}
