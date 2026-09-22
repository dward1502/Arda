//! Real child-process interruption of gateway-admitted fixture state.
use super::*;
use arda_engine::objectives::{LeafExecution, WorkbenchLeafExecution};

#[tokio::test]
async fn admitted_stage_crash_child() {
    let Some(root) = std::env::var_os("ARDA_ADMITTED_CRASH_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let stage = std::env::var("ARDA_ADMITTED_CRASH_STAGE").unwrap();
    assert!(["execute", "verify", "review"].contains(&stage.as_str()));
    let store = ObjectiveStore::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    let claim = store
        .claim_runnable("admitted-crash-worker", now, 100, 1)
        .unwrap()
        .remove(0);
    let crash_path = format!(
        "/v1/runs/{}/nodes/{stage}/execute-provider",
        claim.execution_run_id.as_deref().unwrap()
    );
    let (upstream, _, _) = crate::start_research_harness_at(&root, None).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bound = listener.local_addr().unwrap();
    let router = axum::Router::new().fallback(move |request: axum::extract::Request| {
        let crash_path = crash_path.clone();
        async move {
            let (parts, body) = request.into_parts();
            let path = parts.uri.to_string();
            let body = axum::body::to_bytes(body, 8 * 1024 * 1024).await.unwrap();
            let response = reqwest::Client::new()
                .request(parts.method, format!("http://{upstream}{path}"))
                .header("content-type", "application/json")
                .body(body)
                .send()
                .await
                .unwrap();
            let status = response.status();
            let body = response.bytes().await.unwrap();
            if status.is_success() && path == crash_path {
                std::process::exit(75);
            }
            axum::http::Response::builder()
                .status(status)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body))
                .unwrap()
        }
    });
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    fs::write(
        root.join("admitted-initial-claim.json"),
        serde_json::to_vec(&claim).unwrap(),
    )
    .unwrap();
    let executor = WorkbenchLeafExecution::with_adapter(&root,
        arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter::with_harness_url(&root, format!("http://{bound}")).unwrap());
    let outcome = executor.execute(claim).await;
    panic!("crash boundary not reached: {outcome:?}");
}

pub(super) async fn interrupt(root: &TempDir, stage: &str) -> arda_engine::objectives::ClaimedLeaf {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "research_delivery::admission_restart::admitted_stage_crash_child",
            "--nocapture",
        ])
        .env("ARDA_ADMITTED_CRASH_ROOT", root.path())
        .env("ARDA_ADMITTED_CRASH_STAGE", stage)
        .env_remove("ARDA_HERMES_ADAPTER_CONFIG")
        .env_remove("ARDA_HERMES_RETAINED_ADAPTER_CONFIG")
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(30), command.output())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(75),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&fs::read(root.path().join("admitted-initial-claim.json")).unwrap())
        .unwrap()
}
