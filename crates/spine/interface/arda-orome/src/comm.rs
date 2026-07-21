//! sigil: REPAIR
//! Agent-to-Human (A2H) protocol types inherited from `arda-comm`.

use arda_core::{Task, TaskStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

pub const COMM_SCHEMA_VERSION: &str = "arda.comm.v1";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    pub content: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthPayload {
    Authorize {
        task_id: Uuid,
        description: String,
        reason: String,
        urgency: Priority,
        deadline: Option<DateTime<Utc>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotifyPayload {
    Notify {
        event: String,
        payload: serde_json::Value,
        priority: Priority,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusPayload {
    Status {
        task_id: Uuid,
        status: TaskStatus,
        progress: f32,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClarifyPayload {
    Clarify {
        question: String,
        options: Vec<String>,
        context: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseAction {
    Approve,
    Deny,
    Defer,
    Clarify,
    #[default]
    Acknowledge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2HMessage {
    Authorize {
        task_id: Uuid,
        description: String,
        reason: String,
        urgency: Priority,
        deadline: Option<DateTime<Utc>>,
    },
    Notify {
        event: String,
        payload: serde_json::Value,
        priority: Priority,
    },
    Response {
        request_id: Uuid,
        content: String,
        attachments: Vec<Attachment>,
    },
    Approval {
        request_id: Uuid,
        approved: bool,
        reason: Option<String>,
        conditions: Vec<String>,
    },
    Clarify {
        question: String,
        options: Vec<String>,
        context: serde_json::Value,
    },
    Status {
        task_id: Uuid,
        status: TaskStatus,
        progress: f32,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub id: Uuid,
    pub channel: Channel,
    pub message: A2HMessage,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: Uuid,
    pub channel: Channel,
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub received_at: DateTime<Utc>,
    pub in_reply_to: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanResponse {
    pub message_id: Uuid,
    pub content: String,
    pub action: ResponseAction,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MessageQueue {
    pending: Arc<tokio::sync::mpsc::Sender<OutboundMessage>>,
    _receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<OutboundMessage>>>,
}

impl MessageQueue {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        Self {
            pending: Arc::new(tx),
            _receiver: Arc::new(Mutex::new(rx)),
        }
    }

    pub async fn enqueue(&self, message: OutboundMessage) -> Result<(), CommError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.pending.send(message),
        )
        .await
        .map_err(|_| CommError::Timeout)?
        .map_err(|_| CommError::ChannelError("channel closed".into()))
    }
}

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
        metadata: serde_json::json!({}),
    }
}

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
        metadata: serde_json::json!({}),
    }
}

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
        metadata: serde_json::json!({}),
    }
}
