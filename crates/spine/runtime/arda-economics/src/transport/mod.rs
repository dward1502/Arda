// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod finance_stream;
pub mod ipc;

use crate::service::PlutusService;
use arda_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PlutusDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for PlutusDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: arda_core::layout::arda_root_from(env!("CARGO_MANIFEST_DIR"))
                .join("data/plutus/plutus.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5119),
        }
    }
}

pub struct PlutusDaemon {
    service: PlutusService,
    config: PlutusDaemonConfig,
}

impl PlutusDaemon {
    pub fn new(service: PlutusService, config: PlutusDaemonConfig) -> Self {
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

#[cfg(feature = "http")]
fn join_error(err: tokio::task::JoinError) -> arda_core::error::ArdaError {
    arda_core::error::ArdaError::Agent {
        agent: "plutus".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::{PlutusDaemon, PlutusDaemonConfig};
    use crate::transport::ipc::send_command;
    use crate::PlutusService;
    use serde_json::json;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn daemon_serves_ipc_status_when_http_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket_path = dir.path().join("plutus.sock");
        let service = PlutusService::from_home(dir.path()).expect("service");
        let daemon = PlutusDaemon::new(
            service,
            PlutusDaemonConfig {
                socket_path: socket_path.clone(),
                http_enabled: false,
                http_addr: "127.0.0.1:0".to_owned(),
            },
        );

        let daemon_task = tokio::spawn(async move { daemon.run().await });
        sleep(Duration::from_millis(50)).await;

        let status = send_command(socket_path, "status", json!({})).await;
        let status = match status {
            Ok(status) => status,
            Err(err) => {
                let message = err.to_string();
                if message.contains("Operation not permitted")
                    || message.contains("Permission denied")
                {
                    daemon_task.abort();
                    return;
                }
                panic!("status: {message}");
            }
        };
        assert_eq!(status["authority"], "plutus_service");

        daemon_task.abort();
    }
}
