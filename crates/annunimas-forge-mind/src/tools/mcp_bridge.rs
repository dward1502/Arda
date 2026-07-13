//! TCP bridge to the [ahujasid/blender-mcp](https://github.com/ahujasid/blender-mcp)
//! Blender addon.
//!
//! The upstream addon opens a raw TCP socket inside Blender (default
//! loopback host/port) and accepts newline-tolerant JSON requests of the shape
//! `{"type": "<command>", "params": { ... }}`. The Python `uvx blender-mcp`
//! MCP server is a thin shim over that socket; we connect to it directly so
//! forge-mind has no Python runtime dependency.
//!
//! Enabled with the `mcp-bridge` feature.

use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Default Blender addon address.
pub fn default_addr() -> String {
    format!("{}:{}", "127.0.0.1", 9876)
}

/// Default request/response timeout.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct McpBridge {
    addr: String,
    timeout: Duration,
}

#[derive(Debug, Serialize)]
struct Request<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    params: Value,
}

impl Default for McpBridge {
    fn default() -> Self {
        Self::new(default_addr())
    }
}

impl McpBridge {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    pub fn from_env() -> Self {
        let addr = std::env::var("BLENDER_MCP_ADDR").unwrap_or_else(|_| default_addr());
        let secs = std::env::var("BLENDER_MCP_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        Self {
            addr,
            timeout: Duration::from_secs(secs),
        }
    }

    pub fn with_timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    /// Issue a raw `{type, params}` request and return the parsed response.
    pub async fn call(&self, command: &str, params: Value) -> anyhow::Result<Value> {
        let request = Request {
            kind: command,
            params,
        };
        let bytes = serde_json::to_vec(&request)?;
        tracing::debug!(target: "forge.mcp", addr = %self.addr, command, "blender-mcp call");

        let response = timeout(self.timeout, self.exchange(&bytes))
            .await
            .map_err(|_| anyhow::anyhow!("blender-mcp timeout talking to {}", self.addr))??;

        if let Some(status) = response.get("status").and_then(Value::as_str) {
            if status != "success" {
                let msg = response
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown blender-mcp error");
                anyhow::bail!("blender-mcp error: {msg}");
            }
        }
        Ok(response)
    }

    async fn exchange(&self, bytes: &[u8]) -> anyhow::Result<Value> {
        let mut stream = TcpStream::connect(&self.addr).await?;
        stream.write_all(bytes).await?;
        stream.flush().await?;

        let mut buf = Vec::with_capacity(8192);
        let mut chunk = [0u8; 8192];
        loop {
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Ok(value) = serde_json::from_slice::<Value>(&buf) {
                return Ok(value);
            }
        }
        serde_json::from_slice::<Value>(&buf)
            .map_err(|e| anyhow::anyhow!("malformed blender-mcp response: {e}"))
    }

    // ---- typed wrappers for the upstream addon commands ----

    pub async fn get_scene_info(&self) -> anyhow::Result<Value> {
        self.call("get_scene_info", json!({})).await
    }

    pub async fn get_object_info(&self, name: &str) -> anyhow::Result<Value> {
        self.call("get_object_info", json!({ "name": name })).await
    }

    pub async fn execute_code(&self, code: &str) -> anyhow::Result<Value> {
        self.call("execute_code", json!({ "code": code })).await
    }

    pub async fn get_viewport_screenshot(&self, max_size: Option<u32>) -> anyhow::Result<Value> {
        let mut params = json!({});
        if let Some(s) = max_size {
            params["max_size"] = json!(s);
        }
        self.call("get_viewport_screenshot", params).await
    }

    pub async fn get_polyhaven_status(&self) -> anyhow::Result<Value> {
        self.call("get_polyhaven_status", json!({})).await
    }

    pub async fn download_polyhaven_asset(
        &self,
        asset_id: &str,
        asset_type: &str,
        resolution: Option<&str>,
    ) -> anyhow::Result<Value> {
        let mut params = json!({
            "asset_id": asset_id,
            "asset_type": asset_type,
        });
        if let Some(r) = resolution {
            params["resolution"] = json!(r);
        }
        self.call("download_polyhaven_asset", params).await
    }
}
