use crate::adaptive::service::status::PackageRuntimeSignals;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: std::process::ExitStatus,
}

impl CommandOutput {
    pub fn ok(self) -> Result<Self, std::io::Error> {
        if self.status.success() {
            Ok(self)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("command failed: {:?}", self.status),
            ))
        }
    }
}
use chrono::Utc;
use serde_json::Value as JsonValue;
use std::fs;
use std::time::Duration as StdDuration;

struct LlmfitSignals {
    backend: String,
    recommendation_count: usize,
    local_max_params_b: Option<f64>,
    top_model_names: Vec<String>,
}

fn collect_llmfit_signals() -> LlmfitSignals {
    let output = match command_output_with_timeout(
        Command::new("llmfit").args(["recommend", "--json"]),
        StdDuration::from_secs(3),
    ) {
        Ok(output) => output,
        Err(_) => {
            return LlmfitSignals {
                backend: "optional_signal_absent".to_string(),
                recommendation_count: 0,
                local_max_params_b: None,
                top_model_names: Vec::new(),
            };
        }
    };
    if !output.status.success() {
        return LlmfitSignals {
            backend: "probe_failed".to_string(),
            recommendation_count: 0,
            local_max_params_b: None,
            top_model_names: Vec::new(),
        };
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    let models = value
        .get("models")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    LlmfitSignals {
        backend: value
            .get("system")
            .and_then(|v| v.get("backend"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        recommendation_count: models.len(),
        local_max_params_b: models
            .iter()
            .take(5)
            .filter_map(|entry| entry.get("params_b").and_then(|v| v.as_f64()))
            .reduce(f64::max),
        top_model_names: models
            .iter()
            .take(3)
            .filter_map(|entry| {
                entry
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
            })
            .collect::<Vec<_>>(),
    }
}

struct NanoclawSignals {
    binary_present: bool,
    runtime_ready: bool,
    probe_state: String,
}

fn collect_nanoclaw_signals() -> NanoclawSignals {
    let path = super::paths::arda_root().join("core/state/package_runtime_activation.json");
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(parsed) = serde_json::from_str::<JsonValue>(&content) {
            if let Some(surface) = parsed
                .get("surfaces")
                .and_then(|value| value.get("nanoclaw"))
            {
                return NanoclawSignals {
                    binary_present: surface
                        .get("binary_present")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(true),
                    runtime_ready: surface
                        .get("runtime_ready")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false),
                    probe_state: surface
                        .get("status")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                };
            }
        }
    }
    let output = match command_output_with_timeout(
        Command::new("bash")
            .arg("scripts/runtime/nanoclaw_runtime.sh")
            .arg("status"),
        StdDuration::from_secs(3),
    ) {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return NanoclawSignals {
                binary_present: false,
                runtime_ready: false,
                probe_state: "missing_binary".to_string(),
            };
        }
        Err(_) => {
            return NanoclawSignals {
                binary_present: true,
                runtime_ready: false,
                probe_state: "probe_error".to_string(),
            };
        }
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed = serde_json::from_str::<JsonValue>(text.trim()).ok();
    NanoclawSignals {
        binary_present: parsed
            .as_ref()
            .and_then(|value| value.get("binary_present"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(true),
        runtime_ready: parsed
            .as_ref()
            .and_then(|value| value.get("runtime_ready"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false),
        probe_state: parsed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(JsonValue::as_str)
            .unwrap_or(if output.status.success() {
                "ready"
            } else {
                "error"
            })
            .to_string(),
    }
}

pub fn command_output_with_timeout(
    command: &mut Command,
    _timeout: StdDuration,
) -> Result<crate::adaptive::service::bootstrap_runtime::CommandOutput, std::io::Error> {
    let output = command.output()?;
    Ok(crate::adaptive::service::bootstrap_runtime::CommandOutput {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
    })
}

pub(super) fn collect_package_runtime_signals() -> PackageRuntimeSignals {
    let llmfit = collect_llmfit_signals();
    let nanoclaw = collect_nanoclaw_signals();
    PackageRuntimeSignals {
        generated_at_utc: Utc::now().to_rfc3339(),
        llmfit_backend: llmfit.backend,
        llmfit_recommendation_count: llmfit.recommendation_count,
        llmfit_local_max_params_b: llmfit.local_max_params_b,
        llmfit_top_model_names: llmfit.top_model_names,
        nanoclaw_binary_present: nanoclaw.binary_present,
        nanoclaw_runtime_ready: nanoclaw.runtime_ready,
        nanoclaw_probe_state: nanoclaw.probe_state,
    }
}