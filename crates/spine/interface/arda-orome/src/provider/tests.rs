// sigil: REPAIR
//! Provider adapter/streaming tests.

use crate::provider::{
    ProviderAdapter, ProviderCapabilities, ProviderKind, ProviderRegistry, StreamSession,
    StreamingSurface,
};

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
    surface.session.push_chunk(1, "hello", serde_json::json!({"token": "a"}));
    surface.session.push_chunk(2, "world", serde_json::json!({"token": "b"}));

    assert_eq!(surface.session().events.len(), 2);
}
