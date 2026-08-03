//! Policy-gated command providers with bounded evidence receipts.

pub mod cargo_commands;
pub mod module_structure;
mod runner;
pub mod security;
pub mod source;

use serde::{Deserialize, Serialize};

use crate::constants::{PROVIDER_COMPLETED, PROVIDER_MALFORMED_OUTPUT};
use crate::contracts::{CapabilityOutcome, CommandReceipt};

pub use runner::ProviderRunner;

/// A registered read-only provider command. The program and arguments are
/// selected by code/profile; audit requests can only allow or deny its ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandProviderSpec {
    pub provider_id: String,
    pub capability: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub working_directory_relative: String,
    pub timeout_seconds: u64,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
    #[serde(default)]
    pub version_args: Vec<String>,
}

impl CommandProviderSpec {
    pub fn cargo_check() -> Self {
        cargo_commands::cargo_check()
    }

    pub fn cargo_audit() -> Self {
        security::cargo_audit()
    }

    pub fn cargo_deny() -> Self {
        security::cargo_deny()
    }

    pub fn cargo_modules_structure() -> Self {
        module_structure::cargo_modules_structure()
    }

    pub fn new<I, S>(provider_id: &str, capability: &str, program: &str, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            provider_id: provider_id.to_string(),
            capability: capability.to_string(),
            program: program.to_string(),
            args: args.into_iter().map(Into::into).collect(),
            working_directory_relative: ".".to_string(),
            timeout_seconds: 30,
            max_stdout_bytes: 512 * 1024,
            max_stderr_bytes: 128 * 1024,
            version_args: vec!["--version".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderExecution {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub receipt: CommandReceipt,
}

impl ProviderExecution {
    /// Parse retained stdout as JSON and disclose malformed/truncated output as
    /// a capability state instead of fabricating a successful result.
    pub fn json_outcome(
        &self,
        spec: &CommandProviderSpec,
    ) -> (Option<serde_json::Value>, CapabilityOutcome) {
        if self.receipt.status != crate::contracts::CommandReceiptStatus::Completed {
            return (
                None,
                CapabilityOutcome {
                    capability: spec.capability.clone(),
                    provider_id: Some(spec.provider_id.clone()),
                    status: format!("{:?}", self.receipt.status).to_lowercase(),
                    detail: Some("provider command did not complete successfully".to_string()),
                },
            );
        }
        if self.receipt.truncated {
            return (
                None,
                CapabilityOutcome {
                    capability: spec.capability.clone(),
                    provider_id: Some(spec.provider_id.clone()),
                    status: PROVIDER_MALFORMED_OUTPUT.to_string(),
                    detail: Some("provider output was truncated before parsing".to_string()),
                },
            );
        }
        match serde_json::from_slice(&self.stdout) {
            Ok(value) => (
                Some(value),
                CapabilityOutcome {
                    capability: spec.capability.clone(),
                    provider_id: Some(spec.provider_id.clone()),
                    status: PROVIDER_COMPLETED.to_string(),
                    detail: None,
                },
            ),
            Err(_) => (
                None,
                CapabilityOutcome {
                    capability: spec.capability.clone(),
                    provider_id: Some(spec.provider_id.clone()),
                    status: PROVIDER_MALFORMED_OUTPUT.to_string(),
                    detail: Some("provider stdout was not valid JSON".to_string()),
                },
            ),
        }
    }
}
