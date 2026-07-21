use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::onboarding::constants::{
    ONBOARDING_APPLY_RESULT_CONTRACT, ONBOARDING_APPROVAL_CONTRACT,
    ONBOARDING_SERVICE_PLAN_CONTRACT,
};
use crate::onboarding::device_scan;
use crate::onboarding::guided::build_guided_session;
use crate::onboarding::helpers::{
    action_is_approved, action_receipt_path, make_apply_result, now_utc,
};
use crate::onboarding::io::{
    build_proposed_config, onboarding_run_dir, write_json, write_profile, write_readiness,
};
use crate::onboarding::prerequisites::build_prerequisite_report;
use crate::onboarding::private_config::{
    apply_private_config_baseline, build_operator_answers_template, build_private_config_stage,
    write_private_config_stage,
};
use crate::onboarding::provider_checklist;
use crate::onboarding::readiness::build_readiness_projection;
use crate::onboarding::types::*;

pub fn build_service_plan(profile: &EnvironmentProfile, root: &Path) -> ServicePlan {
    let readiness = build_readiness_projection(profile, root);
    let mut actions = Vec::new();

    actions.push(ServiceAction {
        action_id: "onboarding.emit_profile".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit environment profile snapshot".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding detect --write".to_string(),
        target_path: Some(root.join("core/state/environment_profile.json").to_string_lossy().to_string()),
        requires_human_gate: false,
        description: "Generate or refresh canonical environment profile in read-only onboarding contract format.".to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.emit_readiness".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit setup readiness projection".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding readiness --output core/state/setup_console_readiness.json".to_string(),
        target_path: Some(root.join("core/state/setup_console_readiness.json").to_string_lossy().to_string()),
        requires_human_gate: false,
        description: "Refresh setup lane readiness state used by ARDA and onboarding console.".to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.emit_provider_checklist".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit provider/action checklist".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding provider-checklist".to_string(),
        target_path: Some(
            root.join("audit/onboarding-runs/latest-providers.json")
                .to_string_lossy()
                .to_string(),
        ),
        requires_human_gate: false,
        description: "Generate provider readiness + env-key checklist for Manwe onboarding."
            .to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.emit_device_scan".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit local and topology scan".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding device-scan".to_string(),
        target_path: Some(
            root.join("audit/onboarding-runs/latest-device.json")
                .to_string_lossy()
                .to_string(),
        ),
        requires_human_gate: false,
        description: "Scan local runtime, Tailscale state, and capabilities.".to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.emit_prerequisites".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit installer prerequisite report".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding prerequisites".to_string(),
        target_path: Some(root.join("audit/onboarding-runs/latest-prerequisites.json").to_string_lossy().to_string()),
        requires_human_gate: false,
        description: "Classify local tools, repo files, runtime paths, and native GUI/build prerequisites for a new machine.".to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.stage_private_config".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Stage private config proposal".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding private-config-stage".to_string(),
        target_path: Some(root.join("audit/onboarding-runs/latest-private-config-stage.json").to_string_lossy().to_string()),
        requires_human_gate: false,
        description: "Generate a secret-safe proposed arda.env artifact under audit/onboarding-runs without writing the private env file.".to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.emit_guided_session".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit guided onboarding session".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding guided-session".to_string(),
        target_path: Some(
            root.join("audit/onboarding-runs/latest-guided-session.json")
                .to_string_lossy()
                .to_string(),
        ),
        requires_human_gate: false,
        description:
            "Generate operator-choice prompts and next actions for the First Light onboarding flow."
                .to_string(),
        risk: "read_only".to_string(),
    });

    actions.push(ServiceAction {
        action_id: "onboarding.emit_proposed_config".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit proposed configuration artifact".to_string(),
        command_hint: "cargo run -p arda-cli -- onboarding propose-config".to_string(),
        target_path: Some(
            root.join("audit/onboarding-runs")
                .to_string_lossy()
                .to_string(),
        ),
        requires_human_gate: false,
        description:
            "Render a machine-specific proposal from arda.template.toml into an audit artifact."
                .to_string(),
        risk: "read_only".to_string(),
    });

    if profile.safety.mutation_requires_human_gate {
        actions.push(ServiceAction {
            action_id: "onboarding.write_mutation_receipt".to_string(),
            action_type: "human_gate".to_string(),
            title: "Prepare human gate for any future writes".to_string(),
            command_hint: "Open a human approval receipt path using a future onboarding apply flow.".to_string(),
            target_path: None,
            requires_human_gate: true,
            description: "Any runtime mutation path is currently not automatically applied; a human gate is required first.".to_string(),
            risk: "human_gated".to_string(),
        });

        actions.push(ServiceAction {
            action_id: "onboarding.write_private_config_baseline".to_string(),
            action_type: "private_config_write".to_string(),
            title: "Write private non-secret config baseline".to_string(),
            command_hint: "cargo run -p arda-cli -- onboarding apply-config --receipt <approved-receipt.json> --action-id onboarding.write_private_config_baseline".to_string(),
            target_path: Some(
                PathBuf::from(&profile.paths.config_dir.value)
                    .join("arda.env")
                    .to_string_lossy()
                    .to_string(),
            ),
            requires_human_gate: true,
            description: "Create or update the private arda.env baseline with non-secret staged values only; provider secrets remain placeholders for the operator.".to_string(),
            risk: "human_gated_private_config_write_no_secrets".to_string(),
        });
    }

    if profile
        .missing_gates
        .iter()
        .any(|item| item.contains("MANWE_BASE_URL"))
    {
        actions.push(ServiceAction {
            action_id: "onboarding.set_manwe_endpoint".to_string(),
            action_type: "human_gate".to_string(),
            title: "Set MANWE_BASE_URL before service start work".to_string(),
            command_hint:
                "export MANWE_BASE_URL=http://127.0.0.1:3001 and write to ~/.config/arda/arda.env"
                    .to_string(),
            target_path: Some(format!(
                "{}/.config/arda/arda.env",
                env::var("HOME").unwrap_or_default()
            )),
            requires_human_gate: true,
            description:
                "MANWE endpoint is required for full active runtime setup and launch steps."
                    .to_string(),
            risk: "human_gated".to_string(),
        });
    }

    if profile
        .missing_gates
        .iter()
        .any(|item| item.contains("HERMES_BASE_URL"))
    {
        actions.push(ServiceAction {
            action_id: "onboarding.set_hermes_endpoint".to_string(),
            action_type: "human_gate".to_string(),
            title: "Set HERMES_BASE_URL for comms onboarding".to_string(),
            command_hint: "export HERMES_BASE_URL=http://127.0.0.1:8082 and write to ~/.config/arda/arda.env".to_string(),
            target_path: Some(format!("{}/.config/arda/arda.env", env::var("HOME").unwrap_or_default())),
            requires_human_gate: true,
            description: "Hermes endpoint is required for communication path planning and live-dispatch checks.".to_string(),
            risk: "human_gated".to_string(),
        });
    }

    let checklist = provider_checklist(root);
    for provider in checklist.providers {
        if !provider.missing_env.is_empty() {
            actions.push(ServiceAction {
                action_id: format!("onboarding.provider.{}/set_env", provider.provider_id),
                action_type: "provider_env".to_string(),
                title: format!("Set {} provider env keys", provider.provider_name),
                command_hint: provider
                    .missing_env
                    .iter()
                    .map(|env_key| format!("set {env_key} in ~/.config/arda/arda.env"))
                    .collect::<Vec<_>>()
                    .join(" && "),
                target_path: Some(format!("{}/.config/arda/arda.env", env::var("HOME").unwrap_or_default())),
                requires_human_gate: true,
                description: format!(
                    "{} provider is enabled but is missing required env keys; setup is required before service dispatch. {}",
                    provider.provider_id,
                    provider
                        .action_hint
                        .as_ref()
                        .and_then(|hint| hint.description.clone())
                        .unwrap_or_else(|| "No inline hints available.".to_string())
                ),
                risk: "human_gated".to_string(),
            });
        }
    }

    if let Some(model) = profile
        .endpoints
        .local_model_default
        .as_ref()
        .filter(|model| !model.value.is_empty())
        .map(|model| &model.value)
    {
        actions.push(ServiceAction {
            action_id: "onboarding.low_vram_assist".to_string(),
            action_type: "local_model_handoff".to_string(),
            title: "Enable low-VRAM local assistant handoff suggestion".to_string(),
            command_hint: format!(
                "Point local helper flow at {} if service is healthy and low-VRAM model supports local-only prompts.",
                model
            ),
            target_path: None,
            requires_human_gate: false,
            description: "Optional local-assistant handoff path for non-technical onboarding prompts.".to_string(),
            risk: "safe_local".to_string(),
        });
    }

    ServicePlan {
        contract: ONBOARDING_SERVICE_PLAN_CONTRACT.to_string(),
        generated_at_utc: now_utc(),
        profile: profile.profile.clone(),
        machine_role: profile.machine_role.clone(),
        gate_status: readiness.gate_status,
        approval_contract_required: ONBOARDING_APPROVAL_CONTRACT.to_string(),
        actions,
    }
}

