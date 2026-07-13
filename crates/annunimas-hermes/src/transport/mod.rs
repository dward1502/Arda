// sigil: REPAIR
#[cfg(feature = "http")]
pub mod http;
pub mod ipc;

use crate::context_enrichment::{ContextEnrichmentService, ScoringWeights};
use crate::service::HermesService;
use annunimas_core::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct HermesDaemonConfig {
    pub socket_path: PathBuf,
    pub http_enabled: bool,
    pub http_addr: String,
}

impl Default for HermesDaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: expand_home("data/hermes/hermes.sock"),
            http_enabled: true,
            http_addr: format!("{}:{}", "127.0.0.1", 5117),
        }
    }
}

pub struct HermesDaemon {
    service: HermesService,
    config: HermesDaemonConfig,
}

impl HermesDaemon {
    pub fn new(service: HermesService, config: HermesDaemonConfig) -> Self {
        Self { service, config }
    }

    /// Preload common context queries to warm up the cache
    /// This reduces first-message latency by loading context before the daemon starts listening
    async fn preload_context(&self) -> Result<()> {
        let scoring_weights = ScoringWeights::default();
        let enrichment = ContextEnrichmentService::new(scoring_weights);

        // Warm up context cache with common queries that Hermes might receive
        let common_queries = vec![
            "council session",
            "message classification",
            "boardroom message",
            "task status",
            "agent health",
        ];

        for query in common_queries {
            if let Err(err) = enrichment.enrich_prompt(query) {
                tracing::debug!(error = %err, "failed to preload context for query: {}", query);
                // Continue with other queries even if one fails
            } else {
                tracing::debug!("preloaded context for query: {}", query);
            }

            // Small delay to prevent overwhelming Mnemosyne on startup
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        // Preload context during startup to reduce first-message latency
        if let Err(err) = self.preload_context().await {
            tracing::warn!(error = %err, "failed to preload Hermes context during startup");
            // Continue anyway - this is an optimization, not a requirement
        }

        let service_for_ipc = self.service.clone();
        let socket_path = self.config.socket_path.clone();
        let service_for_poll = self.service.clone();
        let poll_task = tokio::spawn(async move {
            loop {
                if let Err(err) = service_for_poll.poll_providers_once().await {
                    tracing::debug!(error = %err, "HERMES provider poll failed");
                }
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
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
                poll_task.abort();
                let ipc_inner = ipc_result.map_err(join_error)?;
                let http_inner = http_result.map_err(join_error)?;
                ipc_inner?;
                http_inner?;
                Ok(())
            } else {
                let out = ipc::run_ipc_server(service_for_ipc, socket_path).await;
                poll_task.abort();
                out
            }
        }

        #[cfg(not(feature = "http"))]
        {
            let out = ipc::run_ipc_server(service_for_ipc, socket_path).await;
            poll_task.abort();
            out
        }
    }
}

fn join_error(err: tokio::task::JoinError) -> annunimas_core::error::AnnunimasError {
    annunimas_core::error::AnnunimasError::Agent {
        agent: "hermes".to_string(),
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
