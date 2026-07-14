use super::*;

pub(crate) fn enforce_policy_guard(command: &Commands) -> anyhow::Result<()> {
    let ruleset = load_active_ruleset();
    let policy = ruleset.get("policy").cloned().unwrap_or_else(|| json!({}));
    let strict_gate = policy
        .get("gate_strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let active_ruleset = ruleset
        .get("active_ruleset")
        .and_then(|v| v.as_str())
        .unwrap_or("annunimas_totality")
        .to_string();
    let network_templates: Vec<String> = policy
        .get("network_unlock_templates")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|v| v.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let descriptor = command_policy_descriptor(command);
    let mut denied_reason: Option<String> = None;
    let autonomy_runtime = load_autonomy_runtime_state();
    let permission_profile = load_permission_profile_state();
    let approval_state = load_human_augmentation_approval_state();
    let governance_lane = governance_lane_for_descriptor(&descriptor, &policy);
    let governance_policy_mode = governance_policy_mode_for_descriptor(&descriptor, &policy);

    if strict_gate && descriptor.is_network {
        let sig = descriptor.signature.to_ascii_lowercase();
        let allowed_by_template = network_templates
            .iter()
            .any(|t| sig.contains(t) || t.contains(&sig));
        if !allowed_by_template {
            denied_reason = Some(format!(
                "strict_gate blocks network command without unlock template match: {}",
                descriptor.signature
            ));
        }
    }

    if denied_reason.is_none() && strict_gate && descriptor.is_destructive {
        if let Commands::Hades {
            command:
                HadesCommands::Remove {
                    quorum_approvers,
                    quorum_evidence,
                    ..
                },
        } = command
        {
            let triad = ["aurelius", "bacon", "sun_tzu"];
            let mut unique = std::collections::BTreeSet::new();
            for approver in quorum_approvers {
                let normalized = approver.trim().to_ascii_lowercase();
                if triad.contains(&normalized.as_str()) {
                    unique.insert(normalized);
                }
            }
            if unique.len() < 2 || quorum_evidence.is_empty() {
                denied_reason = Some(
                    "strict_gate requires 2-of-3 triad quorum approvers plus evidence for hades remove"
                        .to_string(),
                );
            }
        }
    }

    if denied_reason.is_none() {
        if let Some(reason) = governance_policy_denial_reason(&descriptor, &policy, &approval_state)
        {
            denied_reason = Some(reason);
        }
    }

    if denied_reason.is_none() {
        if let Some(reason) =
            human_augmentation_denial_reason(&descriptor, &policy, &approval_state)
        {
            denied_reason = Some(reason);
        }
    }

    if denied_reason.is_none() {
        if let Some(reason) = permission_profile_denial_reason(&descriptor, &permission_profile) {
            denied_reason = Some(reason);
        }
    }

    if denied_reason.is_none()
        && autonomy_runtime
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("normal")
            == "degraded"
        && (descriptor.is_network || descriptor.is_destructive)
    {
        let reasons = autonomy_runtime
            .get("violations")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "unspecified".to_string());
        denied_reason = Some(format!(
            "autonomy_runtime is degraded; blocking high-impact command (violations={reasons})"
        ));
    }

    let allowed = denied_reason.is_none();
    let reason = denied_reason.unwrap_or_else(|| "allowed".to_string());
    persist_policy_guard_record(&json!({
        "ts_utc": Utc::now().to_rfc3339(),
        "active_ruleset": active_ruleset,
        "strict_gate": strict_gate,
        "command": descriptor.signature,
        "is_network": descriptor.is_network,
        "is_destructive": descriptor.is_destructive,
        "decision_class": descriptor.decision_class,
        "governance_lane": governance_lane.as_str(),
        "governance_policy": {
            "mode": governance_policy_mode,
            "decision_class": descriptor.decision_class,
            "lane": governance_lane.as_str()
        },
        "autonomy_mode": autonomy_runtime.get("mode").cloned().unwrap_or(json!("normal")),
        "permission_profile": permission_profile
            .get("active_profile")
            .cloned()
            .unwrap_or(json!("unknown")),
        "allowed": allowed,
        "reason": reason,
    }));
    persist_permission_profile_decision(&json!({
        "ts_utc": Utc::now().to_rfc3339(),
        "command": descriptor.signature,
        "is_network": descriptor.is_network,
        "is_destructive": descriptor.is_destructive,
        "decision_class": descriptor.decision_class,
        "governance_lane": governance_lane.as_str(),
        "governance_policy": {
            "mode": governance_policy_mode,
            "decision_class": descriptor.decision_class,
            "lane": governance_lane.as_str()
        },
        "active_profile": permission_profile.get("active_profile").cloned().unwrap_or(json!("unknown")),
        "allowed": allowed,
        "reason": reason,
    }));
    if !allowed {
        emit_policy_guard_escalation(&descriptor.signature, &reason);
        anyhow::bail!("policy_guard denied command: {reason}");
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct CommandPolicyDescriptor {
    signature: String,
    is_network: bool,
    is_destructive: bool,
    decision_class: &'static str,
    human_authorizers: Vec<String>,
    triad_approvers: Vec<String>,
    triad_evidence: Vec<String>,
}

fn command_policy_descriptor(command: &Commands) -> CommandPolicyDescriptor {
    match command {
        Commands::Hades {
            command:
                HadesCommands::Remove {
                    authorized_by,
                    quorum_approvers,
                    quorum_evidence,
                    ..
                },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- hades remove".to_string(),
            is_network: false,
            is_destructive: true,
            decision_class: "destructive_delete",
            human_authorizers: vec![authorized_by.to_ascii_lowercase()],
            triad_approvers: quorum_approvers
                .iter()
                .map(|value| value.trim().to_ascii_lowercase())
                .collect(),
            triad_evidence: quorum_evidence.clone(),
        },
        Commands::Hermes {
            command: HermesCommands::Send { .. },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- hermes send".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        Commands::Hermes {
            command:
                HermesCommands::IlluvatarFanout { .. }
                | HermesCommands::PollOnce
                | HermesCommands::RetryOutbound { .. }
                | HermesCommands::RetryRerouteDlq { .. },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- hermes network-op".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        Commands::Charon {
            command:
                CharonCommands::Route { .. }
                | CharonCommands::Cooldown { .. }
                | CharonCommands::ProviderResult { .. }
                | CharonCommands::ReloadConfig,
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- charon route-governor".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "provider_reroute",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        Commands::Charon {
            command: CharonCommands::Proxy { .. },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- charon network-op".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        Commands::Control {
            command:
                ControlCommands::ApplyOpencodeRouteGovernor { .. }
                | ControlCommands::ApplyRuntimeRecoveryRouteGovernor { .. },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- control route-governor".to_string(),
            is_network: false,
            is_destructive: false,
            decision_class: "provider_reroute",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        Commands::Athena {
            command: AthenaCommands::PolicyPromote { .. },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- athena policy-promote".to_string(),
            is_network: false,
            is_destructive: false,
            decision_class: "strategy_change",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        Commands::Utility {
            command: UtilityCommands::TaskPivot { .. },
        } => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- utility task-pivot".to_string(),
            is_network: false,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
        _ => CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- local-op".to_string(),
            is_network: false,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GovernanceLane {
    Autonomous,
    TriadQuorum,
    HumanRequired,
}

impl GovernanceLane {
    fn as_str(self) -> &'static str {
        match self {
            Self::Autonomous => "autonomous",
            Self::TriadQuorum => "triad_quorum",
            Self::HumanRequired => "human_required",
        }
    }
}

fn governance_lane_for_descriptor(
    descriptor: &CommandPolicyDescriptor,
    policy: &serde_json::Value,
) -> GovernanceLane {
    let routing = policy
        .get("human_augmentation")
        .and_then(|v| v.get("critical_decision_routing"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    if decision_class_in(
        &routing,
        "human_required_classes",
        descriptor.decision_class,
    ) {
        GovernanceLane::HumanRequired
    } else if decision_class_in(&routing, "triad_quorum_classes", descriptor.decision_class) {
        GovernanceLane::TriadQuorum
    } else {
        GovernanceLane::Autonomous
    }
}

fn decision_class_in(routing: &serde_json::Value, key: &str, needle: &str) -> bool {
    routing
        .get(key)
        .and_then(|v| v.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|v| v.as_str())
                .any(|v| v == needle)
        })
        .unwrap_or(false)
}

fn load_human_augmentation_approval_state() -> serde_json::Value {
    let path = std::path::PathBuf::from("core/state/human_augmentation_approval.json");
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => {
            return json!({
                "approvals": []
            });
        }
    };
    serde_json::from_str(&content).unwrap_or_else(|_| {
        json!({
            "approvals": []
        })
    })
}

fn human_augmentation_denial_reason(
    descriptor: &CommandPolicyDescriptor,
    policy: &serde_json::Value,
    approval_state: &serde_json::Value,
) -> Option<String> {
    match governance_lane_for_descriptor(descriptor, policy) {
        GovernanceLane::Autonomous => None,
        GovernanceLane::TriadQuorum => {
            if has_descriptor_triad_quorum(descriptor)
                || has_matching_approval(approval_state, descriptor, |approval| {
                    approval_has_triad_quorum(approval)
                })
            {
                None
            } else {
                Some(format!(
                    "decision class '{}' requires 2-of-3 philosopher triad approval with evidence",
                    descriptor.decision_class
                ))
            }
        }
        GovernanceLane::HumanRequired => {
            let requires_sovereign_override = policy
                .get("human_augmentation")
                .and_then(|v| v.get("consensus"))
                .and_then(|v| v.get("sovereign_override_required_for"))
                .and_then(|v| v.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|v| v.as_str())
                        .any(|v| v == descriptor.decision_class)
                })
                .unwrap_or(false);
            if has_descriptor_human_approval(descriptor, requires_sovereign_override)
                || has_matching_approval(approval_state, descriptor, |approval| {
                    approval_has_human_authorizer(approval, requires_sovereign_override)
                })
            {
                None
            } else if requires_sovereign_override {
                Some(format!(
                    "decision class '{}' requires sovereign human approval from arandur",
                    descriptor.decision_class
                ))
            } else {
                Some(format!(
                    "decision class '{}' requires human approval from ceo or arandur",
                    descriptor.decision_class
                ))
            }
        }
    }
}

fn governance_policy_mode_for_descriptor(
    descriptor: &CommandPolicyDescriptor,
    policy: &serde_json::Value,
) -> &'static str {
    if let Some(mode) = policy
        .get("governance_policy")
        .and_then(|v| v.get("action_class_modes"))
        .and_then(|v| v.get(descriptor.decision_class))
        .and_then(|v| v.as_str())
        .and_then(normalize_governance_policy_mode)
    {
        return mode;
    }

    match governance_lane_for_descriptor(descriptor, policy) {
        GovernanceLane::Autonomous => "record_and_proceed",
        GovernanceLane::TriadQuorum => "escalate_to_human",
        GovernanceLane::HumanRequired => "require_independent_receipts",
    }
}

fn normalize_governance_policy_mode(mode: &str) -> Option<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "observe_only" => Some("observe_only"),
        "record_and_proceed" => Some("record_and_proceed"),
        "block_on_fail" => Some("block_on_fail"),
        "escalate_to_human" => Some("escalate_to_human"),
        "require_independent_receipts" => Some("require_independent_receipts"),
        _ => None,
    }
}

