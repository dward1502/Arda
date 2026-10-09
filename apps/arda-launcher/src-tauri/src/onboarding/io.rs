use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::onboarding::helpers::{now_run_id, now_utc, today_stamp};
use crate::onboarding::types::*;

pub fn write_json(path: &Path, payload: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(payload)? + "\n")?;
    Ok(())
}

pub fn write_profile(path: &Path, profile: &EnvironmentProfile) -> Result<()> {
    let payload = serde_json::to_value(profile)?;
    write_json(path, &payload)
}

pub fn write_readiness(path: &Path, projection: &ReadinessProjection) -> Result<()> {
    let payload = serde_json::to_value(projection)?;
    write_json(path, &payload)
}

pub fn build_proposed_config(profile: &EnvironmentProfile, root: &Path) -> Result<PathBuf> {
    let template_path = root.join("config/templates/arda.local.profile.toml");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("read template {}", template_path.display()))?;
    // Parse and replace owned fields structurally: shell placeholders are not a renderer,
    // and string substitution fails to escape operator paths correctly.
    let mut rendered: toml::Value = toml::from_str(&template)?;
    for section in ["profile", "paths", "endpoints", "safety"] {
        anyhow::ensure!(
            rendered
                .get(section)
                .and_then(toml::Value::as_table)
                .is_some(),
            "template missing table {section}"
        );
    }
    let table = rendered
        .as_table_mut()
        .context("template must be a table")?;
    let section = table
        .get_mut("profile")
        .and_then(toml::Value::as_table_mut)
        .context("profile table")?;
    section.insert("id".into(), profile.profile.clone().into());
    section.insert("machine_role".into(), profile.machine_role.clone().into());
    let paths = table
        .get_mut("paths")
        .and_then(toml::Value::as_table_mut)
        .context("paths table")?;
    for (key, value) in [
        ("arda_root", &profile.paths.arda_root.value),
        ("config_dir", &profile.paths.config_dir.value),
        ("data_dir", &profile.paths.data_dir.value),
        ("cache_dir", &profile.paths.cache_dir.value),
        ("runtime_dir", &profile.paths.runtime_dir.value),
    ] {
        paths.insert(key.into(), value.clone().into());
    }
    paths.remove("build_cache_root");
    if let Some(path) = &profile.paths.build_cache_root {
        paths.insert("build_cache_root".into(), path.value.clone().into());
    }
    let endpoints = table
        .get_mut("endpoints")
        .and_then(toml::Value::as_table_mut)
        .context("endpoints table")?;
    for (key, value) in [
        ("manwe_base_url", &profile.endpoints.manwe_base_url),
        ("hermes_base_url", &profile.endpoints.hermes_base_url),
        ("arda_hud_url", &profile.endpoints.arda_hud_url),
        (
            "local_model_base_url",
            &profile.endpoints.local_model_base_url,
        ),
        ("litellm_proxy_url", &profile.endpoints.litellm_proxy_url),
        ("crawl4ai_url", &profile.endpoints.crawl4ai_url),
        ("search_runtime_url", &profile.endpoints.search_runtime_url),
    ] {
        endpoints.remove(key);
        if let Some(value) = value {
            endpoints.insert(key.to_string(), value.value.clone().into());
        }
    }
    endpoints.remove("local_model_default");
    if let Some(model) = &profile.endpoints.local_model_default {
        endpoints.insert("local_model_default".into(), model.value.clone().into());
    }
    let safety = table
        .get_mut("safety")
        .and_then(toml::Value::as_table_mut)
        .context("safety table")?;
    safety.insert(
        "autonomy_posture".into(),
        profile.safety.autonomy_posture.clone().into(),
    );
    safety.insert(
        "mutation_requires_human_gate".into(),
        profile.safety.mutation_requires_human_gate.into(),
    );
    safety.insert("destructive_allowed_by_default".into(), false.into());
    let rendered = toml::to_string_pretty(&rendered)?;
    let out_dir = onboarding_run_dir(root)?;
    let out_path = out_dir.join("proposed-config.toml");
    fs::write(&out_path, rendered)?;
    let receipt_path = out_dir.join("proposed-config.receipt.json");
    let receipt = json!({
        "contract": "arda.onboarding.proposed-config.v1",
        "profile": profile.profile,
        "generated_at_utc": now_utc(),
        "source_template": template_path.to_string_lossy(),
        "proposed_config_path": out_path.to_string_lossy(),
        "notes": "Read-only proposal artifact only; no in-place config writes occurred.",
        "machine_path_profile": {
            "arda_root": profile.paths.arda_root.value,
            "machine_role": profile.machine_role,
        },
    });
    write_json(&receipt_path, &receipt)?;
    Ok(out_path)
}

pub fn onboarding_run_dir(root: &Path) -> Result<PathBuf> {
    let dir = root
        .join("audit")
        .join("onboarding-runs")
        .join(today_stamp())
        .join(format!("onboarding-{}", now_run_id()));
    fs::create_dir_all(&dir).context("create onboarding run dir")?;
    Ok(dir)
}

pub fn write_onboarding_receipt(
    root: &Path,
    run_id: &str,
    kind: &str,
    payload: &Value,
) -> Result<PathBuf> {
    let out_dir = onboarding_run_dir(root)?;
    let out_path = out_dir.join(format!("{kind}-{run_id}-receipt.json"));
    write_json(&out_path, payload)?;
    Ok(out_path)
}

pub fn read_json_optional(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}
