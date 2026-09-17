//! Migration-shape and corrupt-binding qualification for original admissions.
use super::*;
use sha2::{Digest, Sha256};

#[test]
fn pre_admission_table_schema_upgrades_without_inventing_historical_inputs() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("legacy-admission", "legacy-key");
    let before = store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    drop(store);
    // Build the pre-extension schema shape while retaining the provisioned
    // authority and existing canonical objective. This is not a historical binary.
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("DROP TABLE objective_admissions").unwrap();
    drop(db);
    let reopened = ObjectiveStore::open_existing(&path).unwrap();
    assert!(reopened
        .authenticated_admission(&input.idempotency_key, &input.operator_id)
        .is_err());
    assert_eq!(
        reopened
            .create_authenticated_objective(input.clone(), 200)
            .unwrap(),
        before
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM objective_admissions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        count, 0,
        "identical legacy replay must not backfill mutable state"
    );
    assert!(reopened
        .authenticated_admission(&input.idempotency_key, &input.operator_id)
        .is_err());
}

#[test]
fn admission_identity_bindings_are_checked_even_with_a_matching_payload_digest() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("binding", "binding-key");
    store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    for field in ["id", "idempotency_key", "operator_id"] {
        let mut changed = serde_json::to_value(&input).unwrap();
        changed[field] = "different".into();
        let changed: NewObjective = serde_json::from_value(changed).unwrap();
        // Compute the same canonical digest as production. These intentionally
        // inconsistent local records must fail the separate identity checks.
        let digest = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&changed).unwrap())
        );
        db.execute(
            "UPDATE objective_admissions SET input_json=?1 WHERE objective_id='binding'",
            [serde_json::to_string(&changed).unwrap()],
        )
        .unwrap();
        db.execute(
            "UPDATE objectives SET payload_digest=?1 WHERE id='binding'",
            [digest],
        )
        .unwrap();
        let error = store
            .authenticated_admission(&input.idempotency_key, &input.operator_id)
            .unwrap_err();
        assert!(
            error.to_string().contains("integrity mismatch"),
            "{field}: {error}"
        );
    }
    db.execute(
        "UPDATE objective_admissions SET input_json='not json' WHERE objective_id='binding'",
        [],
    )
    .unwrap();
    assert!(store
        .authenticated_admission(&input.idempotency_key, &input.operator_id)
        .unwrap_err()
        .to_string()
        .contains("decode original objective admission"));
}
