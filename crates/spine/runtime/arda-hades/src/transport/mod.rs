// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::service::HadesService;
use arda_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct HadesDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for HadesDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: expand_home("data/hades/hades.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5112),
        }
    }
}

pub struct HadesDaemon {
    service: HadesService,
    config: HadesDaemonConfig,
}

impl HadesDaemon {
    pub fn new(service: HadesService, config: HadesDaemonConfig) -> Self {
        Self { service, config }
    }

    pub async fn run(self) -> Result<()> {
        let service_for_ipc = self.service.clone();
        let socket_path = self.config.socket_path.clone();
        let service_for_sweep = self.service.clone();
        let sweep_task = tokio::spawn(async move {
            loop {
                if let Err(err) = service_for_sweep.sweep("scheduled", None) {
                    tracing::debug!(error = %err, "HADES scheduled sweep failed");
                }
                tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
            }
        });

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
                sweep_task.abort();
                let ipc_inner = ipc_result.map_err(join_error)?;
                let http_inner = http_result.map_err(join_error)?;
                ipc_inner?;
                http_inner?;
                Ok(())
            } else {
                let out = ipc::run_ipc_server(service_for_ipc, socket_path).await;
                sweep_task.abort();
                out
            }
        }

        #[cfg(not(feature = "http"))]
        {
            let out = ipc::run_ipc_server(service_for_ipc, socket_path).await;
            sweep_task.abort();
            out
        }
    }
}

fn join_error(err: tokio::task::JoinError) -> arda_core::error::ArdaError {
    arda_core::error::ArdaError::Agent {
        agent: "hades".to_owned(),
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
