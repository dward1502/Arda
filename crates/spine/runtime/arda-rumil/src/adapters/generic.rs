use std::path::Path;

use crate::adapters::{outcome, provider_allowed, ProviderAdapter};
use crate::constants::{PROVIDER_COMPLETED, PROVIDER_SKIPPED_BY_POLICY};
use crate::contracts::{AuditRequest, CapabilityOutcome};
use crate::error::Result;
use crate::inventory_repo;
use crate::policy::AuditPolicy;

pub const CAPABILITY: &str = "generic_inventory";
pub const PROVIDER_ID: &str = "rumil.generic_inventory.v1";

#[derive(Debug, Default, Clone, Copy)]
pub struct GenericAdapter;

impl ProviderAdapter for GenericAdapter {
    fn capability(&self) -> &str {
        CAPABILITY
    }

    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn run(
        &self,
        _request: &AuditRequest,
        policy: &AuditPolicy,
        project_root: &Path,
    ) -> Result<(serde_json::Value, CapabilityOutcome)> {
        if !provider_allowed(policy, PROVIDER_ID) {
            return Ok((
                serde_json::Value::Null,
                outcome(
                    CAPABILITY,
                    PROVIDER_ID,
                    PROVIDER_SKIPPED_BY_POLICY,
                    Some("provider is not allowlisted".to_string()),
                ),
            ));
        }
        let report = inventory_repo(project_root, policy)?;
        let detail = if report.is_complete() {
            None
        } else {
            Some(report.truncation_reasons.join("; "))
        };
        Ok((
            serde_json::to_value(report.file_records())?,
            outcome(CAPABILITY, PROVIDER_ID, PROVIDER_COMPLETED, detail),
        ))
    }
}
