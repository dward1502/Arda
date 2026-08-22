//! Typed semantic handoff receipt for delivering governed organism context.
//!
//! This is an Oromë envelope receipt, not a transport session or worker
//! registry. It binds canonical Arda references to the context delivered to one
//! worker attempt and intentionally carries no transcript or vendor session id.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const WORKER_CONTEXT_HANDOFF_SCHEMA_VERSION: &str = "arda.handoff-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerContextHandoffReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub objective_id: String,
    pub run_id: String,
    pub node_id: String,
    pub source_authority: String,
    pub destination_consumer: String,
    pub capsule_id: String,
    pub capsule_digest: String,
    pub context_use_receipt_ref: String,
    #[serde(default)]
    pub parent_receipts: Vec<String>,
    pub recorded_at_unix_ms: u128,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorkerContextHandoffError {
    #[error("worker context handoff field `{0}` cannot be empty")]
    Empty(&'static str),
    #[error("worker context handoff digest is invalid")]
    InvalidDigest,
    #[error("failed to serialize worker context handoff: {0}")]
    Serialize(String),
}

impl WorkerContextHandoffReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        objective_id: impl Into<String>,
        run_id: impl Into<String>,
        node_id: impl Into<String>,
        destination_consumer: impl Into<String>,
        capsule_id: impl Into<String>,
        capsule_digest: impl Into<String>,
        context_use_receipt_ref: impl Into<String>,
        parent_receipts: Vec<String>,
        recorded_at_unix_ms: u128,
    ) -> Result<Self, WorkerContextHandoffError> {
        let mut receipt = Self {
            schema_version: WORKER_CONTEXT_HANDOFF_SCHEMA_VERSION.into(),
            receipt_id: String::new(),
            receipt_digest: String::new(),
            objective_id: objective_id.into(),
            run_id: run_id.into(),
            node_id: node_id.into(),
            source_authority: "vaire:organism-context".into(),
            destination_consumer: destination_consumer.into(),
            capsule_id: capsule_id.into(),
            capsule_digest: capsule_digest.into(),
            context_use_receipt_ref: context_use_receipt_ref.into(),
            parent_receipts,
            recorded_at_unix_ms,
        };
        receipt.validate_fields()?;
        let identity = digest_bytes(
            serde_json::to_string(&serde_json::json!({
                "objective_id": receipt.objective_id,
                "run_id": receipt.run_id,
                "node_id": receipt.node_id,
                "destination_consumer": receipt.destination_consumer,
                "capsule_digest": receipt.capsule_digest,
                "context_use_receipt_ref": receipt.context_use_receipt_ref,
            }))
            .map_err(|error| WorkerContextHandoffError::Serialize(error.to_string()))?
            .as_bytes(),
        );
        receipt.receipt_id = format!("handoff:{}", identity.trim_start_matches("sha256:"));
        receipt.receipt_digest = receipt.computed_digest()?;
        Ok(receipt)
    }

    pub fn computed_digest(&self) -> Result<String, WorkerContextHandoffError> {
        let mut unsigned = self.clone();
        unsigned.receipt_digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&unsigned).map_err(
            |error| WorkerContextHandoffError::Serialize(error.to_string()),
        )?))
    }

    pub fn has_valid_digest(&self) -> Result<bool, WorkerContextHandoffError> {
        self.validate_fields()?;
        Ok(self.computed_digest()? == self.receipt_digest)
    }

    pub fn receipt_ref(&self) -> String {
        format!("arda://orome/handoffs/{}", self.receipt_id)
    }

    fn validate_fields(&self) -> Result<(), WorkerContextHandoffError> {
        for (field, value) in [
            ("objective_id", self.objective_id.as_str()),
            ("run_id", self.run_id.as_str()),
            ("node_id", self.node_id.as_str()),
            ("source_authority", self.source_authority.as_str()),
            ("destination_consumer", self.destination_consumer.as_str()),
            ("capsule_id", self.capsule_id.as_str()),
            (
                "context_use_receipt_ref",
                self.context_use_receipt_ref.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(WorkerContextHandoffError::Empty(field));
            }
        }
        if !is_digest(&self.capsule_digest)
            || (!self.receipt_digest.is_empty() && !is_digest(&self.receipt_digest))
        {
            return Err(WorkerContextHandoffError::InvalidDigest);
        }
        Ok(())
    }
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn is_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
