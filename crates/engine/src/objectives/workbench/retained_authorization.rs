//! Per-mutation retained authority; provider POSTs also carry the checked lease.
use super::*;

pub(super) struct RetainedAuthorization {
    store: super::super::ObjectiveStore,
    claim: ClaimedLeaf,
    expected: serde_json::Value,
    run_id: String,
}
impl RetainedAuthorization {
    pub(super) fn new(
        store: super::super::ObjectiveStore,
        claim: ClaimedLeaf,
        item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Self> {
        Ok(Self {
            store,
            claim,
            expected: serde_json::to_value(item)?,
            run_id: item.run_id.clone(),
        })
    }
    fn checked(&self, item: &ExplicitWorkbenchWorkItem) -> Result<super::super::RetainedExecution> {
        if serde_json::to_value(item)? != self.expected {
            anyhow::bail!("retained workspace authorization is for another work item");
        }
        let binding = self
            .store
            .retained_execution(&self.run_id, Utc::now().timestamp_millis())?
            .ok_or_else(|| anyhow::anyhow!("retained workspace authority is absent"))?;
        if binding.lease.generation != self.claim.attempt
            || binding.lease.owner != self.claim.lease_owner
            || binding.lease.expires_ms != self.claim.lease_expires_ms
        {
            anyhow::bail!("retained workspace claim was fenced");
        }
        Ok(binding)
    }
}
impl ExplicitWorkspaceAuthorization for RetainedAuthorization {
    fn authorize(&self, item: &ExplicitWorkbenchWorkItem) -> Result<()> {
        self.checked(item).map(|_| ())
    }
    fn provider_lease(&self, item: &ExplicitWorkbenchWorkItem) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self.checked(item)?.lease)?)
    }
}
