#![cfg(feature = "full-cli")]
// sigil: ∇ ◈ ↝
//
// `arda-cli loop …` — Phase 1 autonomy loop driver.
//
// Today: `seed-goals` materializes config/goals_seed.json into
// core/state/goals/<id>.json contract records. `tick` runs one
// pass of the loop (Planner -> Dispatcher -> Reflector). See
// docs/plans/PHASE_1_PLAN.md.

use std::path::{Path, PathBuf};

use arda_core::contract::{Goal, GoalPriority};
use arda_core::state::{self, StateRoot};
use chrono::Utc;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::arda_root;

/// Build the joule estimator for the dispatcher. Loads tariffs from
/// `<root>/config/joule_tariffs.toml` when present; falls back to
/// the in-code default table otherwise. Logs which path was taken.
fn build_dispatch_estimator() -> arda_economics::EstimatorMeter {
    let path = arda_root().join("config").join("joule_tariffs.toml");
    if path.exists() {
        match arda_economics::EstimatorMeter::load_from_path(&path) {
            Ok(m) => {
                println!("estimator: loaded tariffs from {}", path.display());
                return m;
            }
            Err(e) => {
                eprintln!(
                    "estimator: failed to load {} ({}); using default tariffs",
                    path.display(),
                    e
                );
            }
        }
    }
    arda_economics::EstimatorMeter::with_default_tariffs()
}

/// Load `<root>/config/governance_gates.yaml`. Falls back to the
/// permissive (no-op) gate if missing or malformed.
fn build_governance_gates() -> arda_core::governance_gates::GovernanceGates {
    let path = arda_root()
        .join("config")
        .join("governance_gates.yaml");
    if path.exists() {
        match arda_core::governance_gates::GovernanceGates::load_from_path(&path) {
            Ok(g) => {
                println!("gates:     loaded governance gates from {}", path.display());
                return g;
            }
            Err(e) => {
                eprintln!(
                    "gates:     failed to load {} ({}); using permissive defaults",
                    path.display(),
                    e
                );
            }
        }
    }
    arda_core::governance_gates::GovernanceGates::permissive()
}

