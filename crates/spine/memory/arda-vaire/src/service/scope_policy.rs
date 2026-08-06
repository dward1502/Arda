use arda_core::contract::{MemoryRecord, MemoryState};
use serde::{Deserialize, Serialize};

/// Primary governance domain, independent from subsystem-specific memory scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryDomain {
    Personal,
    Business,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyOperation {
    Read,
    Write,
}

/// Identity and declared purpose of a memory consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerContext {
    pub consumer_id: String,
    pub declared_domains: Vec<MemoryDomain>,
    pub purpose: Option<String>,
    pub operator_authorized: bool,
}

impl ConsumerContext {
    pub fn new(consumer_id: impl Into<String>, declared_domains: Vec<MemoryDomain>) -> Self {
        Self {
            consumer_id: consumer_id.into(),
            declared_domains,
            purpose: None,
            operator_authorized: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDisposition {
    Allow,
    Redact(Vec<String>),
    Quarantine,
    Block,
}

/// Evaluate one read or write at the memory-domain boundary.
pub fn evaluate(
    record: &MemoryRecord,
    operation: PolicyOperation,
    context: Option<&ConsumerContext>,
) -> PolicyDisposition {
    if operation == PolicyOperation::Write {
        if domain(record) == MemoryDomain::System && contains_embedded_secret(&record.content) {
            return PolicyDisposition::Block;
        }
        if domain(record) == MemoryDomain::Personal
            && provenance_mismatch(record)
            && !context.is_some_and(|value| value.operator_authorized)
        {
            return PolicyDisposition::Quarantine;
        }
        return PolicyDisposition::Allow;
    }

    if matches!(
        record.state,
        MemoryState::Decayed | MemoryState::Quarantined | MemoryState::Revoked
    ) {
        return PolicyDisposition::Block;
    }
    if domain(record) != MemoryDomain::Personal {
        return PolicyDisposition::Allow;
    }
    let Some(context) = context else {
        return PolicyDisposition::Block;
    };
    if context.declared_domains.contains(&MemoryDomain::Personal) {
        return PolicyDisposition::Allow;
    }
    if record
        .extensions
        .get("evidence_class")
        .and_then(|value| value.as_str())
        == Some("confirmed")
    {
        return PolicyDisposition::Redact(
            record
                .extensions
                .keys()
                .filter(|key| {
                    key.starts_with("sensitivity.health") || key.starts_with("sensitivity.identity")
                })
                .cloned()
                .collect(),
        );
    }
    PolicyDisposition::Block
}

/// Return a policy-safe clone of a record.
pub fn redact(record: &MemoryRecord, field_paths: &[String]) -> MemoryRecord {
    let mut redacted = record.clone();
    redacted.content = record
        .extensions
        .get("public_summary")
        .and_then(|value| value.as_str())
        .unwrap_or("[redacted personal memory]")
        .to_owned();
    for field in field_paths {
        redacted.extensions.remove(field);
    }
    redacted
}

pub fn domain(record: &MemoryRecord) -> MemoryDomain {
    match record
        .extensions
        .get("memory_domain")
        .and_then(|value| value.as_str())
    {
        Some("personal") => MemoryDomain::Personal,
        Some("business") => MemoryDomain::Business,
        _ => MemoryDomain::System,
    }
}

fn contains_embedded_secret(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    ["authorization: bearer ", "api_key=", "api-key=", "token="]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn provenance_mismatch(record: &MemoryRecord) -> bool {
    if record
        .extensions
        .get("source_external")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        return true;
    }
    let expected = record
        .extensions
        .get("source_expected")
        .and_then(|value| value.as_str());
    let observed = record
        .extensions
        .get("source_observed")
        .and_then(|value| value.as_str());
    matches!((expected, observed), (Some(expected), Some(observed)) if expected != observed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::contract::MemoryKind;

    fn record(domain: &str) -> MemoryRecord {
        let mut record = MemoryRecord::new("record", MemoryKind::Episodic, "source", "private");
        record
            .extensions
            .insert("memory_domain".into(), serde_json::json!(domain));
        record
    }

    #[test]
    fn personal_read_without_context_is_blocked() {
        assert_eq!(
            evaluate(&record("personal"), PolicyOperation::Read, None),
            PolicyDisposition::Block
        );
    }

    #[test]
    fn business_consumer_gets_redacted_personal_record() {
        let mut memory = record("personal");
        memory.extensions.insert(
            "public_summary".into(),
            serde_json::json!("operator had a scheduling conflict"),
        );
        memory
            .extensions
            .insert("sensitivity.health".into(), serde_json::json!("diagnosis"));
        memory
            .extensions
            .insert("evidence_class".into(), serde_json::json!("confirmed"));
        let context = ConsumerContext::new("business", vec![MemoryDomain::Business]);
        let PolicyDisposition::Redact(fields) =
            evaluate(&memory, PolicyOperation::Read, Some(&context))
        else {
            panic!("expected redaction");
        };
        let redacted = redact(&memory, &fields);
        assert_eq!(redacted.content, "operator had a scheduling conflict");
        assert!(!redacted.extensions.contains_key("sensitivity.health"));
    }

    #[test]
    fn business_record_is_allowed_for_any_authenticated_consumer() {
        let context = ConsumerContext::new("persona", vec![MemoryDomain::Personal]);
        assert_eq!(
            evaluate(&record("business"), PolicyOperation::Read, Some(&context)),
            PolicyDisposition::Allow
        );
    }

    #[test]
    fn raw_system_record_with_embedded_token_is_blocked_at_write() {
        let mut memory = record("system");
        memory.content = "Authorization: Bearer secret-token".into();
        assert_eq!(
            evaluate(&memory, PolicyOperation::Write, None),
            PolicyDisposition::Block
        );
    }

    #[test]
    fn external_personal_write_without_operator_authority_is_quarantined() {
        let mut memory = record("personal");
        memory
            .extensions
            .insert("source_external".into(), serde_json::json!(true));
        assert_eq!(
            evaluate(&memory, PolicyOperation::Write, None),
            PolicyDisposition::Quarantine
        );
        assert_eq!(memory.state, MemoryState::Active);
    }
}
