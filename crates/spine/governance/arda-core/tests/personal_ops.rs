use arda_core::personal_ops::{
    CaptureContent, CaptureSource, EvidenceClass, InboxCapture, PersonalItem, PersonalItemKind,
    PersonalOpsEnvelope, PersonalOpsError, ReminderDeliveryState, ReminderPolicy, ReminderReceipt,
    PERSONAL_OPS_SCHEMA_VERSION,
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn captured_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 30, 12, 0, 0).unwrap()
}

#[test]
fn capture_requires_only_content_source_and_timestamp() {
    let capture = InboxCapture {
        capture_id: Uuid::new_v4(),
        captured_at: captured_at(),
        source: CaptureSource::Text,
        content: CaptureContent {
            text: Some("Call the transplant coordinator".to_owned()),
            audio_reference: None,
        },
        attachments: Vec::new(),
        project_id: None,
        priority: None,
        due_at: None,
    };

    capture.validate().expect("minimal capture is valid");
    let encoded = serde_json::to_value(PersonalOpsEnvelope::new(capture.clone())).unwrap();
    assert_eq!(encoded["schema_version"], PERSONAL_OPS_SCHEMA_VERSION);
    assert!(capture.project_id.is_none());
    assert!(capture.priority.is_none());
    assert!(capture.due_at.is_none());
}

#[test]
fn capture_rejects_missing_text_and_audio_reference() {
    let capture = InboxCapture {
        capture_id: Uuid::new_v4(),
        captured_at: captured_at(),
        source: CaptureSource::Text,
        content: CaptureContent {
            text: None,
            audio_reference: None,
        },
        attachments: Vec::new(),
        project_id: None,
        priority: None,
        due_at: None,
    };

    assert!(matches!(
        capture.validate(),
        Err(PersonalOpsError::MissingCaptureContent)
    ));
}

#[test]
fn inferred_classification_cannot_overwrite_operator_authored_kind() {
    let mut item = PersonalItem::from_capture(
        InboxCapture {
            capture_id: Uuid::new_v4(),
            captured_at: captured_at(),
            source: CaptureSource::Text,
            content: CaptureContent {
                text: Some("Prepare appointment questions".to_owned()),
                audio_reference: None,
            },
            attachments: Vec::new(),
            project_id: None,
            priority: None,
            due_at: None,
        },
        PersonalItemKind::Note,
        EvidenceClass::OperatorAuthored,
    )
    .unwrap();

    let error = item
        .apply_inferred_kind(PersonalItemKind::Task, 0.91, "contains an action verb")
        .expect_err("operator-authored kind is protected");
    assert!(matches!(
        error,
        PersonalOpsError::OperatorAuthoredFieldProtected { field: "kind" }
    ));
    assert_eq!(item.kind.value, PersonalItemKind::Note);
}

#[test]
fn inferred_classification_is_reversible_and_retains_prior_value() {
    let mut item = PersonalItem::from_capture(
        InboxCapture {
            capture_id: Uuid::new_v4(),
            captured_at: captured_at(),
            source: CaptureSource::Text,
            content: CaptureContent {
                text: Some("Review Stage 4 receipts".to_owned()),
                audio_reference: None,
            },
            attachments: Vec::new(),
            project_id: None,
            priority: None,
            due_at: None,
        },
        PersonalItemKind::Note,
        EvidenceClass::Unavailable,
    )
    .unwrap();

    let change = item
        .apply_inferred_kind(PersonalItemKind::Task, 0.87, "explicit review action")
        .unwrap();
    assert_eq!(change.previous, PersonalItemKind::Note);
    assert_eq!(change.current, PersonalItemKind::Task);
    assert_eq!(item.kind.evidence_class, EvidenceClass::Inferred);

    item.revert_kind(change).unwrap();
    assert_eq!(item.kind.value, PersonalItemKind::Note);
    assert_eq!(item.kind.evidence_class, EvidenceClass::Unavailable);
}

#[test]
fn health_items_require_evidence_class_and_non_clinical_disclosure() {
    let capture = InboxCapture {
        capture_id: Uuid::new_v4(),
        captured_at: captured_at(),
        source: CaptureSource::Text,
        content: CaptureContent {
            text: Some("Record morning temperature".to_owned()),
            audio_reference: None,
        },
        attachments: Vec::new(),
        project_id: None,
        priority: None,
        due_at: None,
    };
    let item = PersonalItem::from_capture(
        capture,
        PersonalItemKind::Health,
        EvidenceClass::SelfReported,
    )
    .unwrap();

    assert_eq!(item.kind.evidence_class, EvidenceClass::SelfReported);
    assert!(item
        .non_clinical_disclosure
        .as_deref()
        .is_some_and(|value| value.contains("not clinical")));
}

#[test]
fn reminder_policy_is_bounded_and_delivery_truth_is_explicit() {
    let policy = ReminderPolicy::default();
    assert!(policy.max_attempts > 0);
    assert!(policy.max_attempts <= 5);
    assert!(policy.minimum_interval_minutes > 0);

    let attempted = ReminderReceipt {
        reminder_id: Uuid::new_v4(),
        item_id: Uuid::new_v4(),
        attempted_at: captured_at(),
        state: ReminderDeliveryState::Attempted,
        channel: "local".to_owned(),
        receipt_reference: None,
    };
    assert!(!attempted.was_delivered());
}
