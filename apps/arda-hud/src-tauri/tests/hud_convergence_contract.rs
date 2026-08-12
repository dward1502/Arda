use serde_json::Value;

fn shared_contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../../spec/hud-convergence/v1/fixtures/valid-shared-contract.json"
    ))
    .expect("shared HUD convergence contract fixture")
}

#[test]
fn tauri_boundary_uses_backend_receipts_without_creating_authority() {
    let contract = shared_contract();
    let intent = contract["mutation"]["intent"]
        .as_object()
        .expect("mutation intent object");
    let receipt = contract["mutation"]["receipt"]
        .as_object()
        .expect("mutation receipt object");

    assert_eq!(intent["intent_id"], receipt["intent_id"]);
    assert_eq!(intent["operator_id"], receipt["operator_id"]);
    assert_eq!(intent["action"], receipt["action"]);
    assert_eq!(receipt["authority_owner"], "arda-engine");
    assert!(!intent.contains_key("receipt_id"));
    assert!(!intent.contains_key("durable_reference"));
    assert_eq!(
        contract["error_envelope"]["schema_version"],
        "arda.hud.error.v1"
    );
    assert_eq!(contract["error_envelope"]["status"], "failed");
}

#[test]
fn tauri_boundary_pins_per_run_stream_recovery() {
    let contract = shared_contract();
    let stream = &contract["event_stream"];

    assert_eq!(stream["run_id"], "run-42");
    assert_eq!(stream["reconnect_from_cursor"], true);
    assert_eq!(
        stream["gap_policy"],
        "reload_durable_run_before_accepting_later_events"
    );
    assert_eq!(stream["ownership"], "one_independent_stream_per_run");
    assert_eq!(stream["durable_recovery"], "backend_run_store");
}