fn governance_policy_denial_reason(
    descriptor: &CommandPolicyDescriptor,
    policy: &serde_json::Value,
    approval_state: &serde_json::Value,
) -> Option<String> {
    match governance_policy_mode_for_descriptor(descriptor, policy) {
        "observe_only" => None,
        "record_and_proceed" => None,
        "block_on_fail" => {
            if has_descriptor_triad_quorum(descriptor) {
                None
            } else {
                Some(format!(
                    "policy mode 'block_on_fail' requires 2-of-3 philosopher triad evidence for decision class '{}'",
                    descriptor.decision_class
                ))
            }
        }
        "escalate_to_human" => {
            if has_matching_approval(
                approval_state,
                descriptor,
                approval_has_any_human_authorizer,
            ) {
                None
            } else {
                Some(format!(
                    "policy mode 'escalate_to_human' requires matching human approval for decision class '{}'",
                    descriptor.decision_class
                ))
            }
        }
        "require_independent_receipts" => {
            if has_matching_approval(
                approval_state,
                descriptor,
                approval_has_independent_receipts,
            ) {
                None
            } else {
                Some(format!(
                    "policy mode 'require_independent_receipts' requires matching human approval and independent receipts for decision class '{}'",
                    descriptor.decision_class
                ))
            }
        }
        _ => None,
    }
}

