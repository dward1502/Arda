#![cfg(feature = "full-cli")]
use super::*;

pub(crate) async fn run_maintenance_cycle(
    sweep_type: &str,
    cooldown_seconds: i64,
    prune: bool,
    prune_threshold_pct: u8,
) -> anyhow::Result<serde_json::Value> {
    let prometheus = PrometheusService::from_core("core")?;
    let athena = AthenaStore::from_default_or_workspace_fallback()?;
    let hades = HadesService::from_default_or_fallback()?;
    let manwe = ManweService::from_default_or_fallback()?;
    let mnemosyne = MnemosyneService::from_default_or_fallback()?;
    let hermes = HermesService::from_default_or_fallback()?;
    let plutus = PlutusService::from_default_or_workspace_fallback()?;

    let sweep = hades.sweep(sweep_type, None)?;
    let hades_status = hades.status()?;
    let prometheus_status = prometheus.status()?;
    let athena_status = athena.status()?;
    let hermes_status = hermes.status().await?;
    let mnemosyne_status = mnemosyne.status()?;
    let plutus_status = plutus.status().await?;

    let mut auto_cooldown = Vec::new();
    for provider in manwe.providers().await {
        if provider.enabled && provider.consecutive_failures >= 3 && !provider.in_cooldown {
            manwe
                .mark_provider_cooldown(&provider.id, cooldown_seconds)
                .await?;
            auto_cooldown.push(provider.id);
        }
    }

    let manwe_status = manwe.status().await?;
    let home = home_root();
    let var_disk_pct = disk_usage_percent(home.to_string_lossy().as_ref());
    let ruleset = load_active_ruleset();
    let system_control = load_system_control_state();
    let governance_observation = build_governance_observation(
        &serde_json::to_value(&prometheus_status)?,
        &serde_json::to_value(&hermes_status)?,
        &serde_json::to_value(&manwe_status)?,
        &serde_json::to_value(&hades_status)?,
        &serde_json::to_value(&athena_status)?,
        &plutus_status,
        &mnemosyne_status,
        var_disk_pct,
        &ruleset,
        &system_control,
    );
    persist_governance_observation(&governance_observation);
    let queue_observability = queue_observability_snapshot();
    persist_queue_observability(&queue_observability);
    let mut issues = Vec::new();
    if manwe_status.providers_degraded > 0 {
        issues.push(format!(
            "manwe_degraded_providers={}",
            manwe_status.providers_degraded
        ));
    }
    if manwe_status.providers_exhausted > 0 {
        issues.push(format!(
            "manwe_exhausted_providers={}",
            manwe_status.providers_exhausted
        ));
    }
    if hades_status.pending_actions >= 25 {
        issues.push(format!(
            "hades_pending_actions={}",
            hades_status.pending_actions
        ));
    }
    if hades_status.orphans_active >= 10 {
        issues.push(format!(
            "hades_orphans_active={}",
            hades_status.orphans_active
        ));
    }
    if var_disk_pct.is_some_and(|pct| pct >= prune_threshold_pct) {
        issues.push(format!(
            "disk_pressure_pct={} threshold={}",
            var_disk_pct.unwrap_or_default(),
            prune_threshold_pct
        ));
    }
    let autonomy_ready = governance_observation
        .get("goal")
        .and_then(|v| v.get("autonomy_ready"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !autonomy_ready {
        issues.push("autonomy_observation_score_below_threshold".to_string());
    }

    let mut prune_result = json!({"attempted": false, "applied": false});
    if prune {
        let should_prune = var_disk_pct
            .map(|pct| pct >= prune_threshold_pct)
            .unwrap_or(false);
        if should_prune {
            prune_result = json!({
                "attempted": true,
                "applied": true,
                "details": prune_workspace_artifacts(),
            });
        } else {
            prune_result = json!({
                "attempted": true,
                "applied": false,
                "reason": format!(
                    "disk usage below threshold (usage={:?}, threshold={})",
                    var_disk_pct, prune_threshold_pct
                )
            });
        }
    }

    let warden_report = emit_warden_informant_event("prometheus_maintenance", &issues);

    Ok(json!({
        "ts_utc": Utc::now().to_rfc3339(),
        "cycle": "prometheus_maintenance",
        "hades_sweep": sweep,
        "hades_status": hades_status,
        "manwe_status": manwe_status,
        "prometheus_status": prometheus_status,
        "athena_status": athena_status,
        "hermes_status": hermes_status,
        "mnemosyne_status": mnemosyne_status,
        "plutus_status": plutus_status,
        "governance_observation": governance_observation,
        "queue_observability": queue_observability,
        "active_ruleset": ruleset,
        "manwe_auto_cooldown_applied": auto_cooldown,
        "disk_var_used_pct": var_disk_pct,
        "prune": prune_result,
        "warden_informant": warden_report,
    }))
}

pub(crate) fn runtime_surface() -> serde_json::Value {
    let ruleset = load_active_ruleset();
    json!({
        "paths": {
            "ARDA_ATHENA_HOME": env_or("ARDA_ATHENA_HOME", "data/athena"),
            "ARDA_PROMETHEUS_HOME": env_or("ARDA_PROMETHEUS_HOME", "data/prometheus"),
            "ARDA_MANWE_HOME": env_or("ARDA_MANWE_HOME", "data/manwe"),
            "ARDA_HADES_HOME": env_or("ARDA_HADES_HOME", "data/hades"),
            "ARDA_HERMES_HOME": env_or("ARDA_HERMES_HOME", "data/hermes"),
            "ARDA_MNEMOSYNE_HOME": env_or("ARDA_MNEMOSYNE_HOME", "data/mnemosyne"),
            "ARDA_APOLLO_HOME": env_or("ARDA_APOLLO_HOME", "data/apollo"),
            "ARDA_PLUTUS_HOME": env_or("ARDA_PLUTUS_HOME", "data/plutus"),
            "ARDA_ORACLE_HOME": env_or("ARDA_ORACLE_HOME", "data/oracle"),
            "ARDA_WORLD_STATE_PATH": env_or("ARDA_WORLD_STATE_PATH", "core/state/world.json"),
            "ARDA_SOTERION_INDEX_PATH": env_or("ARDA_SOTERION_INDEX_PATH", "data/soterion_index.json"),
            "ARDA_TASK_QUEUE_PATH": env_or("ARDA_TASK_QUEUE_PATH", "core/projects/tasks/queue.jsonl"),
            "ARDA_PROJECT_TASK_QUEUE_PATH": env_or("ARDA_PROJECT_TASK_QUEUE_PATH", "core/projects/tasks/queue.jsonl"),
            "ARDA_DAILY_QUEUE_PATH": env_or("ARDA_DAILY_QUEUE_PATH", "core/queue/queue.jsonl"),
            "ARDA_WARDEN_QUEUE_PATH": env_or("ARDA_WARDEN_QUEUE_PATH", "data/warden/informant_queue.jsonl"),
            "ARDA_SYSTEM_CONTROL_PATH": env_or("ARDA_SYSTEM_CONTROL_PATH", "core/state/system_control.json"),
        },
        "sockets": {
            "ARDA_ATHENA_SOCKET": env_or("ARDA_ATHENA_SOCKET", &default_runtime_socket("athena.sock")),
            "ARDA_PROMETHEUS_SOCKET": env_or("ARDA_PROMETHEUS_SOCKET", &default_runtime_socket("prometheus.sock")),
            "ARDA_MANWE_SOCKET": env_or("ARDA_MANWE_SOCKET", &default_runtime_socket("manwe.sock")),
            "ARDA_HADES_SOCKET": env_or("ARDA_HADES_SOCKET", &default_runtime_socket("hades.sock")),
            "ARDA_HERMES_SOCKET": env_or("ARDA_HERMES_SOCKET", &default_runtime_socket("hermes.sock")),
            "ARDA_MNEMOSYNE_SOCKET": env_or("ARDA_MNEMOSYNE_SOCKET", &default_runtime_socket("mnemosyne.sock")),
            "ARDA_APOLLO_SOCKET": env_or("ARDA_APOLLO_SOCKET", &default_runtime_socket("apollo.sock")),
            "ARDA_PLUTUS_SOCKET": env_or("ARDA_PLUTUS_SOCKET", &default_runtime_socket("plutus.sock")),
            "ARDA_ORACLE_SOCKET": env_or("ARDA_ORACLE_SOCKET", &default_runtime_socket("oracle.sock")),
        },
        "models": {
            "ARDA_LOCAL_MODEL": env_or("ARDA_LOCAL_MODEL", "qwen2.5-coder:3b"),
        },
        "ruleset": ruleset
    })
}

pub(crate) fn format_tools_output(config: &Config, registered_agents: &[&str]) -> String {
    let mut lines = vec![
        "Arda v0.1.0 — Registered Agents".to_string(),
        "─────────────────────────────────────".to_string(),
    ];
    for agent_name in registered_agents {
        lines.push(format!("  • {}", agent_name));
    }
    lines.push(String::new());
    lines.push("Hardened Subsystems:".to_string());
    for (name, role, surface) in hardened_subsystem_inventory() {
        lines.push(format!("  • {name:<11} {role:<15} {surface}"));
    }
    lines.push(String::new());
    lines.push(format!(
        "LLM Provider: {} ({})",
        config.llm.default_provider,
        config
            .llm
            .providers
            .get(&config.llm.default_provider)
            .map(|p| p.default_model.as_str())
            .unwrap_or("unknown")
    ));
    lines.join("\n") + "\n"
}

pub(crate) fn format_status_output(
    config: &Config,
    config_path: &str,
    provider_name: &str,
    provider_model: &str,
) -> String {
    let runtime_surface = runtime_surface();
    let queue_observability = queue_observability_snapshot();
    let projection_inventory = subsystem_projection_inventory()
        .into_iter()
        .map(|(name, path)| {
            json!({
                "name": name,
                "path": path,
                "present": std::path::Path::new(path).exists()
            })
        })
        .collect::<Vec<_>>();
    let projection_present = projection_inventory
        .iter()
        .filter(|entry| entry.get("present").and_then(|v| v.as_bool()) == Some(true))
        .count();

    let mut lines = vec![
        "Arda v0.1.0".to_string(),
        "─────────────────────────────────────".to_string(),
        format!("System:    {}", config.system.name),
        format!("Provider:  {} → {}", provider_name, provider_model),
        format!(
            "Joule Budget: {} (base cost: {})",
            config.joulework.threshold, config.joulework.base_cost
        ),
        format!("Data Dir:  {}", config.paths.data_dir),
        format!("Ledger:    {}", config.paths.ledger_dir),
        format!("Config:    {}", config_path),
        format!(
            "Projections: {}/{} present",
            projection_present,
            projection_inventory.len()
        ),
    ];

    let provider_count = config.llm.providers.len();
    lines.push(format!("\nProviders ({}):", provider_count));
    for (name, pcfg) in &config.llm.providers {
        let active = if name == &config.llm.default_provider {
            " ← active"
        } else {
            ""
        };
        lines.push(format!(
            "  • {} — {} ({}){}",
            name, pcfg.base_url, pcfg.default_model, active
        ));
    }

    lines.push("\nRuntime Paths:".to_string());
    for section in ["paths", "sockets"] {
        if let Some(entries) = runtime_surface.get(section).and_then(|v| v.as_object()) {
            for (key, value) in entries {
                if let Some(path) = value.as_str() {
                    lines.push(format!("  • {} = {}", key, path));
                }
            }
        }
    }

    lines.push("\nShared State:".to_string());
    for entry in projection_inventory {
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let present = entry
            .get("present")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let status = if present { "present" } else { "missing" };
        lines.push(format!("  • {} — {} ({})", name, path, status));
    }

    lines.push("\nQueues:".to_string());
    if let Some(queues) = queue_observability
        .get("breakdown")
        .and_then(|v| v.as_object())
    {
        for (name, entry) in queues {
            if entry.get("alias_of").is_some() {
                continue;
            }
            let path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let count = entry
                .get("queued")
                .or_else(|| entry.get("open"))
                .or_else(|| entry.get("pending_records"))
                .or_else(|| entry.get("pending_deep"))
                .or_else(|| entry.get("total_records"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let mut detail = count.to_string();
            if let (Some(historical), Some(closed)) = (
                entry.get("historical_records").and_then(|v| v.as_u64()),
                entry.get("closed_records").and_then(|v| v.as_u64()),
            ) {
                detail = format!(
                    "{count}; historical={historical}, closed_by_append_only_ledger={closed}"
                );
            }
            lines.push(format!("  • {} — {} ({})", name, path, detail));
        }
    }

    lines.join("\n") + "\n"
}

pub(crate) fn list_agent_personalities() -> anyhow::Result<serde_json::Value> {
    let path = personality_registry_path();
    let content = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
        Err(err) => return Err(err.into()),
    };
    let value: serde_json::Value = serde_json::from_str(&content)?;
    let mut entries = value
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(agent_id, profile)| {
                    json!({
                        "agent_id": agent_id,
                        "profile": profile
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| {
        let aa = a.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
        let bb = b.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
        aa.cmp(bb)
    });
    Ok(json!({
        "path": path,
        "count": entries.len(),
        "entries": entries,
    }))
}

pub(crate) fn set_agent_personality(
    agent_id: &str,
    personality: &str,
    comms_style: &str,
    notes: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    let path = personality_registry_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
        Err(err) => return Err(err.into()),
    };
    let mut registry: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&content)?;

    let profile = json!({
        "personality": personality,
        "comms_style": comms_style,
        "notes": notes,
        "updated_at_utc": Utc::now().to_rfc3339(),
    });
    registry.insert(agent_id.to_string(), profile.clone());

    fs::write(&path, serde_json::to_string_pretty(&registry)?)?;
    Ok(json!({
        "ok": true,
        "agent_id": agent_id,
        "profile": profile,
        "path": path,
    }))
}

pub(crate) fn spawn_maintenance_cycle(
    sweep_type: &str,
    cooldown_seconds: i64,
    prune: bool,
    prune_threshold_pct: u8,
) -> anyhow::Result<serde_json::Value> {
    fs::create_dir_all("data/prometheus")?;
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let log_path = format!("data/prometheus/maintenance_async_{ts}.log");
    let stdout = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("prometheus")
        .arg("maintenance")
        .arg("--sweep-type")
        .arg(sweep_type)
        .arg("--cooldown-seconds")
        .arg(cooldown_seconds.to_string())
        .arg("--prune-threshold-pct")
        .arg(prune_threshold_pct.to_string())
        .stdout(stdout)
        .stderr(stderr);
    if prune {
        cmd.arg("--prune");
    }

    let child = cmd.spawn()?;
    Ok(json!({
        "ts_utc": Utc::now().to_rfc3339(),
        "cycle": "prometheus_maintenance_async",
        "spawned": true,
        "pid": child.id(),
        "log_path": log_path,
    }))
}

pub(crate) fn load_queued_tasks(limit: usize) -> anyhow::Result<Vec<QueuedTask>> {
    let path = task_queue_path();
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status == "queued" {
            let task_id = value
                .get("task_id")
                .and_then(|v| v.as_str())
                .or_else(|| value.get("id").and_then(|v| v.as_str()));
            let Some(task_id) = task_id else {
                continue;
            };
            let task = QueuedTask {
                task_id: task_id.to_string(),
            };
            out.push(task);
            if out.len() >= limit {
                break;
            }
        }
    }
    Ok(out)
}

pub(crate) fn task_queue_path() -> String {
    env_or(
        "ARDAD_TASK_QUEUE_PATH",
        "core/projects/tasks/queue.jsonl",
    )
}

pub(crate) fn warden_queue_path() -> String {
    env_or(
        "ARDAD_WARDEN_QUEUE_PATH",
        "data/warden/informant_queue.jsonl",
    )
}

fn prune_workspace_artifacts() -> serde_json::Value {
    let target_remove = fs::remove_dir_all("target")
        .map(|_| json!({"ok": true}))
        .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
    let target_recreate = fs::create_dir_all("target")
        .map(|_| json!({"ok": true}))
        .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
    let active_target = std::env::var("CARGO_TARGET_DIR").ok();
    let tmp_remove = if active_target.as_deref() == Some("/tmp/arda-target") {
        json!({
            "ok": false,
            "skipped": true,
            "reason": "active CARGO_TARGET_DIR is /tmp/arda-target"
        })
    } else {
        fs::remove_dir_all("/tmp/arda-target")
            .map(|_| json!({"ok": true}))
            .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}))
    };

    json!({
        "workspace_target_remove": target_remove,
        "workspace_target_recreate": target_recreate,
        "active_cargo_target_dir": active_target,
        "tmp_target_remove": tmp_remove,
    })
}

