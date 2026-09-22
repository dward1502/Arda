//! C3.1 engineering qualification, not genuine messaging/operator acceptance.
use super::*;
use arda_engine::objectives::{
    ClaimedLeaf, LeafExecution, LeafExecutionResult, ObjectiveRuntime, ObjectiveRuntimeStatus,
};
use arda_engine::supervisor::Shutdown;
use std::{future::Future, pin::Pin, time::Duration};

async fn admit(bound: std::net::SocketAddr) -> (Value, String) {
    let client = gateway_client();
    let contract: Value = serde_json::from_str(include_str!(
        "../../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract, "envelope": mutation_envelope("c31-attach")}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let request = gateway_message(
        "c31-admit",
        &format!("arda objective {PROJECT_ID} C3.1 isolated qualification"),
    );
    let response: Value = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&request)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    (request, created_objective_id(&response).to_owned())
}

fn authority_rows(root: &std::path::Path) -> Vec<Vec<String>> {
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let tables: Vec<String> = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut statement = db
                .prepare(&format!(
                    "SELECT * FROM \"{}\" ORDER BY rowid",
                    table.replace('"', "\"\"")
                ))
                .unwrap();
            let columns = statement.column_count();
            statement
                .query_map([], |row| {
                    Ok((0..columns)
                        .map(|column| format!("{:?}", row.get_ref(column).unwrap()))
                        .collect::<Vec<_>>()
                        .join("|"))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        })
        .collect()
}

#[tokio::test]
async fn gateway_loss_matrix_refuses_intake_control_and_replay_without_mutation() {
    for damage in ["directory", "database", "replacement", "marker"] {
        let root = TempDir::new().unwrap();
        let (bound, shutdown, handle) = start_harness(&root).await;
        let (original, objective_id) = admit(bound).await;
        let task_id =
            ObjectiveStore::open_existing(root.path().join("data/arda/objectives.sqlite3"))
                .unwrap()
                .list_leaves(&objective_id)
                .unwrap()[0]
                .id
                .clone();
        let before = authority_rows(root.path());
        let ledger = root
            .path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl");
        let ledger_before = fs::read(&ledger).unwrap();
        let directory = root.path().join("data/arda");
        let target = match damage {
            "directory" => directory.clone(),
            "marker" => directory.join("objectives.sqlite3.authority.json"),
            _ => directory.join("objectives.sqlite3"),
        };
        let saved = root.path().join("saved-authority");
        fs::rename(&target, &saved).unwrap();
        if damage == "replacement" {
            fs::copy(&saved, &target).unwrap();
        }
        let replacement_before = fs::read(&target).ok();
        for request in [
            original,
            gateway_message(
                "c31-new",
                &format!("arda objective {PROJECT_ID} must not be admitted"),
            ),
            gateway_message(
                "c31-control",
                &format!("arda pause-task {task_id} {objective_id} must not mutate"),
            ),
        ] {
            let response = gateway_client()
                .post(format!("http://{bound}/v1/operator/messages"))
                .json(&request)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 409, "{damage}");
            let body = response.text().await.unwrap();
            // A committed replay may be rejected by the earlier transport fence;
            // fresh intake/control must reach the authority-loss refusal.
            if request["event"]["message_id"] == "c31-admit" {
                assert!(
                    body.contains("duplicate transport event"),
                    "{damage}: {body}"
                );
            } else {
                assert!(body.contains("objective store"), "{damage}: {body}");
            }
            assert_eq!(fs::read(&ledger).unwrap(), ledger_before, "{damage}");
            assert_eq!(fs::read(&target).ok(), replacement_before, "{damage}");
            if damage != "replacement" {
                assert!(!target.exists(), "{damage}");
            }
        }
        if damage == "replacement" {
            fs::remove_file(&target).unwrap();
        }
        fs::rename(saved, target).unwrap();
        assert_eq!(authority_rows(root.path()), before, "{damage}");
        shutdown.notify_waiters();
        handle.await.unwrap();
    }
}

struct NoExecution;
impl LeafExecution for NoExecution {
    fn execute(
        &self,
        _: ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<LeafExecutionResult>> + Send>> {
        panic!("zero-capacity notification fixture must never execute work")
    }
}

async fn idle(status: &mut tokio::sync::watch::Receiver<ObjectiveRuntimeStatus>) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            status.changed().await.unwrap();
            if status.borrow_and_update().phase == "waiting" {
                break;
            }
        }
    })
    .await
    .expect("gateway mutation must wake resident before its one-hour poll");
}

#[tokio::test]
async fn gateway_controls_wake_resident_and_replays_preserve_canonical_rows() {
    let root = TempDir::new().unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let (_, objective_id) = admit(bound).await;
    let store =
        ObjectiveStore::open_existing(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let task_id = store.list_leaves(&objective_id).unwrap()[0].id.clone();
    // Exercise the real run loop and separately reopened HTTP mutation stores.
    // Zero capacity deliberately qualifies notification, not provider execution.
    let mut runtime = ObjectiveRuntime::new(store, NoExecution, "c31-runtime", 0, 60_000);
    let mut status = runtime.subscribe_status();
    let stop = Shutdown::new();
    let runtime_stop = stop.clone();
    let runner = tokio::spawn(async move {
        runtime
            .run_until_shutdown(
                runtime_stop,
                Duration::from_secs(3600),
                Duration::from_secs(1),
            )
            .await
    });
    idle(&mut status).await;
    let url = format!("http://{bound}/v1/operator/messages");
    let mut requests = Vec::new();
    for (index, command) in [
        format!("arda pause-task {task_id} {objective_id} qualification pause"),
        format!("arda resume-task {task_id} {objective_id} qualification resume"),
        format!("arda reprioritize {task_id} {objective_id} high qualification priority"),
        format!("arda approve-objective {task_id} {objective_id} qualification approval"),
        format!("arda cancel-task {task_id} {objective_id} qualification cancel"),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            tokio::time::timeout(Duration::from_millis(100), status.changed())
                .await
                .is_err(),
            "resident must be quiescent before testing a control wakeup"
        );
        let request = gateway_message(&format!("c31-control-{index}"), &command);
        gateway_client()
            .post(&url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        idle(&mut status).await;
        let before = authority_rows(root.path());
        let ledger = root
            .path()
            .join("core/state/orome/operator-session/operator_sessions.jsonl");
        let ledger_before = fs::read(&ledger).unwrap();
        let changed_payload = format!("{command} changed reason");
        for text in [
            command.as_str(),
            changed_payload.as_str(),
            "arda context",
            "arda objectives",
        ] {
            let mut replay = request.clone();
            replay["event"]["text"] = json!(text);
            let response = gateway_client()
                .post(&url)
                .json(&replay)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 409);
            assert_eq!(authority_rows(root.path()), before);
            assert_eq!(fs::read(&ledger).unwrap(), ledger_before);
        }
        requests.push(request);
    }
    stop.trigger();
    tokio::time::timeout(Duration::from_secs(3), runner)
        .await
        .unwrap()
        .unwrap();
    shutdown.notify_waiters();
    handle.await.unwrap();
    let before = authority_rows(root.path());
    let ledger = root
        .path()
        .join("core/state/orome/operator-session/operator_sessions.jsonl");
    let ledger_before = fs::read(&ledger).unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    for request in requests {
        let response = gateway_client()
            .post(format!("http://{bound}/v1/operator/messages"))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 409);
        assert_eq!(authority_rows(root.path()), before);
        assert_eq!(fs::read(&ledger).unwrap(), ledger_before);
    }
    shutdown.notify_waiters();
    handle.await.unwrap();
}
