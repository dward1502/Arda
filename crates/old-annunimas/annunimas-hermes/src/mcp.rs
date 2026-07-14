// sigil: REPAIR
//! MCP Integration for Hermes
//!
//! Email, Slack, and Discord channel adapters for inter-agent communication.

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP Channel trait for external communication adapters
#[async_trait]
pub trait McpChannel: Send + Sync {
    /// Send a message through this channel
    async fn send(&self, message: &str, recipient: &str) -> Result<(), McpChannelError>;

    /// Send in chunks for providers that benefit from incremental delivery.
    async fn send_stream(&self, message: &str, recipient: &str) -> Result<usize, McpChannelError> {
        self.send(message, recipient).await?;
        Ok(1)
    }

    /// Receive messages from this channel
    async fn receive(&self) -> Result<Vec<McpMessage>, McpChannelError>;

    /// Check channel health
    async fn health_check(&self) -> bool;
}

/// Errors that can occur in MCP channel operations
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
    pub fn connection_failed(reason: &str) -> Self {
        Self {
            code: "CONNECTION_FAILED".into(),
            message: reason.into(),
        }
    }

    pub fn send_failed(reason: &str) -> Self {
        Self {
            code: "SEND_FAILED".into(),
            message: reason.into(),
        }
    }

    pub fn receive_failed(reason: &str) -> Self {
        Self {
            code: "RECEIVE_FAILED".into(),
            message: reason.into(),
        }
    }
}

/// Incoming message from MCP channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub channel: McpChannelType,
    #[serde(default)]
    pub channel_target: Option<String>,
    #[serde(default)]
    pub sender_is_bot: bool,
}

/// MCP Channel type
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

// =============================================================================
// Email Channel
// =============================================================================

pub struct EmailChannel {
    smtp_host: String,
    smtp_port: u16,
    smtp_username: String,
    smtp_password: String,
    from_address: String,
    imap_host: String,
    imap_port: u16,
    imap_username: String,
    imap_password: String,
    imap_mailbox: String,
}

