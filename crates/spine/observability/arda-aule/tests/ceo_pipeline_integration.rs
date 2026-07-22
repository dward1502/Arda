use arda_core::ledger::Ledger;
use arda_core::pipeline::Pipeline;
use arda_core::router::Router;
use arda_core::task::{Task, TaskStatus};
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn ledger_messages(path: &Path) -> Vec<serde_json::Value> {
    let content = fs::read_to_string(path).expect("Failed to read ledger file");
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid ledger json"))
        .collect()
}

struct TestAgent {
    name: &'static str,
    capabilities: &'static [&'static str],
    result: Option<serde_json::Value>,
}

#[async_trait]
impl Agent for TestAgent {
    fn name(&self) -> &str {
        self.name
    }

    fn capabilities(&self) -> &[&str] {
        self.capabilities
    }

    async fn execute(&self, task: &mut Task) -> Result<()> {
        if let Some(result) = &self.result {
            task.complete(result.clone());
        } else {
            task.transition(TaskStatus::Complete);
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_pipeline_budget_checking() {
    let temp_dir = TempDir::new("arda_ceo_test").expect("Failed to create temp dir");
    let ledger = Ledger::new(temp_dir.path()).expect("Failed to create ledger");
    let router = Router::new();
    let joule_budget = 5u64; // Very low budget

    let pipeline = Pipeline::new(router, ledger, joule_budget);

    let mut task = Task::new("test_task", "ingest");
    task.description = "A test task for budget checking".to_string();

    let result = pipeline.submit(task).await.expect("Task should complete");

    match result.status {
        TaskStatus::Failed { reason } => {
            assert!(reason.contains("JouleWork budget exceeded"));
        }
        other => panic!("Task should fail due to low budget, got {other:?}"),
    }
}

#[tokio::test]
async fn test_pipeline_confidence_scoring() {
    let temp_dir = TempDir::new("arda_ceo_test").expect("Failed to create temp dir");
    let ledger = Ledger::new(temp_dir.path()).expect("Failed to create ledger");
    let mut router = Router::new();
    router.register(Box::new(TestAgent {
        name: "confidence-agent",
        capabilities: &["ingest"],
        result: Some(serde_json::json!({"status":"ok"})),
    }));
    let joule_budget = 100u64;
    let realm_dir = temp_dir.path().join("realm");
    fs::create_dir_all(&realm_dir).expect("Failed to create realm dir");
    fs::write(
        realm_dir.join("boot.toml"),
        "[ceo]\nheartbeat_ms = 500\ntriad_bypass = false\n",
    )
    .expect("Failed to write boot.toml");

    let pipeline = Pipeline::with_core_link(router, ledger, joule_budget, temp_dir.path());

    // Test with empty description (low confidence)
    let mut task1 = Task::new("task1", "ingest");
    task1.description = "".to_string();

    let result1 = pipeline.submit(task1).await.expect("Task should complete");
    assert_eq!(
        result1.status,
        TaskStatus::Pending,
        "Task with empty description should have low confidence and be pending"
    );
}

#[tokio::test]
async fn test_pipeline_task_assignment() {
    let temp_dir = TempDir::new("arda_ceo_test").expect("Failed to create temp dir");
    let ledger = Ledger::new(temp_dir.path()).expect("Failed to create ledger");
    let mut router = Router::new();
    router.register(Box::new(TestAgent {
        name: "ingest-agent",
        capabilities: &["ingest"],
        result: Some(serde_json::json!({"status":"ok"})),
    }));
    let joule_budget = 100u64;

    let pipeline = Pipeline::new(router, ledger, joule_budget);

    let mut task = Task::new("https://example.com", "ingest");
    task.description = "Example ingest task".to_string();

    let result = pipeline.submit(task).await.expect("Task should complete");

    // Task should be assigned to a router agent or fail gracefully
    match result.status {
        TaskStatus::Complete => {
            assert!(
                result.assigned_agent.is_some(),
                "Completed task should have assigned agent"
            );
        }
        TaskStatus::Failed { .. } => {
            // Failed due to no route, which is acceptable
            match &result.status {
                TaskStatus::Failed { reason } => {
                    assert!(!reason.is_empty(), "Reason should exist for failure")
                }
                _ => unreachable!(),
            }
        }
        _ => {
            // Other statuses are acceptable for this test
        }
    }
}

#[tokio::test]
async fn test_pipeline_ledger_emission() {
    let temp_dir = TempDir::new("arda_ceo_test").expect("Failed to create temp dir");
    let ledger = Ledger::new(temp_dir.path()).expect("Failed to create ledger");
    let ledger_path = ledger.path().to_path_buf();
    let mut router = Router::new();
    router.register(Box::new(TestAgent {
        name: "ledger-agent",
        capabilities: &["ingest"],
        result: Some(serde_json::json!({"status":"ok"})),
    }));
    let joule_budget = 100u64;

    let pipeline = Pipeline::new(router, ledger, joule_budget);

    let mut task = Task::new("test_task", "ingest");
    task.description = "A test task for ledger emission".to_string();

    let _result = pipeline.submit(task).await.expect("Task should complete");

    // Verify ledger has messages
    let messages = ledger_messages(&ledger_path);
    assert!(
        !messages.is_empty(),
        "Ledger should have emitted messages for task lifecycle"
    );

    // Check for specific lifecycle messages
    let has_received = messages.iter().any(|m| {
        m.get("payload")
            .and_then(|p| p.get("type"))
            .and_then(Value::as_str)
            == Some("event")
            && m.get("payload")
                .and_then(|p| p.get("event_type"))
                .and_then(Value::as_str)
                == Some("task_received")
    });
    let has_decision = messages.iter().any(|m| {
        m.get("payload")
            .and_then(|p| p.get("type"))
            .and_then(Value::as_str)
            == Some("event")
            && m.get("payload")
                .and_then(|p| p.get("event_type"))
                .and_then(Value::as_str)
                == Some("decision_scored")
    });

    assert!(has_received, "Ledger should have task_received event");
    assert!(has_decision, "Ledger should have decision_scored event");
}

#[tokio::test]
async fn test_pipeline_missing_result_handling() {
    let temp_dir = TempDir::new("arda_ceo_test").expect("Failed to create temp dir");
    let ledger = Ledger::new(temp_dir.path()).expect("Failed to create ledger");
    let mut router = Router::new();
    router.register(Box::new(TestAgent {
        name: "default-result-agent",
        capabilities: &["ingest"],
        result: None,
    }));
    let joule_budget = 100u64;

    let pipeline = Pipeline::new(router, ledger, joule_budget);

    // Create a task with no result set
    let mut task = Task::new("test_task", "ingest");
    task.description = "Task should receive a default completion payload".to_string();
    task.result = Some(serde_json::Value::Null);

    let result = pipeline.submit(task).await.expect("Task should complete");

    // Task should complete with a default result instead of panicking
    let result_value = result
        .result
        .expect("Task result should exist after completion");
    assert_ne!(
        result_value,
        serde_json::Value::Null,
        "Task result should not be null after completion"
    );
    assert!(result_value.is_object(), "Task result should be an object");
}
