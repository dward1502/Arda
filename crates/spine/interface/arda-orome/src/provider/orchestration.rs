// sigil: REPAIR
//! Bounded provider dispatch, retry, streaming, and fanout orchestration.

use super::{
    DispatchReceipt, ProviderAdapterError, ProviderConfig, ProviderRuntime, StreamChunk,
    StreamEnded, StreamEvent,
};
use async_trait::async_trait;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchPolicy {
    pub timeout_ms: u64,
    pub max_attempts: usize,
    pub retry_backoff_ms: u64,
    pub max_fanout: usize,
}

impl Default for DispatchPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            max_attempts: 3,
            retry_backoff_ms: 100,
            max_fanout: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FleetScope {
    Local,
    TrustedFleet,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeCommunicationPolicy {
    pub allowed_scopes: Vec<FleetScope>,
    /// Provider IDs explicitly trusted for fleet-scoped network dispatch.
    #[serde(default)]
    pub trusted_fleet_provider_ids: Vec<String>,
    pub require_external_approval: bool,
}

impl Default for EdgeCommunicationPolicy {
    fn default() -> Self {
        Self {
            allowed_scopes: vec![FleetScope::Local],
            trusted_fleet_provider_ids: Vec::new(),
            require_external_approval: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoutingIntent {
    Direct { provider_id: String },
    Fanout { provider_ids: Vec<String> },
}

impl RoutingIntent {
    pub fn direct(provider_id: impl Into<String>) -> Self {
        Self::Direct {
            provider_id: provider_id.into(),
        }
    }

    pub fn fanout<I, S>(provider_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Fanout {
            provider_ids: provider_ids.into_iter().map(Into::into).collect(),
        }
    }

    fn targets(self) -> Vec<String> {
        match self {
            Self::Direct { provider_id } => vec![provider_id],
            Self::Fanout { provider_ids } => provider_ids,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransportRequest {
    pub message_id: String,
    pub payload: String,
    pub streaming: bool,
    pub fleet_scope: FleetScope,
    pub approved: bool,
    expires_at: Option<Instant>,
}

impl TransportRequest {
    pub fn new(message_id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            payload: payload.into(),
            streaming: false,
            fleet_scope: FleetScope::Local,
            approved: false,
            expires_at: None,
        }
    }

    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.expires_at = Some(Instant::now() + ttl);
        self
    }

    pub fn for_scope(mut self, scope: FleetScope) -> Self {
        self.fleet_scope = scope;
        self
    }

    pub fn approved(mut self, approved: bool) -> Self {
        self.approved = approved;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| Instant::now() >= expires_at)
    }
}

#[async_trait]
pub trait ProviderTransport: Send + Sync {
    async fn send(
        &self,
        provider: &ProviderConfig,
        request: &TransportRequest,
    ) -> Result<TransportOutcome, ProviderAdapterError>;
}

/// Transport evidence returned to the bounded dispatcher.
#[derive(Debug, Clone, Default)]
pub struct TransportOutcome {
    pub events: Vec<StreamEvent>,
    /// Provider-assigned ID proving that a live transport accepted the message.
    pub provider_message_id: Option<String>,
}

/// Deterministic no-network transport used by engine/CLI smoke probes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ManualTransport;

impl ManualTransport {
    pub fn test() -> Self {
        Self
    }

    pub fn events_for(request: &TransportRequest) -> Vec<StreamEvent> {
        if !request.streaming {
            return Vec::new();
        }
        vec![
            StreamEvent::Chunk(StreamChunk {
                sequence: 0,
                delta: request.payload.clone(),
                provider_metadata: serde_json::json!({"transport": "manual"}),
            }),
            StreamEvent::Ended(StreamEnded {
                finished: true,
                reason: Some("manual_complete".to_string()),
            }),
        ]
    }
}

#[async_trait]
impl ProviderTransport for ManualTransport {
    async fn send(
        &self,
        _provider: &ProviderConfig,
        request: &TransportRequest,
    ) -> Result<TransportOutcome, ProviderAdapterError> {
        Ok(TransportOutcome {
            events: Self::events_for(request),
            provider_message_id: None,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutReceipt {
    pub receipts: Vec<DispatchReceipt>,
    pub error: Option<String>,
}

impl FanoutReceipt {
    pub fn succeeded(&self) -> bool {
        self.error.is_none()
            && !self.receipts.is_empty()
            && self.receipts.iter().all(|receipt| receipt.dispatched)
    }
}

#[derive(Debug, Default)]
pub(crate) struct DispatchMetrics {
    attempts: AtomicUsize,
    retries: AtomicUsize,
    succeeded: AtomicUsize,
    failed: AtomicUsize,
    timed_out: AtomicUsize,
    fanout_targets: AtomicUsize,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchMetricsSnapshot {
    pub attempts: usize,
    pub retries: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub fanout_targets: usize,
}

impl DispatchMetrics {
    fn snapshot(&self) -> DispatchMetricsSnapshot {
        DispatchMetricsSnapshot {
            attempts: self.attempts.load(Ordering::Relaxed),
            retries: self.retries.load(Ordering::Relaxed),
            succeeded: self.succeeded.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            timed_out: self.timed_out.load(Ordering::Relaxed),
            fanout_targets: self.fanout_targets.load(Ordering::Relaxed),
        }
    }
}

impl ProviderRuntime {
    pub fn with_dispatch_policy(mut self, policy: DispatchPolicy) -> Self {
        self.dispatch_policy = policy;
        self
    }

    pub fn with_edge_policy(mut self, policy: EdgeCommunicationPolicy) -> Self {
        self.edge_policy = policy;
        self
    }

    pub fn metrics(&self) -> DispatchMetricsSnapshot {
        self.dispatch_metrics.snapshot()
    }

    pub async fn dispatch<T: ProviderTransport + ?Sized>(
        &self,
        intent: RoutingIntent,
        request: TransportRequest,
        transport: &T,
    ) -> FanoutReceipt {
        let targets = intent.targets();
        if let Some(error) = self.validate_targets(&targets, &request) {
            return FanoutReceipt {
                receipts: Vec::new(),
                error: Some(error),
            };
        }
        self.dispatch_metrics
            .fanout_targets
            .fetch_add(targets.len(), Ordering::Relaxed);
        let mut receipts = Vec::with_capacity(targets.len());
        for provider_id in targets {
            receipts.push(self.dispatch_one(&provider_id, &request, transport).await);
        }
        FanoutReceipt {
            receipts,
            error: None,
        }
    }

    pub async fn dispatch_shared<T: ProviderTransport + 'static>(
        &self,
        intent: RoutingIntent,
        request: TransportRequest,
        transport: Arc<T>,
    ) -> FanoutReceipt {
        let targets = intent.targets();
        if let Some(error) = self.validate_targets(&targets, &request) {
            return FanoutReceipt {
                receipts: Vec::new(),
                error: Some(error),
            };
        }
        self.dispatch_metrics
            .fanout_targets
            .fetch_add(targets.len(), Ordering::Relaxed);
        let futures = targets.iter().map(|provider_id| {
            let transport = Arc::clone(&transport);
            let request = request.clone();
            async move {
                self.dispatch_one(provider_id, &request, transport.as_ref())
                    .await
            }
        });
        FanoutReceipt {
            receipts: join_all(futures).await,
            error: None,
        }
    }

    fn validate_targets(&self, targets: &[String], request: &TransportRequest) -> Option<String> {
        if targets.is_empty() {
            Some("no_provider_targets".to_string())
        } else if targets.len() > self.dispatch_policy.max_fanout.max(1) {
            Some("fanout_limit_exceeded".to_string())
        } else if !self
            .edge_policy
            .allowed_scopes
            .contains(&request.fleet_scope)
        {
            Some("fleet_scope_not_allowed".to_string())
        } else if request.fleet_scope == FleetScope::TrustedFleet
            && targets.iter().any(|provider_id| {
                !self
                    .edge_policy
                    .trusted_fleet_provider_ids
                    .contains(provider_id)
            })
        {
            Some("trusted_fleet_target_not_allowed".to_string())
        } else if request.fleet_scope == FleetScope::External
            && self.edge_policy.require_external_approval
            && !request.approved
        {
            Some("external_approval_required".to_string())
        } else {
            None
        }
    }

    async fn dispatch_one<T: ProviderTransport + ?Sized>(
        &self,
        provider_id: &str,
        request: &TransportRequest,
        transport: &T,
    ) -> DispatchReceipt {
        let mut receipt = DispatchReceipt {
            provider_id: provider_id.to_string(),
            streaming: request.streaming,
            ..DispatchReceipt::default()
        };
        if request.is_expired() {
            receipt.error = Some("request_expired".to_string());
            self.dispatch_metrics.failed.fetch_add(1, Ordering::Relaxed);
            return receipt;
        }
        let Some(provider) = self.providers.iter().find(|item| item.id == provider_id) else {
            receipt.error = Some("unknown_provider".to_string());
            self.dispatch_metrics.failed.fetch_add(1, Ordering::Relaxed);
            return receipt;
        };
        let max_attempts = self.dispatch_policy.max_attempts.max(1);
        for attempt in 1..=max_attempts {
            receipt.attempts = attempt;
            self.dispatch_metrics
                .attempts
                .fetch_add(1, Ordering::Relaxed);
            let result = tokio::time::timeout(
                Duration::from_millis(self.dispatch_policy.timeout_ms.max(1)),
                transport.send(provider, request),
            )
            .await;
            match result {
                Ok(Ok(outcome)) => {
                    receipt.dispatched = true;
                    receipt.provider_message_id = outcome.provider_message_id;
                    receipt.chunks_sent = outcome
                        .events
                        .iter()
                        .filter(|event| matches!(event, StreamEvent::Chunk(_)))
                        .count();
                    self.dispatch_metrics
                        .succeeded
                        .fetch_add(1, Ordering::Relaxed);
                    return receipt;
                }
                Ok(Err(error)) => {
                    receipt.error = Some(format!("{}: {}", error.code, error.message));
                    if !error.retryable || attempt == max_attempts {
                        break;
                    }
                }
                Err(_) => {
                    receipt.timed_out = true;
                    receipt.error = Some("provider_timeout".to_string());
                    if attempt == max_attempts {
                        break;
                    }
                }
            }
            self.dispatch_metrics
                .retries
                .fetch_add(1, Ordering::Relaxed);
            if self.dispatch_policy.retry_backoff_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.dispatch_policy.retry_backoff_ms))
                    .await;
            }
            if request.is_expired() {
                receipt.error = Some("request_expired".to_string());
                break;
            }
        }
        self.dispatch_metrics.failed.fetch_add(1, Ordering::Relaxed);
        if receipt.timed_out {
            self.dispatch_metrics
                .timed_out
                .fetch_add(1, Ordering::Relaxed);
        }
        receipt
    }
}