impl EmailChannel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        smtp_host: impl Into<String>,
        smtp_port: u16,
        smtp_username: impl Into<String>,
        smtp_password: impl Into<String>,
        from_address: impl Into<String>,
        imap_host: impl Into<String>,
        imap_port: u16,
        imap_username: impl Into<String>,
        imap_password: impl Into<String>,
        imap_mailbox: impl Into<String>,
    ) -> Self {
        Self {
            smtp_host: smtp_host.into(),
            smtp_port,
            smtp_username: smtp_username.into(),
            smtp_password: smtp_password.into(),
            from_address: from_address.into(),
            imap_host: imap_host.into(),
            imap_port,
            imap_username: imap_username.into(),
            imap_password: imap_password.into(),
            imap_mailbox: imap_mailbox.into(),
        }
    }

    pub fn default_config() -> Self {
        Self::new(
            std::env::var("EMAIL_SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            std::env::var("EMAIL_SMTP_PORT")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(587),
            std::env::var("EMAIL_SMTP_USERNAME").unwrap_or_default(),
            std::env::var("EMAIL_SMTP_PASSWORD").unwrap_or_default(),
            std::env::var("EMAIL_FROM").unwrap_or_else(|_| "hermes@annunimas.local".to_string()),
            std::env::var("EMAIL_IMAP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            std::env::var("EMAIL_IMAP_PORT")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(993),
            std::env::var("EMAIL_IMAP_USERNAME")
                .or_else(|_| std::env::var("EMAIL_SMTP_USERNAME"))
                .unwrap_or_default(),
            std::env::var("EMAIL_IMAP_PASSWORD")
                .or_else(|_| std::env::var("EMAIL_SMTP_PASSWORD"))
                .unwrap_or_default(),
            std::env::var("EMAIL_IMAP_MAILBOX").unwrap_or_else(|_| "INBOX".to_string()),
        )
    }

    fn can_send(&self) -> bool {
        !self.smtp_host.is_empty() && !self.from_address.is_empty()
    }

    fn can_receive(&self) -> bool {
        !self.imap_host.is_empty()
            && !self.imap_username.is_empty()
            && !self.imap_password.is_empty()
    }

    fn resolve_recipient<'a>(&'a self, recipient: &'a str) -> Option<Cow<'a, str>> {
        if recipient.contains('@') {
            return Some(Cow::Borrowed(recipient));
        }
        let alias_key = format!(
            "EMAIL_ALIAS_{}",
            recipient
                .trim()
                .to_ascii_uppercase()
                .replace(['-', ' '], "_")
        );
        std::env::var(alias_key)
            .ok()
            .map(Cow::Owned)
            .or_else(|| std::env::var("EMAIL_DEFAULT_TO").ok().map(Cow::Owned))
    }
}

#[async_trait]
impl McpChannel for EmailChannel {
    async fn send(&self, message: &str, recipient: &str) -> Result<(), McpChannelError> {
        if !self.can_send() {
            return Err(McpChannelError::connection_failed(
                "EMAIL_SMTP_HOST or EMAIL_FROM not configured",
            ));
        }
        let recipient = self.resolve_recipient(recipient).ok_or_else(|| {
            McpChannelError::send_failed("email recipient missing or invalid alias")
        })?;
        let from: Mailbox = self
            .from_address
            .parse()
            .map_err(|e| McpChannelError::send_failed(&format!("invalid EMAIL_FROM: {e}")))?;
        let to: Mailbox = recipient
            .parse()
            .map_err(|e| McpChannelError::send_failed(&format!("invalid recipient: {e}")))?;
        let email = Message::builder()
            .from(from)
            .to(to)
            .subject("Hermes Agent Message")
            .body(message.to_string())
            .map_err(|e| McpChannelError::send_failed(&format!("message build failed: {e}")))?;

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.smtp_host)
            .map_err(|e| {
                McpChannelError::connection_failed(&format!("smtp relay setup failed: {e}"))
            })?;
        builder = builder.port(self.smtp_port);
        if !self.smtp_username.is_empty() && !self.smtp_password.is_empty() {
            builder = builder.credentials(Credentials::new(
                self.smtp_username.clone(),
                self.smtp_password.clone(),
            ));
        }
        let mailer = builder.build();
        mailer
            .send(email)
            .await
            .map_err(|e| McpChannelError::send_failed(&format!("smtp send failed: {e}")))?;

        tracing::info!(
            "EmailChannel send: to={}, body_len={}",
            recipient,
            message.len()
        );
        Ok(())
    }

    async fn receive(&self) -> Result<Vec<McpMessage>, McpChannelError> {
        if !self.can_receive() {
            return Ok(Vec::new());
        }
        let host = self.imap_host.clone();
        let port = self.imap_port;
        let username = self.imap_username.clone();
        let password = self.imap_password.clone();
        let mailbox = self.imap_mailbox.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<McpMessage>, McpChannelError> {
            let client = imap::ClientBuilder::new(&host, port)
                .connect()
                .map_err(|e| {
                    McpChannelError::connection_failed(&format!("imap connect failed: {e}"))
                })?;
            let mut session = client.login(&username, &password).map_err(|(e, _)| {
                McpChannelError::connection_failed(&format!("imap login failed: {e}"))
            })?;
            session.select(&mailbox).map_err(|e| {
                McpChannelError::receive_failed(&format!("imap select failed: {e}"))
            })?;
            let uids = session.uid_search("UNSEEN").map_err(|e| {
                McpChannelError::receive_failed(&format!("imap search failed: {e}"))
            })?;
            let mut uid_vec: Vec<u32> = uids.into_iter().collect();
            uid_vec.sort_unstable();

            let mut out = Vec::new();
            for uid in uid_vec.into_iter().rev().take(20).rev() {
                let fetches = session
                    .uid_fetch(uid.to_string(), "BODY.PEEK[HEADER] BODY.PEEK[TEXT]")
                    .map_err(|e| {
                        McpChannelError::receive_failed(&format!("imap fetch failed: {e}"))
                    })?;
                for fetch in fetches.iter() {
                    let header = fetch
                        .header()
                        .and_then(|v| std::str::from_utf8(v).ok())
                        .unwrap_or_default();
                    let body = fetch
                        .body()
                        .and_then(|v| std::str::from_utf8(v).ok())
                        .unwrap_or_default();
                    let sender = header
                        .lines()
                        .find_map(|line| line.strip_prefix("From:"))
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or("email");
                    out.push(McpMessage {
                        id: format!("imap-{uid}"),
                        sender: sender.to_string(),
                        content: body.chars().take(4000).collect::<String>(),
                        timestamp: chrono::Utc::now(),
                        channel: McpChannelType::Email,
                        channel_target: None,
                        sender_is_bot: false,
                    });
                }
            }
            let _ = session.logout();
            Ok(out)
        })
        .await
        .map_err(|e| McpChannelError::receive_failed(&format!("imap receive task failed: {e}")))?
    }

    async fn health_check(&self) -> bool {
        if !self.can_send() {
            return false;
        }
        tokio::net::TcpStream::connect((self.smtp_host.as_str(), self.smtp_port))
            .await
            .is_ok()
    }
}

