//! Stable binding shared by original context capture and recovery validation.
use super::{LeafExecutionSpec, StageReceipt};
use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub(super) struct ResidentRequestBinding<'a> {
    pub objective_id: &'a str,
    pub leaf_id: &'a str,
    pub project_id: &'a str,
    pub project_contract_digest: &'a str,
    pub workspace_root: &'a str,
    pub authority: &'a str,
    pub execution: &'a LeafExecutionSpec,
    pub dependencies: &'a [StageReceipt],
}

impl ResidentRequestBinding<'_> {
    pub fn digest(&self) -> Result<String> {
        // Value preserves the existing json! map encoding rather than hashing
        // a potentially different struct field-order representation.
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&serde_json::to_value(self)?)?)
        ))
    }
}
