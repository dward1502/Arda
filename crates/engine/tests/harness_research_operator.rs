use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

async fn start_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator-0".to_string(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().expect("loopback address")),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (bound, shutdown, handle)
}

fn envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-research-1",
            "approval_id": "approval-research-1",
            "ledger_writes": [],
            "decision": "policy_safe",
            "created_at_utc": "2026-08-12T00:00:00Z"
        },
        "idempotency_key": key
    })
}

fn question() -> Value {
    json!({
        "schema_version": "arda.warden.watchlist.v1",
        "question_id": "550e8400-e29b-41d4-a716-446655440010",
        "owner": "operator-0",
        "question": "What changed in the Arda runtime?",
        "rationale": "Keep the operator brief current.",
        "tags": ["runtime"],
        "cadence": {"kind": "manual"},
        "expires_at_utc": "2027-08-12T00:00:00Z",
        "source_policy": {
            "policy_id": "public-web",
            "allowed_sources": ["https://"],
            "max_sources_per_run": 5,
            "allow_private_targets": false
        },
        "evidence_requirements": {
            "minimum_canonical_sources": 1,
            "require_canonical_fetch": true,
            "max_source_age_seconds": 604800
        },
        "contradiction_policy": "require_disclosure",
        "budgets": {
            "max_results": 10,
            "max_fetch_bytes": 2000000,
            "max_tokens": 4000,
            "max_attempts": 2
        },
        "notification_policy": {"enabled": false, "destination": null},
        "state": "enabled",
        "backend_suggestion_ids": []
    })
}

#[tokio::test]
async fn authenticated_research_watchlist_survives_process_restart() {
    let root = TempDir::new().expect("temporary root");
    let client = reqwest::Client::new();
    let (bound, shutdown, handle) = start_harness(&root).await;

    let unauthorized = client
        .get(format!("http://{bound}/v1/research/questions"))
        .send()
        .await
        .expect("unauthorized request");
    assert_eq!(unauthorized.status(), 403);

    let created_question: Value = client
        .post(format!("http://{bound}/v1/research/questions"))
        .header("x-arda-operator-id", "operator-0")
        .json(&json!({
            "question": question(),
            "read_only": true,
            "envelope": envelope("question-create-1")
        }))
        .send()
        .await
        .expect("create question")
        .error_for_status()
        .expect("question accepted")
        .json()
        .await
        .expect("question response");
    assert_eq!(created_question["question"]["owner"], "operator-0");

    let watchlist_id = "550e8400-e29b-41d4-a716-446655440011";
    client
        .post(format!("http://{bound}/v1/research/watchlists"))
        .header("x-arda-operator-id", "operator-0")
        .json(&json!({
            "watchlist": {
                "schema_version": "arda.warden.watchlist.v1",
                "watchlist_id": watchlist_id,
                "name": "Runtime watch",
                "question_ids": [question()["question_id"]],
                "state": "enabled"
            },
            "envelope": envelope("watchlist-create-1")
        }))
        .send()
        .await
        .expect("create watchlist")
        .error_for_status()
        .expect("watchlist accepted");

    let paused: Value = client
        .post(format!(
            "http://{bound}/v1/research/watchlists/{watchlist_id}/pause"
        ))
        .header("x-arda-operator-id", "operator-0")
        .json(&envelope("watchlist-pause-1"))
        .send()
        .await
        .expect("pause watchlist")
        .error_for_status()
        .expect("pause accepted")
        .json()
        .await
        .expect("pause response");
    assert_eq!(paused["state"], "paused");

    shutdown.notify_waiters();
    handle.await.expect("first harness join");

    let (restarted_bound, restarted_shutdown, restarted_handle) = start_harness(&root).await;
    let recovered: Value = client
        .get(format!(
            "http://{restarted_bound}/v1/research/watchlists/{watchlist_id}"
        ))
        .header("x-arda-operator-id", "operator-0")
        .send()
        .await
        .expect("recover watchlist")
        .error_for_status()
        .expect("recovered watchlist accepted")
        .json()
        .await
        .expect("recovered watchlist response");
    assert_eq!(recovered["state"], "paused");
    assert_eq!(recovered["question_ids"][0], question()["question_id"]);

    let resumed: Value = client
        .post(format!(
            "http://{restarted_bound}/v1/research/watchlists/{watchlist_id}/resume"
        ))
        .header("x-arda-operator-id", "operator-0")
        .json(&envelope("watchlist-resume-1"))
        .send()
        .await
        .expect("resume watchlist")
        .error_for_status()
        .expect("resume accepted")
        .json()
        .await
        .expect("resume response");
    assert_eq!(resumed["state"], "enabled");

    let retired: Value = client
        .post(format!(
            "http://{restarted_bound}/v1/research/watchlists/{watchlist_id}/retire"
        ))
        .header("x-arda-operator-id", "operator-0")
        .json(&envelope("watchlist-retire-1"))
        .send()
        .await
        .expect("retire watchlist")
        .error_for_status()
        .expect("retire accepted")
        .json()
        .await
        .expect("retire response");
    assert_eq!(retired["state"], "retired");

    restarted_shutdown.notify_waiters();
    restarted_handle.await.expect("restarted harness join");

    let (final_bound, final_shutdown, final_handle) = start_harness(&root).await;
    let durable_retired: Value = client
        .get(format!(
            "http://{final_bound}/v1/research/watchlists/{watchlist_id}"
        ))
        .header("x-arda-operator-id", "operator-0")
        .send()
        .await
        .expect("recover retired watchlist")
        .error_for_status()
        .expect("retired watchlist accepted")
        .json()
        .await
        .expect("retired watchlist response");
    assert_eq!(durable_retired["state"], "retired");

    final_shutdown.notify_waiters();
    final_handle.await.expect("final harness join");
}
