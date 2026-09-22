use super::*;
mod admission;
mod admission_restart;
use axum::{routing::post, Json, Router};
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn retained_admission_budget_is_environment_isolated() {
    let root = TempDir::new().unwrap();
    let child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "research_delivery::retained_admission_budget_child",
            "--ignored",
            "--nocapture",
        ])
        .env("ARDA_RETAINED_BUDGET_FIXTURE", "1")
        .env_remove("ARDA_HERMES_ADAPTER_CONFIG")
        .env(
            "ARDA_HERMES_RETAINED_ADAPTER_CONFIG",
            root.path().join("retained.toml"),
        )
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(std::time::Duration::from_secs(60), child)
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("retained-budget-child-completed"));
}

#[tokio::test]
#[ignore = "subprocess-only retained configuration fixture"]
async fn retained_admission_budget_child() {
    assert_eq!(std::env::var("ARDA_RETAINED_BUDGET_FIXTURE").unwrap(), "1");
    run_publication_fixture(false, "execute").await;
    println!("retained-budget-child-completed");
}

#[tokio::test]
async fn authenticated_research_publishes_and_replays_without_discovery() {
    for stage in ["execute", "verify", "review"] {
        run_publication_fixture(false, stage).await;
    }
}

#[tokio::test]
#[ignore = "explicit live HTTPS capture; discovery is a fixture and storage is isolated"]
async fn live_public_https_capture_publishes_and_recovers() {
    run_publication_fixture(true, "execute").await;
}

async fn run_publication_fixture(real_public_source: bool, crash_stage: &str) {
    let root = TempDir::new().unwrap();
    let discoveries = Arc::new(AtomicUsize::new(0));
    let count = discoveries.clone();
    let app = Router::new()
        .route("/suggestions", post(|Json(suggestion): Json<Value>| async move {
            Json(serde_json::json!({"status":"accepted", "suggestion":suggestion}))
        }))
        .route("/discover", post(move || {let count=count.clone(); async move {
            count.fetch_add(1, Ordering::SeqCst);
            let results = if real_public_source { serde_json::json!([{"title":"Example Domain", "url":"https://example.com/"}]) } else { serde_json::json!([]) };
            Json(serde_json::json!({"report":{"provider":"fixture", "results":results},"memory":{"memory_id":null}}))
        }}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let (bound, shutdown, handle) =
        start_research_harness(&root, Some(format!("http://{address}"))).await;
    let client = gateway_client();
    let body = gateway_message(
        "question-brief",
        "arda research source-backed local testing",
    );
    let first: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(discoveries.load(Ordering::SeqCst), 1);
    assert!(first["summary"].as_str().unwrap().contains("No commitment"));
    let files: Vec<_> = fs::read_dir(root.path().join("data/workbench/research/briefs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let raw = fs::read(&files[0]).unwrap();
    let brief: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(brief["subject"]["kind"], "question");
    assert_eq!(brief["subject"]["owner"], "discord-user-1");
    assert_eq!(brief["execution_authorized"], false);
    if real_public_source {
        assert_eq!(
            brief["citations"].as_array().unwrap().len(),
            1,
            "failures: {}",
            brief["source_failures"]
        );
        assert!(brief["citations"][0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("Example Domain"));
        assert!(brief["citations"][0]["content_sha256"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
    }
    assert!(brief.get("run_id").is_none());
    assert!(brief.get("objective_id").is_none());
    // Emulate the crash boundary after publication but before response journal
    // completion. Restart must reconcile the existing brief without discovery.
    let operation = fs::read_dir(root.path().join("data/workbench/research/operations"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .unwrap();
    let mut record: Value = serde_json::from_slice(&fs::read(&operation).unwrap()).unwrap();
    record["response"] = Value::Null;
    record["phase"] = "executing".into();
    fs::write(&operation, serde_json::to_vec(&record).unwrap()).unwrap();
    shutdown.notify_waiters();
    handle.await.unwrap();
    let (bound, shutdown, handle) = start_research_harness(&root, None).await;
    let replay: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(replay, first);
    assert_eq!(fs::read(&files[0]).unwrap(), raw);
    let read: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message(
            "read-question-brief",
            &format!(
                "arda research-result {}",
                brief["brief_id"].as_str().unwrap()
            ),
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(read["summary"].as_str().unwrap().contains("No commitment"));
    let research_refs = |value: &Value| {
        value["evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v.as_str().unwrap().starts_with("arda://research/"))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(research_refs(&read), research_refs(&first));
    let command = format!(
        "arda research-result {}",
        brief["brief_id"].as_str().unwrap()
    );
    let mut public = gateway_message("public-question-brief", &command);
    public["event"]["source"]["chat_type"] = "group".into();
    assert_eq!(
        client
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&public)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    let mut tampered = brief.clone();
    tampered["summary"] = "forged research".into();
    fs::write(&files[0], serde_json::to_vec(&tampered).unwrap()).unwrap();
    let tampered_read = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway_message("tampered-question-brief", &command))
        .send()
        .await
        .unwrap();
    assert_eq!(
        tampered_read.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(tampered_read
        .text()
        .await
        .unwrap()
        .contains("integrity check failed"));
    assert_eq!(discoveries.load(Ordering::SeqCst), 1);
    assert!(!root.path().join("data/runs").exists());
    if !real_public_source {
        admission::exercise(&root, bound, &client, &brief, crash_stage).await;
    }
    shutdown.notify_waiters();
    handle.await.unwrap();
    server.abort();
    let _ = server.await;
}
