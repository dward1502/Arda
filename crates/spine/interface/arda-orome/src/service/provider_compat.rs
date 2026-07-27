use std::sync::Arc;

use arda_core::Result;

use crate::mcp::McpMessage;
use crate::provider::{
    DispatchPolicy, DispatchReceipt, ManualTransport, ProviderRuntime, RoutingIntent,
    TransportRequest,
};
use crate::types::OutboundMessage;

#[cfg(test)]
use crate::mcp::McpChannel;

impl ProviderRuntime {
    pub fn configured_provider_ids(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|provider| provider.id.clone())
            .collect()
    }

    /// Dispatches through the deterministic no-network transport while the
    /// service runtime remains an opt-in migration surface.
    pub async fn dispatch_with_retry(
        &self,
        message: &OutboundMessage,
        max_attempts: usize,
        retry_backoff_ms: u64,
    ) -> DispatchReceipt {
        #[cfg(test)]
        if let Some(channel) = self.test_channels.get(&message.provider) {
            return match channel.send_stream(&message.body, &message.channel).await {
                Ok(chunks_sent) => DispatchReceipt {
                    dispatched: true,
                    attempts: 1,
                    streaming: message.stream,
                    chunks_sent,
                    provider_id: message.provider.clone(),
                    provider_message_id: None,
                    error: None,
                    timed_out: false,
                },
                Err(error) => DispatchReceipt {
                    dispatched: false,
                    attempts: 1,
                    streaming: message.stream,
                    chunks_sent: 0,
                    provider_id: message.provider.clone(),
                    provider_message_id: None,
                    error: Some(error.to_string()),
                    timed_out: false,
                },
            };
        }

        let runtime = self.clone().with_dispatch_policy(DispatchPolicy {
            max_attempts,
            retry_backoff_ms,
            ..self.dispatch_policy
        });
        let request = TransportRequest::new(
            format!(
                "{}:{}:{}",
                message.provider, message.channel, message.created_at_utc
            ),
            message.body.clone(),
        )
        .streaming(message.stream);
        let receipt = runtime
            .dispatch_shared(
                RoutingIntent::direct(message.provider.clone()),
                request,
                Arc::new(ManualTransport::test()),
            )
            .await;

        receipt
            .receipts
            .into_iter()
            .next()
            .unwrap_or_else(|| DispatchReceipt {
                provider_id: message.provider.clone(),
                streaming: message.stream,
                error: receipt
                    .error
                    .or_else(|| Some("no dispatch receipt".to_string())),
                ..DispatchReceipt::default()
            })
    }

    /// Live inbound adapters are not part of the staged migration surface yet.
    pub async fn poll_once(&self) -> Result<Vec<(String, McpMessage)>> {
        Ok(Vec::new())
    }

    /// No provider is reported online until a concrete live transport exists.
    pub async fn online_provider_ids(&self) -> Vec<String> {
        #[cfg(test)]
        if !self.test_channels.is_empty() {
            let mut ids = self.test_channels.keys().cloned().collect::<Vec<_>>();
            ids.sort();
            return ids;
        }

        Vec::new()
    }

    pub async fn offline_provider_ids(&self) -> Vec<String> {
        self.configured_provider_ids()
    }

    #[cfg(test)]
    pub fn from_test_channels(
        providers: Vec<crate::provider::ProviderConfig>,
        channels: Vec<(String, Arc<dyn McpChannel>)>,
    ) -> Self {
        let mut runtime = Self::new(providers);
        runtime.test_channels = channels.into_iter().collect();
        runtime
    }
}
