use serde_json::Value;
use std::path::Path;

use crate::onboarding::constants::{
    ONBOARDING_GUIDED_SESSION_CONTRACT, ONBOARDING_OPERATOR_ANSWERS_CONTRACT,
};
use crate::onboarding::device::device_scan;
use crate::onboarding::helpers::now_utc;
use crate::onboarding::prerequisites::build_prerequisite_report;
use crate::onboarding::private_config::build_operator_answers_template;
use crate::onboarding::provider::provider_checklist;
use crate::onboarding::types::*;

fn guided_step(
    step_id: &str,
    title: &str,
    status: &str,
    prompt: &str,
    evidence: Vec<String>,
    next_action: &str,
) -> GuidedStep {
    GuidedStep {
        step_id: step_id.to_string(),
        title: title.to_string(),
        status: status.to_string(),
        prompt: prompt.to_string(),
        evidence,
        next_action: next_action.to_string(),
    }
}

pub fn build_guided_session(
    profile: &EnvironmentProfile,
    root: &Path,
    answers_override: Option<OperatorAnswers>,
) -> GuidedSession {
    let answers =
        answers_override.unwrap_or_else(|| build_operator_answers_template(profile, root));
    let providers = provider_checklist(root);
    let device = device_scan();
    let prerequisite_report = build_prerequisite_report(profile, root);
    let missing_provider_env = providers
        .providers
        .iter()
        .filter(|provider| provider.enabled && !provider.missing_env.is_empty())
        .count();
    let prereq_warnings = prerequisite_report
        .summary
        .get("warn")
        .copied()
        .unwrap_or(0);
    let peer_summary = device.tailscale.get("tailscale_peer_summary");
    let peer_total = peer_summary
        .and_then(|value| value.get("total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let active_peers = peer_summary
        .and_then(|value| value.get("active_online"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let local_assistant_available = profile.endpoints.local_model_base_url.is_some()
        || providers.providers.iter().any(|provider| {
            provider.enabled
                && provider
                    .provider_profile
                    .as_ref()
                    .and_then(|profile| profile.locality.as_deref())
                    == Some("local")
        });

    let mut steps = vec![
        guided_step(
            "welcome",
            "Welcome",
            "ready",
            "Start from local evidence and keep setup changes gated.",
            vec![
                "First Light console available as a local static app.".to_string(),
                "Authority remains crate-generated contracts and receipts.".to_string(),
            ],
            "Review machine role before writing any private config.",
        ),
        guided_step(
            "identify_machine",
            "Identify this machine",
            if answers.machine_role == "unknown" { "needs_confirmation" } else { "ready" },
            "Choose what this machine is for without relying on hostname assumptions.",
            vec![
                format!("detected_role={}", profile.machine_role),
                format!("operator_role={}", answers.machine_role),
                format!("profile={}", answers.profile),
            ],
            "Edit operator answers if the detected role is wrong.",
        ),
        guided_step(
            "discover_capabilities",
            "Discover capabilities",
            if prereq_warnings == 0 { "ready" } else { "warn" },
            "Check tools, local runtime, GUI build prerequisites, and private path readiness.",
            vec![
                format!("prerequisite_warnings={prereq_warnings}"),
                format!("platform={}", device.platform),
                format!("architecture={}", device.architecture),
            ],
            "Resolve prerequisite warnings only where this host needs that capability.",
        ),
        guided_step(
            "connect_providers",
            "Connect providers",
            if missing_provider_env == 0 { "ready" } else { "needs_secrets" },
            "Pick Manwe providers and keep provider keys in the private env file only.",
            vec![
                format!("configured_providers={}", providers.providers.len()),
                format!("selected_providers={}", answers.selected_providers.join(",")),
                format!("missing_provider_env={missing_provider_env}"),
            ],
            "Use provider checklist and private config stage before attempting live routes.",
        ),
        guided_step(
            "privacy_autonomy",
            "Configure privacy and autonomy",
            if answers.mutation_requires_human_gate { "gated" } else { "review" },
            "Select read-only, gated mutation, or disabled mutation posture.",
            vec![
                format!("autonomy_posture={}", answers.autonomy_posture),
                format!("mutation_requires_human_gate={}", answers.mutation_requires_human_gate),
            ],
            "Keep human gate enabled until private config and service plans are reviewed.",
        ),
        guided_step(
            "l3_readiness",
            "Verify L3 readiness",
            "human_gated",
            "Separate safe-local packet selection from bounded mutation authority before trusting autonomous work.",
            vec![
                "read core/state/l3_readiness_projection.json".to_string(),
                "bounded mutation must be true before local mutation".to_string(),
                "broad, external, destructive, service, credential, fleet, funds, legal, and customer actions remain blocked".to_string(),
            ],
            "Run the L3 readiness projection export and review docs/operations/l3-readiness-onboarding.md.",
        ),
        guided_step(
            "communications",
            "Configure communications",
            if answers.enable_hermes_discord { "optional_enabled" } else { "optional_skipped" },
            "Hermes and Discord are optional onboarding capabilities, not install prerequisites.",
            vec![
                format!("enable_hermes_discord={}", answers.enable_hermes_discord),
                format!("hermes_gate_missing={}", profile.missing_gates.iter().any(|gate| gate == "HERMES_BASE_URL")),
            ],
            "Use Hermes-specific checks only after endpoint and token setup are ready.",
        ),
        guided_step(
            "devices",
            "Configure local and fleet devices",
            if answers.enable_fleet_discovery { "ready" } else { "optional_skipped" },
            "Detect local and Tailscale devices while allowing names and capabilities to differ.",
            vec![
                format!("peer_total={peer_total}"),
                format!("active_peers={active_peers}"),
                format!("fleet_discovery={}", answers.enable_fleet_discovery),
            ],
            "Treat duplicate/offline peers as review evidence, not hard install blockers.",
        ),
        guided_step(
            "assistant_handoff",
            "Local assistant handoff",
            if answers.prefer_local_assistant && local_assistant_available {
                "ready"
            } else if answers.prefer_local_assistant {
                "warn"
            } else {
                "optional_skipped"
            },
            "Offer a low-VRAM local helper when a local route is available; otherwise continue deterministically.",
            vec![
                format!("prefer_local_assistant={}", answers.prefer_local_assistant),
                format!("local_assistant_available={local_assistant_available}"),
            ],
            "Use deterministic prompts when no local model route is healthy.",
        ),
        guided_step(
            "review_apply",
            "Review and apply",
            "human_gated",
            "Review proposed changes and apply only receipt-backed safe steps.",
            vec![
                "service-plan read-only actions can refresh projections".to_string(),
                "mutating actions require arda.onboarding.human_gate.v1".to_string(),
            ],
            "Run apply-config without approval to refresh projections; provide approval only for reviewed future writes.",
        ),
    ];

    if profile
        .missing_gates
        .iter()
        .any(|gate| gate == "MANWE_BASE_URL")
    {
        steps.push(guided_step(
            "manwe_endpoint",
            "Manwe endpoint gate",
            "warn",
            "Manwe is the model router; endpoint setup is required for live routing.",
            vec!["missing MANWE_BASE_URL".to_string()],
            "Stage MANWE_BASE_URL in the private config proposal before service checks.",
        ));
    }

    let next_actions = steps
        .iter()
        .filter(|step| step.status != "ready" && step.status != "optional_skipped")
        .map(|step| format!("{}: {}", step.step_id, step.next_action))
        .collect::<Vec<_>>();

    GuidedSession {
        contract: ONBOARDING_GUIDED_SESSION_CONTRACT.to_string(),
        generated_at_utc: now_utc(),
        profile: answers.profile.clone(),
        machine_role: answers.machine_role.clone(),
        answers_contract: ONBOARDING_OPERATOR_ANSWERS_CONTRACT.to_string(),
        answers,
        steps,
        next_actions,
    }
}
