use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpChannelError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for McpChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for McpChannelError {}

impl McpChannelError {
    pub fn connection_failed(reason: impl Into<String>) -> Self {
        Self {
            code: "CONNECTION_FAILED".into(),
            message: reason.into(),
        }
    }
    pub fn send_failed(reason: impl Into<String>) -> Self {
        Self {
            code: "SEND_FAILED".into(),
            message: reason.into(),
        }
    }
    pub fn receive_failed(reason: impl Into<String>) -> Self {
        Self {
            code: "RECEIVE_FAILED".into(),
            message: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub channel: McpChannelType,
    pub channel_target: Option<String>,
    pub sender_is_bot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpChannelType {
    Email,
    Slack,
    Discord,
    Http,
    WebSocket,
}

impl std::fmt::Display for McpChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpChannelType::Email => write!(f, "email"),
            McpChannelType::Slack => write!(f, "slack"),
            McpChannelType::Discord => write!(f, "discord"),
            McpChannelType::Http => write!(f, "http"),
            McpChannelType::WebSocket => write!(f, "websocket"),
        }
    }
}

#[async_trait]
pub trait McpChannel: Send + Sync {
    async fn send(&self, message: &str, recipient: &str) -> Result<(), McpChannelError>;
    async fn send_stream(&self, message: &str, recipient: &str) -> Result<usize, McpChannelError> {
        self.send(message, recipient).await?;
        Ok(1)
    }
    async fn receive(&self) -> Result<Vec<McpMessage>, McpChannelError>;
    async fn health_check(&self) -> bool;
}
