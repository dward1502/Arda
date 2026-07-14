// sigil: REPAIR
//! Read-only Arandur evidence registry for completion receipts and provenance.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const EVIDENCE_REGISTRY_CONTRACT: &str = "annunimas.arandur.evidence_registry.v1";
pub const OPERATOR_EXECUTION_RECEIPT_CONTRACT: &str =
    "annunimas.arandur.operator_approved_candidates_execution.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRecord {
    pub contract: String,
    pub source_record_id: Option<String>,
    pub candidate_id: Option<String>,
    pub approval_packet_id: Option<String>,
    pub status: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EvidenceRegistry {
    pub contract: String,
    pub evidence: Vec<EvidenceRecord>,
}

impl EvidenceRegistry {
    pub fn from_audit_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let audit_root = root.join("audit");
        let mut registry = Self {
            contract: EVIDENCE_REGISTRY_CONTRACT.into(),
            evidence: Vec::new(),
        };
        registry.scan_receipts(root, &audit_root);
        registry
    }

    pub fn operator_approved_candidate_receipts(&self) -> BTreeMap<(String, String), String> {
        self.evidence
            .iter()
            .filter(|record| {
                record.contract == OPERATOR_EXECUTION_RECEIPT_CONTRACT
                    && record.status == "executed_verified"
            })
            .filter_map(|record| {
                let candidate_id = record.candidate_id.as_ref()?;
                let approval_packet_id = record.approval_packet_id.as_ref()?;
                Some((
                    (candidate_id.clone(), approval_packet_id.clone()),
                    record.path.clone(),
                ))
            })
            .collect()
    }

    fn scan_receipts(&mut self, root: &Path, audit_root: &Path) {
        let mut stack = vec![audit_root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.file_name().and_then(|name| name.to_str()) == Some("execution_receipt.json")
                {
                    self.record_receipt(root, &path);
                }
            }
        }
    }

    fn record_receipt(&mut self, root: &Path, receipt_path: &Path) {
        let Ok(content) = fs::read_to_string(receipt_path) else {
            return;
        };
        let Ok(value) = serde_json::from_str::<Value>(&content) else {
            return;
        };
        let Some(contract) = value.get("contract").and_then(Value::as_str) else {
            return;
        };
        let display_path = receipt_path
            .strip_prefix(root)
            .unwrap_or(receipt_path)
            .to_string_lossy()
            .to_string();

        if let Some(tasks) = value.get("tasks").and_then(Value::as_array) {
            self.evidence.extend(tasks.iter().map(|task| {
                EvidenceRecord {
                    contract: contract.to_string(),
                    source_record_id: first_string(
                        task,
                        &["source_record_id", "recommendation_id"],
                    ),
                    candidate_id: first_string(task, &["candidate_id", "id"]),
                    approval_packet_id: first_string(task, &["approval_packet_id", "approval_id"]),
                    status: task
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    path: display_path.clone(),
                }
            }));
        } else {
            self.evidence.push(EvidenceRecord {
                contract: contract.to_string(),
                source_record_id: first_string(&value, &["source_record_id", "recommendation_id"]),
                candidate_id: first_string(&value, &["candidate_id", "id"]),
                approval_packet_id: first_string(&value, &["approval_packet_id", "approval_id"]),
                status: value
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                path: display_path,
            });
        }
    }
}

fn first_string(value: &Value, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_operator_execution_receipts_by_candidate_and_approval_packet() {
        let dir = tempfile::tempdir().expect("tempdir");
        let receipt_dir = dir.path().join("audit/packet");
        std::fs::create_dir_all(&receipt_dir).expect("receipt dir");
        std::fs::write(
            receipt_dir.join("execution_receipt.json"),
            r#"{
              "contract":"annunimas.arandur.operator_approved_candidates_execution.v1",
              "tasks":[
                {"candidate_id":"candidate-a","approval_packet_id":"approval-a","source_record_id":"rec-a","status":"executed_verified"},
                {"candidate_id":"candidate-b","approval_packet_id":"approval-b","status":"blocked"}
              ]
            }"#,
        )
        .expect("write receipt");

        let registry = EvidenceRegistry::from_audit_root(dir.path());
        let index = registry.operator_approved_candidate_receipts();

        assert_eq!(registry.contract, EVIDENCE_REGISTRY_CONTRACT);
        assert_eq!(registry.evidence.len(), 2);
        assert_eq!(index.len(), 1);
        assert_eq!(
            index.get(&("candidate-a".into(), "approval-a".into())),
            Some(&"audit/packet/execution_receipt.json".to_string())
        );
    }
}