// =============================================================================
// Slack Channel
// =============================================================================

pub struct SlackChannel {
    bot_token: String,
    default_channel: String,
    api_url: String,
    last_seen_ts: Arc<Mutex<Option<String>>>,
}

impl SlackChannel {
    pub fn new(bot_token: impl Into<String>, default_channel: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            default_channel: default_channel.into(),
            api_url: "https://slack.com/api".into(),
            last_seen_ts: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with environment variable detection
    pub fn from_env() -> Self {
        Self::new(
            std::env::var("SLACK_BOT_TOKEN").unwrap_or_default(),
            std::env::var("SLACK_DEFAULT_CHANNEL").unwrap_or_else(|_| "#hermes".into()),
        )
    }

    pub fn is_configured(&self) -> bool {
        !self.bot_token.is_empty()
    }
}

#[async_trait]
impl McpChannel for SlackChannel {
    async fn send(&self, message: &str, recipient: &str) -> Result<(), McpChannelError> {
        if !self.is_configured() {
            return Err(McpChannelError::connection_failed(
                "SLACK_BOT_TOKEN not set",
            ));
        }
        let channel = if recipient.trim().is_empty() {
            self.default_channel.as_str()
        } else {
            recipient
        };
        let url = format!("{}/chat.postMessage", self.api_url);
        let response = reqwest::Client::new()
            .post(url)
            .bearer_auth(&self.bot_token)
            .json(&serde_json::json!({
                "channel": channel,
                "text": message,
            }))
            .send()
            .await
            .map_err(|e| McpChannelError::send_failed(&format!("slack request failed: {e}")))?;
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| McpChannelError::send_failed(&format!("slack decode failed: {e}")))?;
        if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let err = value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_error");
            return Err(McpChannelError::send_failed(&format!(
                "slack api error: {err}"
            )));
        }
        tracing::info!(
            "SlackChannel send: channel={}, message_len={}",
            channel,
            message.len()
        );
        Ok(())
    }

    async fn receive(&self) -> Result<Vec<McpMessage>, McpChannelError> {
        if !self.is_configured() {
            return Err(McpChannelError::connection_failed(
                "SLACK_BOT_TOKEN not set",
            ));
        }

        let poll_channel =
            std::env::var("SLACK_POLL_CHANNEL").unwrap_or_else(|_| self.default_channel.clone());
        if poll_channel.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut query = vec![
            ("channel", poll_channel.clone()),
            ("limit", "20".to_string()),
        ];
        if let Some(oldest) = self.last_seen_ts.lock().await.clone() {
            query.push(("oldest", oldest));
            query.push(("inclusive", "false".to_string()));
        }
        let url = format!("{}/conversations.history", self.api_url);
        let response = reqwest::Client::new()
            .get(url)
            .bearer_auth(&self.bot_token)
            .query(&query)
            .send()
            .await
            .map_err(|e| McpChannelError::receive_failed(&format!("slack history failed: {e}")))?;
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| McpChannelError::receive_failed(&format!("slack decode failed: {e}")))?;
        if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let err = value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_error");
            return Err(McpChannelError::receive_failed(&format!(
                "slack api error: {err}"
            )));
        }
        let mut messages = value
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        messages.sort_by(|a, b| {
            let ta = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            let tb = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            ta.partial_cmp(tb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut out = Vec::new();
        let mut newest_ts: Option<String> = None;
        for entry in messages {
            if entry
                .get("subtype")
                .and_then(|v| v.as_str())
                .map(|s| s == "bot_message")
                .unwrap_or(false)
            {
                continue;
            }
            let Some(ts) = entry.get("ts").and_then(|v| v.as_str()) else {
                continue;
            };
            let text = entry
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let sender = entry
                .get("user")
                .and_then(|v| v.as_str())
                .or_else(|| entry.get("username").and_then(|v| v.as_str()))
                .unwrap_or("slack")
                .to_string();
            out.push(McpMessage {
                id: entry
                    .get("client_msg_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(ts)
                    .to_string(),
                sender,
                content: text,
                timestamp: chrono::Utc::now(),
                channel: McpChannelType::Slack,
                channel_target: Some(poll_channel.clone()),
                sender_is_bot: false,
            });
            newest_ts = Some(ts.to_string());
        }
        if let Some(ts) = newest_ts {
            *self.last_seen_ts.lock().await = Some(ts);
        }
        Ok(out)
    }

    async fn health_check(&self) -> bool {
        if !self.is_configured() {
            return false;
        }
        let url = format!("{}/auth.test", self.api_url);
        match reqwest::Client::new()
            .post(url)
            .bearer_auth(&self.bot_token)
            .send()
            .await
        {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(v) => v.get("ok").and_then(|x| x.as_bool()) == Some(true),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}

// =============================================================================
// Discord Channel
// =============================================================================

pub struct DiscordChannel {
    bot_token: String,
    _app_id: String,
    default_channel_id: String,
    _public_key: String,
    http: reqwest::Client,
}

impl DiscordChannel {
    pub fn new(
        bot_token: impl Into<String>,
        app_id: impl Into<String>,
        channel_id: impl Into<String>,
        public_key: impl Into<String>,
    ) -> Self {
        Self {
            bot_token: bot_token.into(),
            _app_id: app_id.into(),
            default_channel_id: channel_id.into(),
            _public_key: public_key.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("DISCORD_BOT_TOKEN").ok()?;
        let app_id = std::env::var("DISCORD_APP_ID").unwrap_or_default();
        let channel_id = std::env::var("DISCORD_CHANNEL_ID").ok()?;
        let public_key = std::env::var("DISCORD_PUBLIC_KEY").unwrap_or_default();

        Some(Self::new(bot_token, app_id, channel_id, public_key))
    }

    pub fn is_configured(&self) -> bool {
        !self.bot_token.is_empty() && !self.default_channel_id.is_empty()
    }

    fn resolve_channel_id<'a>(&'a self, recipient: &'a str) -> Cow<'a, str> {
        let trimmed = recipient.trim();
        if trimmed.is_empty() {
            return Cow::Borrowed(self.default_channel_id.as_str());
        }
        if trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return Cow::Borrowed(trimmed);
        }

        let routing_target = if trimmed.contains("urgent") || trimmed.contains("critical") {
            "ALERTS"
        } else if trimmed.contains("research") {
            "RESEARCH"
        } else if trimmed.contains("boardroom") {
            "BOARDROOM"
        } else {
            trimmed
        };

        let normalized = routing_target
            .trim_start_matches('#')
            .to_ascii_uppercase()
            .replace(['-', ' ', '_'], "_");

        let alias_key = format!("DISCORD_CHANNEL_{normalized}");
        std::env::var(&alias_key)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(Cow::Owned)
            .unwrap_or_else(|| Cow::Borrowed(self.default_channel_id.as_str()))
    }
    async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<DiscordMessage, DiscordApiError> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );

        let body = serde_json::json!({
            "content": content
        });

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DiscordApiError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Discord API error: {} - {}", status, body);
            return Err(DiscordApiError::Api(status.as_u16(), body));
        }

        let msg: DiscordMessage = response
            .json()
            .await
            .map_err(|e| DiscordApiError::Parse(e.to_string()))?;

        Ok(msg)
    }

    async fn get_messages(&self, limit: u8) -> Result<Vec<McpMessage>, DiscordApiError> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages?limit={}",
            self.default_channel_id, limit
        );

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| DiscordApiError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscordApiError::Api(status.as_u16(), body));
        }

        let messages: Vec<DiscordMessage> = response
            .json()
            .await
            .map_err(|e| DiscordApiError::Parse(e.to_string()))?;

        Ok(messages
            .into_iter()
            .filter(|m| !m.author.bot.unwrap_or(false))
            .map(|m| McpMessage {
                id: m.id,
                sender: m.author.username,
                content: m.content,
                timestamp: m.timestamp,
                channel: McpChannelType::Discord,
                channel_target: Some(m.channel_id),
                sender_is_bot: false,
            })
            .collect())
    }

    pub async fn create_interaction_response(
        &self,
        interaction_id: &str,
        token: &str,
        content: &str,
    ) -> Result<(), DiscordApiError> {
        let url = format!(
            "https://discord.com/api/v10/interactions/{}/{}/callback",
            interaction_id, token
        );

        let body = serde_json::json!({
            "type": 4,
            "data": {
                "content": content
            }
        });

        let response = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DiscordApiError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscordApiError::Api(status.as_u16(), body));
        }

        Ok(())
    }

    pub async fn get_channel(&self) -> Result<DiscordChannelInfo, DiscordApiError> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}",
            self.default_channel_id
        );

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| DiscordApiError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscordApiError::Api(status.as_u16(), body));
        }

        let channel: DiscordChannelInfo = response
            .json()
            .await
            .map_err(|e| DiscordApiError::Parse(e.to_string()))?;

        Ok(channel)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DiscordMessage {
    pub id: String,
    pub content: String,
    pub author: DiscordUser,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub channel_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub bot: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DiscordChannelInfo {
    pub id: String,
    pub name: Option<String>,
    pub guild_id: Option<String>,
    pub last_message_id: Option<String>,
}

#[derive(Debug)]
pub enum DiscordApiError {
    Network(String),
    Api(u16, String),
    Parse(String),
}

impl std::fmt::Display for DiscordApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscordApiError::Network(e) => write!(f, "Network: {}", e),
            DiscordApiError::Api(code, body) => write!(f, "API {}: {}", code, body),
            DiscordApiError::Parse(e) => write!(f, "Parse: {}", e),
        }
    }
}

