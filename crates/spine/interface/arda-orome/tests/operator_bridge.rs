use arda_orome::operator_bridge::{
    ApprovalBinding, ApprovalSingleUseState, AttachmentProvenance, Audience, BridgeApproval,
    BridgeAttachment, BridgeError, BridgeLineage, BridgeOperation, BridgeRequest,
    ContentSensitivity, HermesMessageEvent, HermesSessionSource, OperatorBridge, OperatorIdentity,
    OperatorTransportHealth, TransportHealthInput, TransportHealthState,
};
use chrono::{TimeZone, Utc};
use tempfile::tempdir;

fn capture_request(event_id: &str) -> BridgeRequest {
    BridgeRequest {
        operator: OperatorIdentity {
            operator_id: "operator:mythos".into(),
            authenticated: true,
            authentication_method: "gateway_identity".into(),
            authenticated_at: "2026-08-09T10:00:00Z".into(),
        },
        lineage: BridgeLineage {
            session_id: "session:phone-1".into(),
            objective_id: None,
            project_id: None,
            task_id: None,
            run_id: None,
        },
        adapter_id: "hermes-gateway:relay-primary".into(),
        audience: Audience::OperatorPrivate,
        sensitivity: ContentSensitivity::Private,
        operation: BridgeOperation::Capture,
        event: HermesMessageEvent {
            text: "Remember the follow-up from my appointment".into(),
            message_type: "text".into(),
            user_id: Some("operator:mythos".into()),
            user_name: Some("Mythos".into()),
            source: HermesSessionSource {
                platform: "telegram".into(),
                chat_id: "conversation:private-42".into(),
                chat_type: "dm".into(),
                thread_id: None,
                message_id: Some(event_id.into()),
            },
            message_id: Some(event_id.into()),
            media_urls: vec!["gateway-attachment://telegram/voice-1".into()],
            media_types: vec!["audio/ogg".into()],
            timestamp: "2026-08-09T10:00:01Z".into(),
            prompt_response: None,
        },
        attachments: vec![BridgeAttachment {
            attachment_id: "attachment:voice-1".into(),
            media_type: "audio/ogg".into(),
            byte_length: 1024,
            content_digest: format!("sha256:{}", "a".repeat(64)),
            source_ref: "gateway-attachment://telegram/voice-1".into(),
            provenance: AttachmentProvenance {
                transport_event_id: event_id.into(),
                operator_supplied: true,
                captured_at: "2026-08-09T10:00:01Z".into(),
            },
        }],
        approval: None,
    }
}

fn approval_request(event_id: &str, action_digest: &str) -> (BridgeRequest, ApprovalBinding) {
    let mut request = capture_request(event_id);
    request.lineage.task_id = Some("task:repair".into());
    request.lineage.run_id = Some("run:repair-17".into());
    request.operation = BridgeOperation::Approve;
    request.sensitivity = ContentSensitivity::Internal;
    request.event.text = "Approve this exact repair dispatch".into();
    request.event.prompt_response = Some(arda_orome::operator_bridge::HermesPromptResponse {
        prompt_id: "approval:repair-17".into(),
        option_id: "approve".into(),
        label: Some("Approve".into()),
        prompt_message_id: Some("prompt-message:17".into()),
    });
    request.approval = Some(BridgeApproval {
        scope: vec!["dispatch:repair".into()],
        action_digest: action_digest.into(),
        expires_at: "2026-08-09T10:30:00Z".into(),
        single_use_state: ApprovalSingleUseState::Available,
        consumed_by_event_id: None,
    });
    let binding = ApprovalBinding {
        prompt_id: "approval:repair-17".into(),
        operator_id: "operator:mythos".into(),
        action_digest: action_digest.into(),
        scope: vec!["dispatch:repair".into()],
        session_id: "session:phone-1".into(),
        task_id: Some("task:repair".into()),
        run_id: Some("run:repair-17".into()),
        conversation_id: "conversation:private-42".into(),
        thread_id: None,
    };
    (request, binding)
}

#[test]
fn full_hermes_message_event_normalizes_to_operator_session_and_correlated_response() {
    let dir = tempdir().expect("tempdir");
    let bridge = OperatorBridge::new(dir.path()).expect("bridge");

    let session = bridge
        .ingest(
            capture_request("telegram:update:7001"),
            Utc.with_ymd_and_hms(2026, 8, 9, 10, 1, 0).unwrap(),
        )
        .expect("normalized event");

    assert_eq!(session.schema_version, "arda.operator-session.v1");
    let serialized = serde_json::to_value(&session).expect("serialize session");
    let keys = serialized
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        keys,
        std::collections::BTreeSet::from([
            "approval",
            "content",
            "delivery",
            "incoming",
            "lineage",
            "operation",
            "operator",
            "projection",
            "result",
            "schema_version",
        ])
    );
    assert_eq!(session.incoming.event_id, "telegram:update:7001");
    assert_eq!(
        session.incoming.idempotency_key,
        "operator-event:telegram:telegram:update:7001"
    );
    assert_eq!(session.projection.transport, "telegram");
    assert_eq!(
        session.projection.platform_message_id,
        "telegram:update:7001"
    );
    assert_eq!(
        session.content.attachments[0].provenance.transport_event_id,
        session.incoming.event_id
    );

    let response = bridge.correlate_response(&session, "Captured", vec!["arda://events/1".into()]);
    assert_eq!(response.session_id, "session:phone-1");
    assert_eq!(
        response.reply_to_platform_message_id,
        "telegram:update:7001"
    );
    assert_eq!(response.conversation_id, "conversation:private-42");
}

