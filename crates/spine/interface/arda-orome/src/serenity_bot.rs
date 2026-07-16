// sigil: REPAIR
use crate::service::HermesService;
use crate::types::CommsEventRisk;
use arda_mandos::{OracleEngine, OracleQuery, VerdictOutcome};
use chrono::Utc;
use serde_json::Value;
use serenity::all::*;
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Discord bot configuration
#[derive(Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub application_id: u64,
    pub guild_id: Option<u64>,
    pub channel_id: Option<u64>,
    pub ready_message: Option<String>,
}

impl DiscordConfig {
    pub fn new(token: impl Into<String>, application_id: u64) -> Self {
        Self {
            token: token.into(),
            application_id,
            guild_id: None,
            channel_id: None,
            ready_message: None,
        }
    }

    pub fn with_guild(mut self, guild_id: u64) -> Self {
        self.guild_id = Some(guild_id);
        self
    }

    pub fn with_channel(mut self, channel_id: u64) -> Self {
        self.channel_id = Some(channel_id);
        self
    }

    pub fn with_ready_message(mut self, ready_message: impl Into<String>) -> Self {
        let ready_message = ready_message.into();
        if !ready_message.trim().is_empty() {
            self.ready_message = Some(ready_message);
        }
        self
    }
}

/// System state for slash commands
#[derive(Clone, Default)]
pub struct SystemState {
    pub agents: Arc<RwLock<Vec<AgentStatus>>>,
    pub edges: Arc<RwLock<Vec<EdgeDevice>>>,
    pub joule_work: Arc<RwLock<f64>>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct AgentStatus {
    pub name: String,
    pub status: String,
    pub role: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EdgeDevice {
    pub name: String,
    pub device_type: String,
    pub ip: String,
    pub status: String,
    pub tailscale: bool,
}

impl SystemState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_agent(&self, name: &str, status: &str, role: &str) {
        let mut agents = self.agents.write().await;
        agents.push(AgentStatus {
            name: name.to_string(),
            status: status.to_string(),
            role: role.to_string(),
        });
    }

    pub async fn register_edge(&self, name: &str, device_type: &str, ip: &str, status: &str) {
        let mut edges = self.edges.write().await;
        edges.push(EdgeDevice {
            name: name.to_string(),
            device_type: device_type.to_string(),
            ip: ip.to_string(),
            status: status.to_string(),
            tailscale: true,
        });
    }

    pub async fn update_joule_work(&self, amount: f64) {
        let mut jw = self.joule_work.write().await;
        *jw = amount;
    }
}

/// Main Discord bot with slash commands
pub struct DiscordBot {
    client: Option<serenity::Client>,
    config: DiscordConfig,
    state: SystemState,
}

struct BotHandler {
    state: SystemState,
    channel_id: Option<u64>,
    ready_message: Option<String>,
}

#[serenity::async_trait]
impl EventHandler for BotHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            let _ = cmd
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
                )
                .await;

            let content = match cmd.data.name.as_str() {
                "status" => handle_status(&self.state, &cmd).await,
                "agents" => handle_agents(&self.state).await,
                "edge" => handle_edge(&self.state).await,
                "query" => handle_query(&cmd).await,
                "plans" => handle_plans(&cmd),
                "tasks" => handle_tasks(&cmd),
                "task" => handle_task(&cmd),
                "review" => handle_review(&cmd),
                "continue" => handle_continue(&cmd),
                "council" => handle_council(&cmd),
                "gateway" => handle_gateway(&cmd),
                "help" => handle_help(),
                _ => "Unknown command".to_string(),
            };

            record_operating_room_interaction(&annunimas_root(), &cmd, content.len());

            let _ = cmd
                .edit_response(&ctx.http, EditInteractionResponse::new().content(content))
                .await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("Discord bot connected: {}", ready.user.name);
        if let Some(channel_id) = self.channel_id {
            let content = self.ready_message.clone().unwrap_or_else(|| {
                "∇ Arandur operating room online. Use `/plans`, `/tasks`, `/review`, `/council`, `/gateway`, or `/status`.".to_string()
            });
            let _ = ChannelId::new(channel_id)
                .send_message(&ctx.http, CreateMessage::new().content(content))
                .await;
        }
    }
}

impl DiscordBot {
    pub fn new(token: impl Into<String>, application_id: u64) -> Self {
        Self {
            client: None,
            config: DiscordConfig::new(token, application_id),
            state: SystemState::new(),
        }
    }

    pub fn with_guild(mut self, guild_id: u64) -> Self {
        self.config.guild_id = Some(guild_id);
        self
    }

    pub fn with_channel(mut self, channel_id: u64) -> Self {
        self.config.channel_id = Some(channel_id);
        self
    }

    pub fn with_ready_message(mut self, ready_message: impl Into<String>) -> Self {
        self.config = self.config.with_ready_message(ready_message);
        self
    }

    pub fn state(&self) -> &SystemState {
        &self.state
    }

    /// Start the Discord bot with slash commands
    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MEMBERS;

        let state = self.state.clone();
        let channel_id = self.config.channel_id;
        let ready_message = self.config.ready_message.clone();

        let client = serenity::Client::builder(&self.config.token, intents)
            .application_id(ApplicationId::new(self.config.application_id))
            .event_handler(BotHandler {
                state,
                channel_id,
                ready_message,
            })
            .await?;

        // Register slash commands
        register_slash_commands(
            &client.http,
            self.config.application_id,
            self.config.guild_id,
        )
        .await?;

        self.client = Some(client);
        if let Some(client) = &mut self.client {
            client.start().await?;
        }
        Ok(())
    }

    /// Send a message to a channel
    pub async fn send_message(
        &self,
        channel_id: u64,
        content: &str,
    ) -> Result<Message, serenity::Error> {
        let http = &self
            .client
            .as_ref()
            .ok_or_else(|| serenity::Error::Other("Client not initialized"))?
            .http;

        let channel = ChannelId::new(channel_id);
        channel
            .send_message(http, CreateMessage::new().content(content))
            .await
    }

    /// Send embed message
    pub async fn send_embed(
        &self,
        channel_id: u64,
        embed: CreateEmbed,
    ) -> Result<Message, serenity::Error> {
        let http = &self
            .client
            .as_ref()
            .ok_or_else(|| serenity::Error::Other("Client not initialized"))?
            .http;

        let channel = ChannelId::new(channel_id);
        channel
            .send_message(http, CreateMessage::new().embed(embed))
            .await
    }
}

/// Register global slash commands
async fn register_slash_commands(
    http: &Http,
    _app_id: u64,
    guild_id: Option<u64>,
) -> Result<(), serenity::Error> {
    let commands = vec![
        CreateCommand::new("status")
            .description("Check Annunimas system status")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "detail",
                "Level of detail to show",
            )),
        CreateCommand::new("agents").description("List all agent statuses"),
        CreateCommand::new("edge").description("List edge devices (Pi5, Beelink)"),
        CreateCommand::new("query")
            .description("Query Oracle knowledge base")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "question", "Your question")
                    .required(true),
            ),
        CreateCommand::new("plans")
            .description("List or show Annunimas plans")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "list or show")
                    .add_string_choice("list", "list")
                    .add_string_choice("show", "show"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "target",
                "Plan filename, path, or title fragment",
            )),
        CreateCommand::new("tasks")
            .description("List or show Annunimas project tasks")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "action",
                    "list, next, or show",
                )
                .add_string_choice("list", "list")
                .add_string_choice("next", "next")
                .add_string_choice("show", "show"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "target",
                "Task id or title fragment",
            )),
        CreateCommand::new("task")
            .description("Propose a task or request a governed run")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "action",
                    "propose, request_run, or promote_council",
                )
                .required(true)
                .add_string_choice("propose", "propose")
                .add_string_choice("request_run", "request_run")
                .add_string_choice("promote_council", "promote_council"),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "target",
                    "Task title, task id, or council note id",
                )
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "notes",
                "Optional context; required task:<id> when promoting council notes",
            )),
        CreateCommand::new("review")
            .description("Show the latest Annunimas task/review activity")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "latest")
                    .add_string_choice("latest", "latest"),
            ),
        CreateCommand::new("continue")
            .description("Continue a work-stream task or thread without direct execution")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "target",
                    "Task id, title fragment, or thread label to continue",
                )
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "notes",
                "Optional continuation context",
            )),
        CreateCommand::new("council")
            .description("Show or record discussion-only Arandur council notes")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "status or note")
                    .add_string_choice("status", "status")
                    .add_string_choice("note", "note"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "prompt",
                "Discussion-only council note or local summary prompt",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "task_ref",
                "Optional task:<id> anchor; does not mutate the task queue",
            )),
        CreateCommand::new("gateway")
            .description("Show Hermes Agent gateway activation readiness")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "action",
                    "activation_check, remote_confidence, or record_receipt",
                )
                .add_string_choice("activation_check", "activation_check")
                .add_string_choice("remote_confidence", "remote_confidence")
                .add_string_choice("record_receipt", "record_receipt"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "task_id",
                "Canonical task id for record_receipt",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "summary",
                "Hermes Agent result summary for record_receipt",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "background_task_id",
                "Optional Hermes Agent background task id",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "status",
                    "completed or needs_review",
                )
                .add_string_choice("completed", "completed")
                .add_string_choice("needs_review", "needs_review")
                .add_string_choice("blocked", "blocked"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "verification",
                "Optional verification, separated by semicolons",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "changed_file",
                "Optional changed paths, separated by semicolons",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "blockers",
                "Optional blockers, separated by semicolons",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "next_action",
                "Optional next action",
            )),
        CreateCommand::new("help").description("Get help with Annunimas commands"),
    ];

    if let Some(guild_id) = guild_id {
        GuildId::new(guild_id).set_commands(http, commands).await?;
    } else {
        http.create_global_commands(&commands).await?;
    }
    Ok(())
}

