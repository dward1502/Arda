#![cfg(feature = "full-cli")]
// sigil: REPAIR
mod arda;
mod chronos;
mod fleet;
mod governance_runtime;
mod hermes_command;
mod human_context;
mod io;
mod memory;
mod operations_flow;
mod operator_actions;
mod package_enablement;
mod paperclip;
mod snapshot;
mod soterion;
mod storage_pressure;
mod support;
mod topology;
mod warden;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use self::io::{
    collect_file_entries_recursive, collect_file_paths, collect_json_file_summaries_recursive,
    collect_markdown_file_summaries, collect_markdown_file_summaries_recursive,
    count_files_with_extension, directory_size_bytes, merge_fleet_nodes, read_edge_targets,
    read_fleet_config_meta, read_fleet_config_nodes, read_json_file, read_toml_as_json,
    read_yaml_as_json, rel_path, summarize_markdown_file,
};
use self::support::{
    count_bool_field, escalation_dedupe_key, is_expired_rfc3339, latest_events_by_key,
    latest_jsonl_entries_by_id, latest_jsonl_entries_by_source_id, latest_task_rows_by_id,
    read_all_jsonl, read_env_assignment, read_recent_jsonl, read_recent_mnemosyne_episodic,
    summarize_description, summarize_env_file, summarize_field_count_value, summarize_field_counts,
};

const CORE_STATE_SCHEMA_VERSION: &str = "annunimas.core.state.v1";

#[derive(Debug, Clone)]
pub struct CoreAutonomyProfile {
    pub heartbeat_ms: u64,
    pub triad_bypass: bool,
    pub base_costs: HashMap<String, f64>,
    pub world_status: Option<String>,
    pub world_resonance: Option<f64>,
    pub source_root: PathBuf,
}

impl CoreAutonomyProfile {
    pub fn load(core_root: impl AsRef<Path>) -> Option<Self> {
        Self::try_load(core_root).ok()
    }

    pub fn try_load(core_root: impl AsRef<Path>) -> Result<Self, crate::error::PrometheusError> {
        let core_root = core_root.as_ref().to_path_buf();
        let boot_path = core_root.join("realm").join("boot.toml");
        let boot = try_read_boot(&boot_path)?;
        let identity = read_identity(&core_root.join("realm").join("annunimas.toml"));
        let agents = read_agents(&core_root.join("realm").join("agents.toml"));
        let world = reconcile_world_state(&core_root, &boot, identity.as_ref(), agents.as_ref());
        write_system_manifest(
            &core_root,
            &boot,
            identity.as_ref(),
            agents.as_ref(),
            world.as_ref(),
        );
        fleet::write_fleet_runtime_projection(&core_root);
        fleet::write_fleet_nodes_projection(&core_root);
        fleet::write_fleet_models_projection(&core_root);
        fleet::write_fleet_hardware_projection(&core_root);
        fleet::write_fleet_health_projection(&core_root);
        fleet::write_fleet_backbone_projection(&core_root);
        warden::write_warden_guardhouse(&core_root);
        warden::write_warden_policy_authority(&core_root);
        warden::write_warden_edge_contract(&core_root);
        warden::write_warden_nightly_doctrine(&core_root);
        topology::write_runtime_topology_projection(&core_root);
        topology::write_charon_router_projection(&core_root);
        operations_flow::write_hades_lifecycle_projection(&core_root);
        hermes_command::write_hermes_command_projection(&core_root);
        memory::write_mnemosyne_continuity_projection(&core_root);
        memory::write_memory_identity_projection(&core_root);
        memory::write_memory_activity_projection(&core_root);
        memory::write_memory_scopes_projection(&core_root);
        arda::write_athena_runtime_projection(&core_root);
        arda::write_apollo_runtime_projection(&core_root);
        arda::write_plutus_runtime_projection(&core_root);
        arda::write_oracle_runtime_projection(&core_root);
        chronos::write_chronos_status_projection(&core_root);
        human_context::write_business_runtime_projection(&core_root);
        human_context::write_personal_runtime_projection(&core_root);
        human_context::write_human_context_projection(&core_root);
        operations_flow::write_queue_summary_projection(&core_root);
        arda::write_repo_reorganization_projection(&core_root);
        topology::write_output_topology_projection(&core_root);
        governance_runtime::write_system_control_projection(&core_root);
        package_enablement::write_package_health_projection(&core_root);
        package_enablement::write_package_enablement_projection(&core_root);
        governance_runtime::write_runtime_settings_projection(&core_root);
        governance_runtime::write_control_plane_lockdown_projection(&core_root);
        storage_pressure::write_storage_pressure_projection(&core_root);
        governance_runtime::write_governance_runtime_projection(&core_root);
        operations_flow::write_operations_flow_projection(&core_root);
        paperclip::write_paperclip_alignment_projection(&core_root);
        operations_flow::write_escalation_runtime_projection(&core_root);
        operator_actions::write_operator_actions_projection(&core_root);
        soterion::write_soterion_render_projection(&core_root);
        snapshot::write_arda_snapshot(&core_root);
        arda::write_arda_source_map(&core_root);

        Ok(Self {
            heartbeat_ms: boot.ceo.heartbeat_ms.unwrap_or(500),
            triad_bypass: boot.ceo.triad_bypass.unwrap_or(false),
            base_costs: boot
                .joulework
                .and_then(|j| j.base_costs)
                .unwrap_or_default(),
            world_status: world.as_ref().map(|w| w.system.status.clone()),
            world_resonance: world.map(|w| w.metrics.system_resonance),
            source_root: core_root,
        })
    }

    pub fn base_cost_for(&self, task_type: &str) -> Option<f64> {
        self.base_costs
            .get(&task_type.to_ascii_lowercase())
            .copied()
            .or_else(|| self.base_costs.get(task_type).copied())
    }
}

#[derive(Debug, Deserialize)]
struct BootConfig {
    system: Option<BootSystemConfig>,
    ceo: BootCeoConfig,
    startup: Option<BootStartupConfig>,
    joulework: Option<BootJouleWorkConfig>,
}

