// sigil: REPAIR
//! Provider adapter/streaming tests.

use crate::provider::runtime::{DispatchReceipt, ProviderConfig, ProviderType};
use crate::provider::{
    ProviderAdapter, ProviderCapabilities, ProviderKind, ProviderRegistry, ProviderRuntime,
    StreamingSurface,
};

#[test]
fn runtime_from_defaults_contains_discord() {
    let runtime = ProviderRuntime::default();
    let provider = runtime.select("discord").expect("default discord provider");
    assert_eq!(provider.providers.len(), 1);
    assert_eq!(provider.providers[0].id, "discord");
}

#[test]
fn provider_config_from_kind_maps_canonical_kind() {
    let config = ProviderConfig {
        id: "edge-http-1".to_string(),
        kind: ProviderType::Http,
        name: "EdgeHTTP1".to_string(),
        endpoint: "http://edge-1/v1".to_string(),
        capabilities: Vec::new(),
    };
    let mut registry = ProviderRegistry::new();
    registry.register(ProviderAdapter::new(
        match config.kind {
            ProviderType::Discord => ProviderKind::Discord,
            ProviderType::Slack => ProviderKind::Slack,
            ProviderType::Http => ProviderKind::Http,
            ProviderType::Email => ProviderKind::Email,
            ProviderType::Matrix => ProviderKind::Matrix,
            ProviderType::Custom => ProviderKind::Custom,
        },
        &config.id,
        &config.name,
        ProviderCapabilities::new(true),
        &config.endpoint,
    ));

    assert!(registry.by_id("edge-http-1").is_some());
}

#[test]
fn dispatch_receipt_defaults_are_safe() {
    let receipt = DispatchReceipt::default();
    assert!(!receipt.dispatched);
    assert_eq!(receipt.attempts, 0);
    assert!(!receipt.streaming);
    assert_eq!(receipt.chunks_sent, 0);
    assert!(receipt.provider_id.is_empty());
    assert!(receipt.error.is_none());
}

#[test]
fn runtime_select_adapter_round_trip() {
    let mut registry = ProviderRegistry::new();
    registry.register(ProviderAdapter::new(
        ProviderKind::Http,
        "edge-http-1",
        "EdgeHTTP1",
        ProviderCapabilities::new(true),
        "http://edge-1/v1",
    ));
    registry.register(ProviderAdapter::new(
        ProviderKind::Slack,
        "edge-slack-1",
        "EdgeSlack1",
        ProviderCapabilities::new(true),
        "slack://edge-1",
    ));

    let runtime = ProviderRuntime::new(vec![
        ProviderConfig {
            id: "edge-http-1".to_string(),
            kind: ProviderType::Http,
            name: "EdgeHTTP1".to_string(),
            endpoint: "http://edge-1/v1".to_string(),
            capabilities: Vec::new(),
        },
        ProviderConfig {
            id: "edge-slack-1".to_string(),
            kind: ProviderType::Slack,
            name: "EdgeSlack1".to_string(),
            endpoint: "slack://edge-1".to_string(),
            capabilities: Vec::new(),
        },
    ]);

    let provider = runtime.select("edge-http-1").expect("selected provider");
    assert!(
        provider
            .providers
            .iter()
            .any(|entry| entry.id == "edge-http-1"),
        "selected provider should include edge-http-1 config"
    );
    assert!(
        registry.by_id("edge-http-1").is_some(),
        "registry should still resolve edge-http-1 adapter"
    );
    assert!(
        registry.by_id("edge-slack-1").is_some(),
        "registry should still resolve edge-slack-1 adapter"
    );
}

#[test]
fn registry_resolves_direct_capable_adapter() {
    let mut registry = ProviderRegistry::new();
    registry.register(ProviderAdapter::new(
        ProviderKind::Http,
        "edge-http-1",
        "EdgeHTTP1",
        ProviderCapabilities::new(true),
        "http://edge-1/v1",
    ));

    let handle = registry
        .resolve_direct_capable("edge-http-1")
        .expect("direct capable adapter");

    assert_eq!(handle.id, "edge-http-1");
    assert!(handle.capabilities.supports_direct);
}

#[test]
fn registry_returns_streaming_adapters() {
    let mut registry = ProviderRegistry::new();
    registry.register(ProviderAdapter::new(
        ProviderKind::Http,
        "stream-a",
        "StreamA",
        ProviderCapabilities::new(true),
        "http://stream-a/v1",
    ));
    registry.register(ProviderAdapter::new(
        ProviderKind::Http,
        "stream-b",
        "StreamB",
        ProviderCapabilities::new(true),
        "http://stream-b/v1",
    ));

    assert_eq!(registry.streaming_adapters().len(), 2);
}

#[test]
fn streaming_surface_records_chunks() {
    let mut surface = StreamingSurface::new("edge-http-1", "msg-123");
    surface
        .session
        .push_chunk(1, "hello", serde_json::json!({"token": "a"}));
    surface
        .session
        .push_chunk(2, "world", serde_json::json!({"token": "b"}));

    assert_eq!(surface.session().events.len(), 2);
}
