//! Canonical read-only outpost access contracts.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub const OUTPOST_ACCESS_SCHEMA_VERSION: &str = "arda.outpost-access.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutpostAccessContract {
    pub schema_version: String,
    #[serde(default)]
    pub enrollments: Vec<OutpostEnrollment>,
}

impl OutpostAccessContract {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != OUTPOST_ACCESS_SCHEMA_VERSION {
            return Err(format!(
                "unsupported outpost access schema: {}",
                self.schema_version
            ));
        }

        for enrollment in &self.enrollments {
            enrollment.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutpostEnrollment {
    pub outpost_id: String,
    pub bearer_env: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub revoked: bool,
    pub network_posture: NetworkPosture,
}

impl OutpostEnrollment {
    fn validate(&self) -> Result<(), String> {
        if self.outpost_id.trim().is_empty() {
            return Err("outpost_id must not be empty".to_string());
        }
        if self.bearer_env.trim().is_empty() {
            return Err(format!("{} bearer_env must not be empty", self.outpost_id));
        }
        if self
            .capabilities
            .iter()
            .any(|capability| capability.trim().is_empty() || !capability.contains('.'))
        {
            return Err(format!(
                "{} capabilities must use namespace.action form",
                self.outpost_id
            ));
        }
        if self.network_posture.allowed_ips.is_empty() {
            return Err(format!(
                "{} must declare at least one allowed IP",
                self.outpost_id
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkPosture {
    pub allow_forwarded: bool,
    pub allowed_ips: Vec<IpAddr>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
schema_version = "arda.outpost-access.v1"

[[enrollments]]
outpost_id = "node-pi5-citadel-avatar"
bearer_env = "ARDA_CITADEL_PRESENCE_BEARER"
capabilities = ["presence.read"]
revoked = false
network_posture = { allow_forwarded = true, allowed_ips = ["100.119.130.127"] }
"#;

    #[test]
    fn parses_and_validates_canonical_contract() {
        let contract: OutpostAccessContract = toml::from_str(VALID).expect("parse contract");
        contract.validate().expect("validate contract");
        assert_eq!(contract.enrollments.len(), 1);
    }

    #[test]
    fn rejects_unknown_fields() {
        let error = toml::from_str::<OutpostAccessContract>(&format!("{VALID}\nunknown = true"))
            .expect_err("unknown field must fail");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_empty_network_posture() {
        let invalid = VALID.replace("allowed_ips = [\"100.119.130.127\"]", "allowed_ips = []");
        let contract: OutpostAccessContract = toml::from_str(&invalid).expect("parse contract");
        assert!(contract.validate().is_err());
    }
}
