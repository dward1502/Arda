//! Execution-owner lifetime participation, separate from workspace authority.
use super::*;

/// Implementors retain the runtime's shared maintenance-exclusion lease.
/// This lease grants no workspace permission or retained execution authority.
pub trait ExplicitRuntimeLease: Send + Sync {}

/// Admission must validate the exact root and canonical objective/leaf/run binding
/// and refuse permanently abandoned history before returning an owned lease.
pub trait ExplicitRuntimeAdmission: Send + Sync {
    fn admit(
        &self,
        root: &Path,
        item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Box<dyn ExplicitRuntimeLease>>;
}

#[cfg(test)]
pub(super) struct FixtureAdmission;
#[cfg(test)]
impl ExplicitRuntimeLease for () {}
#[cfg(test)]
impl ExplicitRuntimeAdmission for FixtureAdmission {
    fn admit(
        &self,
        _: &Path,
        _: &ExplicitWorkbenchWorkItem,
    ) -> Result<Box<dyn ExplicitRuntimeLease>> {
        Ok(Box::new(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Deny;
    impl ExplicitRuntimeAdmission for Deny {
        fn admit(
            &self,
            _: &Path,
            _: &ExplicitWorkbenchWorkItem,
        ) -> Result<Box<dyn ExplicitRuntimeLease>> {
            bail!("fixture runtime admission denied")
        }
    }

    #[tokio::test]
    async fn runtime_denial_precedes_validation_http_and_close_write() {
        let root = tempfile::tempdir().unwrap();
        // Deliberately invalid item: admission must precede even validation.
        let item: ExplicitWorkbenchWorkItem = serde_json::from_value(json!({
            "objective_id":"objective", "leaf_id":"leaf", "run_id":"run",
            "objective":"", "execution_prompt":"", "verification_prompt":"", "review_prompt":"",
            "project_id":"", "project_contract_digest":"", "workspace_root":root.path(),
            "approval_envelope":{}, "objective_plan_receipt":""
        }))
        .unwrap();
        let adapter =
            WorkbenchExecutionAdapter::with_harness_url(root.path(), "http://127.0.0.1:9").unwrap();
        for result in [
            adapter.execute(&item, &Deny).await.map(|_| ()),
            adapter
                .execute_authorized(
                    &item,
                    &|_: &ExplicitWorkbenchWorkItem| {
                        panic!("workspace authorization before admission")
                    },
                    &Deny,
                )
                .await
                .map(|_| ()),
            adapter.reconcile(&item, &Deny).await.map(|_| ()),
            adapter.reconcile_for_retry(&item, &Deny).await.map(|_| ()),
            item.persist_close_receipt(root.path(), "parent", &Deny),
        ] {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("fixture runtime admission denied"));
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