#[derive(Debug, Deserialize)]
struct BootSystemConfig {
    name: Option<String>,
    version: Option<String>,
    sigil: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootCeoConfig {
    agent_id: Option<String>,
    heartbeat_ms: Option<u64>,
    triad_bypass: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BootStartupConfig {
    sequence: Option<Vec<String>>,
    on_failure: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootJouleWorkConfig {
    base_costs: Option<HashMap<String, f64>>,
}

#[derive(Debug, Deserialize)]
struct WorldState {
    system: WorldSystem,
    metrics: WorldMetrics,
}

#[derive(Debug, Deserialize)]
struct WorldSystem {
    status: String,
}

#[derive(Debug, Deserialize)]
struct WorldMetrics {
    system_resonance: f64,
}

#[derive(Debug, Deserialize)]
struct RealmIdentityConfig {
    identity: IdentityConfig,
    realms: RealmsConfig,
}

#[derive(Debug, Deserialize)]
struct IdentityConfig {
    name: String,
    sigil: String,
}

#[derive(Debug, Deserialize)]
struct RealmsConfig {
    definition: Vec<RealmDefinition>,
}

#[derive(Debug, Deserialize)]
struct RealmDefinition {
    id: String,
    color: String,
}

#[derive(Debug, Deserialize)]
struct AgentRosterFile {
    agent: Vec<AgentDefinition>,
}

#[derive(Debug, Deserialize)]
struct AgentDefinition {
    id: String,
    sigil: String,
    name: String,
    title: Option<String>,
    realm: String,
    clearance: String,
    description: Option<String>,
    soterion: Option<AgentSoterion>,
}

#[derive(Debug, Deserialize)]
struct AgentSoterion {
    resonance: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct EdgeTargetsFile {
    node: Option<Vec<EdgeTargetNode>>,
}

#[derive(Debug, Deserialize)]
struct EdgeTargetNode {
    id: Option<String>,
    role: Option<String>,
    hostname: Option<String>,
    tailscale_ip: Option<String>,
    ssh_user: Option<String>,
    athena_enabled: Option<bool>,
    hermes_enabled: Option<bool>,
    warden_enabled: Option<bool>,
    charon_enabled: Option<bool>,
    oracle_enabled: Option<bool>,
    plutus_enabled: Option<bool>,
    node_class: Option<String>,
    enrollment_status: Option<String>,
    llm_runtime: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FleetConfigFile {
    fleet: Option<FleetConfigSection>,
    exports: Option<FleetExportsSection>,
    nodes: Option<Vec<FleetNodeConfig>>,
}

#[derive(Debug, Deserialize)]
struct FleetConfigSection {
    enabled: Option<bool>,
    status_view_mode: Option<String>,
    stale_offline_threshold_days: Option<u64>,
    include_recent_offline_in_status: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct FleetExportsSection {
    prometheus_dir: Option<String>,
    ceo_layer_prometheus_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FleetNodeConfig {
    id: Option<String>,
    role: Option<String>,
    hostname: Option<String>,
    display_name: Option<String>,
    tailscale_ip: Option<String>,
    tailscale_name: Option<String>,
    ssh_user: Option<String>,
    node_class: Option<String>,
    enrollment_status: Option<String>,
    llm_runtime: Option<String>,
    charon_provider_id: Option<String>,
    base_url: Option<String>,
    health_url: Option<String>,
    models_url: Option<String>,
    expected_models: Option<Vec<String>>,
    startup_priority: Option<u64>,
    restart_scope: Option<String>,
    restart_cmd: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootNightlyConfig {
    enabled: Option<bool>,
    run_at: Option<String>,
    archive_complete_after_days: Option<u64>,
    prune_low_resonance: Option<bool>,
    min_resonance_threshold: Option<f64>,
    compact_ledger: Option<bool>,
    emit_daily_summary: Option<bool>,
    summary_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootNightlyFile {
    nightly: Option<BootNightlyConfig>,
}

fn try_read_boot(path: &Path) -> Result<BootConfig, crate::error::PrometheusError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            crate::error::PrometheusError::BootConfigMissing {
                path: path.to_path_buf(),
            }
        } else {
            crate::error::PrometheusError::Io(e)
        }
    })?;
    toml::from_str::<BootConfig>(&content).map_err(|source| {
        crate::error::PrometheusError::BootConfigInvalid {
            path: path.to_path_buf(),
            source,
        }
    })
}

fn reconcile_world_state(
    core_root: &Path,
    boot: &BootConfig,
    identity: Option<&RealmIdentityConfig>,
    agents: Option<&AgentRosterFile>,
) -> Option<WorldState> {
    let world_path = core_root.join("state").join("world.json");
    let existing_value = fs::read_to_string(&world_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok());

    if let Some(existing) = existing_value.as_ref() {
        if !needs_world_reconciliation(existing) {
            return serde_json::from_value(existing.clone()).ok();
        }
    }

    let identity = identity?;
    let agents = agents?;

    let rebuilt = build_world_state(core_root, boot, identity, agents, existing_value.as_ref());
    if let Some(parent) = world_path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    let payload = serde_json::to_string_pretty(&rebuilt).ok()? + "\n";
    fs::write(&world_path, payload).ok()?;
    serde_json::from_value(rebuilt).ok()
}

fn needs_world_reconciliation(value: &Value) -> bool {
    let system_status = value
        .get("system")
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let agents = value
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if agents.is_empty() {
        return true;
    }
    let arandur_online = agents.iter().any(|agent| {
        agent.get("id").and_then(Value::as_str) == Some("arandur")
            && agent
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status.eq_ignore_ascii_case("online"))
    });
    let all_heartbeats_empty = agents.iter().all(|agent| {
        agent.get("last_heartbeat").is_none()
            || agent.get("last_heartbeat").is_some_and(Value::is_null)
    });
    system_status.is_empty()
        || system_status == "INITIALIZING"
        || !arandur_online
        || all_heartbeats_empty
}

fn build_world_state(
    core_root: &Path,
    boot: &BootConfig,
    identity: &RealmIdentityConfig,
    agents: &AgentRosterFile,
    existing: Option<&Value>,
) -> Value {
    let now = Utc::now().to_rfc3339();
    let system_name = boot
        .system
        .as_ref()
        .and_then(|s| s.name.as_deref())
        .unwrap_or(identity.identity.name.as_str());
    let system_version = boot
        .system
        .as_ref()
        .and_then(|s| s.version.as_deref())
        .unwrap_or("0.1.0");
    let system_sigil = boot
        .system
        .as_ref()
        .and_then(|s| s.sigil.as_deref())
        .unwrap_or(identity.identity.sigil.as_str());
    let ceo_id = boot.ceo.agent_id.as_deref().unwrap_or("arandur");

    let existing_agents = existing
        .and_then(|v| v.get("agents"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let existing_by_id = existing_agents
        .into_iter()
        .filter_map(|agent| {
            let id = agent.get("id").and_then(Value::as_str)?.to_string();
            Some((id, agent))
        })
        .collect::<HashMap<_, _>>();
    let realm_colors = identity
        .realms
        .definition
        .iter()
        .map(|realm| (realm.id.as_str(), realm.color.as_str()))
        .collect::<HashMap<_, _>>();

    let merged_agents = agents
        .agent
        .iter()
        .map(|agent| {
            let existing_agent = existing_by_id.get(&agent.id);
            let default_status = if agent.id == ceo_id {
                "ONLINE"
            } else {
                "OFFLINE"
            };
            let default_heartbeat = if agent.id == ceo_id {
                Some(now.clone())
            } else {
                None
            };
            json!({
                "id": agent.id,
                "name": agent.name,
                "sigil": agent.sigil,
                "realm": agent.realm,
                "clearance": agent.clearance,
                "status": existing_agent
                    .and_then(|v| v.get("status"))
                    .and_then(Value::as_str)
                    .filter(|_| agent.id != ceo_id)
                    .unwrap_or(default_status),
                "health": existing_agent
                    .and_then(|v| v.get("health"))
                    .and_then(Value::as_f64)
                    .unwrap_or(if agent.id == ceo_id { 1.0 } else { 0.0 }),
                "resonance": existing_agent
                    .and_then(|v| v.get("resonance"))
                    .and_then(Value::as_f64)
                    .unwrap_or(agent.soterion.as_ref().and_then(|s| s.resonance).unwrap_or(0.0)),
                "phi_harmonic": existing_agent
                    .and_then(|v| v.get("phi_harmonic"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.618),
                "trust_score": existing_agent
                    .and_then(|v| v.get("trust_score"))
                    .and_then(Value::as_f64)
                    .unwrap_or(if agent.id == ceo_id { 1.0 } else { 0.9 }),
                "active_tasks": existing_agent
                    .and_then(|v| v.get("active_tasks"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                "joule_cost_session": existing_agent
                    .and_then(|v| v.get("joule_cost_session"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                "last_heartbeat": existing_agent
                    .and_then(|v| v.get("last_heartbeat"))
                    .cloned()
                    .unwrap_or_else(|| default_heartbeat.map(Value::String).unwrap_or(Value::Null)),
                "color": existing_agent
                    .and_then(|v| v.get("color"))
                    .and_then(Value::as_str)
                    .unwrap_or(realm_colors.get(agent.realm.as_str()).copied().unwrap_or("#ffffff"))
            })
        })
        .collect::<Vec<_>>();

    let active_tasks = merged_agents
        .iter()
        .map(|agent| {
            agent
                .get("active_tasks")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        })
        .sum::<u64>();
    let system_resonance = merged_agents
        .iter()
        .map(|agent| {
            agent
                .get("resonance")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        })
        .sum::<f64>()
        / merged_agents.len().max(1) as f64;

    json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "system": {
            "name": system_name,
            "version": system_version,
            "sigil": system_sigil,
            "status": "READY",
            "boot_timestamp": existing
                .and_then(|v| v.get("system"))
                .and_then(|v| v.get("boot_timestamp"))
                .and_then(Value::as_str)
                .filter(|ts| !ts.is_empty() && !ts.starts_with("2026-02-28T00:00:00"))
                .unwrap_or(now.as_str()),
            "uptime_seconds": existing
                .and_then(|v| v.get("system"))
                .and_then(|v| v.get("uptime_seconds"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        },
        "metrics": {
            "system_resonance": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("system_resonance"))
                .and_then(Value::as_f64)
                .unwrap_or(system_resonance),
            "phi_alignment": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("phi_alignment"))
                .and_then(Value::as_f64)
                .unwrap_or(0.618),
            "love_equation": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("love_equation"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            "active_tasks": active_tasks,
            "completed_tasks": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("completed_tasks"))
                .and_then(Value::as_u64)
                .unwrap_or(0),
            "triad_gates_evaluated": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("triad_gates_evaluated"))
                .and_then(Value::as_u64)
                .unwrap_or(0),
            "joule_cost_total": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("joule_cost_total"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            "joule_cost_session": existing
                .and_then(|v| v.get("metrics"))
                .and_then(|v| v.get("joule_cost_session"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        },
        "agents": merged_agents,
        "flows": existing.and_then(|v| v.get("flows")).cloned().unwrap_or_else(|| json!([])),
        "recent_decisions": existing
            .and_then(|v| v.get("recent_decisions"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        "phi_spiral": existing
            .and_then(|v| v.get("phi_spiral"))
            .cloned()
            .unwrap_or_else(|| json!({
                "current_alignment": 0.618,
                "target": 0.618,
                "convergence_rate": 0.0
            })),
        "source": {
            "core_root": core_root.display().to_string(),
            "reconciled_at_utc": now,
            "authority": "core_realm"
        }
    })
}

fn read_identity(path: &Path) -> Option<RealmIdentityConfig> {
    let content = fs::read_to_string(path).ok()?;
    toml::from_str::<RealmIdentityConfig>(&content).ok()
}

fn read_agents(path: &Path) -> Option<AgentRosterFile> {
    let content = fs::read_to_string(path).ok()?;
    toml::from_str::<AgentRosterFile>(&content).ok()
}

fn write_system_manifest(
    core_root: &Path,
    boot: &BootConfig,
    identity: Option<&RealmIdentityConfig>,
    agents: Option<&AgentRosterFile>,
    world: Option<&WorldState>,
) {
    let (Some(identity), Some(agents)) = (identity, agents) else {
        return;
    };
    let manifest_path = core_root.join("state").join("system_manifest.json");
    if let Some(parent) = manifest_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let now = Utc::now().to_rfc3339();
    let ceo_id = boot.ceo.agent_id.as_deref().unwrap_or("arandur");
    let active_ruleset = read_json_file(core_root.join("state").join("active_ruleset.json"))
        .unwrap_or_else(|| json!({"active_ruleset":"annunimas_totality"}));
    let autonomy_runtime = read_json_file(core_root.join("state").join("autonomy_runtime.json"))
        .unwrap_or_else(
            || json!({"mode":"normal","source":"system_manifest_default","violations":[]}),
        );
    let permission_profiles =
        read_json_file(core_root.join("state").join("permission_profiles.json"))
            .unwrap_or_else(|| json!({"active_profile":"unknown"}));
    let destructive_quorum =
        read_json_file(core_root.join("state").join("destructive_quorum.json"))
            .unwrap_or_else(|| json!({"enabled": true}));
    let interrupt_authority =
        read_json_file(core_root.join("state").join("interrupt_authority.json"))
            .unwrap_or_else(|| json!({"default":{"allow":[]}}));

    let systems = agents
        .agent
        .iter()
        .map(|agent| {
            json!({
                "id": agent.id,
                "name": agent.name,
                "title": agent.title.as_deref().unwrap_or("Agent"),
                "sigil": agent.sigil,
                "realm": agent.realm,
                "clearance": agent.clearance,
                "is_ceo": agent.id == ceo_id,
                "summary": summarize_description(agent.description.as_deref()),
                "role": if agent.id == ceo_id { "sovereign_orchestrator" } else { "domain_agent" }
            })
        })
        .collect::<Vec<_>>();

    let realms = identity
        .realms
        .definition
        .iter()
        .map(|realm| {
            json!({
                "id": realm.id,
                "color": realm.color
            })
        })
        .collect::<Vec<_>>();

    let manifest = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": now,
        "authority": "core_realm",
        "system": {
            "name": boot.system.as_ref().and_then(|s| s.name.as_deref()).unwrap_or(identity.identity.name.as_str()),
            "version": boot.system.as_ref().and_then(|s| s.version.as_deref()).unwrap_or("0.1.0"),
            "sigil": boot.system.as_ref().and_then(|s| s.sigil.as_deref()).unwrap_or(identity.identity.sigil.as_str()),
            "world_status": world.map(|w| w.system.status.as_str()).unwrap_or("UNKNOWN"),
            "world_resonance": world.map(|w| w.metrics.system_resonance).unwrap_or(0.0)
        },
        "ceo": {
            "id": ceo_id,
            "triad_bypass": boot.ceo.triad_bypass.unwrap_or(false),
            "heartbeat_ms": boot.ceo.heartbeat_ms.unwrap_or(500)
        },
        "startup": {
            "sequence": boot.startup.as_ref().and_then(|s| s.sequence.clone()).unwrap_or_default(),
            "on_failure": boot.startup.as_ref().and_then(|s| s.on_failure.clone()).unwrap_or_else(|| "halt_and_log".to_string())
        },
        "governance": {
            "active_ruleset": active_ruleset,
            "autonomy_runtime": autonomy_runtime,
            "permission_profile": permission_profiles,
            "destructive_quorum": destructive_quorum,
            "interrupt_authority": interrupt_authority
        },
        "realms": realms,
        "systems": systems
    });
    let _ = fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn storage_root_entry(
    workspace_root: &Path,
    relative: &str,
    classification: &str,
    purpose: &str,
) -> Value {
    let path = workspace_root.join(relative);
    json!({
        "path": relative,
        "classification": classification,
        "purpose": purpose,
        "exists": path.exists(),
        "bytes": directory_size_bytes(&path),
    })
}

#[cfg(test)]
mod tests {
    use super::operations_flow::write_queue_summary_projection;
    use super::{CoreAutonomyProfile, CORE_STATE_SCHEMA_VERSION};
    use serde_json::{json, Value};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn reconciles_bootstrap_world_state_from_core_realm() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("realm")).expect("realm mkdir");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");

        fs::write(
            core_root.join("realm/boot.toml"),
            r#"
[system]
name = "Annunimas"
version = "0.1.0"
sigil = "𓀀"

[ceo]
agent_id = "arandur"
heartbeat_ms = 500
triad_bypass = true

[nightly]
enabled = true
run_at = "03:00"
archive_complete_after_days = 7
prune_low_resonance = true
min_resonance_threshold = 25.0
compact_ledger = true
emit_daily_summary = true
summary_path = "data/summaries/"

[joulework.base_costs]
ingest = 2.0
"#,
        )
        .expect("boot write");

        fs::write(
            core_root.join("realm/annunimas.toml"),
            r##"
[identity]
name = "Annunimas"
sigil = "𓀀"

[[realms.definition]]
id = "command"
color = "#ffd700"

[[realms.definition]]
id = "knowledge"
color = "#00e5ff"
"##,
        )
        .expect("identity write");

        fs::write(
            core_root.join("realm/agents.toml"),
            r#"
[[agent]]
id = "arandur"
sigil = "𓀀"
name = "Arandur"
realm = "command"
clearance = "sovereign"
[agent.soterion]
resonance = 1.0

[[agent]]
id = "athena"
sigil = "𓁿"
name = "Athena"
realm = "knowledge"
clearance = "guardian"
[agent.soterion]
resonance = 0.95
"#,
        )
        .expect("agents write");

        fs::write(
            core_root.join("state/world.json"),
            r#"{
  "system": {"status":"INITIALIZING","boot_timestamp":"2026-02-28T00:00:00Z","uptime_seconds":0},
  "metrics": {"system_resonance":0.0},
  "agents": [
    {"id":"arandur","status":"OFFLINE","last_heartbeat":null},
    {"id":"athena","status":"OFFLINE","last_heartbeat":null}
  ]
}"#,
        )
        .expect("world write");

        let profile = CoreAutonomyProfile::load(&core_root).expect("profile");
        assert_eq!(profile.world_status.as_deref(), Some("READY"));

        let world: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/world.json")).expect("world read"),
        )
        .expect("world parse");
        assert_eq!(world["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(world["system"]["status"], "READY");
        assert_eq!(world["agents"][0]["id"], "arandur");
        assert_eq!(world["agents"][0]["status"], "ONLINE");
        assert_eq!(world["agents"][1]["id"], "athena");
        assert_eq!(world["agents"][1]["status"], "OFFLINE");

        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/system_manifest.json"))
                .expect("manifest read"),
        )
        .expect("manifest parse");
        assert_eq!(manifest["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(manifest["ceo"]["id"], "arandur");
        assert_eq!(manifest["system"]["world_status"], "READY");
        assert_eq!(manifest["systems"][0]["id"], "arandur");

        let guardhouse: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_guardhouse.json"))
                .expect("guardhouse read"),
        )
        .expect("guardhouse parse");
        assert_eq!(guardhouse["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(guardhouse["authority"], "warden_system_projection");
        assert_eq!(guardhouse["system_model"], "distributed_guardhouse");
        assert_eq!(guardhouse["queue"]["recent_event_count"], 0);

        let policy: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_policy_authority.json"))
                .expect("policy read"),
        )
        .expect("policy parse");
        assert_eq!(policy["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(policy["authority"], "warden_policy_projection");
        assert_eq!(policy["permission_profile"]["active_profile"], "unknown");

        let edge: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_edge_contract.json"))
                .expect("edge read"),
        )
        .expect("edge parse");
        assert_eq!(edge["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(edge["authority"], "warden_edge_projection");
        assert_eq!(edge["edge_contract"]["ack_mode"], "queue_only_no_edge_ack");

        let doctrine: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_nightly_doctrine.json"))
                .expect("doctrine read"),
        )
        .expect("doctrine parse");
        assert_eq!(doctrine["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(doctrine["authority"], "warden_nightly_projection");
        assert_eq!(doctrine["declared_doctrine"]["run_at"], "03:00");

        let charon: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/charon_router.json")).expect("charon read"),
        )
        .expect("charon parse");
        assert_eq!(charon["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(charon["authority"], "charon_router_projection");

        let hades: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/hades_lifecycle.json")).expect("hades read"),
        )
        .expect("hades parse");
        assert_eq!(hades["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(hades["authority"], "hades_lifecycle_projection");

        let hermes: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/hermes_command.json")).expect("hermes read"),
        )
        .expect("hermes parse");
        assert_eq!(hermes["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(hermes["authority"], "hermes_command_projection");

        let mnemosyne: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/mnemosyne_continuity.json"))
                .expect("mnemosyne read"),
        )
        .expect("mnemosyne parse");
        assert_eq!(mnemosyne["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(mnemosyne["authority"], "mnemosyne_continuity_projection");

        let memory_identity: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/memory_identity.json"))
                .expect("memory identity read"),
        )
        .expect("memory identity parse");
        assert_eq!(memory_identity["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(memory_identity["authority"], "memory_identity_projection");

        let memory_activity: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/memory_activity.json"))
                .expect("memory activity read"),
        )
        .expect("memory activity parse");
        assert_eq!(memory_activity["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(memory_activity["authority"], "memory_activity_projection");

        let memory_scopes: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/memory_scopes.json"))
                .expect("memory scopes read"),
        )
        .expect("memory scopes parse");
        assert_eq!(memory_scopes["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(memory_scopes["authority"], "memory_scopes_projection");

        let athena: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/athena_runtime.json")).expect("athena read"),
        )
        .expect("athena parse");
        assert_eq!(athena["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(athena["authority"], "athena_runtime_projection");

        let apollo: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/apollo_runtime.json")).expect("apollo read"),
        )
        .expect("apollo parse");
        assert_eq!(apollo["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(apollo["authority"], "apollo_runtime_projection");

        let plutus: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/plutus_runtime.json")).expect("plutus read"),
        )
        .expect("plutus parse");
        assert_eq!(plutus["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(plutus["authority"], "plutus_runtime_projection");

        let oracle: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/oracle_runtime.json")).expect("oracle read"),
        )
        .expect("oracle parse");
        assert_eq!(oracle["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(oracle["authority"], "oracle_runtime_projection");

        let human: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/human_context.json")).expect("human read"),
        )
        .expect("human parse");
        assert_eq!(human["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(human["authority"], "human_context_projection");

        let queue: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/queue_summary.json")).expect("queue read"),
        )
        .expect("queue parse");
        assert_eq!(queue["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(queue["authority"], "queue_summary_projection");

        let repo: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/repo_reorganization.json"))
                .expect("repo read"),
        )
        .expect("repo parse");
        assert_eq!(repo["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(repo["authority"], "repo_reorganization_projection");
        assert_eq!(repo["status"], "completed");

        let output_topology: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/output_topology.json"))
                .expect("output topology read"),
        )
        .expect("output topology parse");
        assert_eq!(output_topology["authority"], "output_topology_projection");
        assert_eq!(
            output_topology["long_term_accounting_candidates"][0]["recommended_action"],
            "mirror_tree_compact"
        );

        let system_control: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/system_control.json"))
                .expect("system control read"),
        )
        .expect("system control parse");
        assert_eq!(system_control["authority"], "system_control_projection");

        let package_health: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/package_health.json"))
                .expect("package health read"),
        )
        .expect("package health parse");
        assert_eq!(package_health["authority"], "package_observation_export");

        let package_enablement: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/package_enablement.json"))
                .expect("package enablement read"),
        )
        .expect("package enablement parse");
        assert_eq!(
            package_enablement["authority"],
            "package_enablement_projection"
        );

        let runtime_settings: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/runtime_settings.json"))
                .expect("runtime settings read"),
        )
        .expect("runtime settings parse");
        assert_eq!(runtime_settings["authority"], "runtime_settings_projection");

        let control_plane_lockdown: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/control_plane_lockdown.json"))
                .expect("control plane lockdown read"),
        )
        .expect("control plane lockdown parse");
        assert_eq!(
            control_plane_lockdown["authority"],
            "control_plane_lockdown_projection"
        );

        let governance_runtime: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/governance_runtime.json"))
                .expect("governance runtime read"),
        )
        .expect("governance runtime parse");
        assert_eq!(
            governance_runtime["authority"],
            "governance_runtime_projection"
        );

        let operations_flow: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/operations_flow.json"))
                .expect("operations flow read"),
        )
        .expect("operations flow parse");
        assert_eq!(operations_flow["authority"], "operations_flow_projection");

        let paperclip_alignment: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/paperclip_alignment.json"))
                .expect("paperclip alignment read"),
        )
        .expect("paperclip alignment parse");
        assert_eq!(
            paperclip_alignment["authority"],
            "paperclip_alignment_projection"
        );

        let escalation_runtime: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/escalation_runtime.json"))
                .expect("escalation runtime read"),
        )
        .expect("escalation runtime parse");
        assert_eq!(
            escalation_runtime["authority"],
            "escalation_runtime_projection"
        );

        fs::write(
            core_root.join("state/output_accounting.json"),
            serde_json::to_string_pretty(&json!({
                "authority": "output_accounting_projection",
                "status": "completed"
            }))
            .expect("output accounting json"),
        )
        .expect("write output accounting fixture");
        fs::write(
            core_root.join("state/extension_surface_contract.json"),
            serde_json::to_string_pretty(&json!({
                "authority": "framework_digestion_materialization",
                "status": "completed"
            }))
            .expect("extension surface json"),
        )
        .expect("write extension surface fixture");
        fs::write(
            core_root.join("state/network_native_node_onboarding_contract.json"),
            serde_json::to_string_pretty(&json!({
                "authority": "network_native_node_onboarding_contract_export",
                "status": "completed"
            }))
            .expect("network onboarding contract json"),
        )
        .expect("write network onboarding contract fixture");
        fs::write(
            core_root.join("state/aipkg_marketplace_separation_contract.json"),
            serde_json::to_string_pretty(&json!({
                "authority": "aipkg_marketplace_separation_export",
                "status": "completed"
            }))
            .expect("aipkg marketplace json"),
        )
        .expect("write aipkg marketplace fixture");
        let _ = CoreAutonomyProfile::load(&core_root).expect("profile refresh");

        let storage_pressure: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/storage_pressure.json"))
                .expect("storage pressure read"),
        )
        .expect("storage pressure parse");
        assert_eq!(storage_pressure["authority"], "storage_pressure_projection");
        assert!(storage_pressure["workspace_roots"].is_array());
        assert!(storage_pressure["summary"]["total_observed_workspace_bytes"].is_number());
        assert!(storage_pressure["status"]["disk_alert_active"].is_boolean());

        let arda_snapshot: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/arda_snapshot.json"))
                .expect("arda snapshot read"),
        )
        .expect("arda snapshot parse");
        let system_snapshot: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/system_snapshot.json"))
                .expect("system snapshot read"),
        )
        .expect("system snapshot parse");
        assert_eq!(arda_snapshot["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(arda_snapshot["authority"], "arda_snapshot_projection");
        assert_eq!(system_snapshot, arda_snapshot);
        assert_eq!(arda_snapshot["primary_entrypoint"], true);
        assert_eq!(arda_snapshot["status"]["athena_projection_ready"], true);
        assert_eq!(arda_snapshot["status"]["queue_projection_ready"], true);
        assert_eq!(arda_snapshot["status"]["package_observation_ready"], true);
        assert_eq!(arda_snapshot["status"]["package_enablement_ready"], true);
        assert_eq!(arda_snapshot["status"]["runtime_settings_ready"], true);
        assert_eq!(arda_snapshot["status"]["output_topology_ready"], true);
        assert_eq!(arda_snapshot["status"]["output_accounting_ready"], true);
        assert_eq!(
            arda_snapshot["status"]["extension_surface_contract_ready"],
            true
        );
        assert_eq!(
            arda_snapshot["status"]["aipkg_marketplace_separation_contract_ready"],
            true
        );
        assert_eq!(
            arda_snapshot["status"]["network_native_node_onboarding_contract_ready"],
            true
        );
        assert_eq!(arda_snapshot["status"]["governance_runtime_ready"], true);
        assert_eq!(arda_snapshot["status"]["operations_flow_ready"], true);
        assert_eq!(arda_snapshot["status"]["paperclip_alignment_ready"], true);
        assert_eq!(arda_snapshot["status"]["escalation_runtime_ready"], true);
        assert!(arda_snapshot["sections"]["control_plane_lockdown"].is_object());
        assert_eq!(
            arda_snapshot["sections"]["output_accounting"]["authority"],
            "output_accounting_projection"
        );
        assert_eq!(
            arda_snapshot["sections"]["paperclip_alignment"]["authority"],
            "paperclip_alignment_projection"
        );
        assert_eq!(
            arda_snapshot["sections"]["escalation_runtime"]["authority"],
            "escalation_runtime_projection"
        );

        let arda_map: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/arda_source_map.json"))
                .expect("arda map read"),
        )
        .expect("arda map parse");
        let system_map: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/system_source_map.json"))
                .expect("system source map read"),
        )
        .expect("system source map parse");
        assert_eq!(arda_map["schema_version"], CORE_STATE_SCHEMA_VERSION);
        assert_eq!(arda_map["authority"], "arda_source_map_projection");
        assert_eq!(system_map, arda_map);
        assert_eq!(
            arda_map["arda_primary_entrypoint_recommended"],
            "core/state/arda_snapshot.json"
        );
        assert_eq!(arda_map["arda_primary_entrypoint_status"], "implemented");
        assert_eq!(
            arda_map["system_primary_entrypoint_recommended"],
            "core/state/system_snapshot.json"
        );
        assert_eq!(arda_map["system_primary_entrypoint_status"], "implemented");
        assert!(arda_map["sections"]
            .as_array()
            .is_some_and(|sections| !sections.is_empty()));
        assert!(arda_map["sections"].as_array().is_some_and(|sections| {
            sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("routing_and_comms")
                    && section["supplemental_sources"]
                        .as_array()
                        .is_some_and(|sources| {
                            sources.iter().any(|source| {
                                source.as_str()
                                    == Some("data/prometheus/arda_presence_events.jsonl")
                            })
                        })
            })
        }));
        assert!(arda_map["sections"].as_array().is_some_and(|sections| {
            sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("output_accounting")
            })
        }));
        assert!(arda_map["sections"].as_array().is_some_and(|sections| {
            sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("framework_extensions")
            })
        }));
        assert!(arda_map["sections"].as_array().is_some_and(|sections| {
            sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("package_marketplace_doctrine")
            })
        }));
        assert!(arda_map["sections"].as_array().is_some_and(|sections| {
            sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("network_native_onboarding")
            })
        }));
        assert!(arda_map["sections"]
            .as_array()
            .is_some_and(|sections| sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("planning_and_queue")
                    && section["primary_sources"]
                        .as_array()
                        .is_some_and(|sources| {
                            sources.iter().any(|source| {
                                source.as_str() == Some("core/state/escalation_runtime.json")
                            })
                        })
            })));
        assert!(arda_map["sections"].as_array().is_some_and(|sections| {
            sections.iter().any(|section| {
                section.get("id").and_then(Value::as_str) == Some("paperclip_alignment")
                    && section["primary_sources"]
                        .as_array()
                        .is_some_and(|sources| {
                            sources.iter().any(|source| {
                                source.as_str() == Some("core/state/paperclip_alignment.json")
                            })
                        })
            })
        }));
    }

    #[test]
    fn control_plane_lockdown_projection_tracks_runtime_contracts() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        let config_root = dir.path().join("config");
        let data_root = dir.path().join("data");
        fs::create_dir_all(core_root.join("realm")).expect("realm mkdir");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::create_dir_all(core_root.join("metrics")).expect("metrics mkdir");
        fs::create_dir_all(&config_root).expect("config mkdir");
        fs::create_dir_all(data_root.join("prometheus").join("control_plane"))
            .expect("policy dir mkdir");

        fs::write(
            core_root.join("realm/boot.toml"),
            "[ceo]\nagent_id = \"arandur\"\nheartbeat_ms = 500\ntriad_bypass = false\n",
        )
        .expect("boot write");
        fs::write(
            core_root.join("realm/annunimas.toml"),
            "[identity]\nname = \"Annunimas\"\nsigil = \"A\"\n[[realms.definition]]\nid = \"command\"\ncolor = \"#fff\"\n",
        )
        .expect("identity write");
        fs::write(
            core_root.join("realm/agents.toml"),
            "[[agent]]\nid = \"arandur\"\nsigil = \"A\"\nname = \"Arandur\"\nrealm = \"command\"\nclearance = \"sovereign\"\n",
        )
        .expect("agents write");
        fs::write(
            core_root.join("state/autonomy_runtime.json"),
            "{ \"mode\": \"constrained\", \"auto_degraded\": false }\n",
        )
        .expect("autonomy write");
        fs::write(
            core_root.join("state/destructive_quorum.json"),
            "{ \"enabled\": true, \"required_approvers\": 2 }\n",
        )
        .expect("quorum write");
        fs::write(
            core_root.join("state/permission_profiles.json"),
            "{ \"active_profile\": \"ceo_operator\", \"profiles\": { \"ceo_operator\": { \"subject\": \"ceo\" } } }\n",
        )
        .expect("profiles write");
        fs::write(
            data_root
                .join("prometheus")
                .join("control_plane")
                .join("policy_snapshot.json"),
            "{ \"authority\": \"control_plane_policy_export\", \"derived_defaults\": { \"execution_lane\": \"planning\" } }\n",
        )
        .expect("policy write");

        let sock_a = dir.path().join("run").join("athena.sock");
        let sock_b = dir.path().join("run").join("charon.sock");
        fs::create_dir_all(sock_a.parent().expect("sock parent")).expect("run mkdir");
        fs::write(&sock_a, "").expect("sock a write");
        fs::write(&sock_b, "").expect("sock b write");
        fs::write(
            config_root.join("runtime.generated.env"),
            format!(
                "ANNUNIMAS_AUTONOMY_REQUIRED_LIVE_SOCKETS={}:{}\n",
                sock_a.display(),
                sock_b.display()
            ),
        )
        .expect("runtime env write");

        let _ = CoreAutonomyProfile::load(&core_root).expect("profile");

        let lockdown: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/control_plane_lockdown.json"))
                .expect("lockdown read"),
        )
        .expect("lockdown parse");
        assert_eq!(lockdown["authority"], "control_plane_lockdown_projection");
        assert_eq!(lockdown["status"]["control_plane_policy_present"], true);
        assert_eq!(lockdown["status"]["runtime_socket_contract_present"], true);
        assert_eq!(lockdown["status"]["required_sockets_live"], true);
        assert_eq!(lockdown["status"]["lockdown_ready"], true);

        let arda_snapshot: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/arda_snapshot.json"))
                .expect("arda snapshot read"),
        )
        .expect("arda snapshot parse");
        assert_eq!(
            arda_snapshot["status"]["control_plane_lockdown_ready"],
            true
        );

        let arda_map: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/arda_source_map.json"))
                .expect("arda map read"),
        )
        .expect("arda map parse");
        assert!(arda_map["sections"]
            .as_array()
            .is_some_and(
                |sections| sections.iter().any(|section| section["primary_sources"]
                    .as_array()
                    .is_some_and(|sources| sources
                        .iter()
                        .any(|source| source.as_str()
                            == Some("core/state/control_plane_lockdown.json"))))
            ));
    }

