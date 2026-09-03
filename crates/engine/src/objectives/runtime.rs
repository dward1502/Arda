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
        for (claim, result) in claims.into_iter().zip(results) {
            let result = result.with_context(|| {
                format!(
                    "execute objective `{}` leaf `{}`",
                    claim.objective_id, claim.leaf_id
                )
            })?;
            if result.receipts.is_empty() {
                anyhow::bail!("leaf `{}` returned no canonical receipts", claim.leaf_id);
            }
            for receipt in result.receipts {
                self.store.record_stage_receipt(
                    &claim.leaf_id,
                    &claim.lease_owner,
                    receipt,
                    now_ms,
                )?;
            }
            let leaf = self
                .store
                .leaf(&claim.leaf_id)?
                .ok_or_else(|| anyhow::anyhow!("claimed leaf `{}` disappeared", claim.leaf_id))?;
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
            outcomes.push(LeafRoundOutcome {
                objective_id: claim.objective_id,
                leaf_id: claim.leaf_id,
                terminal_receipt_digest,
            });
        }
        Ok(outcomes)
    }
}
