use arda_engine::adapters::{
    AdapterCancellation, AdapterError, AdapterProcessConfig, AdapterRequest, AdapterStatus,
    JsonlAdapter,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

fn python_executable() -> PathBuf {
    std::fs::canonicalize("/usr/bin/python3").expect("python3 executable")
}

fn sdk_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sdk/python")
        .canonicalize()
        .expect("python sdk root")
}

fn config(root: &TempDir, timeout: Duration) -> AdapterProcessConfig {
    let mut environment = BTreeMap::new();
    environment.insert("PYTHONPATH".to_string(), sdk_root().display().to_string());
    AdapterProcessConfig {
        executable: python_executable(),
        args: vec![
            "-u".into(),
            "-m".into(),
            "arda_project_adapter.server".into(),
        ],
        expected_adapter: "arda-python-reference".into(),
        expected_adapter_version: "1.0.0".into(),
        project_root: root.path().to_path_buf(),
        cwd: root.path().to_path_buf(),
        environment,
        environment_allowlist: BTreeSet::from(["PYTHONPATH".to_string()]),
        capabilities: BTreeSet::from([
            "echo".to_string(),
            "inspect".to_string(),
            "progress".to_string(),
            "sleep".to_string(),
        ]),
        timeout,
        cancellation_grace: Duration::from_millis(100),
        max_line_bytes: 64 * 1024,
    }
}

fn request(id: &str, operation: &str, arguments: serde_json::Value) -> AdapterRequest {
    AdapterRequest {
        id: id.to_string(),
        operation: operation.to_string(),
        arguments,
        timeout: Duration::from_secs(5),
        required_capabilities: BTreeSet::from([operation.to_string()]),
        idempotency_key: format!("idempotency-{id}"),
        recovery_token: None,
    }
}

fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn read_pid(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .expect("adapter pid file")
        .trim()
        .parse()
        .expect("numeric adapter pid")
}

#[tokio::test]
async fn adapter_rejects_impersonated_or_downgraded_handshake_identity() {
    let root = TempDir::new().expect("project root");
    let mut adapter_config = config(&root, Duration::from_secs(1));
    adapter_config.args = vec![
        "-u".into(),
        "-c".into(),
        r#"import json,sys
message=json.loads(sys.stdin.readline())
print(json.dumps({"schema_version":"arda.project-adapter.v1","id":"bad:initialized","type":"initialized","request_id":message["id"],"adapter":"impersonator","adapter_version":"0.1.0","capabilities":["echo"],"recovery_supported":False}),flush=True)"#.into(),
    ];
    let adapter = JsonlAdapter::new(adapter_config).expect("valid process config");

    let error = adapter
        .execute(
            request("identity-mismatch", "echo", json!({})),
            AdapterCancellation::new(),
        )
        .await
        .expect_err("unexpected adapter identity must fail closed");

    assert!(
        matches!(error, AdapterError::Protocol(ref message) if message.contains("adapter identity mismatch")),
        "expected an adapter identity protocol error, got {error:?}"
    );
}

#[tokio::test]
async fn adapter_enforces_cwd_and_environment_allowlist() {
    let root = TempDir::new().expect("project root");
    let mut adapter_config = config(&root, Duration::from_secs(2));
    adapter_config
        .environment
        .insert("ARDA_ALLOWED_TEST".into(), "visible".into());
    adapter_config
        .environment_allowlist
        .insert("ARDA_ALLOWED_TEST".into());
    let adapter = JsonlAdapter::new(adapter_config).expect("valid adapter config");

    let result = adapter
        .execute(
            request(
                "inspect-1",
                "inspect",
                json!({"environment": ["ARDA_ALLOWED_TEST", "HOME"]}),
            ),
            AdapterCancellation::new(),
        )
        .await
        .expect("adapter result");

    assert_eq!(result.status, AdapterStatus::Succeeded);
    assert_eq!(result.output["cwd"], root.path().display().to_string());
    assert_eq!(result.output["environment"]["ARDA_ALLOWED_TEST"], "visible");
    assert!(result.output["environment"].get("HOME").is_none());
    assert_eq!(result.provenance.cwd, root.path());
}

#[test]
fn adapter_rejects_executable_cwd_and_environment_boundary_violations() {
    let root = TempDir::new().expect("project root");

    let mut invalid_executable = config(&root, Duration::from_secs(1));
    invalid_executable.executable = PathBuf::from("python3");
    assert!(matches!(
        JsonlAdapter::new(invalid_executable),
        Err(AdapterError::ExecutableNotAbsolute(_))
    ));

    let outside = TempDir::new().expect("outside cwd");
    let mut invalid_cwd = config(&root, Duration::from_secs(1));
    invalid_cwd.cwd = outside.path().to_path_buf();
    assert!(matches!(
        JsonlAdapter::new(invalid_cwd),
        Err(AdapterError::CwdOutsideProject { .. })
    ));

    let mut invalid_environment = config(&root, Duration::from_secs(1));
    invalid_environment
        .environment
        .insert("HOME".into(), "/should/not/leak".into());
    assert!(matches!(
        JsonlAdapter::new(invalid_environment),
        Err(AdapterError::EnvironmentDenied(key)) if key == "HOME"
    ));
}

