use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticChannel {
    pub name: String,
    pub env_key: String,
    pub fallback: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticChannelResolution {
    pub requested: String,
    pub semantic_channel: String,
    pub env_key: String,
    pub discord_recipient: String,
    pub fallback_used: bool,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordChannelPlanEntry {
    pub semantic_channel: String,
    pub required_name: String,
    pub env_key: String,
    pub purpose: String,
    pub existing_channel_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordChannelPermissionSummary {
    pub source: String,
    pub manage_channels: bool,
    pub can_create_channels: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordChannelPlan {
    pub schema_version: String,
    pub mode: String,
    pub guild_id: Option<String>,
    pub category_id: Option<String>,
    pub required_channels: Vec<DiscordChannelPlanEntry>,
    pub existing_channel_count: usize,
    pub missing_channel_count: usize,
    pub permission_summary: DiscordChannelPermissionSummary,
    pub secrets_redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordChannelDryRunReceipt {
    pub schema_version: String,
    pub dry_run: bool,
    pub approved: bool,
    pub would_create: Vec<DiscordChannelPlanEntry>,
    pub blocked_reason: Option<String>,
    pub mutation_performed: bool,
    pub permission_summary: DiscordChannelPermissionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArdaHudProjection {
    pub schema_version: String,
    pub surface: String,
    pub semantic_channel: String,
    pub panel: String,
    pub state_key: String,
    pub ui_identity: String,
    pub risk_class: String,
    pub trust_boundary: String,
    pub source_map_path: String,
    pub triage_registry_path: String,
    pub subscribable: bool,
    pub canonical_refs: Vec<String>,
    #[serde(default)]
    pub external_refs: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArdaHudProjectionContract {
    pub schema_version: String,
    pub surface: String,
    pub risk_class: String,
    pub source_map_path: String,
    pub triage_registry_path: String,
    pub subscriptions: Vec<ArdaHudProjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArdaHudProjectionStateReceipt {
    pub schema_version: String,
    pub contract_path: String,
    pub source_map_path: String,
    pub triage_registry_path: String,
    pub risk_class: String,
    pub subscription_count: usize,
    pub persisted_at_utc: String,
}

impl HermesService {
    pub fn semantic_channel_registry(&self) -> Vec<SemanticChannel> {
        semantic_channel_registry()
    }

    pub fn resolve_semantic_discord_channel(
        &self,
        requested: &str,
    ) -> Result<SemanticChannelResolution> {
        resolve_semantic_discord_channel(requested)
    }

    pub fn render_arda_hud_channel_projection(
        &self,
        requested: &str,
        discord_thread_id: Option<&str>,
    ) -> Result<ArdaHudProjection> {
        render_arda_hud_channel_projection(requested, discord_thread_id)
    }

    pub fn arda_hud_projection_contract(&self) -> ArdaHudProjectionContract {
        let subscriptions = semantic_channel_registry()
            .into_iter()
            .map(|channel| render_arda_hud_projection_for_channel(&channel, None))
            .collect();
        ArdaHudProjectionContract {
            schema_version: "annunimas.hermes.arda_hud_projection_contract.v1".to_string(),
            surface: "arda".to_string(),
            risk_class: "low_risk_projection".to_string(),
            source_map_path: arda_source_map_path(),
            triage_registry_path: knowledge_triage_registry_path(),
            subscriptions,
        }
    }

    pub fn persist_arda_hud_projection_contract(&self) -> Result<ArdaHudProjectionStateReceipt> {
        let contract = self.arda_hud_projection_contract();
        let contract_path = arda_projection_contract_path_buf_for_root(&self.root);
        let source_map_path = arda_source_map_path_buf_for_root(&self.root);
        let triage_registry_path = knowledge_triage_registry_path_buf_for_root(&self.root);

        if let Some(parent) = contract_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &contract_path,
            format!("{}\n", serde_json::to_string_pretty(&contract)?),
        )?;

        upsert_arda_source_map_projection(&source_map_path, &contract_path, &triage_registry_path)?;
        append_arda_projection_triage_entries(&triage_registry_path, &contract)?;

        Ok(ArdaHudProjectionStateReceipt {
            schema_version: "annunimas.hermes.arda_hud_projection_state_receipt.v1".to_string(),
            contract_path: contract_path.to_string_lossy().to_string(),
            source_map_path: source_map_path.to_string_lossy().to_string(),
            triage_registry_path: triage_registry_path.to_string_lossy().to_string(),
            risk_class: contract.risk_class,
            subscription_count: contract.subscriptions.len(),
            persisted_at_utc: Utc::now().to_rfc3339(),
        })
    }

    pub fn discord_channel_plan(&self) -> DiscordChannelPlan {
        discord_channel_plan()
    }

    pub fn apply_discord_channel_plan_dry_run(
        &self,
        approved: bool,
    ) -> DiscordChannelDryRunReceipt {
        let plan = discord_channel_plan();
        let would_create = plan
            .required_channels
            .iter()
            .filter(|entry| entry.status == "missing")
            .cloned()
            .collect::<Vec<_>>();
        let blocked_reason = if !approved {
            Some("operator_approval_required".to_string())
        } else if !plan.permission_summary.can_create_channels {
            Some(plan.permission_summary.reason.clone())
        } else {
            None
        };
        DiscordChannelDryRunReceipt {
            schema_version: "annunimas.hermes.discord_channel_dry_run_receipt.v1".to_string(),
            dry_run: true,
            approved,
            would_create,
            blocked_reason,
            mutation_performed: false,
            permission_summary: plan.permission_summary,
        }
    }
}

fn semantic_channel_registry() -> Vec<SemanticChannel> {
    vec![
        semantic_channel(
            "general",
            "general",
            "low-risk system awareness and bridge status",
        ),
        semantic_channel(
            "work-stream",
            "tasks",
            "live operator work stream for back-and-forth task and plan review",
        ),
        semantic_channel(
            "tasks",
            "boardroom",
            "task proposals, approvals, pivots, and completion summaries",
        ),
        semantic_channel(
            "subagents",
            "tasks",
            "bounded subagent completion and review packets",
        ),
        semantic_channel(
            "council",
            "boardroom",
            "AI-agent council discussion and quorum summaries",
        ),
        semantic_channel(
            "research-forge",
            "general",
            "research and forge updates safe for operator visibility",
        ),
        semantic_channel(
            "governance-audit",
            "boardroom",
            "WARDEN/Soterion policy, audit, and safety evidence",
        ),
    ]
}

fn semantic_channel(name: &str, fallback: &str, purpose: &str) -> SemanticChannel {
    SemanticChannel {
        name: name.to_string(),
        env_key: discord_env_key(name),
        fallback: fallback.to_string(),
        purpose: purpose.to_string(),
    }
}

fn discord_channel_plan() -> DiscordChannelPlan {
    let entries = semantic_channel_registry()
        .into_iter()
        .map(|channel| {
            let existing_channel_id = std::env::var(&channel.env_key)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|_| "[REDACTED]".to_string());
            let status = if existing_channel_id.is_some() {
                "exists"
            } else {
                "missing"
            }
            .to_string();
            DiscordChannelPlanEntry {
                semantic_channel: channel.name.clone(),
                required_name: format!("annunimas-{}", channel.name),
                env_key: channel.env_key,
                purpose: channel.purpose,
                existing_channel_id,
                status,
            }
        })
        .collect::<Vec<_>>();
    let existing_channel_count = entries
        .iter()
        .filter(|entry| entry.status == "exists")
        .count();
    let missing_channel_count = entries.len().saturating_sub(existing_channel_count);
    DiscordChannelPlan {
        schema_version: "annunimas.hermes.discord_channel_plan.v1".to_string(),
        mode: "read_only_discovery".to_string(),
        guild_id: redacted_env_value("DISCORD_GUILD_ID"),
        category_id: redacted_env_value("DISCORD_CATEGORY_ID"),
        required_channels: entries,
        existing_channel_count,
        missing_channel_count,
        permission_summary: discord_channel_permission_summary(),
        secrets_redacted: true,
    }
}

fn discord_channel_permission_summary() -> DiscordChannelPermissionSummary {
    let explicit_manage = std::env::var("DISCORD_MANAGE_CHANNELS")
        .ok()
        .map(|value| parse_boolish(&value));
    let manage_channels = explicit_manage
        .or_else(|| {
            std::env::var("DISCORD_BOT_PERMISSIONS")
                .ok()
                .map(|value| value.to_ascii_uppercase().contains("MANAGE_CHANNELS"))
        })
        .unwrap_or(false);
    let source = if explicit_manage.is_some() {
        "DISCORD_MANAGE_CHANNELS".to_string()
    } else if std::env::var_os("DISCORD_BOT_PERMISSIONS").is_some() {
        "DISCORD_BOT_PERMISSIONS".to_string()
    } else {
        "not_configured".to_string()
    };
    let reason = if manage_channels {
        "manage_channels_permission_declared".to_string()
    } else {
        "manage_channels_permission_not_declared".to_string()
    };
    DiscordChannelPermissionSummary {
        source,
        manage_channels,
        can_create_channels: manage_channels,
        reason,
    }
}

fn parse_boolish(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on" | "manage_channels"
    )
}

fn redacted_env_value(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|_| "[REDACTED]".to_string())
}

fn resolve_semantic_discord_channel(requested: &str) -> Result<SemanticChannelResolution> {
    let requested = normalize_semantic_channel(requested);
    let registry = semantic_channel_registry();
    let channel = registry
        .iter()
        .find(|entry| entry.name == requested)
        .ok_or_else(|| {
            ArdaError::Task(format!(
                "unknown semantic Discord channel: {requested}; expected one of general, work-stream, tasks, subagents, council, research-forge, governance-audit"
            ))
        })?;
    let configured = std::env::var(&channel.env_key)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let discord_recipient = if configured {
        channel.name.clone()
    } else {
        channel.fallback.clone()
    };
    Ok(SemanticChannelResolution {
        requested,
        semantic_channel: channel.name.clone(),
        env_key: channel.env_key.clone(),
        discord_recipient,
        fallback_used: !configured,
        configured,
    })
}

fn render_arda_hud_channel_projection(
    requested: &str,
    discord_thread_id: Option<&str>,
) -> Result<ArdaHudProjection> {
    let requested = normalize_semantic_channel(requested);
    let registry = semantic_channel_registry();
    let channel = registry
        .iter()
        .find(|entry| entry.name == requested)
        .ok_or_else(|| {
            ArdaError::Task(format!(
                "unknown ARDA HUD semantic channel: {requested}; expected one of general, work-stream, tasks, subagents, council, research-forge, governance-audit"
            ))
        })?;
    Ok(render_arda_hud_projection_for_channel(
        channel,
        discord_thread_id,
    ))
}

fn render_arda_hud_projection_for_channel(
    channel: &SemanticChannel,
    discord_thread_id: Option<&str>,
) -> ArdaHudProjection {
    let panel = arda_panel_for_semantic_channel(&channel.name);
    let state_key = arda_state_key(&channel.name);
    let mut external_refs = std::collections::BTreeMap::new();
    if let Some(thread_id) = discord_thread_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        external_refs.insert(
            "discord_thread_id".to_string(),
            serde_json::Value::String(thread_id.to_string()),
        );
    }
    ArdaHudProjection {
        schema_version: "annunimas.hermes.arda_hud_projection.v1".to_string(),
        surface: "arda".to_string(),
        semantic_channel: channel.name.clone(),
        panel: panel.clone(),
        state_key: state_key.clone(),
        ui_identity: format!("arda:{panel}:{}", channel.name),
        risk_class: "low_risk_projection".to_string(),
        trust_boundary: "semantic_projection_only_external_identity_noncanonical".to_string(),
        source_map_path: arda_source_map_path(),
        triage_registry_path: knowledge_triage_registry_path(),
        subscribable: true,
        canonical_refs: vec![
            format!("semantic_channel:{}", channel.name),
            format!("state:{state_key}"),
        ],
        external_refs,
    }
}

fn arda_panel_for_semantic_channel(semantic_channel: &str) -> String {
    match semantic_channel {
        "council" | "governance-audit" => "boardroom".to_string(),
        "work-stream" | "tasks" | "subagents" => "workstation".to_string(),
        _ => "world".to_string(),
    }
}

fn arda_state_key(semantic_channel: &str) -> String {
    format!("hermes.semantic_channel.{semantic_channel}")
}

fn arda_source_map_path() -> String {
    arda_source_map_path_buf().to_string_lossy().to_string()
}

fn arda_source_map_path_buf() -> PathBuf {
    std::env::var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("core/state/arda_source_map.json"))
}

fn arda_source_map_path_buf_for_root(root: &Path) -> PathBuf {
    std::env::var("ANNUNIMAS_ARDA_SOURCE_MAP_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_state_root_for_service_root(root).join("arda_source_map.json"))
}

fn knowledge_triage_registry_path() -> String {
    knowledge_triage_registry_path_buf()
        .to_string_lossy()
        .to_string()
}

fn knowledge_triage_registry_path_buf() -> PathBuf {
    std::env::var("ANNUNIMAS_TRIAGE_REGISTRY_PATH")
        .or_else(|_| std::env::var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("core/state/knowledge_triage_registry.jsonl"))
}

fn knowledge_triage_registry_path_buf_for_root(root: &Path) -> PathBuf {
    std::env::var("ANNUNIMAS_TRIAGE_REGISTRY_PATH")
        .or_else(|_| std::env::var("ANNUNIMAS_KNOWLEDGE_TRIAGE_REGISTRY_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            default_state_root_for_service_root(root).join("knowledge_triage_registry.jsonl")
        })
}

fn arda_projection_contract_path_buf_for_root(root: &Path) -> PathBuf {
    std::env::var("ANNUNIMAS_HERMES_ARDA_PROJECTION_CONTRACT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            default_state_root_for_service_root(root).join("hermes_arda_projection_contract.json")
        })
}

fn default_state_root_for_service_root(root: &Path) -> PathBuf {
    if root.file_name().and_then(|name| name.to_str()) == Some("hermes") {
        if let Some(data_dir) = root.parent() {
            if data_dir.file_name().and_then(|name| name.to_str()) == Some("data") {
                if let Some(workspace_root) = data_dir.parent() {
                    return workspace_root.join("core/state");
                }
            }
        }
    }
    root.join("core/state")
}

fn upsert_arda_source_map_projection(
    source_map_path: &Path,
    contract_path: &Path,
    triage_registry_path: &Path,
) -> Result<()> {
    if let Some(parent) = source_map_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing_source_map = fs::read_to_string(source_map_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let mut source_map = existing_source_map.clone().unwrap_or_else(|| {
        serde_json::json!({
            "schema_version": "annunimas.core.state.v1",
            "sections": []
        })
    });

    if !source_map.is_object() {
        source_map = serde_json::json!({
            "schema_version": "annunimas.core.state.v1",
            "sections": []
        });
    }

    let source_map_object = source_map.as_object_mut().ok_or_else(|| {
        ArdaError::Task("ARDA source map root must be a JSON object".to_string())
    })?;
    source_map_object
        .entry("schema_version".to_string())
        .or_insert_with(|| serde_json::Value::String("annunimas.core.state.v1".to_string()));
    let sections_value = source_map_object
        .entry("sections".to_string())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    if !sections_value.is_array() {
        *sections_value = serde_json::Value::Array(Vec::new());
    }
    let sections = sections_value.as_array_mut().ok_or_else(|| {
        ArdaError::Task("ARDA source map sections must be an array".to_string())
    })?;
    sections.retain(|section| {
        section.get("id").and_then(|value| value.as_str()) != Some("hermes_arda_projection")
    });
    sections.push(serde_json::json!({
        "id": "hermes_arda_projection",
        "title": "Hermes ARDA Projection",
        "owner": "hermes",
        "status": "ready",
        "arda_panels": ["boardroom", "workstation", "world"],
        "primary_sources": [contract_path.to_string_lossy().to_string()],
        "supplemental_sources": [triage_registry_path.to_string_lossy().to_string()],
        "risk_class": "low_risk_projection",
        "trust_boundary": "semantic_projection_only_external_identity_noncanonical"
    }));

    if existing_source_map.as_ref() == Some(&source_map) {
        return Ok(());
    }

    fs::write(
        source_map_path,
        format!("{}\n", serde_json::to_string_pretty(&source_map)?),
    )?;
    Ok(())
}

fn append_arda_projection_triage_entries(
    triage_registry_path: &Path,
    contract: &ArdaHudProjectionContract,
) -> Result<()> {
    if let Some(parent) = triage_registry_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let observed_at_utc = Utc::now().to_rfc3339();
    for subscription in &contract.subscriptions {
        let entry = serde_json::json!({
            "schema_version": "annunimas.hermes.arda_projection_triage.v1",
            "observed_at_utc": observed_at_utc,
            "source": "annunimas-hermes",
            "state_key": subscription.state_key,
            "semantic_channel": subscription.semantic_channel,
            "ui_identity": subscription.ui_identity,
            "panel": subscription.panel,
            "surface": subscription.surface,
            "risk_class": subscription.risk_class,
            "trust_boundary": subscription.trust_boundary,
            "source_map_path": contract.source_map_path,
            "contract_schema_version": contract.schema_version,
            "canonical_refs": subscription.canonical_refs,
            "lifecycle_status": "projection_contract_registered"
        });
        if arda_projection_triage_entry_exists(triage_registry_path, &entry) {
            continue;
        }
        append_jsonl(triage_registry_path, &entry)?;
    }
    Ok(())
}

fn arda_projection_triage_entry_exists(
    triage_registry_path: &Path,
    candidate: &serde_json::Value,
) -> bool {
    let Ok(raw) = fs::read_to_string(triage_registry_path) else {
        return false;
    };

    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .any(|existing| arda_projection_triage_entries_semantically_equal(&existing, candidate))
}

fn arda_projection_triage_entries_semantically_equal(
    left: &serde_json::Value,
    right: &serde_json::Value,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    if let Some(obj) = left.as_object_mut() {
        obj.remove("observed_at_utc");
    }
    if let Some(obj) = right.as_object_mut() {
        obj.remove("observed_at_utc");
    }
    left == right
}

fn normalize_semantic_channel(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "workstream" | "work" | "work-room" | "workroom" => "work-stream".to_string(),
        "boardroom" | "ops-boardroom" | "ceo-boardroom" => "tasks".to_string(),
        "client-bridge" => "general".to_string(),
        _ => normalized,
    }
}

fn discord_env_key(name: &str) -> String {
    format!(
        "DISCORD_CHANNEL_{}",
        name.trim().to_ascii_uppercase().replace(['-', ' '], "_")
    )
}
