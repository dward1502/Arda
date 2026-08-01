#![cfg(feature = "full-cli")]
use super::read_json_file;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(super) fn summarize_json_file(path: &Path) -> Value {
    let parsed = read_json_file(path.to_path_buf()).unwrap_or_else(|| json!({}));
    let rel = path.display().to_string();
    json!({
        "path": rel,
        "exists": path.exists(),
        "top_level_keys": parsed.as_object().map(|map| map.keys().take(12).cloned().collect::<Vec<_>>()).unwrap_or_default(),
        "body_preview": parsed.to_string().chars().take(240).collect::<String>()
    })
}

pub(super) fn summarize_env_file(path: &Path) -> Value {
    let content = fs::read_to_string(path).unwrap_or_default();
    let keys = content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || !trimmed.contains('=') {
                return None;
            }
            Some(
                trimmed
                    .split('=')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();
    json!({
        "path": path.display().to_string(),
        "exists": path.exists(),
        "keys_total": keys.len(),
        "keys": keys
    })
}

pub(super) fn collect_athena_repo_source_map(books_root: &Path) -> HashMap<String, String> {
    let mut sources = HashMap::new();
    let Ok(entries) = fs::read_dir(books_root) else {
        return sources;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(source_id) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            let title = value
                .get("data")
                .and_then(|data| data.get("title"))
                .and_then(Value::as_str);
            if let Some(title) = title.filter(|title| title.starts_with("https://github.com/")) {
                sources
                    .entry(title.to_string())
                    .or_insert_with(|| source_id.to_string());
                break;
            }
        }
    }
    sources
}

pub(super) fn read_latest_policy_readiness(path: &Path) -> HashMap<String, Value> {
    let mut readiness = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return readiness;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let Some(source_id) = value.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        readiness.insert(source_id.to_string(), value);
    }
    readiness
}

pub(super) fn package_provider_id(tool: &str) -> Option<&'static str> {
    match tool {
        "litellm" => Some("litellm_gateway"),
        _ => None,
    }
}

pub(super) fn package_runtime_surface_key(tool: &str) -> &str {
    match tool {
        "playwright-mcp" => "playwright_mcp",
        "oh-my-opencode" => "oh_my_opencode",
        _ => tool,
    }
}

pub(super) fn package_required_shared_env_keys(tool: &str) -> Vec<&'static str> {
    match tool {
        "litellm" => vec!["LITELLM_API_KEY"],
        _ => Vec::new(),
    }
}

pub(super) fn package_required_runtime_env_keys(tool: &str) -> Vec<&'static str> {
    match tool {
        "litellm" => vec!["LITELLM_PROXY_URL"],
        "crawl4ai" => vec!["ARDA_CRAWL4AI_URL"],
        "playwright-mcp" => vec!["ARDA_PLAYWRIGHT_MCP_CMD"],
        "nanoclaw" => vec![
            "ARDA_NANOCLAW_ROOT",
            "ARDA_NANOCLAW_EDGE_TARGET",
            "ARDA_NANOCLAW_EDGE_TRANSPORT",
        ],
        _ => Vec::new(),
    }
}

pub(super) fn package_integration_lane(tool: &str, meta: &Value) -> &'static str {
    match tool {
        "litellm" => "manwe_provider",
        "crawl4ai" => "athena_ingestion",
        "playwright-mcp" => "mcp_browser",
        "discord-mcp" => "mcp_communications",
        "nanoclaw" => "edge_runtime",
        "llmfit" => "model_selection",
        _ => match meta
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
        {
            "agent-framework" => "agent_framework",
            "agent-skills" => "agent_skills",
            "knowledge" => "knowledge",
            _ => "research",
        },
    }
}

pub(super) fn package_activation_status(
    tool: &str,
    integration_state: &str,
    provider_configured: bool,
    runtime_surface: &Value,
) -> &'static str {
    let runtime_status = runtime_surface
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let runtime_ok = runtime_surface
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match tool {
        "litellm" if provider_configured && runtime_status == "running" && runtime_ok => {
            "active_in_system"
        }
        "crawl4ai" if runtime_status == "running" && runtime_ok => "active_in_system",
        "llmfit" if runtime_status == "ready" && runtime_ok => "active_signal",
        "oh-my-opencode"
            if runtime_status == "ready"
                && runtime_ok
                && runtime_surface
                    .get("route_contract_ready")
                    .and_then(Value::as_bool)
                    == Some(true) =>
        {
            "active_signal"
        }
        "playwright-mcp" if runtime_status == "contract_ready" && runtime_ok => {
            "governed_on_demand"
        }
        "nanoclaw"
            if runtime_surface.get("auth_ready").and_then(Value::as_bool) == Some(false)
                && runtime_surface.get("control_mode").and_then(Value::as_str)
                    == Some("whatsapp") =>
        {
            "blocked_on_auth"
        }
        "nanoclaw" if runtime_status == "contract_ready" && runtime_ok => "governed_on_demand",
        _ if integration_state == "ready_for_activation" => "activation_frontier",
        _ if integration_state == "configuration_ready" => "configuration_frontier",
        _ => "planned",
    }
}

