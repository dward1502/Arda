// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::service::MnemosyneService;
use annunimas_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MnemosyneDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for MnemosyneDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: expand_home("data/mnemosyne/mnemosyne.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5115),
        }
    }
}

pub struct MnemosyneDaemon {
    service: MnemosyneService,
    config: MnemosyneDaemonConfig,
}

impl MnemosyneDaemon {
    pub fn new(service: MnemosyneService, config: MnemosyneDaemonConfig) -> Self {
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

fn join_error(err: tokio::task::JoinError) -> annunimas_core::error::AnnunimasError {
    annunimas_core::error::AnnunimasError::Agent {
        agent: "mnemosyne".to_owned(),
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
