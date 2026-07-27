// sigil: REPAIR
//! Receipt-backed HTTP JSON provider transport.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    FleetScope, ProviderAdapterError, ProviderConfig, ProviderTransport, ProviderType, StreamChunk,
    StreamEnded, StreamEvent, TransportOutcome, TransportRequest,
};

const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Concrete JSON-over-HTTP transport. Success requires a provider-assigned
/// message ID; an HTTP 2xx response alone is not delivery proof.
#[derive(Debug, Clone)]
pub struct HttpJsonTransport {
    client: reqwest::Client,
    max_response_bytes: usize,
}

impl Default for HttpJsonTransport {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("static HTTP transport client configuration must build"),
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

#[derive(Debug, Serialize)]
struct HttpTransportEnvelope<'a> {
    message_id: &'a str,
    payload: &'a str,
    streaming: bool,
    fleet_scope: FleetScope,
}

#[derive(Debug, Deserialize)]
struct HttpTransportReceipt {
    message_id: Option<String>,
    #[serde(default)]
    chunks: Vec<String>,
}

#[async_trait]
impl ProviderTransport for HttpJsonTransport {
    async fn send(
        &self,
        provider: &ProviderConfig,
        request: &TransportRequest,
    ) -> Result<TransportOutcome, ProviderAdapterError> {
        if provider.kind != ProviderType::Http {
            return Err(ProviderAdapterError::new(
                "unsupported_provider_kind",
                "HTTP transport requires an http provider",
                false,
            ));
        }
        if !provider.endpoint.starts_with("http://") && !provider.endpoint.starts_with("https://") {
            return Err(ProviderAdapterError::new(
                "invalid_provider_endpoint",
                "HTTP provider endpoint must use http or https",
                false,
            ));
        }

        let mut response = self
            .client
            .post(&provider.endpoint)
            .json(&HttpTransportEnvelope {
                message_id: &request.message_id,
                payload: &request.payload,
                streaming: request.streaming,
                fleet_scope: request.fleet_scope,
            })
            .send()
            .await
            .map_err(|error| {
                ProviderAdapterError::new(
                    "http_transport_error",
                    error.to_string(),
                    error.is_connect() || error.is_timeout(),
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ProviderAdapterError::new(
                "http_provider_status",
                format!("provider returned HTTP {status}"),
                status.is_server_error() || status.as_u16() == 429,
            ));
        }

        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(ProviderAdapterError::new(
                "http_receipt_too_large",
                format!(
                    "provider receipt exceeded {} bytes",
                    self.max_response_bytes
                ),
                false,
            ));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            ProviderAdapterError::new("http_receipt_read", error.to_string(), true)
        })? {
            if body.len() + chunk.len() > self.max_response_bytes {
                return Err(ProviderAdapterError::new(
                    "http_receipt_too_large",
                    format!(
                        "provider receipt exceeded {} bytes",
                        self.max_response_bytes
                    ),
                    false,
                ));
            }
            body.extend_from_slice(&chunk);
        }
        let receipt: HttpTransportReceipt = serde_json::from_slice(&body).map_err(|error| {
            ProviderAdapterError::new("invalid_provider_receipt", error.to_string(), false)
        })?;
        let provider_message_id = receipt
            .message_id
            .filter(|message_id| !message_id.trim().is_empty())
            .ok_or_else(|| {
                ProviderAdapterError::new(
                    "missing_provider_receipt",
                    "provider response did not contain a non-empty message_id",
                    false,
                )
            })?;

        let mut events = Vec::new();
        if request.streaming {
            events.extend(
                receipt
                    .chunks
                    .into_iter()
                    .enumerate()
                    .map(|(sequence, delta)| {
                        StreamEvent::Chunk(StreamChunk {
                            sequence: sequence as u64,
                            delta,
                            provider_metadata: serde_json::json!({
                                "transport": "http_json",
                                "provider_message_id": provider_message_id,
                            }),
                        })
                    }),
            );
            events.push(StreamEvent::Ended(StreamEnded {
                finished: true,
                reason: Some("provider_receipt".to_string()),
            }));
        }

        Ok(TransportOutcome {
            events,
            provider_message_id: Some(provider_message_id),
        })
    }
}
