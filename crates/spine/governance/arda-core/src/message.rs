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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_defaults_and_payload_round_trip() {
        let msg = Message {
            id: Uuid::new_v4(),
            schema_version: default_message_schema_version(),
            payload: MessagePayload::Event {
                source: "tests".into(),
                event_type: "check".into(),
                payload: json!({"ok": true}),
            },
            meta: SoterionMeta::default(),
            timestamp: Utc::now(),
        };

        let encoded = serde_json::to_string(&msg).expect("encode message");
        let decoded: Message = serde_json::from_str(&encoded).expect("decode message");
        assert_eq!(decoded.schema_version, MESSAGE_SCHEMA_VERSION);
        assert_eq!(decoded.meta.sigil, None);
        assert_eq!(decoded.meta.tags.len(), 0);
        assert!(matches!(decoded.payload, MessagePayload::Event { .. }));
    }

    #[test]
    fn message_defaults_are_emitted_and_survive_round_trip() {
        let original = Message::event("t", "e", json!(null));

        assert_eq!(original.schema_version, MESSAGE_SCHEMA_VERSION);
        assert!(original.meta.tags.is_empty());
        assert!(original.meta.extra.is_empty());

        let encoded = serde_json::to_string(&original).expect("encode message");
        let decoded: Message = serde_json::from_str(&encoded).expect("decode message");

        assert_eq!(decoded.schema_version, MESSAGE_SCHEMA_VERSION);
        assert!(decoded.meta.tags.is_empty());
        assert!(decoded.meta.extra.is_empty());
        assert!(matches!(decoded.payload, MessagePayload::Event { .. }));
    }
}
