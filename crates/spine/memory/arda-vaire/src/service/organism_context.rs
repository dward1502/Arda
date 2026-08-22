use super::scope_policy::MemoryDomain;
use arda_core::capability_composition::{
    CompositionAuthorityClass, DataClass, EgressTarget, RoleKind,
};
use arda_core::run_graph::{ObjectiveId, RunId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrganismContextError {
    #[error("invalid organism context JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported organism context schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("organism context field `{0}` cannot be empty")]
    EmptyField(&'static str),
    #[error("organism context field `{field}` exceeds its bound")]
    FieldTooLarge { field: &'static str },
    #[error("organism context expiry must be after generation")]
    InvalidExpiry,
    #[error("organism context field `{field}` contains duplicate reference `{value}`")]
    DuplicateReference { field: &'static str, value: String },
    #[error("organism context has conflicting capability `{0}`")]
    CapabilityConflict(String),
    #[error("sensitive context cannot permit external egress")]
    SensitiveExternalEgress,
    #[error("personal context requires explicit operator authority")]
    PersonalContextRequiresOperator,
    #[error("return contract output bound is invalid")]
    InvalidReturnBound,
    #[error("failed to serialize canonical organism context: {0}")]
    Serialize(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganismContext {
    pub schema_version: String,
    pub organism_id: String,
    pub generated_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub consumer: ContextConsumer,
    pub lineage: ContextLineage,
    pub objective: ContextObjective,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub memory_refs: Vec<String>,
    #[serde(default)]
    pub unresolved_failures: Vec<ContextFailure>,
    pub return_contract: ContextReturnContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextConsumer {
    pub consumer_id: String,
    pub role: RoleKind,
    pub authority_ceiling: CompositionAuthorityClass,
    pub operator_authorized: bool,
    pub memory_domains: Vec<MemoryDomain>,
    pub data_classes: Vec<DataClass>,
    pub permitted_egress: Vec<EgressTarget>,
    #[serde(default)]
    pub compute_node_refs: Vec<String>,
    pub agent_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextLineage {
    pub objective_id: ObjectiveId,
    pub project_id: Option<Uuid>,
    pub run_id: Option<RunId>,
    pub task_id: Option<String>,
    pub session_ref: Option<String>,
    #[serde(default)]
    pub parent_receipts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextObjective {
    pub requested_outcome: String,
    pub acceptance_conditions: Vec<String>,
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub forbidden_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextFailure {
    pub failure_id: String,
    pub class: String,
    pub summary: String,
    pub receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReturnContract {
    pub schema_version: String,
    pub required_receipt_types: Vec<String>,
    pub max_output_bytes: usize,
}

impl OrganismContext {
    pub const SCHEMA_VERSION: &'static str = "arda.organism-context.v1";
    pub const MAX_OUTPUT_BYTES: usize = 1_048_576;

    pub fn from_json_str(raw: &str) -> Result<Self, OrganismContextError> {
        let context: Self = serde_json::from_str(raw)
            .map_err(|error| OrganismContextError::InvalidJson(error.to_string()))?;
        context.validate()?;
        Ok(context)
    }

    pub fn validate(&self) -> Result<(), OrganismContextError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(OrganismContextError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        bounded_identifier("organism_id", &self.organism_id, 128)?;
        bounded_identifier(
            "lineage.objective_id",
            self.lineage.objective_id.as_str(),
            256,
        )?;
        bounded_identifier("consumer.consumer_id", &self.consumer.consumer_id, 256)?;
        if self.expires_at_unix_ms <= self.generated_at_unix_ms {
            return Err(OrganismContextError::InvalidExpiry);
        }
        if let Some(run_id) = &self.lineage.run_id {
            bounded_identifier("lineage.run_id", run_id.as_str(), 256)?;
        }
        for (field, value) in [
            ("lineage.task_id", self.lineage.task_id.as_deref()),
            ("lineage.session_ref", self.lineage.session_ref.as_deref()),
            ("consumer.agent_ref", self.consumer.agent_ref.as_deref()),
        ] {
            if let Some(value) = value {
                bounded_identifier(field, value, 512)?;
            }
        }
        bounded_text(
            "objective.requested_outcome",
            &self.objective.requested_outcome,
            4096,
        )?;
        if self.objective.acceptance_conditions.is_empty() {
            return Err(OrganismContextError::EmptyField(
                "objective.acceptance_conditions",
            ));
        }
        if self.objective.required_capabilities.is_empty() {
            return Err(OrganismContextError::EmptyField(
                "objective.required_capabilities",
            ));
        }
        reject_duplicates(
            "consumer.memory_domains",
            &self.consumer.memory_domains,
            |value| format!("{value:?}"),
        )?;
        reject_duplicates(
            "consumer.data_classes",
            &self.consumer.data_classes,
            |value| format!("{value:?}"),
        )?;
        reject_duplicates(
            "consumer.permitted_egress",
            &self.consumer.permitted_egress,
            |value| format!("{value:?}"),
        )?;
        reject_string_duplicates(
            "consumer.compute_node_refs",
            &self.consumer.compute_node_refs,
        )?;
        reject_text_duplicates(
            "objective.acceptance_conditions",
            &self.objective.acceptance_conditions,
            2048,
        )?;
        reject_string_duplicates(
            "objective.required_capabilities",
            &self.objective.required_capabilities,
        )?;
        reject_string_duplicates(
            "objective.forbidden_capabilities",
            &self.objective.forbidden_capabilities,
        )?;
        reject_string_duplicates("evidence_refs", &self.evidence_refs)?;
        reject_string_duplicates("memory_refs", &self.memory_refs)?;
        reject_string_duplicates("lineage.parent_receipts", &self.lineage.parent_receipts)?;
        reject_string_duplicates(
            "return_contract.required_receipt_types",
            &self.return_contract.required_receipt_types,
        )?;
        let forbidden = self
            .objective
            .forbidden_capabilities
            .iter()
            .collect::<BTreeSet<_>>();
        if let Some(conflict) = self
            .objective
            .required_capabilities
            .iter()
            .find(|capability| forbidden.contains(capability))
        {
            return Err(OrganismContextError::CapabilityConflict(
                (*conflict).clone(),
            ));
        }
        let sensitive = self.consumer.data_classes.iter().any(|class| {
            matches!(
                class,
                DataClass::Private | DataClass::Health | DataClass::Financial
            )
        });
        let external = self.consumer.permitted_egress.iter().any(|target| {
            matches!(
                target,
                EgressTarget::HostedProvider
                    | EgressTarget::ExternalAdapter
                    | EgressTarget::PublicNetwork
            )
        });
        if sensitive && external {
            return Err(OrganismContextError::SensitiveExternalEgress);
        }
        if self
            .consumer
            .memory_domains
            .contains(&MemoryDomain::Personal)
            && !self.consumer.operator_authorized
        {
            return Err(OrganismContextError::PersonalContextRequiresOperator);
        }
        if self.return_contract.schema_version.trim().is_empty()
            || self.return_contract.required_receipt_types.is_empty()
        {
            return Err(OrganismContextError::EmptyField("return_contract"));
        }
        if self.return_contract.max_output_bytes == 0
            || self.return_contract.max_output_bytes > Self::MAX_OUTPUT_BYTES
        {
            return Err(OrganismContextError::InvalidReturnBound);
        }
        let mut failure_ids = BTreeSet::new();
        for failure in &self.unresolved_failures {
            bounded_identifier("unresolved_failures.failure_id", &failure.failure_id, 256)?;
            if !failure_ids.insert(failure.failure_id.as_str()) {
                return Err(OrganismContextError::DuplicateReference {
                    field: "unresolved_failures.failure_id",
                    value: failure.failure_id.clone(),
                });
            }
            bounded_identifier("unresolved_failures.class", &failure.class, 128)?;
            bounded_text("unresolved_failures.summary", &failure.summary, 2048)?;
            if let Some(receipt_ref) = &failure.receipt_ref {
                bounded_identifier("unresolved_failures.receipt_ref", receipt_ref, 512)?;
            }
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, OrganismContextError> {
        self.validate()?;
        serde_json::to_string(self)
            .map_err(|error| OrganismContextError::Serialize(error.to_string()))
    }

    pub fn digest(&self) -> Result<String, OrganismContextError> {
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(self.canonical_json()?.as_bytes())
        ))
    }
}

fn bounded_identifier(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), OrganismContextError> {
    if value.trim().is_empty() {
        return Err(OrganismContextError::EmptyField(field));
    }
    if value.len() > max
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_:./+".contains(character))
    {
        return Err(OrganismContextError::FieldTooLarge { field });
    }
    Ok(())
}

fn bounded_text(field: &'static str, value: &str, max: usize) -> Result<(), OrganismContextError> {
    if value.trim().is_empty() {
        return Err(OrganismContextError::EmptyField(field));
    }
    if value.len() > max || value.as_bytes().contains(&0) {
        return Err(OrganismContextError::FieldTooLarge { field });
    }
    Ok(())
}

fn reject_duplicates<T, F>(
    field: &'static str,
    values: &[T],
    display: F,
) -> Result<(), OrganismContextError>
where
    T: PartialEq,
    F: Fn(&T) -> String,
{
    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(OrganismContextError::DuplicateReference {
                field,
                value: display(value),
            });
        }
    }
    Ok(())
}

fn reject_string_duplicates(
    field: &'static str,
    values: &[String],
) -> Result<(), OrganismContextError> {
    for value in values {
        bounded_identifier(field, value, 512)?;
    }
    reject_duplicates(field, values, Clone::clone)
}

fn reject_text_duplicates(
    field: &'static str,
    values: &[String],
    max: usize,
) -> Result<(), OrganismContextError> {
    for value in values {
        bounded_text(field, value, max)?;
    }
    reject_duplicates(field, values, Clone::clone)
}
