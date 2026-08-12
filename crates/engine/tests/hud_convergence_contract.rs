use serde_json::Value;

fn shared_contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../spec/hud-convergence/v1/fixtures/valid-shared-contract.json"
    ))
    .expect("shared HUD convergence contract fixture")
}

#[test]
fn shared_fixture_pins_engine_authority_and_durable_recovery() {
    let contract = shared_contract();

    assert_eq!(contract["identity"]["authority_owner"], "arda-engine");
    assert_eq!(
        contract["mutation"]["receipt"]["authority_owner"],
        "arda-engine"
    );
    assert_eq!(
        contract["workbench"]["plan_result"]["authority_owner"],
        "arda-engine"
    );
    assert_eq!(
        contract["event_stream"]["durable_recovery"],
        "backend_run_store"
    );
    assert_eq!(
        contract["error_envelope"]["schema_version"],
        "arda.hud.error.v1"
    );
    assert_eq!(contract["error_envelope"]["status"], "failed");

    let intent = contract["mutation"]["intent"]
        .as_object()
        .expect("mutation intent object");
    assert!(!intent.contains_key("policy_decision"));
    assert!(!intent.contains_key("recorded_at_utc"));
    assert!(!intent.contains_key("receipt_id"));
}

#[test]
fn shared_fixture_pins_all_seven_projection_states() {
    let contract = shared_contract();
    let states = contract["load_states"]
        .as_array()
        .expect("load states")
        .iter()
        .map(|state| state["status"].as_str().expect("status"))
        .collect::<Vec<_>>();

    assert_eq!(
        states,
        [
            "loading",
            "healthy",
            "stale",
            "partial",
            "degraded",
            "unavailable",
            "failed",
        ]
    );
}