#[test]
fn duplicate_transport_event_and_replayed_approval_fail_closed() {
    let dir = tempdir().expect("tempdir");
    let bridge = OperatorBridge::new(dir.path()).expect("bridge");
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 10, 1, 0).unwrap();
    let digest = format!("sha256:{}", "b".repeat(64));

    bridge
        .ingest(capture_request("telegram:update:1"), now)
        .expect("first");
    assert_eq!(
        bridge.ingest(capture_request("telegram:update:1"), now),
        Err(BridgeError::DuplicateEvent("telegram:update:1".into()))
    );

    let (approval, binding) = approval_request("telegram:update:2", &digest);
    bridge
        .ingest_approval(approval, &binding, now)
        .expect("approval");
    let (replayed, binding) = approval_request("telegram:update:3", &digest);
    assert_eq!(
        bridge.ingest_approval(replayed, &binding, now),
        Err(BridgeError::ApprovalAlreadyConsumed(digest))
    );
}

#[test]
fn transport_cannot_supply_credentials_or_canonical_pending_approval() {
    let request = capture_request("telegram:update:authority");
    let mut value = serde_json::to_value(request).expect("serialize request");
    let object = value.as_object_mut().expect("request object");
    object.insert("platform_token".into(), serde_json::json!("secret"));
    assert!(serde_json::from_value::<BridgeRequest>(value).is_err());

    let mut value = serde_json::to_value(capture_request("telegram:update:binding"))
        .expect("serialize request");
    value.as_object_mut().expect("request object").insert(
        "pending_approval".into(),
        serde_json::json!({"action_digest": format!("sha256:{}", "d".repeat(64))}),
    );
    assert!(serde_json::from_value::<BridgeRequest>(value).is_err());
}

#[test]
fn stale_or_mismatched_approval_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let bridge = OperatorBridge::new(dir.path()).expect("bridge");
    let digest = format!("sha256:{}", "c".repeat(64));
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 11, 0, 0).unwrap();

    let (stale, binding) = approval_request("matrix:event:stale", &digest);
    assert!(matches!(
        bridge.ingest_approval(stale, &binding, now),
        Err(BridgeError::ApprovalExpired)
    ));

    let (mismatched, mut binding) = approval_request("matrix:event:mismatch", &digest);
    binding.run_id = Some("run:other".into());
    assert!(matches!(
        bridge.ingest_approval(
            mismatched,
            &binding,
            Utc.with_ymd_and_hms(2026, 8, 9, 10, 1, 0).unwrap()
        ),
        Err(BridgeError::ApprovalMismatch(_))
    ));
}

#[test]
fn sensitive_group_projection_is_redacted_without_losing_lineage() {
    let dir = tempdir().expect("tempdir");
    let bridge = OperatorBridge::new(dir.path()).expect("bridge");
    let mut request = capture_request("discord:message:1");
    request.event.source.platform = "discord".into();
    request.event.source.chat_type = "group".into();
    request.audience = Audience::Group;
    request.sensitivity = ContentSensitivity::Health;

    let session = bridge
        .ingest(request, Utc.with_ymd_and_hms(2026, 8, 9, 10, 1, 0).unwrap())
        .expect("redacted projection");

    assert_eq!(session.content.text, None);
    assert!(session.content.attachments.is_empty());
    assert_eq!(session.lineage.session_id, "session:phone-1");
}

#[test]
fn health_projection_distinguishes_configuration_availability_and_staleness() {
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 10, 0, 0).unwrap();
    let not_configured = OperatorTransportHealth::derive(TransportHealthInput::default(), now);
    assert_eq!(not_configured.state, TransportHealthState::NotConfigured);

    let stale = OperatorTransportHealth::derive(
        TransportHealthInput {
            configured: true,
            connected: true,
            authenticated: true,
            last_success_at: Some("2026-08-09T09:00:00Z".into()),
            last_error_code: None,
            stale_after_seconds: 300,
        },
        now,
    );
    assert_eq!(stale.state, TransportHealthState::Stale);

    let degraded = OperatorTransportHealth::derive(
        TransportHealthInput {
            configured: true,
            connected: true,
            authenticated: true,
            last_success_at: Some("2026-08-09T09:59:00Z".into()),
            last_error_code: Some("delivery_retrying".into()),
            stale_after_seconds: 300,
        },
        now,
    );
    assert_eq!(degraded.state, TransportHealthState::Degraded);
}