fn subsystem_projection_inventory() -> Vec<(&'static str, &'static str)> {
    vec![
        ("world", "core/state/world.json"),
        ("system_manifest", "core/state/system_manifest.json"),
        ("warden_guardhouse", "core/state/warden_guardhouse.json"),
        (
            "warden_policy_authority",
            "core/state/warden_policy_authority.json",
        ),
        (
            "warden_edge_contract",
            "core/state/warden_edge_contract.json",
        ),
        ("runtime_topology", "core/state/runtime_topology.json"),
        (
            "warden_nightly_doctrine",
            "core/state/warden_nightly_doctrine.json",
        ),
        ("manwe_router", "core/state/manwe_router.json"),
        ("hades_lifecycle", "core/state/hades_lifecycle.json"),
        ("hermes_command", "core/state/hermes_command.json"),
        (
            "mnemosyne_continuity",
            "core/state/mnemosyne_continuity.json",
        ),
        ("apollo_runtime", "core/state/apollo_runtime.json"),
        ("plutus_runtime", "core/state/plutus_runtime.json"),
        ("oracle_runtime", "core/state/oracle_runtime.json"),
    ]
}

fn hardened_subsystem_inventory() -> Vec<(String, String, String)> {
    vec![
        (
            "athena".to_string(),
            "knowledge".to_string(),
            default_runtime_socket("athena.sock"),
        ),
        (
            "prometheus".to_string(),
            "executive".to_string(),
            default_runtime_socket("prometheus.sock"),
        ),
        (
            "manwe".to_string(),
            "router".to_string(),
            default_runtime_socket("manwe.sock"),
        ),
        (
            "hades".to_string(),
            "lifecycle".to_string(),
            default_runtime_socket("hades.sock"),
        ),
        (
            "hermes".to_string(),
            "communications".to_string(),
            default_runtime_socket("hermes.sock"),
        ),
        (
            "mnemosyne".to_string(),
            "continuity".to_string(),
            default_runtime_socket("mnemosyne.sock"),
        ),
        (
            "warden".to_string(),
            "guardhouse".to_string(),
            "core/state/warden_guardhouse.json".to_string(),
        ),
        (
            "oracle".to_string(),
            "governance".to_string(),
            "crates/arda-oracle".to_string(),
        ),
        (
            "plutus".to_string(),
            "joulework".to_string(),
            "crates/arda-plutus".to_string(),
        ),
        (
            "apollo".to_string(),
            "workflow".to_string(),
            "crates/arda-apollo".to_string(),
        ),
    ]
}