    #[test]
    fn governance_and_operations_projections_surface_live_runtime_contracts() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        let metrics_root = core_root.join("metrics").join("by_crate");
        fs::create_dir_all(core_root.join("realm")).expect("realm mkdir");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::create_dir_all(metrics_root.join("governance")).expect("governance mkdir");
        fs::create_dir_all(metrics_root.join("prometheus")).expect("prometheus mkdir");

        fs::write(
            core_root.join("realm/boot.toml"),
            "[system]\nname = \"Annunimas\"\nversion = \"0.1.0\"\nsigil = \"A\"\n[ceo]\nagent_id = \"arandur\"\nheartbeat_ms = 500\ntriad_bypass = false\n[joulework.base_costs]\nanalyze = 5.0\ncommunicate = 3.0\n",
        )
        .expect("boot write");
        fs::write(
            core_root.join("realm/annunimas.toml"),
            "[identity]\nname = \"Annunimas\"\nsigil = \"A\"\n[[realms.definition]]\nid = \"command\"\ncolor = \"#fff\"\n",
        )
        .expect("identity write");
        fs::write(
            core_root.join("realm/agents.toml"),
            "[[agent]]\nid = \"arandur\"\nsigil = \"A\"\nname = \"Arandur\"\nrealm = \"command\"\nclearance = \"sovereign\"\n",
        )
        .expect("agents write");
        fs::write(
            core_root.join("state/active_ruleset.json"),
            "{ \"active_ruleset\": \"annunimas_totality\" }\n",
        )
        .expect("ruleset write");
        fs::write(
            core_root.join("state/autonomy_runtime.json"),
            "{ \"mode\": \"degraded\", \"auto_degraded\": true }\n",
        )
        .expect("autonomy write");
        fs::write(
            core_root.join("state/control_plane_lockdown.json"),
            "{ \"status\": { \"lockdown_ready\": true } }\n",
        )
        .expect("lockdown write");
        fs::write(
            core_root.join("state/queue_summary.json"),
            "{ \"authority\": \"queue_summary_projection\", \"project_tasks\": { \"counts_by_status\": { \"queued\": 3 } } }\n",
        )
        .expect("queue summary write");
        fs::write(
            metrics_root.join("governance").join("signals.json"),
            r#"{
  "control": {
    "thresholds": {
      "joulework_min": 0.45,
      "love_equation_min": 0.45,
      "provider_health_min": 0.4,
      "queue_health_min": 0.4
    }
  },
  "goal": {
    "autonomy_ready": false,
    "autonomy_threshold": 0.65,
    "attention_required": []
  },
  "signals": {
    "autonomy_observation_score": 0.61,
    "avg_joulework": 0.52,
    "avg_love_eq": 0.44,
    "bacon_lite_recent_confidence": 0.57,
    "provider_health": 1.0,
    "queue_health": 0.9,
    "triad_pass_rate": 1.0
  }
}"#,
        )
        .expect("signals write");
        fs::write(
            metrics_root.join("prometheus").join("ops_dashboard.json"),
            r#"{
  "prometheus": {
    "pending_escalations": 9,
    "resource_state": "stable"
  },
  "queue_observability": {
    "summary": {
      "projects_queue_queued": 3,
      "total_known_work_items": 11
    }
  },
  "athena": { "deep_queue_depth": 0 },
  "hermes": { "queue_depth": 2 },
  "hades": { "pending_actions": 1 },
  "charon": { "status": { "providers_ready": 3 } },
  "mnemosyne": { "status": { "ok": true } },
  "apollo": { "executor": { "summary": { "completed_total": 1 } } },
  "plutus": { "runtime_ready": true }
}"#,
        )
        .expect("ops write");

        let _ = CoreAutonomyProfile::load(&core_root).expect("profile");

        let governance: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/governance_runtime.json"))
                .expect("governance read"),
        )
        .expect("governance parse");
        assert_eq!(governance["authority"], "governance_runtime_projection");
        assert_eq!(governance["derived"]["thresholds_met"]["autonomy"], false);
        assert_eq!(
            governance["derived"]["attention_required"]
                .as_array()
                .map(|items| items.len()),
            Some(2)
        );

        let operations: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/operations_flow.json"))
                .expect("operations read"),
        )
        .expect("operations parse");
        assert_eq!(operations["authority"], "operations_flow_projection");
        assert_eq!(
            operations["derived"]["queue_posture"]["projects_queue_queued"],
            3
        );
        assert_eq!(
            operations["derived"]["queue_posture"]["pending_escalations"],
            9
        );
        assert_eq!(operations["derived"]["control_plane_ready"], false);
        assert_eq!(operations["derived"]["autonomy_ready"], false);

        let arda_snapshot: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/arda_snapshot.json"))
                .expect("arda snapshot read"),
        )
        .expect("arda snapshot parse");
        assert_eq!(arda_snapshot["status"]["governance_runtime_ready"], true);
        assert_eq!(arda_snapshot["status"]["operations_flow_ready"], true);

        let arda_map: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/arda_source_map.json"))
                .expect("arda map read"),
        )
        .expect("arda map parse");
        assert!(arda_map["sections"]
            .as_array()
            .is_some_and(|sections| sections
                .iter()
                .any(|section| section["id"] == "governance_and_operations")));
    }

    #[test]
    fn queue_summary_uses_latest_task_state_per_id() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::create_dir_all(core_root.join("projects/tasks")).expect("task mkdir");
        fs::create_dir_all(core_root.join("projects/Plans")).expect("plans mkdir");

        fs::write(
            core_root.join("projects/tasks/queue.jsonl"),
            concat!(
                "{\"id\":\"tsk_a\",\"title\":\"A\",\"owner\":\"prometheus\",\"priority\":\"high\",\"status\":\"queued\"}\n",
                "{\"id\":\"tsk_a\",\"title\":\"A\",\"owner\":\"prometheus\",\"priority\":\"high\",\"status\":\"completed\",\"result\":\"completed\"}\n",
                "{\"id\":\"tsk_b\",\"title\":\"B\",\"owner\":\"athena\",\"priority\":\"medium\",\"status\":\"queued\"}\n"
            ),
        )
        .expect("queue write");

        write_queue_summary_projection(&core_root);

        let queue: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/queue_summary.json")).expect("queue read"),
        )
        .expect("queue parse");
        assert_eq!(queue["project_tasks"]["counts_by_status"]["queued"], 1);
        assert_eq!(queue["project_tasks"]["counts_by_status"]["completed"], 1);
        assert_eq!(queue["project_tasks"]["total_effective"], 2);
        assert_eq!(queue["project_tasks"]["open_compact"][0]["id"], "tsk_b");
        assert_eq!(queue["project_tasks"]["recent_compact"][0]["id"], "tsk_a");
        assert!(queue["project_tasks"]["recent"].is_null());
        assert_eq!(
            queue["agent_reading_policy"]["default_surface"],
            "core/state/queue_active.json"
        );
        assert_eq!(queue["arda_hints"]["alert_on_queued_tasks"], true);
    }

    #[test]
    fn writes_warden_guardhouse_from_queue_and_policy_inputs() {
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        let data_root = dir.path().join("data");
        let config_root = dir.path().join("config");
        fs::create_dir_all(core_root.join("realm")).expect("realm mkdir");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::create_dir_all(&config_root).expect("config mkdir");
        fs::create_dir_all(data_root.join("warden")).expect("warden mkdir");
        fs::create_dir_all(data_root.join("prometheus")).expect("prometheus mkdir");
        fs::create_dir_all(data_root.join("fleet").join("informants")).expect("fleet mkdir");
        fs::create_dir_all(data_root.join("hermes")).expect("hermes mkdir");
        fs::create_dir_all(data_root.join("charon")).expect("charon data mkdir");
        fs::create_dir_all(data_root.join("hades")).expect("hades data mkdir");
        fs::create_dir_all(core_root.join("edge")).expect("edge mkdir");
        fs::create_dir_all(core_root.join("metrics").join("by_crate").join("hades"))
            .expect("hades metrics mkdir");
        fs::create_dir_all(core_root.join("metrics").join("by_crate").join("charon"))
            .expect("charon metrics mkdir");
        fs::create_dir_all(core_root.join("metrics").join("by_crate").join("hermes"))
            .expect("hermes metrics mkdir");
        fs::create_dir_all(core_root.join("metrics").join("by_crate").join("mnemosyne"))
            .expect("mnemosyne metrics mkdir");
        fs::create_dir_all(data_root.join("mnemosyne").join("episodic").join("2026-03"))
            .expect("mnemosyne episodic mkdir");

        fs::write(
            core_root.join("realm/boot.toml"),
            "[ceo]\nagent_id = \"arandur\"\n[nightly]\nenabled = true\nrun_at = \"03:00\"\narchive_complete_after_days = 7\nprune_low_resonance = true\nmin_resonance_threshold = 25.0\ncompact_ledger = true\nemit_daily_summary = true\nsummary_path = \"data/summaries/\"\n",
        )
        .expect("boot write");
        fs::write(
            core_root.join("realm/annunimas.toml"),
            "[identity]\nname = \"Annunimas\"\nsigil = \"𓀀\"\n[[realms.definition]]\nid = \"command\"\ncolor = \"#fff\"\n",
        )
        .expect("identity write");
        fs::write(
            core_root.join("realm/agents.toml"),
            "[[agent]]\nid = \"arandur\"\nsigil = \"𓀀\"\nname = \"Arandur\"\nrealm = \"command\"\nclearance = \"sovereign\"\n",
        )
        .expect("agents write");
        fs::write(
            core_root.join("state/permission_profiles.json"),
            "{ \"active_profile\": \"operator\", \"profiles\": { \"operator\": { \"expires_at_utc\": null, \"scopes\": { \"network\": { \"allowed\": true, \"expires_at_utc\": null }, \"destructive\": { \"allowed\": true, \"expires_at_utc\": null } } } } }\n",
        )
        .expect("permission write");
        fs::write(
            core_root.join("state/destructive_quorum.json"),
            "{ \"enabled\": true, \"min_reviewers\": 2 }\n",
        )
        .expect("quorum write");
        fs::write(
            core_root.join("state/interrupt_authority.json"),
            "{ \"default\": { \"allow\": [\"illuvatar\"] } }\n",
        )
        .expect("interrupt write");
        fs::write(
            data_root.join("prometheus/health_workflow_last.json"),
            "{ \"status\": \"attention_required\", \"issues\": [\"disk_pressure\"] }\n",
        )
        .expect("health workflow write");
        fs::write(
            data_root.join("prometheus/maintenance_last.json"),
            "{ \"sweep_type\": \"scheduled\", \"compaction\": true }\n",
        )
        .expect("maintenance write");
        fs::write(
            data_root.join("prometheus/drift_report_last.json"),
            "{ \"drift_count\": 1, \"auto_open\": true }\n",
        )
        .expect("drift write");
        fs::write(
            data_root.join("prometheus/pressure_guard_last.json"),
            "{ \"status\": \"attention_required\", \"oversize_files\": 1 }\n",
        )
        .expect("pressure write");
        fs::write(
            core_root.join("metrics/audit_latest.json"),
            "{ \"summary\": { \"largest_file\": \"core/metrics/history/example.json\", \"oversize_files\": 1 } }\n",
        )
        .expect("audit write");
        fs::write(
            core_root.join("metrics/by_crate/hades/status.json"),
            "{ \"scheduler\": { \"nightly_hour_utc\": 4, \"watch_paths\": [\"core\", \"docs\"] }, \"pending_actions\": 1 }\n",
        )
        .expect("hades status write");
        fs::write(
            core_root.join("metrics/by_crate/hades/queue.json"),
            "[{\"task_id\":\"hds_1\",\"action\":\"remove\",\"file\":\"alpha\"}]\n",
        )
        .expect("hades queue write");
        fs::write(
            core_root.join("metrics/by_crate/charon/status.json"),
            "{ \"charon_version\": \"0.1.0\", \"providers_total\": 2, \"providers_healthy\": 1 }\n",
        )
        .expect("charon status write");
        fs::write(
            core_root.join("metrics/by_crate/charon/providers.json"),
            "[{\"id\":\"local_fallback\",\"in_cooldown\":false,\"error_count\":0,\"consecutive_failures\":0,\"requests_used_day\":1,\"requests_per_day\":null},{\"id\":\"groq\",\"in_cooldown\":true,\"error_count\":5,\"consecutive_failures\":3,\"requests_used_day\":100,\"requests_per_day\":100}]\n",
        )
        .expect("charon providers write");
        fs::write(
            core_root.join("metrics/by_crate/charon/state.json"),
            "{ \"providers\": [{\"id\":\"local_fallback\"}] }\n",
        )
        .expect("charon state write");
        fs::write(
            data_root.join("charon/state.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-09T11:00:00Z\",\"event\":\"route_selected\",\"payload\":{\"provider_id\":\"local_fallback\"}}\n",
                "{\"ts\":\"2026-03-09T11:01:00Z\",\"event\":\"provider_result\",\"payload\":{\"provider_id\":\"groq\",\"ok\":false}}\n"
            ),
        )
        .expect("charon event write");
        fs::write(
            core_root.join("metrics/by_crate/hermes/status.json"),
            "{ \"queue_depth\": 2, \"boardroom_active\": true, \"providers_online\": [\"discord\"], \"providers_offline\": [\"email\"], \"messages_today\": { \"outbound\": 1 } }\n",
        )
        .expect("hermes status write");
        fs::write(
            core_root.join("metrics/by_crate/hermes/providers.json"),
            "{ \"configured\": [\"discord\", \"email\"], \"online\": [\"discord\"], \"offline\": [\"email\"] }\n",
        )
        .expect("hermes providers write");
        fs::write(
            core_root.join("metrics/by_crate/hermes/subcomponents.json"),
            "[{\"id\":\"boardroom_manager\",\"status\":\"running\"},{\"id\":\"discord_listener\",\"status\":\"running\"}]\n",
        )
        .expect("hermes subcomponents write");
        fs::write(
            data_root.join("hermes/boardroom.jsonl"),
            "{\"from_agent\":\"athena\",\"message_type\":\"report\",\"priority\":\"normal\",\"subject\":\"Corpus\",\"body\":\"Depth stable\",\"posted_at_utc\":\"2026-03-09T11:00:00Z\"}\n",
        )
        .expect("hermes boardroom write");
        fs::write(
            data_root.join("hermes/interruptions.jsonl"),
            "{\"event_id\":\"int_1\",\"policy_authorized\":false,\"sender\":\"operator\",\"disposition\":\"override\"}\n",
        )
        .expect("hermes interruptions write");
        fs::write(
            data_root.join("hermes/decision_metrics.jsonl"),
            "{\"event\":\"decision_prompt_created\",\"prompt_id\":\"dp_1\"}\n",
        )
        .expect("hermes decision metrics write");
        fs::write(
            data_root.join("hermes/council_sessions.jsonl"),
            "{\"session_id\":\"c_1\",\"status\":\"open\",\"topic\":\"routing\"}\n",
        )
        .expect("hermes council sessions write");
        fs::write(
            core_root.join("metrics/by_crate/mnemosyne/status.json"),
            "{ \"ok\": true, \"status\": { \"chain_integrity\": \"head_present\", \"informants_connected\": 0 } }\n",
        )
        .expect("mnemosyne status write");
        fs::write(
            core_root.join("metrics/by_crate/mnemosyne/stats.json"),
            "{ \"generated_at_utc\": \"2026-03-09T11:00:00Z\", \"memory_counts\": { \"core\": 1, \"active\": 1, \"peripheral\": 1, \"transient\": 0, \"consolidated\": 1, \"archived\": 0, \"released\": 1 }, \"last_consolidation_utc\": \"2026-03-09T10:00:00Z\", \"next_consolidation_utc\": \"2026-03-10T10:00:00Z\", \"chain_integrity\": \"head_present\", \"informants_connected\": 0 }\n",
        )
        .expect("mnemosyne stats write");
        fs::write(data_root.join("mnemosyne/chain_head"), "abc123chainhead\n")
            .expect("mnemosyne chain head write");
        fs::write(
            data_root.join("mnemosyne/last_consolidation_utc"),
            "2026-03-09T10:00:00Z\n",
        )
        .expect("mnemosyne consolidation write");
        fs::write(
            data_root
                .join("mnemosyne")
                .join("episodic")
                .join("2026-03")
                .join("mem_a1.jsonl"),
            concat!(
                "{\"sigil\":\"MNEME_CORE\",\"memory_id\":\"mem_a1\",\"created_at_utc\":\"2026-03-09T10:59:00Z\",\"authored_by\":\"mnemosyne\",\"version\":\"0.1.0\",\"hash\":\"sha256:abc\"}\n",
                "{\"type\":\"episodic\",\"source_crate\":\"prometheus\",\"event_type\":\"decision_completed\",\"significance\":0.92,\"content\":\"Continuity verified\",\"tags\":[\"audit\",\"continuity\"],\"ts_utc\":\"2026-03-09T11:00:00Z\"}\n"
            ),
        )
        .expect("mnemosyne episodic write");
        fs::write(
            data_root.join("mnemosyne/noise.jsonl"),
            "{\"ts\":\"2026-03-09T10:30:00Z\",\"event\":{\"crate_name\":\"apollo\"}}\n",
        )
        .expect("mnemosyne noise write");
        fs::write(
            data_root.join("mnemosyne/obsidian_index.jsonl"),
            "{\"sigil\":\"SCROLL\",\"ts_utc\":\"2026-03-09T10:40:00Z\",\"path\":\"notes/test.md\",\"snippet\":\"operator note\"}\n",
        )
        .expect("mnemosyne obsidian write");
        fs::write(
            data_root.join("hades/hades_log.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-09T11:00:00Z\",\"event\":\"repair_detected\",\"file\":\"alpha\",\"details\":{\"athena_task_queued\":true}}\n",
                "{\"ts\":\"2026-03-09T11:01:00Z\",\"event\":\"orphan_found\",\"file\":\"beta\",\"details\":{}}\n"
            ),
        )
        .expect("hades log write");
        fs::write(
            data_root.join("hades/joulework.jsonl"),
            "{\"ts_utc\":\"2026-03-09T11:02:00Z\",\"component\":\"hades\",\"operation\":\"sweep\"}\n",
        )
        .expect("hades joule write");
        fs::write(
            data_root.join("hades/warden_queue.jsonl"),
            "{\"ts\":\"2026-03-09T11:03:00Z\",\"event\":\"repair_detected\",\"file\":\"alpha\",\"synced\":false}\n",
        )
        .expect("hades warden write");
        fs::write(
            data_root.join("hades/athena_handoff_queue.jsonl"),
            "{\"ts_utc\":\"2026-03-09T11:04:00Z\",\"event\":\"repair_detected\",\"status\":\"queued_fallback\"}\n",
        )
        .expect("hades athena handoff write");
        fs::write(
            data_root.join("prometheus/fleet_control_last.json"),
            "{ \"status\": \"healthy\", \"reachable_nodes\": 3 }\n",
        )
        .expect("fleet write");
        fs::write(
            data_root.join("warden/informant_queue.jsonl"),
            concat!(
                "{\"ts_utc\":\"2026-03-09T11:00:00Z\",\"source\":\"metrics_export\",\"event_type\":\"crate_health_heartbeat\",\"crate_name\":\"athena\",\"status\":\"healthy\"}\n",
                "{\"ts_utc\":\"2026-03-09T11:01:00Z\",\"source\":\"health_workflow_router\",\"event_type\":\"health_workflow_planned\",\"status\":\"attention_required\",\"synced\":false}\n",
                "{\"ts_utc\":\"2026-03-09T11:02:00Z\",\"source\":\"repair_pipeline\",\"event_type\":\"repair_detected\",\"file\":\"docs/a.md\",\"status\":\"attention_required\",\"synced\":false}\n",
                "{\"ts_utc\":\"2026-03-09T11:03:00Z\",\"source\":\"repair_pipeline\",\"event_type\":\"repair_detected\",\"file\":\"docs/a.md\",\"status\":\"attention_required\",\"synced\":false}\n",
                "{\"ts_utc\":\"2026-03-09T11:04:00Z\",\"source\":\"repair_pipeline\",\"event_type\":\"repair_detected\",\"file\":\"docs/b.md\",\"status\":\"attention_required\",\"synced\":false}\n"
            ),
        )
        .expect("queue write");
        fs::write(
            data_root.join("prometheus/fleet_control_last.json"),
            "{ \"status\": \"ok\", \"network\": { \"tailscale_ok\": true, \"active_peers\": 1 } }\n",
        )
        .expect("fleet control write");
        fs::write(
            data_root.join("fleet/informants/local_last.json"),
            "{ \"node_id\": \"node-local\", \"hostname\": \"bluefin-main\", \"tailscale_ok\": false, \"tailscale_error\": \"operation not permitted\", \"ollama_ok\": false, \"hardware\": { \"cpu\": { \"model_name\": \"AMD Ryzen 9\" }, \"nvme_devices\": [{\"device\":\"/dev/nvme0n1\",\"model\":\"Samsung 970 EVO\"}] } }\n",
        )
        .expect("local informant write");
        fs::write(
            config_root.join("fleet.toml"),
            r#"
[fleet]
enabled = true
status_view_mode = "active_only"
stale_offline_threshold_days = 14
include_recent_offline_in_status = false

[[nodes]]
id = "node-pi5-warden"
role = "warden_guardhouse"
hostname = "pi5-warden"
display_name = "Pi5 Guardhouse"
tailscale_ip = "100.64.0.10"
node_class = "edge_guardhouse"
enrollment_status = "active"
llm_runtime = "edge_llm_light"
notes = "Primary guardhouse"
"#,
        )
        .expect("fleet config write");
        fs::write(
            core_root.join("edge/targets.example.toml"),
            "[[node]]\nid = \"edge-pi5-01\"\nrole = \"warden\"\nhostname = \"pi5-warden\"\ntailscale_ip = \"100.64.0.10\"\nssh_user = \"annunimas\"\nathena_enabled = true\nhermes_enabled = true\n",
        )
        .expect("targets write");
        fs::write(
            core_root.join("edge/targets.toml"),
            "[[node]]\nid = \"node-pi5-warden\"\nrole = \"warden_guardhouse\"\nhostname = \"pi5-warden\"\ntailscale_ip = \"100.64.0.10\"\nssh_user = \"annunimas\"\nwarden_enabled = true\nhermes_enabled = true\nnode_class = \"edge_guardhouse\"\nenrollment_status = \"active\"\nllm_runtime = \"edge_llm_light\"\nnotes = \"Primary guardhouse\"\n",
        )
        .expect("targets canonical write");
        fs::write(
            core_root.join("state/matrix_boardrooms.json"),
            r##"{
  "defaults": {
    "provider": "matrix",
    "client_surface": "element"
  },
  "root_space": {
    "alias": "#annunimas:matrix.local"
  },
  "rooms": [
    {"id": "ops-boardroom"},
    {"id": "ceo-boardroom"}
  ],
  "routing_contract": {
    "primary_boardroom_room_id": "ops-boardroom"
  },
  "bridge_contracts": {
    "discord": {
      "enabled": true
    }
  },
  "activation_requirements": {
    "federated_rooms_ready": false
  }
}
"##,
        )
        .expect("matrix boardrooms write");
        fs::write(
            core_root.join("state/github_repo_integration.json"),
            r#"{
  "summary": {
    "github_sources_total": 64,
    "integration_coverage_total": 64,
    "ready_for_activation_total": 6
  },
  "registry_tools": [
    {
      "tool": "discord-mcp",
      "package_enablement": {
        "integration_lane": "mcp_communications"
      }
    },
    {
      "tool": "litellm",
      "package_enablement": {
        "integration_lane": "charon_provider"
      }
    }
  ],
  "framework_surfaces": [
    {
      "name": "AgentForge"
    },
    {
      "name": "eliza"
    }
  ]
}
"#,
        )
        .expect("github repo integration write");

        let _ = CoreAutonomyProfile::load(&core_root).expect("profile");

        let guardhouse: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_guardhouse.json"))
                .expect("guardhouse read"),
        )
        .expect("guardhouse parse");
        assert_eq!(guardhouse["queue"]["recent_event_count"], 5);
        assert_eq!(guardhouse["queue"]["status_counts"]["healthy"], 1);
        assert_eq!(
            guardhouse["queue"]["status_counts"]["attention_required"],
            4
        );
        assert_eq!(
            guardhouse["queue"]["effective_status_counts"]["attention_required"],
            3
        );
        assert_eq!(
            guardhouse["queue"]["repair_pressure"]["repeated_repair_noise"],
            1
        );
        assert_eq!(
            guardhouse["queue"]["crate_health_heartbeats"]["athena"]["status"],
            "healthy"
        );
        assert_eq!(
            guardhouse["policy"]["permission_profiles"]["active_profile"],
            "operator"
        );
        assert_eq!(
            guardhouse["health"]["workflow"]["status"],
            "attention_required"
        );
        assert_eq!(guardhouse["health"]["fleet_control"]["status"], "ok");
        assert_eq!(
            guardhouse["health"]["fleet_control"]["network"]["tailscale_ok"],
            true
        );

        fs::write(
            data_root.join("warden/permission_profile_audit.jsonl"),
            concat!(
                "{\"ts_utc\":\"2026-03-09T11:02:00Z\",\"active_profile\":\"operator\",\"allowed\":true,\"reason\":\"allowed\"}\n",
                "{\"ts_utc\":\"2026-03-09T11:03:00Z\",\"active_profile\":\"operator\",\"allowed\":false,\"reason\":\"permission profile expired\"}\n"
            ),
        )
        .expect("permission audit write");
        fs::write(
            data_root.join("prometheus/escalations.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-09T11:04:00Z\",\"reason\":\"policy_guard.denied\",\"severity\":\"critical\"}\n",
                "{\"ts\":\"2026-03-09T11:05:00Z\",\"reason\":\"interrupt_authority_policy.denied\",\"severity\":\"critical\"}\n",
                "{\"ts\":\"2026-03-09T11:06:00Z\",\"reason\":\"destructive quorum denied for remove\",\"severity\":\"critical\"}\n"
            ),
        )
        .expect("escalations write");
        fs::write(
            data_root.join("hermes/reroute_metrics.jsonl"),
            concat!(
                "{\"ts_utc\":\"2026-03-09T11:06:30Z\",\"event\":\"deferred\",\"reason\":\"reroute_rate_limited\"}\n",
                "{\"ts_utc\":\"2026-03-09T11:07:00Z\",\"event\":\"forwarded\",\"handed_off\":true}\n"
            ),
        )
        .expect("reroute metrics write");
        fs::write(
            data_root.join("hermes/reroute_acks.jsonl"),
            "{\"ts_utc\":\"2026-03-09T11:08:00Z\",\"event\":\"forwarded\",\"ack\":{\"acknowledged\":true,\"ack_id\":\"ack-1\"}}\n",
        )
        .expect("reroute acks write");

        let _ = CoreAutonomyProfile::load(&core_root).expect("profile reload");

        let policy: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_policy_authority.json"))
                .expect("policy read"),
        )
        .expect("policy parse");
        assert_eq!(policy["permission_profile"]["active_profile"], "operator");
        assert_eq!(
            policy["permission_profile"]["recent_audit"]["allow_count"],
            1
        );
        assert_eq!(
            policy["permission_profile"]["recent_audit"]["deny_count"],
            1
        );
        assert_eq!(
            policy["policy_guard"]["recent_denials"]
                .as_array()
                .map(|v| v.len()),
            Some(1)
        );
        assert_eq!(
            policy["interrupt_authority"]["recent_policy_denials"]
                .as_array()
                .map(|v| v.len()),
            Some(1)
        );
        assert_eq!(
            policy["destructive_quorum"]["recent_related_escalations"]
                .as_array()
                .map(|v| v.len()),
            Some(1)
        );
        assert_eq!(
            policy["interrupt_authority"]["recent_reroute_acks"][0]["ack"]["ack_id"],
            "ack-1"
        );

        let edge: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_edge_contract.json"))
                .expect("edge read"),
        )
        .expect("edge parse");
        assert_eq!(edge["mesh"]["tailscale_mesh_ok"], true);
        assert_eq!(edge["mesh"]["local_probe_ok"], false);
        assert_eq!(edge["mesh"]["ack_gap_present"], true);
        assert_eq!(edge["inventory"]["configured_targets_count"], 1);
        assert_eq!(edge["edge_contract"]["recent_unsynced_events"], 4);

        let fleet_runtime: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_runtime.json"))
                .expect("fleet runtime read"),
        )
        .expect("fleet runtime parse");
        assert_eq!(fleet_runtime["authority"], "fleet_runtime_projection");
        assert_eq!(fleet_runtime["inventory"]["configured_nodes_count"], 1);

        let fleet_nodes: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_nodes.json"))
                .expect("fleet nodes read"),
        )
        .expect("fleet nodes parse");
        assert_eq!(fleet_nodes["authority"], "fleet_nodes_projection");
        assert_eq!(fleet_nodes["counts"]["configured_total"], 1);

        let fleet_health: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_health.json"))
                .expect("fleet health read"),
        )
        .expect("fleet health parse");
        assert_eq!(fleet_health["authority"], "fleet_health_projection");
        assert_eq!(fleet_health["counts"]["configured_nodes_total"], 1);

        let fleet_hardware: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_hardware.json"))
                .expect("fleet hardware read"),
        )
        .expect("fleet hardware parse");
        assert_eq!(fleet_hardware["authority"], "fleet_hardware_projection");
        assert_eq!(
            fleet_hardware["local_node"]["hardware"]["cpu"]["model_name"],
            "AMD Ryzen 9"
        );

        let fleet_backbone: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/fleet_backbone.json"))
                .expect("fleet backbone read"),
        )
        .expect("fleet backbone parse");
        assert_eq!(fleet_backbone["authority"], "fleet_backbone_projection");

        let doctrine: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/warden_nightly_doctrine.json"))
                .expect("doctrine read"),
        )
        .expect("doctrine parse");
        assert_eq!(doctrine["declared_doctrine"]["run_at"], "03:00");
        assert_eq!(
            doctrine["implemented_runtime"]["drift_detection"]["status"]["drift_count"],
            1
        );
        assert_eq!(
            doctrine["implemented_runtime"]["hades_scheduler"]["status"]["scheduler"]
                ["nightly_hour_utc"],
            4
        );
        assert_eq!(doctrine["ownership_map"][4]["status"], "missing");

        let charon: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/charon_router.json")).expect("charon read"),
        )
        .expect("charon parse");
        assert_eq!(charon["status"]["providers_total"], 2);
        assert_eq!(
            charon["provider_pressure"]["cooldowns"]
                .as_array()
                .map(|v| v.len()),
            Some(1)
        );
        assert_eq!(
            charon["provider_pressure"]["degraded"]
                .as_array()
                .map(|v| v.len()),
            Some(1)
        );
        assert_eq!(charon["recent_events"].as_array().map(|v| v.len()), Some(2));

        let hades: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/hades_lifecycle.json")).expect("hades read"),
        )
        .expect("hades parse");
        assert_eq!(hades["status"]["pending_actions"], 1);
        assert_eq!(hades["queue"].as_array().map(|v| v.len()), Some(1));
        assert_eq!(hades["recent_activity"]["counts"]["repair_events"], 1);
        assert_eq!(hades["recent_activity"]["counts"]["orphan_events"], 1);
        assert_eq!(
            hades["recent_activity"]["counts"]["athena_fallback_handoffs"],
            1
        );

        let hermes: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/hermes_command.json")).expect("hermes read"),
        )
        .expect("hermes parse");
        assert_eq!(hermes["status"]["queue_depth"], 2);
        assert_eq!(
            hermes["communications"]["boardroom_contract"]["provider"],
            "matrix"
        );
        assert_eq!(
            hermes["communications"]["boardroom_contract"]["room_count"],
            2
        );
        assert_eq!(
            hermes["github_repo_integration"]["summary"]["integration_coverage_total"],
            64
        );
        assert_eq!(
            hermes["github_repo_integration"]["communications_registry_tools"]
                .as_array()
                .map(|v| v.len()),
            Some(2)
        );
        assert_eq!(hermes["recent_activity"]["counts"]["boardroom_posts"], 1);
        assert_eq!(
            hermes["recent_activity"]["counts"]["boardroom_contract_rooms"],
            2
        );
        assert_eq!(hermes["recent_activity"]["counts"]["deferred_reroutes"], 1);
        assert_eq!(hermes["recent_activity"]["counts"]["denied_interrupts"], 1);
        assert_eq!(hermes["recent_activity"]["counts"]["open_councils"], 1);
        assert_eq!(
            hermes["arda_hints"]["boardroom_section"],
            "matrix_boardrooms"
        );
        assert_eq!(hermes["arda_hints"]["alert_on_matrix_activation_gap"], true);

        let mnemosyne: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(core_root.join("state/mnemosyne_continuity.json"))
                .expect("mnemosyne read"),
        )
        .expect("mnemosyne parse");
        assert_eq!(mnemosyne["stats"]["memory_counts"]["core"], 1);
        assert_eq!(mnemosyne["continuity"]["chain_head_present"], true);
        assert_eq!(
            mnemosyne["recent_activity"]["counts"]["recent_memory_count"],
            1
        );
        assert_eq!(
            mnemosyne["recent_activity"]["counts"]["high_significance_memories"],
            1
        );
        assert_eq!(mnemosyne["recent_activity"]["counts"]["noise_events"], 1);
        assert_eq!(
            mnemosyne["recent_activity"]["counts"]["obsidian_entries"],
            1
        );
    }
}
