// sigil: REPAIR
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum A2AMessageType {
    Request,
    Response,
    Notification,
    Handshake,
    Heartbeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: Uuid,
    pub msg_type: A2AMessageType,
    pub priority: MessagePriority,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub payload: serde_json::Value,
    pub thread_id: Option<Uuid>,
    pub reply_to: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub delivery_status: DeliveryStatus,
}

impl A2AMessage {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        subject: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            msg_type: A2AMessageType::Request,
            priority: MessagePriority::Normal,
            from: from.into(),
            to: to.into(),
            subject: subject.into(),
            payload,
            thread_id: None,
            reply_to: None,
            created_at: Utc::now(),
            expires_at: None,
            delivery_status: DeliveryStatus::Pending,
        }
    }

    pub fn notification(
        from: impl Into<String>,
        to: impl Into<String>,
        subject: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::new(from, to, subject, payload);
        msg.msg_type = A2AMessageType::Notification;
        msg
    }

    pub fn response_to(mut self, original: &A2AMessage) -> Self {
        self.msg_type = A2AMessageType::Response;
        self.reply_to = Some(original.id);
        self.thread_id = original.thread_id.or(Some(original.id));
        self.to = original.from.clone();
        self
    }

    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_ttl_seconds(mut self, seconds: i64) -> Self {
        self.expires_at = Some(self.created_at + chrono::Duration::seconds(seconds));
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            expires < Utc::now()
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub message: A2AMessage,
    pub signature: Option<String>,
    pub hops: Vec<Hop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hop {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub action: String,
}

impl Envelope {
    pub fn new(message: A2AMessage) -> Self {
        Self {
            message,
            signature: None,
            hops: Vec::new(),
        }
    }

    pub fn sign(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    pub fn add_hop(&mut self, agent_id: impl Into<String>, action: impl Into<String>) {
        self.hops.push(Hop {
            agent_id: agent_id.into(),
            timestamp: Utc::now(),
            action: action.into(),
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    pub subject: String,
    pub participants: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub messages: Vec<Uuid>,
}

impl Thread {
    pub fn new(subject: impl Into<String>, participants: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            subject: subject.into(),
            participants,
            created_at: now,
            last_activity: now,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message_id: Uuid) {
        self.last_activity = Utc::now();
        self.messages.push(message_id);
    }
}
