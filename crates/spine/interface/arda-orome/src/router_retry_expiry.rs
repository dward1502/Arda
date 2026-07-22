// sigil: REPAIR
//! Router retry/expiry tests.

use crate::message::{A2AMessage, DeliveryStatus};
use crate::registry::AgentRegistry;
use crate::router::{MessageRouter, RouteResult};

#[test]
fn retry_failed_resets_pending_only_once() {
    let mut router = MessageRouter::new();
    let mut msg = A2AMessage::new("a", "b", "subject", serde_json::json!({}));
    msg.delivery_status = DeliveryStatus::Failed;
    router.enqueue("b".to_string(), msg.clone());

    let first = router.retry_failed();
    assert_eq!(first.len(), 1);
    assert_eq!(router.total_queued(), 1);

    let second = router.retry_failed();
    assert_eq!(second.len(), 0);
}

#[test]
fn drain_expired_removes_only_expired_messages() {
    let mut router = MessageRouter::new();
    router.enqueue(
        "b".to_string(),
        A2AMessage::new("a", "b", "expired", serde_json::json!({}))
            .with_ttl_seconds(0),
    );
    router.enqueue(
        "c".to_string(),
        A2AMessage::new("a", "c", "fresh", serde_json::json!({})),
    );

    let drained = router.drain_expired();
    assert_eq!(drained, 1);
    assert_eq!(router.total_queued(), 1);
    assert!(router.peek("c").is_some());
}
