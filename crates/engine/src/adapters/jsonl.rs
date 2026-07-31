use super::{
    AdapterCancellation, AdapterError, AdapterProcessConfig, AdapterProvenance, AdapterRequest,
    AdapterResult, AdapterStatus, ADAPTER_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct JsonlAdapter {
    config: AdapterProcessConfig,
}

impl JsonlAdapter {
    pub fn new(mut config: AdapterProcessConfig) -> Result<Self, AdapterError> {
        if !config.executable.is_absolute() {
            return Err(AdapterError::ExecutableNotAbsolute(config.executable));
        }
        let executable = config
            .executable
            .canonicalize()
            .map_err(|_| AdapterError::InvalidExecutable(config.executable.clone()))?;
        if !executable
            .metadata()
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
        {
            return Err(AdapterError::InvalidExecutable(executable));
        }
        let project_root = config
            .project_root
            .canonicalize()
            .map_err(|_| AdapterError::InvalidProjectRoot(config.project_root.clone()))?;
        if !project_root
            .metadata()
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
        {
            return Err(AdapterError::InvalidProjectRoot(project_root));
        }
        let cwd = config
            .cwd
            .canonicalize()
            .map_err(|_| AdapterError::InvalidCwd(config.cwd.clone()))?;
        if !cwd
            .metadata()
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
        {
            return Err(AdapterError::InvalidCwd(cwd));
        }
        if !cwd.starts_with(&project_root) {
            return Err(AdapterError::CwdOutsideProject { cwd, project_root });
        }
        for key in config.environment.keys() {
            if !config.environment_allowlist.contains(key) {
                return Err(AdapterError::EnvironmentDenied(key.clone()));
            }
            if key.is_empty() || key.contains(['=', '\0']) {
                return Err(AdapterError::InvalidConfig(format!(
                    "invalid environment key {key:?}"
                )));
            }
        }
        if config.timeout.is_zero() {
            return Err(AdapterError::InvalidConfig(
                "timeout must be greater than zero".into(),
            ));
        }
        if config.max_line_bytes < 256 {
            return Err(AdapterError::InvalidConfig(
                "max_line_bytes must be at least 256".into(),
            ));
        }
        config.executable = executable;
        config.project_root = project_root;
        config.cwd = cwd;
        Ok(Self { config })
    }

    pub async fn execute(
        &self,
        request: AdapterRequest,
        cancellation: AdapterCancellation,
    ) -> Result<AdapterResult, AdapterError> {
        self.validate_request(&request)?;
        let effective_timeout = self.config.timeout.min(request.timeout);
        let mut command = Command::new(&self.config.executable);
        command
            .args(&self.config.args)
            .current_dir(&self.config.cwd)
            .env_clear()
            .envs(&self.config.environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(AdapterError::Spawn)?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| AdapterError::Protocol("adapter stdin was not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AdapterError::Protocol("adapter stdout was not piped".into()))?;
        let mut reader = BufReader::new(stdout);
        let mut cancellation_rx = cancellation.subscribe();
        let session = self.run_session(
            &request,
            effective_timeout,
            &mut stdin,
            &mut reader,
            &mut cancellation_rx,
        );
        let outcome = match tokio::time::timeout(effective_timeout, session).await {
            Ok(result) => result,
            Err(_) => Err(AdapterError::Timeout),
        };

        if matches!(outcome, Err(AdapterError::Cancelled)) {
            let cancel = json!({
                "schema_version": ADAPTER_SCHEMA_VERSION,
                "id": format!("{}:cancel", request.id),
                "type": "cancel",
                "request_id": request.id,
            });
            let _ = write_frame(&mut stdin, &cancel, self.config.max_line_bytes).await;
        }
        drop(stdin);
        reap_process(&mut child, self.config.cancellation_grace).await;
        outcome
    }

    fn validate_request(&self, request: &AdapterRequest) -> Result<(), AdapterError> {
        if request.id.is_empty()
            || request.operation.is_empty()
            || request.idempotency_key.is_empty()
            || request.timeout.is_zero()
        {
            return Err(AdapterError::InvalidConfig(
                "request id, operation, idempotency key, and timeout must be non-empty".into(),
            ));
        }
        if !request.arguments.is_object() {
            return Err(AdapterError::InvalidConfig(
                "request arguments must be a JSON object".into(),
            ));
        }
        for capability in request
            .required_capabilities
            .iter()
            .chain(std::iter::once(&request.operation))
        {
            if !self.config.capabilities.contains(capability) {
                return Err(AdapterError::DeniedCapability {
                    capability: capability.clone(),
                    reason: "capability is outside the engine allowlist".into(),
                });
            }
        }
        Ok(())
    }

    async fn run_session(
        &self,
        request: &AdapterRequest,
        effective_timeout: Duration,
        stdin: &mut ChildStdin,
        reader: &mut BufReader<ChildStdout>,
        cancellation: &mut watch::Receiver<bool>,
    ) -> Result<AdapterResult, AdapterError> {
        let initialize_id = format!("{}:initialize", request.id);
        write_frame(
            stdin,
            &json!({
                "schema_version": ADAPTER_SCHEMA_VERSION,
                "id": initialize_id,
                "type": "initialize",
                "protocol_version": "1",
                "project_root": self.config.project_root,
                "allowed_capabilities": self.config.capabilities,
            }),
            self.config.max_line_bytes,
        )
        .await?;
        let initialized = read_frame(reader, self.config.max_line_bytes, cancellation).await?;
        expect_response(&initialized, "initialized", &initialize_id)?;
        let advertised = string_set(&initialized, "capabilities")?;
        if !advertised.is_subset(&self.config.capabilities) {
            return Err(AdapterError::Protocol(
                "adapter advertised a capability outside the allowlist".into(),
            ));
        }
        for capability in request
            .required_capabilities
            .iter()
            .chain(std::iter::once(&request.operation))
        {
            if !advertised.contains(capability) {
                return Err(AdapterError::DeniedCapability {
                    capability: capability.clone(),
                    reason: "adapter did not advertise the required capability".into(),
                });
            }
        }

        let health_id = format!("{}:health", request.id);
        write_frame(
            stdin,
            &json!({
                "schema_version": ADAPTER_SCHEMA_VERSION,
                "id": health_id,
                "type": "health",
            }),
            self.config.max_line_bytes,
        )
        .await?;
        let health = read_frame(reader, self.config.max_line_bytes, cancellation).await?;
        expect_response(&health, "health_status", &health_id)?;
        if health.get("status").and_then(Value::as_str) != Some("ready") {
            return Err(AdapterError::Protocol(
                "adapter health status is not ready".into(),
            ));
        }

        let timeout_ms = u64::try_from(effective_timeout.as_millis()).unwrap_or(u64::MAX);
        let mut wire_request = json!({
            "schema_version": ADAPTER_SCHEMA_VERSION,
            "id": request.id,
            "type": "request",
            "operation": request.operation,
            "arguments": request.arguments,
            "timeout_ms": timeout_ms.max(1),
            "required_capabilities": request.required_capabilities,
            "idempotency_key": request.idempotency_key,
        });
        if let Some(token) = &request.recovery_token {
            wire_request["recovery_token"] = Value::String(token.clone());
        }
        write_frame(stdin, &wire_request, self.config.max_line_bytes).await?;

        let mut progress_sequence = 0_u64;
        loop {
            let frame = read_frame(reader, self.config.max_line_bytes, cancellation).await?;
            expect_common(&frame)?;
            let kind = frame
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let correlation = frame.get("request_id").and_then(Value::as_str);
            if correlation != Some(request.id.as_str()) {
                return Err(AdapterError::Protocol(
                    "adapter response correlation mismatch".into(),
                ));
            }
            match kind {
                "progress" => {
                    let sequence =
                        frame
                            .get("sequence")
                            .and_then(Value::as_u64)
                            .ok_or_else(|| {
                                AdapterError::Protocol("invalid progress sequence".into())
                            })?;
                    if sequence != progress_sequence + 1 {
                        return Err(AdapterError::Protocol(
                            "progress sequence must be strictly contiguous".into(),
                        ));
                    }
                    progress_sequence = sequence;
                }
                "result" => return self.parse_result(frame),
                "denied_capability" => {
                    return Err(AdapterError::DeniedCapability {
                        capability: required_string(&frame, "capability")?.to_string(),
                        reason: required_string(&frame, "reason")?.to_string(),
                    });
                }
                "error" => {
                    return Err(AdapterError::Protocol(format!(
                        "adapter error {}: {}",
                        required_string(&frame, "code")?,
                        required_string(&frame, "message")?
                    )));
                }
                other => {
                    return Err(AdapterError::Protocol(format!(
                        "unexpected adapter message type {other:?}"
                    )));
                }
            }
        }
    }

    fn parse_result(&self, frame: Value) -> Result<AdapterResult, AdapterError> {
        let status: AdapterStatus = serde_json::from_value(
            frame
                .get("status")
                .cloned()
                .ok_or_else(|| AdapterError::Protocol("result is missing status".into()))?,
        )
        .map_err(|error| AdapterError::Protocol(format!("invalid result status: {error}")))?;
        let provenance: AdapterProvenance = serde_json::from_value(
            frame
                .get("provenance")
                .cloned()
                .ok_or_else(|| AdapterError::Protocol("result is missing provenance".into()))?,
        )
        .map_err(|error| AdapterError::Protocol(format!("invalid provenance: {error}")))?;
        let provenance_cwd = provenance
            .cwd
            .canonicalize()
            .map_err(|_| AdapterError::Protocol("provenance cwd does not exist".into()))?;
        if provenance_cwd != self.config.cwd {
            return Err(AdapterError::Protocol(
                "provenance cwd does not match configured cwd".into(),
            ));
        }
        let digest = provenance.request_digest.strip_prefix("sha256:");
        if !matches!(digest, Some(value) if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
        {
            return Err(AdapterError::Protocol(
                "provenance request digest is not canonical SHA-256".into(),
            ));
        }
        Ok(AdapterResult {
            status,
            output: frame.get("output").cloned().unwrap_or(Value::Null),
            provenance,
            recovery_token: frame
                .get("recovery_token")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }
}

async fn write_frame(
    stdin: &mut ChildStdin,
    frame: &Value,
    max_line_bytes: usize,
) -> Result<(), AdapterError> {
    let mut encoded = serde_json::to_vec(frame)
        .map_err(|error| AdapterError::Protocol(format!("cannot encode message: {error}")))?;
    encoded.push(b'\n');
    if encoded.len() > max_line_bytes {
        return Err(AdapterError::Protocol(
            "outbound adapter frame exceeds line limit".into(),
        ));
    }
    stdin.write_all(&encoded).await.map_err(AdapterError::Io)?;
    stdin.flush().await.map_err(AdapterError::Io)
}

async fn read_frame(
    reader: &mut BufReader<ChildStdout>,
    max_line_bytes: usize,
    cancellation: &mut watch::Receiver<bool>,
) -> Result<Value, AdapterError> {
    if *cancellation.borrow() {
        return Err(AdapterError::Cancelled);
    }
    let mut bytes = Vec::new();
    let read = tokio::select! {
        changed = cancellation.changed() => {
            if changed.is_ok() && *cancellation.borrow() {
                return Err(AdapterError::Cancelled);
            }
            return Err(AdapterError::Cancelled);
        }
        read = async {
            let mut limited = reader.take((max_line_bytes + 1) as u64);
            limited.read_until(b'\n', &mut bytes).await
        } => read.map_err(AdapterError::Io)?,
    };
    if read == 0 {
        return Err(AdapterError::Protocol(
            "adapter exited before a terminal response".into(),
        ));
    }
    if bytes.len() > max_line_bytes {
        return Err(AdapterError::Protocol(
            "adapter frame exceeds line limit".into(),
        ));
    }
    if bytes.last() != Some(&b'\n') {
        return Err(AdapterError::Protocol(
            "adapter emitted a truncated JSON line".into(),
        ));
    }
    bytes.pop();
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| AdapterError::Protocol(format!("adapter emitted invalid JSON: {error}")))
}

fn expect_common(frame: &Value) -> Result<(), AdapterError> {
    if frame.get("schema_version").and_then(Value::as_str) != Some(ADAPTER_SCHEMA_VERSION) {
        return Err(AdapterError::Protocol(
            "adapter response has unsupported schema version".into(),
        ));
    }
    if frame
        .get("id")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(AdapterError::Protocol(
            "adapter response has no non-empty id".into(),
        ));
    }
    Ok(())
}

fn expect_response(frame: &Value, kind: &str, request_id: &str) -> Result<(), AdapterError> {
    expect_common(frame)?;
    if frame.get("type").and_then(Value::as_str) == Some("error") {
        return Err(AdapterError::Protocol(format!(
            "adapter error {}: {}",
            required_string(frame, "code")?,
            required_string(frame, "message")?
        )));
    }
    if frame.get("type").and_then(Value::as_str) != Some(kind)
        || frame.get("request_id").and_then(Value::as_str) != Some(request_id)
    {
        return Err(AdapterError::Protocol(format!(
            "expected correlated {kind} response"
        )));
    }
    Ok(())
}

fn string_set(frame: &Value, field: &str) -> Result<BTreeSet<String>, AdapterError> {
    let values = frame
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::Protocol(format!("{field} must be an array")))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    AdapterError::Protocol(format!("{field} entries must be non-empty strings"))
                })
        })
        .collect()
}

fn required_string<'a>(frame: &'a Value, field: &str) -> Result<&'a str, AdapterError> {
    frame
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AdapterError::Protocol(format!("missing non-empty {field}")))
}

async fn reap_process(child: &mut Child, grace: Duration) {
    if matches!(tokio::time::timeout(grace, child.wait()).await, Ok(Ok(_))) {
        return;
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
}