fn approval_has_any_human_authorizer(approval: &serde_json::Value) -> bool {
    approval_has_human_authorizer(approval, false)
}

fn approval_has_independent_receipts(approval: &serde_json::Value) -> bool {
    approval_has_human_authorizer(approval, false)
        && approval
            .get("independent_receipts")
            .and_then(|v| v.as_array())
            .map(|entries| entries.len() >= 2)
            .unwrap_or(false)
}

fn has_descriptor_triad_quorum(descriptor: &CommandPolicyDescriptor) -> bool {
    let triad = ["aurelius", "bacon", "sun_tzu"];
    let unique = descriptor
        .triad_approvers
        .iter()
        .filter_map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            triad.contains(&normalized.as_str()).then_some(normalized)
        })
        .collect::<std::collections::BTreeSet<_>>();
    unique.len() >= 2 && !descriptor.triad_evidence.is_empty()
}

fn has_descriptor_human_approval(
    descriptor: &CommandPolicyDescriptor,
    requires_sovereign_override: bool,
) -> bool {
    descriptor.human_authorizers.iter().any(|approver| {
        let normalized = approver.trim().to_ascii_lowercase();
        if requires_sovereign_override {
            normalized == "arandur"
        } else {
            normalized == "ceo" || normalized == "arandur"
        }
    })
}

fn has_matching_approval<F>(
    approval_state: &serde_json::Value,
    descriptor: &CommandPolicyDescriptor,
    validator: F,
) -> bool
where
    F: Fn(&serde_json::Value) -> bool,
{
    approval_state
        .get("approvals")
        .and_then(|v| v.as_array())
        .map(|approvals| {
            approvals.iter().any(|approval| {
                if approval.get("status").and_then(|v| v.as_str()) != Some("approved") {
                    return false;
                }
                if is_expired_ts(approval.get("expires_at_utc").and_then(|v| v.as_str())) {
                    return false;
                }
                let class_match = approval
                    .get("decision_class")
                    .and_then(|v| v.as_str())
                    .map(|value| value == descriptor.decision_class)
                    .unwrap_or(false);
                if !class_match {
                    return false;
                }
                let signature_match = approval
                    .get("command_signature")
                    .and_then(|v| v.as_str())
                    .map(|value| value == descriptor.signature)
                    .unwrap_or(true);
                signature_match && validator(approval)
            })
        })
        .unwrap_or(false)
}

fn approval_has_triad_quorum(approval: &serde_json::Value) -> bool {
    let triad = ["aurelius", "bacon", "sun_tzu"];
    let unique = approval
        .get("approvers")
        .and_then(|v| v.as_array())
        .map(|approvers| {
            approvers
                .iter()
                .filter_map(|entry| entry.as_str())
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| triad.contains(&value.as_str()))
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let evidence_present = approval
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|entries| !entries.is_empty())
        .unwrap_or(false);
    unique.len() >= 2 && evidence_present
}

fn approval_has_human_authorizer(
    approval: &serde_json::Value,
    requires_sovereign_override: bool,
) -> bool {
    approval
        .get("approvers")
        .and_then(|v| v.as_array())
        .map(|approvers| {
            approvers
                .iter()
                .filter_map(|entry| entry.as_str())
                .any(|value| {
                    let normalized = value.trim().to_ascii_lowercase();
                    if requires_sovereign_override {
                        normalized == "arandur"
                    } else {
                        normalized == "ceo" || normalized == "arandur"
                    }
                })
        })
        .unwrap_or(false)
}

fn persist_policy_guard_record(record: &serde_json::Value) {
    let path = crate::annunimas_root().join("data/prometheus/policy_guard.jsonl");
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            tracing::warn!(error = %err, "failed to create policy_guard parent directory");
            return;
        }
    }
    let mut file = match fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => file,
        Err(err) => {
            tracing::warn!(error = %err, "failed to open policy_guard log");
            return;
        }
    };
    let line = match serde_json::to_string(record) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(error = %err, "failed to serialize policy_guard record");
            return;
        }
    };
    if let Err(err) = writeln!(file, "{line}") {
        tracing::warn!(error = %err, "failed to append policy_guard record");
    }
}

