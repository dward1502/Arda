use arda_orome::provider::{
    DispatchPolicy, EdgeCommunicationPolicy, FleetScope, ManualTransport, ProviderAdapterError,
    ProviderConfig, ProviderRuntime, ProviderTransport, ProviderType, RoutingIntent, StreamEvent,
    TransportRequest,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn provider(id: &str) -> ProviderConfig {
    ProviderConfig {
        id: id.to_string(),
        kind: ProviderType::Http,
        name: id.to_string(),
        endpoint: format!("manual://{id}"),
        capabilities: vec!["streaming".to_string()],
    }
}

struct FlakyTransport {
    attempts: AtomicUsize,
}

#[async_trait]
impl ProviderTransport for FlakyTransport {
    async fn send(
        &self,
        _provider: &ProviderConfig,
        request: &TransportRequest,
    ) -> Result<Vec<StreamEvent>, ProviderAdapterError> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt == 1 {
            Err(ProviderAdapterError::new("temporary", "retry me", true))
        } else {
            Ok(ManualTransport::events_for(request))
        }
    }
}

struct SlowTransport;

#[async_trait]
impl ProviderTransport for SlowTransport {
    async fn send(
        &self,
        _provider: &ProviderConfig,
        _request: &TransportRequest,
    ) -> Result<Vec<StreamEvent>, ProviderAdapterError> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn retryable_provider_failure_is_retried_and_recorded() {
    let runtime =
        ProviderRuntime::new(vec![provider("edge-a")]).with_dispatch_policy(DispatchPolicy {
            timeout_ms: 100,
            max_attempts: 2,
            retry_backoff_ms: 0,
            max_fanout: 4,
        });
    let transport = FlakyTransport {
        attempts: AtomicUsize::new(0),
    };

    let receipt = runtime
        .dispatch(
            RoutingIntent::direct("edge-a"),
            TransportRequest::new("msg-1", "hello").streaming(true),
            &transport,
        )
        .await;

    assert!(receipt.succeeded());
    assert_eq!(receipt.receipts[0].attempts, 2);
    assert_eq!(receipt.receipts[0].chunks_sent, 1);
    assert!(receipt.receipts[0].streaming);
    assert_eq!(transport.attempts.load(Ordering::SeqCst), 2);
    let metrics = runtime.metrics();
    assert_eq!(metrics.attempts, 2);
    assert_eq!(metrics.retries, 1);
    assert_eq!(metrics.succeeded, 1);
}

#[tokio::test]
async fn provider_timeout_is_bounded_and_reported() {
    let runtime =
        ProviderRuntime::new(vec![provider("slow")]).with_dispatch_policy(DispatchPolicy {
            timeout_ms: 5,
            max_attempts: 2,
            retry_backoff_ms: 0,
            max_fanout: 4,
        });

    let receipt = runtime
        .dispatch(
            RoutingIntent::direct("slow"),
            TransportRequest::new("msg-timeout", "hello"),
            &SlowTransport,
        )
        .await;

    assert!(!receipt.succeeded());
    assert_eq!(receipt.receipts[0].attempts, 2);
    assert!(receipt.receipts[0].timed_out);
    assert_eq!(runtime.metrics().timed_out, 1);
}

#[tokio::test]
async fn expired_request_is_rejected_without_transport_attempt() {
    let runtime = ProviderRuntime::new(vec![provider("edge-a")]);
    let transport = FlakyTransport {
        attempts: AtomicUsize::new(0),
    };

    let receipt = runtime
        .dispatch(
            RoutingIntent::direct("edge-a"),
            TransportRequest::new("expired", "hello").with_ttl(Duration::ZERO),
            &transport,
        )
        .await;

    assert!(!receipt.succeeded());
    assert_eq!(receipt.receipts[0].attempts, 0);
    assert_eq!(
        receipt.receipts[0].error.as_deref(),
        Some("request_expired")
    );
    assert_eq!(transport.attempts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn fanout_is_parallel_bounded_and_observable() {
    let runtime = ProviderRuntime::new(vec![provider("edge-a"), provider("edge-b")])
        .with_dispatch_policy(DispatchPolicy {
            timeout_ms: 100,
            max_attempts: 1,
            retry_backoff_ms: 0,
            max_fanout: 2,
        });
    let transport = Arc::new(ManualTransport::test());

    let receipt = runtime
        .dispatch_shared(
            RoutingIntent::fanout(["edge-a", "edge-b"]),
            TransportRequest::new("fanout", "hello fleet").streaming(true),
            transport,
        )
        .await;

    assert!(receipt.succeeded());
    assert_eq!(receipt.receipts.len(), 2);
    assert_eq!(runtime.metrics().fanout_targets, 2);

    let rejected = runtime
        .dispatch(
            RoutingIntent::fanout(["edge-a", "edge-b", "edge-c"]),
            TransportRequest::new("too-wide", "hello fleet"),
            &ManualTransport::test(),
        )
        .await;
    assert_eq!(rejected.error.as_deref(), Some("fanout_limit_exceeded"));
    assert!(rejected.receipts.is_empty());
}

#[tokio::test]
async fn edge_policy_requires_approval_for_external_dispatch() {
    let runtime =
        ProviderRuntime::new(vec![provider("edge-a")]).with_edge_policy(EdgeCommunicationPolicy {
            allowed_scopes: vec![
                FleetScope::Local,
                FleetScope::TrustedFleet,
                FleetScope::External,
            ],
            require_external_approval: true,
        });

    let blocked = runtime
        .dispatch(
            RoutingIntent::direct("edge-a"),
            TransportRequest::new("external", "hello").for_scope(FleetScope::External),
            &ManualTransport::test(),
        )
        .await;
    assert_eq!(blocked.error.as_deref(), Some("external_approval_required"));

    let approved = runtime
        .dispatch(
            RoutingIntent::direct("edge-a"),
            TransportRequest::new("external-approved", "hello")
                .for_scope(FleetScope::External)
                .approved(true),
            &ManualTransport::test(),
        )
        .await;
    assert!(approved.succeeded());
}
