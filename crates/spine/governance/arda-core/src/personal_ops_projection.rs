use crate::personal_ops::{
    ClassificationReason, EvidenceClass, ItemClassifiedEvent, ItemCompletedEvent,
    ItemScheduledEvent, ItemState, PersonalItemKind, PersonalOpsEnvelope, PersonalOpsRecord,
    ReminderAcknowledgedEvent, ReminderAttemptedEvent, ReminderDeliveryState, ReminderPolicy,
    ReminderState, NON_CLINICAL_DISCLOSURE,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

/// A rebuilt projection of personal-operations state.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PersonalOpsProjection {
    pub generated_at: String,
    pub event_count: usize,
    pub inbox: Vec<InboxItem>,
    pub today: Vec<ProjectedItem>,
    pub waiting: Vec<ProjectedItem>,
    pub scheduled: Vec<ProjectedItem>,
    pub completed: Vec<ProjectedItem>,
}

/// A capture that has not yet been classified into a PersonalItem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InboxItem {
    pub capture_id: Uuid,
    pub operator_id: String,
    pub content: String,
    pub audio_reference: Option<String>,
    pub occurred_at: String,
}

/// A projected PersonalItem with its current scheduling / completion state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectedItem {
    pub item_id: Uuid,
    pub kind: PersonalItemKind,
    pub operator_id: String,
    pub content: String,
    pub evidence_class: EvidenceClass,
    pub confidence: Option<f32>,
    pub classification_reason: Option<ClassificationReason>,
    pub scheduled_at: Option<String>,
    pub due_at: Option<String>,
    pub completed_at: Option<String>,
    pub reminder_state: Option<ReminderState>,
    pub reminder_attempts: u64,
    pub reminder_acknowledged_at: Option<String>,
    pub current_state: ItemState,
}

/// Build a full projection from an ordered event log.
///
/// The `events` slice MUST be in chronological order (ascending
/// `occurred_at`).
pub fn build_projection(
    events: &[PersonalOpsEnvelope<PersonalOpsRecord>],
    now: DateTime<Utc>,
    operator_local_date: chrono::NaiveDate,
) -> PersonalOpsProjection {
    let mut projection = PersonalOpsProjection {
        generated_at: now.to_rfc3339(),
        event_count: events.len(),
        ..Default::default()
    };

    let mut items: BTreeMap<Uuid, ProjectedItem> = BTreeMap::new();
    let mut inbox_captures: Vec<InboxItem> = Vec::new();

    for envelope in events {
        match &envelope.record {
            PersonalOpsRecord::CaptureRecorded(e) => {
                inbox_captures.push(InboxItem {
                    capture_id: e.capture.capture_id,
                    operator_id: e.operator_id.clone(),
                    content: e.capture.content.text.clone().unwrap_or_default(),
                    audio_reference: e.capture.content.audio_reference.clone(),
                    occurred_at: e.occurred_at.to_rfc3339(),
                });
            }
            PersonalOpsRecord::ItemClassified(e) => {
                handle_classification(e, &mut items, &mut inbox_captures);
            }
            PersonalOpsRecord::ItemScheduled(e) => {
                handle_scheduled(e, &mut items);
            }
            PersonalOpsRecord::ItemCompleted(e) => {
                handle_completed(e, &mut items);
            }
            PersonalOpsRecord::ReminderAttempted(e) => {
                handle_reminder_attempted(e, &mut items);
            }
            PersonalOpsRecord::ReminderAcknowledged(e) => {
                handle_reminder_acknowledged(e, &mut items);
            }
        }
    }

    projection.inbox = inbox_captures;

    for item in items.into_values() {
        bucket_item(&item, &mut projection, operator_local_date);
    }

    projection
}

fn reason_from_evidence(evidence: EvidenceClass) -> Option<ClassificationReason> {
    match evidence {
        EvidenceClass::OperatorAuthored => Some(ClassificationReason::OperatorInput),
        EvidenceClass::Imported => Some(ClassificationReason::Import),
        EvidenceClass::Inferred | EvidenceClass::DeviceMeasured | EvidenceClass::SelfReported => {
            Some(ClassificationReason::Inference)
        }
        EvidenceClass::Unavailable => None,
    }
}

fn handle_classification(
    e: &ItemClassifiedEvent,
    items: &mut BTreeMap<Uuid, ProjectedItem>,
    inbox: &mut Vec<InboxItem>,
) {
    let item_id = e.item_id;
    let confidence = e.confidence;

    inbox.retain(|item| item.capture_id != item_id);

    if !items.contains_key(&item_id) {
        items.insert(
            item_id,
            ProjectedItem {
                item_id,
                kind: e.kind,
                operator_id: e.operator_id.clone(),
                content: String::new(),
                evidence_class: e.evidence_class,
                confidence,
                classification_reason: reason_from_evidence(e.evidence_class),
                scheduled_at: None,
                due_at: None,
                completed_at: None,
                reminder_state: None,
                reminder_attempts: 0,
                reminder_acknowledged_at: None,
                current_state: ItemState::Active,
            },
        );
    } else if let Some(item) = items.get_mut(&item_id) {
        if item.evidence_class != EvidenceClass::OperatorAuthored {
            item.kind = e.kind;
            item.evidence_class = e.evidence_class;
            item.confidence = confidence;
        }
    }
}

