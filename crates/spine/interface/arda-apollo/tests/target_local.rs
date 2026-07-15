use annunimas_apollo::{ApolloService, ExecutionPriority, ExecutionRequest};
use anyhow::Result;
use serde_json::json;

fn request(task_id: &str) -> ExecutionRequest {
    ExecutionRequest {
        task_id: task_id.to_string(),
        agent_id: "apollo-target-local".to_string(),
        payload: json!({"op": "target-local-audit"}),
        priority: ExecutionPriority::Normal,
        timeout_secs: 30,
    }
}

#[tokio::test]
async fn status_uses_target_local_home_without_workspace_state() -> Result<()> {
    let home = tempfile::tempdir()?;
    let service = ApolloService::from_home(home.path())?;

    let paths = service.runtime_paths();
    assert_eq!(paths.home, home.path().to_string_lossy());
    assert!(paths.status_path.ends_with("runtime_status.json"));
    assert!(paths.requests_path.ends_with("pending_requests.json"));

    let status = service.status().await?;
    assert_eq!(status["schema_version"], "annunimas.apollo.runtime.v1");
    assert_eq!(status["authority"], "apollo_service");
    assert!(home.path().join("runtime_status.json").exists());
    assert!(!home.path().join("pending_requests.json").exists());

    Ok(())
}

#[tokio::test]
async fn pending_request_persists_and_reloads_from_target_local_home() -> Result<()> {
    let home = tempfile::tempdir()?;
    let service = ApolloService::from_home(home.path())?;
    let task_id = service.submit(request("task_target_local_apollo")).await?;

    let pending_path = home.path().join("pending_requests.json");
    assert!(pending_path.exists());
    assert!(std::fs::read_to_string(&pending_path)?.contains(&task_id));

    let restarted = ApolloService::from_home(home.path())?;
    let status = restarted.status().await?;
    assert_eq!(status["executor"]["queue"]["depth"], 1);
    assert_eq!(status["executor"]["queue"]["pending_tasks"][0], task_id);

    let result = restarted.execute(&task_id).await?;
    assert!(result.is_some());

    let final_status = restarted.status().await?;
    assert_eq!(final_status["executor"]["queue"]["depth"], 0);
    assert_eq!(final_status["executor"]["summary"]["completed_total"], 1);

    Ok(())
}
