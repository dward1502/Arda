#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Wire boot.toml + world.json into autopilot config and registry.

use super::delegation::{AgentCapabilities, AgentRegistry};
use crate::prometheus::core_link::CoreAutonomyProfile;
use super::source_registry::AgentRosterSnapshot;
use std::collections::BTreeMap;
use std::path::Path;

/// Default capability table mapped from agent realm.
/// Realms come from `core/realm/agents.toml` via world.json.
fn capabilities_for_realm(realm: &str) -> Vec<&'static str> {
    match realm {
        "command" => vec!["ops", "policy", "build", "decide"],
        "knowledge" => vec!["analysis", "research", "ingest", "synthesize"],
        "governance" => vec!["governance", "decide"],
        "finance" => vec!["budget"],
        "communications" => vec!["comms", "communicate"],
        "monitoring" => vec!["monitor"],
        "memory" => vec!["memory", "archive"],
        "inference" => vec!["inference", "synthesize"],
        _ => vec!["ops"],
    }
}

pub fn load_registry_from_world(
    world_path: impl AsRef<Path>,
    heartbeat_timeout_secs: u64,
) -> AgentRegistry {
    let mut reg = AgentRegistry::new();
    let Some(roster) = AgentRosterSnapshot::from_world_file(world_path, heartbeat_timeout_secs)
    else {
        return seed_default_registry();
    };
    for agent in &roster.agents {
        // Roster status carries ONLINE/SILENT/OFFLINE. Treat ONLINE as available;
        // others get registered with current_load saturated so they are not selected.
        let online = agent.status == "ONLINE";
        // We don't have realm in AgentRosterSnapshot, fall back to id-based heuristic.
        let realm = realm_from_id(&agent.id);
        let task_types: Vec<String> = capabilities_for_realm(realm)
            .into_iter()
            .map(String::from)
            .collect();
        reg.register(AgentCapabilities {
            agent_id: agent.id.clone(),
            task_types,
            max_concurrent: if online { 8 } else { 0 },
            current_load: 0,
            success_rate: 0.8,
        });
    }
    if reg.agents().count() == 0 {
        seed_default_registry()
    } else {
        reg
    }
}

fn realm_from_id(id: &str) -> &'static str {
    match id {
        "arandur" | "ceo" | "prometheus" => "command",
        "athena" => "knowledge",
        "oracle" | "council" => "governance",
        "plutus" => "finance",
        "hermes" => "communications",
        "warden" => "monitoring",
        "mnemosyne" => "memory",
        "manwe" => "inference",
        _ => "command",
    }
}

pub fn seed_default_registry() -> AgentRegistry {
    let mut r = AgentRegistry::new();
    let entries: &[(&str, &str)] = &[
        ("ceo", "command"),
        ("warden", "monitoring"),
        ("athena", "knowledge"),
        ("prometheus", "command"),
        ("hermes", "communications"),
        ("manwe", "inference"),
        ("plutus", "finance"),
        ("council", "governance"),
        ("oracle", "governance"),
        ("mnemosyne", "memory"),
    ];
    for (id, realm) in entries {
        r.register(AgentCapabilities {
            agent_id: (*id).into(),
            task_types: capabilities_for_realm(realm)
                .into_iter()
                .map(String::from)
                .collect(),
            max_concurrent: 8,
            current_load: 0,
            success_rate: 0.8,
        });
    }
    r
}

#[derive(Debug, Clone)]
pub struct LoadedDefaults {
    pub joule_budget: f64,
    pub heartbeat_ms: u64,
    pub base_costs: BTreeMap<String, f64>,
}

pub fn load_defaults(core_root: impl AsRef<Path>) -> LoadedDefaults {
    let profile = CoreAutonomyProfile::load(&core_root);
    let base_costs: BTreeMap<String, f64> = profile
        .as_ref()
        .map(|p| p.base_costs.iter().map(|(k, v)| (k.clone(), *v)).collect())
        .unwrap_or_default();
    // Budget heuristic: sum of base_costs * 50 (≈50 task batch headroom), floor 500, cap 50_000.
    let sum: f64 = base_costs.values().copied().sum();
    let joule_budget = (sum * 50.0).clamp(500.0, 50_000.0);
    let heartbeat_ms = profile.as_ref().map(|p| p.heartbeat_ms).unwrap_or(500);
    LoadedDefaults {
        joule_budget,
        heartbeat_ms,
        base_costs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeded_registry_has_known_agents() {
        let r = seed_default_registry();
        assert!(r.agents().any(|a| a.agent_id == "warden"));
        assert!(r.agents().any(|a| a.agent_id == "athena"));
    }
    #[test]
    fn defaults_returns_sane_budget() {
        let d = load_defaults("/nonexistent");
        assert!(d.joule_budget >= 500.0);
    }
}