/// Handle /status command
async fn handle_status(state: &SystemState, cmd: &CommandInteraction) -> String {
    let detail = cmd
        .data
        .options
        .iter()
        .find(|o| o.name == "detail")
        .and_then(|o| o.value.as_str())
        .unwrap_or("brief");

    let jw = state.joule_work.read().await;
    let agents = state.agents.read().await;
    let control_plane: Vec<&AgentStatus> = agents
        .iter()
        .filter(|agent| {
            matches!(
                agent.name.to_ascii_lowercase().as_str(),
                "arandur" | "ceo" | "prometheus" | "warden" | "hermes" | "charon"
            )
        })
        .collect();

    if detail == "full" {
        let focus = if control_plane.is_empty() {
            agents.iter().collect::<Vec<_>>()
        } else {
            control_plane
        };
        let agent_list: Vec<String> = focus
            .iter()
            .map(|a| {
                format!(
                    "{} {} {}",
                    sigil(&a.name),
                    a.name.to_ascii_lowercase(),
                    status_mark(&a.status)
                )
            })
            .collect();

        format!(
            "𓅃 ∇ {:.2}\n{}\n∑ {}",
            *jw,
            if agent_list.is_empty() {
                "∅".to_string()
            } else {
                agent_list.join("\n")
            },
            agents.len()
        )
    } else {
        format!("𓅃 ∇ {:.2} | ∑ {}", *jw, agents.len())
    }
}

/// Handle /agents command
async fn handle_agents(state: &SystemState) -> String {
    let agents = state.agents.read().await;

    let list: Vec<String> = agents
        .iter()
        .filter(|a| {
            matches!(
                a.name.to_ascii_lowercase().as_str(),
                "arandur" | "ceo" | "prometheus" | "warden" | "hermes" | "charon"
            )
        })
        .map(|a| {
            format!(
                "{} {} {}",
                sigil(&a.name),
                a.name.to_ascii_lowercase(),
                status_mark(&a.status)
            )
        })
        .collect();

    if list.is_empty() {
        "∇ arandur | 𓃭 warden | 𓅃 hermes".to_string()
    } else {
        list.join(" | ")
    }
}

/// Handle /edge command
async fn handle_edge(state: &SystemState) -> String {
    let edges = state.edges.read().await;

    if edges.is_empty() {
        "**𓊝 Edge Devices**\nNo edge devices registered\n\nUse `/edge add <name> <type> <ip>` to add a device".to_string()
    } else {
        let list: Vec<String> = edges
            .iter()
            .map(|e| {
                let icon = if e.tailscale { "🛡️" } else { "📡" };
                let status_icon = match e.status.as_str() {
                    "online" => "🟢",
                    "offline" => "🔴",
                    _ => "🟡",
                };
                format!(
                    "{} {} **{}** ({}) - `{}`",
                    icon, status_icon, e.name, e.device_type, e.ip
                )
            })
            .collect();

        format!("**𓊝 Edge Devices** (Tailscale)\n{}\n━", list.join("\n"))
    }
}

fn sigil(agent: &str) -> &'static str {
    match agent.to_ascii_lowercase().as_str() {
        "arandur" | "ceo" => "∇",
        "prometheus" => "∇",
        "warden" => "𓃭",
        "hermes" => "𓅃",
        "charon" => "↝",
        _ => "𓁿",
    }
}

fn status_mark(status: &str) -> &'static str {
    match status.to_ascii_lowercase().as_str() {
        "online" | "ready" => "◈",
        "working" | "busy" => "⚡",
        "idle" | "away" => "↝",
        "error" | "offline" => "✖",
        _ => "◈",
    }
}

/// Handle /query command
async fn handle_query(cmd: &CommandInteraction) -> String {
    let question = cmd
        .data
        .options
        .iter()
        .find(|o| o.name == "question")
        .and_then(|o| o.value.as_str())
        .unwrap_or("");

    if question.is_empty() {
        "Please provide a question".to_string()
    } else {
        let mut engine = OracleEngine::new();
        let query = OracleQuery {
            id: Uuid::new_v4().to_string(),
            task: question.to_string(),
            context: Vec::new(),
            requester: "discord".to_string(),
            timestamp: Utc::now(),
        };
        let verdict = engine.evaluate(query);
        let outcome = match verdict.outcome {
            VerdictOutcome::Pass => "PASS",
            VerdictOutcome::Fail => "FAIL",
            VerdictOutcome::Conditional => "CONDITIONAL",
        };

        format!(
            "**𓆣 Oracle Query**\n\nQuestion: `{}`\nOutcome: **{}**\nResonance: `{:.2}`",
            question, outcome, verdict.resonance_score
        )
    }
}

fn option_str(cmd: &CommandInteraction, name: &str) -> Option<String> {
    cmd.data
        .options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| o.value.as_str())
        .map(|value| value.to_string())
}