#[derive(Subcommand)]
pub(crate) enum LoopCommands {
    /// Materialize config/goals_seed.json into core/state/goals/
    SeedGoals {
        /// Seed file (defaults to `<root>/config/goals_seed.json`)
        #[arg(long)]
        seed: Option<PathBuf>,
        /// State root override (defaults to `<root>/core/state`)
        #[arg(long)]
        state_root: Option<PathBuf>,
        /// Overwrite existing goal records of the same id
        #[arg(long)]
        force: bool,
    },
    /// Run one autonomy loop pass (Planner -> Dispatcher -> Reflector)
    Tick {
        /// State root override (defaults to `<root>/core/state`)
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Show a one-screen summary of loop state (goals, today's plans, last reflection)
    Status {
        /// State root override (defaults to <root>/core/state)
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Halt switch — pause the dispatcher without disabling the timer.
    /// `set` writes core/state/HALT (with optional reason); `clear`
    /// removes it; `status` reports the current state.
    Halt {
        #[command(subcommand)]
        command: HaltCommands,
    },
    /// Warden monitoring and alert management
    Warden {
        #[command(subcommand)]
        command: WardenCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum HaltCommands {
    /// Engage the halt — dispatcher refuses new work next tick.
    Set {
        /// Reason recorded in the HALT file body (operator-readable).
        #[arg(long)]
        reason: Option<String>,
        /// State root override (defaults to <root>/core/state)
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Clear the halt — dispatcher resumes next tick.
    Clear {
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Show whether HALT is engaged and what the recorded reason is.
    Status {
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

/// Show recent warden alerts
#[derive(Subcommand)]
pub(crate) enum WardenCommands {
    /// Show recent warden alerts
    Status {
        /// State root override (defaults to `<root>/core/state`)
        #[arg(long)]
        state_root: Option<PathBuf>,
        /// Number of alerts to show (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Clear all warden alerts
    Clear {
        /// State root override (defaults to <root>/core/state)
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Enforce WARDEN security/monitoring invariants; dry-run by default.
    Enforce {
        /// State root override (defaults to <root>/core/state)
        #[arg(long)]
        state_root: Option<PathBuf>,
        /// Guardhouse projection path override
        #[arg(long)]
        guardhouse: Option<PathBuf>,
        /// Policy authority projection path override
        #[arg(long)]
        policy_authority: Option<PathBuf>,
        /// Permission profile path override
        #[arg(long)]
        permission_profiles: Option<PathBuf>,
        /// Governance runtime projection path override
        #[arg(long)]
        governance_runtime: Option<PathBuf>,
        /// Report output path; JSON contract is always written for evidence.
        #[arg(long, default_value = "core/state/warden_security_enforcement.json")]
        out: PathBuf,
        /// Append-only JSONL audit trail path.
        #[arg(long, default_value = "data/warden/security_enforcement.jsonl")]
        findings: PathBuf,
        /// Apply enforcement actions such as engaging HALT on critical findings.
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
}

pub(crate) fn handle(cmd: LoopCommands) -> anyhow::Result<()> {
    match cmd {
        LoopCommands::SeedGoals {
            seed,
            state_root,
            force,
        } => seed_goals(seed, state_root, force),
        LoopCommands::Tick { state_root } => tick(state_root),
        LoopCommands::Status { state_root } => status(state_root),
        LoopCommands::Halt { command } => handle_halt(command),
        LoopCommands::Warden { command } => handle_warden(command),
    }
}

pub(crate) fn handle_halt(cmd: HaltCommands) -> anyhow::Result<()> {
    match cmd {
        HaltCommands::Set { reason, state_root } => halt_set(reason, state_root),
        HaltCommands::Clear { state_root } => halt_clear(state_root),
        HaltCommands::Status { state_root } => halt_status(state_root),
    }
}

pub(crate) fn handle_warden(cmd: WardenCommands) -> anyhow::Result<()> {
    match cmd {
        WardenCommands::Status { state_root, limit } => warden_status(state_root, limit),
        WardenCommands::Clear { state_root } => warden_clear(state_root),
        WardenCommands::Enforce {
            state_root,
            guardhouse,
            policy_authority,
            permission_profiles,
            governance_runtime,
            out,
            findings,
            apply,
        } => warden_enforce(WardenEnforceArgs {
            state_root_arg: state_root,
            guardhouse_arg: guardhouse,
            policy_authority_arg: policy_authority,
            permission_profiles_arg: permission_profiles,
            governance_runtime_arg: governance_runtime,
            out,
            findings_path: findings,
            apply,
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
struct WardenEnforcementFinding {
    severity: String,
    class: String,
    evidence_path: String,
    reason: String,
    recommended_action: String,
}

struct WardenEnforceArgs {
    state_root_arg: Option<PathBuf>,
    guardhouse_arg: Option<PathBuf>,
    policy_authority_arg: Option<PathBuf>,
    permission_profiles_arg: Option<PathBuf>,
    governance_runtime_arg: Option<PathBuf>,
    out: PathBuf,
    findings_path: PathBuf,
    apply: bool,
}

fn warden_enforce(args: WardenEnforceArgs) -> anyhow::Result<()> {
    let WardenEnforceArgs {
        state_root_arg,
        guardhouse_arg,
        policy_authority_arg,
        permission_profiles_arg,
        governance_runtime_arg,
        out,
        findings_path,
        apply,
    } = args;
    let root = arda_root();
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    let guardhouse_path =
        guardhouse_arg.unwrap_or_else(|| state_root_path.join("warden_guardhouse.json"));
    let policy_authority_path = policy_authority_arg
        .unwrap_or_else(|| state_root_path.join("warden_policy_authority.json"));
    let permission_profiles_path =
        permission_profiles_arg.unwrap_or_else(|| state_root_path.join("permission_profiles.json"));
    let governance_runtime_path =
        governance_runtime_arg.unwrap_or_else(|| state_root_path.join("governance_runtime.json"));

    let mut findings = Vec::new();
    let guardhouse = load_json_required(&guardhouse_path, &mut findings);
    let policy_authority = load_json_required(&policy_authority_path, &mut findings);
    let permission_profiles = load_json_required(&permission_profiles_path, &mut findings);
    let governance_runtime = load_json_required(&governance_runtime_path, &mut findings);

    if let Some(value) = guardhouse.as_ref() {
        inspect_guardhouse(value, &guardhouse_path, &mut findings);
    }
    if let Some(value) = policy_authority.as_ref() {
        inspect_policy_authority(value, &policy_authority_path, &mut findings);
    }
    if let Some(value) = permission_profiles.as_ref() {
        inspect_permission_profiles(value, &permission_profiles_path, &mut findings);
    }
    if let Some(value) = governance_runtime.as_ref() {
        inspect_governance_runtime(value, &governance_runtime_path, &mut findings);
    }

    let critical_total = findings.iter().filter(|f| f.severity == "critical").count();
    let warning_total = findings.iter().filter(|f| f.severity == "warning").count();
    let dry_run = !apply;
    let halt_path = state_root_path.join(arda_core::loop_engine::HALT_FILE_NAME);
    let mut actions_applied: Vec<Value> = Vec::new();
    let mut actions_planned: Vec<Value> = Vec::new();

    if critical_total > 0 {
        let action = json!({
            "action": "engage_halt",
            "target": halt_path,
            "reason": "critical WARDEN security/monitoring enforcement finding present"
        });
        if apply {
            if let Some(parent) = halt_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                &halt_path,
                format!(
                    "HALT engaged at {}\nreason: critical WARDEN security/monitoring enforcement finding present\n",
                    Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
                ),
            )?;
            actions_applied.push(action);
        } else {
            actions_planned.push(action);
        }
    }

    let status = if critical_total > 0 {
        "blocked"
    } else if warning_total > 0 {
        "review"
    } else {
        "pass"
    };
    let report = json!({
        "contract": "arda.warden.security_monitoring_enforcement.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "root": root,
        "state_root": state_root_path,
        "dry_run": dry_run,
        "apply": apply,
        "status": status,
        "summary": {
            "findings_total": findings.len(),
            "critical_total": critical_total,
            "warning_total": warning_total,
            "actions_planned_total": actions_planned.len(),
            "actions_applied_total": actions_applied.len()
        },
        "inputs": {
            "guardhouse": guardhouse_path,
            "policy_authority": policy_authority_path,
            "permission_profiles": permission_profiles_path,
            "governance_runtime": governance_runtime_path
        },
        "findings": findings,
        "actions_planned": actions_planned,
        "actions_applied": actions_applied,
        "safety": {
            "dry_run_first": true,
            "destructive_operations": false,
            "apply_required_for_halt": true
        }
    });

    write_json_pretty(&out, &report)?;
    append_jsonl(&findings_path, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn load_json_required(path: &Path, findings: &mut Vec<WardenEnforcementFinding>) -> Option<Value> {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(value) => Some(value),
            Err(err) => {
                findings.push(WardenEnforcementFinding {
                    severity: "critical".to_string(),
                    class: "malformed_security_state".to_string(),
                    evidence_path: path.display().to_string(),
                    reason: format!("required WARDEN state file is not valid JSON: {err}"),
                    recommended_action: "repair projection before autonomous enforcement"
                        .to_string(),
                });
                None
            }
        },
        Err(err) => {
            findings.push(WardenEnforcementFinding {
                severity: "critical".to_string(),
                class: "missing_security_state".to_string(),
                evidence_path: path.display().to_string(),
                reason: format!("required WARDEN state file could not be read: {err}"),
                recommended_action: "restore projection before autonomous enforcement".to_string(),
            });
            None
        }
    }
}

fn inspect_guardhouse(value: &Value, path: &Path, findings: &mut Vec<WardenEnforcementFinding>) {
    require_array_member(
        value,
        path,
        "duties",
        "drift_watch",
        "missing_monitoring_duty",
        "WARDEN guardhouse must retain drift_watch duty for monitoring enforcement",
        findings,
    );
    require_array_member(
        value,
        path,
        "duties",
        "quarantine_authority",
        "missing_security_duty",
        "WARDEN guardhouse must retain quarantine_authority for security enforcement",
        findings,
    );
    if value
        .get("generated_at_utc")
        .and_then(Value::as_str)
        .is_none()
    {
        findings.push(WardenEnforcementFinding {
            severity: "warning".to_string(),
            class: "missing_projection_timestamp".to_string(),
            evidence_path: path.display().to_string(),
            reason: "warden_guardhouse.json has no generated_at_utc timestamp".to_string(),
            recommended_action: "regenerate guardhouse projection".to_string(),
        });
    }
    if let Some(total) = value
        .pointer("/health/fleet_control/connection_cleanup/stale_offline_total")
        .and_then(Value::as_u64)
    {
        if total > 0 {
            findings.push(WardenEnforcementFinding {
                severity: "warning".to_string(),
                class: "stale_fleet_nodes".to_string(),
                evidence_path: format!(
                    "{}#/health/fleet_control/connection_cleanup",
                    path.display()
                ),
                reason: format!("{total} stale offline fleet node(s) require operator review"),
                recommended_action: "review stale candidates before trusting fleet health"
                    .to_string(),
            });
        }
    }
}

fn inspect_policy_authority(
    value: &Value,
    path: &Path,
    findings: &mut Vec<WardenEnforcementFinding>,
) {
    let policy = value.pointer("/destructive_quorum/policy");
    let enabled = policy
        .and_then(|v| v.get("enabled"))
        .and_then(Value::as_bool);
    if enabled != Some(true) {
        findings.push(WardenEnforcementFinding {
            severity: "critical".to_string(),
            class: "destructive_quorum_disabled".to_string(),
            evidence_path: format!("{}#/destructive_quorum/policy", path.display()),
            reason: "destructive quorum policy is not explicitly enabled".to_string(),
            recommended_action: "enable destructive quorum before allowing high-impact commands"
                .to_string(),
        });
    }
    let required_approvers = policy
        .and_then(|v| v.get("required_approvers"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if required_approvers < 2 {
        findings.push(WardenEnforcementFinding {
            severity: "critical".to_string(),
            class: "destructive_quorum_too_weak".to_string(),
            evidence_path: format!(
                "{}#/destructive_quorum/policy/required_approvers",
                path.display()
            ),
            reason: format!(
                "destructive quorum requires {required_approvers} approver(s); minimum is 2"
            ),
            recommended_action: "restore 2-of-3 triad quorum for destructive actions".to_string(),
        });
    }
}

fn inspect_permission_profiles(
    value: &Value,
    path: &Path,
    findings: &mut Vec<WardenEnforcementFinding>,
) {
    let active = value.get("active_profile").and_then(Value::as_str);
    let profiles = value.get("profiles").and_then(Value::as_object);
    if active.is_none() {
        findings.push(WardenEnforcementFinding {
            severity: "critical".to_string(),
            class: "missing_active_permission_profile".to_string(),
            evidence_path: path.display().to_string(),
            reason: "permission_profiles.json has no active_profile".to_string(),
            recommended_action: "select a bounded permission profile".to_string(),
        });
    }
    if let (Some(active_name), Some(profile_map)) = (active, profiles) {
        if !profile_map.contains_key(active_name) {
            findings.push(WardenEnforcementFinding {
                severity: "critical".to_string(),
                class: "active_permission_profile_missing".to_string(),
                evidence_path: format!("{}#/active_profile", path.display()),
                reason: format!("active profile {active_name} is not defined"),
                recommended_action: "restore active profile or switch to an existing profile"
                    .to_string(),
            });
        }
    }
    if let Some(profile_map) = profiles {
        for (name, profile) in profile_map {
            if let Some(allowlist) = profile.get("command_allowlist").and_then(Value::as_array) {
                for command in allowlist.iter().filter_map(Value::as_str) {
                    if command == "*" || command.contains("rm -rf") {
                        findings.push(WardenEnforcementFinding {
                            severity: "critical".to_string(),
                            class: "overbroad_command_allowlist".to_string(),
                            evidence_path: format!("{}#/profiles/{name}/command_allowlist", path.display()),
                            reason: format!("profile {name} contains overbroad command allowlist entry {command}"),
                            recommended_action: "replace with explicit bounded command signatures".to_string(),
                        });
                    }
                }
            }
            inspect_profile_scope(name, profile, path, "network", findings);
            inspect_profile_scope(name, profile, path, "destructive", findings);
        }
    }
}

fn inspect_profile_scope(
    profile_name: &str,
    profile: &Value,
    path: &Path,
    scope: &str,
    findings: &mut Vec<WardenEnforcementFinding>,
) {
    let scope_value = profile.pointer(&format!("/scopes/{scope}"));
    if let Some(scope_state) = scope_value {
        if scope_state.get("allowed").and_then(Value::as_bool) == Some(true)
            && scope_state
                .get("expires_at_utc")
                .and_then(Value::as_str)
                .is_none()
        {
            let severity = if scope == "destructive" {
                "critical"
            } else {
                "warning"
            };
            findings.push(WardenEnforcementFinding {
                severity: severity.to_string(),
                class: "unbounded_permission_scope".to_string(),
                evidence_path: format!(
                    "{}#/profiles/{profile_name}/scopes/{scope}",
                    path.display()
                ),
                reason: format!(
                    "profile {profile_name} allows {scope} scope without expires_at_utc"
                ),
                recommended_action: "add an expiry or disable the scope".to_string(),
            });
        }
    }
}

fn inspect_governance_runtime(
    value: &Value,
    path: &Path,
    findings: &mut Vec<WardenEnforcementFinding>,
) {
    require_json_array_contains(
        value,
        path,
        "/contracts/active_ruleset/policy/human_augmentation/critical_decision_routing/human_required_classes",
        "destructive_delete",
        "missing_destructive_delete_human_gate",
        "governance runtime must route destructive_delete to human_required",
        findings,
    );
    require_json_array_contains(
        value,
        path,
        "/contracts/active_ruleset/policy/human_augmentation/critical_decision_routing/triad_quorum_classes",
        "provider_reroute",
        "missing_provider_reroute_triad_gate",
        "governance runtime must route provider_reroute to triad quorum",
        findings,
    );
}

fn require_array_member(
    value: &Value,
    path: &Path,
    key: &str,
    expected: &str,
    class: &str,
    reason: &str,
    findings: &mut Vec<WardenEnforcementFinding>,
) {
    let present = value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().any(|item| item.as_str() == Some(expected)))
        .unwrap_or(false);
    if !present {
        findings.push(WardenEnforcementFinding {
            severity: "critical".to_string(),
            class: class.to_string(),
            evidence_path: format!("{}#/{key}", path.display()),
            reason: reason.to_string(),
            recommended_action: "restore WARDEN monitoring/security projection contract"
                .to_string(),
        });
    }
}

fn require_json_array_contains(
    value: &Value,
    path: &Path,
    pointer: &str,
    expected: &str,
    class: &str,
    reason: &str,
    findings: &mut Vec<WardenEnforcementFinding>,
) {
    let present = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| items.iter().any(|item| item.as_str() == Some(expected)))
        .unwrap_or(false);
    if !present {
        findings.push(WardenEnforcementFinding {
            severity: "critical".to_string(),
            class: class.to_string(),
            evidence_path: format!("{}#{pointer}", path.display()),
            reason: reason.to_string(),
            recommended_action: "restore governance runtime gate before autonomous execution"
                .to_string(),
        });
    }
}

fn write_json_pretty(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn append_jsonl(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn warden_status(state_root_arg: Option<PathBuf>, limit: usize) -> anyhow::Result<()> {
    let root = arda_root();
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    let state = StateRoot::new(state_root_path);
    let alerts = arda_core::loop_alerts::read_recent(&state, limit)?;

    if alerts.is_empty() {
        println!("No warden alerts.");
    } else {
        println!("Recent warden alerts (newest first):");
        for alert in alerts.iter().rev() {
            println!(
                "  {} {:?} subject={}",
                alert.observed_at.format("%Y-%m-%d %H:%M:%SZ"),
                alert.kind,
                alert.subject
            );
            println!("    {}", alert.message);
        }
    }

    Ok(())
}

fn warden_clear(state_root_arg: Option<PathBuf>) -> anyhow::Result<()> {
    let root = arda_root();
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    let warden_dir = state_root_path.join("alerts");

    if !warden_dir.exists() {
        println!("No warden alerts to clear.");
        return Ok(());
    }

    let entries = std::fs::read_dir(&warden_dir)?;
    let mut count = 0;
    for entry in entries {
        let path = entry?.path();
        if path.is_file() {
            std::fs::remove_file(&path)?;
            count += 1;
        }
    }

    println!("Cleared {} warden alert(s).", count);
    Ok(())
}

fn halt_path(state_root_arg: Option<PathBuf>) -> PathBuf {
    let root = arda_root();
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    state_root_path.join(arda_core::loop_engine::HALT_FILE_NAME)
}

fn halt_set(reason: Option<String>, state_root_arg: Option<PathBuf>) -> anyhow::Result<()> {
    let path = halt_path(state_root_arg);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = match reason.as_deref() {
        Some(r) => format!(
            "HALT engaged at {}\nreason: {}\n",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
            r
        ),
        None => format!(
            "HALT engaged at {}\nreason: (none)\n",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        ),
    };
    std::fs::write(&path, body)?;
    println!("HALT engaged at {}", path.display());
    if let Some(r) = reason {
        println!("reason: {r}");
    }
    println!("Dispatcher will refuse new work on the next tick.");
    Ok(())
}

fn halt_clear(state_root_arg: Option<PathBuf>) -> anyhow::Result<()> {
    let path = halt_path(state_root_arg);
    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("HALT cleared ({} removed).", path.display());
        println!("Dispatcher resumes next tick.");
    } else {
        println!("HALT was not engaged ({} did not exist).", path.display());
    }
    Ok(())
}

fn halt_status(state_root_arg: Option<PathBuf>) -> anyhow::Result<()> {
    let path = halt_path(state_root_arg);
    if path.exists() {
        println!("HALT: ENGAGED ({})", path.display());
        match std::fs::read_to_string(&path) {
            Ok(body) if !body.trim().is_empty() => {
                println!("---");
                print!("{body}");
                if !body.ends_with('\n') {
                    println!();
                }
            }
            _ => {}
        }
    } else {
        println!("HALT: clear ({} not present)", path.display());
    }
    Ok(())
}

fn status(state_root_arg: Option<PathBuf>) -> anyhow::Result<()> {
    use arda_core::contract::{GoalStatus, MemoryKind};
    use arda_core::state;

    let root = arda_root();
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    let state = StateRoot::new(state_root_path);
    let queue_path = state.queue_path(&root);

    println!("== loop status ==");
    println!("state_root: {}", state.root().display());
    println!("queue:      {}", queue_path.display());

    // Halt
    let halt = state
        .root()
        .join(arda_core::loop_engine::HALT_FILE_NAME);
    if halt.exists() {
        println!("HALT:       PRESENT — dispatcher will refuse new work");
        if let Ok(body) = std::fs::read_to_string(&halt) {
            for line in body.lines().take(3) {
                if !line.trim().is_empty() {
                    println!("            {line}");
                }
            }
        }
    } else {
        println!("HALT:       absent");
    }

    // Goals
    let goals = state::list_goals(&state).map_err(|e| anyhow::anyhow!("list goals: {}", e))?;
    let active = goals
        .iter()
        .filter(|g| g.status == GoalStatus::Active)
        .count();
    println!(
        "goals:      {} total, {} active, {} other",
        goals.len(),
        active,
        goals.len() - active,
    );
    for g in &goals {
        println!(
            "  - {:>9} {} ({})",
            format!("{:?}", g.status),
            g.id,
            g.title
        );
    }

    // Plans (today)
    let today_prefix = "plan_";
    let today_suffix = format!("_{}", Utc::now().format("%Y%m%d"));
    let plans = state::list_plans(&state).map_err(|e| anyhow::anyhow!("list plans: {}", e))?;
    let today_plans: Vec<_> = plans
        .iter()
        .filter(|p| p.id.starts_with(today_prefix) && p.id.ends_with(&today_suffix))
        .collect();
    println!(
        "plans:      {} total, {} for today ({})",
        plans.len(),
        today_plans.len(),
        Utc::now().format("%Y-%m-%d")
    );

    // Reflections — show most recent 5
    let mut reflections =
        state::list_reflections(&state).map_err(|e| anyhow::anyhow!("list reflections: {}", e))?;
    reflections.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
    println!("reflections: {} total, last 5:", reflections.len());
    for r in reflections.iter().take(5) {
        println!(
            "  - {} task={} plan={} score={:.2} {:?}",
            r.completed_at.format("%Y-%m-%d %H:%M:%SZ"),
            r.task_id,
            r.plan_id,
            r.score,
            r.outcome
        );
    }

    // Memory (contract)
    let ep = state::list_memory(&state, MemoryKind::Episodic)
        .map(|v| v.len())
        .unwrap_or(0);
    let se = state::list_memory(&state, MemoryKind::Semantic)
        .map(|v| v.len())
        .unwrap_or(0);
    println!(
        "memory:     {} episodic, {} semantic (contract records)",
        ep, se
    );

    let economy = loop_status_economy_snapshot(&state)
        .map_err(|e| anyhow::anyhow!("loop economy snapshot: {}", e))?;
    println!(
        "economy:    {:.2} J today across {} decisions",
        economy.total_joules_today, economy.decisions_today
    );
    println!(
        "            {:.2} J/min last 60s, {} bid decisions today",
        economy.joules_per_minute_last_60s, economy.bid_count_today
    );
    if economy.joules_by_agent.is_empty() {
        println!("            joules by agent: none yet");
    } else {
        for (agent, joules) in &economy.joules_by_agent {
            println!("            agent {agent}: {joules:.2} J");
        }
    }
    match &economy.latest_bid_spread {
        Some(spread) => println!(
            "            latest bid spread: low={:.2} J high={:.2} J spread={:.2} J bidders={}",
            spread.low_joules, spread.high_joules, spread.spread_joules, spread.bidders
        ),
        None => println!("            latest bid spread: none yet"),
    }
    println!(
        "            snapshot: {}",
        arda_core::loop_economy::snapshot_path(&state).display()
    );

    Ok(())
}

fn loop_status_economy_snapshot(
    state: &StateRoot,
) -> std::io::Result<arda_core::loop_economy::LoopEconomySnapshot> {
    arda_core::loop_economy::build_snapshot(state)
}

fn format_triad_summary(
    passes: usize,
    conditionals: usize,
    vetoes: usize,
    blocked: usize,
    unconsulted: usize,
) -> String {
    format!(
        "triad:     {passes} pass, {conditionals} conditional, {vetoes} VETO recorded, {blocked} blocked by policy, {unconsulted} unconsulted"
    )
}

fn tick(state_root_arg: Option<PathBuf>) -> anyhow::Result<()> {
    let root = arda_root();
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    let state = StateRoot::new(state_root_path);

    println!("loop tick: state_root={}", state.root().display());

    // Phase 1 step 3+4 — planner emits Plans and Tasks.
    let queue_path = state.queue_path(&root);
    let plan_pass = crate::prometheus::planner::run(&state, Some(&queue_path))
        .map_err(|e| anyhow::anyhow!("planner: {}", e))?;
    println!(
        "planner:   {} goals considered, {} plans written, {} skipped (already exist), {} inactive, {} without recipe, {} tasks emitted -> {}",
        plan_pass.goals_considered,
        plan_pass.plans_written.len(),
        plan_pass.plans_skipped_existing.len(),
        plan_pass.goals_inactive.len(),
        plan_pass.goals_without_recipe.len(),
        plan_pass.tasks_emitted,
        queue_path.display(),
    );
    for id in &plan_pass.plans_written {
        println!("  + {}", id);
    }
    for id in &plan_pass.goals_without_recipe {
        eprintln!("  ! goal has no planner recipe: {}", id);
    }

    // Phase 1 step 5 — dispatcher. Phase 2 step 4: real joule
    // estimates via the plutus EstimatorMeter. Tariffs come from
    // config/joule_tariffs.toml when present, default table otherwise.
    let estimator = build_dispatch_estimator();
    let triad = arda_governance::LiveTriad::new();
    let bid_board = arda_core::loop_engine::StaticBidBoard;
    let gates = build_governance_gates();
    let dispatch_pass = arda_core::loop_engine::dispatch_full(
        &state,
        &queue_path,
        arda_core::loop_engine::DEFAULT_DISPATCH_CAP_PER_TICK,
        &estimator,
        &triad,
        &bid_board,
        &gates,
    )
    .map_err(|e| anyhow::anyhow!("dispatcher: {}", e))?;
    if dispatch_pass.halted {
        println!("dispatch:  HALTED (core/state/HALT present); refused to dispatch new work");
    } else {
        println!(
            "dispatch:  {} tasks seen, {} dispatched, {} no-route, {} already terminal, {} budget-blocked{}",
            dispatch_pass.tasks_seen,
            dispatch_pass.dispatched.len(),
            dispatch_pass.no_route.len(),
            dispatch_pass.already_terminal.len(),
            dispatch_pass.budget_blocked.len(),
            match dispatch_pass.capped_at {
                Some(c) => format!(" (CAPPED at {c}/tick)"),
                None => String::new(),
            },
        );
        for entry in &dispatch_pass.budget_blocked {
            eprintln!("  ! budget-blocked: {}", entry);
        }
        println!(
            "{}",
            format_triad_summary(
                dispatch_pass.triad_passes,
                dispatch_pass.triad_conditionals,
                dispatch_pass.triad_vetoes.len(),
                dispatch_pass.triad_blocked.len(),
                dispatch_pass.triad_unconsulted.len(),
            )
        );
        println!(
            "market:    {} bids ledgered, {} market collapses",
            dispatch_pass.bids_recorded,
            dispatch_pass.market_collapses.len(),
        );
        if dispatch_pass.councils_held > 0 {
            println!(
                "council:   {} deliberation(s) ledgered, {:.2} J charged to goal budgets",
                dispatch_pass.councils_held, dispatch_pass.council_joules_charged,
            );
        }
    }
    for entry in &dispatch_pass.no_route {
        eprintln!("  ! no-route: {}", entry);
    }

    // Phase 1 step 6 — reflector.
    let reflect_pass = arda_core::loop_engine::reflect(&state, &queue_path)
        .map_err(|e| anyhow::anyhow!("reflector: {}", e))?;
    println!(
        "reflect:   {} terminal tasks seen, {} reflections written, {} already reflected, {} no plan link",
        reflect_pass.tasks_seen,
        reflect_pass.reflections_written.len(),
        reflect_pass.already_reflected.len(),
        reflect_pass.no_plan_link.len(),
    );

    let reflections =
        state::list_reflections(&state).map_err(|e| anyhow::anyhow!("list reflections: {}", e))?;
    let mut alerts = arda_core::loop_alerts::analyze_tick(&dispatch_pass, &reflections);
    alerts.extend(
        arda_core::loop_alerts::analyze_chaos_log(&state)
            .map_err(|e| anyhow::anyhow!("chaos alerts: {}", e))?,
    );
    let alerts_written = arda_core::loop_alerts::append_alerts(&state, &alerts)
        .map_err(|e| anyhow::anyhow!("warden alerts: {}", e))?;
    println!("warden:   {} alert(s) written", alerts_written);

    let economy = arda_core::loop_economy::write_snapshot(&state)
        .map_err(|e| anyhow::anyhow!("loop economy snapshot: {}", e))?;
    println!(
        "economy:  {:.2} J today, {:.2} J/min last 60s, {} bid decisions -> {}",
        economy.total_joules_today,
        economy.joules_per_minute_last_60s,
        economy.bid_count_today,
        arda_core::loop_economy::snapshot_path(&state).display()
    );

    Ok(())
}

#[derive(Debug, Deserialize)]
struct SeedDoc {
    goals: Vec<SeedGoal>,
}

#[derive(Debug, Deserialize)]
struct SeedGoal {
    id: String,
    title: String,
    intent: String,
    owner_agent: String,
    priority: GoalPriority,
    #[serde(default)]
    joule_budget_per_day: Option<f64>,
}

fn seed_goals(
    seed_arg: Option<PathBuf>,
    state_root_arg: Option<PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    let root = arda_root();
    let seed_path = seed_arg.unwrap_or_else(|| root.join("config/goals_seed.json"));
    let state_root_path = state_root_arg.unwrap_or_else(|| root.join("core/state"));
    let state = StateRoot::new(state_root_path);

    let bytes = std::fs::read(&seed_path)
        .map_err(|e| anyhow::anyhow!("read {}: {}", seed_path.display(), e))?;
    let doc: SeedDoc = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("parse {}: {}", seed_path.display(), e))?;

    let existing: std::collections::HashSet<String> = state::list_goals(&state)
        .map_err(|e| anyhow::anyhow!("list existing goals: {}", e))?
        .into_iter()
        .map(|g| g.id)
        .collect();

    let mut written = 0usize;
    let mut skipped = 0usize;
    let now = Utc::now();

    for sg in doc.goals {
        if existing.contains(&sg.id) && !force {
            println!(
                "skip:    {} (already present; pass --force to overwrite)",
                sg.id
            );
            skipped += 1;
            continue;
        }
        let mut goal = Goal::new(&sg.id, &sg.title, &sg.intent, &sg.owner_agent, sg.priority);
        goal.joule_budget_per_day = sg.joule_budget_per_day;
        // If overwriting, keep created_at honest by reading the prior record.
        if existing.contains(&sg.id) {
            if let Ok(prior) = state::read_goal(&state, &sg.id) {
                goal.created_at = prior.created_at;
            }
            goal.updated_at = now;
        }
        let path = state::write_goal(&state, &goal)
            .map_err(|e| anyhow::anyhow!("write {}: {}", sg.id, e))?;
        println!("wrote:   {} -> {}", sg.id, path.display());
        written += 1;
    }

    println!("seed-goals: {written} written, {skipped} skipped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn triad_summary_discloses_recorded_vetoes_policy_blocks_and_unconsulted() {
        assert_eq!(
            format_triad_summary(2, 1, 3, 4, 5),
            "triad:     2 pass, 1 conditional, 3 VETO recorded, 4 blocked by policy, 5 unconsulted"
        );
    }

    #[test]
    fn loop_status_economy_snapshot_does_not_write_projection_file() {
        let root = unique_temp_dir("arda-loop-status-economy");
        let state = StateRoot::new(&root);
        let snapshot_path = arda_core::loop_economy::snapshot_path(&state);

        let snapshot = loop_status_economy_snapshot(&state).expect("economy snapshot");

        assert_eq!(snapshot.decisions_today, 0);
        assert!(
            !snapshot_path.exists(),
            "loop status should read economy state without refreshing {}",
            snapshot_path.display()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn warden_enforce_dry_run_reports_critical_without_writing_halt() {
        let root = unique_temp_dir("arda-warden-enforce");
        let state_root = root.join("core/state");
        let data_root = root.join("data/warden");
        std::fs::create_dir_all(&state_root).expect("state root");
        std::fs::create_dir_all(&data_root).expect("data root");

        let guardhouse = state_root.join("warden_guardhouse.json");
        let policy_authority = state_root.join("warden_policy_authority.json");
        let permission_profiles = state_root.join("permission_profiles.json");
        let governance_runtime = state_root.join("governance_runtime.json");
        let out = state_root.join("warden_security_enforcement.json");
        let findings = data_root.join("security_enforcement.jsonl");

        std::fs::write(
            &guardhouse,
            serde_json::json!({
                "generated_at_utc": "2099-01-01T00:00:00Z",
                "duties": ["drift_watch", "quarantine_authority"]
            })
            .to_string(),
        )
        .expect("guardhouse");
        std::fs::write(
            &policy_authority,
            serde_json::json!({
                "destructive_quorum": {
                    "policy": {"enabled": true, "required_approvers": 2}
                }
            })
            .to_string(),
        )
        .expect("policy authority");
        std::fs::write(
            &permission_profiles,
            serde_json::json!({
                "active_profile": "operator",
                "profiles": {
                    "operator": {
                        "command_allowlist": ["*"],
                        "scopes": {
                            "network": {"allowed": true, "expires_at_utc": "2099-01-01T00:00:00Z"},
                            "destructive": {"allowed": false}
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("permission profiles");
        std::fs::write(
            &governance_runtime,
            serde_json::json!({
                "contracts": {
                    "active_ruleset": {
                        "policy": {
                            "human_augmentation": {
                                "critical_decision_routing": {
                                    "human_required_classes": ["destructive_delete"],
                                    "triad_quorum_classes": ["provider_reroute"]
                                }
                            }
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("governance runtime");

        warden_enforce(WardenEnforceArgs {
            state_root_arg: Some(state_root.clone()),
            guardhouse_arg: Some(guardhouse),
            policy_authority_arg: Some(policy_authority),
            permission_profiles_arg: Some(permission_profiles),
            governance_runtime_arg: Some(governance_runtime),
            out: out.clone(),
            findings_path: findings.clone(),
            apply: false,
        })
        .expect("warden enforce");

        let report: Value =
            serde_json::from_str(&std::fs::read_to_string(&out).expect("read enforcement report"))
                .expect("parse enforcement report");
        assert_eq!(
            report.get("contract").and_then(Value::as_str),
            Some("arda.warden.security_monitoring_enforcement.v1")
        );
        assert_eq!(report.get("dry_run").and_then(Value::as_bool), Some(true));
        assert_eq!(
            report.get("status").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(
            report
                .pointer("/summary/critical_total")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            report
                .pointer("/summary/actions_planned_total")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert!(!state_root
            .join(arda_core::loop_engine::HALT_FILE_NAME)
            .exists());
        assert!(std::fs::read_to_string(findings)
            .expect("findings jsonl")
            .contains("overbroad_command_allowlist"));
        let _ = std::fs::remove_dir_all(root);
    }
}