pub(crate) fn arda_root() -> PathBuf {
    std::env::var("ARDA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env_or("ARDA_ROOT", "."))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn emit_warden_informant_event(source: &str, issues: &[String]) -> serde_json::Value {
    let queue_path = warden_queue_path();
    if let Some(parent) = std::path::Path::new(&queue_path).parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return json!({
                "ok": false,
                "error": format!("failed to create {}: {err}", parent.display()),
                "queue_path": queue_path,
            });
        }
    }
    if queue_path.trim().is_empty() {
        return json!({
            "ok": false,
            "error": "arda_WARDEN_QUEUE_PATH is empty",
            "queue_path": queue_path,
        });
    }
    let event = json!({
        "ts_utc": Utc::now().to_rfc3339(),
        "event": "maintenance_health_heartbeat",
        "event_type": "maintenance_health_heartbeat",
        "crate_name": "prometheus",
        "informant_id": "prometheus_maintenance",
        "source": source,
        "severity": if issues.is_empty() { "info" } else { "warning" },
        "issues": issues,
        "status": if issues.is_empty() { "healthy" } else { "attention_required" },
        "content": if issues.is_empty() {
            format!("{source} completed without issues")
        } else {
            format!("{source} completed with {} issues", issues.len())
        },
    });

    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&queue_path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(err) = writeln!(file, "{}", event) {
                return json!({
                    "ok": false,
                    "error": err.to_string(),
                    "queue_path": queue_path,
                });
            }
            json!({
                "ok": true,
                "queue_path": queue_path,
                "issues_count": issues.len(),
            })
        }
        Err(err) => json!({
            "ok": false,
            "error": err.to_string(),
            "queue_path": queue_path,
        }),
    }
}

