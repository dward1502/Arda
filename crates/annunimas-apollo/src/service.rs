// sigil: REPAIR
use crate::{
    ApolloExecutor, ExecutionRequest, ExecutionResult, InterruptionAttachment,
    InterruptionAttachmentRequest,
};
use annunimas_plutus::{JouleWorkUnit, PlutusService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const APOLLO_RUNTIME_SCHEMA_VERSION: &str = "annunimas.apollo.runtime.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloRuntimePaths {
    pub home: String,
    pub status_path: String,
    pub requests_path: String,
}

#[derive(Clone)]
pub struct ApolloService {
    home: PathBuf,
    status_path: PathBuf,
    requests_path: PathBuf,
    executor: Arc<ApolloExecutor>,
}

impl ApolloService {
    pub fn from_home(home: impl AsRef<Path>) -> anyhow::Result<Self> {
        let home = home.as_ref().to_path_buf();
        fs::create_dir_all(&home)?;
        Ok(Self {
            status_path: home.join("runtime_status.json"),
            requests_path: home.join("pending_requests.json"),
            home,
            executor: Arc::new(ApolloExecutor::new()),
        })
    }

    pub fn from_default_or_workspace_fallback() -> anyhow::Result<Self> {
        let home = std::env::var("ANNUNIMAS_APOLLO_HOME").unwrap_or_else(|_| "data/apollo".into());
        Self::from_home(home)
    }

    pub fn runtime_paths(&self) -> ApolloRuntimePaths {
        ApolloRuntimePaths {
            home: self.home.to_string_lossy().to_string(),
            status_path: self.status_path.to_string_lossy().to_string(),
            requests_path: self.requests_path.to_string_lossy().to_string(),
        }
    }

    pub async fn status(&self) -> anyhow::Result<serde_json::Value> {
        let snapshot = self.snapshot().await?;
        if !self.status_path.exists() {
            self.persist_snapshot(&snapshot)?;
        }
        Ok(snapshot)
    }

    pub async fn submit(&self, request: ExecutionRequest) -> anyhow::Result<String> {
        self.persist_pending_request(&request)?;
        let task_id = self.executor.submit(request).await;
        self.persist_snapshot(&self.snapshot().await?)?;
        Ok(task_id)
    }

    pub async fn execute(&self, task_id: &str) -> anyhow::Result<Option<ExecutionResult>> {
        self.ensure_task_loaded(task_id).await?;
        let result = self.executor.execute(task_id).await;
        if let Some(execution) = &result {
            self.remove_pending_request(task_id)?;
            self.emit_work_signal_background(
                "apollo",
                execution.joule_work.max(0.25),
                JouleWorkUnit::Compute,
                Some(execution.task_id.clone()),
            );
            if let Some(governance) = &execution.governance {
                self.emit_relationship_signal_background(
                    &execution.agent_id,
                    governance.love_equation_guard.resonance,
                    governance.love_equation_guard.attention,
                    governance.love_equation_guard.reciprocity,
                );
            }
        }
        self.persist_snapshot(&self.snapshot().await?)?;
        Ok(result)
    }

    pub async fn attach_interrupt(
        &self,
        request: InterruptionAttachmentRequest<'_>,
    ) -> anyhow::Result<Option<InterruptionAttachment>> {
        self.ensure_task_loaded(request.task_id).await?;
        let record = self.executor.attach_interrupt(request).await;
        self.persist_snapshot(&self.snapshot().await?)?;
        Ok(record)
    }