pub(super) fn package_next_action(
    integration_state: &str,
    activation_status: &str,
    tool: &str,
) -> &'static str {
    match (tool, activation_status) {
        ("litellm", "active_in_system") => {
            "LiteLLM is already live in MANWE; keep provider health, models, and gateway policy aligned"
        }
        ("crawl4ai", "active_in_system") => {
            "ATHENA can already ingest through crawl4ai; use `arda athena crawl <url>` when capture is needed"
        }
        ("llmfit", "active_signal") => {
            "llmfit recommendations are already visible to MANWE route policy; tune route heuristics rather than wiring a new runtime"
        }
        ("oh-my-opencode", "active_signal") => {
            "OpenCode is already bounded by sovereign route contracts; tune agent route mappings rather than treating it as an unintegrated package"
        }
        ("playwright-mcp", "governed_on_demand") => {
            "Start the supervised bridge only for governed browser sessions; keep it on-demand rather than daemonized"
        }
        ("nanoclaw", "blocked_on_auth") => {
            "Complete NanoClaw channel authentication or edge enrollment before starting the runtime"
        }
        _ => match (tool, integration_state) {
            ("litellm", "ready_for_activation") => {
                "set MANWE litellm_gateway enabled=true and point the proxy URL at the live gateway"
            }
            ("litellm", _) => "complete LiteLLM env contract and provider activation path",
            ("crawl4ai", "ready_for_activation") => {
                "run `arda athena crawl <url>` to capture markdown into ATHENA via the local crawl4ai service"
            }
            ("crawl4ai", "evidence_ready") => {
                "add crawler execution adapter and authenticated ingestion path"
            }
            ("playwright-mcp", "ready_for_activation") => {
                "start the supervised bridge and expose the governed browser session tool through arda-mcp"
            }
            ("playwright-mcp", "evidence_ready") => {
                "define MCP browser contract and bridge process supervision"
            }
            ("nanoclaw", "ready_for_activation") => {
                "run `bash scripts/runtime/nanoclaw_runtime.sh start` after channel auth is present, or route NanoClaw to the configured Tailscale edge target"
            }
            ("nanoclaw", "configuration_ready") => {
                "complete NanoClaw channel authentication or edge enrollment to promote the contract into live runtime use"
            }
            ("nanoclaw", "evidence_ready") => "formalize NanoClaw runtime contract, local preflight, and Tailscale edge enrollment",
            ("llmfit", "evidence_ready") => "feed model-fit recommendations into MANWE route policy",
            _ => "promote from evidence into a bounded runtime or product surface",
        },
    }
}

pub(super) fn read_env_assignment(path: &Path, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (found_key, value) = trimmed.split_once('=')?;
        if found_key.trim() != key {
            return None;
        }
        Some(value.trim().to_string())
    })
}

pub(super) fn summarize_field_count_value(values: &[Value], field: &str, needle: &str) -> usize {
    values
        .iter()
        .filter(|value| {
            value
                .get(field)
                .and_then(Value::as_str)
                .is_some_and(|status| status.eq_ignore_ascii_case(needle))
        })
        .count()
}

pub(super) fn summarize_description(description: Option<&str>) -> String {
    let Some(description) = description else {
        return "No description available.".to_string();
    };
    let normalized = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "No description available.".to_string();
    }
    normalized
        .split('.')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("{s}."))
        .unwrap_or(normalized)
}

pub(super) fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<Value> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let mut values = Vec::new();
    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            values.push(value);
            if values.len() >= limit {
                break;
            }
        }
    }
    values.reverse();
    values
}

pub(super) fn read_all_jsonl(path: &Path) -> Vec<Value> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

