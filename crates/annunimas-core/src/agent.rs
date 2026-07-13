use crate::error::Result;
use crate::soterion::SoterionMeta;
use crate::task::Task;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub version: String,
    pub soterion: Option<SoterionMeta>,
}

impl AgentManifest {
    pub fn with_sigil(mut self, sigil: &str, realm: &str) -> Self {
        self.soterion = Some(SoterionMeta {
            sigil: Some(sigil.to_string()),
            realm: Some(realm.to_string()),
            tags: vec!["agent".to_string()],
            ..Default::default()
        });
        self
    }
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &[&str];
    async fn execute(&self, task: &mut Task) -> Result<()>;
}