pub fn parse_approval_receipt(path: &Path) -> Result<ApprovalReceipt> {
    let raw = fs::read_to_string(path).context("read approval receipt file")?;
    let parsed: Value = serde_json::from_str(&raw).context("parse approval receipt as json")?;
    let receipt: ApprovalReceipt =
        serde_json::from_value(parsed).context("deserialize approval receipt")?;

    if receipt.contract != ONBOARDING_APPROVAL_CONTRACT {
        return Err(anyhow!(
            "unexpected approval contract '{}', expected {}",
            receipt.contract,
            ONBOARDING_APPROVAL_CONTRACT
        ));
    }

    Ok(receipt)
}

pub fn build_approval_template(plan: &ServicePlan, approver: Option<&str>) -> ApprovalReceipt {
    ApprovalReceipt {
        contract: ONBOARDING_APPROVAL_CONTRACT.to_string(),
        approved: false,
        approver: approver.unwrap_or("operator").to_string(),
        reason: "Review and approve only the listed onboarding actions.".to_string(),
        approved_scope: plan
            .actions
            .iter()
            .filter(|action| action.requires_human_gate)
            .map(|action| action.action_id.clone())
            .collect(),
        approved_at_utc: now_utc(),
        notes: Some(
            "Set approved=true only after reviewing target paths and risks. Do not paste secrets here."
                .to_string(),
        ),
    }
}

