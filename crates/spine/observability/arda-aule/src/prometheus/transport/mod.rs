#![cfg(feature = "full-cli")]
// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::service::PrometheusService;
use arda_core::error::Result;
use std::path::{Path, PathBuf};

fn arda_aule_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_AULE_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Clone)]
pub struct PrometheusDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for PrometheusDaemonConfig {
    fn default() -> Self {
        let socket_path = arda_aule_root().join("data/prometheus/prometheus.sock");
        Self {
            socket_path,
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5113),
        }
    }
}

pub struct PrometheusDaemon {
    service: PrometheusService,
    config: PrometheusDaemonConfig,
}

impl PrometheusDaemon {
    pub fn new(service: PrometheusService, config: PrometheusDaemonConfig) -> Self {
        Self { service, config }
    }

    pub async fn run(self) -> Result<()> {
        let service_for_ipc = self.service.clone();
        let socket_path = self.config.socket_path.clone();

        #[cfg(feature = "http")]
        {
            if self.config.http_enabled {
                let service_for_http = self.service.clone();
                let http_addr = self.config.http_addr.clone();

                let ipc_task =
                    tokio::spawn(
                        async move { ipc::run_ipc_server(service_for_ipc, socket_path).await },
                    );
                let http_task = tokio::spawn(async move {
                    http::run_http_server(service_for_http, &http_addr).await
                });

                let (ipc_result, http_result) = tokio::join!(ipc_task, http_task);
                let ipc_inner = ipc_result.map_err(join_error)?;
                let http_inner = http_result.map_err(join_error)?;

                ipc_inner?;
                http_inner?;
                Ok(())
            } else {
                ipc::run_ipc_server(service_for_ipc, socket_path).await
            }
        }

        #[cfg(not(feature = "http"))]
        {
            ipc::run_ipc_server(service_for_ipc, socket_path).await
        }
    }
}

fn join_error(err: tokio::task::JoinError) -> arda_core::error::ArdaError {
    arda_core::error::ArdaError::Agent {
        agent: "prometheus".to_string(),
        message: format!("daemon task failed: {err}"),
    }
}