fn has_pending_matching_escalation(path: &std::path::Path, reason: &str, command: &str) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let same_reason = value.get("reason").and_then(|v| v.as_str()) == Some(reason);
        let same_command = value.get("command").and_then(|v| v.as_str()) == Some(command);
        let pending = value.get("status").and_then(|v| v.as_str()) == Some("pending");
        if same_reason && same_command && pending {
            return true;
        }
    }
    false
}

fn emit_policy_guard_escalation(command: &str, reason: &str) {
    let now = Utc::now();
    let ts = now.to_rfc3339();
    let id = format!("esc_policy_guard_{}", now.timestamp());
    persist_policy_guard_record(&json!({
        "ts_utc": ts,
        "event": "policy_guard_escalation",
        "escalation_id": id,
        "command": command,
        "reason": reason
    }));
    let path = crate::annunimas_root().join("data/prometheus/escalations.jsonl");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if has_pending_matching_escalation(&path, "policy_guard.denied", command) {
        return;
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(
            file,
            "{}",
            json!({
                "escalation_id": id,
                "ts": ts,
                "task_id": "policy_guard",
                "status": "pending",
                "reason": "policy_guard.denied",
                "confidence": 1.0,
                "severity": "critical",
                "command": command,
                "note": reason
            })
        );
    }
}

fn load_autonomy_runtime_state() -> serde_json::Value {
    let path = std::path::PathBuf::from("core/state/autonomy_runtime.json");
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => {
            return json!({
                "mode": "normal",
                "violations": []
            });
        }
    };
    serde_json::from_str(&content).unwrap_or_else(|_| {
        json!({
            "mode": "normal",
            "violations": []
        })
    })
}

fn load_permission_profile_state() -> serde_json::Value {
    let path = std::path::PathBuf::from("core/state/permission_profiles.json");
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => {
            return json!({
                "active_profile": "unknown",
                "profiles": {}
            });
        }
    };
    serde_json::from_str(&content).unwrap_or_else(|_| {
        json!({
            "active_profile": "unknown",
            "profiles": {}
        })
    })
}

fn permission_profile_denial_reason(
    descriptor: &CommandPolicyDescriptor,
    state: &serde_json::Value,
) -> Option<String> {
    if !(descriptor.is_network || descriptor.is_destructive) {
        return None;
    }
    let profile_id = std::env::var("ANNUNIMAS_PERMISSION_PROFILE")
        .ok()
        .or_else(|| {
            state
                .get("active_profile")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let profiles = state.get("profiles").and_then(|v| v.as_object())?;
    let profile = match profiles.get(&profile_id) {
        Some(v) => v,
        None => {
            return Some(format!(
                "permission profile '{profile_id}' not found for high-impact command"
            ));
        }
    };

    if is_expired_ts(profile.get("expires_at_utc").and_then(|v| v.as_str())) {
        return Some(format!("permission profile '{profile_id}' expired"));
    }

    let allowlist = profile
        .get("command_allowlist")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|v| v.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let signature = descriptor.signature.to_ascii_lowercase();
    let allowed_by_cmd = if allowlist.is_empty() {
        false
    } else {
        allowlist
            .iter()
            .any(|entry| signature.contains(entry) || entry.contains(&signature))
    };
    if !allowed_by_cmd {
        return Some(format!(
            "permission profile '{profile_id}' does not allow command signature {}",
            descriptor.signature
        ));
    }

    let scopes = profile.get("scopes").and_then(|v| v.as_object())?;
    if descriptor.is_network {
        let network = scopes.get("network")?;
        let allowed = network
            .get("allowed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !allowed {
            return Some(format!(
                "permission profile '{profile_id}' denies network scope"
            ));
        }
        if is_expired_ts(network.get("expires_at_utc").and_then(|v| v.as_str())) {
            return Some(format!(
                "permission profile '{profile_id}' network scope expired"
            ));
        }
    }
    if descriptor.is_destructive {
        let destructive = scopes.get("destructive")?;
        let allowed = destructive
            .get("allowed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !allowed {
            return Some(format!(
                "permission profile '{profile_id}' denies destructive scope"
            ));
        }
        if is_expired_ts(destructive.get("expires_at_utc").and_then(|v| v.as_str())) {
            return Some(format!(
                "permission profile '{profile_id}' destructive scope expired"
            ));
        }
    }
    None
}

fn is_expired_ts(ts: Option<&str>) -> bool {
    let Some(ts) = ts else {
        return false;
    };
    let parsed = chrono::DateTime::parse_from_rfc3339(ts)
        .map(|v| v.with_timezone(&Utc))
        .ok();
    match parsed {
        Some(exp) => exp < Utc::now(),
        None => false,
    }
}

fn persist_permission_profile_decision(record: &serde_json::Value) {
    let path = crate::annunimas_root().join("data/warden/permission_profile_audit.jsonl");
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            tracing::warn!(
                error = %err,
                "failed to create permission profile audit parent directory"
            );
            return;
        }
    }
    let mut file = match fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => file,
        Err(err) => {
            tracing::warn!(error = %err, "failed to open permission profile audit log");
            return;
        }
    };
    let line = match serde_json::to_string(record) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to serialize permission profile decision"
            );
            return;
        }
    };
    if let Err(err) = writeln!(file, "{line}") {
        tracing::warn!(error = %err, "failed to append permission profile decision");
    }
}

