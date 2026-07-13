// sigil: REPAIR
use crate::error::{AnnunimasError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DAEMON_SCHEMA_VERSION: &str = "annunimas.daemon.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    #[serde(default = "default_daemon_schema_version")]
    pub schema_version: String,
    pub cmd: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    #[serde(default = "default_daemon_schema_version")]
    pub schema_version: String,
    pub ok: bool,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

impl ResponseEnvelope {
    pub fn success(result: Value) -> Self {
        Self {
            schema_version: default_daemon_schema_version(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            schema_version: default_daemon_schema_version(),
            ok: false,
            result: None,
            error: Some(error.into()),
        }
    }

    pub fn into_result(self, agent: &str) -> Result<Value> {
        if self.ok {
            Ok(self.result.unwrap_or_else(|| serde_json::json!({})))
        } else {
            Err(AnnunimasError::Agent {
                agent: agent.to_string(),
                message: self
                    .error
                    .unwrap_or_else(|| "unknown IPC command failure".to_string()),
            })
        }
    }
}

impl CommandEnvelope {
    pub fn new(cmd: impl Into<String>, payload: Value) -> Self {
        Self {
            schema_version: default_daemon_schema_version(),
            cmd: cmd.into(),
            payload,
        }
    }
}

fn default_daemon_schema_version() -> String {
    DAEMON_SCHEMA_VERSION.to_string()
}
