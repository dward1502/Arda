// sigil: REPAIR
pub mod http;
pub mod ipc;

use crate::adaptive::service::ManweService;
use arda_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ManweDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for ManweDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: crate::config::arda_root().join("data/manwe/manwe.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 7171),
        }
    }
}

pub struct ManweDaemon {
    service: ManweService,
    config: ManweDaemonConfig,
}

impl ManweDaemon {
    pub fn new(service: ManweService, config: ManweDaemonConfig) -> Self {
        Self { service, config }
    }

    pub async fn run(self) -> Result<()> {
        let service_for_ipc = self.service.clone();
        let socket_path = self.config.socket_path.clone();
        let service_for_tick = self.service.clone();
        let tick_task = tokio::spawn(async move {
            loop {
                if let Err(err) = service_for_tick.tick_maintenance().await {
                    tracing::debug!(error = %err, "MANWE maintenance tick failed");
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });

        if self.config.http_enabled {
            let service_for_http = self.service.clone();
            let http_addr = self.config.http_addr.clone();
            let ipc_task =
                tokio::spawn(
                    async move { ipc::run_ipc_server(service_for_ipc, socket_path).await },
                );
            let http_task =
                tokio::spawn(
                    async move { http::run_http_server(service_for_http, &http_addr).await },
                );
            tokio::select! {
                ipc_result = ipc_task => {
                    tick_task.abort();
                    let ipc_inner = ipc_result.map_err(join_error)?;
                    ipc_inner?;
                    Ok(())
                }
                http_result = http_task => {
                    tick_task.abort();
                    let http_inner = http_result.map_err(join_error)?;
                    http_inner?;
                    Ok(())
                }
            }
        } else {
            let out = ipc::run_ipc_server(service_for_ipc, socket_path).await;
            tick_task.abort();
            out
        }
    }
}

#[deprecated(note = "use ManweDaemonConfig")]
pub type CharonDaemonConfig = ManweDaemonConfig;
#[deprecated(note = "use ManweDaemon")]
pub type CharonDaemon = ManweDaemon;

fn join_error(err: tokio::task::JoinError) -> arda_core::error::ArdaError {
    arda_core::error::ArdaError::Agent {
        agent: "manwe".to_string(),
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
