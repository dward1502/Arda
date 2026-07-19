#![cfg(feature = "full-cli")]
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

pub(crate) fn export_package_enablement_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/package_enablement.json");
    let registry = read_toml_or(
        &root.join("docs/registry.toml"),
        toml::Value::Table(Default::default()),
    );
    let registry = registry
        .get("tools")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();
    let package_health = read_json_or(&root.join("core/state/package_health.json"), json!({}));
    let observed_tools = package_health
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let charon = read_toml_or(
        &root.join("config/charon.providers.toml"),
        toml::Value::Table(Default::default()),
    );
    let provider_ids: HashSet<String> = charon
        .get("provider")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(toml::Value::as_str))
        .map(ToOwned::to_owned)
        .collect();
    let shared_keys = env_keys(&root.join("config/.env.example"));
    let runtime_keys = env_keys(&root.join("config/runtime.env.example"));
    let repo_sources = athena_repo_source_map(&root.join("data/athena/books"));
    let readiness = latest_policy_readiness(&root.join("data/athena/policy_readiness.jsonl"));
    let runtime_surfaces = read_json_or(
        &root.join("core/state/package_runtime_activation.json"),
        json!({}),
    )
    .get("surfaces")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();

    let mut tools = Vec::new();
    for tool in registry.keys().cloned().collect::<Vec<_>>() {
        let meta = registry
            .get(&tool)
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        let repo = meta
            .get("repo")
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .to_string();
        let repo_url = (!repo.is_empty()).then(|| format!("https://github.com/{repo}"));
        let observation = observed_tools
            .iter()
            .find(|row| row.get("tool").and_then(Value::as_str) == Some(tool.as_str()))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let source_id = repo_url
            .as_deref()
            .and_then(|repo_url| repo_sources.get(repo_url))
            .cloned()
            .unwrap_or_default();
        let policy = readiness
            .get(&source_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let policy_readiness = policy
            .get("policy_readiness")
            .and_then(Value::as_str)
            .unwrap_or("untracked");
        let provider_id = provider_id(&tool);
        let provider_configured = provider_id
            .as_deref()
            .is_some_and(|provider| provider_ids.contains(provider));
        let shared_contract = required_shared_env(&tool);
        let runtime_contract = required_runtime_env(&tool);
        let shared_ready = shared_contract.iter().all(|key| shared_keys.contains(*key));
        let runtime_ready = runtime_contract
            .iter()
            .all(|key| runtime_keys.contains(*key));
        let executable_visible =
            observation.get("binary_path").is_some() || observation.get("version_hint").is_some();
        let runtime_surface_ready = provider_id
            .as_deref()
            .map(|_| provider_configured)
            .unwrap_or(executable_visible);
        let integration_state = if policy_readiness == "policy_ready"
            && runtime_surface_ready
            && shared_ready
            && runtime_ready
        {
            "ready_for_activation"
        } else if policy_readiness == "policy_ready" && runtime_surface_ready {
            "configuration_ready"
        } else if policy_readiness == "policy_ready" {
            "evidence_ready"
        } else if observation
            .get("observation_status")
            .and_then(Value::as_str)
            == Some("observed")
        {
            "observed_only"
        } else {
            "planned_only"
        };
        let runtime_surface = runtime_surfaces
            .get(runtime_surface_key(&tool))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let activation = activation_status(
            &tool,
            integration_state,
            provider_configured,
            &Value::Object(runtime_surface.clone()),
        );
        tools.push(json!({
            "tool": tool,
            "repo": if repo.is_empty() { Value::Null } else { json!(repo) },
            "repo_url": repo_url,
            "category": meta.get("category").and_then(toml::Value::as_str),
            "registry_status": meta.get("status").and_then(toml::Value::as_str),
            "tool_type": meta.get("type").and_then(toml::Value::as_str),
            "integration_lane": integration_lane(
                &tool,
                meta.get("category").and_then(toml::Value::as_str).unwrap_or("unknown"),
            ),
            "source_id": if source_id.is_empty() { Value::Null } else { json!(source_id) },
            "policy_readiness": policy_readiness,
            "policy_confidence": policy
                .get("gate")
                .and_then(|value| value.get("observed"))
                .and_then(|value| value.get("confidence"))
                .cloned()
                .unwrap_or(Value::Null),
            "observation_status": observation.get("observation_status").cloned().unwrap_or_else(|| json!("unobserved")),
            "binary_path": observation.get("binary_path").cloned().unwrap_or(Value::Null),
            "version_hint": observation.get("version_hint").cloned().unwrap_or(Value::Null),
            "provider_id": provider_id,
            "provider_configured": provider_configured,
            "shared_env_contract": shared_contract,
            "shared_env_ready": shared_ready,
            "runtime_env_contract": runtime_contract,
            "runtime_env_ready": runtime_ready,
            "executable_visible": executable_visible,
            "integration_state": integration_state,
            "activation_status": activation,
            "next_action": next_action(&tool, integration_state, activation),
        }));
    }

    let payload = json!({
        "schema_version": "arda.core.state.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_enablement_projection",
        "summary": {
            "tools_total": tools.len(),
            "policy_ready_total": tools.iter().filter(|row| row.get("policy_readiness").and_then(Value::as_str) == Some("policy_ready")).count(),
            "ready_for_activation_total": tools.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("ready_for_activation")).count(),
            "configuration_ready_total": tools.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("configuration_ready")).count(),
            "evidence_ready_total": tools.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("evidence_ready")).count(),
            "observed_only_total": tools.iter().filter(|row| row.get("integration_state").and_then(Value::as_str) == Some("observed_only")).count(),
        },
        "tools": tools,
    });
    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_package_runtime_activation_impl() -> Result<Value> {
    let root = workspace_root();
    let out_path = root.join("core/state/package_runtime_activation.json");
    let opencode_routes_path = root.join("config/opencode_agent_routes.toml");
    let opencode_routes = read_toml_or(
        &opencode_routes_path,
        toml::Value::Table(Default::default()),
    );
    let agents = opencode_routes
        .get("agents")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();

    let mut surfaces = serde_json::Map::new();
    surfaces.insert(
        "litellm".to_string(),
        shell_surface("scripts/litellm_proxy.sh", "status", &root),
    );
    surfaces.insert(
        "crawl4ai".to_string(),
        shell_surface("scripts/runtime/crawl4ai_service.sh", "status", &root),
    );
    surfaces.insert(
        "scrapling".to_string(),
        shell_surface("scripts/runtime/scrapling_runtime.sh", "status", &root),
    );
    surfaces.insert(
        "search_runtime".to_string(),
        shell_surface("scripts/runtime/searxng_service.sh", "status", &root),
    );
    surfaces.insert(
        "nanoclaw".to_string(),
        shell_surface("scripts/runtime/nanoclaw_runtime.sh", "status", &root),
    );
    surfaces.insert(
        "playwright_mcp".to_string(),
        normalize_playwright(shell_surface(
            "scripts/runtime/playwright_mcp_bridge.sh",
            "status",
            &root,
        )),
    );

    let llmfit_binary = find_binary("llmfit");
    let (llmfit_ok, llmfit) = if llmfit_binary.is_some() {
        command_probe(&["llmfit", "recommend", "--json", "--limit", "1"], &root)
    } else {
        (true, "optional signal not installed".to_string())
    };
    surfaces.insert(
        "llmfit".to_string(),
        json!({
            "ok": llmfit_ok,
            "status": if llmfit_binary.is_none() {
                "optional_signal_absent"
            } else if llmfit_ok {
                "ready"
            } else {
                "probe_failed"
            },
            "summary": llmfit,
            "binary_present": llmfit_binary.is_some(),
        }),
    );

    let (opencode_ok, opencode) = command_probe(&["opencode", "--help"], &root);
    surfaces.insert(
        "oh_my_opencode".to_string(),
        json!({
            "ok": opencode_ok,
            "status": if opencode_ok { "ready" } else { "probe_failed" },
            "summary": opencode,
            "route_contract_path": rel(&opencode_routes_path, &root),
            "route_contract_ready": !agents.is_empty(),
            "route_agents_total": agents.len(),
            "route_defaults": opencode_routes.get("defaults").cloned().unwrap_or(toml::Value::Table(Default::default())),
        }),
    );

    let payload = json!({
        "schema_version": "arda.package.runtime-activation.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_enablement + live wrapper/probe checks",
        "surfaces": surfaces,
    });

    write_pretty_json(&out_path, &payload)?;
    Ok(json!({ "out": rel(&out_path, &root) }))
}

