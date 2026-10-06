//! Migration-shape and corrupt-binding qualification for original admissions.

#[test]
fn legacy_project_bindings_replay_without_rewriting_admission_or_digest() {
    #[derive(serde::Serialize)]
    struct LegacyProject<'a> {
        project_id: &'a str,
        contract_digest: &'a str,
    }
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("legacy-project", "legacy-project-key");
    let before = store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    let mut encoded = serde_json::to_string(&input).unwrap();
    for project in &input.projects {
        encoded = encoded.replace(
            &serde_json::to_string(project).unwrap(),
            &serde_json::to_string(&LegacyProject {
                project_id: &project.project_id,
                contract_digest: &project.contract_digest,
            })
            .unwrap(),
        );
    }
    let digest = format!("sha256:{:x}", Sha256::digest(encoded.as_bytes()));
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "UPDATE objective_admissions SET input_json=?1 WHERE objective_id=?2",
        rusqlite::params![encoded, input.id],
    )
    .unwrap();
    db.execute(
        "UPDATE objectives SET payload_digest=?1 WHERE id=?2",
        rusqlite::params![digest, input.id],
    )
    .unwrap();
    drop(store);
    let store = ObjectiveStore::open_existing(&path).unwrap();
    let recovered = store
        .authenticated_admission(&input.idempotency_key, &input.operator_id)
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_string(&recovered).unwrap(), encoded);
    assert_eq!(
        store
            .create_authenticated_objective(recovered, 200)
            .unwrap(),
        before
    );
    let retained: (String, String) = db.query_row("SELECT a.input_json,o.payload_digest FROM objective_admissions a JOIN objectives o ON a.objective_id=o.id WHERE o.id=?1", [&input.id], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(retained, (encoded, digest));
}
use super::*;
use sha2::{Digest, Sha256};

#[test]
fn project_metadata_rejects_blank_checks_without_creating_admission() {
    for checks in [
        vec!["".into()],
        vec![" \t".into()],
        vec!["build".into(), "".into()],
    ] {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        let mut input = objective("blank-check", "blank-check-key");
        input.projects[0].checks = checks;
        let error = store
            .create_authenticated_objective(input, 100)
            .unwrap_err();
        assert!(
            error.to_string().contains("project authority metadata"),
            "{error}"
        );
        let db = rusqlite::Connection::open(path).unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM objective_admissions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[test]
fn populated_project_metadata_roundtrips_digest_and_conflicts_on_changed_replay() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let input = objective("metadata", "metadata-key");
    let original = store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    let encoded = serde_json::to_string(&input).unwrap();
    drop(store);
    let store = ObjectiveStore::open_existing(&path).unwrap();
    let saved = store
        .authenticated_admission(&input.idempotency_key, &input.operator_id)
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_string(&saved).unwrap(), encoded);
    assert_eq!(
        store.create_authenticated_objective(saved, 200).unwrap(),
        original
    );
    for authority in [true, false] {
        let mut changed = input.clone();
        if authority {
            changed.projects[0].authority.push_str("-changed");
        } else {
            changed.projects[0].checks.push("additional-check".into());
        }
        assert!(store.create_authenticated_objective(changed, 300).is_err());
    }
    let db = rusqlite::Connection::open(path).unwrap();
    let retained:(String,String)=db.query_row("SELECT a.input_json,o.payload_digest FROM objective_admissions a JOIN objectives o ON o.id=a.objective_id",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(
        retained,
        (
            encoded.clone(),
            format!("sha256:{:x}", Sha256::digest(encoded.as_bytes()))
        )
    );
}

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
