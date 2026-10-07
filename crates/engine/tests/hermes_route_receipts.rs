use arda_engine::adapters::HermesExecutionReceipt;
use arda_workbench_executor::compatibility::CanonicalHermesExecutionReceipt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

// Frozen v3 encoding, in the original typed serializer's field order.
const LEGACY_UNSIGNED: &str = r#"{"schema_version":"arda.execution-receipt.v3","receipt_digest":"","authority_binding_digest":"sha256:authority","run_id":"run","node_id":"execute","idempotency_key":"key","status":"succeeded","summary":"fixture","tool_evidence":[],"test_evidence":[],"artifacts":[],"usage":{"provider":"retained-manwe","model":"fixture-model","api_calls":1,"input_tokens":1,"output_tokens":1,"total_tokens":2,"estimated_cost_usd":0.0,"cost_measurement":"unknown","completed":true,"failed":false},"adapter":"hermes-workbench","adapter_version":"1","project_contract_digest":"contract","parent_receipts":[],"recorded_at_unix_ms":123}"#;

fn legacy() -> Value {
    serde_json::from_str(LEGACY_UNSIGNED).unwrap()
}

fn attested() -> Value {
    let mut value = legacy();
    value["schema_version"] = json!("arda.execution-receipt.v4");
    value["inference_routes"] = json!({
        "profile_digest":format!("sha256:{}", "1".repeat(64)),
        "selected_provider":"fixture-upstream", "selected_model":"fixture-model",
        "routes":[{"route_id":"route-one", "provider_id":"fixture-upstream",
                   "model_id":"fixture-model", "route_class":"tool_oriented"}]
    });
    value
}

#[test]
fn historical_receipt_digest_and_serialization_are_unchanged() {
    let mut value = legacy();
    let expected = format!("sha256:{:x}", Sha256::digest(LEGACY_UNSIGNED.as_bytes()));
    value["receipt_digest"] = json!(expected);
    let receipt: HermesExecutionReceipt = serde_json::from_value(value.clone()).unwrap();
    assert!(receipt.has_valid_digest().unwrap());
    assert_eq!(serde_json::to_value(&receipt).unwrap(), value);
    let reader: CanonicalHermesExecutionReceipt = serde_json::from_value(value).unwrap();
    assert!(reader.has_valid_digest().unwrap());
    assert_eq!(
        reader.computed_digest().unwrap(),
        receipt.computed_digest().unwrap()
    );
}

#[test]
fn persisted_route_evidence_survives_reopen_and_binds_the_digest() {
    let mut receipt: HermesExecutionReceipt = serde_json::from_value(attested()).unwrap();
    receipt.receipt_digest = receipt.computed_digest().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("run/execution-receipts/execute.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    let mut reopened: HermesExecutionReceipt =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(reopened.has_valid_digest().unwrap());
    arda_workbench_executor::compatibility::exact_pre_review_receipt(
        &path,
        "execute",
        &reopened.receipt_digest,
    )
    .unwrap();
    assert_eq!(reopened.usage.provider.as_deref(), Some("retained-manwe"));
    assert_eq!(
        reopened.inference_routes.as_ref().unwrap().routes[0].provider_id,
        "fixture-upstream"
    );
    let downstream: CanonicalHermesExecutionReceipt =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(downstream.has_valid_digest().unwrap());
    assert_eq!(
        downstream.computed_digest().unwrap(),
        reopened.computed_digest().unwrap()
    );
    reopened.inference_routes.as_mut().unwrap().routes[0].route_id = "altered".into();
    assert!(!reopened.has_valid_digest().unwrap());
}

#[test]
fn invalid_route_claims_fail_even_with_a_recomputed_digest() {
    for change in [
        "missing",
        "empty",
        "provider",
        "model",
        "class",
        "profile",
        "downgrade",
        "export-model",
    ] {
        let mut value = attested();
        match change {
            "missing" => {
                value.as_object_mut().unwrap().remove("inference_routes");
            }
            "empty" => value["inference_routes"]["routes"] = json!([]),
            "provider" => value["inference_routes"]["routes"][0]["provider_id"] = json!("wrong"),
            "model" => value["inference_routes"]["routes"][0]["model_id"] = json!("wrong"),
            "class" => value["inference_routes"]["routes"][0]["route_class"] = json!("interactive"),
            "profile" => value["inference_routes"]["profile_digest"] = json!("bad"),
            "downgrade" => value["schema_version"] = json!("arda.execution-receipt.v3"),
            "export-model" => value["usage"]["model"] = json!("wrong"),
            _ => unreachable!(),
        }
        let mut receipt: HermesExecutionReceipt = serde_json::from_value(value).unwrap();
        receipt.receipt_digest = receipt.computed_digest().unwrap();
        assert!(!receipt.has_valid_digest().unwrap(), "{change}");
        let reader: CanonicalHermesExecutionReceipt =
            serde_json::from_value(serde_json::to_value(receipt).unwrap()).unwrap();
        assert!(!reader.has_valid_digest().unwrap(), "{change}");
    }
}
