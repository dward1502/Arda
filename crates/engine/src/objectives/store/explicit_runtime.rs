//! Mandatory Aule owner admission. Lifetime exclusion is not execution authority.
use super::*;
use arda_aule::prometheus::autopilot::workbench_executor::{
    ExplicitRuntimeAdmission, ExplicitRuntimeLease, ExplicitWorkbenchWorkItem,
};

impl ExplicitRuntimeLease for ObjectiveStore {}
impl ExplicitRuntimeAdmission for ObjectiveStore {
    fn admit(
        &self,
        root: &Path,
        item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Box<dyn ExplicitRuntimeLease>> {
        anyhow::ensure!(
            authority::normalize(&root.join("data/arda/objectives.sqlite3"))? == self.path,
            "explicit runtime root does not match ObjectiveStore"
        );
        // Reuse the owned store lease; do not reopen/migrate authority or acquire
        // a SQLite writer across await. Recovery Close can already hold its
        // Engine writer and RunStore lock; this independent WAL reader is safe
        // because maintenance cannot publish a disposition while this store lives.
        let db = self.connection()?;
        crate::objectives::abandonment::guards::reject_in(
            &db,
            Some(&item.objective_id),
            Some(&item.leaf_id),
            Some(&item.run_id),
        )?;
        let bound: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaves WHERE id=?1 AND objective_id=?2 AND execution_run_id=?3)",
            params![item.leaf_id, item.objective_id, item.run_id], |r| r.get(0),
        )?;
        anyhow::ensure!(bound, "explicit runtime canonical run binding mismatch");
        Ok(Box::new(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> ExplicitWorkbenchWorkItem {
        serde_json::from_value(serde_json::json!({
            "objective_id":"objective", "leaf_id":"leaf-0", "run_id":"run-0-attempt-1",
            "objective":"fixture", "execution_prompt":"", "verification_prompt":"", "review_prompt":"",
            "project_id":"", "project_contract_digest":"", "workspace_root":"/fixture",
            "approval_envelope":{}, "objective_plan_receipt":""
        })).unwrap()
    }

    #[tokio::test]
    async fn explicit_runtime_lease_covers_http_await_and_releases_on_return_or_drop() {
        use arda_aule::prometheus::autopilot::workbench_executor::WorkbenchExecutionAdapter;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        struct Transfer(std::sync::Mutex<Option<ObjectiveStore>>);
        impl ExplicitRuntimeAdmission for Transfer {
            fn admit(
                &self,
                root: &Path,
                item: &ExplicitWorkbenchWorkItem,
            ) -> Result<Box<dyn ExplicitRuntimeLease>> {
                self.0.lock().unwrap().take().unwrap().admit(root, item)
            }
        }
        for mode in 0..4 {
            for cancel in [false, true] {
                let (root, store, _) = crate::objectives::abandonment::tests::fixture();
                let path = root.path().join("data/arda/objectives.sqlite3");
                let owner = Transfer(std::sync::Mutex::new(Some(store)));
                let mut work = item();
                work.workspace_root = root.path().to_owned();
                work.project_id = "project".into();
                work.execution_prompt = "execute".into();
                work.verification_prompt = "verify".into();
                work.review_prompt = "review".into();
                work.project_contract_digest = format!("sha256:{}", "a".repeat(64));
                work.objective_plan_receipt = format!("sha256:{}", "b".repeat(64));
                work.approval_envelope = serde_json::json!({"approval":{
                    "schema_version":"arda.orome.task_approval.v1", "approval_id":"approved",
                    "ledger_writes":["data/arda/objectives.sqlite3","data/runs"]
                }});
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let url = format!("http://{}", listener.local_addr().unwrap());
                let adapter =
                    WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
                let (seen_tx, seen_rx) = tokio::sync::oneshot::channel();
                let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
                let server = tokio::spawn(async move {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut request = [0; 4096];
                    let count = stream.read(&mut request).await.unwrap();
                    assert!(std::str::from_utf8(&request[..count])
                        .unwrap()
                        .starts_with("GET /v1/runs/run-0-attempt-1 "));
                    seen_tx.send(()).unwrap();
                    if finish_rx.await.is_ok() {
                        stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
                    }
                });
                let authority = |_: &ExplicitWorkbenchWorkItem| Ok(());
                let mut operation = Box::pin(async {
                    match mode {
                        0 => adapter.execute(&work, &owner).await.map(|_| ()),
                        1 => adapter
                            .execute_authorized(&work, &authority, &owner)
                            .await
                            .map(|_| ()),
                        2 => adapter.reconcile(&work, &owner).await.map(|_| ()),
                        _ => adapter.reconcile_for_retry(&work, &owner).await.map(|_| ()),
                    }
                });
                tokio::select! {
                    result = &mut operation => panic!("returned before HTTP await: {result:?}"),
                    result = tokio::time::timeout(std::time::Duration::from_secs(5), seen_rx) => result.unwrap().unwrap(),
                }
                assert!(owner.0.lock().unwrap().is_none());
                assert!(ObjectiveStore::open_existing_maintenance(&path).is_err());
                // The runtime lifetime lease must not hold a SQLite writer across HTTP.
                let db = Connection::open(&path).unwrap();
                db.busy_timeout(std::time::Duration::ZERO).unwrap();
                db.execute_batch("BEGIN IMMEDIATE; ROLLBACK;").unwrap();
                drop(db);
                if cancel {
                    drop(operation);
                    drop(finish_tx);
                } else {
                    finish_tx.send(()).unwrap();
                    let error = operation.await.unwrap_err();
                    assert!(error.to_string().contains("500"), "{error}");
                }
                server.await.unwrap();
                assert!(ObjectiveStore::open_existing_maintenance(&path).is_ok());
            }
        }
    }

    #[test]
    fn explicit_runtime_close_publishes_under_existing_engine_writer() {
        let (root, store, _) = crate::objectives::abandonment::tests::fixture();
        let mut db = store.connection().unwrap();
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let work = item();
        // Reproduce the nested Engine writer; no writer may be reacquired by admission.
        // This is not a complete recovery-RunStore integration fixture.
        let parent = format!("sha256:{}", "a".repeat(64));
        work.persist_close_receipt(root.path(), &parent, &store)
            .unwrap();
        assert!(ObjectiveStore::open_existing_maintenance(
            root.path().join("data/arda/objectives.sqlite3")
        )
        .is_err());
        let receipt_path = root
            .path()
            .join("data/runs")
            .join(&work.run_id)
            .join("execution-receipts/close.json");
        let before = std::fs::read(&receipt_path).unwrap();
        let receipt: serde_json::Value = serde_json::from_slice(&before).unwrap();
        assert_eq!(
            receipt["receipt_digest"],
            work.close_request_body(&parent).unwrap()["receipt_digest"]
        );
        // A second publication preserves the canonical bytes.
        work.persist_close_receipt(root.path(), &parent, &store)
            .unwrap();
        assert_eq!(before, std::fs::read(&receipt_path).unwrap());
        tx.rollback().unwrap();
        drop(db);
        drop(store);
        assert!(ObjectiveStore::open_existing_maintenance(
            root.path().join("data/arda/objectives.sqlite3")
        )
        .is_ok());
    }

    #[test]
    fn explicit_runtime_owned_lease_survives_store_drop() {
        let (root, store, _) = crate::objectives::abandonment::tests::fixture();
        let lease = store.admit(root.path(), &item()).unwrap();
        drop(store);
        let path = root.path().join("data/arda/objectives.sqlite3");
        assert!(ObjectiveStore::open_existing_maintenance(&path).is_err());
        drop(lease);
        assert!(ObjectiveStore::open_existing_maintenance(&path).is_ok());
    }

    #[test]
    fn explicit_runtime_refuses_wrong_root_binding_and_abandonment() {
        let (root, store, _) = crate::objectives::abandonment::tests::fixture();
        let (other, _, _) = crate::objectives::abandonment::tests::fixture();
        assert!(store.admit(other.path(), &item()).is_err());
        for field in ["objective_id", "leaf_id", "run_id"] {
            let mut value = serde_json::to_value(item()).unwrap();
            value[field] = serde_json::json!("unrelated");
            assert!(store
                .admit(root.path(), &serde_json::from_value(value).unwrap())
                .is_err());
        }
        store.admit(root.path(), &item()).unwrap();
        let db = store.connection().unwrap();
        db.execute_batch("INSERT INTO operator_abandonment_authorizations VALUES('event','{}');
            INSERT INTO retained_snapshot_operator_abandonments VALUES('leaf-0','run-0-attempt-1','objective','event','{}');").unwrap();
        let error = store.admit(root.path(), &item()).err().unwrap();
        assert!(error.to_string().contains("abandonment"), "{error}");
    }
}