fn annunimas_root() -> PathBuf {
    env::var("ANNUNIMAS_ROOT")
        .map(PathBuf::from)
        .or_else(|_| env::current_dir())
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn short_text(value: &str, limit: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= limit {
        compact
    } else {
        format!("{}...", &compact[..limit.saturating_sub(3)])
    }
}

fn handle_plans(cmd: &CommandInteraction) -> String {
    let action = option_str(cmd, "action").unwrap_or_else(|| "list".to_string());
    let target = option_str(cmd, "target");
    render_plans_command(&annunimas_root(), &action, target.as_deref())
}

fn handle_tasks(cmd: &CommandInteraction) -> String {
    let action = option_str(cmd, "action").unwrap_or_else(|| "list".to_string());
    let target = option_str(cmd, "target");
    render_tasks_command(&annunimas_root(), &action, target.as_deref())
}

fn handle_task(cmd: &CommandInteraction) -> String {
    let action = option_str(cmd, "action").unwrap_or_default();
    let target = option_str(cmd, "target").unwrap_or_default();
    let notes = option_str(cmd, "notes");
    render_task_mutation_command(&annunimas_root(), &action, &target, notes.as_deref())
}

fn handle_review(cmd: &CommandInteraction) -> String {
    let action = option_str(cmd, "action").unwrap_or_else(|| "latest".to_string());
    render_review_command(&annunimas_root(), &action)
}

fn handle_continue(cmd: &CommandInteraction) -> String {
    let target = option_str(cmd, "target").unwrap_or_default();
    let notes = option_str(cmd, "notes");
    render_continue_command(&annunimas_root(), &target, notes.as_deref())
}

fn handle_council(cmd: &CommandInteraction) -> String {
    let action = option_str(cmd, "action").unwrap_or_else(|| "status".to_string());
    let prompt = option_str(cmd, "prompt");
    let task_ref = option_str(cmd, "task_ref");
    render_council_command(
        &annunimas_root(),
        &action,
        prompt.as_deref(),
        task_ref.as_deref(),
    )
}

fn handle_gateway(cmd: &CommandInteraction) -> String {
    let action = option_str(cmd, "action").unwrap_or_else(|| "activation_check".to_string());
    let input = GatewayReceiptInput {
        task_id: option_str(cmd, "task_id"),
        background_task_id: option_str(cmd, "background_task_id"),
        status: option_str(cmd, "status"),
        summary: option_str(cmd, "summary"),
        verification: option_str(cmd, "verification"),
        changed_file: option_str(cmd, "changed_file"),
        blockers: option_str(cmd, "blockers"),
        next_action: option_str(cmd, "next_action"),
    };
    render_gateway_command(&annunimas_root(), &action, input)
}

fn record_operating_room_interaction(root: &Path, cmd: &CommandInteraction, response_len: usize) {
    if !is_operating_room_command(&cmd.data.name) {
        return;
    }
    let option_names = cmd
        .data
        .options
        .iter()
        .map(|option| option.name.clone())
        .collect::<Vec<_>>();
    let receipt = build_operating_room_interaction_receipt(
        &cmd.data.name,
        option_names,
        semantic_channel_for_interaction(root, &cmd.data.name, Some(cmd.channel_id.get())),
        Some(cmd.user.id.get()),
        cmd.guild_id.map(|id| id.get()),
        Some(cmd.channel_id.get()),
        Some(cmd.id.get()),
        response_len,
    );
    let path = root.join("data/hermes/discord_operating_room_interactions.jsonl");
    if let Err(err) = append_jsonl_value(&path, &receipt) {
        tracing::warn!("failed to record Discord operating-room interaction: {err}");
    }
}

fn is_operating_room_command(command: &str) -> bool {
    matches!(
        command,
        "plans" | "tasks" | "task" | "review" | "continue" | "council" | "gateway"
    )
}

fn build_operating_room_interaction_receipt(
    command: &str,
    option_names: Vec<String>,
    semantic_channel: String,
    user_id: Option<u64>,
    guild_id: Option<u64>,
    channel_id: Option<u64>,
    interaction_id: Option<u64>,
    response_len: usize,
) -> Value {
    serde_json::json!({
        "schema_version": "annunimas.hermes.discord_operating_room_interaction.v1",
        "recorded_at_utc": Utc::now().to_rfc3339(),
        "command": format!("/{command}"),
        "semantic_channel": semantic_channel,
        "option_names": option_names,
        "user_id_redacted": user_id.map(redacted_numeric_id),
        "guild_id": guild_id,
        "channel_id": channel_id,
        "interaction_id": interaction_id,
        "response_len": response_len,
        "proof_role": "operator_originated_discord_command_receipt",
        "content_policy": "option_values_and_response_content_not_recorded"
    })
}

fn semantic_channel_for_interaction(root: &Path, command: &str, channel_id: Option<u64>) -> String {
    let Some(channel_id) = channel_id.map(|id| id.to_string()) else {
        return default_semantic_for_command(command).to_string();
    };
    let env = load_discord_channel_env(root);
    let channel_matches = |keys: &[&str]| {
        keys.iter()
            .any(|key| env.get(*key).map(String::as_str) == Some(channel_id.as_str()))
    };

    if command == "council" && channel_matches(&["DISCORD_CHANNEL_COUNCIL"]) {
        return "council".to_string();
    }
    if channel_matches(&["DISCORD_CHANNEL_WORK_STREAM", "DISCORD_CHANNEL_TASKS"]) {
        return "work-stream".to_string();
    }
    if channel_matches(&["DISCORD_CHANNEL_GENERAL", "DISCORD_CHANNEL_ID"]) {
        return "general".to_string();
    }
    if channel_matches(&["DISCORD_CHANNEL_COUNCIL", "DISCORD_CHANNEL_BOARDROOM"]) {
        return "council".to_string();
    }

    default_semantic_for_command(command).to_string()
}

fn default_semantic_for_command(command: &str) -> &'static str {
    match command {
        "council" => "council",
        "plans" | "tasks" | "task" | "review" | "continue" | "gateway" => "work-stream",
        _ => "general",
    }
}

fn load_discord_channel_env(root: &Path) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for (key, value) in env::vars() {
        if key.starts_with("DISCORD_CHANNEL_") {
            values.insert(key, value);
        }
    }
    let env_path = root.join("config/.env");
    if let Ok(raw) = fs::read_to_string(&env_path) {
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.starts_with("DISCORD_CHANNEL_") {
                values.insert(key.to_string(), value.trim().trim_matches('"').to_string());
            }
        }
    }
    values
}

fn redacted_numeric_id(id: u64) -> String {
    let raw = id.to_string();
    let keep = raw.len().saturating_sub(4);
    format!("redacted:{}", &raw[keep..])
}

fn render_plans_command(root: &Path, action: &str, target: Option<&str>) -> String {
    let plans = load_plan_entries(root);
    if plans.is_empty() {
        return "No plan files found under `docs/plans`.".to_string();
    }

    if action.eq_ignore_ascii_case("show") {
        let Some(target) = target.map(str::trim).filter(|value| !value.is_empty()) else {
            return "Provide a plan filename, path, or title fragment with `target`.".to_string();
        };
        if let Some(plan) = find_plan_entry(&plans, target) {
            return format!(
                "**Plan**\n`{}`\n{}\n\n{}",
                plan.path,
                plan.title,
                short_text(&plan.preview, 1500)
            );
        }
        return format!("No plan matched `{target}`.");
    }

    let rows = plans
        .iter()
        .take(8)
        .map(|plan| format!("`{}` — {}", plan.path, plan.title))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "**Available Plans** ({})\n{}\n\nUse `/plans action:show target:<path-or-title>` for detail.",
        plans.len(),
        rows
    )
}

fn render_tasks_command(root: &Path, action: &str, target: Option<&str>) -> String {
    let tasks = latest_project_tasks(root);
    if tasks.is_empty() {
        return "No project tasks found in `core/projects/tasks/queue.jsonl`.".to_string();
    }

    if action.eq_ignore_ascii_case("show") {
        let Some(target) = target.map(str::trim).filter(|value| !value.is_empty()) else {
            return "Provide a task id or title fragment with `target`.".to_string();
        };
        if let Some(task) = find_task(&tasks, target) {
            return format_task_detail(task);
        }
        return format!("No task matched `{target}`.");
    }

    let status_filter = if action.eq_ignore_ascii_case("next") {
        Some("queued")
    } else {
        None
    };
    let rows = tasks
        .iter()
        .filter(|task| {
            status_filter
                .map(|status| task_str(task, "status") == status)
                .unwrap_or(true)
        })
        .take(8)
        .map(format_task_row)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        return format!("No `{}` tasks found.", status_filter.unwrap_or("project"));
    }

    let title = if action.eq_ignore_ascii_case("next") {
        "Next Queued Tasks"
    } else {
        "Recent Project Tasks"
    };
    format!(
        "**{}**\n{}\n\nUse `/tasks action:show target:<task-id>` for detail.",
        title,
        rows.join("\n")
    )
}

fn render_task_mutation_command(
    root: &Path,
    action: &str,
    target: &str,
    notes: Option<&str>,
) -> String {
    let target = target.trim();
    if target.is_empty() {
        return "Provide a task title or task id in `target`.".to_string();
    }

    match action.trim().to_ascii_lowercase().as_str() {
        "propose" => propose_work_stream_task(root, target, notes),
        "request_run" | "run" => request_task_run(root, target, notes),
        "promote_council" => promote_council_note_request(root, target, notes),
        _ => "Supported task actions: `propose`, `request_run`, `promote_council`.".to_string(),
    }
}

fn propose_work_stream_task(root: &Path, title: &str, notes: Option<&str>) -> String {
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let now = Utc::now();
    let task_id = format!("tsk_{}_{}", now.format("%Y%m%d"), slug_fragment(title, 48));
    let task = serde_json::json!({
        "id": task_id,
        "title": title,
        "owner": "hermes",
        "priority": "normal",
        "status": "queued",
        "queued_at_utc": now.to_rfc3339(),
        "glyphs": ["↝"],
        "notes": notes.unwrap_or("Proposed from Discord work-stream."),
        "meta": {
            "origin": "discord_work_stream",
            "scope": "hermes_discord_gateway",
            "authority": "operator_explicit_slash_command"
        }
    });

    match append_jsonl_value(&queue_path, &task) {
        Ok(()) => format!(
            "**Task Proposed**\n`{}` — {}\nstatus: `queued`\nsource: `core/projects/tasks/queue.jsonl`",
            task_id, title
        ),
        Err(err) => format!("Task proposal failed: {err}"),
    }
}

