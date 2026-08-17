//! Durable transcript-free Hermes session lineage references.

use super::scope_policy::{ConsumerContext, MemoryDomain};
use super::MnemosyneService;
use arda_core::error::{ArdaError, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;

pub const VAIRE_CONTINUITY_SCHEMA_VERSION: &str = "arda.vaire-continuity.v1";
const MAX_ID: usize = 256;
const MAX_REFS: usize = 32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityPrivacyClass {
    PublicRoom,
    SharedRoom,
    PrivateRoom,
    PersonalDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SurfaceHistoryEntry {
    pub surface_id: String,
    pub privacy_class: ContinuityPrivacyClass,
    pub observed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContinuityProvenance {
    pub source: String,
    pub source_event_ref: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContinuityRecord {
    pub schema_version: String,
    pub record_id: String,
    pub operator_ref: String,
    pub session_lineage_id: String,
    pub current_session_id: String,
    #[serde(default)]
    pub topic_refs: Vec<String>,
    #[serde(default)]
    pub commitment_refs: Vec<String>,
    #[serde(default)]
    pub active_surface_history: Vec<SurfaceHistoryEntry>,
    #[serde(default)]
    pub handoff_receipt_refs: Vec<String>,
    #[serde(default)]
    pub memory_scope_refs: Vec<String>,
    pub authorized_domains: Vec<MemoryDomain>,
    pub provenance: ContinuityProvenance,
    pub replay_key: String,
}

impl ContinuityRecord {
    fn validate(&self) -> Result<()> {
        if self.schema_version != VAIRE_CONTINUITY_SCHEMA_VERSION {
            return Err(invalid("unsupported Vairë continuity schema"));
        }
        for value in [
            &self.record_id,
            &self.operator_ref,
            &self.session_lineage_id,
            &self.current_session_id,
            &self.provenance.source,
            &self.provenance.source_event_ref,
        ] {
            if value.trim().is_empty() || value.len() > MAX_ID {
                return Err(invalid("continuity identity is missing or out of bounds"));
            }
        }
        if self.authorized_domains.is_empty() {
            return Err(invalid("continuity record requires an authorized domain"));
        }
        for refs in [
            &self.topic_refs,
            &self.commitment_refs,
            &self.handoff_receipt_refs,
            &self.memory_scope_refs,
        ] {
            if refs.len() > MAX_REFS
                || refs
                    .iter()
                    .any(|value| value.trim().is_empty() || value.len() > MAX_ID)
            {
                return Err(invalid("continuity references are out of bounds"));
            }
        }
        if self.active_surface_history.len() > MAX_REFS
            || self.active_surface_history.iter().any(|surface| {
                surface.surface_id.trim().is_empty()
                    || surface.surface_id.len() > MAX_ID
                    || surface.expires_at <= surface.observed_at
            })
        {
            return Err(invalid("continuity surface history is invalid"));
        }
        let digest = self.replay_key.strip_prefix("sha256:");
        if !digest.is_some_and(|value| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(invalid("continuity replay key is invalid"));
        }
        Ok(())
    }
}

impl MnemosyneService {
    pub fn record_continuity(&self, record: ContinuityRecord) -> Result<bool> {
        record.validate()?;
        let path = self.root.join("continuity").join("records.jsonl");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)?;
        file.lock_exclusive()?;
        let existing = fs::read_to_string(&path).unwrap_or_default();
        for line in existing.lines().filter(|line| !line.trim().is_empty()) {
            let prior: ContinuityRecord = serde_json::from_str(line)?;
            if prior.replay_key == record.replay_key || prior.record_id == record.record_id {
                if prior == record {
                    FileExt::unlock(&file)?;
                    return Ok(false);
                }
                FileExt::unlock(&file)?;
                return Err(invalid("continuity replay altered the durable record"));
            }
        }
        serde_json::to_writer(&mut file, &record)?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        FileExt::unlock(&file)?;
        Ok(true)
    }

    pub fn recall_continuity(
        &self,
        lineage: &str,
        context: &ConsumerContext,
        now: DateTime<Utc>,
    ) -> Result<Vec<ContinuityRecord>> {
        if lineage.trim().is_empty() || lineage.len() > MAX_ID {
            return Err(invalid("continuity lineage is invalid"));
        }
        let path = self.root.join("continuity").join("records.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut records = Vec::new();
        for line in fs::read_to_string(path)?
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let mut record: ContinuityRecord = serde_json::from_str(line)?;
            if record.session_lineage_id != lineage
                || !record
                    .authorized_domains
                    .iter()
                    .all(|domain| context.declared_domains.contains(domain))
            {
                continue;
            }
            record
                .active_surface_history
                .retain(|surface| surface.expires_at > now);
            records.push(record);
        }
        Ok(records)
    }
}

fn invalid(message: impl Into<String>) -> ArdaError {
    ArdaError::Agent {
        agent: "vaire".into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::scope_policy::{ConsumerContext, MemoryDomain};
    use chrono::{Duration, Utc};
    use tempfile::TempDir;

    fn record(domains: Vec<MemoryDomain>) -> ContinuityRecord {
        let now = Utc::now();
        ContinuityRecord {
            schema_version: VAIRE_CONTINUITY_SCHEMA_VERSION.into(),
            record_id: "continuity-1".into(),
            operator_ref: "operator-1".into(),
            session_lineage_id: "lineage-1".into(),
            current_session_id: "session-1".into(),
            topic_refs: vec!["topic:phase-2".into()],
            commitment_refs: vec!["commitment:finish-phase-2".into()],
            active_surface_history: vec![SurfaceHistoryEntry {
                surface_id: "discord:private-chat".into(),
                privacy_class: ContinuityPrivacyClass::PersonalDevice,
                observed_at: now,
                expires_at: now + Duration::minutes(15),
            }],
            handoff_receipt_refs: vec!["arda://continuity/receipts/one".into()],
            memory_scope_refs: vec!["vaire:scope:system-continuity".into()],
            authorized_domains: domains,
            provenance: ContinuityProvenance {
                source: "hermes-gateway".into(),
                source_event_ref: "hermes:message-1".into(),
                recorded_at: now,
            },
            replay_key: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
        }
    }

    #[test]
    fn personal_and_business_domains_remain_partitioned_without_explicit_overlap() {
        let root = TempDir::new().unwrap();
        let service = MnemosyneService::new(root.path()).unwrap();
        service
            .record_continuity(record(vec![MemoryDomain::Personal]))
            .unwrap();
        let business = ConsumerContext::new("business", vec![MemoryDomain::Business]);
        assert!(service
            .recall_continuity("lineage-1", &business, Utc::now())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn explicit_overlap_allows_authorized_reference_retrieval() {
        let root = TempDir::new().unwrap();
        let service = MnemosyneService::new(root.path()).unwrap();
        service
            .record_continuity(record(vec![MemoryDomain::Personal, MemoryDomain::Business]))
            .unwrap();
        let overlap = ConsumerContext::new(
            "operator-overlap",
            vec![MemoryDomain::Personal, MemoryDomain::Business],
        );
        let recalled = service
            .recall_continuity("lineage-1", &overlap, Utc::now())
            .unwrap();
        assert_eq!(recalled.len(), 1);
        assert_eq!(
            recalled[0].commitment_refs,
            vec!["commitment:finish-phase-2"]
        );
    }

    #[test]
    fn expired_surface_state_is_removed_but_durable_lineage_remains() {
        let root = TempDir::new().unwrap();
        let service = MnemosyneService::new(root.path()).unwrap();
        let item = record(vec![MemoryDomain::System]);
        let after_expiry = item.active_surface_history[0].expires_at + Duration::seconds(1);
        service.record_continuity(item).unwrap();
        let context = ConsumerContext::new("hud", vec![MemoryDomain::System]);
        let recalled = service
            .recall_continuity("lineage-1", &context, after_expiry)
            .unwrap();
        assert_eq!(recalled.len(), 1);
        assert!(recalled[0].active_surface_history.is_empty());
        assert_eq!(recalled[0].session_lineage_id, "lineage-1");
    }

    #[test]
    fn continuity_serialization_contains_references_and_no_transcript() {
        let value = serde_json::to_value(record(vec![MemoryDomain::System])).unwrap();
        assert!(value.get("transcript").is_none());
        assert_eq!(value["provenance"]["source_event_ref"], "hermes:message-1");
        assert_eq!(
            value["handoff_receipt_refs"][0],
            "arda://continuity/receipts/one"
        );
    }
}
