// sigil: REPAIR
//
// Persistent content-addressed cache for expensive deep-analysis results.
// Keys are derived from the normalized query, a canonical document-ID set,
// and the model identity so callers cannot reuse results across model changes.

use arda_core::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::{athena_error, DeepBookEntry};

const SCHEMA_VERSION: &str = "arda.athena.deep-cache.v1";

#[derive(Debug, Serialize, Deserialize)]
struct DeepCacheRecord {
    schema_version: String,
    key: String,
    query: String,
    relevant_doc_ids: Vec<String>,
    model_id: String,
    result: DeepBookEntry,
}

pub(super) struct DeepAnalysisCache {
    root: PathBuf,
}

impl DeepAnalysisCache {
    pub(super) fn new(store_root: &Path) -> Self {
        Self {
            root: store_root.join("cache/deep_analysis"),
        }
    }

    pub(super) fn load(
        &self,
        query: &str,
        relevant_doc_ids: &[String],
        model_id: &str,
    ) -> Result<Option<DeepBookEntry>> {
        let (key, query, relevant_doc_ids, model_id) =
            canonical_key_parts(query, relevant_doc_ids, model_id);
        let path = self.path_for_key(&key);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let record: DeepCacheRecord = serde_json::from_slice(&bytes).map_err(|err| {
            athena_error(format!(
                "invalid deep-analysis cache record {}: {err}",
                path.display()
            ))
        })?;
        if record.schema_version != SCHEMA_VERSION
            || record.key != key
            || record.query != query
            || record.relevant_doc_ids != relevant_doc_ids
            || record.model_id != model_id
        {
            return Ok(None);
        }
        Ok(Some(record.result))
    }

    pub(super) fn store(
        &self,
        query: &str,
        relevant_doc_ids: &[String],
        model_id: &str,
        result: &DeepBookEntry,
    ) -> Result<()> {
        let (key, query, relevant_doc_ids, model_id) =
            canonical_key_parts(query, relevant_doc_ids, model_id);
        fs::create_dir_all(&self.root)?;
        let record = DeepCacheRecord {
            schema_version: SCHEMA_VERSION.to_string(),
            key: key.clone(),
            query,
            relevant_doc_ids,
            model_id,
            result: result.clone(),
        };
        let bytes = serde_json::to_vec(&record)?;
        let path = self.path_for_key(&key);
        let temporary = self.root.join(format!(".{key}.tmp"));
        fs::write(&temporary, bytes)?;
        fs::rename(&temporary, path)?;
        Ok(())
    }

    pub(super) fn invalidate_doc(&self, doc_id: &str) -> Result<usize> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => return Err(err.into()),
        };
        let mut removed = 0;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let Ok(record) = serde_json::from_slice::<DeepCacheRecord>(&bytes) else {
                continue;
            };
            if record.relevant_doc_ids.iter().any(|value| value == doc_id) {
                fs::remove_file(path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        self.root.join(format!("{key}.json"))
    }
}

fn canonical_key_parts(
    query: &str,
    relevant_doc_ids: &[String],
    model_id: &str,
) -> (String, String, Vec<String>, String) {
    let query = query.trim().to_string();
    let model_id = model_id.trim().to_string();
    let mut relevant_doc_ids = relevant_doc_ids
        .iter()
        .map(|doc_id| doc_id.trim().to_string())
        .filter(|doc_id| !doc_id.is_empty())
        .collect::<Vec<_>>();
    relevant_doc_ids.sort();
    relevant_doc_ids.dedup();

    let mut digest = Sha256::new();
    hash_field(&mut digest, query.as_bytes());
    for doc_id in &relevant_doc_ids {
        hash_field(&mut digest, doc_id.as_bytes());
    }
    hash_field(&mut digest, model_id.as_bytes());
    let key = format!("{:x}", digest.finalize());
    (key, query, relevant_doc_ids, model_id)
}

fn hash_field(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::canonical_key_parts;

    fn cache_key(query: &str, relevant_doc_ids: &[String], model_id: &str) -> String {
        canonical_key_parts(query, relevant_doc_ids, model_id).0
    }

    #[test]
    fn cache_key_canonicalizes_document_order_and_separates_inputs() {
        let canonical = cache_key(
            " deep analyze source ",
            &[
                "doc-b".to_string(),
                "doc-a".to_string(),
                "doc-a".to_string(),
            ],
            "model-a",
        );
        assert_eq!(
            canonical,
            cache_key(
                "deep analyze source",
                &["doc-a".to_string(), "doc-b".to_string()],
                "model-a",
            )
        );
        assert_ne!(
            canonical,
            cache_key(
                "different query",
                &["doc-a".to_string(), "doc-b".to_string()],
                "model-a",
            )
        );
        assert_ne!(
            canonical,
            cache_key("deep analyze source", &["doc-a".to_string()], "model-a")
        );
        assert_ne!(
            canonical,
            cache_key(
                "deep analyze source",
                &["doc-a".to_string(), "doc-b".to_string()],
                "model-b",
            )
        );
    }
}
