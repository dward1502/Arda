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
        if !is_valid_package_id(&self.package_id) {
            return Err(ArdaError::Task(
                "aipkg package_id must be dotted and namespaced".into(),
            ));
        }
        if !is_sha256_digest(&self.package_digest) {
            return Err(ArdaError::Task(
                "aipkg package_digest must be a sha256 digest".into(),
            ));
        }
        if self.version.trim().is_empty() {
            return Err(ArdaError::Task("aipkg version must not be empty".into()));
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
        self.preflight_check_with_signature("")
    }

    pub fn preflight_check_with_signature(
        &self,
        signature: impl Into<String>,
    ) -> Result<AipkgPreflightReceipt> {
        self.validate()?;

        Ok(AipkgPreflightReceipt {
            package_id: self.package_id.clone(),
            version: self.version.clone(),
            digest: self.package_digest.clone(),
            runtime_profile: self.runtime_profile.clone(),
            approved: true,
            joule_budget: None,
            expires_at_utc: (Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
            signature: signature.into(),
        })
    }
}

fn is_valid_package_id(package_id: &str) -> bool {
    let parts = package_id.split('.').collect::<Vec<_>>();
    parts.len() >= 2
        && parts[0]
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && parts.into_iter().all(is_valid_package_segment)
}

fn is_valid_package_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_sha256_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_timestamp(value: &str, field: &str) -> Result<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| ArdaError::Task(format!("aipkg {field} must be an RFC3339 timestamp")))
}

