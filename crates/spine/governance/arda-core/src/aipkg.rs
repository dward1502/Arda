// sigil: ANKH
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{ArdaError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AipkgManifest {
    pub manifest_version: String,
    pub package_id: String,
    pub version: String,
    pub package_digest: String,
    pub runtime_profile: String,
    pub preflight: AipkgPreflight,
    pub governance: AipkgGovernance,
    pub receipts: AipkgReceiptPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AipkgPreflight {
    pub zero_work_required: bool,
    pub compatibility_required: bool,
    pub quote_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AipkgGovernance {
    pub triad_required: bool,
    pub bacon_lite_required: bool,
    pub joulework_budget_required: bool,
    pub love_eq_guard_required: bool,
    pub soterion_trace_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AipkgReceiptPolicy {
    pub preflight_required: bool,
    pub execution_required: bool,
    pub validation_required: bool,
    pub settlement_optional: bool,
    pub signatures_required: bool,
}

impl AipkgManifest {
    pub fn validate(&self) -> Result<()> {
        if self.manifest_version != "0.1" {
            return Err(ArdaError::Task("aipkg manifest_version must be 0.1".into()));
        }
        if !self.package_id.contains('.') {
            return Err(ArdaError::Task(
                "aipkg package_id must be dotted and namespaced".into(),
            ));
        }
        if !self.package_digest.starts_with("sha256:") {
            return Err(ArdaError::Task(
                "aipkg package_digest must be a sha256 digest".into(),
            ));
        }
        if !matches!(
            self.runtime_profile.as_str(),
            "wasm-wasi" | "oci-sandboxed" | "local-sovereign"
        ) {
            return Err(ArdaError::Task(
                "aipkg runtime_profile must be wasm-wasi, oci-sandboxed, or local-sovereign".into(),
            ));
        }
        if !self.preflight.zero_work_required
            || !self.preflight.compatibility_required
            || !self.preflight.quote_required
        {
            return Err(ArdaError::Task(
                "aipkg preflight must enforce zero-work, compatibility, and quote phases".into(),
            ));
        }
        if !self.governance.triad_required
            || !self.governance.bacon_lite_required
            || !self.governance.joulework_budget_required
            || !self.governance.love_eq_guard_required
            || !self.governance.soterion_trace_required
        {
            return Err(ArdaError::Task(
                "aipkg governance must require triad, bacon-lite, joulework, love-eq, and soterion trace".into(),
            ));
        }
        if !self.receipts.preflight_required
            || !self.receipts.execution_required
            || !self.receipts.validation_required
            || !self.receipts.signatures_required
        {
            return Err(ArdaError::Task(
                "aipkg receipts must require preflight, execution, validation, and signatures"
                    .into(),
            ));
        }
        Ok(())
    }

    pub fn preflight_check(&self) -> Result<AipkgPreflightReceipt> {
        self.validate()?;

        Ok(AipkgPreflightReceipt {
            package_id: self.package_id.clone(),
            version: self.version.clone(),
            digest: self.package_digest.clone(),
            runtime_profile: self.runtime_profile.clone(),
            approved: true,
            joule_budget: None,
            expires_at_utc: Utc::now().to_rfc3339(),
            signature: "".into(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AipkgPreflightReceipt {
    pub package_id: String,
    pub version: String,
    pub digest: String,
    pub runtime_profile: String,
    pub approved: bool,
    pub joule_budget: Option<u64>,
    pub expires_at_utc: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AipkgExecutionReceipt {
    pub package_id: String,
    pub version: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub joule_cost_actual: u64,
    pub exit_code: i32,
    pub output_digest: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AipkgValidationReceipt {
    pub package_id: String,
    pub version: String,
    pub validated_at_utc: String,
    pub triad_passed: bool,
    pub bacon_lite_passed: bool,
    pub joule_within_budget: bool,
    pub love_acceptable: bool,
    pub overall_passed: bool,
    pub validator_id: String,
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> AipkgManifest {
        AipkgManifest {
            manifest_version: "0.1".into(),
            package_id: "org.arda.demo".into(),
            version: "0.1.0".into(),
            package_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            runtime_profile: "local-sovereign".into(),
            preflight: AipkgPreflight {
                zero_work_required: true,
                compatibility_required: true,
                quote_required: true,
            },
            governance: AipkgGovernance {
                triad_required: true,
                bacon_lite_required: true,
                joulework_budget_required: true,
                love_eq_guard_required: true,
                soterion_trace_required: true,
            },
            receipts: AipkgReceiptPolicy {
                preflight_required: true,
                execution_required: true,
                validation_required: true,
                settlement_optional: true,
                signatures_required: true,
            },
        }
    }

    #[test]
    fn aipkg_manifest_requires_governance_and_preflight_law() {
        let manifest = valid_manifest();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn aipkg_manifest_rejects_missing_gates() {
        let mut manifest = valid_manifest();
        manifest.governance.joulework_budget_required = false;
        let err = manifest.validate().unwrap_err().to_string();
        assert!(err.contains("governance"));
    }
}
