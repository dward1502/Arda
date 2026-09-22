use super::*;
use arda_engine::supervisor::Shutdown;
use std::{os::unix::fs::PermissionsExt, time::Duration};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn provider_waiter_loss_keeps_owner_and_exact_start_error() {
    exercise_waiter_loss(false).await;
}

#[tokio::test]
async fn provider_waiter_loss_shutdown_drains_live_child() {
    exercise_waiter_loss(true).await;
}

async fn exercise_waiter_loss(stop_while_running: bool) {
    let root = TempDir::new().unwrap();
    let socket = root.path().join("provider.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let executable = root.path().join("blocked-provider");
    fs::write(&executable, format!("#!/usr/bin/python3\nimport socket,sys\ns=socket.socket(socket.AF_UNIX)\ns.connect('{}')\ns.recv(1)\nsys.exit(71)\n", socket.display())).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    write_file_only_hermes_config(&root);
    let config = root.path().join("config/adapters/hermes-workbench.toml");
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("/bin/true", executable.to_str().unwrap())
            .replace("max_timeout_ms = 1000", "max_timeout_ms = 30000"),
    )
    .unwrap();
    let shutdown = Shutdown::new();
    let (bound, server) = arda_engine::harness::serve_with_shutdown(
        Some("127.0.0.1:0".parse().unwrap()),
        harness_state(&root),
        shutdown.clone(),
    )
    .await
    .unwrap();
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    let id = "run-waiter-loss";
    let mut work = graph(id, "inspect", "inspect");
    work["nodes"][0]["timeout_ms"] = json!(30000);
    work["nodes"][0]["parent_receipts"] = json!(["receipt:approval"]);
    work["nodes"]
        .as_array_mut()
        .unwrap()
        .push(graph(id, "approval", "approval")["nodes"][0].clone());
    work["nodes"][1]["idempotency_key"] = json!("waiter-approval");
    work["edges"] = json!([{"id":"approval-inspect","from":"approval","to":"inspect","parent_receipt":"receipt:approval"}]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({"project_id":PROJECT_ID,"graph":work,"envelope":envelope("waiter-plan")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("http://{bound}/v1/runs/{id}/approve"))
        .json(&json!({"node_id":"approval","envelope":envelope("waiter-approve")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let url = format!("http://{bound}/v1/runs/{id}/nodes/inspect/execute-provider");
    let body = json!({"envelope":envelope("waiter-execute"),"objective":"inspect fixture"});
    let request = {
        let client = client.clone();
        let url = url.clone();
        let body = body.clone();
        tokio::spawn(async move { client.post(url).json(&body).send().await })
    };
    let (mut worker, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    if stop_while_running {
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        use tokio::io::AsyncReadExt;
        let mut byte = [0];
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), worker.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        return;
    }
    // A duplicate may only subscribe, not conflict or launch another provider.
    let repeat = client.post(&url).json(&body).send();
    tokio::pin!(repeat);
    let early = tokio::time::timeout(Duration::from_millis(150), &mut repeat).await;
    // Release before asserting so the real child and server can be drained on RED.
    let _ = worker.write_all(b"x").await;
    let response = match early {
        Ok(r) => r.unwrap(),
        Err(_) => tokio::time::timeout(Duration::from_secs(5), &mut repeat)
            .await
            .unwrap()
            .unwrap(),
    };
    let status = response.status();
    let response_body = response.text().await.unwrap();
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    assert!(!status.is_success());
    assert!(
        response_body.contains("71"),
        "exact child error lost: {status} {response_body}"
    );
    let events =
        fs::read_to_string(root.path().join(format!("data/runs/{id}/events.jsonl"))).unwrap();
    let keys: Vec<String> = events
        .lines()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["idempotency_key"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        keys.iter()
            .filter(|k| k.contains(":provider-running:"))
            .count(),
        1
    );
    assert_eq!(
        keys.iter()
            .filter(|k| k.contains(":provider-error:"))
            .count(),
        1
    );
}