pub(super) fn latest_jsonl_entries_by_id(path: &Path) -> Vec<Value> {
    let mut order: Vec<String> = Vec::new();
    let mut rows: HashMap<String, Value> = HashMap::new();
    for entry in read_all_jsonl(path) {
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        match rows.entry(id.to_string()) {
            std::collections::hash_map::Entry::Vacant(e) => {
                order.push(e.key().clone());
                e.insert(entry);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                *e.get_mut() = entry;
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| rows.remove(&id))
        .collect()
}

pub(super) fn latest_jsonl_entries_by_source_id(
    path: &Path,
    source_ids: &[&str],
) -> HashMap<String, Value> {
    let wanted = source_ids
        .iter()
        .map(|id| (*id).to_string())
        .collect::<std::collections::HashSet<_>>();
    let mut rows = HashMap::new();
    for entry in read_all_jsonl(path) {
        let Some(source_id) = entry.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        if wanted.contains(source_id) {
            rows.insert(source_id.to_string(), entry);
        }
    }
    rows
}

pub(super) fn latest_task_rows_by_id(path: &Path, task_ids: &[&str]) -> HashMap<String, Value> {
    let wanted = task_ids
        .iter()
        .map(|id| (*id).to_string())
        .collect::<std::collections::HashSet<_>>();
    let mut rows = HashMap::new();
    for entry in read_all_jsonl(path) {
        let Some(task_id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        if wanted.contains(task_id) {
            rows.insert(task_id.to_string(), entry);
        }
    }
    rows
}

pub(super) fn escalation_dedupe_key(row: &Value) -> String {
    let reason = row
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if reason == "policy_guard.denied" {
        let command = row
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return format!("{reason}|{command}");
    }
    if reason.starts_with("core_pressure_guard.") {
        return reason.to_string();
    }
    let task_id = row
        .get("task_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!("{reason}|{task_id}")
}

pub(super) fn count_jsonl_lines(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}

pub(super) fn read_recent_mnemosyne_episodic(root: &Path, limit: usize) -> Vec<Value> {
    let mut entries = Vec::new();
    let months = match fs::read_dir(root) {
        Ok(months) => months,
        Err(_) => return entries,
    };
    for month in months.flatten() {
        let month_path = month.path();
        if !month_path.is_dir() {
            continue;
        }
        let files = match fs::read_dir(month_path) {
            Ok(files) => files,
            Err(_) => continue,
        };
        for file in files.flatten() {
            let path = file.path();
            if path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
                continue;
            }
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let mut lines = content.lines();
            let _header = lines.next();
            let Some(body_line) = lines.next() else {
                continue;
            };
            if let Ok(body) = serde_json::from_str::<Value>(body_line) {
                entries.push(body);
            }
        }
    }
    entries.sort_by(|a, b| {
        let a_ts = a.get("ts_utc").and_then(Value::as_str).unwrap_or_default();
        let b_ts = b.get("ts_utc").and_then(Value::as_str).unwrap_or_default();
        a_ts.cmp(b_ts)
    });
    let keep = limit.max(1);
    if entries.len() > keep {
        entries = entries.split_off(entries.len() - keep);
    }
    entries
}

pub(super) fn summarize_field_counts(events: &[Value], field: &str) -> Value {
    let mut counts = serde_json::Map::new();
    for value in events {
        let key = value
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let current = counts.get(&key).and_then(Value::as_u64).unwrap_or(0);
        counts.insert(key, Value::from(current + 1));
    }
    Value::Object(counts)
}

pub(super) fn latest_events_by_key(
    events: &[Value],
    filter_field: &str,
    filter_value: &str,
    group_field: &str,
) -> Value {
    let mut grouped = serde_json::Map::new();
    for event in events {
        if event.get(filter_field).and_then(Value::as_str) != Some(filter_value) {
            continue;
        }
        let Some(group) = event.get(group_field).and_then(Value::as_str) else {
            continue;
        };
        grouped.insert(group.to_string(), event.clone());
    }
    Value::Object(grouped)
}

pub(super) fn count_bool_field(events: &[Value], field: &str, target: bool) -> usize {
    events
        .iter()
        .filter(|value| value.get(field).and_then(Value::as_bool) == Some(target))
        .count()
}

pub(super) fn is_expired_rfc3339(ts: Option<&str>) -> bool {
    let Some(ts) = ts else {
        return false;
    };
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|value| value.with_timezone(&Utc) < Utc::now())
        .unwrap_or(false)
}
