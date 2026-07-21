pub mod human;
pub mod ingest;
pub mod learning;
pub mod transport;

use arda_core::agent::{Agent, AgentManifest};
use arda_core::error::{ArdaError, Result};
use arda_core::llm::{ChatMessage, ChatRequest, LlmProvider};
use arda_core::task::Task;
use arda_core::SoterionMeta;
use async_trait::async_trait;
use ingest::AthenaStore;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AthenaAgent {
    manifest: AgentManifest,
    capabilities: Vec<&'static str>,
    llm: Arc<dyn LlmProvider>,
    model_routes: HashMap<String, String>,
    store: AthenaStore,
}

impl AthenaAgent {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Result<Self> {
        Self::with_model_routes(llm, HashMap::new())
    }

    pub fn with_model_routes(
        llm: Arc<dyn LlmProvider>,
        model_routes: HashMap<String, String>,
    ) -> Result<Self> {
        let capabilities = vec![
            "ingest",
            "query",
            "deep_analyze",
            "deep",
            "research",
            "code",
            "decision",
            "general",
        ];
        let store =
            AthenaStore::from_default_or_workspace_fallback().map_err(|e| ArdaError::Agent {
                agent: "athena".to_string(),
                message: format!("failed to initialize Athena storage: {e}"),
            })?;

        let manifest = AgentManifest {
            name: "athena".to_string(),
            description: "Knowledge ingest and synthesis agent".to_string(),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
            version: "0.1.0".to_string(),
            soterion: Some(SoterionMeta {
                sigil: Some("𓁿".to_string()),
                realm: Some("knowledge".to_string()),
                tags: vec!["knowledge".to_string(), "ingest".to_string()],
                resonance: Some(80.0),
                triad_gate: Some("athena".to_string()),
                joule_cost: Some(10.0),
                clearance: Some("full".to_string()),
                extra: std::collections::HashMap::new(),
            }),
        };

        Ok(Self {
            manifest,
            capabilities,
            llm,
            model_routes,
            store,
        })
    }

    fn build_system_prompt(&self, task_type: &str) -> String {
        match task_type {
            "research" => "You are Athena, an AI research agent. Provide thorough, well-structured analysis. Be concise but comprehensive.".into(),
            "code" => "You are Athena, an AI coding agent. Write clean, idiomatic code. Explain your approach briefly.".into(),
            "decision" => "You are Athena, an AI decision-support agent. Analyze tradeoffs, recommend an action, explain your reasoning.".into(),
            "ingest" => "You are Athena, an AI ingestion agent. Summarize and extract key information from the provided content.".into(),
            _ => "You are Athena, an AI agent in the Arda system. Help the user with their request.".into(),
        }
    }
}

#[async_trait]
impl Agent for AthenaAgent {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn capabilities(&self) -> &[&str] {
        &self.capabilities
    }

    async fn execute(&self, task: &mut Task) -> Result<()> {
        task.start_execution();

        if task.task_type == "ingest" {
            let record = self
                .store
                .ingest(&task.description, "orchestrator", "pipeline task")?;

            task.complete(serde_json::json!({
                "agent": self.name(),
                "mode": "local_ingest_mvp",
                "storage_root": self.store.root().display().to_string(),
                "record": record,
            }));
            return Ok(());
        }

        if task.task_type == "query" {
            let response = self.store.query(&task.description, 8)?;
            task.complete(serde_json::json!({
                "agent": self.name(),
                "mode": "local_query_mvp",
                "query": response,
            }));
            return Ok(());
        }

        if task.task_type == "deep_analyze" || task.task_type == "deep" {
            let source_id = task.description.trim();
            let queued =
                self.store
                    .queue_deep_analysis(source_id, "orchestrator", "pipeline deep task")?;
            let deep = self.store.deep_analyze(source_id)?;
            task.complete(serde_json::json!({
                "agent": self.name(),
                "mode": "local_deep_analysis_mvp",
                "queued": queued,
                "deep": deep,
            }));
            return Ok(());
        }

        let system_prompt = self.build_system_prompt(&task.task_type);

        // Get model for task type, fall back to default
        let model = self
            .model_routes
            .get(&task.task_type)
            .cloned()
            .unwrap_or_else(|| self.llm.default_model().to_string());

        let request = ChatRequest::new(vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(&task.description),
        ])
        .with_model(model)
        .with_max_tokens(2048);

        let model_used = request
            .model
            .clone()
            .unwrap_or_else(|| self.llm.default_model().to_string());
        tracing::info!(
            agent = self.name(),
            provider = self.llm.provider_name(),
            model = model_used.as_str(),
            task_type = task.task_type.as_str(),
            "Executing task"
        );

        match self.llm.chat(request).await {
            Ok(response) => {
                let usage_info = response.usage.as_ref().map(|u| {
                    serde_json::json!({
                        "prompt_tokens": u.prompt_tokens,
                        "completion_tokens": u.completion_tokens,
                        "total_tokens": u.total_tokens,
                    })
                });

                task.complete(serde_json::json!({
                    "agent": self.name(),
                    "model": response.model,
                    "provider": self.llm.provider_name(),
                    "response": response.content,
                    "usage": usage_info,
                    "finish_reason": response.finish_reason,
                }));

                Ok(())
            }
            Err(e) => {
                tracing::error!(agent = self.name(), error = %e, "LLM call failed");
                Err(e)
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    pub(crate) fn env_guard() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
