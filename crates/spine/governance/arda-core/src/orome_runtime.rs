use std::collections::HashMap;
use std::path::{PathBuf};
use std::sync::{Arc, RwLock};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OromeRuntimeStateError {
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization failure: {0}")]
    Serialize(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRegistryState {
    pub entries: Vec<AgentRecord>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Online,
    Busy,
    Away,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub agent_id: String,
    pub name: String,
    pub status: AgentStatus,
    pub last_heartbeat: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl AgentRecord {
    pub fn new(agent_id: impl Into<String>, name: impl Into<String>, status: AgentStatus) -> Self {
        Self {
            agent_id: agent_id.into(),
            name: name.into(),
            status,
            last_heartbeat: Utc::now().to_rfc3339(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    state: Arc<RwLock<AgentRegistryState>>,
    path: PathBuf,
}

impl AgentRegistry {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Self { state: Arc::new(RwLock::new(state)), path }
    }

    pub fn upsert(&self, record: AgentRecord) -> Result<(), OromeRuntimeStateError> {
        let mut state = self.state.write().unwrap();
        if let Some(existing) = state.entries.iter_mut().find(|entry| entry.agent_id == record.agent_id) {
            *existing = record;
        } else {
            state.entries.push(record);
        }
        state.updated_at = Utc::now().to_rfc3339();
        self.persist(&state)
    }

    pub fn update_status(&self, agent_id: &str, status: AgentStatus) -> Result<(), OromeRuntimeStateError> {
        let mut state = self.state.write().unwrap();
        if let Some(entry) = state.entries.iter_mut().find(|entry| entry.agent_id == agent_id) {
            entry.status = status;
            entry.last_heartbeat = Utc::now().to_rfc3339();
        } else {
            state.entries.push(AgentRecord::new(agent_id, agent_id, status));
        }
        state.updated_at = Utc::now().to_rfc3339();
        self.persist(&state)
    }

    pub fn snapshot(&self) -> AgentRegistryState {
        self.state.read().unwrap().clone()
    }

    pub fn shared(self) -> SharedRegistryStateStorage {
        SharedRegistryStateStorage(SharedAgentRegistry::Registry(Arc::new(self)))
    }

    fn persist(&self, state: &AgentRegistryState) -> Result<(), OromeRuntimeStateError> {
        let json = serde_json::to_string_pretty(state).map_err(|err| OromeRuntimeStateError::Serialize(err.to_string()))?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, format!("{json}\n"))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OromeCoreRuntimeState {
    pub schema_version: String,
    pub routed_at: String,
    pub route_id: String,
    pub current_task_id: Option<String>,
    pub queued_messages: Vec<RoutedMessage>,
    pub agent_registry: AgentRegistryState,
    pub bookmarks: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedMessage {
    pub message_id: String,
    pub provider: String,
    pub channel: String,
    pub subject: String,
    pub body: String,
    pub priority: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct RouterState {
    state: Arc<RwLock<OromeCoreRuntimeState>>,
    state_root: PathBuf,
}

#[derive(Debug, Clone)]
pub enum SharedAgentRegistry {
    Registry(std::sync::Arc<AgentRegistry>),
    Placeholder,
}

impl Default for SharedAgentRegistry {
    fn default() -> Self {
        Self::Placeholder
    }
}

#[derive(Debug, Clone, Default)]
pub struct SharedRegistryStateStorage(pub SharedAgentRegistry);

#[derive(Debug, Clone, Default)]
pub struct SharedRouterStateStorage(pub SharedAgentRegistry);

impl RouterState {
    pub fn new(state_root: impl Into<PathBuf>) -> Self {
        let state_root = state_root.into();
        let state = std::fs::read_to_string(state_root.join("runtime").join("om.json"))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Self { state: Arc::new(RwLock::new(state)), state_root }
    }

    pub fn update_route_state(
        &self,
        task_id: Option<String>,
        route_id: String,
        message: RoutedMessage,
    ) -> Result<(), OromeRuntimeStateError> {
        let mut state = self.state.write().unwrap();
        state.schema_version = "arda.orome.runtime.v1".to_string();
        state.routed_at = Utc::now().to_rfc3339();
        state.route_id = route_id;
        state.current_task_id = task_id;
        state.queued_messages.push(message);
        self.persist(&state)
    }

    pub fn snapshot(&self) -> OromeCoreRuntimeState {
        self.state.read().unwrap().clone()
    }

    pub fn shared(self) -> SharedRouterStateStorage {
        SharedRouterStateStorage(SharedAgentRegistry::Placeholder)
    }

    fn persist(&self, state: &OromeCoreRuntimeState) -> Result<(), OromeRuntimeStateError> {
        let target = self.state_root.join("runtime").join("om.json");
        let json = serde_json::to_string_pretty(state).map_err(|err| OromeRuntimeStateError::Serialize(err.to_string()))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, format!("{json}\n"))?;
        Ok(())
    }
}

impl SharedRegistryStateStorage {
    pub fn upsert_agent(&self, record: AgentRecord) -> Result<(), OromeRuntimeStateError> {
        match &self.0 {
            SharedAgentRegistry::Registry(registry) => registry.upsert(record),
            SharedAgentRegistry::Placeholder => Ok(()),
        }
    }

    pub fn snapshot(&self) -> AgentRegistryState {
        match &self.0 {
            SharedAgentRegistry::Registry(registry) => registry.snapshot(),
            SharedAgentRegistry::Placeholder => AgentRegistryState::default(),
        }
    }
}
