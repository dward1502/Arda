use super::{proxy, CharonService};
use arda_core::error::{ArdaError, Result};
use std::sync::Arc;
use std::time::Duration as StdDuration;

#[derive(Clone, Eq, PartialEq, Hash)]
pub(crate) struct HttpClientKey {
    pub provider_id: String,
    pub is_stream: bool,
    pub execution_lane: String,
}

impl CharonService {
    /// Returns a cached reqwest::Client for the given (provider, mode, lane).
    /// Reusing clients preserves the connection pool and avoids the TLS
    /// handshake on every proxy call. Different modes need different timeout
    /// semantics: streaming uses connect_timeout + read_timeout (per-read),
    /// non-streaming uses connect_timeout + total .timeout(). Lane-tuned
    /// per `proxy_timeout_for_provider`.
    pub(crate) async fn http_client_for(
        &self,
        provider_id: &str,
        is_stream: bool,
        execution_lane: &str,
    ) -> Result<Arc<reqwest::Client>> {
        let key = HttpClientKey {
            provider_id: provider_id.to_string(),
            is_stream,
            execution_lane: execution_lane.to_string(),
        };
        if let Some(client) = self.http_clients.read().await.get(&key) {
            return Ok(client.clone());
        }
        let mut guard = self.http_clients.write().await;
        if let Some(client) = guard.get(&key) {
            return Ok(client.clone());
        }
        let timeout = proxy::proxy_timeout_for_provider(provider_id, execution_lane);
        let mut builder = reqwest::Client::builder().connect_timeout(StdDuration::from_secs(15));
        builder = if is_stream {
            builder.read_timeout(timeout)
        } else {
            builder.timeout(timeout)
        };
        if provider_id == "nvidia" {
            builder = builder.http1_only();
        }
        let client = builder.build().map_err(|err| ArdaError::Agent {
            agent: "charon".to_string(),
            message: format!("failed to build HTTP client: {err}"),
        })?;
        let client = Arc::new(client);
        guard.insert(key, client.clone());
        Ok(client)
    }
}
