use arda_outpost_scout::{build_runtime_router, ScoutRuntimeState};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use tower::ServiceExt;

fn serve_search_fixture() -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind search fixture");
    let address = listener.local_addr().expect("search fixture address");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept search request");
        let mut request = [0_u8; 4096];
        let bytes_read = stream.read(&mut request).expect("read search request");
        assert!(bytes_read > 0, "search request must not be empty");
        let body = r#"{"results":[{"title":"Agent governance","url":"https://example.com/governance","content":"A new governed agent runtime","engine":"fixture","score":1.0}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(), body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write search response");
    });
    (format!("http://{address}"), handle)
}

#[tokio::test]
async fn search_route_persists_and_recall_returns_the_observation() {
    let root = tempfile::tempdir().expect("runtime root");
    let (searxng_url, fixture) = serve_search_fixture();
    let state =
        ScoutRuntimeState::new(root.path(), searxng_url, "node-pi5-warden").expect("runtime state");
    let app = build_runtime_router(state);

    let search = app
        .clone()
        .oneshot(
            Request::post("/search")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"query":"new agent governance", "limit":3}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("search response");
    fixture.join().expect("search fixture");
    assert_eq!(search.status(), StatusCode::OK);
    let search_json: Value = serde_json::from_slice(
        &to_bytes(search.into_body(), 256 * 1024)
            .await
            .expect("search body"),
    )
    .expect("search json");
    assert_eq!(
        search_json["report"]["results"][0]["url"],
        "https://example.com/governance"
    );
    assert!(search_json["memory"]["memory_id"].is_string());

    let recall = app
        .oneshot(
            Request::post("/recall")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"hours":24, "query":"new agent governance", "limit":5}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("recall response");
    assert_eq!(recall.status(), StatusCode::OK);
    let recall_json: Value = serde_json::from_slice(
        &to_bytes(recall.into_body(), 256 * 1024)
            .await
            .expect("recall body"),
    )
    .expect("recall json");
    assert_eq!(recall_json["status"], "available");
    assert_eq!(recall_json["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        recall_json["records"][0]["observation"]["scope"]["Custom"],
        "internet_research"
    );
}

#[tokio::test]
async fn health_route_identifies_the_warden_runtime() {
    let root = tempfile::tempdir().expect("runtime root");
    let state = ScoutRuntimeState::new(root.path(), "http://127.0.0.1:9", "node-pi5-warden")
        .expect("runtime state");
    let response = build_runtime_router(state)
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .expect("health response");
    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("health body"),
    )
    .expect("health json");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["source"], "node-pi5-warden");
    assert_eq!(json["authority"], "advisory");
}