impl std::error::Error for DiscordApiError {}

impl From<DiscordApiError> for McpChannelError {
    fn from(e: DiscordApiError) -> Self {
        McpChannelError::send_failed(&e.to_string())
    }
}

#[async_trait]
impl McpChannel for DiscordChannel {
    async fn send(&self, message: &str, recipient: &str) -> Result<(), McpChannelError> {
        if !self.is_configured() {
            return Err(McpChannelError::connection_failed(
                "DISCORD_BOT_TOKEN or DISCORD_CHANNEL_ID not set",
            ));
        }
        let channel_id = self.resolve_channel_id(recipient);

        tracing::info!(
            "DiscordChannel send: channel={}, message_len={}",
            channel_id,
            message.len()
        );

        self.send_message(&channel_id, message).await?;
        Ok(())
    }

    async fn send_stream(&self, message: &str, recipient: &str) -> Result<usize, McpChannelError> {
        if !self.is_configured() {
            return Err(McpChannelError::connection_failed(
                "DISCORD_BOT_TOKEN or DISCORD_CHANNEL_ID not set",
            ));
        }
        let channel_id = self.resolve_channel_id(recipient);
        // Discord message content hard limit is 2000 chars; use a conservative chunk size.
        let chunk_size = 1900usize;
        let chars = message.chars().collect::<Vec<_>>();
        let total = chars.len().max(1);
        let mut sent = 0usize;
        let mut offset = 0usize;
        while offset < total {
            let end = (offset + chunk_size).min(total);
            let chunk = chars[offset..end].iter().collect::<String>();
            self.send_message(&channel_id, &chunk).await?;
            sent += 1;
            offset = end;
        }
        Ok(sent)
    }