fn ruleset_state_path() -> std::path::PathBuf {
    std::path::PathBuf::from("core/state/active_ruleset.json")
}

fn system_control_state_path() -> std::path::PathBuf {
    std::path::PathBuf::from("core/state/system_control.json")
}

fn default_ruleset_state() -> serde_json::Value {
    serde_json::json!({
        "active_ruleset": "annunimas_totality",
        "reason": "default fallback",
        "selected_at_utc": Utc::now().to_rfc3339(),
        "expires_at_utc": null,
        "policy": ruleset_policy("annunimas_totality")
    })
}

fn validator_contract(profile: &str) -> serde_json::Value {
    match profile {
        "citadel_business" => serde_json::json!({
            "schema_version": "annunimas.validators.v2",
            "core": {
                "joulework": {
                    "validator": "joulework",
                    "version": "v1.2.0",
                    "mode": "always_on",
                    "required": true,
                    "threshold": 0.55,
                    "scope": "system"
                },
                "love_equation": {
                    "validator": "love_equation",
                    "version": "v1.2.0",
                    "mode": "always_on",
                    "required": true,
                    "threshold": 0.50,
                    "scope": "system"
                },
                "philosopher_triad": {
                    "validator": "philosopher_triad",
                    "version": "v2.0.0",
                    "mode": "consensus_2_of_3",
                    "required": true,
                    "threshold": 0.55,
                    "members": [
                        {"id": "aurelius", "version": "v1.0.0", "role": "ethics_and_sovereignty"},
                        {"id": "bacon", "version": "v1.0.0", "role": "evidence_and_method"},
                        {"id": "sun_tzu", "version": "v1.0.0", "role": "strategy_and_adversarial_review"}
                    ]
                }
            },
            "light": [
                {
                    "validator": "philosopher_light.bacon_lite",
                    "version": "v1.1.0",
                    "scope": "bounded_research_and_tool_selection",
                    "required_for": ["planning", "analysis", "research"]
                },
                {
                    "validator": "philosopher_light.marcus_lite",
                    "version": "v1.0.0",
                    "scope": "local_exec_and_recovery_hygiene",
                    "required_for": ["runtime_recovery", "maintenance", "bounded_exec"]
                },
                {
                    "validator": "philosopher_light.sun_tzu_lite",
                    "version": "v1.0.0",
                    "scope": "routing_and_competitive_positioning",
                    "required_for": ["routing", "provider_selection", "go_to_market"]
                }
            ]
        }),
        _ => serde_json::json!({
            "schema_version": "annunimas.validators.v2",
            "core": {
                "joulework": {
                    "validator": "joulework",
                    "version": "v1.2.0",
                    "mode": "always_on",
                    "required": true,
                    "threshold": 0.45,
                    "scope": "system"
                },
                "love_equation": {
                    "validator": "love_equation",
                    "version": "v1.2.0",
                    "mode": "always_on",
                    "required": true,
                    "threshold": 0.45,
                    "scope": "system"
                },
                "philosopher_triad": {
                    "validator": "philosopher_triad",
                    "version": "v2.0.0",
                    "mode": "consensus_2_of_3",
                    "required": true,
                    "threshold": 0.45,
                    "members": [
                        {"id": "aurelius", "version": "v1.0.0", "role": "ethics_and_sovereignty"},
                        {"id": "bacon", "version": "v1.0.0", "role": "evidence_and_method"},
                        {"id": "sun_tzu", "version": "v1.0.0", "role": "strategy_and_adversarial_review"}
                    ]
                }
            },
            "light": [
                {
                    "validator": "philosopher_light.bacon_lite",
                    "version": "v1.1.0",
                    "scope": "bounded_research_and_tool_selection",
                    "required_for": ["planning", "analysis", "research"]
                },
                {
                    "validator": "philosopher_light.marcus_lite",
                    "version": "v1.0.0",
                    "scope": "local_exec_and_recovery_hygiene",
                    "required_for": ["runtime_recovery", "maintenance", "bounded_exec"]
                },
                {
                    "validator": "philosopher_light.sun_tzu_lite",
                    "version": "v1.0.0",
                    "scope": "routing_and_competitive_positioning",
                    "required_for": ["routing", "provider_selection", "go_to_market"]
                }
            ]
        }),
    }
}

fn human_augmentation_policy(profile: &str) -> serde_json::Value {
    let default_quorum = if profile == "citadel_business" {
        0.67
    } else {
        0.66
    };
    serde_json::json!({
        "mode": "human_augmentation",
        "operating_model": "zero_human_company_with_sovereign_escalation",
        "critical_decision_routing": {
            "autonomous_classes": [
                "bounded_research",
                "local_refactors",
                "safe_exports",
                "routine_maintenance"
            ],
            "triad_quorum_classes": [
                "strategy_change",
                "provider_reroute",
                "pricing_change",
                "customer_commitment",
                "data_retention_change"
            ],
            "human_required_classes": [
                "funds_movement",
                "legal_commitment",
                "human_identity_or_access_change",
                "destructive_delete",
                "fleet_reimage"
            ]
        },
        "consensus": {
            "triad_required": true,
            "triad_mode": "2_of_3",
            "triad_quorum_ratio": default_quorum,
            "sovereign_override_required_for": [
                "funds_movement",
                "legal_commitment",
                "fleet_reimage"
            ]
        }
    })
}

