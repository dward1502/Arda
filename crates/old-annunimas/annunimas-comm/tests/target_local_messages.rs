use annunimas_comm::{
    authorize_request, notify, status_update, A2HMessage, Channel, MessageQueue, Priority,
    COMM_SCHEMA_VERSION,
};
use annunimas_core::{Task, TaskStatus};

fn governed_task() -> Task {
    let mut task = Task::new(
        "verify human-facing delivery before changing runtime state",
        "communications",
    );
    task.status = TaskStatus::Running;
    task.joule_cost_estimated = 4.0;
    task.joule_cost_actual = 3.5;
    task
}

#[test]
fn authorization_messages_preserve_task_fields_and_governance_metadata() -> Result<(), String> {
    let task = governed_task();

    let outbound = authorize_request(
        &task,
        "operator approval required".to_owned(),
        Priority::High,
        None,
    );

    assert_eq!(outbound.channel, Channel::Discord);
    assert_eq!(outbound.metadata["schema_version"], COMM_SCHEMA_VERSION);
    assert_eq!(outbound.metadata["message_kind"], "authorize");
    assert_eq!(outbound.metadata["priority"], "high");
    assert_eq!(outbound.metadata["task_type"], "communications");
    assert!(outbound.metadata["governance"]["triad_passed"].is_boolean());
    assert!(outbound.metadata["governance"]["bacon_lite_passed"].is_boolean());
    assert!(outbound.metadata["governance"]["resonance"]
        .as_f64()
        .is_some());

    match outbound.message {
        A2HMessage::Authorize {
            task_id,
            description,
            reason,
            urgency,
            deadline,
        } => {
            assert_eq!(task_id, task.id);
            assert_eq!(description, task.description);
            assert_eq!(reason, "operator approval required");
            assert_eq!(urgency, Priority::High);
            assert!(deadline.is_none());
            Ok(())
        }
        other => Err(format!("unexpected message variant: {other:?}")),
    }
}

#[test]
fn notifications_carry_event_payload_without_task_governance() -> Result<(), String> {
    let outbound = notify(
        "delivery.receipt",
        serde_json::json!({"message_id": "msg-001", "delivered": true}),
        Priority::Critical,
    );

    assert_eq!(outbound.metadata["schema_version"], COMM_SCHEMA_VERSION);
    assert_eq!(outbound.metadata["message_kind"], "notify");
    assert_eq!(outbound.metadata["priority"], "critical");
    assert_eq!(outbound.metadata["event"], "delivery.receipt");
    assert!(outbound.metadata["governance"].is_null());

    match outbound.message {
        A2HMessage::Notify {
            event,
            payload,
            priority,
        } => {
            assert_eq!(event, "delivery.receipt");
            assert_eq!(payload["delivered"], true);
            assert_eq!(priority, Priority::Critical);
            Ok(())
        }
        other => Err(format!("unexpected message variant: {other:?}")),
    }
}

#[test]
fn status_updates_reflect_current_task_status_and_progress() -> Result<(), String> {
    let task = governed_task();

    let outbound = status_update(&task, 0.75, "awaiting operator review".to_owned());

    assert_eq!(outbound.metadata["message_kind"], "status");
    assert_eq!(outbound.metadata["priority"], "normal");
    assert_eq!(outbound.metadata["task_status"], "running");
    assert!(outbound.metadata["resonance"].as_f64().is_some());

    match outbound.message {
        A2HMessage::Status {
            task_id,
            status,
            progress,
            message,
        } => {
            assert_eq!(task_id, task.id);
            assert_eq!(status, TaskStatus::Running);
            assert_eq!(progress, 0.75);
            assert_eq!(message, "awaiting operator review");
            Ok(())
        }
        other => Err(format!("unexpected message variant: {other:?}")),
    }
}

#[tokio::test]
async fn message_queue_accepts_burst_within_configured_capacity(
) -> Result<(), annunimas_comm::CommError> {
    let queue = MessageQueue::new(3);

    for index in 0..3 {
        let message = notify(
            "queue.accepted",
            serde_json::json!({"index": index}),
            Priority::Normal,
        );
        queue.enqueue(message).await?;
    }

    Ok(())
}
