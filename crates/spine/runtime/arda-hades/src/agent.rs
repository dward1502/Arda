use crate::service::HadesService;
use arda_core::agent::{Agent, AgentManifest};
use arda_core::error::{ArdaError, Result};
use arda_core::task::Task;
use arda_core::SoterionMeta;
use async_trait::async_trait;

pub struct HadesAgent {
    manifest: AgentManifest,
    capabilities: Vec<&'static str>,
    service: HadesService,
}

impl HadesAgent {
    pub fn new() -> Result<Self> {
        let capabilities = vec!["sweep", "remove", "cleanup", "lifecycle", "order"];
        let manifest = AgentManifest {
            name: "hades".to_owned(),
            description: "Cleanup, lifecycle, and order maintenance agent".to_owned(),
            capabilities: capabilities.iter().map(|v| (*v).to_owned()).collect(),
            version: "0.1.0".to_owned(),
            soterion: Some(SoterionMeta {
                sigil: Some("𓁷".to_owned()),
                realm: Some("operations".to_owned()),
                tags: vec!["cleanup".to_owned(), "lifecycle".to_owned()],
                resonance: Some(65.0),
                triad_gate: Some("hades".to_owned()),
                joule_cost: Some(3.0),
                clearance: Some("normal".to_owned()),
                extra: std::collections::HashMap::new(),
            }),
        };
        let service =
            HadesService::from_default_or_fallback().map_err(|err| ArdaError::Agent {
                agent: "hades".to_owned(),
                message: format!("failed to initialize HADES storage: {err}"),
            })?;

        Ok(Self {
            manifest,
            capabilities,
            service,
        })
    }
}

#[async_trait]
impl Agent for HadesAgent {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn capabilities(&self) -> &[&str] {
        &self.capabilities
    }

    async fn execute(&self, task: &mut Task) -> Result<()> {
        task.start_execution();
        match task.task_type.as_str() {
            "sweep" => {
                let result = self.service.sweep("task", None)?;
                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "mode": "sweep",
                    "result": result
                }));
            }
            "remove" => {
                let queued = self
                    .service
                    .queue_remove(&task.description, "orchestrator")?;
                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "mode": "remove",
                    "queued": queued
                }));
            }
            _ => {
                let status = self.service.status()?;
                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "mode": "status",
                    "status": status
                }));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_hades_agent_from_runtime_home() {
        let dir = tempfile::tempdir().expect("tempdir");
        let original = std::env::var_os("ARDA_HADES_HOME");
        std::env::set_var("ARDA_HADES_HOME", dir.path());

        let agent = HadesAgent::new().expect("agent");
        assert_eq!(agent.name(), "hades");
        assert!(agent.capabilities().contains(&"sweep"));

        match original {
            Some(value) => std::env::set_var("ARDA_HADES_HOME", value),
            None => std::env::remove_var("ARDA_HADES_HOME"),
        }
    }
}
