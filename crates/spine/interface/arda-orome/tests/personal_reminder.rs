//! Tests for the personal reminder routing adapter.
//!
//! Verifies that "Attempted" and "Delivered" are never conflated,
//! fatigue caps, quiet windows, and snooze/dismiss state are respected.

use arda_orome::personal_reminder::{
    acknowledgement_receipt, delivered_receipt, evaluate_reminder_routing, suppressed_receipt,
    ReminderRoutingState, RoutingDecision,
};
use arda_orome::types::{PersonalReminderDeliveryState, PersonalReminderRequest};
use chrono::{TimeZone, Utc};

fn base_request() -> PersonalReminderRequest {
    PersonalReminderRequest {
        schema_version: "arda.orome.personal-reminder.v1".to_string(),
        reminder_id: "rem-001".to_string(),
        item_id: "item-001".to_string(),
        operator_id: "operator-0".to_string(),
        subject: "Medication reminder".to_string(),
        body: "Take your evening meds".to_string(),
        provider: "discord".to_string(),
        channel: "direct".to_string(),
        attempt_number: 0,
        max_attempts: 3,
        quiet_mode: false,
        snoozed_until_utc: None,
        created_at_utc: "2026-08-02T10:00:00Z".to_string(),
    }
}

fn base_state() -> ReminderRoutingState {
    ReminderRoutingState {
        item_id: "item-001".to_string(),
        provider: "discord".to_string(),
        channel: "direct".to_string(),
        attempt_count: 0,
        max_attempts: 3,
        last_attempted_at: None,
        snoozed_until_utc: None,
        dismissed: false,
    }
}

#[test]
fn deliver_when_no_suppression() {
    let req = base_request();
    let state = base_state();
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    assert_eq!(decision, RoutingDecision::Deliver);
}

#[test]
fn suppressed_when_quiet_mode() {
    let mut req = base_request();
    req.quiet_mode = true;
    let state = base_state();
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    match decision {
        RoutingDecision::Suppressed { reason, .. } => {
            assert_eq!(reason, "quiet_mode_active");
        }
        _ => panic!("expected Suppressed, got {:?}", decision),
    }
}

#[test]
fn suppressed_when_snoozed() {
    let req = base_request();
    let mut state = base_state();
    state.snoozed_until_utc = Some("2026-08-02T11:00:00Z".to_string());
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    match decision {
        RoutingDecision::Suppressed { reason, .. } => {
            assert_eq!(reason, "snoozed_until_window_passes");
        }
        _ => panic!("expected Suppressed, got {:?}", decision),
    }
}

#[test]
fn deliver_after_snooze_window_passes() {
    let req = base_request();
    let mut state = base_state();
    state.snoozed_until_utc = Some("2026-08-02T10:00:00Z".to_string());
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    assert_eq!(decision, RoutingDecision::Deliver);
}

#[test]
fn suppressed_within_minimum_interval() {
    let req = base_request();
    let mut state = base_state();
    state.last_attempted_at = Some("2026-08-02T10:25:00Z".to_string());
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    match decision {
        RoutingDecision::Suppressed { reason, .. } => {
            assert_eq!(reason, "within_minimum_interval");
        }
        _ => panic!("expected Suppressed, got {:?}", decision),
    }
}

#[test]
fn deliver_after_minimum_interval() {
    let req = base_request();
    let mut state = base_state();
    state.last_attempted_at = Some("2026-08-02T10:10:00Z".to_string());
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    assert_eq!(decision, RoutingDecision::Deliver);
}

#[test]
fn retired_when_max_attempts_exhausted() {
    let req = base_request();
    let mut state = base_state();
    state.attempt_count = 3;
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    match decision {
        RoutingDecision::Retired { reason } => {
            assert_eq!(reason, "max_attempts_exhausted");
        }
        _ => panic!("expected Retired, got {:?}", decision),
    }
}

#[test]
fn retired_when_dismissed() {
    let req = base_request();
    let mut state = base_state();
    state.dismissed = true;
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let decision = evaluate_reminder_routing(&req, &state, now);
    match decision {
        RoutingDecision::Retired { reason } => {
            assert_eq!(reason, "dismissed_by_operator");
        }
        _ => panic!("expected Retired, got {:?}", decision),
    }
}

#[test]
fn delivered_receipt_marks_state_delivered_when_provider_message_id_present() {
    let req = base_request();
    let state = base_state();
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let receipt = delivered_receipt(&req, &state, Some("provider-msg-123".to_string()), now);

    assert_eq!(receipt.state, PersonalReminderDeliveryState::Delivered);
    assert_eq!(
        receipt.provider_message_id.as_deref(),
        Some("provider-msg-123")
    );
    assert!(!receipt.suppressed);
    assert_eq!(receipt.attempt_number, 1);
}

#[test]
fn delivered_receipt_marks_state_attempted_when_no_provider_message_id() {
    let req = base_request();
    let state = base_state();
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let receipt = delivered_receipt(&req, &state, None, now);

    assert_eq!(receipt.state, PersonalReminderDeliveryState::Attempted);
    assert!(receipt.provider_message_id.is_none());
    assert!(!receipt.suppressed);
}

#[test]
fn suppressed_receipt_has_suppressed_flag_and_error() {
    let req = base_request();
    let state = base_state();
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let receipt = suppressed_receipt(&req, &state, "quiet_mode_active", now);

    assert_eq!(receipt.state, PersonalReminderDeliveryState::Attempted);
    assert!(receipt.suppressed);
    assert_eq!(receipt.error.as_deref(), Some("quiet_mode_active"));
    assert!(receipt.provider_message_id.is_none());
}

#[test]
fn acknowledgement_receipt_preserves_attempt_count() {
    let req = base_request();
    let mut state = base_state();
    state.attempt_count = 2;
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let receipt = acknowledgement_receipt(
        &req,
        &state,
        PersonalReminderDeliveryState::Acknowledged,
        Some("ack-123".to_string()),
        now,
    );

    assert_eq!(receipt.state, PersonalReminderDeliveryState::Acknowledged);
    assert_eq!(receipt.attempt_number, 2);
    assert!(!receipt.suppressed);
}

#[test]
fn non_clinical_disclosure_present_for_health_reminder() {
    let mut req = base_request();
    req.body = "Time for your blood pressure check".to_string();
    let state = base_state();
    let now = Utc.with_ymd_and_hms(2026, 8, 2, 10, 30, 0).unwrap();

    let receipt = delivered_receipt(&req, &state, Some("msg-1".to_string()), now);
    let disclosure = receipt.non_clinical_disclosure.expect("disclosure present");
    assert!(disclosure.contains("not clinical measurement or medical advice"));
}
