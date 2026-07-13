use super::*;

pub(super) fn load_personality_registry() -> std::collections::HashMap<String, serde_json::Value> {
    let path = std::env::var("ANNUNIMAS_AGENT_PERSONALITY_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("core/state/agent_personalities.json"));
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return std::collections::HashMap::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return std::collections::HashMap::new(),
    };
    value
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .unwrap_or_default()
}

pub(super) fn default_root() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_HERMES_HOME") {
        return PathBuf::from(custom);
    }
    PathBuf::from("data/hermes")
}

pub(super) fn default_world_state_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_WORLD_STATE_PATH") {
        return PathBuf::from(custom);
    }
    PathBuf::from("core/state/world.json")
}

pub(super) fn default_interrupt_policy_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_INTERRUPT_AUTH_POLICY_PATH") {
        return PathBuf::from(custom);
    }
    PathBuf::from("core/state/interrupt_authority.json")
}

pub(super) fn default_discord_runtime_state_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ANNUNIMAS_HERMES_DISCORD_RUNTIME_PATH") {
        return PathBuf::from(custom);
    }
    PathBuf::from("core/state/hermes_discord_runtime.json")
}

pub(super) fn is_permission_error(err: &AnnunimasError) -> bool {
    matches!(
        err,
        AnnunimasError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}

pub(super) fn touch(path: &Path) -> Result<()> {
    OpenOptions::new().create(true).append(true).open(path)?;
    Ok(())
}

pub(super) fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.lock_exclusive()?;
    let line = serde_json::to_string(value)?;
    let write_result = (|| -> Result<()> {
        writeln!(file, "{line}")?;
        file.sync_data()?;
        Ok(())
    })();
    let unlock_result = file.unlock().map_err(AnnunimasError::Ledger);
    write_result?;
    unlock_result?;
    Ok(())
}

pub(super) fn count_malformed_jsonl(path: &Path) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_err())
        .count()
}

pub(super) fn normalize_relationship_target(channel: &str, provider: &str) -> String {
    let source = if channel.trim().is_empty() {
        provider.trim()
    } else {
        channel.trim()
    };
    let normalized = source
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    normalized.trim_matches('_').to_string()
}

pub(super) fn env_flag_enabled(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "0" | "false" | "off" | "no" => false,
            "1" | "true" | "on" | "yes" => true,
            _ => default,
        },
        Err(_) => default,
    }
}

pub(super) fn normalize_choice(input: &str) -> Option<String> {
    let trimmed = input.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "a" | "a." | "a)" => Some("a".to_string()),
        "b" | "b." | "b)" => Some("b".to_string()),
        "c" | "c." | "c)" => Some("c".to_string()),
        _ => None,
    }
}

pub(super) fn evaluate_interrupt_authority(
    sender: &str,
    disposition: &InterruptionDisposition,
) -> (bool, String) {
    let mut policy = serde_json::json!({
        "default": {"allow": ["note"]},
        "senders": {
            "operator": {"allow": ["note", "reroute"]},
            "illuvatar": {"allow": ["note", "reroute", "override"]}
        }
    });
    let path = default_interrupt_policy_path();
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
            policy = parsed;
        }
    }
    let key = match disposition {
        InterruptionDisposition::Note => "note",
        InterruptionDisposition::Reroute => "reroute",
        InterruptionDisposition::Override => "override",
    };
    let sender_l = sender.to_ascii_lowercase();
    let allow_sender = policy
        .get("senders")
        .and_then(|v| v.get(&sender_l))
        .and_then(|v| v.get("allow"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some(key)));
    let allowed = allow_sender.unwrap_or_else(|| {
        policy
            .get("default")
            .and_then(|v| v.get("allow"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|v| v.as_str() == Some(key)))
            .unwrap_or(false)
    });
    let reason = if allowed {
        format!("policy_allowed sender={} disposition={}", sender_l, key)
    } else {
        format!("policy_denied sender={} disposition={}", sender_l, key)
    };
    (allowed, reason)
}

pub(super) fn classify_interruption_intent(content: &str) -> (InterruptionDisposition, String) {
    let normalized = content.to_ascii_lowercase();
    let override_signals = ["override", "stop now", "abort", "cancel", "kill process"];
    if override_signals.iter().any(|s| normalized.contains(s)) {
        return (
            InterruptionDisposition::Override,
            "matched override control keyword".to_string(),
        );
    }
    let reroute_signals = ["reroute", "switch", "instead", "change to", "route to"];
    if reroute_signals.iter().any(|s| normalized.contains(s)) {
        return (
            InterruptionDisposition::Reroute,
            "matched reroute directive keyword".to_string(),
        );
    }
    (
        InterruptionDisposition::Note,
        "defaulted to note capture (no override/reroute signal)".to_string(),
    )
}

pub(super) fn read_jsonl_lines(path: &Path) -> Vec<serde_json::Value> {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect()
}

pub(super) fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<serde_json::Value> {
    let mut lines = read_jsonl_lines(path);
    lines.reverse();
    lines.truncate(limit.max(1));
    lines
}

pub(super) fn active_agents_from_world_state() -> usize {
    let content = match fs::read_to_string(default_world_state_path()) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    value
        .get("agents")
        .and_then(|v| v.as_array())
        .map(|agents| {
            agents
                .iter()
                .filter(|agent| {
                    let status = agent
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                    let active_tasks = agent
                        .get("active_tasks")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    active_tasks > 0 || status == "online"
                })
                .count()
        })
        .unwrap_or(0)
}