fn ruleset_policy(profile: &str) -> serde_json::Value {
    match profile {
        "citadel_business" => serde_json::json!({
            "profile": "citadel_business",
            "gate_strict": true,
            "autonomy_score_threshold": 0.72,
            "triad_required_pass_rate": 0.55,
            "enable_exec_council": false,
            "signal_thresholds": {
                "joulework_min": 0.55,
                "love_equation_min": 0.50,
                "provider_health_min": 0.55,
                "queue_health_min": 0.55,
                "observation_coverage_min": 0.82
            },
            "validators": validator_contract("citadel_business"),
            "human_augmentation": human_augmentation_policy("citadel_business"),
            "network_unlock_templates": [
                "git push",
                "cargo run -p annunimas-cli -- hermes send"
            ]
        }),
        _ => serde_json::json!({
            "profile": "annunimas_totality",
            "gate_strict": false,
            "autonomy_score_threshold": 0.65,
            "triad_required_pass_rate": 0.45,
            "enable_exec_council": true,
            "signal_thresholds": {
                "joulework_min": 0.45,
                "love_equation_min": 0.45,
                "provider_health_min": 0.40,
                "queue_health_min": 0.40,
                "observation_coverage_min": 0.80
            },
            "validators": validator_contract("annunimas_totality"),
            "human_augmentation": human_augmentation_policy("annunimas_totality"),
            "network_unlock_templates": [
                "git push",
                "cargo run -p annunimas-cli -- hermes send"
            ]
        }),
    }
}

fn normalize_ruleset_profile(profile: &str) -> Option<&'static str> {
    let normalized = profile.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "annunimas_totality" | "annunimas" | "totality" => Some("annunimas_totality"),
        "citadel_business" | "citadel" | "business" => Some("citadel_business"),
        _ => None,
    }
}

pub(crate) fn load_active_ruleset() -> serde_json::Value {
    let path = ruleset_state_path();
    let content = match fs::read_to_string(&path) {
        Ok(v) => v,
        Err(_) => return default_ruleset_state(),
    };
    let mut value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return default_ruleset_state(),
    };
    let profile = value
        .get("active_ruleset")
        .and_then(|v| v.as_str())
        .and_then(normalize_ruleset_profile)
        .unwrap_or("annunimas_totality");
    value["active_ruleset"] = serde_json::json!(profile);
    let default_policy = ruleset_policy(profile);
    if value.get("policy").is_none() {
        value["policy"] = default_policy;
    } else {
        merge_missing_json(&mut value["policy"], &default_policy);
    }
    value
}

fn merge_missing_json(target: &mut serde_json::Value, defaults: &serde_json::Value) {
    if let (Some(target_obj), Some(default_obj)) = (target.as_object_mut(), defaults.as_object()) {
        for (key, default_value) in default_obj {
            match target_obj.get_mut(key) {
                Some(existing) => merge_missing_json(existing, default_value),
                None => {
                    target_obj.insert(key.clone(), default_value.clone());
                }
            }
        }
        return;
    }

    if target.is_null() {
        *target = defaults.clone();
    }
}

pub(crate) fn default_governance_weights(profile: &str) -> serde_json::Value {
    match profile {
        "citadel_business" => serde_json::json!({
            "triad_pass_rate": 0.26,
            "joulework": 0.20,
            "love_equation": 0.12,
            "bacon_lite_confidence": 0.12,
            "retinue_game_theory": 0.14,
            "provider_health": 0.08,
            "queue_health": 0.04,
            "observation_coverage": 0.02,
            "disk_health": 0.02
        }),
        _ => serde_json::json!({
            "triad_pass_rate": 0.22,
            "joulework": 0.22,
            "love_equation": 0.16,
            "bacon_lite_confidence": 0.10,
            "retinue_game_theory": 0.14,
            "provider_health": 0.08,
            "queue_health": 0.04,
            "observation_coverage": 0.02,
            "disk_health": 0.02
        }),
    }
}

pub(crate) fn default_signal_thresholds(profile: &str) -> serde_json::Value {
    ruleset_policy(profile)
        .get("signal_thresholds")
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "joulework_min": 0.45,
                "love_equation_min": 0.45,
                "provider_health_min": 0.40,
                "queue_health_min": 0.40,
                "observation_coverage_min": 0.80
            })
        })
}

fn default_system_control_state() -> serde_json::Value {
    let ruleset = load_active_ruleset();
    let profile = ruleset
        .get("active_ruleset")
        .and_then(|v| v.as_str())
        .and_then(normalize_ruleset_profile)
        .unwrap_or("annunimas_totality");
    serde_json::json!({
        "schema_version": "annunimas.system.control.v1",
        "sigil": "⚡",
        "active_ruleset": profile,
        "updated_at_utc": Utc::now().to_rfc3339(),
        "governance": {
            "always_on": {
                "joulework_required": true,
                "love_equation_influence": true,
                "triad_influence": true,
                "bacon_lite_influence": true
            },
            "weights": default_governance_weights(profile),
            "thresholds": default_signal_thresholds(profile),
            "validators": ruleset
                .get("policy")
                .and_then(|v| v.get("validators"))
                .cloned()
                .unwrap_or_else(|| validator_contract(profile)),
            "human_augmentation": ruleset
                .get("policy")
                .and_then(|v| v.get("human_augmentation"))
                .cloned()
                .unwrap_or_else(|| human_augmentation_policy(profile))
        },
        "providers": {
            "settings_backing_store": "core/state/system_control.json",
            "usage_limits_path": "config/llm_usage_limits.yaml",
            "provider_registry_path": "docs/registry.toml",
            "charon_provider_config_path": "config/charon.providers.toml",
            "env_keys": [
                "OPENROUTER_API_KEY",
                "CEREBRAS_API_KEY",
                "GROQ_API_KEY",
                "GEMINI_API_KEY",
                "OPENAI_API_KEY",
                "ANTHROPIC_API_KEY",
                "LITELLM_API_KEY",
                "LITELLM_PROXY_URL",
                "WARDEN_WEBHOOK_URL"
            ]
        },
        "storage": {
            "hades": {
                "retention_days": {
                    "action_queue": 14,
                    "log": 30,
                    "joulework": 30,
                    "warden_queue": 7
                },
                "max_keep": {
                    "action_queue": 200000,
                    "log": 500000,
                    "joulework": 50000,
                    "warden_queue": 250000
                },
                "backup": {
                    "max_keep_per_store": 2,
                    "retention_days": 3
                }
            },
            "hermes": {
                "retention_days": {
                    "messages": 30,
                    "queue": 14,
                    "decisions": 30
                }
            },
            "mnemosyne": {
                "compact_retention_days": 180
            }
        },
        "package_observation": {
            "registry_path": "docs/registry.toml",
            "workspace_scan_roots": [
                "apps",
                "config",
                "core",
                "crates",
                "docs",
                "human",
                "scripts"
            ],
            "critical_tools": [
                "litellm",
                "oh-my-opencode",
                "llmfit",
                "nanoclaw",
                "crawl4ai"
            ]
        }
    })
}