pub fn apply_service_plan(
    profile: &EnvironmentProfile,
    root: &Path,
    receipt: Option<&ApprovalReceipt>,
    action_filter: Option<&[String]>,
) -> Result<Vec<ApplyResult>> {
    let plan = build_service_plan(profile, root);
    let mut results = Vec::new();

    let filter: BTreeSet<&str> = action_filter
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .collect();

    for action in &plan.actions {
        if !filter.is_empty() && !filter.contains(action.action_id.as_str()) {
            continue;
        }

        if action.requires_human_gate && !action_is_approved(receipt, action.action_id.as_str()) {
            results.push(make_apply_result(
                action.action_id.as_str(),
                false,
                "blocked: human gate missing or does not include this action",
            ));
            continue;
        }

        let executed = match action.action_id.as_str() {
            "onboarding.emit_profile" => {
                write_profile(&root.join("core/state/environment_profile.json"), profile)?;
                true
            }
            "onboarding.emit_readiness" => {
                let readiness = build_readiness_projection(profile, root);
                write_readiness(
                    &root.join("core/state/setup_console_readiness.json"),
                    &readiness,
                )?;
                true
            }
            "onboarding.emit_provider_checklist" => {
                let checklist = provider_checklist(root);
                write_json(
                    &root.join("audit/onboarding-runs/latest-providers.json"),
                    &serde_json::to_value(&checklist)?,
                )?;
                write_json(
                    &root.join("audit/onboarding-runs/provider-checklist.json"),
                    &serde_json::to_value(&checklist)?,
                )?;
                true
            }
            "onboarding.emit_device_scan" => {
                let scan = device_scan();
                write_json(
                    &root.join("audit/onboarding-runs/latest-device.json"),
                    &serde_json::to_value(&scan)?,
                )?;
                true
            }
            "onboarding.emit_prerequisites" => {
                let report = build_prerequisite_report(profile, root);
                write_json(
                    &root.join("audit/onboarding-runs/latest-prerequisites.json"),
                    &serde_json::to_value(&report)?,
                )?;
                true
            }
            "onboarding.stage_private_config" => {
                let stage = build_private_config_stage(profile, root);
                write_private_config_stage(&stage, root)?;
                true
            }
            "onboarding.emit_guided_session" => {
                let answers = build_operator_answers_template(profile, root);
                let session = build_guided_session(profile, root, Some(answers.clone()));
                write_json(
                    &root.join("audit/onboarding-runs/latest-operator-answers-template.json"),
                    &serde_json::to_value(&answers)?,
                )?;
                write_json(
                    &root.join("audit/onboarding-runs/latest-guided-session.json"),
                    &serde_json::to_value(&session)?,
                )?;
                true
            }
            "onboarding.emit_proposed_config" => {
                build_proposed_config(profile, root)?;
                true
            }
            "onboarding.write_private_config_baseline" => {
                let stage = build_private_config_stage(profile, root);
                apply_private_config_baseline(&stage, root)?;
                true
            }
            other => {
                if !action.requires_human_gate {
                    // Keep these as explicit no-op receipts to preserve auditability.
                    false
                } else {
                    let _path = action_receipt_path(root, other);
                    false
                }
            }
        };

        results.push(make_apply_result(
            action.action_id.as_str(),
            executed,
            if executed {
                "completed"
            } else if action.requires_human_gate {
                "acknowledged: gated mutation remains receipt-only in this onboarding slice"
            } else {
                "skipped: no writable implementation for this action in this slice"
            },
        ));
    }

    let run_dir = onboarding_run_dir(root)?;
    let apply_receipt = json!({
        "contract": ONBOARDING_APPLY_RESULT_CONTRACT,
        "generated_at_utc": now_utc(),
        "plan_contract": plan.contract,
        "plan_machine_role": plan.machine_role,
        "plan_profile": plan.profile,
        "results": results,
        "action_count": plan.actions.len(),
        "applied_count": results.iter().filter(|r| r.execute).count(),
    });
    write_json(&run_dir.join("apply_receipt.json"), &apply_receipt)?;
    write_json(
        &root.join("audit/onboarding-runs/latest-apply-result.json"),
        &apply_receipt,
    )?;

    Ok(results)
}
