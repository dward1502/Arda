use arda_outpost_protocol::queue::{AckStatus, ProduceError};
use arda_outpost_protocol::{
    consume_queue, generate_queue, AgentFeedback, OutpostObservation, SCHEMA_VERSION,
};
use serde_json::json;

#[test]
fn queue_accepts_scout_feedback_in_generate_consume_path() {
    let queue = generate_queue(8);
    queue.create_topic("crates").expect("topic should exist");

    let feedback = AgentFeedback::new(
        "arda-outpost-scout",
        "crates".to_string(),
        arda_outpost_protocol::ObservationClassification::DerivedEstimate,
        arda_outpost_protocol::AuthorityClass::Advisory,
        0.8,
        SCHEMA_VERSION.to_string(),
        json!({"test": "payload"}),
    );

    let observation: OutpostObservation =
        feedback.into_outpost_observation("arda-outpost-scout://survey".to_string());

    let enqueued = queue
        .produce("crates", &observation)
        .expect("produce should accept observation within capacity");

    assert_eq!(enqueued.attempts, 1);
    assert!(matches!(enqueued.ack, AckStatus::Pending));

    let consumed = consume_queue(&queue, "crates")
        .expect("consume should return queued observation")
        .expect("queue should contain exactly one observation");

    assert_eq!(consumed.observation.source, "arda-outpost-scout");
    assert_eq!(consumed.observation.schema_version, SCHEMA_VERSION);
    assert_eq!(
        consumed.observation.provenance.as_deref(),
        Some("arda-outpost-scout://survey")
    );
    assert!((consumed.observation.confidence - 0.8).abs() < f32::EPSILON);
}

fn fixture_observation() -> OutpostObservation {
    AgentFeedback::new(
        "arda-outpost-scout",
        "crates".to_string(),
        arda_outpost_protocol::ObservationClassification::DerivedEstimate,
        arda_outpost_protocol::AuthorityClass::Advisory,
        0.8,
        SCHEMA_VERSION.to_string(),
        json!({"test": "payload"}),
    )
    .into_outpost_observation("arda-outpost-scout://survey".to_string())
}

#[test]
fn each_topic_enforces_the_requested_capacity() {
    let queue = generate_queue(1);
    queue.create_topic("alpha").expect("alpha");
    queue.create_topic("beta").expect("beta");
    let observation = fixture_observation();

    queue.produce("alpha", &observation).expect("alpha produce");
    queue.produce("beta", &observation).expect("beta produce");
    assert!(matches!(
        queue.produce("alpha", &observation),
        Err(ProduceError::Full)
    ));
    assert!(matches!(
        queue.produce("beta", &observation),
        Err(ProduceError::Full)
    ));
}

#[test]
fn failed_ack_requeues_with_retry_metadata() {
    let queue = generate_queue(1);
    queue.create_topic("scout").expect("topic");
    queue
        .produce("scout", &fixture_observation())
        .expect("produce");
    let consumed = consume_queue(&queue, "scout")
        .expect("consume")
        .expect("queued observation");

    queue
        .ack("scout", &consumed, AckStatus::Failed)
        .expect("failed ack");
    let retried = consume_queue(&queue, "scout")
        .expect("consume retry")
        .expect("retried observation");
    assert_eq!(retried.attempts, 2);
    assert_eq!(retried.ack, AckStatus::Pending);
    assert_eq!(
        retried.last_error.as_deref(),
        Some("consumer reported failure")
    );
}
