// sigil: REPAIR
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::soterion::SoterionMeta;

pub const MESSAGE_SCHEMA_VERSION: &str = "ardas.message.v1";

/// Message envelope with Soterion metadata baked in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    #[serde(default = "default_message_schema_version")]
    pub schema_version: String,
    pub payload: MessagePayload,
    pub meta: SoterionMeta,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePayload {
    TaskAssignment {
        task_id: Uuid,
        agent: String,
    },
    TaskComplete {
        task_id: Uuid,
        agent: String,
        result: serde_json::Value,
    },
    TaskFailed {
        task_id: Uuid,
        agent: String,
        reason: String,
    },
    Event {
        source: String,
        event_type: String,
        payload: serde_json::Value,
    },
}

impl Message {
    pub fn new(payload: MessagePayload, meta: SoterionMeta) -> Self {
        Self {
            id: Uuid::new_v4(),
            schema_version: default_message_schema_version(),
            payload,
            meta,
            timestamp: Utc::now(),
        }
    }

    pub fn task_assignment(task_id: Uuid, agent: impl Into<String>) -> Self {
        Self::new(
            MessagePayload::TaskAssignment {
                task_id,
                agent: agent.into(),
            },
            SoterionMeta::default(),
        )
    }

    pub fn task_complete(
        task_id: Uuid,
        agent: impl Into<String>,
        result: serde_json::Value,
    ) -> Self {
        Self::new(
            MessagePayload::TaskComplete {
                task_id,
                agent: agent.into(),
                result,
            },
            SoterionMeta::default(),
        )
    }

    pub fn task_failed(task_id: Uuid, agent: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(
            MessagePayload::TaskFailed {
                task_id,
                agent: agent.into(),
                reason: reason.into(),
            },
            SoterionMeta::default(),
        )
    }

    pub fn event(
        source: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self::new(
            MessagePayload::Event {
                source: source.into(),
                event_type: event_type.into(),
                payload,
            },
            SoterionMeta::default(),
        )
    }

    /// Create a message with full SoterionMeta
    pub fn with_soterion(payload: MessagePayload, meta: SoterionMeta) -> Self {
        Self::new(payload, meta)
    }
}

fn default_message_schema_version() -> String {
    MESSAGE_SCHEMA_VERSION.to_string()
}
