use super::*;
use crate::onboarding::constants::ONBOARDING_APPROVAL_CONTRACT;
use crate::onboarding::helpers::{action_is_approved, secret_safe_preview};
use crate::onboarding::private_config::merge_private_env;
use serde_json::{json, Value};
use std::path::Path;

fn approval(scope: Vec<&str>, approved: bool) -> ApprovalReceipt {
    ApprovalReceipt {
        contract: ONBOARDING_APPROVAL_CONTRACT.to_string(),
        approved,
        approver: "operator".to_string(),
        reason: "test".to_string(),
        approved_scope: scope.into_iter().map(str::to_string).collect(),
        approved_at_utc: "2026-06-02T00:00:00Z".to_string(),
        notes: None,
    }
}

#[test]
fn human_gate_requires_explicit_approval() {
    assert!(!action_is_approved(None, "onboarding.set_manwe_endpoint"));
    assert!(!action_is_approved(
        Some(&approval(vec!["onboarding.set_manwe_endpoint"], false)),
        "onboarding.set_manwe_endpoint"
    ));
    assert!(action_is_approved(
        Some(&approval(vec!["onboarding.set_manwe_endpoint"], true)),
        "onboarding.set_manwe_endpoint"
    ));
    assert!(action_is_approved(
        Some(&approval(vec!["all"], true)),
        "onboarding.set_manwe_endpoint"
    ));
}

#[test]
fn service_action_contract_round_trips() {
    let action = ServiceAction {
        action_id: "onboarding.emit_profile".to_string(),
        action_type: "emit_projection".to_string(),
        title: "Emit profile".to_string(),
        command_hint: "ARDA onboarding detect --write".to_string(),
        target_path: Some("core/state/environment_profile.json".to_string()),
        requires_human_gate: false,
        description: "test".to_string(),
        risk: "read_only".to_string(),
    };
    let encoded = serde_json::to_string(&action).expect("serialize action");
    let decoded: ServiceAction = serde_json::from_str(&encoded).expect("deserialize action");
    assert_eq!(decoded.action_id, "onboarding.emit_profile");
    assert!(!decoded.requires_human_gate);
}

#[test]
fn secret_preview_never_exposes_secret_value() {
    let (preview, present, source) = secret_safe_preview(
        "TEST_API_KEY",
        Some("super-secret-token".to_string()),
        true,
        "",
    );
    assert!(present);
    assert!(matches!(source, ValueSource::Environment));
    assert!(!preview.contains("super-secret-token"));
    assert!(preview.starts_with("<secret-present"));
}

#[test]
fn private_env_merge_skips_secrets_and_preserves_unknown_lines() {
    let entries = vec![
        PrivateConfigEntry {
            key: "ARDA_PROFILE".to_string(),
            value_preview: "local".to_string(),
            source: ValueSource::Default,
            required: true,
            secret: false,
            present: true,
            recommendation: "test".to_string(),
        },
        PrivateConfigEntry {
            key: "OPENAI_API_KEY".to_string(),
            value_preview: "<secret-present:10 chars>".to_string(),
            source: ValueSource::Environment,
            required: false,
            secret: true,
            present: true,
            recommendation: "test".to_string(),
        },
    ];
    let existing = "# keep me\nCUSTOM_FLAG=yes\nARDA_PROFILE=\"old\"\n";
    let (merged, changed) = merge_private_env(existing, &entries);

    assert!(merged.contains("# keep me"));
    assert!(merged.contains("CUSTOM_FLAG=yes"));
    assert!(merged.contains("ARDA_PROFILE=\"local\""));
    assert!(!merged.contains("OPENAI_API_KEY"));
    assert_eq!(changed, vec!["ARDA_PROFILE".to_string()]);
}

#[test]
fn l3_readiness_checklist_preserves_human_gates() {
    let checklist = l3_readiness_onboarding_checklist();
    let gates = checklist
        .get("human_gates")
        .and_then(Value::as_array)
        .expect("human gates");
    assert!(gates
        .iter()
        .any(|gate| gate.as_str().unwrap_or("").contains("destructive")));
    assert!(gates
        .iter()
        .any(|gate| gate.as_str().unwrap_or("").contains("credential")));
    assert_eq!(
        checklist.pointer("/low_power_route_posture/default"),
        Some(&json!(
            "capability_and_context_headroom_before_local_preference"
        ))
    );
}

#[test]
fn guided_session_includes_l3_readiness_step() {
    let root = Path::new(".");
    let profile =
        build_environment_profile(Some(root), Some("local"), Some("workstation")).expect("profile");
    let session = build_guided_session(&profile, root, None);
    let l3_step = session
        .steps
        .iter()
        .find(|step| step.step_id == "l3_readiness")
        .expect("l3 step");
    assert_eq!(l3_step.status, "human_gated");
    assert!(l3_step
        .evidence
        .iter()
        .any(|entry| entry.contains("l3_readiness_projection")));
}
