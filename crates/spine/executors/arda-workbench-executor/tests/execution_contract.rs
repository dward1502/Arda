use anyhow::Result;
use arda_workbench_executor::execution_contract::*;
use futures::future::BoxFuture;
use serde_json::json;
use std::path::Path;

fn wire_item() -> serde_json::Value {
    json!({
        "objective_id":"objective", "leaf_id":"leaf", "run_id":"run",
        "objective":"inspect", "execution_prompt":"inspect", "verification_prompt":"verify",
        "review_prompt":"review", "project_id":"project", "project_contract_digest":"digest",
        "workspace_root":"/fixture", "approval_envelope":{}, "objective_plan_receipt":"plan"
    })
}

#[test]
fn existing_wire_defaults_and_receipt_shape_are_preserved() {
    let item: ExplicitWorkbenchWorkItem = serde_json::from_value(wire_item()).unwrap();
    let mut expected = wire_item();
    expected["read_only"] = json!(false);
    expected["dependency_receipts"] = json!([]);
    assert_eq!(serde_json::to_value(&item).unwrap(), expected);
    let outcome = json!({"run_id":"run", "status":"completed", "root_receipt_digest":"close",
        "receipts":[{"stage":"close", "digest":"close", "path":"receipt.json"}]});
    let decoded: ExplicitExecutionOutcome = serde_json::from_value(outcome.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), outcome);
}

#[test]
fn serialized_work_items_cannot_supply_process_local_authority() {
    for key in [
        "provider_lease",
        "recovery_window",
        "workspace_authorization",
    ] {
        let mut wire = wire_item();
        wire[key] = json!({"authorized":true});
        assert!(serde_json::from_value::<ExplicitWorkbenchWorkItem>(wire).is_err());
    }
}

struct NoDispatch;
impl ExplicitWorkbenchExecution for NoDispatch {
    fn execute_explicit<'a>(
        &'a self,
        _: &'a ExplicitWorkbenchWorkItem,
        _: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>> {
        panic!("unsupported authorization/reconciliation must never dispatch")
    }
}

struct NoAdmission;
impl ExplicitRuntimeAdmission for NoAdmission {
    fn admit(
        &self,
        _: &Path,
        _: &ExplicitWorkbenchWorkItem,
    ) -> Result<Box<dyn ExplicitRuntimeLease>> {
        panic!("contract defaults must perform no admission or side effects")
    }
}

#[test]
fn contract_defaults_refuse_retained_execution_and_do_not_invent_completion() {
    let item: ExplicitWorkbenchWorkItem = serde_json::from_value(wire_item()).unwrap();
    let authority = |_: &ExplicitWorkbenchWorkItem| -> Result<()> {
        panic!("unsupported retained execution must not consult authority")
    };
    futures::executor::block_on(async {
        let error = NoDispatch
            .execute_explicit_authorized(&item, &NoAdmission, &authority)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("does not implement retained workspace authorization"));
        assert!(NoDispatch
            .reconcile_explicit(&item, &NoAdmission)
            .await
            .unwrap()
            .is_none());
        assert!(NoDispatch
            .inspect_retry_explicit(&item, &NoAdmission)
            .await
            .unwrap()
            .is_none());
    });
    assert!(authority.provider_lease(&item).is_err());
    assert!(authority.recovery_window(&item).unwrap().is_none());
}
