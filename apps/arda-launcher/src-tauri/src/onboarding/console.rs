use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::device_scan;
use crate::environment::build_environment_profile;
use crate::onboarding::guided::build_guided_session;
use crate::onboarding::helpers::now_utc;
use crate::onboarding::io::{onboarding_run_dir, write_json, write_profile, write_readiness};
use crate::onboarding::prerequisites::build_prerequisite_report;
use crate::onboarding::private_config::{
    build_operator_answers_template, build_private_config_stage, write_private_config_stage,
};
use crate::onboarding::provider::provider_checklist;
use crate::onboarding::readiness::build_readiness_projection;
use crate::onboarding::service_plan::{build_approval_template, build_service_plan};

pub fn launch_console(
    root: &Path,
    bind: &str,
    port: u16,
    serve_seconds: u64,
    open_browser: bool,
) -> Result<()> {
    let addr = format!("{bind}:{port}");
    let index_url = format!("http://{bind}:{port}/apps/arda-launcher/");
    let run_dir = onboarding_run_dir(root)?;
    let receipt = run_dir.join("launch_receipt.json");
    let profile = build_environment_profile(Some(root), None, None)?;
    let readiness = build_readiness_projection(&profile, root);
    let providers = provider_checklist(root);
    let device = device_scan();
    let prerequisites = build_prerequisite_report(&profile, root);
    let private_config = build_private_config_stage(&profile, root);
    let operator_answers = build_operator_answers_template(&profile, root);
    let guided_session = build_guided_session(&profile, root, Some(operator_answers.clone()));
    let service_plan = build_service_plan(&profile, root);
    let approval_template = build_approval_template(&service_plan, None);

    write_profile(&root.join("core/state/environment_profile.json"), &profile)?;
    write_readiness(
        &root.join("core/state/setup_console_readiness.json"),
        &readiness,
    )?;
    write_json(
        &root.join("audit/onboarding-runs/latest-providers.json"),
        &serde_json::to_value(&providers)?,
    )?;
    write_json(
        &root.join("audit/onboarding-runs/latest-device.json"),
        &serde_json::to_value(&device)?,
    )?;
    write_json(
        &root.join("audit/onboarding-runs/latest-prerequisites.json"),
        &serde_json::to_value(&prerequisites)?,
    )?;
    write_private_config_stage(&private_config, root)?;
    write_json(
        &root.join("audit/onboarding-runs/latest-operator-answers-template.json"),
        &serde_json::to_value(&operator_answers)?,
    )?;
    write_json(
        &root.join("audit/onboarding-runs/latest-guided-session.json"),
        &serde_json::to_value(&guided_session)?,
    )?;
    write_json(
        &root.join("audit/onboarding-runs/latest-service-plan.json"),
        &serde_json::to_value(&service_plan)?,
    )?;
    write_json(
        &root.join("audit/onboarding-runs/latest-approval-template.json"),
        &serde_json::to_value(&approval_template)?,
    )?;
    write_json(
        &receipt,
        &json!({
            "contract": "arda.onboarding.launch_receipt.v1",
            "run": now_utc(),
            "address": addr,
            "url": index_url,
            "artifacts": {
                "environment_profile": "core/state/environment_profile.json",
                "setup_readiness": "core/state/setup_console_readiness.json",
                "provider_checklist": "audit/onboarding-runs/latest-providers.json",
                "device_scan": "audit/onboarding-runs/latest-device.json",
                "prerequisites": "audit/onboarding-runs/latest-prerequisites.json",
                "private_config_stage": "audit/onboarding-runs/latest-private-config-stage.json",
                "private_config_proposal": "audit/onboarding-runs/latest-proposed-arda.env",
                "operator_answers_template": "audit/onboarding-runs/latest-operator-answers-template.json",
                "guided_session": "audit/onboarding-runs/latest-guided-session.json",
                "service_plan": "audit/onboarding-runs/latest-service-plan.json",
                "approval_template": "audit/onboarding-runs/latest-approval-template.json"
            }
        }),
    )?;

    let mut server = Command::new("python3")
        .args(["-m", "http.server", &port.to_string(), "--bind", bind])
        .current_dir(root)
        .spawn()
        .context("start simple onboarding server")?;

    std::thread::sleep(Duration::from_secs(1));
    if let Some(exit) = server.try_wait()? {
        let code = exit.code().unwrap_or(-1);
        return Err(anyhow!(
            "onboarding server failed to start on {addr} with exit code {code}; check python/network permissions"
        ));
    }

    println!("Started onboarding console at {index_url}");
    println!("Artifacts: {}", run_dir.display());
    println!("Press Ctrl-C or wait to stop this process.");

    if open_browser {
        let _ = Command::new("xdg-open").arg(&index_url).spawn();
    }

    if serve_seconds == 0 {
        let status = server
            .wait()
            .map_err(|e| anyhow!("failed waiting for onboarding server: {e}"))?;
        if !status.success() {
            return Err(anyhow!(
                "onboarding server exited early with status {status}"
            ));
        }
        return Ok(());
    }

    thread::sleep(Duration::from_secs(serve_seconds));
    if let Some(status) = server
        .try_wait()
        .map_err(|e| anyhow!("failed checking onboarding server state: {e}"))?
    {
        if !status.success() {
            return Err(anyhow!(
                "onboarding server exited during runtime with status {status}"
            ));
        }
        return Ok(());
    }

    let _ = server.kill();
    let _ = server.wait();
    Ok(())
}