fn require_signature(signature: &str, receipt: &str) -> Result<()> {
    if signature.trim().is_empty() {
        return Err(ArdaError::Task(format!(
            "aipkg {receipt} signature is required"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AipkgGovernanceEvidence {
    pub triad_passed: bool,
    pub bacon_lite_passed: bool,
    pub joule_within_budget: bool,
    pub love_acceptable: bool,
}

impl AipkgValidationReceipt {
    pub fn from_evidence(
        manifest: &AipkgManifest,
        evidence: AipkgGovernanceEvidence,
        validator_id: impl Into<String>,
        signature: impl Into<String>,
    ) -> Self {
        let overall_passed = evidence.triad_passed
            && evidence.bacon_lite_passed
            && evidence.joule_within_budget
            && evidence.love_acceptable;
        Self {
            package_id: manifest.package_id.clone(),
            version: manifest.version.clone(),
            validated_at_utc: Utc::now().to_rfc3339(),
            triad_passed: evidence.triad_passed,
            bacon_lite_passed: evidence.bacon_lite_passed,
            joule_within_budget: evidence.joule_within_budget,
            love_acceptable: evidence.love_acceptable,
            overall_passed,
            validator_id: validator_id.into(),
            signature: signature.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AipkgReceiptChain {
    pub preflight: AipkgPreflightReceipt,
    pub execution: AipkgExecutionReceipt,
    pub validation: AipkgValidationReceipt,
}

impl AipkgReceiptChain {
    pub fn validate(&self, manifest: &AipkgManifest) -> Result<()> {
        manifest.validate()?;

        for (receipt, package_id, version) in [
            (
                "preflight receipt",
                self.preflight.package_id.as_str(),
                self.preflight.version.as_str(),
            ),
            (
                "execution receipt",
                self.execution.package_id.as_str(),
                self.execution.version.as_str(),
            ),
            (
                "validation receipt",
                self.validation.package_id.as_str(),
                self.validation.version.as_str(),
            ),
        ] {
            if package_id != manifest.package_id || version != manifest.version {
                return Err(ArdaError::Task(format!(
                    "aipkg {receipt} identity does not match manifest"
                )));
            }
        }

        if self.preflight.digest != manifest.package_digest
            || self.preflight.runtime_profile != manifest.runtime_profile
        {
            return Err(ArdaError::Task(
                "aipkg preflight receipt does not match manifest digest/profile".into(),
            ));
        }
        if !self.preflight.approved {
            return Err(ArdaError::Task(
                "aipkg preflight receipt must be approved".into(),
            ));
        }
        let expires_at = parse_timestamp(&self.preflight.expires_at_utc, "expires_at_utc")?;
        let started_at = parse_timestamp(&self.execution.started_at_utc, "started_at_utc")?;
        let completed_at = parse_timestamp(&self.execution.completed_at_utc, "completed_at_utc")?;
        if started_at > expires_at {
            return Err(ArdaError::Task(
                "aipkg execution started after preflight expiry".into(),
            ));
        }
        if completed_at < started_at {
            return Err(ArdaError::Task(
                "aipkg execution receipt completed before it started".into(),
            ));
        }
        if self.execution.exit_code != 0 {
            return Err(ArdaError::Task(
                "aipkg execution receipt reports a failed execution".into(),
            ));
        }
        if !is_sha256_digest(&self.execution.output_digest) {
            return Err(ArdaError::Task(
                "aipkg execution output_digest must be a sha256 digest".into(),
            ));
        }
        if self
            .preflight
            .joule_budget
            .is_some_and(|budget| self.execution.joule_cost_actual > budget)
        {
            return Err(ArdaError::Task(
                "aipkg execution exceeded the preflight joule budget".into(),
            ));
        }

        let validated_at = parse_timestamp(&self.validation.validated_at_utc, "validated_at_utc")?;
        if validated_at < completed_at {
            return Err(ArdaError::Task(
                "aipkg validation receipt predates execution completion".into(),
            ));
        }
        if self.validation.validator_id.trim().is_empty() {
            return Err(ArdaError::Task(
                "aipkg validation receipt requires validator_id".into(),
            ));
        }
        if !self.validation.triad_passed
            || !self.validation.bacon_lite_passed
            || !self.validation.joule_within_budget
            || !self.validation.love_acceptable
            || !self.validation.overall_passed
        {
            return Err(ArdaError::Task(
                "aipkg validation receipt must record every required gate as passed".into(),
            ));
        }

        if manifest.receipts.signatures_required {
            require_signature(&self.preflight.signature, "preflight receipt")?;
            require_signature(&self.execution.signature, "execution receipt")?;
            require_signature(&self.validation.signature, "validation receipt")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json;

    use crate::aipkg::{
        AipkgExecutionReceipt, AipkgGovernanceEvidence, AipkgManifest, AipkgPreflightReceipt,
        AipkgReceiptChain, AipkgValidationReceipt,
    };

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

    #[test]
    fn aipkg_manifest_rejects_schema_incompatible_identity_and_digest() {
        let mut manifest = valid_manifest();
        manifest.package_id = "Org.Arda.demo".into();
        assert!(manifest.validate().is_err());

        manifest.package_id = "org.arda.demo".into();
        manifest.package_digest = "sha256:abc".into();
        assert!(manifest.validate().is_err());

        manifest.package_digest =
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
        manifest.package_id = "org-name.arda.demo".into();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn aipkg_preflight_check_succeeds_for_valid_manifest() {
        let manifest = valid_manifest();
        let receipt = manifest.preflight_check().unwrap();
        assert!(receipt.approved);
        assert_eq!(receipt.package_id, manifest.package_id);
        assert_eq!(receipt.digest, manifest.package_digest);
        assert_eq!(receipt.runtime_profile, manifest.runtime_profile);
    }

    #[test]
    fn aipkg_preflight_round_trips_through_json() {
        let manifest = valid_manifest();
        let receipt = manifest.preflight_check().unwrap();
        let json = serde_json::to_string(&receipt).expect("aipkg preflight receipt serializes");
        let round_trip: AipkgPreflightReceipt =
            serde_json::from_str(&json).expect("aipkg preflight receipt deserializes");
        assert_eq!(receipt.package_id, round_trip.package_id);
        assert_eq!(receipt.digest, round_trip.digest);
    }

    #[test]
    fn aipkg_execution_receipt_can_be_serialized_as_json() {
        let receipt = AipkgExecutionReceipt {
            package_id: "org.arda.demo".into(),
            version: "0.1.0".into(),
            started_at_utc: "2026-07-21T00:00:00Z".into(),
            completed_at_utc: "2026-07-21T00:00:05Z".into(),
            joule_cost_actual: 120,
            exit_code: 0,
            output_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            signature: "sig:dummy".into(),
        };
        assert!(serde_json::to_string(&receipt)
            .expect("execution receipt serializable")
            .contains("org.arda.demo"));
    }

    #[test]
    fn aipkg_receipt_policy_match_requires_all_required_booleans() {
        let manifest = valid_manifest();
        assert!(manifest.receipts.preflight_required);
        assert!(manifest.receipts.execution_required);
        assert!(manifest.receipts.validation_required);
        assert!(manifest.receipts.signatures_required);
    }

    fn valid_receipt_chain(manifest: &AipkgManifest) -> AipkgReceiptChain {
        let preflight = manifest
            .preflight_check_with_signature("sig:preflight")
            .expect("signed preflight");
        let execution = AipkgExecutionReceipt {
            package_id: manifest.package_id.clone(),
            version: manifest.version.clone(),
            started_at_utc: "2026-07-27T00:00:00Z".into(),
            completed_at_utc: "2026-07-27T00:00:05Z".into(),
            joule_cost_actual: 120,
            exit_code: 0,
            output_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            signature: "sig:execution".into(),
        };
        let validation = AipkgValidationReceipt::from_evidence(
            manifest,
            AipkgGovernanceEvidence {
                triad_passed: true,
                bacon_lite_passed: true,
                joule_within_budget: true,
                love_acceptable: true,
            },
            "arda-test-validator",
            "sig:validation",
        );
        AipkgReceiptChain {
            preflight,
            execution,
            validation,
        }
    }

    #[test]
    fn aipkg_receipt_chain_accepts_matching_signed_pass_evidence() {
        let manifest = valid_manifest();
        assert!(valid_receipt_chain(&manifest).validate(&manifest).is_ok());
    }

    #[test]
    fn aipkg_receipt_chain_rejects_unsigned_or_failed_evidence() {
        let manifest = valid_manifest();
        let mut chain = valid_receipt_chain(&manifest);
        chain.execution.signature.clear();
        assert!(chain.validate(&manifest).is_err());

        let mut chain = valid_receipt_chain(&manifest);
        chain.validation.love_acceptable = false;
        chain.validation.overall_passed = false;
        assert!(chain.validate(&manifest).is_err());
    }

    #[test]
    fn aipkg_receipt_chain_rejects_cross_package_receipts() {
        let manifest = valid_manifest();
        let mut chain = valid_receipt_chain(&manifest);
        chain.execution.package_id = "org.arda.other".into();
        assert!(chain.validate(&manifest).is_err());
    }

    #[test]
    fn aipkg_receipt_chain_enforces_chronology_and_joule_budget() {
        let manifest = valid_manifest();
        let mut chain = valid_receipt_chain(&manifest);
        chain.preflight.expires_at_utc = "2026-07-26T23:59:59Z".into();
        assert!(chain.validate(&manifest).is_err());

        let mut chain = valid_receipt_chain(&manifest);
        chain.preflight.joule_budget = Some(100);
        assert!(chain.validate(&manifest).is_err());
    }
}
