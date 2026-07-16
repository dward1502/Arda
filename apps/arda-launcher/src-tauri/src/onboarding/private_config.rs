use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::onboarding::constants::{
    ONBOARDING_OPERATOR_ANSWERS_CONTRACT, ONBOARDING_PRIVATE_CONFIG_APPLY_CONTRACT,
    ONBOARDING_PRIVATE_CONFIG_CONTRACT,
};
use crate::onboarding::helpers::{config_entry, now_utc};
use crate::onboarding::io::{onboarding_run_dir, write_json};
use crate::onboarding::provider::{provider_checklist, provider_env_keys};
use crate::onboarding::types::*;

pub fn build_private_config_stage(profile: &EnvironmentProfile, root: &Path) -> PrivateConfigStage {
    let target_path = PathBuf::from(&profile.paths.config_dir.value).join("arda.env");
    let proposed_env_path = root
        .join("audit/onboarding-runs/latest-proposed-arda.env")
        .to_string_lossy()
        .to_string();

    let mut entries = vec![
        config_entry(
            "ARDA_PROFILE",
            profile.profile.clone(),
            true,
            false,
            "Identifies the local setup profile.",
        ),
        config_entry(
            "ARDA_MACHINE_ROLE",
            profile.machine_role.clone(),
            true,
            false,
            "Classifies this host without relying on device names.",
        ),
        config_entry(
            "ARDA_ROOT",
            profile.paths.arda_root.value.clone(),
            true,
            false,
            "Points tools at this checkout.",
        ),
        config_entry(
            "ARDA_CONFIG_DIR",
            profile.paths.config_dir.value.clone(),
            true,
            false,
            "Stores private local config outside the repo.",
        ),
        config_entry(
            "ARDA_DATA_DIR",
            profile.paths.data_dir.value.clone(),
            true,
            false,
            "Stores runtime data outside source control.",
        ),
        config_entry(
            "ARDA_CACHE_DIR",
            profile.paths.cache_dir.value.clone(),
            true,
            false,
            "Stores build/runtime caches outside source control.",
        ),
        config_entry(
            "ARDA_RUNTIME_DIR",
            profile.paths.runtime_dir.value.clone(),
            true,
            false,
            "Stores sockets and short-lived runtime files.",
        ),
        config_entry(
            "CHARON_BASE_URL",
            profile
                .endpoints
                .charon_base_url
                .as_ref()
                .map(|url| url.value.clone())
                .unwrap_or_else(|| "http://127.0.0.1:3001".to_string()),
            true,
            false,
            "Required before Charon service checks can be promoted from planning to live setup.",
        ),
        config_entry(
            "HERMES_BASE_URL",
            profile
                .endpoints
                .hermes_base_url
                .as_ref()
                .map(|url| url.value.clone())
                .unwrap_or_else(|| "http://127.0.0.1:8082".to_string()),
            true,
            false,
            "Required before Hermes communication setup and Discord gateway checks.",
        ),
        config_entry(
            "LOCAL_MODEL_BASE_URL",
            profile
                .endpoints
                .local_model_base_url
                .as_ref()
                .map(|url| url.value.clone())
                .unwrap_or_else(|| "http://127.0.0.1:9337/v1".to_string()),
            false,
            false,
            "Optional local model endpoint for low-VRAM onboarding assistance.",
        ),
        config_entry(
            "LOCAL_MODEL_DEFAULT",
            profile
                .endpoints
                .local_model_default
                .as_ref()
                .map(|model| model.value.clone())
                .unwrap_or_else(|| "auto".to_string()),
            false,
            false,
            "Optional local default model selector.",
        ),
        config_entry(
            "LITELLM_PROXY_URL",
            profile
                .endpoints
                .litellm_proxy_url
                .as_ref()
                .map(|url| url.value.clone())
                .unwrap_or_else(|| "http://127.0.0.1:4000/v1".to_string()),
            false,
            false,
            "Optional LiteLLM aggregator endpoint.",
        ),
    ];

    for key in provider_env_keys(root) {
        entries.push(config_entry(
            &key,
            String::new(),
            false,
            true,
            "Provider secret is staged as presence/missing only; paste the real value into the private env file after human review.",
        ));
    }

    let missing_required = entries
        .iter()
        .filter(|entry| entry.required && !entry.present)
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();

    PrivateConfigStage {
        contract: ONBOARDING_PRIVATE_CONFIG_CONTRACT.to_string(),
        generated_at_utc: now_utc(),
        target_path: target_path.to_string_lossy().to_string(),
        write_policy: "audit_stage_only_no_private_env_write".to_string(),
        entries,
        missing_required,
        proposed_env_path,
        receipt_note: "Generated file contains placeholders and secret presence markers only; no source config or private env file was mutated.".to_string(),
    }
}

