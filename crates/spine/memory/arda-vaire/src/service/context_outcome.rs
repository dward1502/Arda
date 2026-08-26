//! Durable disposition receipts for context that reached a consumer.

use super::context_capsule::ContextUseReceipt;
use super::{store, MnemosyneService};
use arda_core::error::{ArdaError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub const CONTEXT_OUTCOME_RECEIPT_SCHEMA_VERSION: &str = "arda.context-outcome-receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextDisposition {
    Used,
    Deferred,
    Rejected,
    Superseded,
    CouldNotRetrieve,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextOutcomeReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub context_use_receipt_id: String,
    pub capsule_id: String,
    pub objective_id: String,
    pub run_id: Option<String>,
    pub consumer_id: String,
    pub disposition: ContextDisposition,
    pub selected_memory_refs: Vec<String>,
    pub influenced_memory_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rationale: String,
    pub recorded_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOutcomeInput {
    pub consumer_id: String,
    pub disposition: ContextDisposition,
    pub influenced_memory_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rationale: String,
    pub recorded_at_unix_ms: u128,
}

impl ContextOutcomeReceipt {
    pub fn has_valid_digest(&self) -> Result<bool> {
        Ok(
            self.schema_version == CONTEXT_OUTCOME_RECEIPT_SCHEMA_VERSION
                && receipt_digest(self)? == self.receipt_digest,
        )
    }
}

impl MnemosyneService {
    /// Close the gap between selecting context and proving how it affected work.
    /// The stable `(context_use_receipt_id, consumer_id, run_id)` identity makes
    /// exact replay a no-op and conflicting replay fail visibly.
    pub fn record_context_outcome(
        &self,
        use_receipt: &ContextUseReceipt,
        input: ContextOutcomeInput,
    ) -> Result<ContextOutcomeReceipt> {
        if !use_receipt.has_valid_digest()? || input.consumer_id != use_receipt.consumer_id {
            return Err(context_error(
                "context outcome is not bound to a valid use receipt",
            ));
        }
        if input.rationale.trim().is_empty() {
            return Err(context_error("context outcome rationale is required"));
        }
        let selected = use_receipt
            .memory_refs
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if input
            .influenced_memory_refs
            .iter()
            .any(|value| !selected.contains(value))
        {
            return Err(context_error(
                "influenced memory was not selected in the context capsule",
            ));
        }
        if input.disposition != ContextDisposition::Used && !input.influenced_memory_refs.is_empty()
        {
            return Err(context_error(
                "only used context can claim memory influence",
            ));
        }
        let identity = format!(
            "{}\0{}\0{}",
            use_receipt.receipt_id,
            input.consumer_id,
            use_receipt.run_id.as_deref().unwrap_or_default()
        );
        let mut receipt = ContextOutcomeReceipt {
            schema_version: CONTEXT_OUTCOME_RECEIPT_SCHEMA_VERSION.into(),
            receipt_id: format!("context-outcome:{}", hex_digest(identity.as_bytes())),
            receipt_digest: String::new(),
            context_use_receipt_id: use_receipt.receipt_id.clone(),
            capsule_id: use_receipt.capsule_id.clone(),
            objective_id: use_receipt.objective_id.clone(),
            run_id: use_receipt.run_id.clone(),
            consumer_id: input.consumer_id,
            disposition: input.disposition,
            selected_memory_refs: use_receipt.memory_refs.clone(),
            influenced_memory_refs: input.influenced_memory_refs,
            evidence_refs: input.evidence_refs,
            rationale: input.rationale,
            recorded_at_unix_ms: input.recorded_at_unix_ms,
        };
        receipt.receipt_digest = receipt_digest(&receipt)?;

        if let Some(existing) = self
            .context_outcome_receipts()?
            .into_iter()
            .find(|item| item.receipt_id == receipt.receipt_id)
        {
            if existing == receipt {
                return Ok(existing);
            }
            return Err(context_error(
                "conflicting replay for context outcome receipt",
            ));
        }
        store::append_jsonl(&self.root.join("context_outcome_receipts.jsonl"), &receipt)?;
        Ok(receipt)
    }

    pub fn context_outcome_receipts(&self) -> Result<Vec<ContextOutcomeReceipt>> {
        let path = self.root.join("context_outcome_receipts.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut receipts = Vec::new();
        let mut identities = BTreeSet::new();
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let receipt: ContextOutcomeReceipt = serde_json::from_str(&line)?;
            if !receipt.has_valid_digest()? || !identities.insert(receipt.receipt_id.clone()) {
                return Err(context_error("invalid context-outcome receipt ledger"));
            }
            receipts.push(receipt);
        }
        Ok(receipts)
    }
}

fn receipt_digest(receipt: &ContextOutcomeReceipt) -> Result<String> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_digest.clear();
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&unsigned)?)
    ))
}

fn hex_digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn context_error(message: impl Into<String>) -> ArdaError {
    ArdaError::Agent {
        agent: "vaire".into(),
        message: message.into(),
    }
}