    async fn receive(&self) -> Result<Vec<McpMessage>, McpChannelError> {
        if !self.is_configured() {
            return Err(McpChannelError::connection_failed(
                "DISCORD_BOT_TOKEN or DISCORD_CHANNEL_ID not set",
            ));
        }

        let messages = self.get_messages(10).await?;
        Ok(messages)
    }

    async fn health_check(&self) -> bool {
        if !self.is_configured() {
            return false;
        }

        match self.get_channel().await {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!("Discord health check failed: {}", e);
                false
            }
        }
    }
}

// =============================================================================
// Channel Registry
// =============================================================================

use std::collections::HashMap;
use tokio::sync::RwLock;

/// Registry for managing MCP channels
pub struct ChannelRegistry {
    channels: Arc<RwLock<HashMap<String, Arc<dyn McpChannel>>>>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a channel with a name
    pub async fn register(&self, name: String, channel: Arc<dyn McpChannel>) {
        self.channels.write().await.insert(name, channel);
    }

    /// Get a channel by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn McpChannel>> {
        self.channels.read().await.get(name).cloned()
    }

    /// List all registered channel names
    pub async fn list(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }

    /// Health check all channels
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let channels = self.channels.read().await;
        let mut results = HashMap::new();

        for (name, channel) in channels.iter() {
            results.insert(name.clone(), channel.health_check().await);
        }

        results
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{DiscordChannel, EmailChannel};
    use std::sync::{Mutex, OnceLock};

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_env_lock<T>(f: impl FnOnce() -> T) -> T {
        let _guard = ENV_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("mcp test env mutex");
        f()
    }

    fn restore_env(key: &str, previous: Option<String>) {
        // SAFETY: mcp tests serialize environment mutation with ENV_MUTEX.
        unsafe {
            if let Some(value) = previous {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }

    #[test]
    fn discord_channel_resolves_numeric_recipient_directly() {
        let channel = DiscordChannel::new("token", "app", "default123", "");
        assert_eq!(channel.resolve_channel_id("1234567890"), "1234567890");
    }

    #[test]
    fn discord_channel_falls_back_to_default_without_alias() {
        with_env_lock(|| {
            let previous = std::env::var("DISCORD_CHANNEL_BOARDROOM").ok();
            // SAFETY: mcp tests serialize environment mutation with ENV_MUTEX.
            unsafe {
                std::env::remove_var("DISCORD_CHANNEL_BOARDROOM");
            }

            let channel = DiscordChannel::new("token", "app", "default123", "");
            assert_eq!(channel.resolve_channel_id("ops-boardroom"), "default123");
            restore_env("DISCORD_CHANNEL_BOARDROOM", previous);
        });
    }

    #[test]
    fn discord_channel_uses_routing_aliases_when_configured() {
        with_env_lock(|| {
            let previous_boardroom = std::env::var("DISCORD_CHANNEL_BOARDROOM").ok();
            let previous_alerts = std::env::var("DISCORD_CHANNEL_ALERTS").ok();
            let channel = DiscordChannel::new("token", "app", "default123", "");
            // SAFETY: mcp tests serialize environment mutation with ENV_MUTEX.
            unsafe {
                std::env::set_var("DISCORD_CHANNEL_BOARDROOM", "board123");
                std::env::set_var("DISCORD_CHANNEL_ALERTS", "alert456");
            }

            assert_eq!(channel.resolve_channel_id("ops-boardroom"), "board123");
            assert_eq!(channel.resolve_channel_id("urgent"), "alert456");

            restore_env("DISCORD_CHANNEL_BOARDROOM", previous_boardroom);
            restore_env("DISCORD_CHANNEL_ALERTS", previous_alerts);
        });
    }

    #[test]
    fn email_channel_resolves_alias_and_default_recipient() {
        with_env_lock(|| {
            let previous_alias = std::env::var("EMAIL_ALIAS_BOARDROOM").ok();
            let previous_default = std::env::var("EMAIL_DEFAULT_TO").ok();
            let channel =
                EmailChannel::new("smtp", 25, "", "", "from@example.com", "", 993, "", "", "");
            // SAFETY: mcp tests serialize environment mutation with ENV_MUTEX.
            unsafe {
                std::env::set_var("EMAIL_ALIAS_BOARDROOM", "board@example.com");
                std::env::set_var("EMAIL_DEFAULT_TO", "default@example.com");
            }

            assert_eq!(
                channel.resolve_recipient("boardroom").as_deref(),
                Some("board@example.com")
            );
            assert_eq!(
                channel.resolve_recipient("unknown").as_deref(),
                Some("default@example.com")
            );
            assert_eq!(
                channel.resolve_recipient("direct@example.com").as_deref(),
                Some("direct@example.com")
            );

            restore_env("EMAIL_ALIAS_BOARDROOM", previous_alias);
            restore_env("EMAIL_DEFAULT_TO", previous_default);
        });
    }
}
