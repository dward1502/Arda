// sigil: REPAIR
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordInteraction {
    pub id: String,
    pub application_id: String,
    pub data: Option<InteractionData>,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub member: Option<GuildMember>,
    pub token: String,
    pub version: i32,
    #[serde(rename = "type")]
    pub interaction_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionData {
    pub id: String,
    pub name: String,
    pub options: Option<Vec<CommandOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOption {
    pub name: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMember {
    pub user: DiscordUser,
    pub roles: Vec<String>,
    pub nick: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub response_type: u8,
    pub data: Option<ResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    pub content: Option<String>,
    pub embeds: Option<Vec<serde_json::Value>>,
    pub flags: Option<u8>,
}

impl InteractionResponse {
    pub fn pong() -> Self {
        Self {
            response_type: 1,
            data: None,
        }
    }

    pub fn message(content: &str) -> Self {
        Self {
            response_type: 4,
            data: Some(ResponseData {
                content: Some(content.to_string()),
                embeds: None,
                flags: None,
            }),
        }
    }
}

pub struct SlashCommandHandler {
    status_response: String,
    agents_response: String,
    help_response: String,
}

impl SlashCommandHandler {
    pub fn new() -> Self {
        Self {
            status_response: "𓅃 ∇ ◈ | 𓃭 ◈ | 𓅃 ◈".to_string(),
            agents_response: "∇ arandur | 𓃭 warden | 𓅃 hermes | ↝ charon".to_string(),
            help_response: "/status /agents /help".to_string(),
        }
    }

    pub fn handle(&self, interaction: &DiscordInteraction) -> InteractionResponse {
        if interaction.interaction_type == 1 {
            return InteractionResponse::pong();
        }

        if let Some(data) = &interaction.data {
            match data.name.as_str() {
                "status" => return InteractionResponse::message(&self.status_response),
                "agents" => return InteractionResponse::message(&self.agents_response),
                "help" => return InteractionResponse::message(&self.help_response),
                _ => {}
            }
        }

        InteractionResponse::message("Unknown command")
    }
}

impl Default for SlashCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
