use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::helpers::{now_run_id, now_utc, today_stamp};
use crate::types::*;

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
    let template_path = root.join("config/arda.template.toml");
    let template = fs::read_to_string(&template_path)
        .with_context(|| format!("read template {}", template_path.display()))?;
    let mut rendered = template;
    let ann_root = profile.paths.arda_root.value.clone();
    let replacements = [
        ("{{profile_id}}", profile.profile.clone()),
        ("{{machine_role}}", profile.machine_role.clone()),
        ("{{arda_root}}", ann_root),
        ("{{config_dir}}", profile.paths.config_dir.value.clone()),
        ("{{data_dir}}", profile.paths.data_dir.value.clone()),
        ("{{cache_dir}}", profile.paths.cache_dir.value.clone()),
        ("{{runtime_dir}}", profile.paths.runtime_dir.value.clone()),
        (
            "{{build_cache_root}}",
            profile
                .paths
                .build_cache_root
                .as_ref()
                .map(|p| p.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{charon_base_url}}",
            profile
                .endpoints
                .charon_base_url
                .as_ref()
                .map(|u| u.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{hermes_base_url}}",
            profile
                .endpoints
                .hermes_base_url
                .as_ref()
                .map(|u| u.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{arda_hud_url}}",
            profile
                .endpoints
                .arda_hud_url
                .as_ref()
                .map(|u| u.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{local_model_base_url}}",
            profile
                .endpoints
                .local_model_base_url
                .as_ref()
                .map(|u| u.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{local_model_default}}",
            profile
                .endpoints
                .local_model_default
                .as_ref()
                .map(|u| u.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{litellm_proxy_url}}",
            profile
                .endpoints
                .litellm_proxy_url
                .as_ref()
                .map(|u| u.value.clone())
                .unwrap_or_default(),
        ),
        (
            "{{autonomy_posture}}",
            profile.safety.autonomy_posture.clone(),
        ),
    ];
    for (old, new_value) in replacements {
        rendered = rendered.replace(old, &new_value);
    }
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
