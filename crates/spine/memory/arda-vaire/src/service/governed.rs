//! Governed knowledge intake: approved deltas only.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

use super::MnemosyneService;
use arda_core::error::{ArdaError, Result};

pub const GOVERNED_KNOWLEDGE_SCHEMA_VERSION: &str = "arda.vaire.governed_knowledge.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovedKnowledgeDelta {
    pub delta_id: String,
    pub source_reference: String,
    pub warden_observation_id: String,
    pub varda_evaluation_id: String,
    pub approval_reference: String,
    pub content: String,
    pub correction_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernedKnowledgeReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub delta_id: String,
    pub source_reference: String,
    pub warden_observation_id: String,
    pub varda_evaluation_id: String,
    pub approval_reference: String,
    pub ingested_at_utc: String,
    pub correction_of: Option<String>,
}

impl MnemosyneService {
    pub fn ingest_approved_delta(
        &self,
        delta: ApprovedKnowledgeDelta,
    ) -> Result<GovernedKnowledgeReceipt> {
        if delta.delta_id.trim().is_empty()
            || delta.source_reference.trim().is_empty()
            || delta.warden_observation_id.trim().is_empty()
            || delta.varda_evaluation_id.trim().is_empty()
            || delta.approval_reference.trim().is_empty()
            || delta.content.trim().is_empty()
        {
            return Err(ArdaError::Agent {
                agent: "vaire".to_owned(),
                message: "approved knowledge delta is missing governance provenance".to_owned(),
            });
        }
        let path = governed_receipts_path(&self.root);
        if let Some(existing) = read_receipts(&path)?.into_iter().find(|receipt| {
            receipt.delta_id == delta.delta_id
                || receipt.approval_reference == delta.approval_reference
        }) {
            return Ok(existing);
        }
        let receipt = GovernedKnowledgeReceipt {
            schema_version: GOVERNED_KNOWLEDGE_SCHEMA_VERSION.to_owned(),
            receipt_id: format!("vrec_{}", uuid::Uuid::new_v4().simple()),
            delta_id: delta.delta_id,
            source_reference: delta.source_reference,
            warden_observation_id: delta.warden_observation_id,
            varda_evaluation_id: delta.varda_evaluation_id,
            approval_reference: delta.approval_reference,
            ingested_at_utc: Utc::now().to_rfc3339(),
            correction_of: delta.correction_of,
        };
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| ArdaError::Agent {
                agent: "vaire".to_owned(),
                message: format!("open governed receipt ledger: {error}"),
            })?;
        serde_json::to_writer(&mut file, &receipt).map_err(|error| ArdaError::Agent {
            agent: "vaire".to_owned(),
            message: format!("serialize governed receipt: {error}"),
        })?;
        writeln!(file).map_err(|error| ArdaError::Agent {
            agent: "vaire".to_owned(),
            message: format!("append governed receipt: {error}"),
        })?;
        file.sync_data().map_err(|error| ArdaError::Agent {
            agent: "vaire".to_owned(),
            message: format!("sync governed receipt: {error}"),
        })?;
        let _ = self.encode(super::InformantEvent {
            informant_id: "athena-governed".to_owned(),
            crate_name: "athena".to_owned(),
            event_type: "approved_knowledge_delta".to_owned(),
            ts_utc: receipt.ingested_at_utc.clone(),
            content: delta.content,
            confidence_hint: Some(1.0),
            tags: vec![
                "governed".to_owned(),
                format!("approval_reference:{}", receipt.approval_reference),
                format!("warden_observation:{}", receipt.warden_observation_id),
                format!("varda_evaluation:{}", receipt.varda_evaluation_id),
            ],
        })?;
        Ok(receipt)
    }
}

fn governed_receipts_path(root: &PathBuf) -> PathBuf {
    root.join("governed_knowledge_receipts.jsonl")
}

fn read_receipts(path: &PathBuf) -> Result<Vec<GovernedKnowledgeReceipt>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(ArdaError::Agent {
                agent: "vaire".to_owned(),
                message: format!("read governed receipt ledger: {error}"),
            });
        }
    };
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|error| ArdaError::Agent {
                agent: "vaire".to_owned(),
                message: format!("parse governed receipt ledger: {error}"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn approved_delta_is_idempotent_and_requires_complete_provenance() {
        let dir = tempdir().unwrap();
        let service = MnemosyneService::new(dir.path()).unwrap();
        let delta = ApprovedKnowledgeDelta {
            delta_id: "delta-1".into(),
            source_reference: "https://example.com#approval=ap-1".into(),
            warden_observation_id: "obs-1".into(),
            varda_evaluation_id: "eval-1".into(),
            approval_reference: "ap-1".into(),
            content: "approved fact".into(),
            correction_of: None,
        };
        let first = service.ingest_approved_delta(delta.clone()).unwrap();
        let second = service.ingest_approved_delta(delta).unwrap();
        assert_eq!(first.receipt_id, second.receipt_id);
        assert_eq!(
            read_receipts(&governed_receipts_path(&dir.path().to_path_buf()))
                .unwrap()
                .len(),
            1
        );
        assert!(
            service
                .ingest_approved_delta(ApprovedKnowledgeDelta {
                    approval_reference: String::new(),
                    ..ApprovedKnowledgeDelta {
                        delta_id: "raw".into(),
                        source_reference: "src".into(),
                        warden_observation_id: "obs".into(),
                        varda_evaluation_id: "eval".into(),
                        approval_reference: "".into(),
                        content: "raw".into(),
                        correction_of: None,
                    }
                })
                .is_err()
        );
    }
}
