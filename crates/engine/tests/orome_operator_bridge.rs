use arda_engine::orome::OromeOperatorRuntime;
use arda_orome::operator_bridge::{
    Audience, BridgeLineage, BridgeOperation, BridgeRequest, ContentSensitivity,
    HermesMessageEvent, HermesSessionSource, OperatorIdentity, TransportHealthInput,
    TransportHealthState,
};
use chrono::{TimeZone, Utc};
use tempfile::tempdir;

#[test]
fn engine_runtime_persists_and_correlates_transport_neutral_operator_event() {
    let dir = tempdir().expect("tempdir");
    let runtime = OromeOperatorRuntime::new(dir.path()).expect("runtime");
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 10, 0, 0).unwrap();
    let request = BridgeRequest {
        operator: OperatorIdentity {
            operator_id: "operator:mythos".into(),
            authenticated: true,
            authentication_method: "gateway_identity".into(),
            authenticated_at: "2026-08-09T09:59:59Z".into(),
        },
        lineage: BridgeLineage {
            session_id: "session:engine-phone-1".into(),
            objective_id: Some("objective:continuity".into()),
            project_id: None,
            task_id: None,
            run_id: None,
        },
        adapter_id: "hermes-gateway:relay-primary".into(),
        audience: Audience::OperatorPrivate,
        sensitivity: ContentSensitivity::Private,
        operation: BridgeOperation::Query,
        event: HermesMessageEvent {
            text: "What was I doing?".into(),
            message_type: "text".into(),
            user_id: Some("operator:mythos".into()),
            user_name: None,
            source: HermesSessionSource {
                platform: "discord".into(),
                chat_id: "discord:dm:42".into(),
                chat_type: "dm".into(),
                thread_id: None,
                message_id: Some("discord:message:99".into()),
            },
            message_id: Some("discord:message:99".into()),
            media_urls: Vec::new(),
            media_types: Vec::new(),
            timestamp: "2026-08-09T10:00:00Z".into(),
            prompt_response: None,
        },
        attachments: Vec::new(),
        approval: None,
    };

    let session = runtime.ingest(request, now).expect("session");
    let response = runtime.correlate_response(
        &session,
        "Continuity result",
        vec!["arda://objectives/objective:continuity".into()],
    );

    assert_eq!(response.session_id, "session:engine-phone-1");
    assert_eq!(
        response.objective_id.as_deref(),
        Some("objective:continuity")
    );
    assert_eq!(response.reply_to_platform_message_id, "discord:message:99");
    assert_eq!(response.transport, "discord");

    let health = runtime.transport_health(
        TransportHealthInput {
            configured: true,
            connected: true,
            authenticated: true,
            last_success_at: Some("2026-08-09T10:00:00Z".into()),
            last_error_code: None,
            stale_after_seconds: 300,
        },
        now,
    );
    assert_eq!(health.state, TransportHealthState::Ready);
}