fn handle_scheduled(e: &ItemScheduledEvent, items: &mut BTreeMap<Uuid, ProjectedItem>) {
    if let Some(item) = items.get_mut(&e.item_id) {
        if let Some(scheduled) = &e.scheduled_at {
            item.scheduled_at = Some(scheduled.to_rfc3339());
        }
        if let Some(due) = &e.due_at {
            item.due_at = Some(due.to_rfc3339());
        }
        if item.current_state == ItemState::Draft {
            item.current_state = ItemState::Active;
        }
    }
}

fn handle_completed(e: &ItemCompletedEvent, items: &mut BTreeMap<Uuid, ProjectedItem>) {
    if let Some(item) = items.get_mut(&e.item_id) {
        item.completed_at = Some(e.completed_at.to_rfc3339());
        item.current_state = ItemState::Completed;
    }
}

fn handle_reminder_attempted(
    e: &ReminderAttemptedEvent,
    items: &mut BTreeMap<Uuid, ProjectedItem>,
) {
    if let Some(item) = items.get_mut(&e.item_id) {
        item.reminder_attempts += 1;
        let attempt_count = if let Some(state) = &item.reminder_state {
            state.attempt_count + 1
        } else {
            1
        };
        item.reminder_state = Some(ReminderState {
            delivery_state: e.receipt.state,
            attempt_count,
            last_acknowledged_at: None,
            policy: ReminderPolicy::default(),
            non_clinical_disclosure: NON_CLINICAL_DISCLOSURE.to_owned(),
        });
    }
}

fn handle_reminder_acknowledged(
    e: &ReminderAcknowledgedEvent,
    items: &mut BTreeMap<Uuid, ProjectedItem>,
) {
    for item in items.values_mut() {
        if let Some(state) = &mut item.reminder_state {
            state.delivery_state = e.state;
            state.last_acknowledged_at = Some(e.occurred_at);
            item.reminder_acknowledged_at = Some(e.occurred_at.to_rfc3339());
        }
    }
}

fn bucket_item(
    item: &ProjectedItem,
    projection: &mut PersonalOpsProjection,
    operator_local_date: chrono::NaiveDate,
) {
    if item.current_state == ItemState::Completed {
        projection.completed.push(clone_item(item));
        return;
    }

    let scheduled_today = item.scheduled_at.as_deref().and_then(|s| {
        s.parse::<DateTime<Utc>>()
            .ok()
            .map(|dt| dt.naive_utc().date())
    });
    let due_today = item.due_at.as_deref().and_then(|s| {
        s.parse::<DateTime<Utc>>()
            .ok()
            .map(|dt| dt.naive_utc().date())
    });

    let is_today_scheduled = scheduled_today == Some(operator_local_date);
    let is_today_due = due_today == Some(operator_local_date);

    if is_today_scheduled || is_today_due {
        projection.today.push(clone_item(item));
    } else if item.scheduled_at.is_none() && item.due_at.is_none() {
        // Active items with no schedule are treated as "today" (current work).
        projection.today.push(clone_item(item));
    }

    let is_waiting = item.reminder_state.as_ref().map_or(false, |state| {
        matches!(
            state.delivery_state,
            ReminderDeliveryState::Attempted | ReminderDeliveryState::Deferred
        )
    });

    if is_waiting && !is_today_scheduled && !is_today_due {
        projection.waiting.push(clone_item(item));
    }

    let is_future = item.scheduled_at.as_deref().and_then(|s| {
        s.parse::<DateTime<Utc>>()
            .ok()
            .map(|dt| dt.naive_utc().date())
    }) != Some(operator_local_date)
        && item.due_at.as_deref().and_then(|s| {
            s.parse::<DateTime<Utc>>()
                .ok()
                .map(|dt| dt.naive_utc().date())
        }) != Some(operator_local_date)
        && (item.scheduled_at.is_some() || item.due_at.is_some());

    if is_future && !is_today_scheduled && !is_today_due {
        projection.scheduled.push(clone_item(item));
    }
}

fn clone_item(item: &ProjectedItem) -> ProjectedItem {
    ProjectedItem {
        item_id: item.item_id,
        kind: item.kind,
        operator_id: item.operator_id.clone(),
        content: item.content.clone(),
        evidence_class: item.evidence_class,
        confidence: item.confidence,
        classification_reason: item.classification_reason,
        scheduled_at: item.scheduled_at.clone(),
        due_at: item.due_at.clone(),
        completed_at: item.completed_at.clone(),
        reminder_state: item.reminder_state.clone(),
        reminder_attempts: item.reminder_attempts,
        reminder_acknowledged_at: item.reminder_acknowledged_at.clone(),
        current_state: item.current_state,
    }
}