pub(crate) fn export_package_health_impl() -> Result<Value> {
    let root = workspace_root();
    let registry_path = root.join("docs/registry.toml");
    let control_path = root.join("core/state/system_control.json");
    let env_example_path = root.join("config/.env.example");
    let runtime_env_example_path = root.join("config/runtime.env.example");
    let out_latest = root.join("data/prometheus/package_health_last.json");
    let out_metrics = root.join("core/metrics/by_crate/prometheus/package_health.json");

    let registry_raw = fs::read_to_string(&registry_path)
        .with_context(|| format!("failed to read {}", rel(&registry_path, &root)))?;
    let registry = parse_toml_document_local(&registry_raw)?;
    let control = read_json_or(&control_path, json!({}));
    let tools = registry
        .get("tools")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();

    let critical_tools = control
        .get("package_observation")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("critical_tools"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let observations = tools
        .iter()
        .map(|(name, meta)| tool_observation(name, meta, &root))
        .collect::<Vec<_>>();
    let critical: Vec<&Value> = observations
        .iter()
        .filter(|entry| {
            entry
                .get("tool")
                .and_then(Value::as_str)
                .map(|tool| critical_tools.iter().any(|critical| critical == tool))
                .unwrap_or(false)
        })
        .collect();

    let attention = critical
        .iter()
        .filter(|entry| {
            entry.get("observation_status").and_then(Value::as_str) == Some("attention_required")
        })
        .filter_map(|entry| {
            entry
                .get("tool")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    let version_blind = critical
        .iter()
        .filter(|entry| entry.get("version_visibility").and_then(Value::as_str) != Some("visible"))
        .filter_map(|entry| {
            entry
                .get("tool")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "arda.package.health.v1",
        "generated_at_utc": now_utc(),
        "authority": "package_observation_export",
        "registry_path": rel(&registry_path, &root),
        "critical_tools": critical_tools,
        "env_templates": {
            "shared": parse_env_example(&env_example_path),
            "runtime": parse_env_example(&runtime_env_example_path),
        },
        "summary": {
            "tools_total": observations.len(),
            "critical_tools_total": critical.len(),
            "critical_attention_required": attention,
            "critical_version_blind": version_blind,
            "observed_with_version": observations
                .iter()
                .filter(|entry| entry.get("version_visibility").and_then(Value::as_str) == Some("visible"))
                .count(),
        },
        "tools": observations,
    });

    write_pretty_json(&out_latest, &payload)?;
    write_pretty_json(&out_metrics, &payload)?;
    Ok(json!({
        "latest": rel(&out_latest, &root),
        "metrics": rel(&out_metrics, &root),
        "payload": payload,
    }))
}

fn parse_toml_document_local(raw: &str) -> Result<toml::Value> {
    let content = if let Some((_, tail)) = raw.split_once("```toml") {
        tail.split_once("```").map(|(body, _)| body).unwrap_or(tail)
    } else {
        raw
    };
    Ok(toml::from_str(content.trim())?)
}

fn parse_env_example(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#') && line.contains('='))
                .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim().to_string()))
                .filter(|key| !key.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn find_binary(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|entry| {
        let candidate = entry.join(name);
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    })
}

fn command_version(cmd: &[&str], cwd: &Path) -> Option<String> {
    let (program, args) = cmd.split_first()?;
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    text.lines()
        .next()
        .map(|line| line.chars().take(200).collect())
}

fn command_probe(cmd: &[&str], cwd: &Path) -> (bool, String) {
    let Some((program, args)) = cmd.split_first() else {
        return (false, "empty command".to_string());
    };
    match Command::new(program).args(args).current_dir(cwd).output() {
        Ok(output) => {
            let text = String::from_utf8_lossy(if output.stdout.is_empty() {
                &output.stderr
            } else {
                &output.stdout
            })
            .trim()
            .chars()
            .take(1200)
            .collect::<String>();
            (output.status.success(), text)
        }
        Err(err) => (false, err.to_string()),
    }
}

fn shell_surface(script: &str, action: &str, root: &Path) -> Value {
    let (ok, output) = command_probe(&["bash", script, action], root);
    if output.is_empty() {
        return json!({"ok": ok, "status": "no_output"});
    }
    match serde_json::from_str::<Value>(&output) {
        Ok(mut payload) if payload.is_object() => {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("ok".to_string(), Value::from(ok));
            }
            payload
        }
        _ => json!({"ok": ok, "status": "invalid_json", "raw_output": output}),
    }
}

fn normalize_playwright(surface: Value) -> Value {
    let mut surface = surface;
    if surface.get("status").and_then(Value::as_str) == Some("not_running") {
        if let Some(obj) = surface.as_object_mut() {
            obj.insert("status".to_string(), Value::from("contract_ready"));
            obj.insert("runtime_mode".to_string(), Value::from("ephemeral_stdio"));
            obj.insert(
                "note".to_string(),
                Value::from(
                    "Playwright MCP is expected to run on-demand for a stdio client session rather than remain daemonized.",
                ),
            );
        }
    }
    surface
}

fn workspace_reference_count(name: &str, meta: &toml::Value, root: &Path) -> usize {
    let repo = meta.get("repo").and_then(toml::Value::as_str).unwrap_or("");
    let image = meta
        .get("image")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let install = meta
        .get("install")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let tag = format!("[tools.{name}]");
    let needles = [repo, tag.as_str(), image, install];
    let search_roots = [
        "apps", "config", "core", "crates", "docs", "human", "scripts",
    ]
    .iter()
    .map(|part| root.join(part))
    .filter(|path| path.exists())
    .collect::<Vec<_>>();
    let mut matched = BTreeSet::new();

    for needle in needles {
        if needle.trim().is_empty() {
            continue;
        }
        let mut cmd = Command::new("rg");
        cmd.arg("-l")
            .arg("-F")
            .arg("--glob")
            .arg("!core/metrics/history/**")
            .arg("--glob")
            .arg("!core/metrics/by_crate/**")
            .arg("--glob")
            .arg("!apps/arda-hud/package-lock.json")
            .arg("--glob")
            .arg("!*.png")
            .arg("--glob")
            .arg("!*.jpg")
            .arg("--glob")
            .arg("!*.jpeg")
            .arg("--glob")
            .arg("!*.gif")
            .arg("--glob")
            .arg("!*.webp")
            .arg("--glob")
            .arg("!*.woff")
            .arg("--glob")
            .arg("!*.woff2")
            .arg(needle);
        for search_root in &search_roots {
            cmd.arg(search_root);
        }
        if let Ok(output) = cmd.current_dir(root).output() {
            if output.status.success() || output.status.code() == Some(1) {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        matched.insert(trimmed.to_string());
                    }
                }
            }
        }
    }

    matched.len()
}