    async fn snapshot(&self) -> anyhow::Result<serde_json::Value> {
        let mut executor = self.executor.status_snapshot().await;
        let persisted = self.read_pending_requests()?;
        if let Some(previous_executor) = self.read_persisted_executor_snapshot()? {
            let has_live_results = executor
                .get("results")
                .and_then(|v| v.as_array())
                .map(|rows| !rows.is_empty())
                .unwrap_or(false);
            if !has_live_results {
                if let Some(previous_results) = previous_executor.get("results").cloned() {
                    executor["results"] = previous_results;
                }
                if let Some(previous_summary) = previous_executor.get("summary").cloned() {
                    executor["summary"] = previous_summary;
                }
                if let Some(previous_interruptions) =
                    previous_executor.get("interruptions").cloned()
                {
                    executor["interruptions"] = previous_interruptions;
                }
            }
        }
        if let Some(queue) = executor.get_mut("queue").and_then(|v| v.as_object_mut()) {
            let mut depth = None;
            if let Some(pending_tasks) = queue
                .get_mut("pending_tasks")
                .and_then(|v| v.as_array_mut())
            {
                let known = pending_tasks
                    .iter()
                    .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                    .collect::<std::collections::HashSet<_>>();
                for task_id in persisted.keys() {
                    if !known.contains(task_id) {
                        pending_tasks.push(json!(task_id));
                    }
                }
                depth = Some(pending_tasks.len());
            }
            if let Some(depth) = depth {
                queue.insert("depth".to_string(), json!(depth));
            }
        }
        Ok(json!({
            "schema_version": APOLLO_RUNTIME_SCHEMA_VERSION,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "authority": "apollo_service",
            "paths": self.runtime_paths(),
            "executor": executor,
        }))
    }

    fn persist_snapshot(&self, snapshot: &serde_json::Value) -> anyhow::Result<()> {
        fs::write(
            &self.status_path,
            serde_json::to_string_pretty(snapshot)? + "\n",
        )?;
        Ok(())
    }

