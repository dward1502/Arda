// sigil: REPAIR
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub realm: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
    pub last_seen: DateTime<Utc>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Online,
    Busy,
    Away,
    Offline,
}

impl AgentInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>, realm: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            realm: realm.into(),
            capabilities: Vec::new(),
            status: AgentStatus::Offline,
            last_seen: Utc::now(),
            endpoint: None,
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn is_available(&self) -> bool {
        self.status != AgentStatus::Offline && self.status != AgentStatus::Busy
    }
}

#[derive(Default)]
pub struct AgentRegistry {
    agents: HashMap<String, AgentInfo>,
    by_realm: HashMap<String, Vec<String>>,
    by_capability: HashMap<String, Vec<String>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, agent: AgentInfo) {
        let id = agent.id.clone();
        let realm = agent.realm.clone();

        self.agents.insert(id.clone(), agent);

        self.by_realm.entry(realm).or_default().push(id.clone());

        if let Some(agent) = self.agents.get(&id) {
            for cap in &agent.capabilities {
                self.by_capability
                    .entry(cap.clone())
                    .or_default()
                    .push(id.clone());
            }
        }
    }

    pub fn unregister(&mut self, agent_id: &str) {
        if let Some(agent) = self.agents.remove(agent_id) {
            if let Some(ids) = self.by_realm.get_mut(&agent.realm) {
                ids.retain(|id| id != agent_id);
            }
            for cap in &agent.capabilities {
                if let Some(ids) = self.by_capability.get_mut(cap) {
                    ids.retain(|id| id != agent_id);
                }
            }
        }
    }

    pub fn get(&self, agent_id: &str) -> Option<&AgentInfo> {
        self.agents.get(agent_id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&AgentInfo> {
        self.agents.values().find(|a| a.name == name)
    }

    pub fn list_all(&self) -> Vec<&AgentInfo> {
        self.agents.values().collect()
    }

    pub fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.values().cloned().collect()
    }

    pub fn list_by_realm(&self, realm: &str) -> Vec<&AgentInfo> {
        self.by_realm
            .get(realm)
            .map(|ids| ids.iter().filter_map(|id| self.agents.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn list_by_capability(&self, capability: &str) -> Vec<&AgentInfo> {
        self.by_capability
            .get(capability)
            .map(|ids| ids.iter().filter_map(|id| self.agents.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn list_available(&self) -> Vec<&AgentInfo> {
        self.agents.values().filter(|a| a.is_available()).collect()
    }

    pub fn update_status(&mut self, agent_id: &str, status: AgentStatus) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = status;
            agent.last_seen = Utc::now();
        }
    }

    pub fn heartbeat(&mut self, agent_id: &str) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.last_seen = Utc::now();
            if agent.status == AgentStatus::Away {
                agent.status = AgentStatus::Online;
            }
        }
    }

    pub fn prune_stale(&mut self, threshold_secs: i64) -> Vec<String> {
        let threshold = Utc::now() - chrono::Duration::seconds(threshold_secs);
        let mut removed = Vec::new();

        let stale: Vec<String> = self
            .agents
            .iter()
            .filter(|(_, a)| a.last_seen < threshold && a.status != AgentStatus::Offline)
            .map(|(id, _)| id.clone())
            .collect();

        for id in stale {
            self.unregister(&id);
            removed.push(id);
        }

        removed
    }
}

pub type SharedRegistry = Arc<RwLock<AgentRegistry>>;

impl AgentRegistry {
    pub fn shared() -> SharedRegistry {
        Arc::new(RwLock::new(Self::new()))
    }
}
