//! Governed, bounded organism-context capsule assembly.
//!
//! Vairë resolves only memory records explicitly named by the canonical
//! `OrganismContext`. The capsule is a transport projection; canonical memory,
//! objective, run, evidence, and receipt authority remain in their existing
//! stores.

use super::organism_context::OrganismContext;
use super::scope_policy::{
    self, ConsumerContext, MemoryDomain, PolicyDisposition, PolicyOperation,
};
use super::{store, MnemosyneService};
use arda_core::contract::{MemoryRecord, MemoryState};
use arda_core::error::{ArdaError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub const CONTEXT_CAPSULE_SCHEMA_VERSION: &str = "arda.organism-context-capsule.v1";
pub const CONTEXT_USE_RECEIPT_SCHEMA_VERSION: &str = "arda.context-use-receipt.v1";
const MAX_MEMORIES: usize = 32;
const MAX_MEMORY_BYTES: usize = 4_096;
const MAX_CAPSULE_MEMORY_BYTES: usize = 32_768;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextMemoryProjection {
    pub memory_id: String,
    pub domain: MemoryDomain,
    pub content: String,
    pub content_digest: String,
    pub source_agent: String,
    pub last_seen_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganismContextCapsule {
    pub schema_version: String,
    pub capsule_id: String,
    pub capsule_digest: String,
    pub context: OrganismContext,
    pub memories: Vec<ContextMemoryProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextUseReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub capsule_id: String,
    pub capsule_digest: String,
    pub objective_id: String,
    pub run_id: Option<String>,
    pub consumer_id: String,
    pub purpose: String,
    pub memory_refs: Vec<String>,
    pub recorded_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextAssembly {
    pub capsule: OrganismContextCapsule,
    pub use_receipt: ContextUseReceipt,
}

impl OrganismContextCapsule {
    pub fn computed_digest(&self) -> Result<String> {
        let canonical = serde_json::to_vec(&serde_json::json!({
            "schema_version": self.schema_version,
            "context": self.context,
            "memories": self.memories,
        }))?;
        Ok(digest_bytes(&canonical))
    }

    pub fn validate(&self, now_unix_ms: u128) -> Result<()> {
        if self.schema_version != CONTEXT_CAPSULE_SCHEMA_VERSION
            || self.capsule_id
                != format!(
                    "capsule:{}",
                    self.capsule_digest.trim_start_matches("sha256:")
                )
            || self.computed_digest()? != self.capsule_digest
            || self.memories.len() != self.context.memory_refs.len()
            || self
                .memories
                .iter()
                .map(|memory| memory.memory_id.as_str())
                .ne(self.context.memory_refs.iter().map(String::as_str))
            || self
                .memories
                .iter()
                .any(|memory| digest_bytes(memory.content.as_bytes()) != memory.content_digest)
        {
            return Err(context_error("invalid organism context capsule"));
        }
        self.context
            .validate()
            .map_err(|error| context_error(error.to_string()))?;
        if now_unix_ms < self.context.generated_at_unix_ms
            || now_unix_ms >= self.context.expires_at_unix_ms
        {
            return Err(context_error("organism context capsule is not active"));
        }
        Ok(())
    }
}

impl ContextUseReceipt {
    pub fn computed_digest(&self) -> Result<String> {
        digest_receipt(self)
    }

    pub fn has_valid_digest(&self) -> Result<bool> {
        Ok(self.schema_version == CONTEXT_USE_RECEIPT_SCHEMA_VERSION
            && self.computed_digest()? == self.receipt_digest)
    }

    pub fn receipt_ref(&self) -> String {
        format!("arda://vaire/context-use/{}", self.receipt_id)
    }
}

impl MnemosyneService {
    /// Assemble one deterministic, purpose-bound capsule from canonical
    /// references and persist an idempotent use receipt.
    pub fn assemble_organism_context(
        &self,
        context: OrganismContext,
        consumer: &ConsumerContext,
        now_unix_ms: u128,
    ) -> Result<ContextAssembly> {
        context
            .validate()
            .map_err(|error| context_error(error.to_string()))?;
        validate_consumer_binding(&context, consumer, now_unix_ms)?;
        if context.memory_refs.len() > MAX_MEMORIES {
            return Err(context_error("context requests too many memory records"));
        }

        let records = self
            .read_contract_records()?
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let mut memories = Vec::with_capacity(context.memory_refs.len());
        let mut total_bytes = 0usize;
        for memory_ref in &context.memory_refs {
            let record = records.get(memory_ref).ok_or_else(|| {
                context_error(format!("requested memory {memory_ref} was not found"))
            })?;
            let governed = governed_projection(record, consumer)?;
            let domain = scope_policy::domain(&governed);
            if !context.consumer.memory_domains.contains(&domain) {
                return Err(context_error(format!(
                    "requested memory {memory_ref} exceeds the context memory domains"
                )));
            }
            if governed.content.len() > MAX_MEMORY_BYTES {
                return Err(context_error(format!(
                    "requested memory {memory_ref} exceeds the per-record capsule bound"
                )));
            }
            total_bytes = total_bytes.saturating_add(governed.content.len());
            if total_bytes > MAX_CAPSULE_MEMORY_BYTES {
                return Err(context_error("resolved memory exceeds the capsule bound"));
            }
            memories.push(ContextMemoryProjection {
                memory_id: governed.id,
                domain,
                content_digest: digest_bytes(governed.content.as_bytes()),
                content: governed.content,
                source_agent: governed.agent,
                last_seen_at_unix_ms: governed.last_seen_at.timestamp_millis(),
            });
        }

        let canonical = serde_json::to_vec(&serde_json::json!({
            "schema_version": CONTEXT_CAPSULE_SCHEMA_VERSION,
            "context": &context,
            "memories": &memories,
        }))?;
        let capsule_digest = digest_bytes(&canonical);
        let capsule = OrganismContextCapsule {
            schema_version: CONTEXT_CAPSULE_SCHEMA_VERSION.into(),
            capsule_id: format!("capsule:{}", capsule_digest.trim_start_matches("sha256:")),
            capsule_digest: capsule_digest.clone(),
            context,
            memories,
        };

        if let Some(existing) = self.context_use_receipts()?.into_iter().find(|receipt| {
            receipt.capsule_digest == capsule_digest
                && receipt.consumer_id == consumer.consumer_id
                && receipt.purpose == consumer.purpose.as_deref().unwrap_or_default()
        }) {
            return Ok(ContextAssembly {
                capsule,
                use_receipt: existing,
            });
        }

        let purpose = consumer.purpose.clone().unwrap_or_default();
        let receipt_id_source =
            format!("{}\0{}\0{}", capsule_digest, consumer.consumer_id, purpose);
        let mut receipt = ContextUseReceipt {
            schema_version: CONTEXT_USE_RECEIPT_SCHEMA_VERSION.into(),
            receipt_id: format!("context-use:{}", hex_digest(receipt_id_source.as_bytes())),
            receipt_digest: String::new(),
            capsule_id: capsule.capsule_id.clone(),
            capsule_digest,
            objective_id: capsule.context.lineage.objective_id.as_str().into(),
            run_id: capsule
                .context
                .lineage
                .run_id
                .as_ref()
                .map(|run_id| run_id.as_str().into()),
            consumer_id: consumer.consumer_id.clone(),
            purpose,
            memory_refs: capsule.context.memory_refs.clone(),
            recorded_at_unix_ms: now_unix_ms,
            expires_at_unix_ms: capsule.context.expires_at_unix_ms,
        };
        receipt.receipt_digest = digest_receipt(&receipt)?;
        store::append_jsonl(&self.root.join("context_use_receipts.jsonl"), &receipt)?;
        Ok(ContextAssembly {
            capsule,
            use_receipt: receipt,
        })
    }

    pub fn context_use_receipt(&self, receipt_id: &str) -> Result<Option<ContextUseReceipt>> {
        Ok(self
            .context_use_receipts()?
            .into_iter()
            .find(|receipt| receipt.receipt_id == receipt_id))
    }

    fn context_use_receipts(&self) -> Result<Vec<ContextUseReceipt>> {
        let path = self.root.join("context_use_receipts.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut receipts = Vec::new();
        let mut identities = BTreeSet::new();
        for line in BufReader::new(File::open(&path)?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let receipt: ContextUseReceipt = serde_json::from_str(&line)?;
            if receipt.schema_version != CONTEXT_USE_RECEIPT_SCHEMA_VERSION
                || digest_receipt(&receipt)? != receipt.receipt_digest
                || !identities.insert(receipt.receipt_id.clone())
            {
                return Err(context_error("invalid context-use receipt ledger"));
            }
            receipts.push(receipt);
        }
        Ok(receipts)
    }
}

fn validate_consumer_binding(
    context: &OrganismContext,
    consumer: &ConsumerContext,
    now_unix_ms: u128,
) -> Result<()> {
    if now_unix_ms < context.generated_at_unix_ms || now_unix_ms >= context.expires_at_unix_ms {
        return Err(context_error("organism context is not active"));
    }
    if consumer.consumer_id != context.consumer.consumer_id
        || consumer.operator_authorized != context.consumer.operator_authorized
        || consumer.declared_domains != context.consumer.memory_domains
        || consumer.purpose.as_deref() != Some(context.objective.requested_outcome.as_str())
    {
        return Err(context_error(
            "consumer identity, purpose, authority, or memory domains do not match the context",
        ));
    }
    Ok(())
}

fn governed_projection(record: &MemoryRecord, consumer: &ConsumerContext) -> Result<MemoryRecord> {
    if !matches!(record.state, MemoryState::Active | MemoryState::Promoted) {
        return Err(context_error(format!(
            "requested memory {} is not current",
            record.id
        )));
    }
    match scope_policy::evaluate(record, PolicyOperation::Read, Some(consumer)) {
        PolicyDisposition::Allow => Ok(record.clone()),
        PolicyDisposition::Redact(fields) => Ok(scope_policy::redact(record, &fields)),
        PolicyDisposition::Block | PolicyDisposition::Quarantine => Err(context_error(format!(
            "requested memory {} was denied by scope policy",
            record.id
        ))),
    }
}

fn digest_receipt(receipt: &ContextUseReceipt) -> Result<String> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_digest.clear();
    Ok(digest_bytes(&serde_json::to_vec(&unsigned)?))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{}", hex_digest(bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn context_error(message: impl Into<String>) -> ArdaError {
    ArdaError::Agent {
        agent: "vaire-context".into(),
        message: message.into(),
    }
}
