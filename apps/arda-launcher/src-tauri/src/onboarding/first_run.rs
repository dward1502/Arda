use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use toml::Value as TomlValue;

use crate::onboarding::guided::build_guided_session;
use crate::onboarding::helpers::now_utc;
use crate::onboarding::prerequisites::build_prerequisite_report;
use crate::onboarding::provider::provider_checklist;
use crate::onboarding::readiness::build_readiness_projection;
use crate::onboarding::service_plan::build_service_plan;
use crate::onboarding::types::*;

const SUPPORTED_PROFILE_ID: &str = "bluefin-lts-10-x86_64";

fn parse_os_release(raw: &str) -> BTreeMap<String, String> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((
                key.to_string(),
                value.trim().trim_matches(['\'', '"']).to_string(),
            ))
        })
        .collect()
}

fn compatibility_from(raw: &str, architecture: &str) -> CompatibilityProjection {
    let os = parse_os_release(raw);
    let os_id = os
        .get("ID")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let version_id = os
        .get("VERSION_ID")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let pretty_name = os
        .get("PRETTY_NAME")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let supported = architecture == "x86_64"
        && os_id == "centos"
        && version_id == "10"
        && pretty_name.starts_with("Bluefin LTS");

    CompatibilityProjection {
        contract: "arda.release-compatibility.v1".to_string(),
        status: if supported {
            "supported"
        } else {
            "unsupported"
        }
        .to_string(),
        profile_id: supported.then(|| SUPPORTED_PROFILE_ID.to_string()),
        supported_profile: SUPPORTED_PROFILE_ID.to_string(),
        architecture: architecture.to_string(),
        os_id,
        version_id,
        pretty_name,
        message: if supported {
            "Supported profile verified before setup actions.".to_string()
        } else {
            "Unsupported profile: installation and configuration mutations remain blocked before any partial setup.".to_string()
        },
    }
}

fn compatibility() -> CompatibilityProjection {
    let raw = fs::read_to_string("/etc/os-release").unwrap_or_default();
    compatibility_from(&raw, std::env::consts::ARCH)
}

fn optional_services(root: &Path) -> Vec<OptionalServiceProjection> {
    let Ok(raw) = fs::read_to_string(root.join("services.toml")) else {
        return Vec::new();
    };
    let Ok(parsed) = raw.parse::<TomlValue>() else {
        return Vec::new();
    };

    parsed
        .get("service")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|service| {
            let table = service.as_table()?;
            if !table
                .get("optional")
                .and_then(TomlValue::as_bool)
                .unwrap_or(false)
            {
                return None;
            }
            Some(OptionalServiceProjection {
                service_id: table.get("name")?.as_str()?.to_string(),
                status: "opt_in".to_string(),
                enabled: false,
                blocks_workbench: false,
                guidance: "Optional application is not started by first-run setup; enable it explicitly after Workbench readiness passes.".to_string(),
            })
        })
        .collect()
}

pub fn build_first_run_projection(root: &Path) -> Result<FirstRunProjection> {
    let profile = crate::onboarding::build_environment_profile(Some(root), None, None)?;
    let compatibility = compatibility();
    let prerequisites = build_prerequisite_report(&profile, root);
    let providers = provider_checklist(root);
    let readiness = build_readiness_projection(&profile, root);
    let service_plan = build_service_plan(&profile, root);
    let guided = build_guided_session(&profile, root, None);
    let missing_provider_env = providers
        .providers
        .iter()
        .filter(|provider| provider.enabled && !provider.missing_env.is_empty())
        .count();
    let endpoint_missing = profile
        .missing_gates
        .iter()
        .any(|gate| gate.contains("MANWE_BASE_URL"));
    let degraded = readiness.gate_status != "pass";
    let recovery = vec![
        RecoveryGuidance {
            condition_id: "offline".to_string(),
            detected: endpoint_missing,
            summary: "No configured Manwe route; Workbench remains available for local inspection without provider execution.".to_string(),
            action: "Configure and approve MANWE_BASE_URL, then rerun first-run readiness. Do not enter provider secrets in receipts.".to_string(),
        },
        RecoveryGuidance {
            condition_id: "provider_unavailable".to_string(),
            detected: missing_provider_env > 0,
            summary: format!("{missing_provider_env} enabled provider configuration(s) are missing required private environment keys."),
            action: "Write provider credentials only to the private Arda environment file after explicit operator approval; then refresh readiness.".to_string(),
        },
        RecoveryGuidance {
            condition_id: "degraded".to_string(),
            detected: degraded,
            summary: "One or more readiness checks require review; diagnostics remain available while startup is blocked.".to_string(),
            action: "Follow each failed or warning check's recovery text, then refresh this sequence. Safe reset requires backup and quarantine.".to_string(),
        },
        RecoveryGuidance {
            condition_id: "recovery".to_string(),
            detected: false,
            summary: "If setup becomes inconsistent, preserve diagnostics and state before changing installation state.".to_string(),
            action: "Capture diagnostics, back up state, run readiness, and use verified restore or safe reset; uninstall preserves state by default.".to_string(),
        },
    ];
    let gate_status = if compatibility.status != "supported" {
        "fail"
    } else {
        readiness.gate_status.as_str()
    };

    Ok(FirstRunProjection {
        contract: "arda.launcher.first-run.v1".to_string(),
        generated_at_utc: now_utc(),
        gate_status: gate_status.to_string(),
        can_start_workbench: gate_status == "pass",
        mutation_policy: "explicit_approval_and_receipt_required".to_string(),
        profile: profile.profile.clone(),
        machine_role: profile.machine_role.clone(),
        compatibility,
        prerequisites,
        providers,
        readiness,
        service_plan,
        guided,
        recovery,
        optional_services: optional_services(root),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_profile_contract_is_exact() {
        let projection = compatibility_from(
            "ID=centos\nVERSION_ID=10\nPRETTY_NAME=\"Bluefin LTS 10\"\n",
            "x86_64",
        );
        assert_eq!(projection.status, "supported");
        assert_eq!(projection.profile_id.as_deref(), Some(SUPPORTED_PROFILE_ID));
    }

    #[test]
    fn unsupported_profile_is_blocked_before_setup() {
        let projection = compatibility_from(
            "ID=ubuntu\nVERSION_ID=24.04\nPRETTY_NAME=\"Ubuntu 24.04\"\n",
            "x86_64",
        );
        assert_eq!(projection.status, "unsupported");
        assert!(projection.profile_id.is_none());
        assert!(projection.message.contains("before any partial setup"));
    }

    #[test]
    fn optional_services_are_non_blocking_and_disabled_by_default() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let optional = optional_services(&root);
        let hud = optional
            .iter()
            .find(|service| service.service_id == "arda-hud")
            .expect("arda-hud remains explicitly optional in services.toml");
        assert!(!hud.enabled);
        assert!(!hud.blocks_workbench);
    }

    #[test]
    fn workspace_first_run_projection_unifies_all_six_stages() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let projection = build_first_run_projection(&root).expect("first-run projection builds");
        assert_eq!(projection.contract, "arda.launcher.first-run.v1");
        assert_eq!(
            projection.mutation_policy,
            "explicit_approval_and_receipt_required"
        );
        assert!(!projection.prerequisites.checks.is_empty());
        assert!(!projection.providers.providers.is_empty());
        assert!(!projection.service_plan.actions.is_empty());
        assert!(!projection.readiness.checks.is_empty());
        assert!(!projection.guided.steps.is_empty());
        if projection.compatibility.status != "supported" {
            assert_eq!(projection.gate_status, "fail");
            assert!(!projection.can_start_workbench);
        }
    }
}