fn tool_observation(name: &str, meta: &toml::Value, root: &Path) -> Value {
    let binary_map: HashMap<&str, &str> = HashMap::from([
        ("llmfit", "llmfit"),
        ("nanoclaw", "nanoclaw"),
        ("litellm", "litellm"),
        ("crawl4ai", "docker"),
        ("playwright-mcp", "npx"),
        ("oh-my-opencode", "opencode"),
    ]);
    let candidate = binary_map.get(name).copied();
    let binary_path = candidate
        .and_then(find_binary)
        .map(|path| path.to_string_lossy().to_string());
    let version_hint = match candidate {
        Some("docker") => command_version(&["docker", "--version"], root),
        Some(program) => command_version(&[program, "--version"], root),
        None => None,
    };
    let references = workspace_reference_count(name, meta, root);
    let registry_status = meta
        .get("status")
        .and_then(toml::Value::as_str)
        .unwrap_or("unknown");
    let observation_status =
        if registry_status == "active" && binary_path.is_none() && references == 0 {
            "attention_required"
        } else if binary_path.is_some() || references > 0 {
            "observed"
        } else {
            "unobserved"
        };
    json!({
        "tool": name,
        "repo": meta.get("repo").and_then(toml::Value::as_str),
        "type": meta.get("type").and_then(toml::Value::as_str).unwrap_or("unknown"),
        "registry_status": registry_status,
        "category": meta.get("category").and_then(toml::Value::as_str).unwrap_or("unknown"),
        "binary_path": binary_path,
        "version_hint": version_hint,
        "install_hint": meta.get("install").and_then(toml::Value::as_str),
        "workspace_references": references,
        "observation_status": observation_status,
        "version_visibility": if version_hint.is_some() { "visible" } else { "missing" },
        "update_visibility": if version_hint.is_some()
            || meta.get("install").and_then(toml::Value::as_str).is_some()
            || references > 0 {
            "ready"
        } else {
            "opaque"
        },
    })
}

