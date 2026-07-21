// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::service::OracleService;
use arda_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct OracleDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for OracleDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: expand_home("data/oracle/oracle.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5120),
        }
    }
}

pub struct OracleDaemon {
    service: OracleService,
    config: OracleDaemonConfig,
}

impl OracleDaemon {
    pub fn new(service: OracleService, config: OracleDaemonConfig) -> Self {
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
                ipc_result.map_err(join_error)??;
                http_result.map_err(join_error)??;
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
        agent: "oracle".to_string(),
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
