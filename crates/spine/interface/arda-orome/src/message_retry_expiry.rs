// sigil: REPAIR
//! Unified message-level retry/expiry tests (A2AMessage + MessageRouter).

use crate::message::{A2AMessage, DeliveryStatus};
use crate::registry::{AgentInfo, AgentRegistry, AgentStatus};
use crate::router::MessageRouter;

#[test]
fn message_is_expired_only_with_past_expires_at() {
    let msg = A2AMessage::notification(
        "illuvatar",
        "edge-1",
        "expire me",
        serde_json::json!("ignore"),
    )
    .with_ttl_seconds(0);

    let fresh = A2AMessage::new("a", "b", "subject", serde_json::json!({}));

    assert!(msg.is_expired());
    assert!(!fresh.is_expired());
}

#[test]
fn registry_routes_busy_agent_to_queue() {
    let mut router = MessageRouter::new();
    let mut registry = AgentRegistry::new();
    registry.register(AgentInfo {
        id: "edge-1".to_string(),
        name: "edge".to_string(),
        realm: "worker".to_string(),
        capabilities: Vec::new(),
        status: AgentStatus::Busy,
        last_seen: chrono::Utc::now(),
        endpoint: Some("local://edge-1".to_string()),
    });
    let msg = A2AMessage::new("illuvatar", "edge-1", "ping", serde_json::json!("hello"));

    match router.route(&msg, &registry) {
        crate::router::RouteResult::Queued(agent_id) => assert_eq!(agent_id, "edge-1"),
        other => panic!("expected queued, got {other:?}"),
    }
    assert_eq!(router.queue_len("edge-1"), 1);
}

#[test]
fn registry_routes_online_agent_to_deliver() {
    let mut router = MessageRouter::new();
    let mut registry = AgentRegistry::new();
    registry.register(AgentInfo {
        id: "edge-1".to_string(),
        name: "edge".to_string(),
        realm: "worker".to_string(),
        capabilities: Vec::new(),
        status: AgentStatus::Online,
        last_seen: chrono::Utc::now(),
        endpoint: Some("local://edge-1".to_string()),
    });
    let msg = A2AMessage::new("illuvatar", "edge-1", "ping", serde_json::json!("hello"));

    match router.route(&msg, &registry) {
        crate::router::RouteResult::Deliver(endpoint) => {
            assert_eq!(endpoint.as_deref(), Some("local://edge-1"))
        }
        other => panic!("expected deliver, got {other:?}"),
    }
}

#[test]
fn retry_failed_only_resets_pending_once() {
    let mut router = MessageRouter::new();
    let mut msg = A2AMessage::new("illuvatar", "edge-1", "send", serde_json::json!({}));
    msg.delivery_status = DeliveryStatus::Failed;
    router.enqueue("edge-1".to_string(), msg.clone());

    assert_eq!(router.retry_failed().len(), 1);
    assert_eq!(router.retry_failed().len(), 0);
}

#[test]
fn drain_expired_removes_only_expired_messages() {
    let mut router = MessageRouter::new();
    router.enqueue(
        "edge-1".to_string(),
        A2AMessage::new("illuvatar", "edge-1", "expired", serde_json::json!({}))
            .with_ttl_seconds(0),
    );
    router.enqueue(
        "edge-2".to_string(),
        A2AMessage::new("illuvatar", "edge-2", "fresh", serde_json::json!({})),
    );

    let drained = router.drain_expired();
    assert_eq!(drained, 1);
    assert_eq!(router.total_queued(), 1);
    assert!(router.peek("edge-2").is_some());
}
