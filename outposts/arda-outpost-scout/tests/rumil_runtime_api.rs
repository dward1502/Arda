use arda_outpost_scout::{build_runtime_router, ScoutRuntimeState};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 512 * 1024)
            .await
            .expect("response body"),
    )
    .expect("response json")
}

#[tokio::test]
async fn audit_route_projects_one_compact_vaire_receipt_and_supports_packet_followup() {
    let root = tempfile::tempdir().expect("runtime root");
    std::fs::create_dir_all(root.path().join("project/src")).expect("project source");
    std::fs::write(
        root.path().join("project/Cargo.toml"),
        "[package]\nname='runtime-fixture'\nversion='0.1.0'\n",
    )
    .expect("manifest");
    std::fs::write(
        root.path().join("project/src/lib.rs"),
        "pub fn fixture() {}\n",
    )
    .expect("source");
    let app = build_runtime_router(
        ScoutRuntimeState::new(root.path(), "http://127.0.0.1:9", "node-pi5-warden")
            .expect("runtime state"),
    );
    let project_id = Uuid::new_v4();
    let request_id = Uuid::new_v4();
    let request = json!({
        "root": "project",
        "project_name": "runtime-fixture",
        "project_kind": "rust",
        "remote_url": null,
        "request": {
            "request_id": request_id,
            "project_id": project_id,
            "profile_id": "warden-read-only",
            "source_revision_expectation": null,
            "requested_capabilities": ["inventory"],
            "root_policy": "bounded_request_root",
            "path_exclusions": [],
            "file_count_budget": 100,
            "byte_budget": 1048576,
            "source_excerpt_budget": 4096,
            "command_timeout_seconds": 5,
            "provider_allowlist": [],
            "redaction_policy": ["default_secrets"],
            "prior_audit_id": null,
            "requested_by": "node-pi5-warden",
            "expires_at_utc": (Utc::now() + Duration::minutes(5)).to_rfc3339(),
            "authority": "advisory_read_only"
        }
    });

    let first = app
        .clone()
        .oneshot(
            Request::post("/audit")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .expect("first audit response");
    assert_eq!(first.status(), StatusCode::OK);
    let first = json_body(first).await;
    assert_eq!(first["audit"]["report"]["completeness"], "complete");
    assert_eq!(first["audit"]["observation"]["authority"], "advisory");
    assert!(first["audit"]["observation"]["payload"]
        .get("file_records")
        .is_none());
    assert!(first["memory"]["memory_id"].is_string());
    let audit_id = first["audit"]["report"]["audit_id"]
        .as_str()
        .expect("audit id")
        .to_string();

    let replay = app
        .clone()
        .oneshot(
            Request::post("/audit")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay = json_body(replay).await;
    assert_eq!(replay["audit"]["replayed"], true);
    assert!(replay["memory"].is_null());

    let followup = app
        .clone()
        .oneshot(
            Request::post("/audit/followup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "audit_id": audit_id,
                        "sections": ["summary", "file_records"],
                        "path_prefix": "src",
                        "file_record_limit": 10
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("follow-up response");
    assert_eq!(followup.status(), StatusCode::OK);
    let followup = json_body(followup).await;
    assert_eq!(followup["authority"], "advisory_read_only");
    assert_eq!(followup["file_records"].as_array().unwrap().len(), 2);

    let recall = app
        .oneshot(
            Request::post("/recall")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"hours": 24, "query": audit_id, "limit": 10}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("recall response");
    assert_eq!(recall.status(), StatusCode::OK);
    let recall = json_body(recall).await;
    assert_eq!(recall["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        recall["records"][0]["observation"]["scope"]["custom"],
        "rumil_audit_receipt"
    );
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!root.path().join("data/approvals").exists());
}

#[tokio::test]
async fn audit_route_rejects_expired_requests_before_scanning() {
    let root = tempfile::tempdir().expect("runtime root");
    std::fs::create_dir(root.path().join("project")).expect("project root");
    let app = build_runtime_router(
        ScoutRuntimeState::new(root.path(), "http://127.0.0.1:9", "node-pi5-warden")
            .expect("runtime state"),
    );
    let response = app
        .oneshot(
            Request::post("/audit")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "root": "project",
                        "project_name": "expired",
                        "project_kind": "rust",
                        "remote_url": null,
                        "request": {
                            "request_id": Uuid::new_v4(),
                            "project_id": Uuid::new_v4(),
                            "profile_id": "warden-read-only",
                            "source_revision_expectation": null,
                            "requested_capabilities": ["inventory"],
                            "root_policy": "bounded_request_root",
                            "path_exclusions": [],
                            "file_count_budget": 10,
                            "byte_budget": 1024,
                            "source_excerpt_budget": 128,
                            "command_timeout_seconds": 1,
                            "provider_allowlist": [],
                            "redaction_policy": [],
                            "prior_audit_id": null,
                            "requested_by": "node-pi5-warden",
                            "expires_at_utc": (Utc::now() - Duration::seconds(1)).to_rfc3339(),
                            "authority": "advisory_read_only"
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("expired response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(!root.path().join("data/warden/rumil_audits").exists());
}
