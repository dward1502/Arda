//! Unknown constraints must never disappear before admission/digest binding.
use super::*;
use arda_engine::objectives::LeafExecutionSpec;
use serde_json::{json, Value};

#[test]
fn approved_deadline_survives_restart_and_cannot_change_on_replay() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let mut input = objective("deadline", "deadline-ingress");
    let mut spec = execution();
    spec["execution_deadline_unix_ms"] = json!(3_600_100_u64);
    input.leaves[0].execution = Some(serde_json::from_value(spec).unwrap());
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    drop(store);
    let store = ObjectiveStore::open_existing(&path).unwrap();
    assert_eq!(
        store
            .authenticated_admission(&input.idempotency_key, &input.operator_id)
            .unwrap(),
        Some(input.clone())
    );
    for replacement in [None, Some(0), Some(7_200_100)] {
        let mut changed = input.clone();
        changed.leaves[0]
            .execution
            .as_mut()
            .unwrap()
            .execution_deadline_unix_ms = replacement;
        assert!(store.create_authenticated_objective(changed, 200).is_err());
    }
    store.create_authenticated_objective(input, 200).unwrap();
}

#[test]
fn legacy_admission_cannot_gain_deadline_on_replay() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    let mut input = objective("legacy-deadline", "legacy-deadline-ingress");
    input.leaves[0].execution = Some(serde_json::from_value(execution()).unwrap());
    store
        .create_authenticated_objective(input.clone(), 100)
        .unwrap();
    input.leaves[0]
        .execution
        .as_mut()
        .unwrap()
        .execution_deadline_unix_ms = Some(7_200_100);
    assert!(store.create_authenticated_objective(input, 200).is_err());
}

fn execution() -> Value {
    json!({
        "objective": "bounded remedy",
        "execution_prompt": "repair the approved defect",
        "verification_prompt": "verify the declared checks",
        "review_prompt": "independently challenge the result",
        "approval_envelope": {},
        "objective_plan_receipt": "sha256:fixture"
    })
}

#[test]
fn unsupported_execution_constraints_cannot_disappear_before_admission() {
    let base = execution();
    let decoded: LeafExecutionSpec = serde_json::from_value(base.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), base);
    for (field, value) in [
        ("budget", json!({"max_attempts": 2})),
        ("max_attempts", json!(2)),
        ("deadline_unix_ms", json!(3600000)),
        ("max_build_jobs", json!(2)),
        ("local_only", json!(true)),
    ] {
        let mut constrained = base.clone();
        constrained[field] = value;
        let result = serde_json::from_value::<LeafExecutionSpec>(constrained);
        assert!(
            result.is_err(),
            "unsupported constraint {field} was silently discarded"
        );
    }
}

#[test]
fn unsupported_objective_and_leaf_constraints_are_rejected_at_ingress() {
    let mut base = serde_json::to_value(objective("budget", "budget-ingress")).unwrap();
    base["leaves"][0]["execution"] = execution();
    assert!(serde_json::from_value::<NewObjective>(base.clone()).is_ok());
    for location in ["objective", "project", "leaf", "execution"] {
        let mut input = base.clone();
        let node = match location {
            "objective" => &mut input,
            "project" => &mut input["projects"][0],
            "leaf" => &mut input["leaves"][0],
            _ => &mut input["leaves"][0]["execution"],
        };
        node["budget"] = json!({"max_attempts": 2, "max_build_jobs": 2, "local_only": true});
        assert!(
            serde_json::from_value::<NewObjective>(input).is_err(),
            "{location} budget disappeared before canonical admission"
        );
    }
}