fn request_task_run(root: &Path, task_id: &str, notes: Option<&str>) -> String {
    let tasks = latest_project_tasks(root);
    let Some(task) = find_task(&tasks, task_id) else {
        return format!("Run request blocked: no task matched `{task_id}`.");
    };
    let canonical_task_id = task_id_for_ref(task);
    let receipt_id = format!(
        "work_stream_run_{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let receipt = serde_json::json!({
        "schema_version": "annunimas.hermes.work_stream_task_run_request.v1",
        "receipt_id": receipt_id,
        "task_ref": format!("task:{canonical_task_id}"),
        "task_title": task_str(task, "title"),
        "requested_by": "discord_work_stream",
        "request_state": "requested",
        "execution_state": "not_started_by_discord",
        "governance_boundary": "discord_can_request_run_but_executor_must_use_canonical_task_policy",
        "notes": notes.unwrap_or("Run requested from Discord work-stream."),
        "created_at_utc": Utc::now().to_rfc3339()
    });
    let receipt_path = root.join("data/hermes/work_stream_requests.jsonl");
    match append_jsonl_value(&receipt_path, &receipt) {
        Ok(()) => format!(
            "**Task Run Requested**\n`task:{}`\nreceipt: `{}`\nstate: `requested`; Discord did not execute terminal work directly.",
            canonical_task_id, receipt_id
        ),
        Err(err) => format!("Task run request failed: {err}"),
    }
}

fn promote_council_note_request(root: &Path, note_id: &str, task_ref: Option<&str>) -> String {
    let Some(task_ref) = task_ref.map(str::trim).filter(|value| !value.is_empty()) else {
        return "Council promotion blocked: provide canonical `task:<id>` in `notes`.".to_string();
    };
    if !task_ref.starts_with("task:") {
        return "Council promotion blocked: `notes` must start with `task:`.".to_string();
    }

    let sessions_path = root.join("data/hermes/council_sessions.jsonl");
    let note_exists = fs::read_to_string(&sessions_path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .any(|value| {
                    value.get("note_id").and_then(Value::as_str) == Some(note_id)
                        && value
                            .get("schema_version")
                            .and_then(Value::as_str)
                            .map(council_note_schema_is_promotable)
                            .unwrap_or(false)
                })
        })
        .unwrap_or(false);
    if !note_exists {
        return format!("Council promotion blocked: note `{note_id}` was not found.");
    }

    let promotion_id = format!(
        "council_promotion_{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let promotion = serde_json::json!({
        "schema_version": "annunimas.hermes.council_discussion_promotion.v1",
        "promotion_id": promotion_id.clone(),
        "note_id": note_id,
        "task_ref": task_ref,
        "promotion_state": "projected",
        "is_authoritative": false,
        "canonical_write_authorized": false,
        "queue_mutated": false,
        "requires_human_approval": true,
        "authority_boundary": "discord_projection_only_not_canonical_authority",
        "canonical_refs": [note_id, task_ref, promotion_id.clone()],
        "promoted_by": "discord_work_stream",
        "promoted_at_utc": Utc::now().to_rfc3339()
    });

    match append_jsonl_value(&sessions_path, &promotion) {
        Ok(()) => format!(
            "**Council Note Promoted**\n`{note_id}` -> `{task_ref}`\npromotion: `{promotion_id}`"
        ),
        Err(err) => format!("Council promotion failed: {err}"),
    }
}

fn render_review_command(root: &Path, action: &str) -> String {
    if !action.eq_ignore_ascii_case("latest") {
        return "Supported review action: `latest`.".to_string();
    }
    let tasks = latest_project_tasks(root);
    let rows = tasks
        .iter()
        .filter(|task| task_str(task, "status") == "completed")
        .take(5)
        .map(format_task_row)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        "No completed task review entries found.".to_string()
    } else {
        format!("**Latest Review Activity**\n{}", rows.join("\n"))
    }
}

fn render_continue_command(root: &Path, target: &str, notes: Option<&str>) -> String {
    let target = target.trim();
    if target.is_empty() {
        return "Provide a task id, title fragment, or thread label in `target`.".to_string();
    }

    let tasks = latest_project_tasks(root);
    let matched_task = find_task(&tasks, target);
    let (target_kind, canonical_ref, display_title) = if let Some(task) = matched_task {
        (
            "task",
            format!("task:{}", task_id_for_ref(task)),
            task_str(task, "title").to_string(),
        )
    } else {
        (
            "thread",
            format!("work-stream:{}", slug_fragment(target, 64)),
            target.to_string(),
        )
    };

    let continuation_id = format!(
        "work_stream_continue_{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let continuation = serde_json::json!({
        "schema_version": "annunimas.hermes.work_stream_continuation_request.v1",
        "continuation_id": continuation_id,
        "semantic_channel": "work-stream",
        "target_kind": target_kind,
        "canonical_ref": canonical_ref,
        "display_title": display_title,
        "requested_by": "discord_work_stream",
        "request_state": "queued_for_conversation",
        "execution_state": "conversation_only_until_explicit_task_run_or_gateway_receipt",
        "governance_boundary": "continue preserves context but does not approve execution or close tasks",
        "notes": notes.unwrap_or("Continue this work-stream thread."),
        "created_at_utc": Utc::now().to_rfc3339()
    });
    let continuation_path = root.join("data/hermes/work_stream_continuations.jsonl");
    match append_jsonl_value(&continuation_path, &continuation) {
        Ok(()) => format!(
            "**Work Stream Continuation Queued**\n`{}` -> `{}`\nstate: `queued_for_conversation`\nsource: `data/hermes/work_stream_continuations.jsonl`",
            continuation
                .get("continuation_id")
                .and_then(Value::as_str)
                .unwrap_or("work_stream_continue_unknown"),
            continuation
                .get("canonical_ref")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        Err(err) => format!("Work-stream continuation failed: {err}"),
    }
}

fn render_council_command(
    root: &Path,
    action: &str,
    prompt: Option<&str>,
    task_ref: Option<&str>,
) -> String {
    if action.eq_ignore_ascii_case("note") {
        return record_local_council_summary_route(root, prompt, task_ref);
    }
    if !action.eq_ignore_ascii_case("status") && !action.trim().is_empty() {
        return "Supported council actions: `status`, `note`. Council notes remain discussion-only until explicitly promoted by a governed task flow.".to_string();
    }
    [
        "**Council Command Seats**",
        "`first` — Arandur: CEO/main orchestrator; final direction and broad situational command.",
        "`second` — Prometheus: executor pipeline; task lifecycle, routing, ledger, and accountability.",
        "`third` — Counsel by default for pressure-test review; Oracle when truth, validation, or triad judgment is required.",
        "",
        "Council notes are discussion-only until promoted to a canonical task event. Use `/council action:note` to record non-authoritative local summary evidence.",
    ]
    .join("\n")
}

fn council_note_schema_is_promotable(schema: &str) -> bool {
    schema.ends_with("council_discussion_note.v1")
        || schema == "annunimas.hermes.local_council_summary_route.v1"
}

fn record_local_council_summary_route(
    root: &Path,
    prompt: Option<&str>,
    task_ref: Option<&str>,
) -> String {
    let summary = prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Council local summary route note recorded without operator-provided prompt.");
    let note_id = format!(
        "local_council_summary_{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let receipt = serde_json::json!({
        "schema_version": "annunimas.hermes.local_council_summary_route.v1",
        "note_id": note_id,
        "session_id": format!("council_discord_{}", Utc::now().format("%Y%m%dT%H%M%SZ")),
        "created_at_utc": Utc::now().to_rfc3339(),
        "semantic_channel": "council",
        "speaker_agent": "discord-council-local-summary",
        "seat": "discussion",
        "message_class": "local_summary_note",
        "summary": summary,
        "task_ref": task_ref,
        "source": "discord_slash_command_council_note",
        "route": {
            "kind": "local_council_summary",
            "is_authoritative": false,
            "requires_human_promotion": true
        },
        "policy_boundary": {
            "conversation_is_not_execution_approval": true,
            "canonical_write_authorized": false,
            "queue_mutated": false,
            "objective_mutated": false,
            "external_messages_sent": false,
            "service_restart_performed": false,
            "model_switch_performed": false
        },
        "evidence_only": true,
        "is_authoritative": false,
        "promotable": task_ref.map(str::trim).filter(|value| !value.is_empty()).is_some(),
        "task_promotion_allowed": false,
        "human_approval_granted": false
    });
    let path = root.join("data/hermes/council_sessions.jsonl");
    match append_jsonl_value(&path, &receipt) {
        Ok(()) => format!(
            "**Council Note Recorded**\n`{}`\nstate: `non-authoritative discussion evidence`\nsource: `data/hermes/council_sessions.jsonl`\nmutation: `none`",
            receipt
                .get("note_id")
                .and_then(Value::as_str)
                .unwrap_or("local_council_summary_unknown")
        ),
        Err(err) => format!("Council note recording failed: {err}"),
    }
}

