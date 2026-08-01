// sigil: REPAIR
mod dispatch;
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::service::OracleService;
use arda_core::error::Result;
use std::future::Future;
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
        self.run_until(std::future::pending()).await
    }

    pub async fn run_until<F>(self, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        let service_for_ipc = self.service.clone();
        let socket_path = self.config.socket_path.clone();
        tokio::pin!(shutdown);

        #[cfg(feature = "http")]
        {
            if self.config.http_enabled {
                let service_for_http = self.service.clone();
                let http_addr = self.config.http_addr.clone();
                let mut ipc_task =
                    tokio::spawn(
                        async move { ipc::run_ipc_server(service_for_ipc, socket_path).await },
                    );
                let mut http_task = tokio::spawn(async move {
                    http::run_http_server(service_for_http, &http_addr).await
                });
                let result = tokio::select! {
                    result = &mut ipc_task => {
                        http_task.abort();
                        let _ = http_task.await;
                        result.map_err(join_error)?
                    }
                    result = &mut http_task => {
                        ipc_task.abort();
                        let _ = ipc_task.await;
                        result.map_err(join_error)?
                    }
                    () = &mut shutdown => {
                        ipc_task.abort();
                        http_task.abort();
                        let _ = tokio::join!(ipc_task, http_task);
                        Ok(())
                    }
                };
                self.service.drain_telemetry().await;
                result
            } else {
                tokio::select! {
                    result = ipc::run_ipc_server(service_for_ipc, socket_path) => result,
                    () = &mut shutdown => {
                        self.service.drain_telemetry().await;
                        Ok(())
                    }
                }
            }
        }

        #[cfg(not(feature = "http"))]
        {
            tokio::select! {
                result = ipc::run_ipc_server(service_for_ipc, socket_path) => result,
                () = &mut shutdown => {
                    self.service.drain_telemetry().await;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(feature = "http")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;
    use tokio::time::{sleep, timeout, Duration};

    #[tokio::test]
    async fn graceful_shutdown_removes_ipc_socket() {
        let temp = tempfile::tempdir().expect("tempdir");
        let socket_path = temp.path().join("oracle.sock");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let config = OracleDaemonConfig {
            socket_path: socket_path.clone(),
            http_enabled: false,
            http_addr: "127.0.0.1:0".to_string(),
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let daemon = tokio::spawn(OracleDaemon::new(service, config).run_until(async move {
            let _ = shutdown_rx.await;
        }));
        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
        assert!(socket_path.exists(), "IPC socket did not become ready");
        shutdown_tx.send(()).expect("shutdown receiver");
        timeout(Duration::from_secs(2), daemon)
            .await
            .expect("daemon shutdown timeout")
            .expect("daemon task")
            .expect("daemon result");
        assert!(
            !socket_path.exists(),
            "IPC socket must be removed on shutdown"
        );
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn listener_failure_cancels_ipc_sibling_and_removes_socket() {
        let temp = tempfile::tempdir().expect("tempdir");
        let socket_path = temp.path().join("oracle.sock");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let config = OracleDaemonConfig {
            socket_path: socket_path.clone(),
            http_enabled: true,
            http_addr: "invalid-http-address".to_string(),
        };
        let result = timeout(
            Duration::from_secs(2),
            OracleDaemon::new(service, config).run(),
        )
        .await
        .expect("daemon must return after listener failure");
        assert!(result.is_err());
        assert!(
            !socket_path.exists(),
            "failed sibling must not leave an IPC socket"
        );
    }
}
