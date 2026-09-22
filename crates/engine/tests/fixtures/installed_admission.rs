//! Gateway-authenticated synthetic evidence feeding the genuine retained runtime.
use super::{admitted_evidence, gateway, PROJECT_ID};
use arda_engine::objectives::{ObjectiveState, ObjectiveStore};
use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use std::{fs, net::SocketAddr, path::Path};

pub fn enabled() -> bool {
    std::env::var("ARDA_TEST_INSTALLED_ADMITTED").as_deref() == Ok("1")
}

pub fn assert_stage_inputs(root: &Path, transcript: &rusqlite::Connection, run_id: &str) {
    let raw = fs::read_to_string(root.join("original-admitted-brief.json")).unwrap();
    let mut statement = transcript
        .prepare("SELECT content FROM messages WHERE role='user'")
        .unwrap();
    let mut stages = Vec::new();
    for prompt in statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
    {
        let prompt = prompt.unwrap();
        let (_, context) = prompt
            .split_once("Canonical node context follows:\n")
            .unwrap();
        let context = serde_json::Deserializer::from_str(context)
            .into_iter::<Value>()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(context["run_id"], run_id);
        let objective = context["objective"].as_str().unwrap();
        assert!(
            objective.contains(&raw),
            "installed stage lost original admitted bytes"
        );
        stages.push(context["node"]["kind"].as_str().unwrap().to_owned());
    }
    stages.sort();
    assert_eq!(stages, ["execute", "review", "verify"]);
}

pub async fn assert_completed_replay(root: &Path, bound: SocketAddr) {
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let snapshot = || {
        [
            "objectives",
            "leaves",
            "resident_context_bindings",
            "objective_admissions",
        ]
        .map(|table| {
            let mut statement = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let columns = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|column| row.get::<_, rusqlite::types::Value>(column))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            rows
        })
    };
    let before = snapshot();
    let original = fs::read_to_string(root.join("original-admission.json")).unwrap();
    assert_eq!(
        db.query_row("SELECT input_json FROM objective_admissions", [], |row| row
            .get::<_, String>(0))
            .unwrap(),
        original
    );
    let message: Value =
        serde_json::from_slice(&fs::read(root.join("admission-message.json")).unwrap()).unwrap();
    let response = gateway::gateway_client()
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(
        snapshot(),
        before,
        "source-absent replay changed canonical state"
    );
}

pub async fn approve(root: &Path, bound: SocketAddr) -> (String, String) {
    // Discovery intentionally supplies no external facts. The trusted-store
    // evidence below is explicitly synthetic, shared with admission tests.
    let app = Router::new()
        .route("/suggestions", post(|Json(s): Json<Value>| async move { Json(json!({"status":"accepted", "suggestion":s})) }))
        .route("/discover", post(|| async { Json(json!({"report":{"provider":"fixture", "results":[]},"memory":{"memory_id":null}})) }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let scout = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let (research_bound, shutdown, harness) =
        gateway::start_research_harness_at(root, Some(format!("http://{address}"))).await;
    let client = gateway::gateway_client();
    client
        .post(format!("http://{research_bound}/v1/operator/messages"))
        .json(&gateway::gateway_message(
            "installed-fixture-question",
            "arda research source-backed local testing",
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    shutdown.notify_waiters();
    harness.await.unwrap();
    scout.abort();
    let briefs: Vec<_> = fs::read_dir(root.join("data/workbench/research/briefs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(briefs.len(), 1);
    let original: Value = serde_json::from_slice(&fs::read(&briefs[0]).unwrap()).unwrap();
    assert_eq!(original["execution_authorized"], false);
    let (brief_id, raw, path) = admitted_evidence::publish(root, &original);
    let message = gateway::gateway_message("installed-fixture-admission", &format!("arda objective-from-brief {brief_id} {PROJECT_ID} Perform the bounded read-only check: execute python3 verify-context-bootstrap.py, verify its exit status, and review context-bootstrap-input.txt and verify-context-bootstrap.py using read_file. Do not modify files. Return the required Arda JSON, with tool_evidence, test_evidence and artifacts as empty arrays; Harness derives actual evidence."));
    let response = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&message)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    let admission: String = db
        .query_row("SELECT input_json FROM objective_admissions", [], |row| {
            row.get(0)
        })
        .unwrap();
    let saved: Value = serde_json::from_str(&admission).unwrap();
    let leaves = saved["leaves"].as_array().unwrap();
    assert_eq!(leaves.len(), 1, "one gateway-admitted project leaf");
    for key in ["execution_prompt", "verification_prompt", "review_prompt"] {
        assert!(leaves[0]["execution"][key].as_str().unwrap().contains(&raw));
    }
    let objective = saved["id"].as_str().unwrap().to_owned();
    let leaf = leaves[0]["id"].as_str().unwrap().to_owned();
    let store = ObjectiveStore::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    assert_eq!(
        store.objective(&objective).unwrap().unwrap().state,
        ObjectiveState::PendingApproval
    );
    let response = client
        .post(format!("http://{bound}/v1/operator/messages"))
        .json(&gateway::gateway_message(
            "installed-fixture-approval",
            &format!(
                "arda approve-objective {leaf} {objective} Explicit synthetic fixture approval"
            ),
        ))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(
        store.objective(&objective).unwrap().unwrap().state,
        ObjectiveState::Approved
    );
    fs::write(root.join("original-admitted-brief.json"), raw).unwrap();
    fs::write(root.join("original-admission.json"), admission).unwrap();
    fs::write(
        root.join("admission-message.json"),
        serde_json::to_vec(&message).unwrap(),
    )
    .unwrap();
    fs::remove_file(path).unwrap();
    // Both budgets were valid at admission; only runtime ordinary routing is poisoned.
    fs::write(
        root.join("config/adapters/hermes-workbench.toml"),
        "deliberately invalid ordinary adapter config",
    )
    .unwrap();
    (objective, leaf)
}