pub fn build_operator_answers_template(
    profile: &EnvironmentProfile,
    root: &Path,
) -> OperatorAnswers {
    let providers = provider_checklist(root);
    let selected_providers = providers
        .providers
        .iter()
        .filter(|provider| provider.enabled && provider.missing_env.is_empty())
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();

    OperatorAnswers {
        contract: ONBOARDING_OPERATOR_ANSWERS_CONTRACT.to_string(),
        generated_at_utc: now_utc(),
        machine_role: profile.machine_role.clone(),
        profile: profile.profile.clone(),
        autonomy_posture: profile.safety.autonomy_posture.clone(),
        mutation_requires_human_gate: profile.safety.mutation_requires_human_gate,
        enable_hermes_discord: false,
        enable_fleet_discovery: true,
        prefer_local_assistant: profile.endpoints.local_model_base_url.is_some()
            || profile.endpoints.local_model_default.is_some(),
        selected_providers,
        notes: vec![
            "Edit this receipt to record operator choices; it is not an approval receipt and cannot authorize writes.".to_string(),
            "Keep API keys out of this file.".to_string(),
        ],
    }
}

pub fn parse_operator_answers(path: &Path) -> Result<OperatorAnswers> {
    let raw = fs::read_to_string(path).context("read operator answers file")?;
    let answers: OperatorAnswers =
        serde_json::from_str(&raw).context("parse operator answers json")?;
    if answers.contract != ONBOARDING_OPERATOR_ANSWERS_CONTRACT {
        return Err(anyhow!(
            "unexpected operator answers contract '{}', expected {}",
            answers.contract,
            ONBOARDING_OPERATOR_ANSWERS_CONTRACT
        ));
    }
    Ok(answers)
}

pub fn write_private_config_stage(stage: &PrivateConfigStage, root: &Path) -> Result<PathBuf> {
    write_json(
        &root.join("audit/onboarding-runs/latest-private-config-stage.json"),
        &serde_json::to_value(stage)?,
    )?;

    let out_path = PathBuf::from(&stage.proposed_env_path);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut rendered = String::new();
    rendered.push_str("# arda.onboarding.private_config_stage.v1\n");
    rendered.push_str("# Audit-stage proposal only. Do not commit secrets.\n");
    for entry in &stage.entries {
        let value = if entry.secret {
            if entry.present {
                "<set-secret-in-private-env>"
            } else {
                "<missing-secret>"
            }
        } else {
            entry.value_preview.as_str()
        };
        rendered.push_str(&format!("{}={:?}\n", entry.key, value));
    }
    fs::write(&out_path, rendered)?;
    Ok(out_path)
}

fn env_assignment_key(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("export ") {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        && !key.is_empty()
    {
        Some(key)
    } else {
        None
    }
}

fn quote_env_value(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    format!("\"{escaped}\"")
}

fn render_env_assignment(key: &str, value: &str) -> String {
    format!("{key}={}", quote_env_value(value))
}

pub(crate) fn merge_private_env(
    existing: &str,
    entries: &[PrivateConfigEntry],
) -> (String, Vec<String>) {
    let mut updates = entries
        .iter()
        .filter(|entry| !entry.secret && entry.present)
        .map(|entry| (entry.key.clone(), entry.value_preview.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut changed_keys = Vec::new();
    let mut lines = Vec::new();

    for line in existing.lines() {
        if let Some(key) = env_assignment_key(line) {
            if let Some(value) = updates.remove(key) {
                lines.push(render_env_assignment(key, &value));
                changed_keys.push(key.to_string());
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if !updates.is_empty() {
        if !lines.is_empty()
            && lines
                .last()
                .map(|line| !line.trim().is_empty())
                .unwrap_or(false)
        {
            lines.push(String::new());
        }
        lines.push("# Added by arda onboarding private config apply.".to_string());
        for (key, value) in updates {
            lines.push(render_env_assignment(&key, &value));
            changed_keys.push(key);
        }
    }

    let mut rendered = lines.join("\n");
    rendered.push('\n');
    (rendered, changed_keys)
}

pub fn apply_private_config_baseline(stage: &PrivateConfigStage, root: &Path) -> Result<PathBuf> {
    let target = PathBuf::from(&stage.target_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&target).unwrap_or_default();
    let (rendered, changed_keys) = merge_private_env(&existing, &stage.entries);
    fs::write(&target, rendered)?;

    let receipt = json!({
        "contract": ONBOARDING_PRIVATE_CONFIG_APPLY_CONTRACT,
        "generated_at_utc": now_utc(),
        "target_path": stage.target_path,
        "write_policy": "human_gated_non_secret_env_merge",
        "changed_keys": changed_keys,
        "secret_keys_skipped": stage.entries.iter().filter(|entry| entry.secret).map(|entry| entry.key.clone()).collect::<Vec<_>>(),
        "notes": "Merged non-secret staged values only. Existing unrecognized lines were preserved. Provider secrets were not written.",
    });
    let run_dir = onboarding_run_dir(root)?;
    let receipt_path = run_dir.join("private_config_apply_receipt.json");
    write_json(&receipt_path, &receipt)?;
    write_json(
        &root.join("audit/onboarding-runs/latest-private-config-apply.json"),
        &receipt,
    )?;
    Ok(receipt_path)
}