fn env_keys(path: &Path) -> HashSet<String> {
    parse_env_example(path).into_iter().collect()
}

fn read_jsonl_objects_local(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
                .filter(|value| value.is_object())
                .collect()
        })
        .unwrap_or_default()
}

fn athena_repo_source_map(books_root: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if !books_root.exists() {
        return out;
    }
    let Ok(entries) = fs::read_dir(books_root) else {
        return out;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("src_") && name.ends_with(".jsonl"))
        {
            continue;
        }
        for row in read_jsonl_objects_local(&path) {
            let title = row
                .get("data")
                .and_then(|value| value.get("title"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let source_id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("")
                .to_string();
            if title.starts_with("https://github.com/") {
                out.insert(title.to_string(), source_id.clone());
            }
        }
    }
    out
}

fn latest_policy_readiness(path: &Path) -> HashMap<String, Value> {
    let mut latest = HashMap::new();
    for row in read_jsonl_objects_local(path) {
        if let Some(source_id) = row.get("source_id").and_then(Value::as_str) {
            if !source_id.is_empty() {
                latest.insert(source_id.to_string(), row);
            }
        }
    }
    latest
}

fn provider_id(tool: &str) -> Option<String> {
    match tool {
        "litellm" => Some("litellm_gateway".to_string()),
        _ => None,
    }
}

fn runtime_surface_key(tool: &str) -> &str {
    match tool {
        "playwright-mcp" => "playwright_mcp",
        "oh-my-opencode" => "oh_my_opencode",
        _ => tool,
    }
}

fn required_shared_env(tool: &str) -> Vec<&'static str> {
    match tool {
        "litellm" => vec!["LITELLM_API_KEY"],
        _ => Vec::new(),
    }
}

