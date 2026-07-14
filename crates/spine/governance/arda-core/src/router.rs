// sigil: REPAIR
use crate::agent::Agent;
use crate::error::Result;
use crate::task::Task;
use std::collections::HashMap;

pub struct Router {
    agents: HashMap<String, Box<dyn Agent>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn register(&mut self, agent: Box<dyn Agent>) {
        self.agents.insert(agent.name().to_string(), agent);
    }

    pub fn route(&self, task: &Task) -> Result<&dyn Agent> {
        for agent in self.agents.values() {
            if agent.capabilities().contains(&task.task_type.as_str()) {
                return Ok(&**agent);
            }
        }
        Err(crate::error::ArdaError::NoRoute(
            task.task_type.clone(),
        ))
    }

    pub fn list_agents(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