    fn read_pending_requests(
        &self,
    ) -> anyhow::Result<std::collections::BTreeMap<String, ExecutionRequest>> {
        match fs::read_to_string(&self.requests_path) {
            Ok(content) if !content.trim().is_empty() => Ok(serde_json::from_str(&content)?),
            Ok(_) => Ok(Default::default()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn write_pending_requests(
        &self,
        requests: &std::collections::BTreeMap<String, ExecutionRequest>,
    ) -> anyhow::Result<()> {
        fs::write(
            &self.requests_path,
            serde_json::to_string_pretty(requests)? + "\n",
        )?;
        Ok(())
    }

    fn persist_pending_request(&self, request: &ExecutionRequest) -> anyhow::Result<()> {
        let mut requests = self.read_pending_requests()?;
        requests.insert(request.task_id.clone(), request.clone());
        self.write_pending_requests(&requests)
    }

    fn read_persisted_executor_snapshot(&self) -> anyhow::Result<Option<serde_json::Value>> {
        let content = match fs::read_to_string(&self.status_path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let snapshot = serde_json::from_str::<serde_json::Value>(&content)?;
        Ok(snapshot.get("executor").cloned())
    }

    fn remove_pending_request(&self, task_id: &str) -> anyhow::Result<()> {
        let mut requests = self.read_pending_requests()?;
        requests.remove(task_id);
        self.write_pending_requests(&requests)
    }

    async fn ensure_task_loaded(&self, task_id: &str) -> anyhow::Result<()> {
        let pending = self.read_pending_requests()?;
        let Some(request) = pending.get(task_id).cloned() else {
            return Ok(());
        };
        if self.executor.get_result(task_id).await.is_some() {
            return Ok(());
        }
        let current_pending = self.executor.pending_tasks().await;
        if current_pending.iter().any(|candidate| candidate == task_id) {
            return Ok(());
        }
        self.executor.submit(request).await;
        Ok(())
    }

    async fn record_work_signal_async(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> anyhow::Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback()?;
        plutus.track_work(agent_id, amount, unit, task_id).await?;
        Ok(())
    }

    async fn record_relationship_signal_async(
        &self,
        from: &str,
        to: &str,
        resonance: f64,
        attention: f64,
        reciprocity: f64,
    ) -> anyhow::Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback()?;
        plutus
            .record_relationship(from, to, resonance, attention, reciprocity)
            .await?;
        Ok(())
    }

    fn emit_work_signal_background(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) {
        let service = self.clone();
        let agent_id = agent_id.to_string();
        tokio::spawn(async move {
            if let Err(err) = service
                .record_work_signal_async(&agent_id, amount, unit, task_id)
                .await
            {
                tracing::debug!(error = %err, "APOLLO plutus work signal failed");
            }
        });
    }

    fn emit_relationship_signal_background(
        &self,
        to_agent: &str,
        resonance: f64,
        attention: f64,
        reciprocity: f64,
    ) {
        let service = self.clone();
        let to_agent = to_agent.to_string();
        tokio::spawn(async move {
            if let Err(err) = service
                .record_relationship_signal_async(
                    "apollo",
                    &to_agent,
                    resonance,
                    attention,
                    reciprocity,
                )
                .await
            {
                tracing::debug!(error = %err, "APOLLO plutus relationship signal failed");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutionPriority;
    use annunimas_plutus::PlutusService;

    #[tokio::test]
    async fn persists_runtime_status_and_interrupts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let plutus_home = temp.path().join("plutus");
        std::env::set_var("ANNUNIMAS_PLUTUS_HOME", &plutus_home);
        let service = ApolloService::from_home(temp.path()).expect("service");
        let task_id = service
            .submit(ExecutionRequest {
                task_id: "task_apollo_runtime".to_string(),
                agent_id: "athena".to_string(),
                payload: json!({"op":"ingest"}),
                priority: ExecutionPriority::High,
                timeout_secs: 30,
            })
            .await
            .expect("submit");
        service
            .attach_interrupt(InterruptionAttachmentRequest {
                task_id: &task_id,
                source: "voice",
                sender: "operator",
                content: "reroute",
                disposition: "reroute",
                run_id: Some("run-1".to_string()),
                session_id: None,
            })
            .await
            .expect("interrupt");
        let result = service.execute(&task_id).await.expect("execute");
        assert!(result.is_some());

        let status = service.status().await.expect("status");
        assert_eq!(status["authority"], "apollo_service");
        assert_eq!(status["executor"]["summary"]["completed_total"], 1);
        assert_eq!(status["executor"]["summary"]["interruptions_total"], 1);
        assert!(
            status["executor"]["summary"]["discovered_tools_total"]
                .as_u64()
                .unwrap_or_default()
                >= 1
        );
        assert!(temp.path().join("runtime_status.json").exists());
        let plutus = PlutusService::from_home(&plutus_home).expect("plutus");
        let mut total = 0.0;
        for _ in 0..20 {
            total = plutus.status().await.expect("plutus status")["joulework"]["total"]
                .as_f64()
                .unwrap_or(0.0);
            if total > 0.0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(total > 0.0);
        std::env::remove_var("ANNUNIMAS_PLUTUS_HOME");
    }

    #[tokio::test]
    async fn pending_request_survives_service_restart_and_executes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        service
            .submit(ExecutionRequest {
                task_id: "task_apollo_continuity".to_string(),
                agent_id: "prometheus".to_string(),
                payload: json!({"op":"continuity"}),
                priority: ExecutionPriority::Normal,
                timeout_secs: 30,
            })
            .await
            .expect("submit");

        let restarted = ApolloService::from_home(temp.path()).expect("restarted");
        let status = restarted.status().await.expect("status");
        assert_eq!(status["executor"]["queue"]["depth"], 1);

        let result = restarted
            .execute("task_apollo_continuity")
            .await
            .expect("execute after restart");
        assert!(result.is_some());

        let final_status = restarted.status().await.expect("status after execute");
        assert_eq!(final_status["executor"]["queue"]["depth"], 0);
        assert_eq!(final_status["executor"]["summary"]["completed_total"], 1);
    }

    #[tokio::test]
    async fn execution_surfaces_tool_plan_and_harness_policy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        let task_id = service
            .submit(ExecutionRequest {
                task_id: "task_apollo_tools".to_string(),
                agent_id: "apollo".to_string(),
                payload: json!({
                    "repository_path": "/tmp/repo",
                    "required_capabilities": ["repository", "verification", "shell"],
                    "command": "cargo test -q"
                }),
                priority: ExecutionPriority::High,
                timeout_secs: 30,
            })
            .await
            .expect("submit");

        let result = service
            .execute(&task_id)
            .await
            .expect("execute")
            .expect("result");
        let output = result.output.expect("output");
        assert!(output["discovered_tools"]
            .as_array()
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert_eq!(
            output["harness_policy"]["approval_required"].as_bool(),
            Some(true)
        );
        assert!(output["execution_sequence"]
            .as_array()
            .map(|rows| rows.iter().any(|v| v.as_str() == Some("verify_outputs")))
            .unwrap_or(false));
    }
}
