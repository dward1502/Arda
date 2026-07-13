// sigil: REPAIR
//! Hermes - Inter-agent messaging and A2A protocol
//!
//! Agent-to-Agent communication, email/Slack/Discord MCP integration

pub mod agent;
pub mod context_cache;
pub mod context_enrichment;
pub mod discord_health;
pub mod discord_safe_message;
pub mod edge;
pub mod formatter;
pub mod intent;
pub mod mcp;
pub mod message;
pub mod mnemosyne_integration;
pub mod poller;
pub mod protocol;
pub mod provider;
pub mod registry;
pub mod relay;
pub mod router;
pub mod serenity_bot;
pub mod service;
pub mod slash;
pub mod transport;
pub mod types;

#[cfg(test)]
pub(crate) static HERMES_PROVIDER_SEND_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub use agent::HermesAgent;
pub use discord_health::{
    DiscordBridgeEvidence, DiscordBridgeReadiness, DiscordBridgeReadinessState,
};
pub use discord_safe_message::{
    validate_discord_safe_message, DiscordSafeMessage, DiscordSafeMessageState,
    DiscordSafeMessageValidation,
};
pub use edge::{DeviceRole, DeviceStatus, EdgeDevice, EdgeRegistry};
pub use formatter::SoterionFormatter;
pub use intent::{classify_message, ClassificationTier};
pub use message::{A2AMessage, A2AMessageType, Envelope};
pub use mnemosyne_integration::spawn_enriched_subagent;
pub use poller::StatusPoller;
pub use provider::{ProviderConfig, ProviderRuntime, ProviderType};
pub use registry::{AgentRegistry, AgentStatus};
pub use relay::CliRelay;
pub use router::MessageRouter;
pub use service::{
    DecisionOption, DecisionPrompt, DiscordChannelDryRunReceipt, DiscordChannelPermissionSummary,
    DiscordChannelPlan, DiscordChannelPlanEntry, HermesService, HermesStatus, HermesSubcomponent,
    MessageStats,
};
pub use slash::{DiscordInteraction, InteractionResponse, SlashCommandHandler};
pub use transport::{expand_home, HermesDaemon, HermesDaemonConfig};
pub use types::{
    BoardroomPost, InboundMessage, IntentClass, IntentResult, IntentRoute, InterruptionDisposition,
    InterruptionMessage, OutboundMessage,
};

pub use mcp::{
    ChannelRegistry, DiscordApiError, DiscordChannel, DiscordChannelInfo, DiscordMessage,
    DiscordUser, EmailChannel, McpChannel, McpChannelError, McpChannelType, McpMessage,
    SlackChannel,
};

pub use serenity_bot::{DiscordBot, DiscordConfig};