#[derive(Debug, Clone, Default)]
struct GatewayReceiptInput {
    task_id: Option<String>,
    background_task_id: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    verification: Option<String>,
    changed_file: Option<String>,
    blockers: Option<String>,
    next_action: Option<String>,
}

fn render_gateway_command(root: &Path, action: &str, input: GatewayReceiptInput) -> String {
    if action.eq_ignore_ascii_case("record_receipt") {
        return record_gateway_receipt(root, input);
    }
    if action.eq_ignore_ascii_case("remote_confidence") || action.eq_ignore_ascii_case("confidence")
    {
        let status = command_stdout("hermes", &["gateway", "status"]);
        return render_remote_confidence_snapshot(root, status.as_deref());
    }
    if !action.eq_ignore_ascii_case("activation_check") {
        return "Supported gateway actions: `activation_check`, `remote_confidence`, `record_receipt`.".to_string();
    }
    let status = command_stdout("hermes", &["gateway", "status"]);
    render_gateway_activation_check(root, status.as_deref())
}

fn render_remote_confidence_snapshot(root: &Path, gateway_status: Option<&str>) -> String {
    let tasks = read_jsonl_values(&root.join("core/projects/tasks/queue.jsonl"));
    let open_tasks = tasks
        .iter()
        .filter(|row| value_status_in(row, &["queued", "open", "ready", "in_progress", "pending"]))
        .count();
    let human_gates = tasks
        .iter()
        .filter(|row| {
            value_status_in(
                row,
                &[
                    "human_gated",
                    "requires_human",
                    "awaiting_human",
                    "needs_human",
                    "blocked",
                ],
            ) || row
                .get("human_required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let gateway_running = gateway_status
        .map(hermes_gateway_status_is_running)
        .unwrap_or(false);
    let autonomy = read_json_file(&root.join("core/control/autonomy/state.json"));
    let autonomy_mode = autonomy
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let flywheel = read_json_file(&root.join("data/flywheel/latest_packet.json"));
    let flywheel_id = flywheel
        .get("packet_id")
        .or_else(|| flywheel.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let council = latest_jsonl_value(&root.join("data/prometheus/council_decisions.jsonl"));
    let council_id = council
        .get("decision_id")
        .or_else(|| council.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let latest_receipts =
        read_jsonl_values(&root.join("data/hermes/hermes_agent_gateway_receipts.jsonl"));
    let receipt_ids = latest_receipts
        .iter()
        .rev()
        .take(3)
        .filter_map(|row| {
            row.get("receipt_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>();
    let overall = if gateway_running && human_gates == 0 {
        "nominal"
    } else {
        "attention_required"
    };
    let receipts = if receipt_ids.is_empty() {
        "none".to_string()
    } else {
        receipt_ids.into_iter().rev().collect::<Vec<_>>().join(", ")
    };

    format!(
        "**Remote Confidence Snapshot**\noverall: {overall}\nDiscord: remote confidence surface\nPrimary consoles: ARDA HUD, Hermes Agent CLI/TUI\nruntime core: continues without Discord attached\ngateway_running: {gateway_running}\nautonomy: {autonomy_mode}\nopen tasks: {open_tasks}\nhuman gates: {human_gates}\nlatest flywheel: {flywheel_id}\nlast council decision: {council_id}\nlast completion receipts: {receipts}\nside_effect_policy: read_only=true service_restart=false credential_change=false external_messages_sent=false"
    )
}

fn read_json_file(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn read_jsonl_values(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn latest_jsonl_value(path: &Path) -> Value {
    read_jsonl_values(path)
        .into_iter()
        .last()
        .unwrap_or_else(|| serde_json::json!({}))
}

fn value_status_in(row: &Value, statuses: &[&str]) -> bool {
    row.get("status")
        .and_then(Value::as_str)
        .map(|status| {
            statuses
                .iter()
                .any(|candidate| status.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

fn record_gateway_receipt(root: &Path, input: GatewayReceiptInput) -> String {
    let Some(task_id) = input
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return "Gateway receipt blocked: provide `task_id`.".to_string();
    };
    let task_lookup = task_id.strip_prefix("task:").unwrap_or(task_id);
    let tasks = latest_project_tasks(root);
    if find_task(&tasks, task_lookup).is_none() {
        return format!("Gateway receipt blocked: no task matched `{task_id}`.");
    }
    let Some(summary) = input
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return "Gateway receipt blocked: provide `summary`.".to_string();
    };

    let status = input
        .status
        .as_deref()
        .unwrap_or("completed")
        .trim()
        .to_ascii_lowercase();
    let verification = split_gateway_list(input.verification.as_deref());
    let changed_files = split_gateway_list(input.changed_file.as_deref());
    let blockers = split_gateway_list(input.blockers.as_deref());
    let next_action = input.next_action.as_deref().unwrap_or(
        "Review Hermes Agent gateway result and decide whether to close or continue the task.",
    );
    let receipt_id = format!(
        "hag_{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let task_ref = if task_id.starts_with("task:") {
        task_id.to_string()
    } else {
        format!("task:{task_id}")
    };
    let receipt = serde_json::json!({
        "schema_version": "annunimas.hermes_agent_gateway_background_result.v1",
        "receipt_id": receipt_id,
        "task_ref": task_ref,
        "source": "hermes_agent_gateway",
        "platform": "discord",
        "semantic_channel": "work-stream",
        "background_task_id": input.background_task_id.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "status": status.clone(),
        "summary": summary,
        "verification": verification.clone(),
        "changed_files": changed_files.clone(),
        "blockers": blockers.clone(),
        "next_action": next_action,
        "policy_boundary": {
            "annunimas_records_authority": true,
            "gateway_result_is_not_approval": true,
            "requires_review_when_unverified_or_blocked": true
        },
        "created_at_utc": Utc::now().to_rfc3339(),
    });
    let receipt_path = root.join("data/hermes/hermes_agent_gateway_receipts.jsonl");
    if let Err(err) = append_jsonl_value(&receipt_path, &receipt) {
        return format!("Gateway receipt failed: {err}");
    }

    let service = match HermesService::new(root.join("data/hermes")) {
        Ok(service) => service,
        Err(err) => return format!("Gateway receipt failed: {err}"),
    };
    let verified = status == "completed" && blockers.is_empty() && !verification.is_empty();
    let risk = if status == "completed" && blockers.is_empty() {
        CommsEventRisk::Low
    } else {
        CommsEventRisk::Medium
    };
    let adapted_summary = if let Some(background_task_id) = input
        .background_task_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        format!("Hermes Agent gateway result from Discord/work-stream: {summary} (background task `{background_task_id}`)")
    } else {
        format!("Hermes Agent gateway result from Discord/work-stream: {summary}")
    };
    let packet = match service.record_subagent_completion_packet(
        task_id,
        "hermes_agent_gateway",
        &adapted_summary,
        verification,
        changed_files,
        blockers,
        risk,
        next_action,
        verified,
    ) {
        Ok(packet) => packet,
        Err(err) => return format!("Gateway receipt failed: {err}"),
    };

    format!(
        "**Gateway Receipt Recorded**\n`{}` -> `{}`\nsubagent packet: `{}`\nreview_required: `{}`\nsource: `data/hermes/hermes_agent_gateway_receipts.jsonl`",
        receipt["receipt_id"].as_str().unwrap_or("unknown-receipt"),
        task_ref,
        packet.completion_id,
        packet.review_required
    )
}

fn split_gateway_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(['\n', ';'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn render_gateway_activation_check(root: &Path, gateway_status: Option<&str>) -> String {
    let plan_exists = root
        .join("docs/plans/2026-05-30-hermes-discord-gateway-unification-plan.md")
        .exists();
    let runbook_exists = root
        .join("docs/operations/hermes-agent-discord-gateway-runbook.md")
        .exists();
    let template_exists = root
        .join("config/hermes_agent_gateway_annunimas.example.yaml")
        .exists();
    let semantic_source =
        fs::read_to_string(root.join("crates/annunimas-hermes/src/service/semantic_channel.rs"))
            .unwrap_or_default();
    let work_stream_ready = semantic_source.contains("\"work-stream\"")
        && semantic_source.contains("\"workstream\"")
        && semantic_source.contains("\"tasks\"");
    let adapter_ready = root
        .join("crates/annunimas-cli/src/commands/utility.rs")
        .exists();
    let gateway_running = gateway_status
        .map(hermes_gateway_status_is_running)
        .unwrap_or(false);
    let missing_env = [
        "ANNUNIMAS_DISCORD_WORK_STREAM_CHANNEL_ID",
        "ANNUNIMAS_OPERATOR_DISCORD_USER_ID",
    ]
    .into_iter()
    .filter(|key| {
        env::var(key)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    })
    .collect::<Vec<_>>();
    let unsafe_allow_all = ["DISCORD_ALLOW_ALL_USERS", "GATEWAY_ALLOW_ALL_USERS"]
        .into_iter()
        .filter(|key| {
            env::var(key)
                .map(|value| {
                    matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "true" | "1" | "yes"
                    )
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    let safe_local_ready = plan_exists
        && runbook_exists
        && template_exists
        && work_stream_ready
        && adapter_ready
        && unsafe_allow_all.is_empty();
    let live_ready = safe_local_ready && gateway_running && missing_env.is_empty();
    let status_label = if live_ready {
        "ready_for_live_gateway"
    } else if safe_local_ready {
        "safe_local_ready_live_human_gates_pending"
    } else {
        "not_ready"
    };
    let mut blockers = Vec::new();
    if !plan_exists || !runbook_exists || !template_exists {
        blockers.push("missing plan/runbook/template artifact".to_string());
    }
    if !work_stream_ready {
        blockers.push("work-stream semantic channel not ready".to_string());
    }
    if !adapter_ready {
        blockers.push("gateway receipt adapter source missing".to_string());
    }
    if !missing_env.is_empty() {
        blockers.push(format!("missing live env: {}", missing_env.join(",")));
    }
    if !unsafe_allow_all.is_empty() {
        blockers.push(format!(
            "unsafe allow-all env enabled: {}",
            unsafe_allow_all.join(",")
        ));
    }
    if !gateway_running {
        blockers.push("Hermes Agent gateway service is not running".to_string());
    }

    let blockers = if blockers.is_empty() {
        "none".to_string()
    } else {
        blockers
            .into_iter()
            .map(|blocker| format!("- {blocker}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "**Hermes Agent Gateway Activation**\nstatus: `{status_label}`\nsafe_local_ready: `{safe_local_ready}` | live_ready: `{live_ready}`\nwork-stream: `{work_stream_ready}` | receipt_adapter: `{adapter_ready}` | gateway_running: `{gateway_running}`\n\n**Blockers**\n{blockers}\n\nRun `cargo run -p annunimas-cli -- utility hermes-agent-gateway-activation-check` for full JSON."
    )
}

fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    Command::new(command)
        .args(args)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

fn hermes_gateway_status_is_running(stdout: &str) -> bool {
    let normalized = stdout.to_ascii_lowercase();
    if normalized.contains("inactive")
        || normalized.contains("dead")
        || normalized.contains("stopped")
        || normalized.contains("service is stopped")
        || normalized.contains("not running")
    {
        return false;
    }
    normalized.contains("active: active")
        || normalized.contains("active (running)")
        || normalized.contains("service is running")
}

#[derive(Debug, Clone)]
struct PlanEntry {
    path: String,
    title: String,
    preview: String,
}

fn load_plan_entries(root: &Path) -> Vec<PlanEntry> {
    let mut entries = Vec::new();
    collect_plan_entries(root, &root.join("docs/plans"), &mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

fn collect_plan_entries(root: &Path, dir: &Path, entries: &mut Vec<PlanEntry>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("archive") {
                collect_plan_entries(root, &path, entries);
            }
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path.as_path())
            .to_string_lossy()
            .to_string();
        let title = content
            .lines()
            .find_map(|line| line.strip_prefix("# ").map(str::trim))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("untitled-plan")
            })
            .to_string();
        let preview = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with("---"))
            .take(20)
            .collect::<Vec<_>>()
            .join("\n");
        entries.push(PlanEntry {
            path: relative,
            title,
            preview,
        });
    }
}

fn find_plan_entry<'a>(plans: &'a [PlanEntry], target: &str) -> Option<&'a PlanEntry> {
    let needle = target.trim().to_ascii_lowercase();
    plans.iter().find(|plan| {
        plan.path.to_ascii_lowercase().contains(&needle)
            || plan.title.to_ascii_lowercase().contains(&needle)
    })
}

fn latest_project_tasks(root: &Path) -> Vec<Value> {
    let path = root.join("core/projects/tasks/queue.jsonl");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut latest = BTreeMap::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(id) = value
            .get("id")
            .or_else(|| value.get("task_id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        latest.insert(id.to_string(), value);
    }
    let mut tasks = latest.into_values().collect::<Vec<_>>();
    tasks.sort_by_key(|task| {
        task.get("completed_at_utc")
            .or_else(|| task.get("queued_at_utc"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    });
    tasks.reverse();
    tasks
}

fn find_task<'a>(tasks: &'a [Value], target: &str) -> Option<&'a Value> {
    let needle = target.trim().to_ascii_lowercase();
    tasks.iter().find(|task| {
        task_str(task, "id").to_ascii_lowercase().contains(&needle)
            || task_str(task, "task_id")
                .to_ascii_lowercase()
                .contains(&needle)
            || task_str(task, "title")
                .to_ascii_lowercase()
                .contains(&needle)
    })
}

fn format_task_row(task: &Value) -> String {
    let id = task_id(task);
    let title = task_str(task, "title");
    let owner = task_str(task, "owner");
    let priority = task_str(task, "priority");
    let status = task_str(task, "status");
    format!("`{id}` — {status}/{priority} — {owner} — {title}")
}

fn format_task_detail(task: &Value) -> String {
    let id = task_id(task);
    let title = task_str(task, "title");
    let owner = task_str(task, "owner");
    let priority = task_str(task, "priority");
    let status = task_str(task, "status");
    let notes = task
        .get("notes")
        .or_else(|| task.get("summary"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let evidence = task
        .get("meta")
        .and_then(|meta| meta.get("evidence").or_else(|| meta.get("plan_path")))
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none recorded".to_string());
    format!(
        "**Task** `{id}`\n{title}\nstatus: `{status}` | owner: `{owner}` | priority: `{priority}`\nevidence: `{}`\n\n{}",
        short_text(&evidence, 500),
        short_text(notes, 900)
    )
}

fn task_id(task: &Value) -> String {
    task_id_for_ref(task)
}

fn task_id_for_ref(task: &Value) -> String {
    task.get("id")
        .or_else(|| task.get("task_id"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-task")
        .to_string()
}

fn task_str<'a>(task: &'a Value, key: &str) -> &'a str {
    task.get(key).and_then(Value::as_str).unwrap_or("")
}

fn append_jsonl_value(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(value).map_err(|err| err.to_string())?
    )
    .map_err(|err| format!("failed to append {}: {err}", path.display()))
}

fn slug_fragment(value: &str, limit: usize) -> String {
    let mut slug = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('_') {
            slug.push('_');
        }
        if slug.len() >= limit {
            break;
        }
    }
    slug.trim_matches('_').to_string()
}

/// Handle /help command
fn handle_help() -> String {
    "**𓀀 Annunimas Commands**

**Slash Commands:**
`/status` - System status (brief/full)
`/agents` - List all agents
`/edge` - List edge devices
`/query` - Query Oracle knowledge base
`/plans` - List or show current plan files
`/tasks` - List, show, or inspect next queued project tasks
`/task` - Propose a task or request a governed run
`/review` - Show latest completed task/review activity
`/continue` - Continue a work-stream task or thread
`/council` - Show Arandur council command seats
`/gateway` - Show gateway readiness or record a Hermes Agent background receipt
`/help` - Show this help

**Text Commands:**
`!status` - Quick status check
`!agents` - Quick agent list
`!help` - Quick help

**Soterion Icons:**
𓀀 Agent | 𓅃 Hermes | 𓂀 Athena | 𓊝 Oracle | 𓆣 Plutus | 𓋹 Apollo"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn plan_listing_discovers_current_docs_plans() {
        let dir = tempdir().expect("tempdir");
        let plans_dir = dir.path().join("docs/plans");
        fs::create_dir_all(&plans_dir).expect("plans dir");
        fs::write(
            plans_dir.join("2026-05-30-hermes-discord-gateway-unification-plan.md"),
            "# Hermes Discord Gateway Unification Plan\n\nBuild Discord into an operating room.\n",
        )
        .expect("plan write");

        let rendered = render_plans_command(dir.path(), "list", None);

        assert!(rendered.contains("Available Plans"));
        assert!(rendered.contains("Hermes Discord Gateway Unification Plan"));
        assert!(
            rendered.contains("docs/plans/2026-05-30-hermes-discord-gateway-unification-plan.md")
        );
    }

    #[test]
    fn task_listing_deduplicates_to_latest_task_state() {
        let dir = tempdir().expect("tempdir");
        let queue = dir.path().join("core/projects/tasks");
        fs::create_dir_all(&queue).expect("queue dir");
        fs::write(
            queue.join("queue.jsonl"),
            r#"{"id":"tsk_a","title":"Old state","owner":"hermes","priority":"normal","status":"queued","queued_at_utc":"2026-05-30T00:00:00Z"}
{"id":"tsk_a","title":"New state","owner":"hermes","priority":"high","status":"completed","queued_at_utc":"2026-05-30T00:00:00Z","completed_at_utc":"2026-05-30T01:00:00Z","notes":"done"}
{"id":"tsk_b","title":"Next queued","owner":"prometheus","priority":"high","status":"queued","queued_at_utc":"2026-05-30T02:00:00Z"}
"#,
        )
        .expect("queue write");

        let next = render_tasks_command(dir.path(), "next", None);
        assert!(next.contains("Next Queued Tasks"));
        assert!(next.contains("tsk_b"));
        assert!(!next.contains("Old state"));

        let detail = render_tasks_command(dir.path(), "show", Some("tsk_a"));
        assert!(detail.contains("New state"));
        assert!(detail.contains("completed"));
    }

    #[test]
    fn task_command_proposes_and_requests_run_with_receipts() {
        let dir = tempdir().expect("tempdir");

        let proposed = render_task_mutation_command(
            dir.path(),
            "propose",
            "Review Discord work stream task runner",
            Some("Keep execution governed."),
        );
        assert!(proposed.contains("Task Proposed"));
        assert!(proposed.contains("core/projects/tasks/queue.jsonl"));
        let queue = fs::read_to_string(dir.path().join("core/projects/tasks/queue.jsonl"))
            .expect("queue read");
        assert!(queue.contains("Review Discord work stream task runner"));
        assert!(queue.contains("discord_work_stream"));

        let requested = render_task_mutation_command(
            dir.path(),
            "request_run",
            "discord_work_stream_task_runner",
            Some("Run after executor policy check."),
        );
        assert!(requested.contains("Task Run Requested"));
        assert!(requested.contains("Discord did not execute terminal work directly"));
        let receipts =
            fs::read_to_string(dir.path().join("data/hermes/work_stream_requests.jsonl"))
                .expect("receipt read");
        assert!(receipts.contains("annunimas.hermes.work_stream_task_run_request.v1"));
        assert!(receipts.contains("not_started_by_discord"));

        let sessions_path = dir.path().join("data/hermes");
        fs::create_dir_all(&sessions_path).expect("sessions dir");
        fs::write(
            sessions_path.join("council_sessions.jsonl"),
            r#"{"schema_version":"annunimas.hermes.council_discussion_note.v1","note_id":"council_note_1","session_id":"council_alpha","agent":"counsel","summary":"discussion-only: promote after review"}
"#,
        )
        .expect("council note write");

        let promoted = render_task_mutation_command(
            dir.path(),
            "promote_council",
            "council_note_1",
            Some("task:tsk_a"),
        );
        assert!(promoted.contains("Council Note Promoted"));
        let sessions = fs::read_to_string(sessions_path.join("council_sessions.jsonl"))
            .expect("sessions read");
        assert!(sessions.contains("annunimas.hermes.council_discussion_promotion.v1"));
        assert!(sessions.contains("task:tsk_a"));

        let continued = render_continue_command(
            dir.path(),
            "discord_work_stream_task_runner",
            Some("Resume this in the work stream."),
        );
        assert!(continued.contains("Work Stream Continuation Queued"));
        assert!(continued.contains("task:"));
        let continuations = fs::read_to_string(
            dir.path()
                .join("data/hermes/work_stream_continuations.jsonl"),
        )
        .expect("continuation read");
        assert!(continuations.contains("annunimas.hermes.work_stream_continuation_request.v1"));
        assert!(
            continuations.contains("conversation_only_until_explicit_task_run_or_gateway_receipt")
        );
    }

    #[test]
    fn main_help_and_council_surface_retire_citadel_commands() {
        let help = handle_help();
        assert!(help.contains("/plans"));
        assert!(help.contains("/tasks"));
        assert!(help.contains("/task"));
        assert!(help.contains("/review"));
        assert!(help.contains("/continue"));
        assert!(help.contains("/council"));
        assert!(help.contains("/gateway"));
        assert!(!help.contains("/citadel"));

        let dir = tempdir().expect("tempdir");
        let council = render_council_command(dir.path(), "status", None, None);
        assert!(council.contains("Arandur"));
        assert!(council.contains("Prometheus"));
        assert!(council.contains("Counsel"));
        assert!(council.contains("Oracle"));
        assert!(council.contains("discussion-only"));
    }

    #[test]
    fn council_note_records_non_authoritative_local_summary_route() {
        let dir = tempdir().expect("tempdir");

        let rendered = render_council_command(
            dir.path(),
            "note",
            Some("Counsel suggests summarizing the Discord council thread only."),
            Some("task:discord-council-local-summary"),
        );

        assert!(rendered.contains("Council Note Recorded"));
        assert!(rendered.contains("non-authoritative"));
        assert!(rendered.contains("data/hermes/council_sessions.jsonl"));
        let sessions = fs::read_to_string(dir.path().join("data/hermes/council_sessions.jsonl"))
            .expect("sessions read");
        assert!(sessions.contains("annunimas.hermes.local_council_summary_route.v1"));
        assert!(sessions.contains("\"is_authoritative\":false"));
        assert!(sessions.contains("\"promotable\":true"));
        assert!(sessions.contains("task:discord-council-local-summary"));
        assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    }

    #[test]
    fn local_council_summary_note_can_be_projected_for_human_promotion() {
        let dir = tempdir().expect("tempdir");

        let note = render_council_command(
            dir.path(),
            "note",
            Some("Operator asked to promote this local summary only after task anchoring."),
            Some("task:discord-council-local-summary"),
        );
        assert!(note.contains("Council Note Recorded"));

        let sessions_path = dir.path().join("data/hermes/council_sessions.jsonl");
        let sessions = fs::read_to_string(&sessions_path).expect("sessions read");
        let note_id = sessions
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find_map(|value| {
                value
                    .get("note_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .expect("note id");

        let promoted = render_task_mutation_command(
            dir.path(),
            "promote_council",
            &note_id,
            Some("task:tsk_discord_council_local_summary"),
        );

        assert!(promoted.contains("Council Note Promoted"));
        let sessions = fs::read_to_string(sessions_path).expect("sessions reread");
        assert!(sessions.contains("annunimas.hermes.local_council_summary_route.v1"));
        assert!(sessions.contains("annunimas.hermes.council_discussion_promotion.v1"));
        assert!(sessions.contains("task:tsk_discord_council_local_summary"));
        assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    }

    #[test]
    fn council_promotion_projection_row_is_explicitly_non_authoritative() {
        let dir = tempdir().expect("tempdir");

        let note = render_council_command(
            dir.path(),
            "note",
            Some("Local council synthesis should remain projection-only until canonical approval."),
            Some("task:discord-council-projection-only"),
        );
        assert!(note.contains("Council Note Recorded"));

        let sessions_path = dir.path().join("data/hermes/council_sessions.jsonl");
        let sessions = fs::read_to_string(&sessions_path).expect("sessions read");
        let note_id = sessions
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find_map(|value| {
                value
                    .get("note_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .expect("note id");

        let promoted = render_task_mutation_command(
            dir.path(),
            "promote_council",
            &note_id,
            Some("task:tsk_discord_council_projection_only"),
        );
        assert!(promoted.contains("Council Note Promoted"));

        let promotion = fs::read_to_string(sessions_path)
            .expect("sessions reread")
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|value| {
                value.get("schema_version").and_then(Value::as_str)
                    == Some("annunimas.hermes.council_discussion_promotion.v1")
            })
            .expect("promotion row");
        assert_eq!(promotion["is_authoritative"], false);
        assert_eq!(promotion["canonical_write_authorized"], false);
        assert_eq!(promotion["queue_mutated"], false);
        assert_eq!(promotion["requires_human_approval"], true);
        assert_eq!(
            promotion["authority_boundary"],
            "discord_projection_only_not_canonical_authority"
        );
        assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    }

    #[test]
    fn operating_room_interaction_receipt_is_redacted_and_content_free() {
        let receipt = build_operating_room_interaction_receipt(
            "continue",
            vec!["target".to_string(), "notes".to_string()],
            "work-stream".to_string(),
            Some(442042210536521752),
            Some(1472515093848916020),
            Some(1472529224911945770),
            Some(1510348765972791456),
            128,
        );

        assert_eq!(
            receipt["schema_version"],
            "annunimas.hermes.discord_operating_room_interaction.v1"
        );
        assert_eq!(receipt["command"], "/continue");
        assert_eq!(receipt["user_id_redacted"], "redacted:1752");
        assert_eq!(
            receipt["content_policy"],
            "option_values_and_response_content_not_recorded"
        );
        assert!(is_operating_room_command("plans"));
        assert!(is_operating_room_command("gateway"));
        assert!(!is_operating_room_command("status"));
        assert!(!receipt
            .to_string()
            .contains("Resume this in the work stream"));
    }

    #[test]
    fn operating_room_interaction_receipt_classifies_channel_from_env() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("config")).expect("config dir");
        fs::write(
            dir.path().join("config/.env"),
            "DISCORD_CHANNEL_GENERAL=111\nDISCORD_CHANNEL_TASKS=222\nDISCORD_CHANNEL_COUNCIL=333\n",
        )
        .expect("env write");

        assert_eq!(
            semantic_channel_for_interaction(dir.path(), "plans", Some(222)),
            "work-stream"
        );
        assert_eq!(
            semantic_channel_for_interaction(dir.path(), "council", Some(333)),
            "council"
        );
        assert_eq!(
            semantic_channel_for_interaction(dir.path(), "council", Some(111)),
            "general"
        );
        assert_eq!(
            semantic_channel_for_interaction(dir.path(), "continue", Some(999)),
            "work-stream"
        );
    }

    #[test]
    fn gateway_activation_surface_reports_human_gated_live_state() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("docs/plans")).expect("plans dir");
        fs::create_dir_all(dir.path().join("docs/operations")).expect("ops dir");
        fs::create_dir_all(dir.path().join("config")).expect("config dir");
        fs::create_dir_all(dir.path().join("crates/annunimas-hermes/src/service"))
            .expect("service dir");
        fs::create_dir_all(dir.path().join("crates/annunimas-cli/src/commands"))
            .expect("cli commands dir");
        fs::write(
            dir.path()
                .join("docs/plans/2026-05-30-hermes-discord-gateway-unification-plan.md"),
            "plan\n",
        )
        .expect("write plan");
        fs::write(
            dir.path()
                .join("docs/operations/hermes-agent-discord-gateway-runbook.md"),
            "runbook\n",
        )
        .expect("write runbook");
        fs::write(
            dir.path()
                .join("config/hermes_agent_gateway_annunimas.example.yaml"),
            "template\n",
        )
        .expect("write template");
        fs::write(
            dir.path()
                .join("crates/annunimas-hermes/src/service/semantic_channel.rs"),
            r#""work-stream" "workstream" "tasks""#,
        )
        .expect("write semantic source");
        fs::write(
            dir.path()
                .join("crates/annunimas-cli/src/commands/utility.rs"),
            "hermes-agent-gateway-receipt\n",
        )
        .expect("write utility source");

        let rendered = render_gateway_activation_check(
            dir.path(),
            Some("Active: inactive (dead)\nUser gateway service is stopped"),
        );

        assert!(rendered.contains("Hermes Agent Gateway Activation"));
        assert!(rendered.contains("safe_local_ready_live_human_gates_pending"));
        assert!(rendered.contains("safe_local_ready: `true`"));
        assert!(rendered.contains("live_ready: `false`"));
        assert!(rendered.contains("gateway_running: `false`"));
        assert!(rendered.contains("missing live env"));
    }

    #[test]
    fn gateway_remote_confidence_surface_is_read_only_and_frames_discord_as_confidence() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("core/projects/tasks")).expect("tasks dir");
        fs::create_dir_all(dir.path().join("data/hermes")).expect("hermes dir");
        fs::create_dir_all(dir.path().join("data/flywheel")).expect("flywheel dir");
        fs::create_dir_all(dir.path().join("data/prometheus")).expect("prometheus dir");
        fs::create_dir_all(dir.path().join("core/control/autonomy")).expect("autonomy dir");
        fs::write(
            dir.path().join("core/projects/tasks/queue.jsonl"),
            concat!(
                "{\"id\":\"tsk_open\",\"title\":\"Open safe-local work\",\"status\":\"queued\",\"priority\":\"high\",\"owner\":\"prometheus\"}\n",
                "{\"id\":\"tsk_gate\",\"title\":\"Needs human approval\",\"status\":\"human_gated\",\"priority\":\"critical\",\"owner\":\"warden\",\"human_required\":true}\n"
            ),
        )
        .expect("queue write");
        fs::write(
            dir.path().join("data/hermes/hermes_agent_gateway_receipts.jsonl"),
            "{\"receipt_id\":\"hag_1\",\"status\":\"completed\",\"summary\":\"Gateway receipt complete\"}\n",
        )
        .expect("receipt write");
        fs::write(
            dir.path().join("data/prometheus/council_decisions.jsonl"),
            "{\"decision_id\":\"council_1\",\"summary\":\"Proceed with safe-local work\"}\n",
        )
        .expect("council write");
        fs::write(
            dir.path().join("data/flywheel/latest_packet.json"),
            "{\"packet_id\":\"flywheel_1\",\"status\":\"ready\"}\n",
        )
        .expect("flywheel write");
        fs::write(
            dir.path().join("core/control/autonomy/state.json"),
            "{\"mode\":\"safe_local\",\"holds\":[\"external_side_effects_human_gated\"]}\n",
        )
        .expect("autonomy write");
        let before = fs::read_to_string(dir.path().join("core/projects/tasks/queue.jsonl"))
            .expect("read queue before");

        let rendered = render_gateway_command(
            dir.path(),
            "remote_confidence",
            GatewayReceiptInput::default(),
        );
        let after = fs::read_to_string(dir.path().join("core/projects/tasks/queue.jsonl"))
            .expect("read queue after");

        assert_eq!(before, after);
        assert!(rendered.contains("Remote Confidence Snapshot"));
        assert!(rendered.contains("Discord: remote confidence surface"));
        assert!(rendered.contains("Primary consoles: ARDA HUD, Hermes Agent CLI/TUI"));
        assert!(rendered.contains("overall: attention_required"));
        assert!(rendered.contains("autonomy: safe_local"));
        assert!(rendered.contains("open tasks: 1"));
        assert!(rendered.contains("human gates: 1"));
        assert!(rendered.contains("flywheel_1"));
        assert!(rendered.contains("council_1"));
        assert!(rendered.contains("hag_1"));
        assert!(rendered.contains("service_restart=false"));
        assert!(rendered.contains("credential_change=false"));
    }

    #[test]
    fn gateway_record_receipt_writes_reviewable_subagent_evidence() {
        let dir = tempdir().expect("tempdir");
        let queue = dir.path().join("core/projects/tasks");
        fs::create_dir_all(&queue).expect("queue dir");
        fs::write(
            queue.join("queue.jsonl"),
            r#"{"id":"tsk_gateway_result","title":"Review Hermes Agent result","owner":"hermes","priority":"high","status":"queued","queued_at_utc":"2026-05-30T00:00:00Z"}
"#,
        )
        .expect("queue write");

        let rendered = render_gateway_command(
            dir.path(),
            "record_receipt",
            GatewayReceiptInput {
                task_id: Some("tsk_gateway_result".to_string()),
                background_task_id: Some("bg_42".to_string()),
                status: Some("completed".to_string()),
                summary: Some("Hermes Agent finished the requested dry-run probe.".to_string()),
                verification: Some("cargo test -p annunimas-hermes serenity_bot".to_string()),
                changed_file: Some("crates/annunimas-hermes/src/serenity_bot.rs".to_string()),
                blockers: None,
                next_action: Some("Review and close if acceptable.".to_string()),
            },
        );

        assert!(rendered.contains("Gateway Receipt Recorded"));
        assert!(rendered.contains("review_required: `false`"));
        let gateway_receipts = fs::read_to_string(
            dir.path()
                .join("data/hermes/hermes_agent_gateway_receipts.jsonl"),
        )
        .expect("gateway receipts");
        assert!(gateway_receipts.contains("annunimas.hermes_agent_gateway_background_result.v1"));
        assert!(gateway_receipts.contains("gateway_result_is_not_approval"));
        assert!(gateway_receipts.contains("bg_42"));
        let messages =
            fs::read_to_string(dir.path().join("data/hermes/messages.jsonl")).expect("messages");
        assert!(messages.contains("annunimas.hermes.subagent_completion_packet.v1"));
        assert!(messages.contains("hermes_agent_gateway"));
        let comms = fs::read_to_string(dir.path().join("data/hermes/comms_events.jsonl"))
            .expect("comms events");
        assert!(comms.contains("subagents"));
    }
}
