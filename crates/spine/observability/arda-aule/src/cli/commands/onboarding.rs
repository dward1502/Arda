#![cfg(feature = "full-cli")]
use std::path::PathBuf;

use arda_hud::{
    apply_service_plan, build_approval_template, build_environment_profile, build_guided_session,
    build_operator_answers_template, build_prerequisite_report, build_private_config_stage,
    build_proposed_config, build_readiness_projection, build_service_plan, device_scan,
    launch_console, parse_approval_receipt, parse_operator_answers, provider_checklist, write_json,
    write_private_config_stage, write_profile, write_readiness,
};
use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use super::super::*;

#[derive(Subcommand)]
pub(crate) enum OnboardingCommands {
    /// Detect machine/environment and render a portable `environment_profile.json`
    Detect {
        /// Profile family override (`local`, `server`, `pi-citadel`, etc.)
        #[arg(long)]
        profile: Option<String>,
        /// Machine role override (`workstation`, `server`, `laptop`, etc.)
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional path for profile write
        #[arg(long)]
        output: Option<PathBuf>,
        /// Persist generated profile to `core/state/environment_profile.json`
        #[arg(long, default_value_t = false)]
        write: bool,
    },
    /// Refresh ARDA-style setup readiness projection
    Readiness {
        /// Optional path for generated state write
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional setup-console receipt directory
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    /// Render a provider/env checklist for Manwe onboarding
    ProviderChecklist {
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate a receipt-backed onboarding action plan
    ServicePlan {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate a human-gate approval receipt template for current service plan
    ApprovalTemplate {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        #[arg(long)]
        approver: Option<String>,
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Detect local hardware/runtime and known peer information
    DeviceScan {
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Render installer prerequisite classification for this machine
    Prerequisites {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Stage secret-safe private config proposal under audit/onboarding-runs
    PrivateConfigStage {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate editable operator-choice answers for the guided flow
    AnswerTemplate {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate guided non-technical onboarding session projection
    GuidedSession {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional edited operator answers JSON
        #[arg(long)]
        answers: Option<PathBuf>,
        /// Optional path for JSON output
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Render a proposed config only (no source-of-truth writes)
    ProposeConfig {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional override path for proposal receipt + toml
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Start the onboarding UI and refresh readiness/profile first
    Launch {
        /// Host bind address
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        /// Port to serve the onboarding app
        #[arg(long, default_value_t = 8087)]
        port: u16,
        /// Run auto-stop after N seconds; use 0 to run indefinitely
        #[arg(long, default_value_t = 0)]
        serve_seconds: u64,
        /// Optional automatic browser open
        #[arg(long, default_value_t = false)]
        open: bool,
    },
    /// Apply service-plan actions with explicit human-gate enforcement
    ApplyConfig {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        machine_role: Option<String>,
        /// Optional approval receipt JSON for human-gated actions
        #[arg(long)]
        receipt: Option<PathBuf>,
        /// Optional action ids to apply; omit to process the whole plan
        #[arg(long)]
        action_id: Vec<String>,
    },
}

pub(crate) fn handle(command: OnboardingCommands) -> Result<()> {
    let root = arda_root();
    match command {
        OnboardingCommands::Detect {
            profile,
            machine_role,
            output,
            write,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let out = output.unwrap_or_else(|| root.join("core/state/environment_profile.json"));
            if write {
                write_profile(&out, &detected)?;
                eprintln!("wrote {}", out.display());
            }
            println!("{}", serde_json::to_string_pretty(&detected)?);
        }
        OnboardingCommands::Readiness { output, out_dir } => {
            let profile = build_environment_profile(Some(&root), None, None)?;
            let projection = build_readiness_projection(&profile, &root);
            let out =
                output.unwrap_or_else(|| root.join("core/state/setup_console_readiness.json"));
            write_readiness(&out, &projection)?;
            let run_dir = out_dir.unwrap_or_else(|| {
                root.join("audit")
                    .join("onboarding-runs")
                    .join(chrono::Utc::now().format("%Y-%m-%d").to_string())
                    .join(format!(
                        "onboarding-readiness-{}",
                        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
                    ))
            });
            std::fs::create_dir_all(&run_dir).ok();
            let receipt_path = run_dir.join("setup_console_readiness_receipt.json");
            write_json(&receipt_path, &serde_json::to_value(&projection)?)?;
            let summary_path = run_dir.join("SUMMARY.md");
            std::fs::write(
                &summary_path,
                format!(
                    "# Setup Console Readiness\n\n- runner: arda-cli onboarding readiness\n- gate_status: {}\n- state: {}\n- receipt: {}\n",
                    projection.gate_status,
                    out.display(),
                    receipt_path.display()
                ),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "runner": "arda-cli onboarding readiness",
                    "output": out,
                    "state": out,
                    "receipt": receipt_path,
                    "summary": summary_path,
                    "gate_status": projection.gate_status
                }))?
            );
        }
        OnboardingCommands::ProviderChecklist { output } => {
            let checklist = provider_checklist(&root);
            let out = output
                .unwrap_or_else(|| root.join("audit/onboarding-runs/provider-checklist.json"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&checklist)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-providers.json"),
                &serde_json::to_value(&checklist)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&checklist)?);
        }
        OnboardingCommands::ServicePlan {
            profile,
            machine_role,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let plan = build_service_plan(&detected, &root);
            let out =
                output.unwrap_or_else(|| root.join("audit/onboarding-runs/service-plan.json"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&plan)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-service-plan.json"),
                &serde_json::to_value(&plan)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        OnboardingCommands::ApprovalTemplate {
            profile,
            machine_role,
            approver,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let plan = build_service_plan(&detected, &root);
            let approval = build_approval_template(&plan, approver.as_deref());
            let out =
                output.unwrap_or_else(|| root.join("audit/onboarding-runs/approval-template.json"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&approval)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-approval-template.json"),
                &serde_json::to_value(&approval)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&approval)?);
        }
        OnboardingCommands::DeviceScan { output } => {
            let scan = device_scan();
            let out = output.unwrap_or_else(|| root.join("audit/onboarding-runs/device-scan.json"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&scan)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-device.json"),
                &serde_json::to_value(&scan)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&scan)?);
        }
        OnboardingCommands::Prerequisites {
            profile,
            machine_role,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let report = build_prerequisite_report(&detected, &root);
            let out =
                output.unwrap_or_else(|| root.join("audit/onboarding-runs/prerequisites.json"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&report)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-prerequisites.json"),
                &serde_json::to_value(&report)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OnboardingCommands::PrivateConfigStage {
            profile,
            machine_role,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let stage = build_private_config_stage(&detected, &root);
            let proposed_path = write_private_config_stage(&stage, &root)?;
            if let Some(out) = output {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                write_json(&out, &serde_json::to_value(&stage)?)?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "stage": stage,
                    "proposed_env": proposed_path,
                }))?
            );
        }
        OnboardingCommands::AnswerTemplate {
            profile,
            machine_role,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let answers = build_operator_answers_template(&detected, &root);
            let out = output.unwrap_or_else(|| {
                root.join("audit/onboarding-runs/operator-answers-template.json")
            });
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&answers)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-operator-answers-template.json"),
                &serde_json::to_value(&answers)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&answers)?);
        }
        OnboardingCommands::GuidedSession {
            profile,
            machine_role,
            answers,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let parsed_answers = match answers {
                Some(path) => Some(parse_operator_answers(&path)?),
                None => None,
            };
            let session = build_guided_session(&detected, &root, parsed_answers);
            let out =
                output.unwrap_or_else(|| root.join("audit/onboarding-runs/guided-session.json"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            write_json(&out, &serde_json::to_value(&session)?)?;
            write_json(
                &root.join("audit/onboarding-runs/latest-guided-session.json"),
                &serde_json::to_value(&session)?,
            )?;
            if !root
                .join("audit/onboarding-runs/latest-operator-answers-template.json")
                .exists()
            {
                write_json(
                    &root.join("audit/onboarding-runs/latest-operator-answers-template.json"),
                    &serde_json::to_value(&session.answers)?,
                )?;
            }
            println!("{}", serde_json::to_string_pretty(&session)?);
        }
        OnboardingCommands::ProposeConfig {
            profile,
            machine_role,
            output,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let out_path = build_proposed_config(&detected, &root)?;
            if let Some(out) = output {
                std::fs::create_dir_all(&out)?;
                let proposed_path = out.join("proposed-config.toml");
                let file = std::fs::read_to_string(&out_path)?;
                std::fs::write(&proposed_path, file)?;
                println!("proposal: {}", proposed_path.display());
            } else {
                println!("proposal: {}", out_path.display());
            }
        }
        OnboardingCommands::Launch {
            bind,
            port,
            serve_seconds,
            open,
        } => {
            launch_console(&root, &bind, port, serve_seconds, open)?;
            if serve_seconds == 0 {
                println!("server stopped");
            } else {
                println!("server stopped after {} seconds", serve_seconds);
            }
        }
        OnboardingCommands::ApplyConfig {
            profile,
            machine_role,
            receipt,
            action_id,
        } => {
            let detected = build_environment_profile(
                Some(&root),
                profile.as_deref(),
                machine_role.as_deref(),
            )?;
            let approval = match receipt {
                Some(path) => Some(parse_approval_receipt(&path)?),
                None => None,
            };
            let action_filter = if action_id.is_empty() {
                None
            } else {
                Some(action_id.as_slice())
            };
            let results = apply_service_plan(&detected, &root, approval.as_ref(), action_filter)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "contract": "arda.onboarding.apply_result.summary.v1",
                    "applied_count": results.iter().filter(|item| item.execute).count(),
                    "total_count": results.len(),
                    "results": results,
                }))?
            );
        }
    }
    Ok(())
}
