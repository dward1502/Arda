// sigil: REPAIR
//! Arda Communication Module
//!
//! Agent-to-Human (A2H) communication protocol implementation.
//! Provides message types and handlers for autonomous system to human communication.

use arda_core::{Task, TaskStatus};
use arda_governance::{
    bacon_lite_validate, calculate_resonance_basic, love_equation_score, profile_joulework,
    triad_validate,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub const COMM_SCHEMA_VERSION: &str = "arda.comm.v1";

/// Communication errors
#[derive(Error, Debug)]
pub enum CommError {
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),

    #[error("Invalid message format: {0}")]
    InvalidFormat(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Human did not respond")]
    NoResponse,
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// Message types in A2H protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2HMessage {
    /// Agent requests authorization for an action
    Authorize {
        task_id: uuid::Uuid,
        description: String,
        reason: String,
        urgency: Priority,
        deadline: Option<DateTime<Utc>>,
    },

    /// Agent notifies human of an event
    Notify {
        event: String,
        payload: serde_json::Value,
        priority: Priority,
    },

    /// Agent responds to human query
    Response {
        request_id: Uuid,
        content: String,
        attachments: Vec<Attachment>,
    },

    /// Human approves or denies a request
    Approval {
        request_id: Uuid,
        approved: bool,
        reason: Option<String>,
        conditions: Vec<String>,
    },

    /// Agent requests clarification
    Clarify {
        question: String,
        options: Vec<String>,
        context: serde_json::Value,
    },

    /// Status update
    Status {
        task_id: uuid::Uuid,
        status: TaskStatus,
        progress: f32,
        message: String,
    },
}

/// Attachment for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    pub content: String, // Base64 encoded
}

/// Outbound message from agent to human
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub id: Uuid,
    pub channel: Channel,
    pub message: A2HMessage,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommGovernanceMetadata {
    pub triad_passed: bool,
    pub bacon_lite_passed: bool,
    pub resonance: f64,
    pub love_equation_score: f64,
    pub joulework_honesty: f64,
    pub joulework_variance: f64,
}

/// Inbound message from human to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: Uuid,
    pub channel: Channel,
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub received_at: DateTime<Utc>,
    pub in_reply_to: Option<Uuid>,
}

/// Communication channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    #[default]
    Discord,
    Email,
    Terminal,
    Webhook,
    WebUI,
}

/// Human response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanResponse {
    pub message_id: Uuid,
    pub content: String,
    pub action: ResponseAction,
    pub timestamp: DateTime<Utc>,
}

/// Possible human responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseAction {
    Approve,
    Deny,
    Defer,
    Clarify,
    Acknowledge,
}

/// Message queue for handling communication
#[derive(Debug, Clone)]
pub struct MessageQueue {
    pending: Arc<tokio::sync::mpsc::Sender<OutboundMessage>>,
    _receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<OutboundMessage>>>,
}

impl MessageQueue {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        Self {
            pending: Arc::new(tx),
            _receiver: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }

    pub async fn enqueue(&self, message: OutboundMessage) -> Result<(), CommError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.pending.send(message),
        )
        .await
        .map_err(|_| CommError::Timeout)?
        .map_err(|_| CommError::ChannelError("Channel closed".into()))
    }
}

/// Build an authorization request
pub fn authorize_request(
    task: &Task,
    reason: String,
    urgency: Priority,
    deadline: Option<DateTime<Utc>>,
) -> OutboundMessage {
    OutboundMessage {
        id: Uuid::new_v4(),
        channel: Channel::Discord,
        message: A2HMessage::Authorize {
            task_id: task.id,
            description: task.description.clone(),
            reason,
            urgency,
            deadline,
        },
        created_at: Utc::now(),
        metadata: governance_metadata("authorize", urgency, Some(task), None),
    }
}

/// Build a notification
pub fn notify(event: &str, payload: serde_json::Value, priority: Priority) -> OutboundMessage {
    OutboundMessage {
        id: Uuid::new_v4(),
        channel: Channel::Discord,
        message: A2HMessage::Notify {
            event: event.to_string(),
            payload,
            priority,
        },
        created_at: Utc::now(),
        metadata: governance_metadata("notify", priority, None, Some(event)),
    }
}

/// Build a status update
pub fn status_update(task: &Task, progress: f32, message: String) -> OutboundMessage {
    OutboundMessage {
        id: Uuid::new_v4(),
        channel: Channel::Discord,
        message: A2HMessage::Status {
            task_id: task.id,
            status: task.status.clone(),
            progress,
            message,
        },
        created_at: Utc::now(),
        metadata: governance_metadata("status", Priority::Normal, Some(task), None),
    }
}

fn governance_metadata(
    message_kind: &str,
    priority: Priority,
    task: Option<&Task>,
    event: Option<&str>,
) -> serde_json::Value {
    let governance = task.map(build_comm_governance);
    serde_json::json!({
        "schema_version": COMM_SCHEMA_VERSION,
        "sigil": "𓅃",
        "message_kind": message_kind,
        "priority": format!("{priority:?}").to_lowercase(),
        "task_type": task.map(|t| t.task_type.clone()),
        "task_status": task.map(|t| format!("{:?}", t.status).to_lowercase()),
        "resonance": governance.as_ref().map(|g| g.resonance),
        "event": event,
        "governance": governance,
    })
}

fn build_comm_governance(task: &Task) -> CommGovernanceMetadata {
    let triad = triad_validate(task, None);
    let bacon = bacon_lite_validate(task);
    let resonance = calculate_resonance_basic(task);
    let love = love_equation_score(task);
    let joulework = profile_joulework(task);

    CommGovernanceMetadata {
        triad_passed: triad.passed,
        bacon_lite_passed: bacon.passed,
        resonance: resonance.value,
        love_equation_score: love.score,
        joulework_honesty: joulework.honesty_ratio,
        joulework_variance: joulework.variance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::Task;

    #[test]
    fn test_message_serialization() {
        let msg = A2HMessage::Authorize {
            task_id: uuid::Uuid::new_v4(),
            description: "Deploy to production".to_string(),
            reason: "Scheduled release".to_string(),
            urgency: Priority::High,
            deadline: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("authorize"));
    }

    #[test]
    fn outbound_builders_include_governance_metadata() {
        let mut task = Task::new("deploy service because rollout must be verified", "deploy");
        task.joule_cost_estimated = 3.0;
        task.joule_cost_actual = 3.2;
        let auth = authorize_request(&task, "release window".to_string(), Priority::High, None);
        assert_eq!(auth.metadata["schema_version"], COMM_SCHEMA_VERSION);
        assert_eq!(auth.metadata["sigil"], "𓅃");
        assert_eq!(auth.metadata["message_kind"], "authorize");
        assert_eq!(auth.metadata["priority"], "high");
        assert_eq!(auth.metadata["task_type"], "deploy");
        assert!(auth.metadata["governance"]["triad_passed"].is_boolean());
        assert!(auth.metadata["governance"]["love_equation_score"]
            .as_f64()
            .is_some());

        let status = status_update(&task, 0.5, "halfway".to_string());
        assert_eq!(status.metadata["message_kind"], "status");
        assert!(status.metadata["resonance"].as_f64().is_some());
        assert!(status.metadata["governance"]["joulework_honesty"]
            .as_f64()
            .is_some());
    }

    #[tokio::test]
    async fn message_queue_enqueues_outbound_message() {
        let queue = MessageQueue::new(4);
        let msg = notify(
            "system_ready",
            serde_json::json!({"ok": true}),
            Priority::Normal,
        );
        queue.enqueue(msg).await.expect("enqueue");
    }
}
