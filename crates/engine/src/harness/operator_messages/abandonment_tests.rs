//! Synthetic authenticated handler coverage; never installed operator acceptance.
use super::*;
use std::sync::Arc;
#[tokio::test]
async fn abandonment_authorization_requires_private_authenticated_ingress_and_replays() {
    const TEST:&str="harness::operator_messages::abandonment_tests::abandonment_authorization_requires_private_authenticated_ingress_and_replays";
    if std::env::var_os("ARDA_ABANDONMENT_TEST_CHILD").is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env("ARDA_ABANDONMENT_TEST_CHILD", "1")
            .env("ARDA_HERMES_LOCAL_CAPABILITY", "synthetic-abandonment-test")
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&result.stdout);
        assert!(
            result.status.success() && out.contains("ABANDONMENT_HANDLER_ASSERTIONS_COMPLETE"),
            "{} {}",
            out,
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let (temp, _store, manifest) = crate::objectives::abandonment::tests::fixture();
    let root = temp.path();
    let state = HarnessState {
        research_store_policy: super::super::ResearchStorePolicy::Isolated,
        harness_addr: "127.0.0.1:0".into(),
        child_pids: Arc::new(tokio::sync::RwLock::new(vec![])),
        service_names: Arc::new(vec![]),
        service_statuses: Arc::new(tokio::sync::RwLock::new(vec![])),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: super::super::DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: super::super::DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: super::super::presence::HarnessPresenceState::default(),
        workbench_root: root.into(),
        operator_id: "operator".into(),
    };
    let message = json!({"operator":{"operator_id":"operator","authenticated":true,"authentication_method":"local_session","authenticated_at":Utc::now().to_rfc3339()},"adapter_id":"hermes-cli","event":{"text":format!("arda authorize-abandonment {}",serde_json::to_string(&manifest).unwrap()),"message_type":"text","user_id":"operator","user_name":"fixture","source":{"platform":"cli","chat_id":"fixture-session","chat_type":"private","thread_id":null,"message_id":"fixture-abandonment"},"message_id":"fixture-abandonment","media_urls":[],"media_types":[],"timestamp":Utc::now().to_rfc3339(),"prompt_response":null}});
    let invoke = |message: Value, authenticated: bool| {
        let state = state.clone();
        async move {
            let mut headers = HeaderMap::new();
            if authenticated {
                headers.insert(
                    "x-arda-local-capability",
                    "synthetic-abandonment-test".parse().unwrap(),
                );
            }
            ingest_local_operator_message(
                State(state),
                ConnectInfo("127.0.0.1:1234".parse().unwrap()),
                None,
                headers,
                Json(serde_json::from_value(message).unwrap()),
            )
            .await
        }
    };
    assert!(invoke(message.clone(), false).await.is_err());
    for (key, value) in [
        ("operator_id", json!("other")),
        ("authenticated", json!(false)),
        ("authenticated_at", json!("2000-01-01T00:00:00Z")),
    ] {
        let mut bad = message.clone();
        bad["operator"][key] = value;
        assert!(invoke(bad, true).await.is_err());
    }
    let mut shared = message.clone();
    shared["event"]["source"]["chat_type"] = json!("group");
    assert!(invoke(shared, true).await.is_err());
    let db = rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM operator_abandonment_authorizations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let first = invoke(message.clone(), true).await.unwrap().0;
    let replay = invoke(message.clone(), true).await.unwrap().0;
    assert_eq!(first.summary, replay.summary);
    assert_eq!(first.evidence_refs, replay.evidence_refs);
    let mut altered = message;
    let mut changed = manifest.clone();
    changed.reason = "valid but changed authorized scope".into();
    changed.validate().unwrap();
    altered["event"]["text"] = json!(format!(
        "arda authorize-abandonment {}",
        serde_json::to_string(&changed).unwrap()
    ));
    assert!(invoke(altered, true).await.is_err());
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM operator_abandonment_authorizations",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM retained_snapshot_releases", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM leaves WHERE stage='execute'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        5
    );
    let ledger = std::fs::read_to_string(
        root.join("core/state/orome/operator-session/operator_sessions.jsonl"),
    )
    .unwrap();
    assert!(ledger.contains("authorize-abandonment"));
    println!("ABANDONMENT_HANDLER_ASSERTIONS_COMPLETE");
}