#[derive(Debug)]
pub(crate) struct QueuedTask {
    pub(crate) task_id: String,
}

fn personality_registry_path() -> std::path::PathBuf {
    std::env::var("ARDA_AGENT_PERSONALITY_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("core/state/agent_personalities.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn cwd_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_root(name: &str) -> anyhow::Result<std::path::PathBuf> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(std::env::temp_dir().join(format!("arda-cli-{name}-{stamp}")))
    }

    #[test]
    fn status_output_is_read_only_for_projection_files() -> anyhow::Result<()> {
        let _guard = cwd_test_lock()
            .lock()
            .map_err(|_| anyhow::anyhow!("cwd test lock poisoned"))?;
        let original_cwd = std::env::current_dir()?;
        let temp_root = temp_root("status-read-only")?;
        let projection_path = temp_root.join("core/state/world.json");
        let queue_path = temp_root.join("core/projects/tasks/queue.jsonl");
        if let Some(parent) = projection_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = queue_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let projection = serde_json::json!({
            "schema_version": "arda.core.state.v1",
            "generated_at_utc": "2026-01-01T00:00:00Z",
            "world": { "status": "stable" }
        });
        fs::write(
            &projection_path,
            serde_json::to_string_pretty(&projection)? + "\n",
        )?;
        fs::write(
            &queue_path,
            "{\"task_id\":\"queued-test\",\"status\":\"queued\"}\n",
        )?;
        let before_projection = fs::read_to_string(&projection_path)?;

        std::env::set_current_dir(&temp_root)?;
        let output_result = std::panic::catch_unwind(|| {
            format_status_output(
                &Config::default(),
                "config/default.toml",
                "opencode",
                "test-model",
            )
        });
        std::env::set_current_dir(&original_cwd)?;
        let output = output_result.map_err(|_| anyhow::anyhow!("format_status_output panicked"))?;

        let after_projection = fs::read_to_string(&projection_path)?;
        assert_eq!(before_projection, after_projection);
        assert!(output.contains("Shared State:"));
        assert!(output.contains("core/state/world.json"));
        assert!(output.contains("Queues:"));
        assert!(output.contains("core/projects/tasks/queue.jsonl"));

        let _ = fs::remove_dir_all(temp_root);
        Ok(())
    }
}
