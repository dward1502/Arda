//! HTTP client for a ComfyUI server.
//!
//! Submits workflow JSON to `/prompt`, polls `/history/<prompt_id>`, downloads
//! outputs via `/view`. Enabled with the `comfyui` feature.
//!
//! Default endpoint: `http://arda-server:8188` (the backbone) — override
//! with `FORGE_COMFYUI_ADDR=<scheme>://<host>:<port>`.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_ADDR: &str = "http://arda-server:8188";
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
pub const DEFAULT_TIMEOUT_SECS: u64 = 1800;
pub const DEFAULT_CLIENT_ID: &str = "arda-forge-mind";

#[derive(Debug, Clone)]
pub struct ComfyUiClient {
    base_url: String,
    poll_interval: Duration,
    timeout: Duration,
    client_id: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
struct PromptRequest<'a> {
    prompt: &'a Value,
    client_id: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
struct PromptResponse {
    prompt_id: String,
    #[serde(default)]
    node_errors: Value,
}

/// One file output the workflow produced (image, mesh, etc.) under a node output's `<channel>` list.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowOutputFile {
    pub filename: String,
    #[serde(default)]
    pub subfolder: String,
    #[serde(default = "default_type")]
    pub r#type: String,
}

fn default_type() -> String {
    "output".to_string()
}

/// Aggregated workflow result after history says it's finished.
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub prompt_id: String,
    pub status: String,
    pub completed: bool,
    pub outputs: Vec<(String, WorkflowOutputFile)>, // (node_id, file)
    pub error: Option<String>,
}

impl Default for ComfyUiClient {
    fn default() -> Self {
        Self::new(DEFAULT_ADDR)
    }
}

impl ComfyUiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self {
            base_url,
            poll_interval: Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            client_id: DEFAULT_CLIENT_ID.to_string(),
            http: reqwest::Client::builder()
                .pool_idle_timeout(Some(Duration::from_secs(30)))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn from_env() -> Self {
        let addr = std::env::var("FORGE_COMFYUI_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
        let timeout_secs = std::env::var("FORGE_COMFYUI_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        let mut client = Self::new(addr);
        client.timeout = Duration::from_secs(timeout_secs);
        client
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn with_timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    pub fn with_poll_interval(mut self, t: Duration) -> Self {
        self.poll_interval = t;
        self
    }

    /// POST a workflow JSON to `/prompt`. Returns the assigned prompt_id.
    pub async fn submit_workflow(&self, workflow: &Value) -> anyhow::Result<String> {
        let url = format!("{}/prompt", self.base_url);
        let body = PromptRequest {
            prompt: workflow,
            client_id: &self.client_id,
        };
        tracing::debug!(target: "forge.comfyui", url = %url, "submit workflow");
        let resp = self.http.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("comfyui /prompt rejected ({status}): {text}");
        }
        let parsed: PromptResponse = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!("comfyui /prompt returned non-PromptResponse JSON: {e}; body: {text}")
        })?;
        if let Some(errs) = parsed.node_errors.as_object() {
            if !errs.is_empty() {
                anyhow::bail!("comfyui workflow validation failed: {}", parsed.node_errors);
            }
        }
        Ok(parsed.prompt_id)
    }

    /// Poll `/history/<prompt_id>` until completion or timeout.
    pub async fn wait_for(&self, prompt_id: &str) -> anyhow::Result<WorkflowResult> {
        let url = format!("{}/history/{}", self.base_url, prompt_id);
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            if tokio::time::Instant::now() > deadline {
                anyhow::bail!(
                    "comfyui /history/{prompt_id} timed out after {:?}",
                    self.timeout
                );
            }
            let resp = self.http.get(&url).send().await?;
            if !resp.status().is_success() {
                tracing::debug!(target: "forge.comfyui", "history poll non-2xx: {}", resp.status());
                tokio::time::sleep(self.poll_interval).await;
                continue;
            }
            let value: Value = resp.json().await?;
            if let Some(entry) = value.get(prompt_id) {
                let status = entry.get("status").cloned().unwrap_or(Value::Null);
                let status_str = status
                    .get("status_str")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let completed = status
                    .get("completed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let done = completed || matches!(status_str.as_str(), "error" | "success");
                if done {
                    let outputs = extract_outputs(entry);
                    let error = extract_error(&status);
                    return Ok(WorkflowResult {
                        prompt_id: prompt_id.to_string(),
                        status: status_str,
                        completed,
                        outputs,
                        error,
                    });
                }
            }
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Submit + wait, one-shot.
    pub async fn run(&self, workflow: &Value) -> anyhow::Result<WorkflowResult> {
        let prompt_id = self.submit_workflow(workflow).await?;
        tracing::info!(target: "forge.comfyui", %prompt_id, "workflow accepted, polling");
        self.wait_for(&prompt_id).await
    }

    /// Download an output file via `/view?filename=...&subfolder=...&type=...`.
    pub async fn download_output(&self, file: &WorkflowOutputFile) -> anyhow::Result<Vec<u8>> {
        let url = format!("{}/view", self.base_url);
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("filename", file.filename.as_str()),
                ("subfolder", file.subfolder.as_str()),
                ("type", file.r#type.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }
}

fn extract_outputs(entry: &Value) -> Vec<(String, WorkflowOutputFile)> {
    let mut out = Vec::new();
    let Some(map) = entry.get("outputs").and_then(Value::as_object) else {
        return out;
    };
    for (node_id, payload) in map {
        let Some(obj) = payload.as_object() else {
            continue;
        };
        for files_value in obj.values() {
            let Some(arr) = files_value.as_array() else {
                continue;
            };
            for entry in arr {
                if let Ok(parsed) = serde_json::from_value::<WorkflowOutputFile>(entry.clone()) {
                    out.push((node_id.clone(), parsed));
                }
            }
        }
    }
    out
}

fn extract_error(status: &Value) -> Option<String> {
    let messages = status.get("messages")?.as_array()?;
    for msg in messages {
        let arr = msg.as_array()?;
        if arr.first().and_then(Value::as_str) == Some("execution_error") {
            let payload = arr.get(1)?;
            let node = payload
                .get("node_type")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let body = payload
                .get("exception_message")
                .and_then(Value::as_str)
                .unwrap_or("(no message)");
            return Some(format!("{node}: {body}"));
        }
    }
    None
}
