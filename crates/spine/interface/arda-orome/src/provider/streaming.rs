// sigil: REPAIR
//! Streaming surface shared by provider adapters.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub sequence: u64,
    pub delta: String,
    pub provider_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEnded {
    pub finished: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum StreamEvent {
    Chunk(StreamChunk),
    Ended(StreamEnded),
}

#[derive(Debug, Clone)]
pub struct StreamSession {
    pub provider_id: String,
    pub message_id: String,
    pub events: Vec<StreamEvent>,
}

impl StreamSession {
    pub fn new(provider_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            message_id: message_id.into(),
            events: Vec::new(),
        }
    }

    pub fn push_chunk(
        &mut self,
        sequence: u64,
        delta: impl Into<String>,
        provider_metadata: serde_json::Value,
    ) {
        self.events.push(StreamEvent::Chunk(StreamChunk {
            sequence,
            delta: delta.into(),
            provider_metadata,
        }));
    }
}

#[derive(Debug, Clone)]
pub struct StreamingSurface {
    pub provider_id: String,
    pub session: StreamSession,
    pub paused: bool,
}

impl StreamingSurface {
    pub fn new(provider_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        Self {
            provider_id: provider_id.clone(),
            session: StreamSession::new(provider_id, message_id),
            paused: false,
        }
    }

    pub fn session(&self) -> &StreamSession {
        &self.session
    }
}