pub(crate) fn load_system_control_state() -> serde_json::Value {
    read_system_control_state_from_path(&system_control_state_path())
}

fn read_system_control_state_from_path(path: &std::path::Path) -> serde_json::Value {
    let defaults = default_system_control_state();
    let mut value = match fs::read_to_string(path) {
        Ok(content) => {
            serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| defaults.clone())
        }
        Err(_) => defaults.clone(),
    };
    merge_missing_json(&mut value, &defaults);
    let profile = load_active_ruleset()
        .get("active_ruleset")
        .and_then(|v| v.as_str())
        .and_then(normalize_ruleset_profile)
        .unwrap_or("annunimas_totality");
    value["active_ruleset"] = serde_json::json!(profile);
    value
}

fn write_system_control_state() -> anyhow::Result<serde_json::Value> {
    let path = system_control_state_path();
    let mut value = read_system_control_state_from_path(&path);
    value["updated_at_utc"] = serde_json::json!(Utc::now().to_rfc3339());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&value)? + "\n")?;
    Ok(value)
}

pub(crate) fn set_active_ruleset(
    profile: &str,
    reason: &str,
    expires_at_utc: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    let Some(profile) = normalize_ruleset_profile(profile) else {
        anyhow::bail!(
            "invalid ruleset profile '{}'; use annunimas_totality or citadel_business",
            profile
        );
    };
    let state = serde_json::json!({
        "active_ruleset": profile,
        "reason": reason,
        "selected_at_utc": Utc::now().to_rfc3339(),
        "expires_at_utc": expires_at_utc,
        "policy": ruleset_policy(profile)
    });
    let path = ruleset_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&state)? + "\n")?;
    let _ = write_system_control_state()?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_policy_descriptor_flags_high_impact_commands() {
        let hades_remove = Commands::Hades {
            command: HadesCommands::Remove {
                file: "danger.txt".to_string(),
                authorized_by: "operator".to_string(),
                quorum_approvers: vec![],
                quorum_evidence: vec![],
                quorum_asserted_at_utc: None,
            },
        };
        let desc = command_policy_descriptor(&hades_remove);
        assert!(desc.is_destructive);
        assert!(!desc.is_network);

        let hermes_send = Commands::Hermes {
            command: HermesCommands::Send {
                provider: "discord".to_string(),
                channel: "ops".to_string(),
                subject: "subject".to_string(),
                body: "body".to_string(),
                stream: false,
            },
        };
        let desc = command_policy_descriptor(&hermes_send);
        assert!(desc.is_network);
        assert!(!desc.is_destructive);

        let local = Commands::Tools;
        let desc = command_policy_descriptor(&local);
        assert!(!desc.is_network);
        assert!(!desc.is_destructive);
    }

    #[test]
    fn permission_profile_denial_reason_enforces_signature_and_scope_rules() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- hermes send".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        let state = json!({
            "active_profile": "operator",
            "profiles": {
                "operator": {
                    "command_allowlist": ["hermes send"],
                    "scopes": {
                        "network": {
                            "allowed": false
                        }
                    }
                }
            }
        });

        let reason =
            permission_profile_denial_reason(&descriptor, &state).expect("expected denial");
        assert!(reason.contains("denies network scope"));

        let allowed_state = json!({
            "active_profile": "operator",
            "profiles": {
                "operator": {
                    "command_allowlist": ["hermes send"],
                    "scopes": {
                        "network": {
                            "allowed": true,
                            "expires_at_utc": "2099-01-01T00:00:00Z"
                        }
                    }
                }
            }
        });
        assert!(permission_profile_denial_reason(&descriptor, &allowed_state).is_none());
    }

    #[test]
    fn human_augmentation_denial_reason_requires_triad_approval_for_provider_reroute() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- charon route-governor".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "provider_reroute",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        let policy = ruleset_policy("annunimas_totality");
        let reason = human_augmentation_denial_reason(&descriptor, &policy, &json!({}))
            .expect("expected triad denial");
        assert!(reason.contains("2-of-3 philosopher triad"));

        let approval_state = json!({
            "approvals": [
                {
                    "status": "approved",
                    "decision_class": "provider_reroute",
                    "approvers": ["aurelius", "bacon"],
                    "evidence": ["ops-123"],
                    "expires_at_utc": "2099-01-01T00:00:00Z"
                }
            ]
        });
        assert!(human_augmentation_denial_reason(&descriptor, &policy, &approval_state).is_none());
    }

    #[test]
    fn read_only_system_control_load_does_not_create_or_refresh_state_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("annunimas-system-control-readonly-{nanos}"));
        let path = dir.join("system_control.json");
        std::fs::create_dir_all(&dir).expect("temp dir");

        let state = read_system_control_state_from_path(&path);

        assert_eq!(state["schema_version"], "annunimas.system.control.v1");
        assert!(
            !path.exists(),
            "read-only status/observability paths must not create {}",
            path.display()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn human_augmentation_denial_reason_requires_human_approval_for_destructive_delete() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- hades remove".to_string(),
            is_network: false,
            is_destructive: true,
            decision_class: "destructive_delete",
            human_authorizers: vec!["orchestrator".to_string()],
            triad_approvers: vec!["aurelius".to_string(), "bacon".to_string()],
            triad_evidence: vec!["ticket-123".to_string()],
        };
        let policy = ruleset_policy("annunimas_totality");
        let reason = human_augmentation_denial_reason(&descriptor, &policy, &json!({}))
            .expect("expected human approval denial");
        assert!(reason.contains("requires human approval"));

        let approved = CommandPolicyDescriptor {
            human_authorizers: vec!["arandur".to_string()],
            ..descriptor
        };
        assert!(human_augmentation_denial_reason(&approved, &policy, &json!({})).is_none());
    }

    #[test]
    fn governance_policy_mode_defaults_to_record_and_proceed_for_routine_actions() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- local-op".to_string(),
            is_network: false,
            is_destructive: false,
            decision_class: "routine_maintenance",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        let policy = json!({});

        assert_eq!(
            governance_policy_mode_for_descriptor(&descriptor, &policy),
            "record_and_proceed"
        );
        assert!(governance_policy_denial_reason(&descriptor, &policy, &json!({})).is_none());
    }

    #[test]
    fn governance_policy_mode_supports_observe_only_and_block_on_fail_overrides() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- audit readonly".to_string(),
            is_network: false,
            is_destructive: false,
            decision_class: "read_only_audit",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        let policy = json!({
            "governance_policy": {
                "action_class_modes": {
                    "read_only_audit": "observe_only",
                    "provider_reroute": "block_on_fail"
                }
            }
        });

        assert_eq!(
            governance_policy_mode_for_descriptor(&descriptor, &policy),
            "observe_only"
        );
        assert!(governance_policy_denial_reason(&descriptor, &policy, &json!({})).is_none());

        let reroute = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- charon route-governor".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "provider_reroute",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        assert_eq!(
            governance_policy_mode_for_descriptor(&reroute, &policy),
            "block_on_fail"
        );
        let reason = governance_policy_denial_reason(&reroute, &policy, &json!({}))
            .expect("expected missing triad evidence denial");
        assert!(reason.contains("policy mode 'block_on_fail'"));

        let approved_reroute = CommandPolicyDescriptor {
            triad_approvers: vec!["aurelius".to_string(), "bacon".to_string()],
            triad_evidence: vec!["triad-verdict-123".to_string()],
            ..reroute
        };
        assert!(governance_policy_denial_reason(&approved_reroute, &policy, &json!({})).is_none());
    }

    #[test]
    fn governance_policy_mode_escalates_to_human_until_matching_approval_exists() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- charon route-governor".to_string(),
            is_network: true,
            is_destructive: false,
            decision_class: "provider_reroute",
            human_authorizers: vec![],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        let policy = ruleset_policy("annunimas_totality");

        assert_eq!(
            governance_policy_mode_for_descriptor(&descriptor, &policy),
            "escalate_to_human"
        );
        let reason = governance_policy_denial_reason(&descriptor, &policy, &json!({}))
            .expect("expected escalation denial");
        assert!(reason.contains("policy mode 'escalate_to_human'"));

        let approval_state = json!({
            "approvals": [{
                "status": "approved",
                "decision_class": "provider_reroute",
                "command_signature": "cargo run -p annunimas-cli -- charon route-governor",
                "approvers": ["ceo"],
                "evidence": ["operator-approval-123"],
                "expires_at_utc": "2099-01-01T00:00:00Z"
            }]
        });
        assert!(governance_policy_denial_reason(&descriptor, &policy, &approval_state).is_none());
    }

    #[test]
    fn governance_policy_mode_requires_independent_receipts_for_destructive_actions() {
        let descriptor = CommandPolicyDescriptor {
            signature: "cargo run -p annunimas-cli -- hades remove".to_string(),
            is_network: false,
            is_destructive: true,
            decision_class: "destructive_delete",
            human_authorizers: vec!["arandur".to_string()],
            triad_approvers: vec![],
            triad_evidence: vec![],
        };
        let policy = ruleset_policy("annunimas_totality");

        assert_eq!(
            governance_policy_mode_for_descriptor(&descriptor, &policy),
            "require_independent_receipts"
        );
        let reason = governance_policy_denial_reason(&descriptor, &policy, &json!({}))
            .expect("expected receipt denial");
        assert!(reason.contains("independent receipts"));

        let approval_state = json!({
            "approvals": [{
                "status": "approved",
                "decision_class": "destructive_delete",
                "command_signature": "cargo run -p annunimas-cli -- hades remove",
                "approvers": ["arandur"],
                "evidence": ["sovereign-approval-123"],
                "independent_receipts": ["triad-receipt-123", "audit-receipt-456"],
                "expires_at_utc": "2099-01-01T00:00:00Z"
            }]
        });
        assert!(governance_policy_denial_reason(&descriptor, &policy, &approval_state).is_none());
    }
}
