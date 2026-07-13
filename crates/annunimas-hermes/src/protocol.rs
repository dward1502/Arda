// sigil: REPAIR
use serde::{Deserialize, Serialize};

use crate::message::{A2AMessage, A2AMessageType, Envelope};
use crate::registry::AgentInfo;

pub struct A2AProtocol {
    version: String,
    max_hops: u8,
}

impl A2AProtocol {
    pub fn new() -> Self {
        Self {
            version: "1.0".into(),
            max_hops: 10,
        }
    }

    pub fn handshake(&self, agent: &AgentInfo) -> HandshakeResult {
        HandshakeResult {
            accepted: true,
            agent_id: agent.id.clone(),
            protocol_version: self.version.clone(),
            capabilities: agent.capabilities.clone(),
            endpoint: agent.endpoint.clone(),
        }
    }

    pub fn build_handshake_request(&self, agent: &AgentInfo) -> A2AMessage {
        A2AMessage::new(
            &agent.id,
            "hermes",
            "handshake",
            serde_json::json!({
                "version": self.version,
                "capabilities": agent.capabilities,
                "realm": agent.realm,
            }),
        )
    }

    pub fn build_handshake_response(&self, agent: &AgentInfo) -> A2AMessage {
        A2AMessage::new(
            "hermes",
            &agent.id,
            "handshake_ack",
            serde_json::json!({
                "version": self.version,
                "status": "accepted",
            }),
        )
    }

    pub fn build_heartbeat(agent_id: &str) -> A2AMessage {
        let mut msg = A2AMessage::new(agent_id, "hermes", "heartbeat", serde_json::json!({}));
        msg.msg_type = A2AMessageType::Heartbeat;
        msg
    }

    pub fn wrap_message(&self, message: A2AMessage, sender: &str) -> Envelope {
        let mut envelope = Envelope::new(message);
        envelope.add_hop(sender, "created");
        envelope
    }

    pub fn forward_message(
        &self,
        envelope: &mut Envelope,
        forwarder: &str,
    ) -> Result<(), ProtocolError> {
        if envelope.hops.len() >= self.max_hops as usize {
            return Err(ProtocolError::max_hops());
        }
        envelope.add_hop(forwarder, "forwarded");
        Ok(())
    }

    pub fn validate_message(&self, envelope: &Envelope) -> ValidationResult {
        if envelope.message.is_expired() {
            return ValidationResult::Invalid("Message expired".into());
        }

        if envelope.hops.len() >= self.max_hops as usize {
            return ValidationResult::Invalid("Max hops exceeded".into());
        }

        ValidationResult::Valid
    }
}

impl Default for A2AProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub accepted: bool,
    pub agent_id: String,
    pub protocol_version: String,
    pub capabilities: Vec<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

impl ProtocolError {
    pub fn max_hops() -> Self {
        Self {
            code: "MAX_HOPS".into(),
            message: "Message exceeded maximum hop count".into(),
        }
    }

    pub fn expired() -> Self {
        Self {
            code: "EXPIRED".into(),
            message: "Message has expired".into(),
        }
    }

    pub fn invalid_signature() -> Self {
        Self {
            code: "INVALID_SIG".into(),
            message: "Message signature verification failed".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            ValidationResult::Valid => None,
            ValidationResult::Invalid(e) => Some(e),
        }
    }
}
