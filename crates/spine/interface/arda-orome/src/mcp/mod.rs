pub mod browser;
pub mod external_sources;
pub mod protocol;
pub mod server;
pub mod tools;

pub use browser::*;
pub use external_sources::*;
pub use protocol::*;
pub use server::{McpServer, *};
pub use tools::*;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpChannelError {
    pub code: String,
    pub message: String,
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
