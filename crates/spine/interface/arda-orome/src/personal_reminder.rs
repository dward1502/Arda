//! Personal reminder delivery adapter.
//!
//! Routes personal-ops reminders through Oromë's dispatch surface.
//! Key invariant: "Attempted" and "Delivered" are never conflated.
//! Repeated reminders respect fatigue caps, quiet windows, and
//! explicit snooze/dismiss state.
//!
//! This module lives under `service/` and is gated behind
//! `service-runtime` since it depends on `HermesService`'s dispatch.

use crate::types::{
    PersonalReminderDeliveryState, PersonalReminderReceipt, PersonalReminderRequest,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Non-clinical wellness disclosure used on all health-related
/// reminder receipts.
const NON_CLINICAL_DISCLOSURE: &str =
    "Wellness assistance only; this record is not clinical measurement or medical advice.";

/// Determine whether a reminder relates to health/medication,
/// warranting the non-clinical disclosure on its receipt.
fn is_health_related(state: &ReminderRoutingState, request: &PersonalReminderRequest) -> bool {
    state.provider.contains("health")
        || request.body.to_lowercase().contains("medication")
        || request.body.to_lowercase().contains("health")
        || request.body.to_lowercase().contains("blood pressure")
        || request.subject.to_lowercase().contains("health")
        || request.subject.to_lowercase().contains("medication")
}

/// Fatigue-capping and quiet-window state for a single reminder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderRoutingState {
    pub item_id: String,
    pub provider: String,
    pub channel: String,
    pub attempt_count: u32,
    pub max_attempts: u32,
    pub last_attempted_at: Option<String>,
    /// ISO-8601 timestamp after which delivery may resume.
    pub snoozed_until_utc: Option<String>,
    /// True if the operator explicitly dismissed this reminder.
    pub dismissed: bool,
}

impl Default for ReminderRoutingState {
    fn default() -> Self {
        Self {
            item_id: String::new(),
            provider: String::new(),
            channel: String::new(),
            attempt_count: 0,
            max_attempts: 3,
            last_attempted_at: None,
            snoozed_until_utc: None,
            dismissed: false,
        }
    }
}

/// Decide whether a reminder should be delivered now, suppressed,
/// or retired based on fatigue, quiet mode, and snooze state.
pub fn evaluate_reminder_routing(
    request: &PersonalReminderRequest,
    state: &ReminderRoutingState,
    now: DateTime<Utc>,
) -> RoutingDecision {
    // Explicit dismissal short-circuits everything.
    if state.dismissed {
        return RoutingDecision::Retired {
            reason: "dismissed_by_operator".to_string(),
        };
    }

    // Quiet mode suppresses delivery but keeps the reminder retrying.
    if request.quiet_mode {
        return RoutingDecision::Suppressed {
            reason: "quiet_mode_active".to_string(),
            suppressed_at: now,
        };
    }

    // Snooze / defer: skip until the snooze window passes.
    if let Some(snooze_str) = &state.snoozed_until_utc {
        if let Ok(snooze_until) = snooze_str.parse::<DateTime<Utc>>() {
            if now < snooze_until {
                return RoutingDecision::Suppressed {
                    reason: "snoozed_until_window_passes".to_string(),
                    suppressed_at: now,
                };
            }
        }
    }

    // Fatigue cap: max_attempts reached.
    if state.attempt_count >= state.max_attempts {
        return RoutingDecision::Retired {
            reason: "max_attempts_exhausted".to_string(),
        };
    }

    // Minimum interval: if last attempt was too recent, suppress.
    if let Some(last_str) = &state.last_attempted_at {
        if let Ok(last) = last_str.parse::<DateTime<Utc>>() {
            let elapsed = now.signed_duration_since(last);
            // Minimum interval is 15 minutes by default (from ReminderPolicy)
            if elapsed.num_minutes() < 15 {
                return RoutingDecision::Suppressed {
                    reason: "within_minimum_interval".to_string(),
                    suppressed_at: now,
                };
            }
        }
    }

    RoutingDecision::Deliver
}

/// Outcome of the routing evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    /// Proceed with delivery attempt.
    Deliver,
    /// Temporarily suppressed (quiet mode, snooze, minimum interval).
    /// The reminder remains active and will be retried.
    Suppressed {
        reason: String,
        suppressed_at: DateTime<Utc>,
    },
    /// Permanently retired (dismissed or exhausted retries).
    Retired { reason: String },
}

/// Build a receipt for a delivery attempt that was suppressed.
pub fn suppressed_receipt(
    request: &PersonalReminderRequest,
    state: &ReminderRoutingState,
    reason: &str,
    now: DateTime<Utc>,
) -> PersonalReminderReceipt {
    PersonalReminderReceipt {
        schema_version: request.schema_version.clone(),
        reminder_id: request.reminder_id.clone(),
        item_id: request.item_id.clone(),
        attempted_at_utc: now.to_rfc3339(),
        state: PersonalReminderDeliveryState::Attempted,
        attempt_number: state.attempt_count + 1,
        max_attempts: state.max_attempts,
        provider: request.provider.clone(),
        channel: request.channel.clone(),
        provider_message_id: None,
        error: Some(reason.to_string()),
        quiet_mode_active: request.quiet_mode,
        suppressed: true,
        non_clinical_disclosure: is_health_related(state, request)
            .then(|| NON_CLINICAL_DISCLOSURE.to_string()),
    }
}

/// Build a receipt for a delivery attempt that was dispatched.
pub fn delivered_receipt(
    request: &PersonalReminderRequest,
    state: &ReminderRoutingState,
    provider_message_id: Option<String>,
    now: DateTime<Utc>,
) -> PersonalReminderReceipt {
    let state_enum = if provider_message_id.is_some() {
        PersonalReminderDeliveryState::Delivered
    } else {
        PersonalReminderDeliveryState::Attempted
    };

    PersonalReminderReceipt {
        schema_version: request.schema_version.clone(),
        reminder_id: request.reminder_id.clone(),
        item_id: request.item_id.clone(),
        attempted_at_utc: now.to_rfc3339(),
        state: state_enum,
        attempt_number: state.attempt_count + 1,
        max_attempts: state.max_attempts,
        provider: request.provider.clone(),
        channel: request.channel.clone(),
        provider_message_id,
        error: None,
        quiet_mode_active: request.quiet_mode,
        suppressed: false,
        non_clinical_disclosure: is_health_related(state, request)
            .then(|| NON_CLINICAL_DISCLOSURE.to_string()),
    }
}

/// Build a receipt for an acknowledgement after delivery.
pub fn acknowledgement_receipt(
    request: &PersonalReminderRequest,
    state: &ReminderRoutingState,
    new_state: PersonalReminderDeliveryState,
    provider_message_id: Option<String>,
    now: DateTime<Utc>,
) -> PersonalReminderReceipt {
    PersonalReminderReceipt {
        schema_version: request.schema_version.clone(),
        reminder_id: request.reminder_id.clone(),
        item_id: request.item_id.clone(),
        attempted_at_utc: now.to_rfc3339(),
        state: new_state,
        attempt_number: state.attempt_count,
        max_attempts: state.max_attempts,
        provider: request.provider.clone(),
        channel: request.channel.clone(),
        provider_message_id,
        error: None,
        quiet_mode_active: request.quiet_mode,
        suppressed: false,
        non_clinical_disclosure: is_health_related(state, request)
            .then(|| NON_CLINICAL_DISCLOSURE.to_string()),
    }
}
