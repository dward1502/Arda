// sigil: REPAIR
//
// Read-time schema migration for append-only Athena JSONL stores.

use serde_json::Value;

pub(super) const CURRENT_JSONL_SCHEMA_VERSION: u64 = 2;

#[derive(Debug, Clone, Copy)]
pub(super) enum JsonlStoreSchema {
    DeepQueue,
    PolicyReadiness,
}

/// Upgrade one append-only record in memory without rewriting historical JSONL.
///
/// Version `0` is the pre-versioning format. Version `1` is the canonical field
/// shape before an explicit marker was emitted; version `2` adds that marker.
/// Future versions are rejected so an older binary cannot silently misread them.
pub(super) fn migrate_jsonl_value(
    schema: JsonlStoreSchema,
    mut value: Value,
) -> std::result::Result<Value, String> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| "JSONL record must be an object".to_string())?;
    let version = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if version > CURRENT_JSONL_SCHEMA_VERSION {
        return Err(format!(
            "unsupported JSONL schema version {version}; current version is {CURRENT_JSONL_SCHEMA_VERSION}"
        ));
    }

    if version == 0 {
        match schema {
            JsonlStoreSchema::DeepQueue => {
                move_field(object, "ts_utc", "ts");
                move_field(object, "state", "status");
            }
            JsonlStoreSchema::PolicyReadiness => {
                move_field(object, "readiness", "policy_readiness");
                move_field(object, "policy_gate", "gate");
            }
        }
    }
    object.insert(
        "schema_version".to_string(),
        Value::from(CURRENT_JSONL_SCHEMA_VERSION),
    );
    Ok(value)
}

fn move_field(object: &mut serde_json::Map<String, Value>, old: &str, new: &str) {
    if !object.contains_key(new) {
        if let Some(value) = object.remove(old) {
            object.insert(new.to_string(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{migrate_jsonl_value, JsonlStoreSchema, CURRENT_JSONL_SCHEMA_VERSION};

    #[test]
    fn migrates_legacy_deep_queue_fields_to_current_schema() {
        let legacy = serde_json::json!({
            "ts_utc": "2026-07-25T12:00:00Z",
            "event": "deep_queued",
            "source_id": "src_legacy",
            "state": "pending_deep",
            "agent": "athena",
            "reason": "legacy"
        });

        let migrated = migrate_jsonl_value(JsonlStoreSchema::DeepQueue, legacy)
            .expect("legacy deep queue migration");
        assert_eq!(migrated["schema_version"], CURRENT_JSONL_SCHEMA_VERSION);
        assert_eq!(migrated["ts"], "2026-07-25T12:00:00Z");
        assert_eq!(migrated["status"], "pending_deep");
    }

    #[test]
    fn migrates_legacy_policy_readiness_fields_to_current_schema() {
        let legacy = serde_json::json!({
            "ts_utc": "2026-07-25T12:00:00Z",
            "source_id": "src_legacy",
            "readiness": "policy_ready",
            "policy_gate": {"passed": true}
        });

        let migrated = migrate_jsonl_value(JsonlStoreSchema::PolicyReadiness, legacy)
            .expect("legacy policy migration");
        assert_eq!(migrated["schema_version"], CURRENT_JSONL_SCHEMA_VERSION);
        assert_eq!(migrated["policy_readiness"], "policy_ready");
        assert_eq!(migrated["gate"]["passed"], true);
    }

    #[test]
    fn rejects_records_from_a_future_schema() {
        let future = serde_json::json!({
            "schema_version": CURRENT_JSONL_SCHEMA_VERSION + 1,
            "source_id": "src_future"
        });
        assert!(migrate_jsonl_value(JsonlStoreSchema::DeepQueue, future).is_err());
    }
}