fn required_runtime_env(tool: &str) -> Vec<&'static str> {
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

fn integration_lane(tool: &str, category: &str) -> &'static str {
    match tool {
        "litellm" => "charon_provider",
        "crawl4ai" => "athena_ingestion",
        "playwright-mcp" => "mcp_browser",
        "discord-mcp" => "mcp_communications",
        "nanoclaw" => "edge_runtime",
        "llmfit" => "model_selection",
        _ => match category {
            "agent-framework" => "agent_framework",
            "agent-skills" => "agent_skills",
            "knowledge" => "knowledge",
            _ => "research",
        },
    }
}

fn activation_status(
    tool: &str,
    integration_state: &str,
    provider_configured: bool,
    runtime_surface: &Value,
) -> &'static str {
    let runtime_status = runtime_surface
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("");
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
        "nanoclaw" if runtime_status == "contract_ready" && runtime_ok => "governed_on_demand",
        "nanoclaw"
            if runtime_surface.get("auth_ready").and_then(Value::as_bool) == Some(false)
                && runtime_surface.get("control_mode").and_then(Value::as_str)
                    == Some("whatsapp") =>
        {
            "blocked_on_auth"
        }
        _ if integration_state == "ready_for_activation" => "activation_frontier",
        _ if integration_state == "configuration_ready" => "configuration_frontier",
        _ => "planned",
    }
}

fn next_action(tool: &str, integration_state: &str, activation: &str) -> &'static str {
    match (tool, activation) {
        ("litellm", "active_in_system") => {
            "LiteLLM is already live in CHARON; keep provider health, models, and gateway policy aligned"
        }
        ("crawl4ai", "active_in_system") => {
            "ATHENA can already ingest through crawl4ai; use `arda athena crawl <url>` when capture is needed"
        }
        ("llmfit", "active_signal") => {
            "llmfit recommendations are already visible to CHARON route policy; tune route heuristics rather than wiring a new runtime"
        }
        ("oh-my-opencode", "active_signal") => {
            "OpenCode is already bounded by sovereign route contracts; tune agent route mappings rather than treating it as an unintegrated package"
        }
        ("playwright-mcp", "governed_on_demand") => {
            "Start the supervised bridge only for governed browser sessions; keep it on-demand rather than daemonized"
        }
        ("nanoclaw", "governed_on_demand") => {
            "NanoClaw is bounded as a headless or Tailscale edge contract; promote live edge visibility before changing doctrine"
        }
        ("nanoclaw", "blocked_on_auth") => {
            "Complete NanoClaw channel authentication or edge enrollment before starting the runtime"
        }
        _ => match (tool, integration_state) {
            ("litellm", "ready_for_activation") => {
                "set CHARON litellm_gateway enabled=true and point the proxy URL at the live gateway"
            }
            ("crawl4ai", "ready_for_activation") => {
                "run `arda athena crawl <url>` to capture markdown into ATHENA via the local crawl4ai service"
            }
            ("playwright-mcp", "ready_for_activation") => {
                "start the supervised bridge and expose the governed browser session tool through arda-mcp"
            }
            ("nanoclaw", "ready_for_activation") => {
                "run `bash scripts/runtime/nanoclaw_runtime.sh start` after channel auth is present, or route NanoClaw to the configured Tailscale edge target"
            }
            ("nanoclaw", "configuration_ready") => {
                "complete NanoClaw channel authentication or edge enrollment to promote the contract into live runtime use"
            }
            ("llmfit", "evidence_ready") => {
                "feed model-fit recommendations into CHARON route policy"
            }
            _ => "promote from evidence into a bounded runtime or product surface",
        },
    }
}
