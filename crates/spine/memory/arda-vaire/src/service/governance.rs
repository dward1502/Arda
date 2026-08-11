use super::scope_policy::{
    self, ConsumerContext, MemoryDomain, PolicyDisposition, PolicyOperation,
};
use super::{store, MnemosyneService};
use arda_core::contract::{MemoryKind, MemoryRecord, MemoryState};
use arda_core::error::{ArdaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionChain {
    pub current: MemoryRecord,
    pub history: Vec<MemoryRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CascadePolicy {
    SourceOnly,
    MarkDerivedStale,
    RegenerateDerived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevokeReceipt {
    pub receipt_id: String,
    pub record_id: String,
    pub cascade: CascadePolicy,
    pub affected_derived_ids: Vec<String>,
    pub revoked_at: DateTime<Utc>,
    pub previous_hash: String,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuarantineReceipt {
    pub record_id: String,
    pub reason: String,
    pub quarantined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetentionSweepReport {
    pub scanned: usize,
    pub decayed: Vec<String>,
}

impl MnemosyneService {
    pub fn write_governed_memory(
        &self,
        mut record: MemoryRecord,
        context: Option<&ConsumerContext>,
    ) -> Result<MemoryRecord> {
        match scope_policy::evaluate(&record, PolicyOperation::Write, context) {
            PolicyDisposition::Allow => {}
            PolicyDisposition::Quarantine => {
                record.state = MemoryState::Quarantined;
                record.extensions.insert(
                    "quarantine_reason".into(),
                    serde_json::json!("personal memory provenance requires operator review"),
                );
                self.observe_governance(None, None, 0, 1);
            }
            PolicyDisposition::Redact(_) => {
                return Err(governance_error(
                    "write policy cannot redact a canonical record",
                ));
            }
            PolicyDisposition::Block => {
                return Err(governance_error("memory write blocked by scope policy"));
            }
        }
        self.write_contract_record(&record)?;
        Ok(record)
    }

    pub fn recall_governed_memories(
        &self,
        context: Option<&ConsumerContext>,
    ) -> Result<Vec<MemoryRecord>> {
        let mut recalled = Vec::new();
        for record in self.read_contract_records()? {
            match scope_policy::evaluate(&record, PolicyOperation::Read, context) {
                PolicyDisposition::Allow => recalled.push(record),
                PolicyDisposition::Redact(fields) => {
                    recalled.push(scope_policy::redact(&record, &fields));
                }
                PolicyDisposition::Quarantine => {}
                PolicyDisposition::Block => {
                    if scope_policy::domain(&record) == MemoryDomain::Personal
                        && context.is_none()
                        && matches!(record.state, MemoryState::Active | MemoryState::Promoted)
                    {
                        return Err(governance_error(
                            "personal memory recall requires consumer context",
                        ));
                    }
                }
            }
        }
        recalled.sort_by(|left, right| {
            right
                .last_seen_at
                .cmp(&left.last_seen_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(recalled)
    }

    pub fn clear_quarantine(
        &self,
        record_id: &str,
        context: &ConsumerContext,
    ) -> Result<MemoryRecord> {
        if !context.operator_authorized {
            return Err(governance_error(
                "quarantine clearance requires explicit operator authority",
            ));
        }
        let mut record = self.require_contract_record(record_id)?;
        if record.state != MemoryState::Quarantined {
            return Err(governance_error(format!(
                "memory {record_id} is not quarantined"
            )));
        }
        record.state = MemoryState::Active;
        record.extensions.insert(
            "quarantine_cleared_by".into(),
            serde_json::json!(context.consumer_id),
        );
        record.extensions.insert(
            "quarantine_cleared_at".into(),
            serde_json::json!(Utc::now()),
        );
        self.write_contract_record(&record)?;
        Ok(record)
    }

    pub fn correct_memory(
        &self,
        record_id: &str,
        replacement_content: impl Into<String>,
        context: &ConsumerContext,
    ) -> Result<MemoryRecord> {
        let mut original = self.require_contract_record(record_id)?;
        if matches!(
            original.state,
            MemoryState::Revoked | MemoryState::Quarantined
        ) {
            return Err(governance_error(
                "only current, reviewed memories can be corrected",
            ));
        }
        let replacement_id = format!("mem_{}", uuid::Uuid::new_v4().simple());
        let mut replacement = MemoryRecord::new(
            &replacement_id,
            original.kind,
            &original.agent,
            replacement_content,
        );
        replacement.salience = original.salience;
        replacement.extensions = original.extensions.clone();
        replacement
            .extensions
            .insert("supersedes".into(), serde_json::json!(record_id));
        let replacement = self.write_governed_memory(replacement, Some(context))?;

        original.state = MemoryState::Revoked;
        original
            .extensions
            .insert("revoked_by".into(), serde_json::json!(replacement.id));
        self.write_contract_record(&original)?;
        Ok(replacement)
    }

    pub fn correction_chain(&self, record_id: &str) -> Result<CorrectionChain> {
        let records = self.read_contract_records()?;
        let mut cursor = record_id.to_owned();
        let mut seen = HashSet::new();
        while let Some(previous) = record_by_id(&records, &cursor)
            .and_then(|record| extension_string(record, "supersedes"))
        {
            if !seen.insert(cursor.clone()) {
                return Err(governance_error("cycle in correction history"));
            }
            cursor = previous;
        }

        let mut history = Vec::new();
        loop {
            let record = record_by_id(&records, &cursor)
                .cloned()
                .ok_or_else(|| governance_error(format!("memory {cursor} not found")))?;
            let next = extension_string(&record, "revoked_by");
            history.push(record);
            let Some(next) = next else { break };
            if !seen.insert(cursor.clone()) {
                return Err(governance_error("cycle in correction chain"));
            }
            cursor = next;
        }
        let current = history
            .last()
            .cloned()
            .ok_or_else(|| governance_error("empty correction chain"))?;
        Ok(CorrectionChain { current, history })
    }

    pub fn compress_episodic_batch(
        &self,
        record_ids: &[String],
        summary: impl Into<String>,
        context: &ConsumerContext,
    ) -> Result<MemoryRecord> {
        if record_ids.len() < 20 {
            return Err(governance_error("compression requires at least 20 records"));
        }
        let mut sources = record_ids
            .iter()
            .map(|id| self.require_contract_record(id))
            .collect::<Result<Vec<_>>>()?;
        let domain = scope_policy::domain(&sources[0]);
        let memory_scope = extension_string(&sources[0], "memory_scope");
        if sources.iter().any(|record| {
            record.kind != MemoryKind::Episodic
                || !matches!(record.state, MemoryState::Active | MemoryState::Promoted)
                || scope_policy::domain(record) != domain
                || extension_string(record, "memory_scope") != memory_scope
                || record.salience >= 0.5
        }) {
            return Err(governance_error(
                "compression requires low-salience eligible episodic records in one domain",
            ));
        }
        let oldest = sources
            .iter()
            .map(|record| record.created_at)
            .min()
            .unwrap();
        let newest = sources
            .iter()
            .map(|record| record.created_at)
            .max()
            .unwrap();
        if newest - oldest > chrono::Duration::days(7) {
            return Err(governance_error(
                "compression batch exceeds the seven-day window",
            ));
        }

        let summary_id = format!("mem_{}", uuid::Uuid::new_v4().simple());
        let mut compressed = MemoryRecord::new(
            &summary_id,
            MemoryKind::Semantic,
            "vaire-compression",
            summary,
        );
        compressed.salience = sources
            .iter()
            .map(|record| record.salience)
            .fold(0.0_f64, f64::max);
        compressed
            .extensions
            .insert("memory_domain".into(), serde_json::to_value(domain)?);
        compressed
            .extensions
            .insert("compressed_from".into(), serde_json::json!(record_ids));
        compressed.extensions.insert(
            "compression_ratio".into(),
            serde_json::json!(record_ids.len() as f64),
        );
        let compressed = self.write_governed_memory(compressed, Some(context))?;
        for source in &mut sources {
            source.state = MemoryState::Decayed;
            source
                .extensions
                .insert("compressed_into".into(), serde_json::json!(compressed.id));
            self.write_contract_record(source)?;
        }
        self.observe_governance(None, Some(record_ids.len()), 0, 0);
        Ok(compressed)
    }

    pub fn apply_retention(&self, as_of: DateTime<Utc>) -> Result<RetentionSweepReport> {
        let mut records = self.read_contract_records()?;
        let scanned = records.len();
        let mut decayed = Vec::new();
        for record in &mut records {
            if record.state != MemoryState::Active {
                continue;
            }
            let config = retention_config(record);
            let recency_override = if scope_policy::domain(record) == MemoryDomain::Personal
                && record
                    .extensions
                    .get("operator_authored")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
                && extension_string(record, "evidence_class").as_deref() == Some("confirmed")
            {
                Some(1.0)
            } else {
                None
            };
            let retrieval_count = record
                .extensions
                .get("retrieval_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let last_retrieved_at = record
                .extensions
                .get("last_retrieved_at")
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse().ok());
            let score = super::retention::score_record(
                record,
                config,
                as_of,
                retrieval_count,
                last_retrieved_at,
                recency_override,
            );
            if super::retention::is_forget_eligible(score, config) {
                record.state = MemoryState::Decayed;
                record.extensions.insert(
                    "retention_score".into(),
                    serde_json::json!({
                        "recency": score.recency,
                        "importance": score.importance,
                        "retrieval_freq": score.retrieval_freq,
                        "composite": score.composite,
                    }),
                );
                self.write_contract_record(record)?;
                decayed.push(record.id.clone());
            }
        }
        decayed.sort();
        self.observe_governance(Some(decayed.len()), None, 0, 0);
        Ok(RetentionSweepReport { scanned, decayed })
    }

    pub fn quarantine_provenance(
        &self,
        source_reference: &str,
        reason: impl Into<String>,
    ) -> Result<Vec<QuarantineReceipt>> {
        let reason = reason.into();
        let mut receipts = Vec::new();
        for mut record in self.read_contract_records()? {
            if record.state == MemoryState::Revoked
                || !["source", "source_reference"]
                    .iter()
                    .any(|key| extension_string(&record, key).as_deref() == Some(source_reference))
            {
                continue;
            }
            record.state = MemoryState::Quarantined;
            record
                .extensions
                .insert("quarantine_reason".into(), serde_json::json!(reason));
            self.write_contract_record(&record)?;
            receipts.push(QuarantineReceipt {
                record_id: record.id,
                reason: reason.clone(),
                quarantined_at: Utc::now(),
            });
        }
        self.observe_governance(None, None, 0, receipts.len());
        Ok(receipts)
    }

    pub fn revoke_memory(&self, record_id: &str, cascade: CascadePolicy) -> Result<RevokeReceipt> {
        let receipts_path = self.root.join("revoke_receipts.jsonl");
        let existing = read_revoke_receipts(&receipts_path)?;
        if let Some(receipt) = existing
            .iter()
            .find(|receipt| receipt.record_id == record_id && receipt.cascade == cascade)
        {
            return Ok(receipt.clone());
        }
        let mut source = self.require_contract_record(record_id)?;
        source.state = MemoryState::Revoked;
        self.write_contract_record(&source)?;

        let mut affected = Vec::new();
        if cascade != CascadePolicy::SourceOnly {
            for mut record in self.read_contract_records()? {
                if record.id == record_id || !references_record(&record, record_id) {
                    continue;
                }
                record
                    .extensions
                    .insert("derivation_stale".into(), serde_json::json!(true));
                if cascade == CascadePolicy::RegenerateDerived {
                    record
                        .extensions
                        .insert("regeneration_required".into(), serde_json::json!(true));
                }
                self.write_contract_record(&record)?;
                affected.push(record.id);
            }
            affected.extend(self.mark_persona_derivations_stale(record_id)?);
        }
        affected.sort();
        affected.dedup();
        let previous_hash = existing
            .last()
            .map(|receipt| receipt.receipt_hash.clone())
            .unwrap_or_default();
        let revoked_at = Utc::now();
        let receipt_hash = revoke_hash(&previous_hash, record_id, cascade, &affected, revoked_at);
        let receipt = RevokeReceipt {
            receipt_id: format!("rev_{}", uuid::Uuid::new_v4().simple()),
            record_id: record_id.to_owned(),
            cascade,
            affected_derived_ids: affected,
            revoked_at,
            previous_hash,
            receipt_hash,
        };
        store::append_jsonl(&receipts_path, &receipt)?;
        self.observe_governance(None, None, 1, 0);
        Ok(receipt)
    }

    pub fn validate_revoke_receipt_chain(&self) -> Result<()> {
        let receipts = read_revoke_receipts(&self.root.join("revoke_receipts.jsonl"))?;
        let mut previous_hash = String::new();
        for receipt in receipts {
            if receipt.previous_hash != previous_hash {
                return Err(governance_error("revoke receipt previous hash mismatch"));
            }
            let expected = revoke_hash(
                &receipt.previous_hash,
                &receipt.record_id,
                receipt.cascade,
                &receipt.affected_derived_ids,
                receipt.revoked_at,
            );
            if receipt.receipt_hash != expected {
                return Err(governance_error("revoke receipt hash mismatch"));
            }
            previous_hash = receipt.receipt_hash;
        }
        Ok(())
    }

    fn contract_root(&self) -> Result<&Path> {
        self.contract_memory_root.as_deref().ok_or_else(|| {
            governance_error("governed memory operations require a contract memory root")
        })
    }

    pub(super) fn read_contract_records(&self) -> Result<Vec<MemoryRecord>> {
        let mut records = Vec::new();
        for path in store::walk_dir(self.contract_root()?)? {
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            match serde_json::from_slice::<MemoryRecord>(&std::fs::read(&path)?) {
                Ok(record) => records.push(record),
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "skipping malformed governed memory")
                }
            }
        }
        Ok(records)
    }

    fn require_contract_record(&self, record_id: &str) -> Result<MemoryRecord> {
        self.read_contract_records()?
            .into_iter()
            .find(|record| record.id == record_id)
            .ok_or_else(|| governance_error(format!("memory {record_id} not found")))
    }

    fn write_contract_record(&self, record: &MemoryRecord) -> Result<()> {
        let kind = match record.kind {
            MemoryKind::Episodic => "episodic",
            MemoryKind::Semantic => "semantic",
        };
        store::write_atomic_json(
            &self
                .contract_root()?
                .join(kind)
                .join(format!("{}.json", record.id)),
            record,
        )
    }

    fn mark_persona_derivations_stale(&self, record_id: &str) -> Result<Vec<String>> {
        let mut affected = Vec::new();
        for path in store::walk_dir(&self.persona_root)? {
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let mut record: MemoryRecord = match serde_json::from_slice(&std::fs::read(&path)?) {
                Ok(record) => record,
                Err(_) => continue,
            };
            if serde_json::to_value(&record)?
                .to_string()
                .contains(record_id)
            {
                record
                    .extensions
                    .insert("derivation_stale".into(), serde_json::json!(true));
                store::write_atomic_json(&path, &record)?;
                affected.push(record.id);
            }
        }
        Ok(affected)
    }
}

fn record_by_id<'a>(records: &'a [MemoryRecord], id: &str) -> Option<&'a MemoryRecord> {
    records.iter().find(|record| record.id == id)
}

fn extension_string(record: &MemoryRecord, key: &str) -> Option<String> {
    record
        .extensions
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

fn retention_config(record: &MemoryRecord) -> super::retention::RetentionConfig {
    match scope_policy::domain(record) {
        MemoryDomain::Personal => super::retention::PERSONAL_RETENTION,
        MemoryDomain::Business => super::retention::BUSINESS_RETENTION,
        MemoryDomain::System if record.state == MemoryState::Promoted => {
            super::retention::SYSTEM_PROMOTED_RETENTION
        }
        MemoryDomain::System => super::retention::SYSTEM_RAW_RETENTION,
    }
}

fn references_record(record: &MemoryRecord, target: &str) -> bool {
    ["compressed_from", "source_ids", "supersedes"]
        .iter()
        .filter_map(|key| record.extensions.get(*key))
        .any(|value| match value {
            serde_json::Value::String(value) => value == target,
            serde_json::Value::Array(values) => {
                values.iter().any(|value| value.as_str() == Some(target))
            }
            _ => false,
        })
}

fn read_revoke_receipts(path: &Path) -> Result<Vec<RevokeReceipt>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

fn revoke_hash(
    previous_hash: &str,
    record_id: &str,
    cascade: CascadePolicy,
    affected: &[String],
    revoked_at: DateTime<Utc>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(previous_hash.as_bytes());
    hasher.update(record_id.as_bytes());
    hasher.update(format!("{cascade:?}").as_bytes());
    for id in affected {
        hasher.update(id.as_bytes());
    }
    hasher.update(revoked_at.to_rfc3339().as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn governance_error(message: impl Into<String>) -> ArdaError {
    ArdaError::Agent {
        agent: "vaire".to_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::contract::{MemoryKind, MemoryState};
    use chrono::Duration;
    use tempfile::tempdir;

    fn service() -> (tempfile::TempDir, MnemosyneService) {
        let directory = tempdir().unwrap();
        let service = MnemosyneService::new(directory.path().join("mnemosyne"))
            .unwrap()
            .with_contract_memory_root(directory.path().join("contract"));
        (directory, service)
    }

    fn context(domain: MemoryDomain) -> ConsumerContext {
        let mut context = ConsumerContext::new("operator", vec![domain]);
        context.operator_authorized = true;
        context
    }

    fn record(id: &str, domain: MemoryDomain, salience: f64) -> MemoryRecord {
        let mut record = MemoryRecord::new(id, MemoryKind::Episodic, "operator", id);
        record.salience = salience;
        record.extensions.insert(
            "memory_domain".into(),
            serde_json::to_value(domain).unwrap(),
        );
        record
            .extensions
            .insert("evidence_class".into(), serde_json::json!("confirmed"));
        record
    }

    #[test]
    fn correction_chain_is_walkable_from_old_or_current_record() {
        let (_directory, service) = service();
        service
            .write_governed_memory(record("old", MemoryDomain::Business, 0.8), None)
            .unwrap();
        let current = service
            .correct_memory("old", "corrected", &context(MemoryDomain::Business))
            .unwrap();
        let from_old = service.correction_chain("old").unwrap();
        let from_current = service.correction_chain(&current.id).unwrap();
        assert_eq!(from_old.current.id, current.id);
        assert_eq!(from_current.current.id, current.id);
        assert_eq!(from_old.history[0].id, "old");
        assert_eq!(from_old.history[0].state, MemoryState::Revoked);
        let recalled = service.recall_recent(24, None).unwrap();
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].memory_id, current.id);
    }

    #[test]
    fn compression_requires_twenty_low_salience_same_domain_records() {
        let (_directory, service) = service();
        let context = context(MemoryDomain::System);
        let mut ids = Vec::new();
        for index in 0..20 {
            let id = format!("raw-{index}");
            let mut memory = record(&id, MemoryDomain::System, 0.2);
            memory.created_at = Utc::now() - Duration::hours(index);
            service
                .write_governed_memory(memory, Some(&context))
                .unwrap();
            ids.push(id);
        }
        assert!(service
            .compress_episodic_batch(&ids[..19], "summary", &context)
            .is_err());
        let mut cross_scope = service.require_contract_record("raw-19").unwrap();
        cross_scope
            .extensions
            .insert("memory_scope".into(), serde_json::json!("different"));
        service.write_contract_record(&cross_scope).unwrap();
        assert!(service
            .compress_episodic_batch(&ids, "summary", &context)
            .is_err());
        cross_scope.extensions.remove("memory_scope");
        service.write_contract_record(&cross_scope).unwrap();
        let summary = service
            .compress_episodic_batch(&ids, "summary", &context)
            .unwrap();
        assert_eq!(
            summary.extensions["compressed_from"]
                .as_array()
                .unwrap()
                .len(),
            20
        );
        assert_eq!(
            summary.extensions["compression_ratio"],
            serde_json::json!(20.0)
        );
        assert_eq!(service.observability_snapshot().compression_runs_total, 1);
    }

    #[test]
    fn provenance_mismatch_is_quarantined_until_operator_clearance() {
        let (_directory, service) = service();
        let mut memory = record("external-personal", MemoryDomain::Personal, 0.8);
        memory
            .extensions
            .insert("source_external".into(), serde_json::json!(true));
        let written = service.write_governed_memory(memory, None).unwrap();
        assert_eq!(written.state, MemoryState::Quarantined);
        assert!(service
            .recall_governed_memories(Some(&context(MemoryDomain::Personal)))
            .unwrap()
            .is_empty());
        assert!(service
            .clear_quarantine(
                "external-personal",
                &ConsumerContext::new("agent", vec![MemoryDomain::Personal])
            )
            .is_err());
        let cleared = service
            .clear_quarantine("external-personal", &context(MemoryDomain::Personal))
            .unwrap();
        assert_eq!(cleared.state, MemoryState::Active);
    }

    #[test]
    fn revoke_is_idempotent_hash_chained_and_marks_derivatives_stale() {
        let (_directory, service) = service();
        service
            .write_governed_memory(record("source", MemoryDomain::Personal, 0.8), None)
            .unwrap();
        let mut derived = record("derived", MemoryDomain::Business, 0.8);
        derived
            .extensions
            .insert("compressed_from".into(), serde_json::json!(["source"]));
        service.write_governed_memory(derived, None).unwrap();
        let mut persona = MemoryRecord::new(
            "persona_identity_operator",
            MemoryKind::Semantic,
            "operator",
            "persona projection",
        );
        persona.extensions.insert(
            "persona.value_evidence".into(),
            serde_json::json!([{"value_id": "care", "source_records": ["source"]}]),
        );
        store::write_atomic_json(&service.persona_root.join("operator.json"), &persona).unwrap();
        let first = service
            .revoke_memory("source", CascadePolicy::MarkDerivedStale)
            .unwrap();
        let second = service
            .revoke_memory("source", CascadePolicy::MarkDerivedStale)
            .unwrap();
        assert_eq!(first, second);
        assert!(!first.receipt_hash.is_empty());
        service.validate_revoke_receipt_chain().unwrap();
        let recalled = service
            .recall_governed_memories(Some(&context(MemoryDomain::Business)))
            .unwrap();
        assert_eq!(recalled[0].extensions["derivation_stale"], true);
        let persona: MemoryRecord = serde_json::from_slice(
            &std::fs::read(service.persona_root.join("operator.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(persona.extensions["derivation_stale"], true);
        let metrics = service.observability_snapshot();
        assert_eq!(metrics.revocation_receipts_total, 1);
    }

    #[test]
    fn retention_transitions_old_raw_system_record_to_decayed() {
        let (_directory, service) = service();
        let mut memory = record("raw", MemoryDomain::System, 0.1);
        memory.created_at = Utc::now() - Duration::hours(48);
        memory.last_seen_at = memory.created_at;
        service.write_governed_memory(memory, None).unwrap();
        let report = service.apply_retention(Utc::now()).unwrap();
        assert_eq!(report.decayed, vec!["raw"]);
        assert!(service.recall_governed_memories(None).unwrap().is_empty());
        let metrics = service.observability_snapshot();
        assert_eq!(metrics.retention_runs_total, 1);
        assert_eq!(metrics.retention_decayed_total, 1);
    }

    #[test]
    fn consolidation_runs_retention_before_promotion() {
        let (_directory, service) = service();
        let mut memory = record("raw", MemoryDomain::System, 0.1);
        memory.created_at = Utc::now() - Duration::hours(48);
        memory.last_seen_at = memory.created_at;
        service.write_governed_memory(memory, None).unwrap();
        service.consolidate(24).unwrap();
        assert_eq!(
            service.require_contract_record("raw").unwrap().state,
            MemoryState::Decayed
        );
        assert_eq!(service.observability_snapshot().retention_runs_total, 1);
    }

    #[test]
    fn missing_context_for_personal_recall_is_an_error_not_an_empty_success() {
        let (_directory, service) = service();
        let mut operator = context(MemoryDomain::Personal);
        service
            .write_governed_memory(
                record("personal", MemoryDomain::Personal, 0.8),
                Some(&operator),
            )
            .unwrap();
        operator.operator_authorized = false;
        assert!(service.recall_governed_memories(None).is_err());
    }

    #[test]
    fn contextual_primary_recall_enforces_the_same_personal_redaction_policy() {
        let (_directory, service) = service();
        let event = super::super::InformantEvent {
            informant_id: "human".into(),
            event_type: "operator_note".into(),
            content: "private diagnosis and scheduling conflict".into(),
            crate_name: "arda-vaire".into(),
            tags: vec![
                "memory_domain:personal".into(),
                "evidence_class:confirmed".into(),
                "public_summary:operator had a scheduling conflict".into(),
                "sensitivity.health:diagnosis".into(),
            ],
            confidence_hint: Some(1.0),
            ts_utc: Utc::now().to_rfc3339(),
        };
        service
            .encode_with_context(event, Some(&context(MemoryDomain::Personal)))
            .unwrap();
        assert!(service
            .recall_recent_scoped_with_context(24, None, None, None)
            .is_err());
        let business = ConsumerContext::new("business", vec![MemoryDomain::Business]);
        let recalled = service
            .recall_recent_scoped_with_context(24, None, None, Some(&business))
            .unwrap();
        assert_eq!(recalled[0].content, "operator had a scheduling conflict");
    }

    #[test]
    fn normal_encode_quarantines_a_personal_source_reference_mismatch() {
        let (_directory, service) = service();
        let event = super::super::InformantEvent {
            informant_id: "external-import".into(),
            event_type: "operator_note".into(),
            content: "unverified personal claim".into(),
            crate_name: "arda-vaire".into(),
            tags: vec![
                "memory_domain:personal".into(),
                "source_expected:operator".into(),
                "source_observed:external-feed".into(),
            ],
            confidence_hint: Some(0.9),
            ts_utc: Utc::now().to_rfc3339(),
        };
        assert!(service.encode(event).unwrap().is_none());
        let records = service.read_contract_records().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].state, MemoryState::Quarantined);
        assert!(records[0].extensions["quarantine_reason"]
            .as_str()
            .unwrap()
            .contains("provenance"));
        assert_eq!(service.observability_snapshot().quarantine_records_total, 1);
    }
}
