use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PERSONAL_OPS_SCHEMA_VERSION: &str = "arda.personal-ops.v1";
pub const NON_CLINICAL_DISCLOSURE: &str =
    "Wellness assistance only; this record is not clinical measurement or medical advice.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalOpsEnvelope<T> {
    pub schema_version: String,
    pub record: T,
}

impl<T> PersonalOpsEnvelope<T> {
    pub fn new(record: T) -> Self {
        Self {
            schema_version: PERSONAL_OPS_SCHEMA_VERSION.to_owned(),
            record,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PersonalOpsError {
    #[error("capture requires non-empty text or an audio reference")]
    MissingCaptureContent,
    #[error("operator-authored field `{field}` cannot be overwritten by inference")]
    OperatorAuthoredFieldProtected { field: &'static str },
    #[error("classification confidence must be between 0.0 and 1.0")]
    InvalidConfidence,
    #[error("classification change no longer matches the current item state")]
    StaleClassificationChange,
    #[error("reminder policy must be bounded")]
    UnboundedReminderPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureSource {
    Text,
    Audio,
    Import,
    Device,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureContent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_reference: Option<String>,
}

impl CaptureContent {
    fn has_content(&self) -> bool {
        self.text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
            || self
                .audio_reference
                .as_deref()
                .is_some_and(|reference| !reference.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureAttachment {
    pub attachment_id: Uuid,
    pub media_type: String,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxCapture {
    pub capture_id: Uuid,
    pub captured_at: DateTime<Utc>,
    pub source: CaptureSource,
    pub content: CaptureContent,
    #[serde(default)]
    pub attachments: Vec<CaptureAttachment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
}

impl InboxCapture {
    pub fn validate(&self) -> Result<(), PersonalOpsError> {
        if !self.content.has_content() {
            return Err(PersonalOpsError::MissingCaptureContent);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    OperatorAuthored,
    Imported,
    Inferred,
    DeviceMeasured,
    SelfReported,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalItemKind {
    Note,
    Task,
    Reminder,
    Appointment,
    Contact,
    Health,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifiedField<T> {
    pub value: T,
    pub evidence_class: EvidenceClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalContextLink {
    pub relation: String,
    pub target_id: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalItem {
    pub item_id: Uuid,
    pub source_capture_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub content: CaptureContent,
    pub kind: ClassifiedField<PersonalItemKind>,
    #[serde(default)]
    pub context_links: Vec<PersonalContextLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_clinical_disclosure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KindClassificationChange {
    pub previous: PersonalItemKind,
    pub previous_evidence_class: EvidenceClass,
    pub current: PersonalItemKind,
    pub confidence: f32,
    pub rationale: String,
}

impl PersonalItem {
    pub fn from_capture(
        capture: InboxCapture,
        kind: PersonalItemKind,
        evidence_class: EvidenceClass,
    ) -> Result<Self, PersonalOpsError> {
        capture.validate()?;
        Ok(Self {
            item_id: Uuid::new_v4(),
            source_capture_id: capture.capture_id,
            created_at: capture.captured_at,
            content: capture.content,
            kind: ClassifiedField {
                value: kind,
                evidence_class,
                confidence: None,
                rationale: None,
            },
            context_links: Vec::new(),
            non_clinical_disclosure: (kind == PersonalItemKind::Health)
                .then(|| NON_CLINICAL_DISCLOSURE.to_owned()),
        })
    }

    pub fn apply_inferred_kind(
        &mut self,
        kind: PersonalItemKind,
        confidence: f32,
        rationale: impl Into<String>,
    ) -> Result<KindClassificationChange, PersonalOpsError> {
        if self.kind.evidence_class == EvidenceClass::OperatorAuthored {
            return Err(PersonalOpsError::OperatorAuthoredFieldProtected { field: "kind" });
        }
        if !(0.0..=1.0).contains(&confidence) {
            return Err(PersonalOpsError::InvalidConfidence);
        }
        let rationale = rationale.into();
        let change = KindClassificationChange {
            previous: self.kind.value,
            previous_evidence_class: self.kind.evidence_class,
            current: kind,
            confidence,
            rationale: rationale.clone(),
        };
        self.kind = ClassifiedField {
            value: kind,
            evidence_class: EvidenceClass::Inferred,
            confidence: Some(confidence),
            rationale: Some(rationale),
        };
        if kind == PersonalItemKind::Health {
            self.non_clinical_disclosure = Some(NON_CLINICAL_DISCLOSURE.to_owned());
        }
        Ok(change)
    }

    pub fn revert_kind(
        &mut self,
        change: KindClassificationChange,
    ) -> Result<(), PersonalOpsError> {
        if self.kind.value != change.current || self.kind.evidence_class != EvidenceClass::Inferred
        {
            return Err(PersonalOpsError::StaleClassificationChange);
        }
        self.kind = ClassifiedField {
            value: change.previous,
            evidence_class: change.previous_evidence_class,
            confidence: None,
            rationale: None,
        };
        if change.previous != PersonalItemKind::Health {
            self.non_clinical_disclosure = None;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietWindow {
    pub start_local: String,
    pub end_local: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionPolicy {
    Silent,
    QuietWindowAware,
    OperatorConfirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderPolicy {
    pub interruption: InterruptionPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_window: Option<QuietWindow>,
    pub max_attempts: u8,
    pub minimum_interval_minutes: u32,
    pub acknowledgement_required: bool,
}

impl Default for ReminderPolicy {
    fn default() -> Self {
        Self {
            interruption: InterruptionPolicy::QuietWindowAware,
            quiet_window: None,
            max_attempts: 3,
            minimum_interval_minutes: 15,
            acknowledgement_required: true,
        }
    }
}

impl ReminderPolicy {
    pub fn validate(&self) -> Result<(), PersonalOpsError> {
        if self.max_attempts == 0 || self.max_attempts > 5 || self.minimum_interval_minutes == 0 {
            return Err(PersonalOpsError::UnboundedReminderPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderDeliveryState {
    Attempted,
    Delivered,
    Acknowledged,
    Deferred,
    Dismissed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderReceipt {
    pub reminder_id: Uuid,
    pub item_id: Uuid,
    pub attempted_at: DateTime<Utc>,
    pub state: ReminderDeliveryState,
    pub channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_reference: Option<String>,
}

impl ReminderReceipt {
    pub fn was_delivered(&self) -> bool {
        matches!(
            self.state,
            ReminderDeliveryState::Delivered
                | ReminderDeliveryState::Acknowledged
                | ReminderDeliveryState::Deferred
                | ReminderDeliveryState::Dismissed
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextResumeCard {
    pub generated_at: DateTime<Utc>,
    pub summary: String,
    #[serde(default)]
    pub source_receipts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyBrief {
    pub generated_at: DateTime<Utc>,
    #[serde(default)]
    pub item_ids: Vec<Uuid>,
    #[serde(default)]
    pub source_receipts: Vec<String>,
    pub uncertainty_disclosure: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalOpsRecordType {
    CaptureRecorded,
    ItemClassified,
    ItemScheduled,
    ItemCompleted,
    ReminderAttempted,
    ReminderAcknowledged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRecordedEvent {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub operator_id: String,
    pub capture: InboxCapture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemClassifiedEvent {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub operator_id: String,
    pub item_id: Uuid,
    pub kind: PersonalItemKind,
    pub evidence_class: EvidenceClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemScheduledEvent {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub operator_id: String,
    pub item_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCompletedEvent {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub operator_id: String,
    pub item_id: Uuid,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderAttemptedEvent {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub operator_id: String,
    pub item_id: Uuid,
    pub receipt: ReminderReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderAcknowledgedEvent {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub operator_id: String,
    pub reminder_id: Uuid,
    pub state: ReminderDeliveryState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum PersonalOpsRecord {
    CaptureRecorded(CaptureRecordedEvent),
    ItemClassified(ItemClassifiedEvent),
    ItemScheduled(ItemScheduledEvent),
    ItemCompleted(ItemCompletedEvent),
    ReminderAttempted(ReminderAttemptedEvent),
    ReminderAcknowledged(ReminderAcknowledgedEvent),
}

impl PersonalOpsRecord {
    pub fn event_id(&self) -> Uuid {
        match self {
            Self::CaptureRecorded(e) => e.event_id,
            Self::ItemClassified(e) => e.event_id,
            Self::ItemScheduled(e) => e.event_id,
            Self::ItemCompleted(e) => e.event_id,
            Self::ReminderAttempted(e) => e.event_id,
            Self::ReminderAcknowledged(e) => e.event_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::CaptureRecorded(e) => e.occurred_at,
            Self::ItemClassified(e) => e.occurred_at,
            Self::ItemScheduled(e) => e.occurred_at,
            Self::ItemCompleted(e) => e.occurred_at,
            Self::ReminderAttempted(e) => e.occurred_at,
            Self::ReminderAcknowledged(e) => e.occurred_at,
        }
    }

    pub fn operator_id(&self) -> &str {
        match self {
            Self::CaptureRecorded(e) => &e.operator_id,
            Self::ItemClassified(e) => &e.operator_id,
            Self::ItemScheduled(e) => &e.operator_id,
            Self::ItemCompleted(e) => &e.operator_id,
            Self::ReminderAttempted(e) => &e.operator_id,
            Self::ReminderAcknowledged(e) => &e.operator_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Draft,
    Active,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationReason {
    OperatorInput,
    Inference,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderState {
    pub delivery_state: ReminderDeliveryState,
    pub attempt_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_acknowledged_at: Option<DateTime<Utc>>,
    pub policy: ReminderPolicy,
    pub non_clinical_disclosure: String,
}
