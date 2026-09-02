use super::{ClaimedLeaf, ObjectiveStore, StageReceipt};
use anyhow::{Context, Result};
use std::future::Future;
use std::pin::Pin;

pub trait LeafExecution: Send + Sync {
    fn execute(
        &self,
        claim: ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafExecutionResult {
    pub receipts: Vec<StageReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafRoundOutcome {
    pub objective_id: String,
    pub leaf_id: String,
    pub terminal_receipt_digest: String,
}

pub struct ObjectiveRuntime<E> {
    store: ObjectiveStore,
    executor: E,
    worker_id: String,
    capacity: usize,
    lease_duration_ms: i64,
}

impl<E> ObjectiveRuntime<E>
where
    E: LeafExecution,
{
    pub fn new(
        store: ObjectiveStore,
        executor: E,
        worker_id: impl Into<String>,
        capacity: usize,
        lease_duration_ms: i64,
    ) -> Self {
        Self {
            store,
            executor,
            worker_id: worker_id.into(),
            capacity,
            lease_duration_ms,
        }
    }

    pub fn store(&self) -> &ObjectiveStore {
        &self.store
    }

    pub async fn run_round(&self, now_ms: i64) -> Result<Vec<LeafRoundOutcome>> {
        let claims = self.store.claim_runnable(
            &self.worker_id,
            now_ms,
            self.lease_duration_ms,
            self.capacity,
        )?;
        let executions = claims
            .iter()
            .cloned()
            .map(|claim| self.executor.execute(claim))
            .collect::<Vec<_>>();
        let results = futures::future::join_all(executions).await;
        let mut outcomes = Vec::with_capacity(claims.len());
        let mut errors = Vec::new();
        for (claim, result) in claims.into_iter().zip(results) {
            let result = match result.with_context(|| {
                format!(
                    "execute objective `{}` leaf `{}`",
                    claim.objective_id, claim.leaf_id
                )
            }) {
                Ok(result) => result,
                Err(error) => {
                    errors.push(format!("{error:#}"));
                    continue;
                }
            };
            if result.receipts.is_empty() {
                errors.push(format!(
                    "leaf `{}` returned no canonical receipts",
                    claim.leaf_id
                ));
                continue;
            }
            let mut receipt_error = None;
            for receipt in result.receipts {
                if let Err(error) = self.store.record_stage_receipt(
                    &claim.leaf_id,
                    &claim.lease_owner,
                    receipt,
                    now_ms,
                ) {
                    receipt_error = Some(error);
                    break;
                }
            }
            if let Some(error) = receipt_error {
                errors.push(format!(
                    "record objective `{}` leaf `{}` receipt: {error:#}",
                    claim.objective_id, claim.leaf_id
                ));
                continue;
            }
            let completed = (|| -> Result<LeafRoundOutcome> {
                let leaf = self.store.leaf(&claim.leaf_id)?.ok_or_else(|| {
                    anyhow::anyhow!("claimed leaf `{}` disappeared", claim.leaf_id)
                })?;
                let terminal_receipt_digest = leaf.current_receipt_digest.ok_or_else(|| {
                    anyhow::anyhow!(
                        "completed leaf `{}` omitted terminal receipt",
                        claim.leaf_id
                    )
                })?;
                self.store.complete_objective_if_ready(
                    &claim.objective_id,
                    &terminal_receipt_digest,
                    now_ms,
                )?;
                Ok(LeafRoundOutcome {
                    objective_id: claim.objective_id,
                    leaf_id: claim.leaf_id.clone(),
                    terminal_receipt_digest,
                })
            })();
            match completed {
                Ok(outcome) => outcomes.push(outcome),
                Err(error) => errors.push(format!(
                    "finalize objective leaf `{}`: {error:#}",
                    claim.leaf_id
                )),
            }
        }
        if !errors.is_empty() {
            anyhow::bail!("objective round failed: {}", errors.join("; "));
        }
        Ok(outcomes)
    }
}
