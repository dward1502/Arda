use arda_core::personal_ops::{
    CaptureContent, CaptureRecordedEvent, CaptureSource, EvidenceClass, InboxCapture,
    ItemClassifiedEvent, ItemCompletedEvent, ItemScheduledEvent, PersonalItemKind,
    PersonalOpsEnvelope, PersonalOpsRecord, ReminderAcknowledgedEvent, ReminderAttemptedEvent,
    ReminderDeliveryState, ReminderReceipt,
};
use arda_core::personal_ops_projection::build_projection;
use chrono::{Datelike, TimeZone, Utc};
use uuid::Uuid;

fn captured_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 30, 12, 0, 0).unwrap()
}

fn make_capture() -> InboxCapture {
    InboxCapture {
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
    }
}

fn make_envelope(record: PersonalOpsRecord) -> PersonalOpsEnvelope<PersonalOpsRecord> {
    PersonalOpsEnvelope {
        schema_version: "arda.personal-ops.v1".to_owned(),
        record,
    }
}

fn capture_event(capture: InboxCapture) -> PersonalOpsRecord {
    PersonalOpsRecord::CaptureRecorded(CaptureRecordedEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        capture,
    })
}

fn classify_event(
    item_id: Uuid,
    kind: PersonalItemKind,
    evidence: EvidenceClass,
) -> PersonalOpsRecord {
    PersonalOpsRecord::ItemClassified(ItemClassifiedEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        item_id,
        kind,
        evidence_class: evidence,
        confidence: None,
        rationale: None,
    })
}

fn schedule_event(
    item_id: Uuid,
    scheduled_at: Option<chrono::DateTime<Utc>>,
    due_at: Option<chrono::DateTime<Utc>>,
) -> PersonalOpsRecord {
    PersonalOpsRecord::ItemScheduled(ItemScheduledEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        item_id,
        scheduled_at,
        due_at,
    })
}

fn complete_event(item_id: Uuid, completed_at: chrono::DateTime<Utc>) -> PersonalOpsRecord {
    PersonalOpsRecord::ItemCompleted(ItemCompletedEvent {
        event_id: Uuid::new_v4(),
        occurred_at: captured_at(),
        operator_id: "operator-0".to_owned(),
        item_id,
        completed_at,
    })
}

#[test]
fn projection_empty_log_yields_empty_views() {
    let events: Vec<PersonalOpsEnvelope<PersonalOpsRecord>> = vec![];
    let now = Utc::now();
    let date = now.naive_local().date();

    let proj = build_projection(&events, now, date);
    assert_eq!(proj.event_count, 0);
    assert!(proj.inbox.is_empty());
    assert!(proj.today.is_empty());
    assert!(proj.waiting.is_empty());
    assert!(proj.scheduled.is_empty());
    assert!(proj.completed.is_empty());
}

#[test]
fn capture_appears_in_inbox_until_classified() {
    let capture = make_capture();
    let capture_id = capture.capture_id;
    let events = vec![make_envelope(capture_event(capture))];
    let now = Utc::now();
    let date = now.naive_local().date();

    let proj = build_projection(&events, now, date);
    assert_eq!(proj.inbox.len(), 1);
    assert_eq!(proj.inbox[0].capture_id, capture_id);
    assert_eq!(proj.inbox[0].operator_id, "operator-0");
    assert_eq!(proj.inbox[0].content, "Call the transplant coordinator");
    assert!(proj.today.is_empty());
}

#[test]
fn classification_promotes_capture_to_today_bucket() {
    let capture = make_capture();
    let capture_id = capture.capture_id;
    let events = vec![
        make_envelope(capture_event(capture)),
        make_envelope(classify_event(
            capture_id,
            PersonalItemKind::Task,
            EvidenceClass::Inferred,
        )),
    ];
    let now = Utc::now();
    let date = now.naive_local().date();

    let proj = build_projection(&events, now, date);
    assert!(
        proj.inbox.is_empty(),
        "capture should leave inbox after classification"
    );
    assert_eq!(proj.today.len(), 1, "inferred task should appear in today");
    assert_eq!(proj.today[0].content, "Call the transplant coordinator");
}