#[tokio::test]
async fn adapter_timeout_terminates_and_reaps_process() {
    let root = TempDir::new().expect("project root");
    let pid_path = root.path().join("timeout.pid");
    let mut adapter_config = config(&root, Duration::from_millis(150));
    adapter_config.environment.insert(
        "ARDA_ADAPTER_PID_FILE".into(),
        pid_path.display().to_string(),
    );
    adapter_config
        .environment_allowlist
        .insert("ARDA_ADAPTER_PID_FILE".into());
    let adapter = JsonlAdapter::new(adapter_config).expect("valid adapter config");

    let error = adapter
        .execute(
            request("sleep-timeout", "sleep", json!({"seconds": 5})),
            AdapterCancellation::new(),
        )
        .await
        .expect_err("sleep must time out");

    assert!(matches!(error, AdapterError::Timeout));
    let pid = read_pid(&pid_path);
    assert!(
        !process_is_alive(pid),
        "timed-out adapter pid {pid} survived"
    );
}

#[tokio::test]
async fn adapter_crash_is_a_typed_failure_without_false_completion() {
    let root = TempDir::new().expect("project root");
    let mut adapter_config = config(&root, Duration::from_secs(1));
    adapter_config.args = vec!["-u".into(), "-c".into(), "raise SystemExit(17)".into()];
    let adapter = JsonlAdapter::new(adapter_config).expect("valid adapter config");

    let error = adapter
        .execute(
            request("adapter-crash", "echo", json!({"value": "safe"})),
            AdapterCancellation::new(),
        )
        .await
        .expect_err("crashed adapter must not complete");

    assert!(matches!(
        error,
        AdapterError::Protocol(_) | AdapterError::Io(_)
    ));
}

#[tokio::test]
async fn adapter_cancellation_terminates_and_reaps_process() {
    let root = TempDir::new().expect("project root");
    let pid_path = root.path().join("cancel.pid");
    let mut adapter_config = config(&root, Duration::from_secs(3));
    adapter_config.environment.insert(
        "ARDA_ADAPTER_PID_FILE".into(),
        pid_path.display().to_string(),
    );
    adapter_config
        .environment_allowlist
        .insert("ARDA_ADAPTER_PID_FILE".into());
    let adapter = JsonlAdapter::new(adapter_config).expect("valid adapter config");
    let cancellation = AdapterCancellation::new();
    let trigger = cancellation.clone();
    let cancel_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        trigger.cancel();
    });

    let error = adapter
        .execute(
            request("sleep-cancel", "sleep", json!({"seconds": 5})),
            cancellation,
        )
        .await
        .expect_err("sleep must be cancelled");
    cancel_task.await.expect("cancellation task");

    assert!(matches!(error, AdapterError::Cancelled));
    let pid = read_pid(&pid_path);
    assert!(
        !process_is_alive(pid),
        "cancelled adapter pid {pid} survived"
    );
}

#[tokio::test]
async fn adapter_rejects_unevaluated_protocol_fields() {
    let root = TempDir::new().expect("project root");
    let mut adapter_config = config(&root, Duration::from_secs(1));
    adapter_config.args = vec![
        "-u".into(),
        "-c".into(),
        r#"import json,sys
message=json.loads(sys.stdin.readline())
print(json.dumps({"schema_version":"arda.project-adapter.v1","id":"bad:initialized","type":"initialized","request_id":message["id"],"adapter":"bad","adapter_version":"1","capabilities":["echo"],"recovery_supported":False,"unexpected":True}),flush=True)"#.into(),
    ];
    let adapter = JsonlAdapter::new(adapter_config).expect("valid process config");

    let error = adapter
        .execute(
            request("invalid-frame", "echo", json!({})),
            AdapterCancellation::new(),
        )
        .await
        .expect_err("unevaluated fields must fail closed");

    assert!(
        matches!(error, AdapterError::Protocol(ref message) if message.contains("unknown fields")),
        "expected an unknown-fields protocol error, got {error:?}"
    );
}

#[tokio::test]
async fn adapter_rejects_oversized_noisy_output_without_unbounded_buffering() {
    let root = TempDir::new().expect("project root");
    let mut adapter_config = config(&root, Duration::from_secs(1));
    adapter_config.max_line_bytes = 256;
    adapter_config.args = vec![
        "-u".into(),
        "-c".into(),
        "import sys; sys.stdout.write('x' * 257 + '\\n'); sys.stdout.flush()".into(),
    ];
    let adapter = JsonlAdapter::new(adapter_config).expect("valid process config");

    let error = adapter
        .execute(
            request("oversized-frame", "echo", json!({})),
            AdapterCancellation::new(),
        )
        .await
        .expect_err("oversized adapter output must fail closed");

    assert!(
        matches!(error, AdapterError::Protocol(ref message) if message.contains("exceeds line limit")),
        "expected a bounded line-limit error, got {error:?}"
    );
}
