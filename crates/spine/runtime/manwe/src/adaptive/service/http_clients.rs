use crate::adaptive::service::proxy;
use crate::adaptive::service::types::CharonService;
use arda_core::error::{ArdaError, Result};
use std::sync::Arc;
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct HttpClientCacheKey {
    pub provider_id: String,
    pub is_stream: bool,
    pub execution_lane: String,
}

#[derive(Debug, Clone, Default)]
pub struct HttpClientCache {
    clients: std::collections::BTreeMap<HttpClientCacheKey, Arc<reqwest::Client>>,
}

impl HttpClientCache {
    pub(crate) fn get(&self, key: &HttpClientCacheKey) -> Option<Arc<reqwest::Client>> {
        self.clients.get(key).cloned()
    }

    pub(crate) fn insert(&mut self, key: HttpClientCacheKey, client: Arc<reqwest::Client>) {
        self.clients.insert(key, client);
    }
}

impl CharonService {
    pub(crate) async fn http_client_for(
        &self,
        provider_id: &str,
        is_stream: bool,
        execution_lane: &str,
    ) -> Result<Arc<reqwest::Client>> {
        let key = HttpClientCacheKey {
            provider_id: provider_id.to_string(),
            is_stream,
            execution_lane: execution_lane.to_string(),
        };
        if let Some(cache) = self.http_clients.read().await.as_ref() {
            if let Some(client) = cache.get(&key) {
                return Ok(client.clone());
            }
        }
        let mut guard = self.http_clients.write().await;
        if let Some(cache) = guard.as_mut() {
            if let Some(client) = cache.get(&key) {
                return Ok(client.clone());
            }
            let timeout = proxy::proxy_timeout_for_provider(provider_id, execution_lane);
            let mut builder =
                reqwest::Client::builder().connect_timeout(StdDuration::from_secs(15));
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
            cache.insert(key, client.clone());
            Ok(client)
        } else {
            let timeout = proxy::proxy_timeout_for_provider(provider_id, execution_lane);
            let mut builder =
                reqwest::Client::builder().connect_timeout(StdDuration::from_secs(15));
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
            Ok(Arc::new(client))
        }
    }
}
