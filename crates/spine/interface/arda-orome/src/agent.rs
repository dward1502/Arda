use crate::service::HermesService;
use crate::types::{BoardroomPost, OutboundMessage};
use arda_core::agent::{Agent, AgentManifest};
use arda_core::error::Result;
use arda_core::task::Task;
use arda_core::SoterionMeta;
use async_trait::async_trait;

pub struct HermesAgent {
    manifest: AgentManifest,
    capabilities: Vec<&'static str>,
    service: HermesService,
}

impl HermesAgent {
    pub fn new() -> Result<Self> {
        let capabilities = vec![
            "message",
            "notify",
            "broadcast",
            "boardroom",
            "calendar_sync",
            "communications",
        ];
        let manifest = AgentManifest {
            name: "hermes".to_string(),
            description: "Communications and boardroom relay agent".to_string(),
            capabilities: capabilities.iter().map(|v| v.to_string()).collect(),
            version: "0.1.0".to_string(),
            soterion: Some(SoterionMeta {
                sigil: Some("𓅃".to_string()),
                realm: Some("communications".to_string()),
                tags: vec!["communications".to_string(), "boardroom".to_string()],
                resonance: Some(70.0),
                triad_gate: Some("hermes".to_string()),
                joule_cost: Some(4.0),
                clearance: Some("normal".to_string()),
                extra: std::collections::HashMap::new(),
            }),
        };
        Ok(Self {
            manifest,
            capabilities,
            service: HermesService::from_default_or_fallback()?,
        })
    }
}

#[async_trait]
impl Agent for HermesAgent {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn capabilities(&self) -> &[&str] {
        &self.capabilities
    }

    async fn execute(&self, task: &mut Task) -> Result<()> {
        task.start_execution();
        match task.task_type.as_str() {
            "boardroom" => {
                let post = BoardroomPost::new(
                    "prometheus",
                    "audit",
                    "Boardroom message",
                    task.description.clone(),
                );
                self.service.boardroom_post(post)?;
                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "mode": "boardroom",
                    "posted": true
                }));
            }
            "calendar_sync" => {
                let out = self.service.calendar_sync()?;
                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "mode": "calendar_sync",
                    "snapshot": out
                }));
            }
            _ => {
                let outbound = OutboundMessage::new(
                    "discord",
                    "boardroom",
                    format!("{} relay", task.task_type),
                    task.description.clone(),
                );
                let queued = self.service.send(outbound).await?;
                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "mode": "relay",
                    "queued": queued
                }));
            }
        }
        Ok(())
    }
}