#[test]
fn classification_then_completion_moves_to_completed() {
    let capture = make_capture();
    let capture_id = capture.capture_id;
    let completed_at = captured_at();
    let events = vec![
        make_envelope(capture_event(capture)),
        make_envelope(classify_event(
            capture_id,
            PersonalItemKind::Task,
            EvidenceClass::OperatorAuthored,
        )),
        make_envelope(complete_event(capture_id, completed_at)),
    ];
    let now = Utc::now();
    let date = now.naive_local().date();

    let proj = build_projection(&events, now, date);
    assert!(proj.inbox.is_empty());
    assert!(
        proj.today.is_empty(),
        "completed items should not appear in today"
    );
    assert_eq!(proj.completed.len(), 1);
    assert!(proj.completed[0].completed_at.is_some());
}

#[test]
fn scheduled_today_appears_in_today_bucket() {
    let capture = make_capture();
    let capture_id = capture.capture_id;
    let today = now_date();
    let scheduled = Utc
        .with_ymd_and_hms(today.year(), today.month(), today.day(), 9, 0, 0)
        .unwrap();
    let events = vec![
        make_envelope(capture_event(capture)),
        make_envelope(classify_event(
            capture_id,
            PersonalItemKind::Reminder,
            EvidenceClass::OperatorAuthored,
        )),
        make_envelope(schedule_event(capture_id, Some(scheduled), None)),
    ];
    let now = Utc::now();
    let date = now.naive_local().date();

    let proj = build_projection(&events, now, date);
    assert_eq!(proj.today.len(), 1);
    assert_eq!(proj.today[0].kind, PersonalItemKind::Reminder);
}

fn now_date() -> chrono::NaiveDateTime {
    Utc::now().naive_local()
}

#[test]
fn reminder_acknowledgement_updates_only_the_matching_reminder() {
    let first = make_capture();
    let first_id = first.capture_id;
    let second = make_capture();
    let second_id = second.capture_id;
    let first_reminder = Uuid::new_v4();
    let second_reminder = Uuid::new_v4();
    let attempted = |item_id, reminder_id| {
        PersonalOpsRecord::ReminderAttempted(ReminderAttemptedEvent {
            event_id: Uuid::new_v4(),
            occurred_at: captured_at(),
            operator_id: "operator-0".to_owned(),
            item_id,
            receipt: ReminderReceipt {
                reminder_id,
                item_id,
                attempted_at: captured_at(),
                state: ReminderDeliveryState::Attempted,
                channel: "local".to_owned(),
                receipt_reference: None,
            },
        })
    };
    let events = vec![
        make_envelope(capture_event(first)),
        make_envelope(classify_event(
            first_id,
            PersonalItemKind::Reminder,
            EvidenceClass::OperatorAuthored,
        )),
        make_envelope(capture_event(second)),
        make_envelope(classify_event(
            second_id,
            PersonalItemKind::Reminder,
            EvidenceClass::OperatorAuthored,
        )),
        make_envelope(attempted(first_id, first_reminder)),
        make_envelope(attempted(second_id, second_reminder)),
        make_envelope(PersonalOpsRecord::ReminderAcknowledged(
            ReminderAcknowledgedEvent {
                event_id: Uuid::new_v4(),
                occurred_at: captured_at(),
                operator_id: "operator-0".to_owned(),
                reminder_id: first_reminder,
                state: ReminderDeliveryState::Acknowledged,
                receipt_reference: None,
            },
        )),
    ];

    let projection = build_projection(&events, Utc::now(), Utc::now().date_naive());
    let first_item = projection
        .today
        .iter()
        .find(|item| item.item_id == first_id)
        .unwrap();
    let second_item = projection
        .today
        .iter()
        .find(|item| item.item_id == second_id)
        .unwrap();

    assert_eq!(
        first_item.reminder_state.as_ref().unwrap().delivery_state,
        ReminderDeliveryState::Acknowledged
    );
    assert_eq!(
        second_item.reminder_state.as_ref().unwrap().delivery_state,
        ReminderDeliveryState::Attempted
    );
}
